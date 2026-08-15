#![cfg_attr(not(test), allow(dead_code))]

use crate::contexts::tooling::skills::application::{
    OverlayApplicationError, OverlayContentScannerPort, OverlayValidatedFile,
};
use crate::contexts::tooling::skills::domain::{
    scan_overlay_text, validate_overlay_media, validate_overlay_path, OverlayMediaError,
    OverlayPathError, OverlayTextScan,
};
use sha2::{Digest, Sha256};

#[derive(Clone, Default)]
pub(crate) struct DeterministicOverlayContentScanner;

impl OverlayContentScannerPort for DeterministicOverlayContentScanner {
    fn scan_text(&self, content: &str) -> OverlayTextScan {
        scan_overlay_text(content)
    }

    fn validate_file(
        &self,
        logical_path: &str,
        media_type: &str,
        content: &[u8],
    ) -> Result<OverlayValidatedFile, OverlayApplicationError> {
        let path = validate_overlay_path(logical_path).map_err(|error| {
            OverlayApplicationError::ImportRejected {
                code: path_error_code(error).to_string(),
            }
        })?;
        let media = validate_overlay_media(&path, media_type, content).map_err(|error| {
            OverlayApplicationError::ImportRejected {
                code: media_error_code(error).to_string(),
            }
        })?;
        Ok(OverlayValidatedFile {
            logical_path: path.as_str().to_string(),
            media_type: media_type.to_string(),
            content_kind: media.content_kind(),
            size_bytes: content.len() as u64,
            content_hash: sha256(content),
        })
    }
}

fn path_error_code(error: OverlayPathError) -> &'static str {
    match error {
        OverlayPathError::Empty => "overlay-path-empty",
        OverlayPathError::TooLong { .. } => "overlay-path-too-long",
        OverlayPathError::TooDeep { .. } => "overlay-path-too-deep",
        OverlayPathError::AbsolutePath => "overlay-path-absolute",
        OverlayPathError::ParentTraversal => "overlay-path-traversal",
        OverlayPathError::CurrentDirectory => "overlay-path-current-directory",
        OverlayPathError::HiddenComponent => "overlay-path-hidden-component",
        OverlayPathError::UnsupportedTopLevel => "overlay-path-unsupported-root",
        OverlayPathError::ReservedExecutablePath => "overlay-path-reserved-executable",
        OverlayPathError::ReservedDevice => "overlay-path-reserved-device",
        OverlayPathError::AlternateDataStream => "overlay-path-alternate-stream",
        OverlayPathError::NonCanonicalSeparator => "overlay-path-noncanonical-separator",
        OverlayPathError::EmptyComponent => "overlay-path-empty-component",
        OverlayPathError::TrailingDotOrSpace => "overlay-path-trailing-dot-or-space",
        OverlayPathError::InvalidCharacter => "overlay-path-invalid-character",
        OverlayPathError::MissingFileName => "overlay-path-missing-file-name",
        OverlayPathError::LinkEscape => "overlay-path-escaping-link",
    }
}

fn media_error_code(error: OverlayMediaError) -> &'static str {
    match error {
        OverlayMediaError::MissingExtension => "overlay-media-missing-extension",
        OverlayMediaError::ProhibitedExtension => "overlay-media-prohibited-extension",
        OverlayMediaError::ProhibitedMediaType => "overlay-media-prohibited-type",
        OverlayMediaError::UnsupportedExtension => "overlay-media-unsupported-extension",
        OverlayMediaError::MediaTypeMismatch => "overlay-media-type-mismatch",
        OverlayMediaError::ExecutableSignature => "overlay-media-executable-signature",
        OverlayMediaError::InvalidUtf8 => "overlay-media-invalid-utf8",
        OverlayMediaError::BinaryOutsideAssets => "overlay-media-binary-outside-assets",
        OverlayMediaError::InvalidBinarySignature => "overlay-media-invalid-binary-signature",
        OverlayMediaError::BinaryTextReadDenied => "overlay-media-binary-text-read-denied",
        OverlayMediaError::TooLarge { .. } => "overlay-media-too-large",
    }
}

fn sha256(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
