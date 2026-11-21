//! Decryption-side helpers for Layer 1. RC4 is symmetric (see `encrypt.rs`),
//! so decryption reuses the same keystream application; this module exists
//! as the decrypt-facing API surface and the place UTF-8 validation lives.

use crate::encrypt::rc4_apply;

/// Decrypts ciphertext produced by `encrypt::encrypt_literal` back into raw
/// bytes. Symmetric with encryption -- see `rc4_apply`.
pub fn decrypt_bytes(ciphertext: &[u8], key: &[u8]) -> Vec<u8> {
    rc4_apply(key, ciphertext)
}

/// Decrypts ciphertext into a `String`, validating UTF-8.
///
/// # Errors
/// Returns `Err` if the decrypted bytes are not valid UTF-8 -- in practice
/// this means the wrong key was used, or the ciphertext/key pairing was
/// corrupted (e.g. a keygen bug). Callers at runtime (`EncryptedStr` in
/// `lib.rs`) decide whether to panic; this function itself never panics.
pub fn decrypt_to_string(
    ciphertext: &[u8],
    key: &[u8],
) -> Result<String, std::string::FromUtf8Error> {
    String::from_utf8(decrypt_bytes(ciphertext, key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encrypt::encrypt_literal;

    #[test]
    fn decrypt_to_string_round_trips() {
        let key = b"another-test-key";
        let plaintext = "ferrox layer 1";
        let ciphertext = encrypt_literal(plaintext, key);
        assert_eq!(decrypt_to_string(&ciphertext, key).unwrap(), plaintext);
    }

    #[test]
    fn decrypt_to_string_reports_bad_key_as_err_not_panic() {
        let key = b"correct-key-0001";
        let wrong_key = b"wrong-key-00001!";
        let ciphertext = encrypt_literal("some secret literal text", key);
        // Only asserting this never panics; a wrong key usually (not
        // always) yields invalid UTF-8, so we don't assert on the Result.
        let _ = decrypt_to_string(&ciphertext, wrong_key);
    }
}
