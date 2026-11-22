//! `ferrox-runtime` -- Layer 3 of Ferrox: runtime anti-tamper and
//! anti-debug, linked directly into the protected binary.
//!
//! ## NF6: no panics, audited
//! This crate must never panic under any input -- a protection library
//! that itself crashes defeats the purpose. `#![deny(clippy::unwrap_used)]`
//! below is the automated half of that audit; every `Option`/`Result` in
//! this crate is handled explicitly (via early-return `?`/`match`/
//! `unwrap_or*`) rather than ever calling `.unwrap()`.
#![deny(clippy::unwrap_used)]

pub mod anti_debug;
pub mod checksum;
pub mod panic_hook;

use std::sync::atomic::{AtomicBool, Ordering};

/// Global tamper flag. Set by either the checksum thread detecting a
/// code-region mismatch (F1.5) or any anti-debug check firing (F1.6).
/// Deliberately a single shared flag rather than per-check flags -- F1.5's
/// response ("silently corrupt program output") doesn't need to know
/// *which* check fired, only *that* something did.
static TAMPER_DETECTED: AtomicBool = AtomicBool::new(false);

/// Marks the process as tampered-with. Internal to the crate's own checks
/// (`checksum`, `anti_debug::*`) -- consuming crates read tamper state via
/// `tamper_detected()`, they don't set it directly.
pub(crate) fn mark_tampered() {
    TAMPER_DETECTED.store(true, Ordering::SeqCst);
}

/// Returns whether tampering has been detected so far. F1.5's actual
/// "silently corrupt output" behavior is the *consuming crate's*
/// responsibility to implement at its own output/return-value sites --
/// `ferrox-runtime` deliberately does not corrupt anything on the
/// caller's behalf (it has no idea what "the program's output" means for
/// an arbitrary protected crate). `examples/license-checker` (Phase 6)
/// demonstrates a concrete call site: checking this flag right before
/// returning its license-valid/invalid verdict, and flipping the verdict
/// if tampering was detected.
pub fn tamper_detected() -> bool {
    TAMPER_DETECTED.load(Ordering::SeqCst)
}

/// Convenience bundle: installs the release-mode panic hook (F1.3) and
/// starts the checksum thread (F1.4). Does **not** run any anti-debug
/// checks -- those are deliberately left for the consuming crate to call
/// individually, from scattered call sites (F1.7), rather than bundled
/// here (bundling them here would itself be the kind of centralization
/// F1.7 asks to avoid).
pub fn init() {
    panic_hook::install();
    checksum::start();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_tampered_sets_the_shared_flag() {
        // Note: `TAMPER_DETECTED` is process-global static state, shared
        // across every test in this crate's test binary (cargo runs unit
        // tests from one binary, by default in parallel). We only assert
        // the one-directional property that holds regardless of test
        // execution order: once marked, it stays marked. We deliberately
        // do NOT assert an initial "false" state, since another test
        // could already have set it.
        mark_tampered();
        assert!(tamper_detected(), "flag must read true after mark_tampered()");
    }

    #[test]
    fn init_does_not_panic() {
        init();
    }
}
