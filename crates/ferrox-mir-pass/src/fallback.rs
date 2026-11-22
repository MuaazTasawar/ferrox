//! F2.7: source-level fallback. If the MIR-level approach (`flatten.rs`,
//! `opaque_predicates.rs`) proves too unstable against future nightly
//! updates to be relied on, this module is where a source-to-source
//! (pre-MIR, proc-macro-level) equivalent transformation would live
//! instead -- weaker (operates on surface syntax, not the compiler's own
//! CFG representation) but immune to `rustc_private` API churn.
//!
//! **Step 7e addresses this**, once 7a-7d establish whether the MIR
//! approach is viable against the pinned nightly. Stub for step 7a.
