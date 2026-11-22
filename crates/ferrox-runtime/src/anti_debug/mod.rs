//! F1.6/F1.7: anti-debug checks. Three independent, non-adjacent checks,
//! each its own public function rather than funneled through one
//! `is_debugged()` aggregator (F1.7 explicitly calls out centralization
//! as something to avoid) -- a consuming crate is expected to call these
//! from different, unrelated call sites scattered through its own logic
//! (demonstrated concretely in `examples/license-checker`, Phase 6), not
//! all from one spot.
//!
//! Each check, on firing, calls `crate::mark_tampered()` directly rather
//! than only returning a value -- so even a caller that ignores the
//! outcome still triggers Layer 3's response (F1.5: silent corruption, no
//! crash, no message).

#[cfg(target_os = "linux")]
pub mod linux;
pub mod timing;
#[cfg(target_os = "windows")]
pub mod windows;
