//! `#[ferrox::protect]` -- Phase 8's attribute macro. Marks a function for
//! Layer 2 (MIR control-flow flattening + opaque predicates, Phase 7) by
//! injecting a call to `ferrox_runtime::__ferrox_protect_marker()` as the
//! function's first statement. This has no runtime effect of its own
//! (the marker function is a no-op) -- it exists purely so
//! `ferrox-mir-pass` can detect, per-function, which functions the user
//! actually asked to protect, rather than flattening every function in
//! the crate (the Phase 7 smoke-test behavior, which was intentionally
//! blanket for testing purposes only).
//!
//! A crate using `#[ferrox::protect]` must depend on `ferrox-runtime`
//! directly, same convention as `enc!` and `ferrox-strings` (the
//! expansion emits an absolute path, `::ferrox_runtime::...`).
//!
//! Without `ferrox-mir-pass` actually running (i.e. a plain `cargo build`,
//! not yet routed through it -- that wiring is Phase 7's remaining
//! step 7f-ii), this attribute is inert: the function compiles and runs
//! completely normally, just with one extra no-op call at the top.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(item as ItemFn);

    let marker_call: syn::Stmt = syn::parse_quote! {
        ::ferrox_runtime::__ferrox_protect_marker();
    };
    func.block.stmts.insert(0, marker_call);

    let expanded: TokenStream2 = quote! { #func };
    expanded.into()
}
