//! F1.6, check 3 of 3: timing-based single-step / breakpoint detection.
//!
//! Works on both platforms (no OS-specific API): runs a small, fixed
//! amount of trivial computation and times it with a high-resolution
//! clock. Under normal execution this takes microseconds; under a
//! debugger single-stepping through instructions, or with a breakpoint
//! set inside the timed region, it takes orders of magnitude longer. This
//! is a coarse heuristic (a careful attacker can single-step *around* the
//! timed region) -- again, one of three independent checks, not a
//! standalone guarantee.

use std::time::Instant;

use crate::mark_tampered;

/// Threshold above which elapsed time for the timed region is considered
/// anomalous. Deliberately generous (normal execution should take low
/// single-digit microseconds; this allows wide margin for a loaded CI
/// runner or slow VM) to avoid false positives on legitimate slow
/// hardware -- tightening this is a documented tuning opportunity, not a
/// correctness requirement from the spec.
const ANOMALY_THRESHOLD_MICROS: u128 = 50_000; // 50ms

/// Runs the timing check once. Safe to call from multiple, unrelated call
/// sites (F1.7); cheap enough (~microseconds of real work, plus one
/// `Instant::now()` pair) to scatter liberally through hot paths.
pub fn check() {
    let start = Instant::now();

    // Deliberately trivial, side-effect-free work: the point is to have
    // *something* to time, not to compute anything meaningful.
    let mut acc: u64 = 0;
    for i in 0..10_000u64 {
        acc = acc.wrapping_add(i).wrapping_mul(2654435761);
    }
    std::hint::black_box(acc);

    let elapsed = start.elapsed();
    if elapsed.as_micros() > ANOMALY_THRESHOLD_MICROS {
        mark_tampered();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_does_not_panic_under_normal_conditions() {
        // NF6: must never panic. Under a normal `cargo test` run (no
        // debugger, no single-stepping), elapsed time should be well
        // under the threshold and this should be a no-op.
        check();
    }

    #[test]
    fn threshold_is_generous_enough_for_test_runners() {
        // Sanity check on the constant itself, not a behavioral test:
        // guards against someone tightening ANOMALY_THRESHOLD_MICROS to
        // something that would make CI flaky.
        assert!(ANOMALY_THRESHOLD_MICROS >= 10_000);
    }
}
