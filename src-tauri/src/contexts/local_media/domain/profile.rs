//! The context-owned local-media profile.
//!
//! These fields deliberately do not live in the generic `AppSettings` aggregate: they have their
//! own revision, validation, worker lifetime, and readiness semantics, and a generic settings save
//! must not be able to restart an inference worker.

use serde::{Deserialize, Serialize};

/// V1 accepts exactly one profile. The field exists so a later change can add named profiles
/// without a storage migration.
pub(crate) const DEFAULT_PROFILE_ID: &str = "default";

pub(crate) const MIN_RECORDING_SECONDS: u32 = 5;
pub(crate) const MAX_RECORDING_SECONDS: u32 = 120;
pub(crate) const MIN_RECORDING_MILLIS: u64 = 300;
pub(crate) const MAX_TTS_CODE_POINTS: usize = 4_000;
pub(crate) const MAX_PDF_PAGES_CEILING: u32 = 50;

/// Where an engine should run. `Auto` means the worker resolves it; the host does not silently
/// rewrite a failed `Cuda` into `Cpu`, because that turns a configuration error into a slow success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LocalMediaDevice {
    #[default]
    Auto,
    Cpu,
    Cuda,
}

impl LocalMediaDevice {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::Cuda => "cuda",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WhisperComputeType {
    #[default]
    Auto,
    Int8,
    Float16,
    Int8Float16,
    Float32,
}

impl WhisperComputeType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Int8 => "int8",
            Self::Float16 => "float16",
            Self::Int8Float16 => "int8_float16",
            Self::Float32 => "float32",
        }
    }
}

/// sherpa-onnx model families. Each needs a different file set, which is why this is an enum and
/// not a free-form string next to a single "model directory" field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TtsModelKind {
    #[default]
    Vits,
    Piper,
    Kokoro,
    Matcha,
}

impl TtsModelKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Vits => "vits",
            Self::Piper => "piper",
            Self::Kokoro => "kokoro",
            Self::Matcha => "matcha",
        }
    }
}

/// Whether the OCR worker asks PaddleOCR to use its CPU acceleration backend.
///
/// Three values rather than a boolean, because "the user has not chosen" and "the user chose the
/// library's own default" are the same state and it must not be confused with an explicit `enabled`:
/// `LibraryDefault` passes no argument at all, leaving the decision where PaddleOCR put it.
///
/// This exists because paddlepaddle's oneDNN executor cannot run every graph -- PP-OCRv6 raises an
/// unimplemented-operator error on load-and-run while the same models succeed with acceleration off.
/// Without a field, an affected user has no recovery path; with a silent fallback, no result would be
/// attributable to a recorded configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum OcrCpuAcceleration {
    #[default]
    LibraryDefault,
    Enabled,
    Disabled,
}

impl OcrCpuAcceleration {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LibraryDefault => "library-default",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct PaddleOcrProfile {
    pub(crate) enabled: bool,
    pub(crate) python_executable: String,
    /// Not a global flag: PaddleX builds its runners from its own pipeline configuration and never
    /// reads `FLAGS_use_mkldnn`, so a process-wide setting is silently ignored.
    pub(crate) cpu_acceleration: OcrCpuAcceleration,
    /// A PaddleX pipeline configuration, or explicit detection + recognition directories. One of
    /// the two forms is required when the engine is enabled.
    pub(crate) paddle_x_config_path: Option<String>,
    pub(crate) text_detection_model_dir: Option<String>,
    pub(crate) text_recognition_model_dir: Option<String>,
    /// Optional. Left unset, the worker disables the text-line orientation stage entirely rather
    /// than letting PaddleOCR resolve a default checkpoint over the network.
    pub(crate) text_line_orientation_model_dir: Option<String>,
    pub(crate) language: String,
    pub(crate) device: LocalMediaDevice,
    pub(crate) max_pdf_pages: u32,
}

impl Default for PaddleOcrProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            python_executable: String::new(),
            cpu_acceleration: OcrCpuAcceleration::LibraryDefault,
            paddle_x_config_path: None,
            text_detection_model_dir: None,
            text_recognition_model_dir: None,
            text_line_orientation_model_dir: None,
            language: "ch".to_string(),
            device: LocalMediaDevice::Auto,
            max_pdf_pages: 20,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct FasterWhisperProfile {
    pub(crate) enabled: bool,
    pub(crate) python_executable: String,
    pub(crate) model_directory: String,
    pub(crate) device: LocalMediaDevice,
    pub(crate) compute_type: WhisperComputeType,
    /// `"auto"` or a language tag. Stored as text because the set is the model's, not ours.
    pub(crate) language: String,
    pub(crate) vad_filter: bool,
    pub(crate) beam_size: u32,
    pub(crate) microphone_device_id: Option<String>,
    pub(crate) max_recording_seconds: u32,
}

impl Default for FasterWhisperProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            python_executable: String::new(),
            model_directory: String::new(),
            device: LocalMediaDevice::Auto,
            compute_type: WhisperComputeType::Auto,
            language: "auto".to_string(),
            vad_filter: true,
            beam_size: 5,
            microphone_device_id: None,
            max_recording_seconds: MAX_RECORDING_SECONDS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct SherpaOnnxTtsProfile {
    pub(crate) enabled: bool,
    pub(crate) python_executable: String,
    pub(crate) model_kind: TtsModelKind,
    /// For `matcha` this is the acoustic model; for every other kind it is the model itself.
    pub(crate) model_path: String,
    pub(crate) tokens_path: String,
    pub(crate) lexicon_path: Option<String>,
    pub(crate) data_dir: Option<String>,
    pub(crate) dict_dir: Option<String>,
    /// Required by `kokoro`; ignored otherwise.
    pub(crate) voices_path: Option<String>,
    /// Required by `matcha`; ignored otherwise.
    pub(crate) vocoder_path: Option<String>,
    pub(crate) rule_fsts: Vec<String>,
    pub(crate) speaker_id: u32,
    pub(crate) speed: f32,
    pub(crate) num_threads: u32,
    pub(crate) device: LocalMediaDevice,
    pub(crate) output_device_id: Option<String>,
}

impl Default for SherpaOnnxTtsProfile {
    fn default() -> Self {
        Self {
            enabled: false,
            python_executable: String::new(),
            model_kind: TtsModelKind::Vits,
            model_path: String::new(),
            tokens_path: String::new(),
            lexicon_path: None,
            data_dir: None,
            dict_dir: None,
            voices_path: None,
            vocoder_path: None,
            rule_fsts: Vec::new(),
            speaker_id: 0,
            speed: 1.0,
            num_threads: 1,
            device: LocalMediaDevice::Cpu,
            output_device_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalMediaProfile {
    pub(crate) profile_id: String,
    pub(crate) revision: i64,
    /// The master switch. Off by default so an upgrade never launches Python or touches a
    /// microphone without the user having asked for it.
    pub(crate) enabled: bool,
    pub(crate) ocr: PaddleOcrProfile,
    pub(crate) stt: FasterWhisperProfile,
    pub(crate) tts: SherpaOnnxTtsProfile,
    pub(crate) updated_at: String,
}

impl LocalMediaProfile {
    /// The row written on first read. Revision starts at 0 so the first successful save is 1.
    pub(crate) fn disabled_default(updated_at: String) -> Self {
        Self {
            profile_id: DEFAULT_PROFILE_ID.to_string(),
            revision: 0,
            enabled: false,
            ocr: PaddleOcrProfile::default(),
            stt: FasterWhisperProfile::default(),
            tts: SherpaOnnxTtsProfile::default(),
            updated_at,
        }
    }

    pub(crate) fn engine_enabled(&self, engine: super::engine::LocalMediaEngine) -> bool {
        if !self.enabled {
            return false;
        }
        match engine {
            super::engine::LocalMediaEngine::Ocr => self.ocr.enabled,
            super::engine::LocalMediaEngine::Stt => self.stt.enabled,
            super::engine::LocalMediaEngine::Tts => self.tts.enabled,
        }
    }

    /// Clamped to the hard ceiling regardless of what was persisted: a stored value from a future
    /// build must not be able to raise a bound the current build enforces.
    pub(crate) fn recording_limit_seconds(&self) -> u32 {
        self.stt
            .max_recording_seconds
            .clamp(MIN_RECORDING_SECONDS, MAX_RECORDING_SECONDS)
    }
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
