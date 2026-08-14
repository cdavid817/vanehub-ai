#![allow(dead_code)]

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const MAX_BROWSER_TRANSFER_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserUploadPayload {
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) media_type: String,
    pub(crate) display_name: String,
    pub(crate) bytes_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserArtifactReference {
    pub(crate) contract_version: u16,
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) size_bytes: u64,
    pub(crate) media_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserArtifactError {
    InvalidRequest,
    NotFound,
    IntegrityFailure,
    TooLarge,
    UnsupportedMedia,
    StorageFailure,
}

pub(crate) trait BrowserArtifactPort: Send + Sync {
    fn read_verified(
        &self,
        artifact_id: &str,
        max_bytes: usize,
    ) -> Result<(String, String, String, Vec<u8>), BrowserArtifactError>;

    fn seal_browser_output(
        &self,
        operation_id: &str,
        display_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<BrowserArtifactReference, BrowserArtifactError>;
}

pub(crate) struct BrowserArtifactBridge {
    artifacts: Arc<dyn BrowserArtifactPort>,
}

impl BrowserArtifactBridge {
    pub(crate) fn new(artifacts: Arc<dyn BrowserArtifactPort>) -> Self {
        Self { artifacts }
    }

    pub(crate) fn upload_payload(
        &self,
        artifact_id: &str,
    ) -> Result<BrowserUploadPayload, BrowserArtifactError> {
        validate_artifact_id(artifact_id)?;
        let (content_hash, media_type, display_name, bytes) = self
            .artifacts
            .read_verified(artifact_id, MAX_BROWSER_TRANSFER_BYTES)?;
        if bytes.len() > MAX_BROWSER_TRANSFER_BYTES {
            return Err(BrowserArtifactError::TooLarge);
        }
        if !allowed_upload_media(&media_type) || hex_digest(&bytes) != content_hash {
            return Err(BrowserArtifactError::IntegrityFailure);
        }
        Ok(BrowserUploadPayload {
            artifact_id: artifact_id.to_string(),
            content_hash,
            media_type,
            display_name,
            bytes_base64: STANDARD.encode(bytes),
        })
    }

    pub(crate) fn seal_download(
        &self,
        operation_id: &str,
        display_name: &str,
        media_type: &str,
        bytes_base64: &str,
    ) -> Result<BrowserArtifactReference, BrowserArtifactError> {
        if operation_id.is_empty()
            || operation_id.len() > 128
            || display_name.is_empty()
            || display_name.len() > 128
            || bytes_base64.len() > MAX_BROWSER_TRANSFER_BYTES * 2
        {
            return Err(BrowserArtifactError::InvalidRequest);
        }
        let bytes = STANDARD
            .decode(bytes_base64)
            .map_err(|_| BrowserArtifactError::InvalidRequest)?;
        if bytes.len() > MAX_BROWSER_TRANSFER_BYTES {
            return Err(BrowserArtifactError::TooLarge);
        }
        self.artifacts
            .seal_browser_output(operation_id, display_name, media_type, &bytes)
    }
}

fn validate_artifact_id(value: &str) -> Result<(), BrowserArtifactError> {
    if !value.starts_with("artifact-")
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(BrowserArtifactError::InvalidRequest);
    }
    Ok(())
}

fn allowed_upload_media(media_type: &str) -> bool {
    matches!(
        media_type,
        "text/plain"
            | "text/csv"
            | "application/json"
            | "application/pdf"
            | "image/png"
            | "image/jpeg"
    )
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
#[path = "artifact_bridge_tests.rs"]
mod tests;
