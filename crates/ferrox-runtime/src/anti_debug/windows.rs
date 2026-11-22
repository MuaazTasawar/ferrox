//! F1.6, check 2 of 3: `IsDebuggerPresent`.
//!
//! The most well-known Windows anti-debug primitive: a flag in the
//! Process Environment Block that the OS sets when a debugger is
//! attached. Trivially bypassable on its own (any competent reverse
//! engineer patches this flag or hooks the call within minutes) -- it is
//! one of three independent, non-adjacent checks (F1.6/F1.7), not the
//! sole line of defense.

use windows_sys::Win32::System::Diagnostics::Debug::IsDebuggerPresent;

use crate::mark_tampered;

/// Runs the `IsDebuggerPresent` check once. Safe to call from multiple,
/// unrelated call sites (F1.7).
pub fn check() {
    // Safety: `IsDebuggerPresent` takes no arguments and has no
    // preconditions; it is always safe to call.
    let present = unsafe { IsDebuggerPresent() };
    if present != 0 {
        mark_tampered();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_does_not_panic_when_undebugged() {
        // NF6: must never panic. Under a normal `cargo test` run (no
        // debugger attached), IsDebuggerPresent should report false and
        // this should be a no-op.
        check();
    }
}
