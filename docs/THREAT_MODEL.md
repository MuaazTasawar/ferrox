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
macro is the user-facing entry point. Defends against `strings <binary>`
scans and casual hex-editor inspection; does **not** defend against a
breakpoint on `EncryptedStr::get()` reading decrypted memory at runtime --
that is Layer 3's job. **Interim caveat** (resolved at Phase 6): `enc!`
still generates its own independent per-site key rather than consuming
`ferrox-keygen`'s output.

### Layer 3 -- Runtime Anti-Tamper / Anti-Debug (Phase 4)

Panic hygiene (silent release-mode panic hook), a self-checksum background
thread, and three independent anti-debug checks (`ptrace` self-trace on
Linux, `IsDebuggerPresent` on Windows, timing-anomaly on both), all
reporting into one shared `tamper_detected()` flag.

**Known limitation, now tracked across two phases:**
The checksum baseline is **trust-on-first-run**, not a true
build-time-embedded hash. A production-grade implementation computes the
hash *after* final linking and patches it into the binary post-build
(the hash of a section can't be known until the bytes exist, post-link).
That patching step requires parsing the compiled binary's format (ELF on
Linux, PE on Windows) to locate a placeholder -- which means a new
dependency (e.g. the `object` crate) not yet in this workspace.

- As of **Phase 4**: trust-on-first-run shipped as the interim; this gap
  was first stated here.
- As of **Phase 5**: `ferrox-cli`'s `build` command was the natural place
  for this post-link step to live (it already orchestrates the build), but
  it does **not** implement it -- `commands/build.rs` states this
  explicitly in both its module docs and its own `eprintln!` output after
  a successful build, so a user running `cargo ferrox build` is told, not
  left to assume more than what shipped.
- **Practical effect, stated plainly:** Ferrox v1, as built through
  Phase 5, defeats a debugger attaching mid-run and patching a check in
  place; it does **not** defeat an attacker who statically patches the
  binary before it is ever executed (the baseline would simply reflect the
  already-patched bytes). Closing this gap is not currently scheduled
  into Phases 6-12's file list and would need an explicit scope addition
  if required for v1.

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
