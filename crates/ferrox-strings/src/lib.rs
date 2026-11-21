//! `ferrox-strings` -- Layer 1 of Ferrox: compile-time string encryption.
//!
//! Scope for this crate (F1.1, F1.2): a symmetric cipher (`encrypt`/
//! `decrypt`) plus a `'static`-friendly runtime wrapper (`EncryptedStr`)
//! that `ferrox-macros`' `enc!()` will expand into (Phase 2). `enc!("x")`
//! becomes a `static` `EncryptedStr` that decrypts itself lazily, once, on
//! first access -- so plaintext exists in memory only after first use, and
//! never in the binary's data section (verified in CI by NF7's `strings`
//! check once the macro and an example binary exist).
//!
//! The actual per-build key comes from `ferrox-keygen` (Phase 3). Until
//! then, this crate is self-contained and tested against hardcoded keys.

pub mod decrypt;
pub mod encrypt;

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
    /// Panics if decryption does not yield valid UTF-8. This should only
    /// happen if the ciphertext/key pairing is corrupted (e.g. a keygen or
    /// macro-expansion bug) -- not a condition that is meaningful to
    /// recover from at the call site, since callers expect `&str`.
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
        // Second call hits the cached value; still correct.
        assert_eq!(es.get(), "hello from ferrox");
    }
}
