//! `ferrox-runtime` -- Layer 3 of Ferrox: runtime anti-tamper and
//! anti-debug, linked directly into the protected binary.
//!
//! ## NF6: no panics, audited
//! This crate must never panic under any input -- a protection library
//! that itself crashes defeats the purpose. `#![deny(clippy::unwrap_used)]`
//! below is the automated half of that audit; every `Option`/`Result` in
//! this crate is handled explicitly rather than ever calling `.unwrap()`.
#![deny(clippy::unwrap_used)]

pub mod anti_debug;
pub mod checksum;
pub mod panic_hook;

use std::sync::atomic::{AtomicBool, Ordering};

static TAMPER_DETECTED: AtomicBool = AtomicBool::new(false);

pub(crate) fn mark_tampered() {
    TAMPER_DETECTED.store(true, Ordering::SeqCst);
}

pub fn tamper_detected() -> bool {
    TAMPER_DETECTED.load(Ordering::SeqCst)
}

pub fn init() {
    panic_hook::install();
    checksum::start();
}

/// Phase 8 marker, consumed by `ferrox-mir-pass`'s `#[ferrox::protect]`
/// detection, not by anything at runtime. `ferrox-macros`' `protect`
/// attribute injects a call to this function as the first statement of
/// any function it's applied to; `ferrox-mir-pass` looks for a call whose
/// target's name ends in `__ferrox_protect_marker` (see
/// `ferrox-mir-pass/src/flatten.rs`'s `is_protected_marker_call` for the
/// exact matching logic and its documented limitation) to decide whether
/// to apply Layer 2 flattening/opaque-predicate transforms to that
/// function. `#[inline(never)]` matters here: it prevents this call from
/// disappearing via inlining before `ferrox-mir-pass` gets a chance to
/// see it in the caller's own MIR (the body itself does nothing, so
/// there's nothing to inline *into* anyway -- `inline(never)` is purely
/// about keeping the *call site* intact in the caller).
#[inline(never)]
#[doc(hidden)]
pub fn __ferrox_protect_marker() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_tampered_sets_the_shared_flag() {
        mark_tampered();
        assert!(tamper_detected(), "flag must read true after mark_tampered()");
    }

    #[test]
    fn init_does_not_panic() {
        init();
    }

    #[test]
    fn protect_marker_does_not_panic() {
        __ferrox_protect_marker();
    }
}
