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

Two layers: a pure, stateless RC4 cipher (`encrypt::rc4_apply` /
`encrypt::encrypt_literal`, `decrypt::decrypt_bytes` /
`decrypt::decrypt_to_string`) and a runtime-facing wrapper,
`EncryptedStr`, built from a `'static` ciphertext slice and a `'static`
key slice. `EncryptedStr::new` is `const fn`, so it can initialize a
`static`; decryption happens lazily inside `OnceLock` on first `.get()`
call and is cached for the process lifetime. RC4 is deliberately
not cryptographically strong against a dedicated attacker -- the goal
(per `THREAT_MODEL.md`) is keeping literals out of `strings <binary>`
output, not providing secrecy.

### `ferrox-macros` -- `enc!` (Phase 2)

`enc!("literal")` is a `#[proc_macro]` (not `macro_rules!`, since it needs
to run arbitrary host-side code -- RC4 encryption -- at macro-expansion
time). It: (1) parses the input as a string literal, (2) generates a
random 32-byte key via `rand::thread_rng()` *on the host, at compile
time*, (3) encrypts the literal with `ferrox_strings::encrypt::encrypt_literal`,
(4) emits both the ciphertext and key as `static` byte-array literals,
wrapped in a `static ferrox_strings::EncryptedStr`, whose `.get()` is
the macro's final expression.

### `ferrox-keygen` (Phase 3)

A build-time-only helper, called from a protected crate's `build.rs`
(not from `ferrox-cli` directly, and not linked into the target). Split
into a pure core (`generate_with_mode(deterministic, seed_override)`,
`derive_from_seed`) and a thin env-reading wrapper (`generate_from_env`,
`run`) -- the pure core is what's unit-tested, so tests never mutate
process-global environment variables under `cargo test`'s parallel runner.
`run()` reads `OUT_DIR`, generates a `FerroxKeys` (seed + 32-byte string
key + 8-byte checksum salt, all derived from one seed via a seeded
`StdRng`), and writes `keys.rs` -- a handful of `pub const` declarations
meant to be pulled in with `include!(concat!(env!("OUT_DIR"), "/keys.rs"))`.

Determinism (NF8) is an env-var escape hatch today
(`FERROX_DETERMINISTIC=1`, optionally `FERROX_SEED=<u64>`) pending
`ferrox-cli` (Phase 5) exposing it as a real `--deterministic` flag.

**Integration note:** `ferrox-keygen` exists and is fully tested as of
this phase, but `enc!` (Phase 2) still generates its own independent
per-site key rather than consuming `keys.rs`'s `FERROX_STRING_KEY`, and
`ferrox-runtime`'s checksum thread (Phase 4, not yet built) will be the
first real consumer of `FERROX_CHECKSUM_SALT`. The concrete wiring of a
crate's `build.rs` + `keys.rs` + `enc!`/runtime all together happens in
Phase 6's `examples/license-checker` -- that is deliberately where this
gets proven end-to-end, not asserted in isolation per-crate.

## Deferred Per-Crate Detail

- `ferrox-runtime` -- after Phase 4.
- `ferrox-cli` -- after Phase 5.
- `ferrox-mir-pass` -- after Phase 7.
- `ferrox-macros` (`#[ferrox::protect]`) -- after Phase 8.
- `ferrox-vm` -- after Phase 9.
- `ferrox-macros` (`#[ferrox::vm]`) -- after Phase 10.
- `ferrox-redteam` -- after Phase 11.
