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
out of memory at runtime -- that is Layer 3's job to make harder.

`ferrox-keygen` (Phase 3) independently satisfies F1.8. **Interim
caveat** (resolved at Phase 6): `enc!` still generates its own
independent per-site key rather than consuming `ferrox-keygen`'s output;
the two are wired together concretely in `examples/license-checker`.

### Layer 3 -- Runtime Anti-Tamper / Anti-Debug (Phase 4)

`ferrox-runtime` provides three independent pieces, all linked directly
into the protected binary:

- **Panic hygiene (F1.3):** `panic_hook::install()` replaces the default
  panic hook with a silent one, but only in release builds
  (`cfg(not(debug_assertions))`) -- developers still see normal panic
  output while iterating locally.
- **Self-checksum (F1.4, F1.5):** `checksum::start()` spawns a background
  thread that re-hashes the process's own loaded code region every
  2-5 seconds (randomized) and compares against a baseline computed on
  first run. On mismatch, it calls `mark_tampered()` -- it does **not**
  crash, print, or otherwise announce the mismatch (F1.5). **Known
  limitation, stated plainly:** the baseline is trust-on-first-run, not a
  true build-time-embedded hash (which requires post-link patching, not
  yet implemented -- planned for `ferrox-cli` in Phase 5). This defeats a
  debugger attaching mid-run and patching a check in place; it does
  **not** defeat an attacker who statically patches the binary before it
  ever runs. The hashed region is also a simplification: the whole loaded
  image/first executable mapping, not a precise `.text`-only
  section-table parse -- arguably stronger in practice (also catches
  read-only data tampering), but worth naming as a deviation from a
  literal "`.text` segment" reading of F1.4.
- **Anti-debug (F1.6, F1.7):** three independent checks, each its own
  function rather than one `is_debugged()` aggregator --
  `anti_debug::linux::check()` (`ptrace(PTRACE_TRACEME)` self-trace, Linux
  only), `anti_debug::windows::check()` (`IsDebuggerPresent`, Windows
  only), and `anti_debug::timing::check()` (timing anomaly on a trivial
  computation, both platforms). All three are meant to be called from
  different, unrelated call sites in the *consuming* crate's own logic
  (F1.7) -- demonstrated concretely in `examples/license-checker`
  (Phase 6), not bundled into one call here.

What ties the above together: `tamper_detected() -> bool` reads a single
shared flag any of the three mechanisms can set. **`ferrox-runtime`
itself never corrupts anything** -- it has no notion of what "the
program's output" means for an arbitrary protected crate. F1.5's actual
silent-corruption behavior is the consuming crate's responsibility, at
whatever call site makes sense for its own logic (again, shown concretely
in Phase 6).

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
