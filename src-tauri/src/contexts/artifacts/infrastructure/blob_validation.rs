use crate::contexts::artifacts::application::ArtifactBlobStoreError;
use sha2::{Digest, Sha256};
use std::path::Path;

const MAX_DISPLAY_NAME_BYTES: usize = 128;

pub(super) fn validate_identifier(value: &str) -> Result<(), ArtifactBlobStoreError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ArtifactBlobStoreError::InvalidOperationId);
    }
    Ok(())
}

pub(super) fn validate_display_name(value: &str) -> Result<(), ArtifactBlobStoreError> {
    let upper = value.trim_end_matches('.').to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "LPT1"];
    if value.is_empty()
        || value.len() > MAX_DISPLAY_NAME_BYTES
        || value == "."
        || value == ".."
        || value.bytes().any(|byte| byte.is_ascii_control())
        || value.contains(['/', '\\', ':'])
        || Path::new(value).is_absolute()
        || reserved.contains(&upper.as_str())
    {
        return Err(ArtifactBlobStoreError::UnsafeDisplayName);
    }
    Ok(())
}

pub(super) fn validate_media(media_type: &str, bytes: &[u8]) -> Result<(), ArtifactBlobStoreError> {
    let valid = match media_type {
        "text/plain" | "text/markdown" | "text/csv" => std::str::from_utf8(bytes).is_ok(),
        "application/json" => serde_json::from_slice::<serde_json::Value>(bytes).is_ok(),
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "application/pdf" => bytes.starts_with(b"%PDF-"),
        _ => return Err(ArtifactBlobStoreError::UnsupportedMediaType),
    };
    if !valid {
        return Err(ArtifactBlobStoreError::InvalidMediaContent);
    }
    Ok(())
}

pub(super) fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}
