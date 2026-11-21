//! Layer 1: compile-time string encryption (F1.1).
//!
//! RC4 stream cipher. RC4 is a known-weak cipher against a dedicated
//! cryptanalyst, which is explicitly fine here: Ferrox's goal is to keep
//! literals out of `strings <binary>` output (F1.2) and raise casual
//! static-analysis cost -- not to provide cryptographic secrecy. The threat
//! model (docs/THREAT_MODEL.md) does not claim otherwise.

/// Runs the RC4 key-scheduling algorithm (KSA), producing the initial
/// permutation state for a given key.
fn ksa(key: &[u8]) -> [u8; 256] {
    let mut s: [u8; 256] = [0; 256];
    for (i, slot) in s.iter_mut().enumerate() {
        *slot = i as u8;
    }
    let mut j: usize = 0;
    for i in 0..256 {
        j = (j + s[i] as usize + key[i % key.len()] as usize) % 256;
        s.swap(i, j);
    }
    s
}

/// RC4 pseudo-random generation algorithm (PRGA): produces `len` bytes of
/// keystream from a KSA'd state.
fn prga(mut s: [u8; 256], len: usize) -> Vec<u8> {
    let mut i: usize = 0;
    let mut j: usize = 0;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        i = (i + 1) % 256;
        j = (j + s[i] as usize) % 256;
        s.swap(i, j);
        let k = s[(s[i] as usize + s[j] as usize) % 256];
        out.push(k);
    }
    out
}

/// Applies RC4 to `data` under `key`. Symmetric: the same call encrypts or
/// decrypts, since RC4 is a stream cipher (XOR against a keystream).
///
/// # Panics
/// Panics if `key` is empty -- an empty key is a caller bug (likely a
/// keygen/build.rs wiring mistake), not a runtime condition Ferrox should
/// mask (NF6's "no panics" rule governs `ferrox-runtime` specifically, not
/// this compile-time-oriented crate).
pub fn rc4_apply(key: &[u8], data: &[u8]) -> Vec<u8> {
    assert!(!key.is_empty(), "ferrox-strings: RC4 key must not be empty");
    let s = ksa(key);
    let keystream = prga(s, data.len());
    data.iter()
        .zip(keystream.iter())
        .map(|(d, k)| d ^ k)
        .collect()
}

/// Encrypts a plaintext string literal at compile time (called from
/// `ferrox-macros`' `enc!` expansion once Phase 2 lands).
pub fn encrypt_literal(plaintext: &str, key: &[u8]) -> Vec<u8> {
    rc4_apply(key, plaintext.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc4_round_trips_arbitrary_text() {
        let key = b"ferrox-test-key-0001";
        let plaintext = "the quick brown fox jumps over the lazy dog";
        let ciphertext = encrypt_literal(plaintext, key);
        assert_ne!(ciphertext, plaintext.as_bytes(), "cipher must not be a no-op");
        let decrypted = rc4_apply(key, &ciphertext);
        assert_eq!(decrypted, plaintext.as_bytes());
    }

    #[test]
    fn rc4_round_trips_empty_string() {
        let key = b"any-key";
        let ciphertext = encrypt_literal("", key);
        assert!(ciphertext.is_empty());
        assert_eq!(rc4_apply(key, &ciphertext), b"".to_vec());
    }

    #[test]
    fn rc4_round_trips_unicode() {
        let key = b"\x01\x02\x03\x04\x05\x06\x07\x08";
        let plaintext = "hello world \u{1f980}";
        let ciphertext = encrypt_literal(plaintext, key);
        let decrypted = rc4_apply(key, &ciphertext);
        assert_eq!(String::from_utf8(decrypted).unwrap(), plaintext);
    }

    #[test]
    #[should_panic(expected = "key must not be empty")]
    fn rc4_rejects_empty_key() {
        let _ = rc4_apply(&[], b"data");
    }
}
