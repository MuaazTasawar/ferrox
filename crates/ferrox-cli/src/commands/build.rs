//! `cargo ferrox build` -- orchestrates a build of the target crate.
//!
//! ## Current scope, stated honestly
//! `ferrox-mir-pass` (Layer 2, Phase 7) does not exist yet, so this command
//! cannot inject `rustc_driver` as a custom compiler front-end. What it
//! DOES do today:
//! - Resolves the target package via `cargo_metadata`, so `--package`
//!   behaves like cargo's own `-p` -- validated against the workspace's
//!   actual member list before shelling out, rather than letting a typo
//!   fail deep inside a `cargo build` subprocess with a less helpful error.
//! - Implements NF8's `--deterministic`/`--seed` flags for real, by setting
//!   `FERROX_DETERMINISTIC`/`FERROX_SEED` on the **child** `cargo build`
//!   process's environment only (never this process's own environment --
//!   same reasoning as `ferrox-keygen`'s pure-core/env-wrapper split: it
//!   keeps the behavior testable and avoids mutating shared state). Any
//!   crate in the build graph whose `build.rs` calls `ferrox_keygen::run()`
//!   picks these up automatically.
//! - Shells out to a normal `cargo build`.
//!
//! ## Known gap, stated honestly (post-link checksum patching)
//! `ferrox-runtime`'s checksum thread (Phase 4) uses a trust-on-first-run
//! baseline specifically because a true build-time-embedded checksum needs
//! a post-link patching step over the compiled binary: rewriting a
//! placeholder constant with the real hash of the final linked artifact.
//! That requires binary-format parsing (ELF on Linux, PE on Windows) to
//! locate the placeholder -- a new dependency (e.g. the `object` crate) not
//! yet pulled into this workspace. **Not implemented by this phase.**
//! Flagged here explicitly, and in `docs/THREAT_MODEL.md`'s Layer 3
//! section, as a tracked gap rather than silently dropped.

use std::process::Command;

use crate::cli::BuildArgs;
use crate::error::FerroxError;

pub fn run(args: &BuildArgs) -> Result<(), FerroxError> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .map_err(FerroxError::Metadata)?;

    let target_package = resolve_package(&metadata, args.package.as_deref())?;

    if args.seed.is_some() && !args.deterministic {
        return Err(FerroxError::SeedWithoutDeterministic);
    }

    eprintln!(
        "ferrox: building package `{target_package}` (ferrox-mir-pass not yet \
         wired in -- see Phase 7; this build currently applies only whatever \
         Layer 1/3 calls the target crate's own code makes)"
    );

    let mut cmd = Command::new("cargo");
    cmd.arg("build");

    if args.release {
        cmd.arg("--release");
    }
    cmd.arg("-p").arg(&target_package);

    if args.deterministic {
        cmd.env("FERROX_DETERMINISTIC", "1");
        if let Some(seed) = args.seed {
            cmd.env("FERROX_SEED", seed.to_string());
        }
    }

    let status = cmd.status().map_err(FerroxError::Spawn)?;
    if !status.success() {
        let detail = match status.code() {
            Some(code) => format!("exit code {code}"),
            None => "terminated by signal".to_string(),
        };
        return Err(FerroxError::CargoBuildFailed { detail });
    }

    eprintln!(
        "ferrox: build succeeded. NOTE: post-link checksum patching is not \
         yet implemented (tracked gap, see docs/THREAT_MODEL.md's Layer 3 \
         section) -- ferrox-runtime's checksum thread is still running in \
         trust-on-first-run mode for this binary."
    );

    Ok(())
}

/// Resolves which package to build: `--package <name>` if given (validated
/// against the workspace's actual member list), the workspace's sole
/// member if there is exactly one and none was named, or an explicit
/// "ambiguous, pass --package" error if the workspace has multiple
/// members and none was named.
fn resolve_package(
    metadata: &cargo_metadata::Metadata,
    requested: Option<&str>,
) -> Result<String, FerroxError> {
    let members: Vec<String> = metadata
        .workspace_packages()
        .iter()
        .map(|p| p.name.clone())
        .collect();

    match requested {
        Some(name) => {
            if members.iter().any(|m| m == name) {
                Ok(name.to_string())
            } else {
                Err(FerroxError::UnknownPackage {
                    requested: name.to_string(),
                    available: members.join(", "),
                })
            }
        }
        None => match members.as_slice() {
            [] => Err(FerroxError::NoWorkspaceMembers),
            [only] => Ok(only.clone()),
            many => Err(FerroxError::AmbiguousPackage {
                available: many.join(", "),
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Not a mock -- `cargo_metadata::Metadata` has no public constructor,
    /// so these tests exercise `resolve_package`'s logic against *this*
    /// real workspace's real metadata, asserting against package names we
    /// know exist in it (e.g. `ferrox-strings`).
    fn this_workspace_metadata() -> cargo_metadata::Metadata {
        cargo_metadata::MetadataCommand::new()
            .exec()
            .expect("cargo metadata must succeed when running ferrox-cli's own test suite")
    }

    #[test]
    fn resolve_package_finds_a_real_known_member() {
        let metadata = this_workspace_metadata();
        let resolved = resolve_package(&metadata, Some("ferrox-strings"));
        assert_eq!(resolved.unwrap(), "ferrox-strings");
    }

    #[test]
    fn resolve_package_errors_clearly_on_unknown_name() {
        let metadata = this_workspace_metadata();
        let err = resolve_package(&metadata, Some("not-a-real-crate-xyz")).unwrap_err();
        match err {
            FerroxError::UnknownPackage { requested, available } => {
                assert_eq!(requested, "not-a-real-crate-xyz");
                assert!(available.contains("ferrox-strings"));
            }
            other => panic!("expected UnknownPackage, got {other:?}"),
        }
    }

    #[test]
    fn resolve_package_is_ambiguous_without_a_name_in_this_multi_member_workspace() {
        let metadata = this_workspace_metadata();
        let err = resolve_package(&metadata, None).unwrap_err();
        assert!(matches!(err, FerroxError::AmbiguousPackage { .. }));
    }
}
