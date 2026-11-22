//! `cargo-ferrox` entry point.

mod cli;
mod commands;
mod error;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let raw_args: Vec<String> = std::env::args().collect();
    // Strip cargo's re-passed subcommand name (see cli.rs's module docs)
    // when invoked as `cargo ferrox ...` rather than directly.
    let args: Vec<String> = if raw_args.len() > 1 && raw_args[1] == "ferrox" {
        std::iter::once(raw_args[0].clone())
            .chain(raw_args.iter().skip(2).cloned())
            .collect()
    } else {
        raw_args
    };

    let cli = Cli::parse_from(args);

    let result = match &cli.command {
        Commands::Build(build_args) => commands::build::run(build_args),
    };

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
