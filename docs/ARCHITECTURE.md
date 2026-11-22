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
cargo ferrox build resolves the target package, sets
FERROX_DETERMINISTIC/FERROX_SEED on the child process if requested (NF8),
and shells out to cargo build (today -- Phase 7 adds rustc_driver injection)
| (functions marked #[ferrox::protect] -> flattened MIR, once Phase 7/8 land)
| (functions marked #[ferrox::vm] -> lowered to bytecode, once Phase 9/10 land)
v
normal codegen, linked against ferrox-runtime + ferrox-strings (+ ferrox-vm interpreter if used),
consuming OUT_DIR/keys.rs via include!()
|
v
hardened binary (checksum baseline still trust-on-first-run -- see
docs/THREAT_MODEL.md's Layer 3 section for the tracked post-link-patching gap)


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

### `ferrox-runtime` (Phase 4)

Linked directly into the protected binary: silent release-mode panic hook,
a self-checksum background thread (trust-on-first-run baseline -- see the
threat model for the documented gap versus a true build-time-embedded
hash), and three independent anti-debug checks (`ptrace` self-trace on
Linux, `IsDebuggerPresent` on Windows, timing-anomaly on both). All three
report into one shared flag read via `tamper_detected() -> bool`.

### `ferrox-cli` (Phase 5)

A `cargo-ferrox` binary (cargo subcommand convention: `main.rs` strips the
leading `"ferrox"` token cargo re-passes as argv[1] before handing the
rest to `clap`). `cargo ferrox build [--release] [-p <pkg>] [--deterministic]
[--seed <u64>]`:

1. Reads `cargo_metadata` and resolves the target package -- `--package`
   is validated against real workspace members (clear error on a typo);
   with no `--package`, a single-member workspace is inferred, a
   multi-member workspace without one errors asking for `--package`.
2. Implements NF8 for real: `--deterministic` sets
   `FERROX_DETERMINISTIC=1` on the **child** `cargo build` process only
   (never mutates this process's own environment, for the same testability
   reasons as `ferrox-keygen`); `--seed <n>` additionally sets
   `FERROX_SEED=<n>`, and is rejected if given without `--deterministic`.
3. Shells out to `cargo build` with those env vars and the resolved
   package/release flag.

**Honest scope note:** this phase does not yet inject `ferrox-mir-pass` as
a custom `rustc_driver` front-end -- that crate doesn't exist until
Phase 7. Today, `cargo ferrox build` is meaningfully different from plain
`cargo build` only in (a) NF8's real determinism flags and (b) the
package-resolution error messages; Layer 2/4 instrumentation is not yet
wired into the pipeline. This is stated in the CLI's own `eprintln!`
output at build time, not just in docs.

**Also not yet done (post-link checksum patching):** `ferrox-runtime`'s
checksum baseline is still trust-on-first-run, as of this phase, because
embedding a true build-time hash requires parsing the compiled binary's
format (ELF/PE) to patch in a placeholder -- a dependency not yet added to
the workspace. Tracked explicitly in `docs/THREAT_MODEL.md`, not silently
dropped.

## Deferred Per-Crate Detail

- `ferrox-mir-pass` -- after Phase 7.
- `ferrox-macros` (`#[ferrox::protect]`) -- after Phase 8.
- `ferrox-vm` -- after Phase 9.
- `ferrox-macros` (`#[ferrox::vm]`) -- after Phase 10.
- `ferrox-redteam` -- after Phase 11.
