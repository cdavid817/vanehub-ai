//! Admitted OCR input: what the bytes actually are, and who owns the staged copy.

use super::error::LocalMediaErrorCode;
use super::ids::StagedInputId;
use serde::{Deserialize, Serialize};

/// How long an unclaimed staged file survives. Long enough that a user can be interrupted between
/// picking a file and starting OCR; short enough that a cancelled pick does not leave bytes on disk
/// until the next application start.
pub(crate) const STAGED_INPUT_TTL_MS: u64 = 10 * 60 * 1000;

const MAX_DISPLAY_NAME_CHARS: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OcrMediaType {
    Image,
    Pdf,
}

impl OcrMediaType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Pdf => "pdf",
        }
    }
}

/// A format the OCR pipeline accepts. Recognized from content, never from the file name.
///
/// The set is bounded by what the host can measure, not by what PaddleOCR can read. Admission has
/// to enforce a decoded-pixel ceiling before handing a file to an inference process, and PNG, JPEG,
/// and BMP are the formats whose dimensions come out of a fixed-offset header read. TIFF and WEBP
/// would need a real parser to answer the same question, so they are refused rather than admitted
/// with the pixel bound quietly unenforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SniffedFormat {
    Png,
    Jpeg,
    Bmp,
    Pdf,
}

impl SniffedFormat {
    pub(crate) fn media_type(self) -> OcrMediaType {
        match self {
            Self::Pdf => OcrMediaType::Pdf,
            _ => OcrMediaType::Image,
        }
    }

    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Bmp => "bmp",
            Self::Pdf => "pdf",
        }
    }
}

/// Identify the content from a bounded prefix.
///
/// An unrecognized prefix returns `None` rather than a best guess: the caller's next step is to
/// hand the path to an inference process, and "probably a JPEG" is not a good enough reason to do
/// that. Extension is never consulted.
pub(crate) fn sniff_media(bytes: &[u8]) -> Option<SniffedFormat> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(SniffedFormat::Png);
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Some(SniffedFormat::Jpeg);
    }
    if bytes.starts_with(b"%PDF-") {
        return Some(SniffedFormat::Pdf);
    }
    // BMP needs its full DIB header present, not just the two-byte signature, because that is
    // where admission reads the dimensions from.
    if bytes.starts_with(b"BM") && bytes.len() >= 26 {
        return Some(SniffedFormat::Bmp);
    }
    None
}

/// Reduce a caller-supplied path to something safe to show.
///
/// Both separators are stripped explicitly instead of going through `Path::file_name`, because that
/// function is platform-aware: a Windows path handled on Linux would keep its directories and the
/// review dialog would display the user's folder layout.
pub(crate) fn sanitize_display_name(raw: &str) -> String {
    let last_component = raw.rsplit(['/', '\\']).next().unwrap_or(raw);
    let cleaned: String = last_component
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_DISPLAY_NAME_CHARS)
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "file".to_string()
    } else {
        trimmed.to_string()
    }
}

/// What the frontend learns about a staged file. No path, no hash, no bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StagedOcrSource {
    pub(crate) staged_input_id: StagedInputId,
    pub(crate) display_name: String,
    pub(crate) media_type: OcrMediaType,
    pub(crate) byte_length: u64,
}

/// The host-side record. Ownership transfer to an operation is a one-way transition guarded here so
/// that two OCR starts racing on the same id cannot both proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StagedInputRecord {
    /// Retained so a record carries its own identity when it is moved out of the map.
    #[allow(
        dead_code,
        reason = "identity travels with the record; expiry and claim read the rest"
    )]
    id: StagedInputId,
    source: StagedOcrSource,
    created_at_ms: u64,
    claimed: bool,
}

impl StagedInputRecord {
    pub(crate) fn new(id: StagedInputId, source: StagedOcrSource, created_at_ms: u64) -> Self {
        Self {
            id,
            source,
            created_at_ms,
            claimed: false,
        }
    }

    pub(crate) fn source(&self) -> &StagedOcrSource {
        &self.source
    }

    /// Exercised by this module's tests; production reads the state it derives, not this.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn is_claimed(&self) -> bool {
        self.claimed
    }

    pub(crate) fn is_expired(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.created_at_ms) >= STAGED_INPUT_TTL_MS
    }

    /// Take ownership for one operation. A reused or expired id reports `INPUT_NOT_FOUND` rather
    /// than a distinct "already claimed" code, so a caller cannot probe which ids exist.
    pub(crate) fn claim(&mut self, now_ms: u64) -> Result<(), LocalMediaErrorCode> {
        if self.claimed || self.is_expired(now_ms) {
            return Err(LocalMediaErrorCode::InputNotFound);
        }
        self.claimed = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "staged_input_tests.rs"]
mod tests;
