//! `ferrox-strings` -- Layer 1 of Ferrox: compile-time string encryption.
//!
//! Scope for this crate (F1.1, F1.2): a symmetric cipher (`encrypt`/
//! `decrypt`), a `'static`-friendly runtime wrapper (`EncryptedStr`) that
//! `ferrox-macros`' `enc!()` expands into, and (Phase 6) a `hex` module
//! used to pass binary key material through `cargo:rustc-env` string
//! values between `ferrox-keygen` and `ferrox-macros`.
//!
//! `enc!("x")` becomes a `static` `EncryptedStr` that decrypts itself
//! lazily, once, on first access -- so plaintext exists in memory only
//! after first use, and never in the binary's data section (verified in
//! CI by the F1.2 `strings` check against `examples/license-checker`).

pub mod decrypt;
pub mod encrypt;
pub mod hex;

use std::sync::OnceLock;

/// A lazily-decrypted string, built from a ciphertext blob and the key to
/// decrypt it. Designed to be constructed as a `static` (both fields are
/// `'static` slices, and `new` is `const fn`), so the ciphertext lives in
/// the binary's data section and the plaintext does not exist until
/// `get()` is first called.
pub struct EncryptedStr {
    ciphertext: &'static [u8],
    key: &'static [u8],
    cell: OnceLock<String>,
}

impl EncryptedStr {
    /// Constructs a new lazily-decrypted string. `const fn` so this can be
    /// used to initialize a `static`.
    pub const fn new(ciphertext: &'static [u8], key: &'static [u8]) -> Self {
        EncryptedStr {
            ciphertext,
            key,
            cell: OnceLock::new(),
        }
    }

    /// Returns the decrypted string, decrypting on first call and caching
    /// the result for the lifetime of the program.
    ///
    /// # Panics
    /// Panics if decryption does not yield valid UTF-8 -- in practice this
    /// should only happen if the ciphertext/key pairing is corrupted.
    pub fn get(&self) -> &str {
        self.cell.get_or_init(|| {
            decrypt::decrypt_to_string(self.ciphertext, self.key).unwrap_or_else(|_| {
                panic!(
                    "ferrox-strings: decrypted bytes were not valid UTF-8 \
                     (corrupted ciphertext/key pairing)"
                )
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encrypt::encrypt_literal;

    #[test]
    fn encrypted_str_decrypts_lazily_and_caches() {
        static KEY: &[u8] = b"lib-level-test-key-01";
        let ciphertext: &'static [u8] =
            Box::leak(encrypt_literal("hello from ferrox", KEY).into_boxed_slice());
        let es = EncryptedStr::new(ciphertext, KEY);
        assert_eq!(es.get(), "hello from ferrox");
        assert_eq!(es.get(), "hello from ferrox");
    }
}
