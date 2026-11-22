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

### Layer 1 -- String Encryption (Phases 1-3, 6)

`ferrox-strings` provides RC4-based encryption; `ferrox-macros`' `enc!`
macro is the user-facing entry point, encrypting literals at compile time
and decrypting lazily, once, at first runtime access. Defends against
`strings <binary>` scans and casual hex-editor inspection; does **not**
defend against a breakpoint on `EncryptedStr::get()` reading decrypted
memory at runtime -- that is Layer 3's job.

**F1.8 caveat: closed as of Phase 6.** `ferrox-keygen`'s `run()` now emits
`cargo:rustc-env=FERROX_STRING_KEY_HEX=<hex>` in addition to `keys.rs`;
`enc!` reads that env var at macro-expansion time and, when present and
valid, uses it as a single key shared by every `enc!` call site in that
build -- rather than each site generating its own independent key. This
is exercised concretely by `examples/license-checker`, whose three
`enc!()` calls (license key, success message, rejection message) all
share one per-build key, and is verified in CI (`.github/workflows/ci.yml`'s
F1.2 `strings` check) rather than only asserted in isolated crate tests.
**Documented fallback, not silent:** a crate using `enc!` without wiring
in `ferrox-keygen` still works, but falls back to the Phase 2
independent-per-site-key behavior.

### Layer 3 -- Runtime Anti-Tamper / Anti-Debug (Phase 4, exercised in Phase 6)

Panic hygiene, a self-checksum background thread (trust-on-first-run --
see the known limitation below, unchanged since Phase 4/5), and three
independent anti-debug checks, all reporting into one shared
`tamper_detected()` flag.

`examples/license-checker` is the first concrete demonstration of F1.5
and F1.7 as actually *used*, not just provided: `ferrox-runtime` itself
never corrupts anything (it has no notion of what "the program's output"
means for an arbitrary crate) -- `license-checker`'s `main()` is the call
site that checks `tamper_detected()` immediately before printing its
verdict and flips a valid license to invalid if tampering was found, with
no message naming tampering ever printed (F1.5). Its three anti-debug
checks are called from three different points in `main` -- at startup,
after reading input, and immediately around the security-relevant
decision -- rather than funneled through one aggregator (F1.7).

**Known limitation, unchanged:** the checksum baseline is still
trust-on-first-run, not a true build-time-embedded hash requiring
post-link binary patching (tracked since Phase 4, restated in Phase 5 --
not addressed by this phase, which focused on Layer 1/3 *usage*, not new
Layer 3 capability).

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
