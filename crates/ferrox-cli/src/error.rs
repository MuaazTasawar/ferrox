//! `ferrox-cli`'s single error type. Every variant carries pre-formatted
//! `String` fields rather than relying on thiserror's attribute-macro
//! field-expression syntax (e.g. inline `.join()` calls inside `#[error(...)]`)
//! -- formatting happens once, in plain Rust, at the point each error is
//! constructed in `commands/build.rs`. Simpler to read and to get right.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FerroxError {
    #[error("ferrox: failed to read cargo metadata: {0}")]
    Metadata(#[source] cargo_metadata::Error),

    #[error("ferrox: unknown package `{requested}`. Available workspace members: {available}")]
    UnknownPackage {
        requested: String,
        available: String,
    },

    #[error("ferrox: workspace has no members to build")]
    NoWorkspaceMembers,

    #[error("ferrox: multiple workspace members found ({available}); pass --package <name> to choose one")]
    AmbiguousPackage { available: String },

    #[error("ferrox: --seed was given without --deterministic (seed only has an effect in deterministic mode)")]
    SeedWithoutDeterministic,

    #[error("ferrox: failed to spawn `cargo build`: {0}")]
    Spawn(#[source] std::io::Error),

    #[error("ferrox: `cargo build` failed ({detail})")]
    CargoBuildFailed { detail: String },
}
