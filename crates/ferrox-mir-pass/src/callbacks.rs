//! `rustc_driver::Callbacks` implementation.
//!
//! Step 7b-i: overrides the `optimized_mir` query provider so
//! `flatten::maybe_flatten` runs on every function body the compiler
//! optimizes -- currently an identity transform (see `flatten.rs`), so
//! this step only proves the hook itself compiles and is wired at the
//! right point, not that any real obfuscation happens yet.
//!
//! API notes, corrected against this nightly's real compiler errors:
//! - The provider field is `providers.queries.optimized_mir`, not
//!   `providers.optimized_mir` directly.
//! - `rustc_mir_transform::provide_default_optimized_mir` does not exist
//!   as public API -- the real `optimized_mir` implementation is private
//!   to that crate. The default provider is instead reached through
//!   `rustc_interface::DEFAULT_QUERY_PROVIDERS`, a static table of the
//!   unmodified default providers that `rustc_interface` exposes
//!   specifically so tools like this one can wrap one query's default
//!   behavior without reimplementing it.

use rustc_driver::Callbacks;
use rustc_interface::interface;
use rustc_interface::DEFAULT_QUERY_PROVIDERS;
use rustc_middle::mir::Body;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::LocalDefId;

use crate::flatten;

#[derive(Default)]
pub struct FerroxCallbacks;

impl Callbacks for FerroxCallbacks {
    fn config(&mut self, config: &mut interface::Config) {
        config.override_queries = Some(|_session, providers| {
            providers.queries.optimized_mir = ferrox_optimized_mir;
        });
    }
}

/// Our replacement for the default `optimized_mir` provider: calls the
/// real default provider first (via `DEFAULT_QUERY_PROVIDERS`, so we
/// start from a fully-optimized body, same as the default pipeline), then
/// runs `flatten::maybe_flatten` on the result before handing it back.
fn ferrox_optimized_mir<'tcx>(tcx: TyCtxt<'tcx>, def_id: LocalDefId) -> &'tcx Body<'tcx> {
    let body = (DEFAULT_QUERY_PROVIDERS.queries.optimized_mir)(tcx, def_id);
    // `optimized_mir` returns a `&'tcx Body`, allocated in the tcx arena --
    // we can't mutate it in place through a shared reference, so clone it,
    // mutate the clone, and re-allocate into the arena. Not how a
    // production pass would do this long-term, but the simplest correct
    // starting point for proving the transform logic.
    let mut owned = body.clone();
    flatten::maybe_flatten(tcx, &mut owned);
    tcx.arena.alloc(owned)
}
