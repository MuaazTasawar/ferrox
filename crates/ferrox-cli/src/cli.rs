//! Command-line argument definitions for `cargo-ferrox`.
//!
//! Cargo invokes a cargo subcommand binary named `cargo-<name>` as
//! `cargo-ferrox ferrox <rest of args...>` -- the subcommand's own name is
//! re-passed as argv[1] by cargo's own dispatch convention, so `main.rs`
//! strips a single leading `"ferrox"` token before handing the rest to
//! clap. This lets the same binary also be run directly during development
//! (`cargo run -p ferrox-cli -- build`) without that extra token, since
//! direct invocation never prepends it.

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "ferrox", version, about = "Ferrox binary hardening pipeline")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Build the target crate through the Ferrox pipeline.
    Build(BuildArgs),
}

#[derive(Args, Debug)]
pub struct BuildArgs {
    /// Build in release mode (passed through to `cargo build --release`).
    #[arg(long)]
    pub release: bool,

    /// Which workspace member to build. Required if the workspace has more
    /// than one member and none is unambiguous.
    #[arg(short, long)]
    pub package: Option<String>,

    /// NF8: force deterministic key generation (same keys across builds),
    /// for diagnosing bugs in the pipeline itself where varying keys
    /// between runs would make a bug impossible to reproduce.
    #[arg(long)]
    pub deterministic: bool,

    /// NF8: pick a specific seed under --deterministic. Has no effect, and
    /// is rejected, without --deterministic.
    #[arg(long)]
    pub seed: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_build_invocation() {
        let cli = Cli::parse_from(["ferrox", "build"]);
        let Commands::Build(args) = cli.command;
        assert!(!args.release);
        assert!(args.package.is_none());
        assert!(!args.deterministic);
        assert!(args.seed.is_none());
    }

    #[test]
    fn parses_full_build_invocation() {
        let cli = Cli::parse_from([
            "ferrox",
            "build",
            "--release",
            "--package",
            "license-checker",
            "--deterministic",
            "--seed",
            "42",
        ]);
        let Commands::Build(args) = cli.command;
        assert!(args.release);
        assert_eq!(args.package.as_deref(), Some("license-checker"));
        assert!(args.deterministic);
        assert_eq!(args.seed, Some(42));
    }
}
