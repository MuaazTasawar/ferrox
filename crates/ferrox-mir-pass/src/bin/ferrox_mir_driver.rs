//! `ferrox-mir-driver` -- a drop-in `rustc` replacement binary.
//!
//! `ferrox-cli` (built on stable, per Phase 5) will eventually point
//! cargo's `RUSTC_WORKSPACE_WRAPPER` at this binary -- the same mechanism
//! `cargo clippy` uses for `clippy-driver` -- so that only this crate,
//! not `ferrox-cli` itself, needs the nightly toolchain. That wiring is
//! step 7f; this binary needs to compile and run standalone first.
//!
//! Step 7a: `FerroxCallbacks` is a no-op passthrough (see `callbacks.rs`),
//! so this binary should currently behave identically to plain `rustc`
//! for any arguments you pass it.
//!
//! API note: this uses `rustc_driver::run_compiler` (a free function) and
//! returns `ExitCode` from `main`, rather than the older
//! `RunCompiler::new(...).run()` + `std::process::exit(i32)` pattern --
//! reconstructed from this nightly's own compiler errors (see commit
//! history/chat log for the exact diagnostics that led here), since
//! `rustc_driver`'s internal API has no stability guarantee and isn't
//! documented for a toolchain this recent.
#![feature(rustc_private)]

extern crate rustc_driver;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut callbacks = ferrox_mir_pass::FerroxCallbacks::default();
    rustc_driver::catch_with_exit_code(|| {
        rustc_driver::run_compiler(&args, &mut callbacks);
    })
}
