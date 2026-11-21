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
| `ferrox-keygen` | Runs in `build.rs`, generates a random per-build seed, emits `keys.rs`. | No (build-time only; its output is linked) |
| `ferrox-runtime` | Anti-tamper checksum thread, anti-debug checks, panic hook (Layer 3). | Yes |
| `ferrox-mir-pass` | `rustc_driver::Callbacks` implementation: control-flow flattening + opaque predicates on MIR (Layer 2). | No (compile-time only) |
| `ferrox-vm` | Stack-based bytecode ISA + interpreter; compiler that lowers flattened MIR to bytecode (Layer 4). | Interpreter: yes. Compiler: no. |
| `ferrox-redteam` | Scripted Ghidra/radare2 analysis + scoring, run against build variants. | No -- explicitly never linked into a protected target |

## Build Pipeline (high level)

source crate
|
v
ferrox-keygen (build.rs) --> keys.rs (per-build random constants)
|
v
ferrox-cli invokes rustc with ferrox-mir-pass as a custom driver
| (functions marked #[ferrox::protect] -> flattened MIR)
| (functions marked #[ferrox::vm] -> lowered to bytecode, see ferrox-vm)
v
normal codegen, linked against ferrox-runtime + ferrox-strings (+ ferrox-vm interpreter if used)
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

`enc!("literal")` is a `#[proc_macro]` (not macro_rules!, since it needs
to run arbitrary host-side code -- RC4 encryption -- at macro-expansion
time). It: (1) parses the input as a string literal, (2) generates a
random 32-byte key via `rand::thread_rng()` *on the host, at compile
time*, (3) encrypts the literal with `ferrox_strings::encrypt::encrypt_literal`,
(4) emits both the ciphertext and key as `static` byte-array literals,
wrapped in a `static ferrox_strings::EncryptedStr`, whose `.get()` is
the macro's final expression. Because expansion emits an absolute path
(`::ferrox_strings::EncryptedStr`), any crate using `enc!` must depend
on `ferrox-strings` directly. Interim limitation, by design: each
`enc!` site currently gets its own independently-random key, not a
shared per-build seed -- Phase 3's `ferrox-keygen` changes *where the
key bytes come from*, not the macro's expansion shape.

## Deferred Per-Crate Detail

- `ferrox-keygen` -- after Phase 3.
- `ferrox-runtime` -- after Phase 4.
- `ferrox-cli` -- after Phase 5.
- `ferrox-mir-pass` -- after Phase 7.
- `ferrox-macros` (`#[ferrox::protect]`) -- after Phase 8.
- `ferrox-vm` -- after Phase 9.
- `ferrox-macros` (`#[ferrox::vm]`) -- after Phase 10.
- `ferrox-redteam` -- after Phase 11.
