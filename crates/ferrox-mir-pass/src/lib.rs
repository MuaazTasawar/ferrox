//! `ferrox-mir-pass` -- Layer 2 of Ferrox: MIR-level control-flow
//! flattening and opaque predicates, implemented as a custom `rustc`
//! front-end via the nightly-only, unstable `rustc_private` APIs.
//!
//! ## Honest risk statement (see rust-toolchain.toml and docs/THREAT_MODEL.md)
//! `rustc_driver`/`rustc_interface`/`rustc_middle` are internal compiler
//! APIs with no stability guarantee -- they can change between nightly
//! dates, which is exactly why `rust-toolchain.toml` pins a specific
//! nightly rather than floating. This crate is being built incrementally,
//! in small steps, each verified against the real pinned toolchain before
//! the next is added (step 7a: this scaffold compiles and runs as a
//! passthrough `rustc`; 7b: real flattening logic; 7c: opaque predicates;
//! 7d: wiring the transforms into the compilation pipeline; 7e: the F2.7
//! source-level fallback for when the MIR approach proves too unstable).
#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

pub mod callbacks;
pub mod fallback;
pub mod flatten;
pub mod opaque_predicates;

pub use callbacks::FerroxCallbacks;
