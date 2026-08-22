//! Naming bytes by their digest.
//!
//! Every store that keeps immutable content — Overlay payloads today, extension packages next —
//! writes the same two lines: hash the bytes to lowercase hexadecimal, and refuse a name that is
//! not exactly that shape. They were written twice already, in two files, with two spellings of
//! the same hexadecimal test. Once is enough.
//!
//! Lowercase is part of the format, not a preference. A store keyed by these strings on a
//! case-insensitive filesystem would otherwise accept `AB…` and `ab…` as one file while treating
//! them as two keys.
//!
//! This is for *stores*, not for every caller that happens to hash something. Roughly sixty files
//! call `Sha256::digest` for their own reasons, many of them in domain layers that may not reach
//! `crate::platform` at all; routing those here would trade a two-line duplicate for an
//! architecture violation.

use sha2::{Digest, Sha256};

/// How long a SHA-256 digest is when written as hexadecimal.
pub(crate) const SHA256_HEX_CHARACTERS: usize = 64;

/// The SHA-256 digest of `bytes`, as lowercase hexadecimal.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// True when `value` is exactly a lowercase hexadecimal SHA-256 digest.
pub(crate) fn is_sha256_hex(value: &str) -> bool {
    value.len() == SHA256_HEX_CHARACTERS
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_digest_is_lowercase_hexadecimal_of_a_known_vector() {
        // RFC 6234's "abc" vector, so a change in the hashing dependency is caught by value rather
        // than by whether two of our own calls still agree with each other.
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert!(is_sha256_hex(&sha256_hex(b"abc")));
    }

    #[test]
    fn a_name_is_a_digest_only_at_the_exact_length_and_alphabet() {
        let digest = sha256_hex(b"abc");
        assert!(is_sha256_hex(&digest));

        assert!(!is_sha256_hex(""));
        assert!(!is_sha256_hex(&digest[..SHA256_HEX_CHARACTERS - 1]));
        assert!(!is_sha256_hex(&format!("{digest}0")));
        assert!(
            !is_sha256_hex(&digest.to_uppercase()),
            "uppercase is a different key on a case-sensitive store and the same file on a \
             case-insensitive one"
        );
        assert!(!is_sha256_hex(&"g".repeat(SHA256_HEX_CHARACTERS)));
        assert!(!is_sha256_hex(&" ".repeat(SHA256_HEX_CHARACTERS)));
        // Length is counted in bytes, and a multi-byte character makes the string shorter than it
        // looks. Neither reading admits it, but the reason differs, so both are pinned.
        assert!(!is_sha256_hex(&"é".repeat(SHA256_HEX_CHARACTERS / 2)));
    }
}
