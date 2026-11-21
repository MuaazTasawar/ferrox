//! Ferrox proc-macros, consumed by code being protected by Ferrox.
//!
//! Important: macro expansions reference sibling Ferrox crates (e.g.
//! `::ferrox_strings::EncryptedStr`) by absolute path. Any crate that uses
//! `enc!` (and, from Phase 8/10 onward, `#[ferrox::protect]` /
//! `#[ferrox::vm]`) must list those crates as *direct* dependencies of its
//! own `Cargo.toml` -- not just rely on them transitively through
//! `ferrox-macros`. `examples/license-checker` (Phase 6) demonstrates the
//! full wiring.

mod enc_macro;

use proc_macro::TokenStream;

/// `ferrox_macros::enc!("literal")` -- encrypts a string literal at compile
/// time (F1.1) and expands to a `&str` expression that decrypts lazily on
/// first access, once. See `enc_macro` for the generated code shape and the
/// interim key-generation note.
///
/// # Example
/// ```ignore
/// let msg = ferrox_macros::enc!("this string never appears in `strings`");
/// println!("{msg}");
/// ```
#[proc_macro]
pub fn enc(input: TokenStream) -> TokenStream {
    enc_macro::expand(input)
}
