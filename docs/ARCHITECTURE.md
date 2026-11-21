# Ferrox Architecture

**Status:** Stub -- expanded as each crate lands, finalized in Phase 12.

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

Filled in at the end of the phase that implements each crate -- kept close
to the real shipped interfaces rather than speculative up front.

- `ferrox-strings` -- after Phase 1.
- `ferrox-macros` (`enc!`) -- after Phase 2.
- `ferrox-keygen` -- after Phase 3.
- `ferrox-runtime` -- after Phase 4.
- `ferrox-cli` -- after Phase 5.
- `ferrox-mir-pass` -- after Phase 7.
- `ferrox-macros` (`#[ferrox::protect]`) -- after Phase 8.
- `ferrox-vm` -- after Phase 9.
- `ferrox-macros` (`#[ferrox::vm]`) -- after Phase 10.
- `ferrox-redteam` -- after Phase 11.
