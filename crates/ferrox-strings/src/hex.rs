//! Minimal hex encode/decode, used to pass binary key material through
//! `cargo:rustc-env` string values (env vars are text, not bytes) between
//! `ferrox-keygen` (encoder, Phase 6) and `ferrox-macros`' `enc!` macro
//! (decoder, Phase 6). Lives here rather than duplicated in both crates,
//! or added as a one-off dependency in either, since `ferrox-strings` is
//! already the natural home for "encoding helpers around key material."

/// Encodes bytes as lowercase hex, two characters per byte.
pub fn encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Decodes a lowercase-or-uppercase hex string back into bytes.
///
/// # Errors
/// Returns `Err` with a human-readable message if the input has odd
/// length or contains a non-hex-digit character. Never panics -- this is
/// called from `enc_macro.rs` at macro-expansion time, where a panic
/// would surface as a confusing proc-macro ICE rather than a clean
/// fallback.
pub fn decode(s: &str) -> Result<Vec<u8>, String> {
    if !s.len().is_multiple_of(2) {
        return Err(format!("hex string has odd length ({})", s.len()));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = (chunk[0] as char)
            .to_digit(16)
            .ok_or_else(|| format!("invalid hex digit: {}", chunk[0] as char))?;
        let lo = (chunk[1] as char)
            .to_digit(16)
            .ok_or_else(|| format!("invalid hex digit: {}", chunk[1] as char))?;
        out.push(((hi << 4) | lo) as u8);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_arbitrary_bytes() {
        let original: Vec<u8> = (0..=255u16).map(|b| b as u8).collect();
        let encoded = encode(&original);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn encode_produces_lowercase_two_chars_per_byte() {
        assert_eq!(encode(&[0x00, 0xff, 0x0a]), "00ff0a");
    }

    #[test]
    fn decode_accepts_uppercase() {
        assert_eq!(decode("FF00").unwrap(), vec![0xff, 0x00]);
    }

    #[test]
    fn decode_rejects_odd_length() {
        assert!(decode("abc").is_err());
    }

    #[test]
    fn decode_rejects_invalid_digit() {
        assert!(decode("zz").is_err());
    }

    #[test]
    fn decode_empty_string_is_empty_vec() {
        assert_eq!(decode("").unwrap(), Vec::<u8>::new());
    }
}
