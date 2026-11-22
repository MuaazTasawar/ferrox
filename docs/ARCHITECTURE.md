# Ferrox Architecture

**Status:** In progress -- expanded as each crate lands, finalized in Phase 12.

## Overview

Ferrox is a Cargo subcommand (`cargo ferrox build`) that wraps a normal Rust
build with a pipeline of hardening passes. It is organized as a Cargo
workspace of single-purpose crates:

| Crate | Role | Linked into target binary? |
|---|---|---|
| `ferrox-cli` | Orchestrates the pipeline: invokes keygen, the MIR pass, and the normal `cargo build`/`rustc` steps in order. | No |
| `ferrox-macros` | Proc-macros consumed by user code: `ferrox::enc!()`, `#[ferrox::protect]`, `#[ferrox::vm]`. | No (macros only; expands to code that links `ferrox-runtime`/`ferrox-vm`) |
| `ferrox-strings` | Compile-time string encryption + runtime decryption helpers (Layer 1). | Yes |
| `ferrox-keygen` | Called from a protected crate's `build.rs`, generates a random per-build seed, emits `keys.rs`. | No (build-time only; its output is `include!`'d, so the *constants* end up linked) |
| `ferrox-runtime` | Anti-tamper checksum thread, anti-debug checks, panic hook (Layer 3). | Yes |
| `ferrox-mir-pass` | `rustc_driver::Callbacks` implementation: control-flow flattening + opaque predicates on MIR (Layer 2). | No (compile-time only) |
| `ferrox-vm` | Stack-based bytecode ISA + interpreter; compiler that lowers flattened MIR to bytecode (Layer 4). | Interpreter: yes. Compiler: no. |
| `ferrox-redteam` | Scripted Ghidra/radare2 analysis + scoring, run against build variants. | No -- explicitly never linked into a protected target |

## Build Pipeline (high level)

source crate
|
v
protected crate's build.rs calls ferrox_keygen::run() --> OUT_DIR/keys.rs
|
v
ferrox-cli invokes rustc with ferrox-mir-pass as a custom driver
| (functions marked #[ferrox::protect] -> flattened MIR)
| (functions marked #[ferrox::vm] -> lowered to bytecode, see ferrox-vm)
v
normal codegen, linked against ferrox-runtime + ferrox-strings (+ ferrox-vm interpreter if used),
consuming OUT_DIR/keys.rs via include!()
|
v
hardened binary


## Per-Crate Detail

### `ferrox-strings` (Phase 1)

A pure, stateless RC4 cipher (`encrypt`/`decrypt`) plus a runtime-facing
wrapper, `EncryptedStr`, built from `'static` ciphertext/key slices.
`EncryptedStr::new` is `const fn`; decryption happens lazily inside a
`OnceLock` on first `.get()` call and is cached for the process lifetime.

### `ferrox-macros` -- `enc!` (Phase 2)

A `#[proc_macro]` that encrypts a string literal at macro-expansion time
(host-side RC4), then emits a `static ferrox_strings::EncryptedStr` whose
`.get()` is the macro's final expression.

### `ferrox-keygen` (Phase 3)

Build-time-only, called from a protected crate's `build.rs`. Pure core
(`generate_with_mode`) is unit-tested without touching process env;
`run()` reads `OUT_DIR` and writes `keys.rs` (seed, 32-byte string key,
8-byte checksum salt, all derived from one seed via a seeded `StdRng`).
`FERROX_DETERMINISTIC`/`FERROX_SEED` env vars satisfy NF8 pending a real
CLI flag in Phase 5.

### `ferrox-runtime` (Phase 4)

Linked directly into the protected binary. Three independent pieces:

- `panic_hook::install()` -- silent panic hook, release builds only.
- `checksum::start()` -- background thread, 2-5s randomized interval,
  re-hashes the process's own loaded code region (FNV-1a over a region
  located via `/proc/self/maps` on Linux or `GetModuleHandleW` +
  `GetModuleInformation` on Windows) and compares against a baseline
  computed on first run (trust-on-first-run; see
  `docs/THREAT_MODEL.md`'s Layer 3 section for the documented limitation
  versus a true build-time-embedded hash).
- `anti_debug::{linux, windows, timing}::check()` -- three independent
  functions (not one aggregator, per F1.7), meant to be called from
  scattered, unrelated sites in the consuming crate.

All three report into one shared flag via `crate::mark_tampered()`, read
back by consumers through `tamper_detected() -> bool`. `ferrox-runtime`
itself never corrupts the protected crate's output -- that is the
consuming crate's responsibility at whatever call site fits its own
logic, demonstrated concretely in `examples/license-checker` (Phase 6).
`#![deny(clippy::unwrap_used)]` at the crate root is the automated half
of NF6's "no panics" audit.

## Deferred Per-Crate Detail

- `ferrox-cli` -- after Phase 5.
- `ferrox-mir-pass` -- after Phase 7.
- `ferrox-macros` (`#[ferrox::protect]`) -- after Phase 8.
- `ferrox-vm` -- after Phase 9.
- `ferrox-macros` (`#[ferrox::vm]`) -- after Phase 10.
- `ferrox-redteam` -- after Phase 11.
