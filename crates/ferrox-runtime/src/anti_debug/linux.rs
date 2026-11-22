//! F1.6, check 1 of 3: `ptrace(PTRACE_TRACEME)` self-trace.
//!
//! A process can only be ptraced by one tracer at a time. If this process
//! is already being traced (i.e. a debugger like gdb is attached),
//! `PTRACE_TRACEME` fails. If it succeeds, no debugger was attached *at
//! the moment of the call* -- so this check immediately detaches again
//! (`PTRACE_DETACH`) to avoid leaving the process in a traced state,
//! which would interfere with the rest of the program.

use crate::mark_tampered;

/// Runs the ptrace self-trace check once. Safe to call from multiple,
/// unrelated call sites (F1.7) -- each call independently attempts
/// trace-then-detach.
pub fn check() {
    // Safety: `ptrace` with `PTRACE_TRACEME` and no further arguments is
    // a well-defined syscall with no memory-safety preconditions beyond
    // "called from a valid process," which is always true here. The
    // return value is checked before any further action.
    let traceme_result = unsafe { libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0) };

    if traceme_result == -1 {
        // Something is already tracing this process.
        mark_tampered();
        return;
    }

    // We successfully became our own tracee -- detach immediately so the
    // process continues running normally rather than staying traced.
    // Safety: PTRACE_DETACH on self, immediately after a successful
    // PTRACE_TRACEME, is the documented, correct way to release it.
    unsafe {
        libc::ptrace(libc::PTRACE_DETACH, 0, 0, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_does_not_panic_when_undebugged() {
        // NF6: this crate's own code must never panic. Under a normal
        // `cargo test` run (no debugger attached), this should complete
        // the trace-then-detach dance without incident.
        check();
    }
}
