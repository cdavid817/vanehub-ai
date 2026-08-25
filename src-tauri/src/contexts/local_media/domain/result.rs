//! Typed terminal results.
//!
//! These are the only structures carrying recognized or synthesized content, and they travel
//! exactly one route: the typed result lookup, read by the composer that started the operation.
//! Nothing here is written to the unified log or attached to an operation label.

use super::engine::LocalMediaRuntimeStatus;
use super::error::LocalMediaErrorCode;
use super::ids::PlaybackId;
use super::staged_input::OcrMediaType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrSourceSummary {
    pub(crate) display_name: String,
    pub(crate) media_type: OcrMediaType,
    pub(crate) page_count: u32,
}

/// One recognized line.
///
/// The composer only ever renders `text`, but OnePiece's tool contract exposes geometry and
/// confidence to the agent, and both entry points read the same result. Carrying them here is what
/// keeps the shared runtime from forcing the tool contract to shrink.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrLine {
    pub(crate) text: String,
    /// Absent when the engine did not report one. Never defaulted to a number: an invented
    /// confidence is worse than an honest unknown.
    pub(crate) confidence: Option<f32>,
    /// Bounding polygon in engine coordinates, when the pipeline produced one.
    pub(crate) polygon: Option<Vec<(f32, f32)>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrPage {
    pub(crate) page_number: u32,
    pub(crate) text: String,
    pub(crate) line_count: u32,
    #[serde(default)]
    pub(crate) lines: Vec<OcrLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrWarning {
    pub(crate) code: String,
    pub(crate) message_key: String,
    pub(crate) page_number: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrProvenance {
    pub(crate) engine: String,
    pub(crate) engine_version: Option<String>,
    pub(crate) profile_revision: i64,
    pub(crate) language: String,
    /// A safe label such as `det+rec:ch`. Never a directory.
    pub(crate) model_identity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrResult {
    pub(crate) source: OcrSourceSummary,
    pub(crate) plain_text: String,
    pub(crate) pages: Vec<OcrPage>,
    pub(crate) warnings: Vec<OcrWarning>,
    pub(crate) provenance: OcrProvenance,
    pub(crate) character_count: u32,
    pub(crate) truncated: bool,
}

impl OcrResult {
    pub(crate) fn is_empty(&self) -> bool {
        self.plain_text.trim().is_empty()
    }

    /// The informational code the review dialog renders when nothing was recognized. `None` for a
    /// successful read: an outcome code here is not an error, so the absence of one is the normal
    /// case rather than the exceptional one.
    ///
    /// The frontend re-derives this from `plainText` because the result crosses IPC as data; the
    /// Rust copy is what pins the rule in a test.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn outcome_code(&self) -> Option<LocalMediaErrorCode> {
        self.is_empty()
            .then_some(LocalMediaErrorCode::NoTextDetected)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TranscriptionProvenance {
    pub(crate) engine: String,
    pub(crate) engine_version: Option<String>,
    pub(crate) profile_revision: i64,
    pub(crate) device: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TranscriptionResult {
    pub(crate) text: String,
    pub(crate) detected_language: Option<String>,
    pub(crate) language_probability: Option<f32>,
    pub(crate) duration_ms: Option<u64>,
    /// Capture stopped at the ceiling. The utterance is complete up to that point and was
    /// transcribed; this is a warning attached to a success.
    pub(crate) limit_reached: bool,
    pub(crate) provenance: TranscriptionProvenance,
}

impl TranscriptionResult {
    pub(crate) fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn outcome_code(&self) -> Option<LocalMediaErrorCode> {
        self.is_empty()
            .then_some(LocalMediaErrorCode::NoSpeechDetected)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SpeechPlaybackResult {
    pub(crate) playback_id: PlaybackId,
    pub(crate) sample_rate: u32,
    pub(crate) duration_ms: u64,
    pub(crate) device_summary: Option<String>,
}

/// The discriminated result the frontend reads. Separate from the generic operation record because
/// that record's `result` field is untyped JSON shared by every context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "result", rename_all = "lowercase")]
pub(crate) enum LocalMediaOperationResult {
    Probe(LocalMediaRuntimeStatus),
    Ocr(OcrResult),
    Stt(TranscriptionResult),
    Tts(SpeechPlaybackResult),
}

/// Join page text into the string the composer appends.
///
/// Empty pages are skipped rather than contributing an empty segment: a blank page in a scan would
/// otherwise show up as a paragraph break the user has to delete. The page still appears in
/// `pages`, so the page count remains truthful.
pub(crate) fn derive_plain_text(pages: &[OcrPage]) -> String {
    pages
        .iter()
        .map(|page| page.text.as_str())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Normalize line endings and outer whitespace, and nothing else.
///
/// Recognized punctuation is left exactly as the engine reported it. "Correcting" full-width
/// punctuation or collapsing internal whitespace would silently edit the user's document.
pub(crate) fn normalize_recognized_text(raw: &str) -> String {
    raw.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\0', "")
        .trim()
        .to_string()
}

#[cfg(test)]
#[path = "result_tests.rs"]
mod tests;
