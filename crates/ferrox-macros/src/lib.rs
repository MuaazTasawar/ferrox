//! Ferrox proc-macros, consumed by code being protected by Ferrox.
//!
//! Important: macro expansions reference sibling Ferrox crates (e.g.
//! `::ferrox_strings::EncryptedStr`, `::ferrox_runtime::__ferrox_protect_marker`)
//! by absolute path. Any crate using `enc!` or `#[protect]` must list
//! those crates as *direct* dependencies of its own `Cargo.toml` -- not
//! just rely on them transitively through `ferrox-macros`.

mod enc_macro;
mod protect_macro;

use proc_macro::TokenStream;

/// `ferrox_macros::enc!("literal")` -- encrypts a string literal at
/// compile time (F1.1) and expands to a `&str` expression that decrypts
/// lazily on first access. See `enc_macro` for the generated code shape.
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

/// `#[ferrox_macros::protect]` -- marks a function for Layer 2 MIR
/// flattening + opaque predicates (Phase 7/8). See `protect_macro` for
/// what it injects and `ferrox-mir-pass`'s detection logic for how it's
/// consumed.
///
/// # Example
/// ```ignore
/// #[ferrox_macros::protect]
/// fn check_license(input: &str) -> bool {
///     input == "secret"
/// }
/// ```
#[proc_macro_attribute]
pub fn protect(attr: TokenStream, item: TokenStream) -> TokenStream {
    protect_macro::expand(attr, item)
}
