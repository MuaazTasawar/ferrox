# Ferrox Threat Model

**Status:** In progress -- expanded per phase, finalized before v1.0 (F4.4).

## Purpose and Legitimate Use

Ferrox raises the cost and time required to reverse-engineer a compiled Rust
binary. It is intended for protecting **your own shipped binaries**:
software licensing checks, anti-cheat logic, and intellectual-property
protection for commercial Rust software.

Ferrox is **not unbreakable**. No binary-hardening tool is. A sufficiently
resourced, patient attacker can defeat any layer here given enough time.
The goal is cost, not impossibility -- and that claim is made explicitly,
here, on day one, rather than implied otherwise.

Ferrox is **not a malware-packing tool**. The techniques below (string
encryption, anti-debug, control-flow obfuscation, virtualization) are
dual-use by nature -- the same techniques appear in both legitimate
commercial protectors (Themida, VMProtect, Denuvo) and in malware. Ferrox's
scope, documentation, and default behavior are built around protecting a
binary you own and ship, not evading detection of malicious code. Anyone
using Ferrox to obfuscate malicious software is acting outside its intended
and documented use.

Ferrox is **not cross-language** -- Rust only, for v1.

## Threat Coverage

| Threat | In scope | Mitigation layer |
|---|---|---|
| Casual static analysis (`strings`, basic disassembly) | Yes | Layer 1 (string encryption) |
| Automated decompilers (Ghidra, IDA) producing clean pseudocode | Yes | Layer 2 (control-flow flattening), Layer 4 (virtualization) |
| Live debugging / dynamic analysis (gdb, x64dbg) | Yes | Layer 3 (anti-debug) |
| Binary patching (NOP'ing a check) | Yes | Layer 3 (self-checksum, silent corruption on tamper) |
| One cracked binary's patch applied to all copies | Yes | `ferrox-keygen` (per-build uniqueness) |
| A skilled, well-resourced reverse engineer with unlimited time | **No** -- explicitly out of scope | -- |
| Side-channel attacks (timing, power analysis) | No | -- |
| Supply-chain attacks on the build toolchain itself | No, but flagged here: pin your toolchain, verify `rustup` component hashes, and treat `ferrox-cli` itself as part of your trusted build environment | -- |

## Platform Support (NF9)

v1 targets **x86-64 Linux** and **x86-64 Windows** only. ARM and macOS are
explicitly out of scope until stated otherwise in a future release -- this
is a deliberate boundary, not an oversight.

## Per-Layer Detail

### Layer 1 -- String Encryption (Phases 1-3)

`ferrox-strings` provides RC4-based encryption; `ferrox-macros`' `enc!`
macro is the user-facing entry point, encrypting literals at compile time
and decrypting lazily, once, at first runtime access. What this defends
against: a `strings <binary>` scan, or a quick look in a hex editor,
finding plaintext license-check messages, error strings, or other
revealing literals. What it does **not** defend against: an attacker who
sets a breakpoint on `EncryptedStr::get()` and reads the decrypted value
out of memory at runtime -- that is Layer 3's job (anti-debug) to make
harder, not Layer 1's. RC4 is used knowingly as a weak-but-sufficient
cipher; see "Purpose and Legitimate Use" above.

`ferrox-keygen` (Phase 3) independently satisfies F1.8 (same seed produces
identical keys; random mode produces different keys per call), with an
NF8 deterministic escape hatch via `FERROX_DETERMINISTIC`/`FERROX_SEED`.

**Interim caveat (narrowed as of Phase 3, resolved at Phase 6):** `enc!`
still generates its own independent per-`enc!`-site key rather than
consuming `ferrox-keygen`'s `FERROX_STRING_KEY`. The two are wired
together concretely in Phase 6's `examples/license-checker` -- that is
deliberately where this caveat closes for real, proven end-to-end, rather
than asserted in isolation per-crate.

**Panic hygiene (F1.3)** is part of Layer 1's requirement set but is
implemented alongside the rest of Layer 3 in Phase 4 (same crate,
`ferrox-runtime`, same release-build-only posture) -- detailed here once
that phase lands.

### Layer 3 (Runtime Anti-Tamper / Anti-Debug)

Filled in after Phase 4.

### Layer 2 (MIR Control-Flow Obfuscation)

Filled in after Phase 7.

### Layer 4 (Bytecode Virtualization)

Filled in after Phase 9.

### Red-Team Validation

Filled in after Phase 11, with a link to `docs/BENCHMARKS.md`.

## License and Legitimate-Use Statement (NF10)

Ferrox is MIT-licensed. By using it, you agree it is intended for protecting
software you have the right to protect. See the README's first paragraph
for the same statement, surfaced up front rather than buried here.
