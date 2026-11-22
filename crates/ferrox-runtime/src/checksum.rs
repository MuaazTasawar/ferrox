//! Layer 3's self-checksum / anti-tamper thread (F1.4, F1.5).
//!
//! ## Known limitation (documented honestly, see docs/THREAT_MODEL.md)
//! Production self-checksumming binaries embed a hash computed *after*
//! final linking, patched into the binary post-build (since the hash of a
//! section depends on bytes that do not exist until the binary is fully
//! linked -- a classic chicken-and-egg problem). That post-link patching
//! step is **not yet implemented** -- it belongs in `ferrox-cli`'s build
//! orchestration (Phase 5) as a post-process step over the compiled
//! artifact.
//!
//! This phase ships the trust-on-first-run (TOFU) interim instead: the
//! baseline hash is computed from the *running* process's own code region
//! the first time `start()` runs, then compared against on each
//! subsequent tick. This still defeats a very common attack shape -- a
//! debugger attaching *after* the process starts and patching a check in
//! place -- but it does not defeat an attacker who statically patches the
//! binary before it ever runs (the baseline would simply reflect the
//! already-patched bytes). docs/THREAT_MODEL.md states this plainly.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Once;
use std::thread;
use std::time::Duration;

use crate::mark_tampered;

static BASELINE_HASH: AtomicU64 = AtomicU64::new(0);
static BASELINE_INIT: Once = Once::new();

/// FNV-1a 64-bit hash. Chosen over a cryptographic hash deliberately
/// (NF12, minimal dependency footprint): this is a tamper-evidence
/// checksum, not a security boundary that needs collision resistance
/// against a cryptanalyst -- the same reasoning `encrypt.rs` applies to
/// RC4 for a different primitive.
fn fnv1a_64(data: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET_BASIS;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

mod locate {
    #[cfg(target_os = "linux")]
    pub mod linux {
        /// Parses `/proc/self/maps` to find the first executable (`r-xp`)
        /// mapping whose backing file matches this process's own
        /// executable path -- i.e. the main binary's code segment.
        ///
        /// Simplification, documented: this locates the first executable
        /// mapping of the main binary, which in a typical non-split
        /// single-segment layout corresponds closely to `.text`, but is
        /// not a true ELF section-header parse. A precise `.text`-only
        /// parse is a documented stretch goal, not required for F1.4 as
        /// specified.
        ///
        /// # Safety
        /// Returns a raw pointer/length describing a region this process
        /// itself has mapped as executable, per the OS's own
        /// authoritative `/proc/self/maps` report. Sound to read for the
        /// lifetime of the process, since a process's own main executable
        /// mapping does not move or unmap itself.
        pub unsafe fn own_code_region() -> Option<(*const u8, usize)> {
            let exe_path = std::env::current_exe().ok()?;
            let exe_name = exe_path.file_name()?.to_str()?;
            let maps = std::fs::read_to_string("/proc/self/maps").ok()?;

            for line in maps.lines() {
                // Format: "start-end perms offset dev inode pathname"
                let mut parts = line.split_whitespace();
                let range = parts.next()?;
                let perms = parts.next()?;
                let pathname = parts.last().unwrap_or("");

                if !perms.starts_with("r-xp") {
                    continue;
                }
                if !pathname.ends_with(exe_name) {
                    continue;
                }

                let (start_str, end_str) = range.split_once('-')?;
                let start = usize::from_str_radix(start_str, 16).ok()?;
                let end = usize::from_str_radix(end_str, 16).ok()?;
                return Some((start as *const u8, end - start));
            }
            None
        }
    }

    #[cfg(target_os = "windows")]
    pub mod windows {
        use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows_sys::Win32::System::ProcessStatus::{GetModuleInformation, MODULEINFO};
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        /// Uses `GetModuleHandleW(None)` to get the main module's base
        /// address and `GetModuleInformation` to get its size.
        ///
        /// Simplification, documented: this hashes the entire loaded PE
        /// image (`.text` + `.rdata` + `.data` + ...), not a `.text`-only
        /// section-table parse. A precise PE section parse (via
        /// `IMAGE_NT_HEADERS`) is a documented stretch goal -- arguably
        /// this simpler approach is *stronger* in practice anyway, since
        /// it also catches tampering with read-only data.
        ///
        /// # Safety
        /// `GetModuleHandleW(null)` and `GetModuleInformation` are
        /// standard Win32 calls with no preconditions beyond a valid
        /// process handle (always true for `GetCurrentProcess()`). The
        /// returned base/size describe this process's own loaded image,
        /// which remains mapped for the process's lifetime.
        pub unsafe fn own_code_region() -> Option<(*const u8, usize)> {
            let handle = GetModuleHandleW(std::ptr::null());
            if handle == 0 {
                return None;
            }
            let process = GetCurrentProcess();
            let mut info: MODULEINFO = std::mem::zeroed();
            let ok = GetModuleInformation(
                process,
                handle,
                &mut info,
                std::mem::size_of::<MODULEINFO>() as u32,
            );
            if ok == 0 {
                return None;
            }
            Some((info.lpBaseOfDll as *const u8, info.SizeOfImage as usize))
        }
    }
}

/// # Safety
/// Delegates to the platform-specific `locate::*::own_code_region`,
/// whose safety contracts are documented at each implementation.
unsafe fn own_code_region() -> Option<(*const u8, usize)> {
    #[cfg(target_os = "linux")]
    {
        locate::linux::own_code_region()
    }
    #[cfg(target_os = "windows")]
    {
        locate::windows::own_code_region()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

fn hash_own_code_region() -> Option<u64> {
    // Safety: see `own_code_region`'s doc comment and its platform
    // implementations.
    let (ptr, len) = unsafe { own_code_region() }?;
    if ptr.is_null() || len == 0 {
        return None;
    }
    // Safety: ptr/len come directly from the OS's own module/mapping
    // metadata for this process, valid per `own_code_region`'s contract.
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    Some(fnv1a_64(slice))
}

/// Starts the background anti-tamper thread (F1.4). Idempotent: calling
/// this more than once (e.g. from multiple scattered call sites, per the
/// spirit of F1.7) only spawns the thread once.
pub fn start() {
    BASELINE_INIT.call_once(|| {
        if let Some(hash) = hash_own_code_region() {
            BASELINE_HASH.store(hash, Ordering::SeqCst);
            spawn_watch_thread();
        }
        // If hashing failed (unsupported platform, or an OS API failure),
        // we deliberately do not spawn the thread and do not panic or
        // report an error (NF6) -- Layer 3 degrades to a no-op rather
        // than crashing the user's build on a platform it doesn't yet
        // support (NF9 limits v1 to Linux/Windows anyway).
    });
}

fn spawn_watch_thread() {
    thread::spawn(|| loop {
        // Randomized 2-5s interval (F1.4). Not cryptographically random
        // -- this only needs to avoid a fixed, predictable period an
        // attacker could time around.
        let secs = 2.0 + rand_fraction() * 3.0;
        thread::sleep(Duration::from_secs_f64(secs));

        match hash_own_code_region() {
            Some(current) if current != BASELINE_HASH.load(Ordering::SeqCst) => {
                // F1.5: silently corrupt, never crash cleanly or print.
                mark_tampered();
            }
            Some(_) => {}
            None => {
                // Couldn't re-hash this tick (e.g. transient OS API
                // failure) -- skip this tick rather than treating a read
                // failure as tamper evidence (would be a false positive)
                // or panicking (NF6).
            }
        }
    });
}

/// Small dependency-free random fraction in `[0.0, 1.0)`, used only to
/// jitter the checksum thread's sleep interval (F1.4). Deliberately not
/// using the `rand` crate here: that would add this crate's first
/// dependency purely for interval jitter, where a coarse
/// non-cryptographic source (current time's sub-second bits) is
/// completely sufficient (NF12).
fn rand_fraction() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos % 1_000_000_000) as f64 / 1_000_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_is_deterministic_and_sensitive_to_input() {
        let a = fnv1a_64(b"hello world");
        let b = fnv1a_64(b"hello world");
        let c = fnv1a_64(b"hello world!");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn hash_own_code_region_succeeds_on_supported_platforms() {
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            let hash = hash_own_code_region();
            assert!(
                hash.is_some(),
                "expected to successfully hash own code region on a supported platform"
            );
        }
    }

    #[test]
    fn rand_fraction_is_in_unit_range() {
        let f = rand_fraction();
        assert!((0.0..1.0).contains(&f));
    }
}

