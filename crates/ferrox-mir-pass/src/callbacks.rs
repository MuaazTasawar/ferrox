//! `rustc_driver::Callbacks` implementation.
//!
//! Step 7a: a no-op passthrough. Its only job right now is to prove this
//! crate links against the pinned nightly's `rustc_driver`/
//! `rustc_interface` and runs as a drop-in `rustc` replacement without
//! altering compiler behavior at all. MIR flattening (`flatten.rs`) and
//! opaque predicates (`opaque_predicates.rs`) are wired in at step 7d, via
//! `rustc_interface::Config::override_queries` hooking the `optimized_mir`
//! query -- once this scaffold is confirmed working.

use rustc_driver::Callbacks;

#[derive(Default)]
pub struct FerroxCallbacks;

impl Callbacks for FerroxCallbacks {
    // Deliberately empty: the default trait method bodies already
    // implement "compile exactly as rustc normally would." Overriding
    // nothing is the correct step-7a behavior.
}
