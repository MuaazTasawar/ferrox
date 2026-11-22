//! Calls `ferrox_keygen::run()`, which (Phase 6) both writes `OUT_DIR/keys.rs`
//! and emits `cargo:rustc-env=FERROX_STRING_KEY_HEX=<hex>` -- the latter is
//! what lets every `enc!()` call in `src/main.rs` share one per-build key
//! (see `ferrox-macros`' `enc_macro.rs` module docs for the mechanism).
fn main() {
    ferrox_keygen::run().expect("ferrox-keygen: failed to generate keys.rs");
}
