# Ferrox Architecture

**Status:** In progress -- expanded as each crate lands, finalized in Phase 12.

## Overview

Ferrox is a Cargo subcommand (`cargo ferrox build`) that wraps a normal Rust
build with a pipeline of hardening passes. It is organized as a Cargo
workspace of single-purpose crates:

| Crate | Role | Linked into target binary? |
|---|---|---|
| `ferrox-cli` | Orchestrates the pipeline: resolves the target package, wires NF8's determinism flags, shells out to `cargo build`. | No |
| `ferrox-macros` | Proc-macros consumed by user code: `ferrox::enc!()`, `#[ferrox::protect]`, `#[ferrox::vm]`. | No (macros only; expands to code that links `ferrox-runtime`/`ferrox-vm`) |
| `ferrox-strings` | Compile-time string encryption, runtime decryption, and shared hex encode/decode helpers (Layer 1). | Yes |
| `ferrox-keygen` | Called from a protected crate's `build.rs`, generates a random per-build seed, emits `keys.rs` and `FERROX_STRING_KEY_HEX`. | No (build-time only; its output is linked via `include!`/the key env var) |
| `ferrox-runtime` | Anti-tamper checksum thread, anti-debug checks, panic hook (Layer 3). | Yes |
| `ferrox-mir-pass` | `rustc_driver::Callbacks` implementation: control-flow flattening + opaque predicates on MIR (Layer 2). | No (compile-time only) |
| `ferrox-vm` | Stack-based bytecode ISA + interpreter; compiler that lowers flattened MIR to bytecode (Layer 4). | Interpreter: yes. Compiler: no. |
| `ferrox-redteam` | Scripted Ghidra/radare2 analysis + scoring, run against build variants. | No -- explicitly never linked into a protected target |

## Build Pipeline (high level)

source crate
|
v
protected crate's build.rs calls ferrox_keygen::run()
--> OUT_DIR/keys.rs AND cargo:rustc-env=FERROX_STRING_KEY_HEX=<hex>
|
v
cargo ferrox build resolves the target package, sets
FERROX_DETERMINISTIC/FERROX_SEED on the child process if requested (NF8),
and shells out to cargo build (today -- Phase 7 adds rustc_driver injection)
| (enc!() calls read FERROX_STRING_KEY_HEX at macro-expansion time -- shared per-build key)
| (functions marked #[ferrox::protect] -> flattened MIR, once Phase 7/8 land)
| (functions marked #[ferrox::vm] -> lowered to bytecode, once Phase 9/10 land)
v
normal codegen, linked against ferrox-runtime + ferrox-strings (+ ferrox-vm interpreter if used)
|
v
hardened binary (checksum baseline still trust-on-first-run -- see
docs/THREAT_MODEL.md's Layer 3 section for the tracked post-link-patching gap)


## Per-Crate Detail

### `ferrox-strings` (Phases 1, 6)

A pure, stateless RC4 cipher (`encrypt`/`decrypt`), a runtime-facing
`EncryptedStr` wrapper (lazy `OnceLock`-cached decryption), and (Phase 6) a
`hex` module (`encode`/`decode`) used to pass binary key material through
`cargo:rustc-env` string values between `ferrox-keygen` and `ferrox-macros`.

### `ferrox-macros` -- `enc!` (Phases 2, 6)

A `#[proc_macro]` that resolves a 32-byte key -- preferring the shared
per-build key from the `FERROX_STRING_KEY_HEX` env var (set by
`ferrox-keygen`'s `run()`, read at macro-expansion time via
`std::env::var`), falling back to an independent per-call-site random key
if that env var is absent or malformed -- then encrypts the literal and
emits a `static ferrox_strings::EncryptedStr` whose `.get()` is the
macro's final expression. `ferrox-macros` and `ferrox-keygen` are
deliberately coupled only by this env var naming convention, not by a
direct crate dependency.

### `ferrox-keygen` (Phases 3, 6)

Build-time-only, called from a protected crate's `build.rs`. `run()`
writes `keys.rs` (seed, 32-byte string key, 8-byte checksum salt -- the
salt is generated but **not yet consumed** by `ferrox-runtime`, stated
plainly in the crate's own docs) and, as of Phase 6, also emits
`cargo:rustc-env=FERROX_STRING_KEY_HEX=<hex>` so `enc!` can pick up a
single, build-coordinated key.

### `ferrox-runtime` (Phase 4, exercised in Phase 6)

Linked directly into the protected binary: silent release-mode panic hook,
a self-checksum background thread (trust-on-first-run baseline), and
three independent anti-debug checks. All three report into one shared
flag read via `tamper_detected() -> bool` -- deliberately inert on its
own; it is the *consuming* crate's job to decide what "corrupt the
output" means, demonstrated concretely in `examples/license-checker`.

### `ferrox-cli` (Phase 5)

A `cargo-ferrox` binary implementing `cargo ferrox build [--release]
[-p <pkg>] [--deterministic] [--seed <u64>]`: resolves the target package
via `cargo_metadata`, wires NF8's determinism flags onto the child `cargo
build` process, and shells out to it. Does not yet inject `ferrox-mir-pass`
as a custom `rustc_driver` front-end (Phase 7).

### `examples/license-checker` (Phase 6)

The first crate that actually wires Layer 1 and Layer 3 together, proving
the whole chain end-to-end rather than per-crate in isolation:

- `build.rs` calls `ferrox_keygen::run()`.
- `src/main.rs` calls `ferrox_runtime::init()` at startup, three `enc!()`
  calls (the one valid license key plus both outcome messages -- all
  sharing one per-build key per the Phase 6 Layer 1 fix), three
  `anti_debug::*::check()` calls at three different, unrelated points in
  `main` (startup, after reading input, around the decision itself --
  F1.7), and a single call site that checks `tamper_detected()`
  immediately before printing the verdict and flips a valid result to
  invalid if tampering was found (F1.5, concretely implemented here, not
  in `ferrox-runtime` itself).
- CI (`.github/workflows/ci.yml`'s `integration` job) builds this crate in
  release mode, functionally checks both the correct-key and wrong-key
  paths, and runs `strings` against the compiled binary asserting none of
  the three `enc!()`-protected literals appear in plaintext -- the F1.2
  requirement, now enforced automatically rather than left as a manual
  check or a TODO.

## Deferred Per-Crate Detail

- `ferrox-mir-pass` -- after Phase 7.
- `ferrox-macros` (`#[ferrox::protect]`) -- after Phase 8.
- `ferrox-vm` -- after Phase 9.
- `ferrox-macros` (`#[ferrox::vm]`) -- after Phase 10.
- `ferrox-redteam` -- after Phase 11.
