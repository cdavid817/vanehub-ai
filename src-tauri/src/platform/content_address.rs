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
use std::io::Read;

/// How long a SHA-256 digest is when written as hexadecimal.
pub(crate) const SHA256_HEX_CHARACTERS: usize = 64;

/// The SHA-256 digest of `bytes`, as lowercase hexadecimal.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg_attr(not(test), allow(dead_code))]
/// How much is read at a time when hashing a stream. Large enough that the syscall cost is
/// irrelevant, small enough that the buffer is not worth thinking about.
const STREAM_CHUNK_BYTES: usize = 64 * 1024;

/// The SHA-256 digest and byte length of everything `source` yields.
///
/// Streamed rather than read-then-hash because the caller is hashing a package that is allowed to
/// be large: `fs::read` on a 64 MiB archive means 64 MiB resident for no reason, and on a source
/// whose length was misreported it means whatever the source felt like sending.
///
/// The length is returned alongside the digest rather than taken from file metadata. They have to
/// describe the same read — a length from `stat` and a digest from a stream are two facts about
/// two moments, and a signature binds both.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn sha256_hex_stream(source: &mut impl Read) -> std::io::Result<(String, u64)> {
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; STREAM_CHUNK_BYTES];
    let mut total = 0_u64;
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total = total.saturating_add(read as u64);
    }
    Ok((
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        total,
    ))
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
    fn a_stream_hashes_to_the_same_digest_as_the_whole_buffer() {
        // A reader that hands back one byte at a time, which is what a network or decompressing
        // source does. If the loop ever assumed a full buffer per read, this is where it shows.
        struct OneByteAtATime<'a>(&'a [u8]);
        impl Read for OneByteAtATime<'_> {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                match (self.0.split_first(), buffer.is_empty()) {
                    (Some((byte, rest)), false) => {
                        buffer[0] = *byte;
                        self.0 = rest;
                        Ok(1)
                    }
                    _ => Ok(0),
                }
            }
        }

        let content = b"the quick brown fox jumps over the lazy dog".repeat(4_096);
        let mut trickle = OneByteAtATime(&content);
        assert_eq!(
            sha256_hex_stream(&mut trickle).expect("hash a trickling stream"),
            (sha256_hex(&content), content.len() as u64)
        );

        let mut empty = &b""[..];
        assert_eq!(
            sha256_hex_stream(&mut empty).expect("hash an empty stream"),
            (sha256_hex(b""), 0)
        );
    }

    #[test]
    fn a_stream_that_fails_part_way_reports_the_error_rather_than_a_partial_digest() {
        struct FailsAfterOneChunk(bool);
        impl Read for FailsAfterOneChunk {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if self.0 {
                    return Err(std::io::Error::other("stream broke"));
                }
                self.0 = true;
                buffer[0] = b'a';
                Ok(1)
            }
        }

        // A digest of the bytes that happened to arrive would be a truthful-looking answer to a
        // question nobody asked, and it would then be compared against a signature.
        assert!(sha256_hex_stream(&mut FailsAfterOneChunk(false)).is_err());
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
