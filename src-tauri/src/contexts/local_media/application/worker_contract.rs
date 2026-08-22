//! Typed request/reply pairs for the engine workers.
//!
//! The wire form is JSON Lines, but that stays in infrastructure. Crossing this boundary with a
//! `serde_json::Value` would put an untyped map in an application port and move the shape of the
//! protocol into the use cases, where a worker-side field rename becomes a silent behaviour change.

use super::super::domain::{LocalMediaEngine, OcrMediaType};
use std::path::PathBuf;

/// What the host asks a worker to do. Exactly one variant per engine plus the shared probe.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum WorkerCall {
    Probe,
    Ocr(OcrWorkerRequest),
    Transcribe(SttWorkerRequest),
    Synthesize(TtsWorkerRequest),
}

impl WorkerCall {
    pub(crate) fn method(&self, engine: LocalMediaEngine) -> &'static str {
        match self {
            Self::Probe => "probe",
            _ => engine.inference_method(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OcrWorkerRequest {
    /// The staged copy, never the path the user picked.
    pub(crate) source_path: PathBuf,
    pub(crate) media_type: OcrMediaType,
    pub(crate) max_pdf_pages: u32,
    pub(crate) max_output_characters: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SttWorkerRequest {
    pub(crate) audio_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TtsWorkerRequest {
    pub(crate) text: String,
    /// Pre-authorized by the host. A reply naming anything else is treated as protocol-invalid
    /// rather than followed.
    pub(crate) output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkerLine {
    pub(crate) text: String,
    pub(crate) confidence: Option<f32>,
    pub(crate) polygon: Option<Vec<(f32, f32)>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkerPage {
    pub(crate) page_number: u32,
    pub(crate) text: String,
    pub(crate) line_count: u32,
    /// Per-line detail. Empty when the engine reported only joined page text; the composer never
    /// needs it, and OnePiece's tool contract degrades to text-only rather than failing.
    pub(crate) lines: Vec<WorkerLine>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ProbeReply {
    pub(crate) package_version: Option<String>,
    pub(crate) device: Option<String>,
    pub(crate) model_identity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct OcrReply {
    pub(crate) pages: Vec<WorkerPage>,
    pub(crate) character_count: u32,
    pub(crate) truncated: bool,
    /// The worker distinguishes "ran and found nothing" from "failed"; the host must not collapse
    /// the two into a crash.
    pub(crate) no_text_detected: bool,
    pub(crate) engine_version: Option<String>,
    pub(crate) model_identity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TranscribeReply {
    pub(crate) text: String,
    pub(crate) detected_language: Option<String>,
    pub(crate) language_probability: Option<f32>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) no_speech_detected: bool,
    pub(crate) engine_version: Option<String>,
    pub(crate) device: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct SynthesizeReply {
    pub(crate) audio_path: PathBuf,
    pub(crate) sample_rate: u32,
    pub(crate) sample_count: u64,
    pub(crate) duration_ms: u64,
    pub(crate) engine_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum WorkerReply {
    Probe(ProbeReply),
    Ocr(OcrReply),
    Transcribe(TranscribeReply),
    Synthesize(SynthesizeReply),
}
