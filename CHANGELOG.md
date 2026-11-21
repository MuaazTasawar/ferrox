# Changelog

All notable changes to Ferrox are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
versioning follows [Semantic Versioning](https://semver.org/) (NF5).

## [Unreleased]

### Added
- Workspace scaffold: `ferrox-cli`, `ferrox-macros`, `ferrox-strings`,
  `ferrox-keygen`, `ferrox-runtime`, `ferrox-mir-pass`, `ferrox-vm`,
  `ferrox-redteam` crates.
- Pinned nightly toolchain (`rust-toolchain.toml`) for future `rustc_driver`
  work in `ferrox-mir-pass`.
- CI pipeline: build, test, clippy (`-D warnings`), fmt check.
- Initial `docs/THREAT_MODEL.md` and `docs/ARCHITECTURE.md`.

### Non-Goals (reaffirmed from day one, see docs/THREAT_MODEL.md)
- Not unbreakable -- raises cost, not impossibility.
- Not a malware-packing tool -- targets protecting your own shipped binary.
- Not cross-language -- Rust only, v1.
