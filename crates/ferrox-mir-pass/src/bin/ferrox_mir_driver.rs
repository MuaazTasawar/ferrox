//! `ferrox-mir-driver` -- a drop-in `rustc` replacement binary, usable
//! two ways:
//! 1. Invoked directly (step 7f-i's smoke test): `ferrox-mir-driver.exe
//!    <normal rustc args>` -- argv[0] is this binary, argv[1..] are
//!    already real rustc flags.
//! 2. Invoked via cargo's `RUSTC_WORKSPACE_WRAPPER` (step 8b's
//!    discrimination test and beyond): cargo calls
//!    `<wrapper> <rustc-path-or-name> <normal rustc args...>`. Confirmed
//!    empirically across two different real invocations that cargo makes
//!    through this mechanism: one call had argv[1] as the literal string
//!    `"rustc"`, another had it as the full resolved path to the real
//!    `rustc.exe`. Rather than matching on either specific value (which
//!    only covers the cases we happened to observe), we strip exactly one
//!    leading argument whenever this is invoked as a wrapper -- consistent
//!    with cargo's own documented wrapper protocol, which always inserts
//!    the wrapped tool's identifier as the first argument before the real
//!    compiler flags, regardless of what that identifier's value is.
//!
//! Detection: when run directly (case 1), argv[1] (if present at all) is
//! the start of real rustc flags, which always begin with `-` (e.g.
//! `--edition`, `--crate-type`) or are a bare source file path containing
//! a path separator or `.rs` extension -- never a bare word like `rustc`
//! or an absolute path ending in `rustc.exe`/`rustc`. So: if argv[1]
//! itself, as a whole word, is exactly `"rustc"` OR its file name (last
//! path component) is `"rustc"` or `"rustc.exe"`, treat it as the
//! wrapper-inserted identifier and strip it.
#![feature(rustc_private)]

extern crate rustc_driver;

use std::process::ExitCode;

fn looks_like_wrapper_identifier(arg: &str) -> bool {
    let file_name = arg.rsplit(['/', '\\']).next().unwrap_or(arg);
    file_name == "rustc" || file_name == "rustc.exe"
}

fn main() -> ExitCode {
    let raw_args: Vec<String> = std::env::args().collect();
    let args: Vec<String> = if raw_args.len() > 1 && looks_like_wrapper_identifier(&raw_args[1]) {
        std::iter::once(raw_args[0].clone())
            .chain(raw_args.iter().skip(2).cloned())
            .collect()
    } else {
        raw_args
    };

    let mut callbacks = ferrox_mir_pass::FerroxCallbacks::default();
    rustc_driver::catch_with_exit_code(|| {
        rustc_driver::run_compiler(&args, &mut callbacks);
    })
}
