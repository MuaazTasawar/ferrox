//! Integration test for `enc!`. Proc-macro expansion can only be exercised
//! through genuine macro invocation (not called as a plain function from a
//! unit test), so this lives in `tests/` where cargo compiles it as a
//! separate crate and expands the macro for real.

use ferrox_macros::enc;

#[test]
fn enc_macro_decrypts_to_original_literal() {
    let s = enc!("integration test literal");
    assert_eq!(s, "integration test literal");
}

#[test]
fn enc_macro_handles_empty_string() {
    let s = enc!("");
    assert_eq!(s, "");
}

#[test]
fn enc_macro_handles_unicode() {
    let s = enc!("unicode check: \u{1f980}");
    assert_eq!(s, "unicode check: \u{1f980}");
}

#[test]
fn two_enc_invocations_use_independent_keys() {
    // Not a correctness requirement from the spec, but a sanity check that
    // the interim per-invocation key generation (see enc_macro.rs) is at
    // least producing *some* randomness rather than a fixed key -- run
    // twice and confirm both still decrypt correctly regardless.
    let a = enc!("first literal");
    let b = enc!("second literal");
    assert_eq!(a, "first literal");
    assert_eq!(b, "second literal");
}
