//! F1.1: `enc!("literal")` -- encrypts a string literal at compile time and
//! expands into a lazily-decrypting `ferrox_strings::EncryptedStr::get()`
//! call.
//!
//! ## Key sourcing (Phase 6)
//! If the consuming crate's `build.rs` calls `ferrox_keygen::run()`,
//! `ferrox-keygen` emits `cargo:rustc-env=FERROX_STRING_KEY_HEX=<hex>`,
//! which cargo sets as a real process environment variable for the
//! `rustc` invocation compiling that crate -- readable here, at
//! macro-expansion time, via plain `std::env::var`. When present and
//! valid (32 bytes after hex-decoding), **every** `enc!` call site in
//! that build shares this one key, satisfying F1.8's "coordinated
//! per-build identity" property for real.
//!
//! **Documented fallback:** if the env var is absent, or present but
//! malformed (wrong length, invalid hex), this falls back to generating
//! an independent random 32-byte key for *this one call site only* --
//! the Phase 2 behavior. This keeps `enc!` usable in a crate that hasn't
//! wired up `ferrox-keygen` at all, at the cost of not sharing a key
//! across sites in that case. `examples/license-checker` (Phase 6) is
//! wired with `ferrox-keygen`, so it exercises the shared-key path.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rand::RngCore;
use syn::{parse_macro_input, LitStr};

const KEY_ENV_VAR: &str = "FERROX_STRING_KEY_HEX";

/// Resolves the 32-byte key to use for this `enc!` expansion: the shared
/// per-build key from `FERROX_STRING_KEY_HEX` if present and valid,
/// otherwise a freshly-generated random key for this call site alone
/// (see module docs for when each path is taken).
fn resolve_key() -> Vec<u8> {
    if let Ok(hex) = std::env::var(KEY_ENV_VAR) {
        if let Ok(bytes) = ferrox_strings::hex::decode(&hex) {
            if bytes.len() == 32 {
                return bytes;
            }
        }
        // Present but malformed -- fall through to the random fallback
        // rather than failing the build outright; a build-time key
        // problem showing up as "strings aren't coordinated this build"
        // is far easier to diagnose than a proc-macro panic deep inside
        // an unrelated crate's compilation.
    }
    let mut key = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

pub fn expand(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let plaintext = lit.value();

    let key = resolve_key();
    let ciphertext = ferrox_strings::encrypt::encrypt_literal(&plaintext, &key);

    let key_bytes = key.iter().map(|b| quote! { #b });
    let cipher_bytes = ciphertext.iter().map(|b| quote! { #b });

    let expanded: TokenStream2 = quote! {
        {
            static __FERROX_KEY: &[u8] = &[ #(#key_bytes),* ];
            static __FERROX_CIPHERTEXT: &[u8] = &[ #(#cipher_bytes),* ];
            static __FERROX_ENC: ::ferrox_strings::EncryptedStr =
                ::ferrox_strings::EncryptedStr::new(__FERROX_CIPHERTEXT, __FERROX_KEY);
            __FERROX_ENC.get()
        }
    };

    expanded.into()
}
