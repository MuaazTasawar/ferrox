//! Control-flow flattening (Layer 2's core transformation).
//!
//! **Step 7b-i (this version):** identity transform only -- logs that it
//! was invoked (gated behind `FERROX_MIR_DEBUG=1` so normal use is quiet)
//! and returns the body completely unchanged. This exists purely to prove
//! the `override_queries` hook in `callbacks.rs` actually compiles against
//! this nightly's real `optimized_mir` provider signature, before any real
//! flattening logic is written on top of it. The real algorithm (rewriting
//! the CFG into a dispatch-loop shape driven by a synthetic state
//! variable) lands in step 7b-ii, once this plumbing is confirmed.

use rustc_middle::mir::Body;
use rustc_middle::ty::TyCtxt;

/// Step 7b-i: no-op. Takes and returns the body unchanged, logging the
/// call when `FERROX_MIR_DEBUG=1` is set in the environment.
pub fn maybe_flatten<'tcx>(tcx: TyCtxt<'tcx>, body: &mut Body<'tcx>) {
    if std::env::var("FERROX_MIR_DEBUG").as_deref() == Ok("1") {
        eprintln!(
            "ferrox-mir-pass: flatten::maybe_flatten invoked for {:?} (identity transform, step 7b-i)",
            tcx.def_path_str(body.source.def_id())
        );
    }
}
