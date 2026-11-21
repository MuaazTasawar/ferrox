//! F1.1: `enc!("literal")` -- encrypts a string literal at compile time and
//! expands into a lazily-decrypting `ferrox_strings::EncryptedStr::get()`
//! call.
//!
//! Key generation (interim, pre-Phase-3): each `enc!` invocation generates
//! its own random 32-byte key at macro-expansion time, i.e. inside this
//! proc-macro, on the *host* running the compiler -- never at the target's
//! runtime, and never derived from anything observable in the output
//! binary. This already satisfies F1.1 ("encrypts at compile time") and
//! F1.2 (only ciphertext bytes land in the binary's data section, not
//! plaintext). Phase 3 wires in `ferrox-keygen` so every `enc!` site in one
//! build shares a single per-build seed (needed for F1.8's "two builds
//! differ" requirement at the whole-binary level); this macro's expansion
//! *shape* does not change when that lands, only where the key bytes come
//! from.
//!
//! Hygiene note: the expansion references `::ferrox_strings::EncryptedStr`
//! by absolute path, which means any crate using `enc!` must depend on
//! `ferrox-strings` directly (not just transitively through this crate).

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use rand::RngCore;
use syn::{parse_macro_input, LitStr};

pub fn expand(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let plaintext = lit.value();

    let mut key = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);

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
