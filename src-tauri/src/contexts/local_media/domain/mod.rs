//! Local-media domain: engines, profiles, operations, admitted inputs, recordings, and results.

mod engine;
mod error;
mod ids;
mod image_bounds;
mod operation;
mod profile;
mod recording;
mod result;
mod staged_input;
mod validation;

pub(crate) use engine::{
    AudioDevice, AudioDeviceCatalog, EngineReadiness, EngineStatus, LocalMediaEngine,
    LocalMediaRuntimeStatus, PlatformSupport, WorkerState,
};
pub(crate) use error::{LocalMediaError, LocalMediaErrorCode};
pub(crate) use ids::{
    ComposerScopeId, LocalMediaOperationId, PlaybackId, RecordingId, StagedInputId,
};
pub(crate) use image_bounds::{exceeds_pixel_limit, image_dimensions};
pub(crate) use operation::{
    AdmissionLimits, LocalMediaOperationKind, LocalMediaPhase, LocalMediaProfileSnapshot,
};
#[cfg(test)]
pub(crate) use profile::TtsModelKind;
pub(crate) use profile::{
    FasterWhisperProfile, LocalMediaProfile, PaddleOcrProfile, SherpaOnnxTtsProfile,
    DEFAULT_PROFILE_ID, MAX_TTS_CODE_POINTS,
};
// Production code reaches the acceleration mode through `PaddleOcrProfile::cpu_acceleration` and
// never names the type, so re-exporting it unconditionally is an unused import outside tests.
#[cfg(test)]
pub(crate) use profile::OcrCpuAcceleration;
pub(crate) use recording::{
    duration_ms_for, CommittedRecording, RecordingHandle, RecordingOutcome, RecordingSummary,
};
pub(crate) use result::{
    derive_plain_text, normalize_recognized_text, LocalMediaOperationResult, OcrLine, OcrPage,
    OcrProvenance, OcrResult, OcrSourceSummary, OcrWarning, SpeechPlaybackResult,
    TranscriptionProvenance, TranscriptionResult,
};
#[cfg(test)]
pub(crate) use staged_input::STAGED_INPUT_TTL_MS;
pub(crate) use staged_input::{
    sanitize_display_name, sniff_media, OcrMediaType, SniffedFormat, StagedInputRecord,
    StagedOcrSource,
};
pub(crate) use validation::{
    classify_model_paths, first_error, validate_profile, PathClassification, ProfileFieldIssue,
};
