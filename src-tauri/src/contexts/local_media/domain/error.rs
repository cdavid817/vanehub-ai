//! Stable local-media error vocabulary.
//!
//! The frontend localizes by `code`; no layer below it produces user-facing prose. `safe_details`
//! is an allowlist of scalars rather than a free map, because the things that would naturally be
//! attached here -- a model path, a Python traceback, the recognized text -- are exactly what the
//! privacy requirement excludes from logs and telemetry.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Every terminal condition the context can report. Adding a variant requires a locale key in all
/// five registered catalogs; the parity test is what enforces that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum LocalMediaErrorCode {
    LocalMediaNativeOnly,
    LocalMediaDisabled,
    EngineDisabled,
    EngineUnconfigured,
    EngineBusy,
    EngineUnavailable,
    PythonNotFound,
    PythonExecutionDenied,
    EngineImportFailed,
    EngineVersionUnsupported,
    ProfileRevisionConflict,
    ModelNotConfigured,
    ModelNotFound,
    ModelIncompatible,
    ModelDownloadBlocked,
    DeviceConfigurationInvalid,
    MicPermissionDenied,
    MicDeviceUnavailable,
    AudioCaptureStartFailed,
    AudioCaptureOverrun,
    RecordingAlreadyActive,
    RecordingNotFound,
    RecordingTooShort,
    RecordingLimitReached,
    InputNotFound,
    InputTooLarge,
    UnsupportedMediaType,
    PdfPageLimitExceeded,
    ImagePixelLimitExceeded,
    NoTextDetected,
    NoSpeechDetected,
    TtsTextTooLong,
    PlaybackDeviceUnavailable,
    WorkerStartFailed,
    WorkerCrashed,
    WorkerProtocolError,
    OperationCancelled,
    OperationResultExpired,
    TempStorageFailed,
    TempCleanupFailed,
    /// Vendor-compatibility codes. Each names a third-party limitation the user can act on, which
    /// a generic `EngineUnavailable` cannot: one is fixed by a setting, the others by moving files.
    PaddleOnednnModelIncompatible,
    ModelPathEncodingUnsupported,
    TtsDataPathEncodingUnsupported,
    TtsPhonemizerDataUnavailable,
}

impl LocalMediaErrorCode {
    /// The wire spelling. Kept as an explicit table rather than derived from the variant name so a
    /// rename cannot silently change a contract the frontend and the Python bridge both depend on.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LocalMediaNativeOnly => "LOCAL_MEDIA_NATIVE_ONLY",
            Self::LocalMediaDisabled => "LOCAL_MEDIA_DISABLED",
            Self::EngineDisabled => "ENGINE_DISABLED",
            Self::EngineUnconfigured => "ENGINE_UNCONFIGURED",
            Self::EngineBusy => "ENGINE_BUSY",
            Self::EngineUnavailable => "ENGINE_UNAVAILABLE",
            Self::PythonNotFound => "PYTHON_NOT_FOUND",
            Self::PythonExecutionDenied => "PYTHON_EXECUTION_DENIED",
            Self::EngineImportFailed => "ENGINE_IMPORT_FAILED",
            Self::EngineVersionUnsupported => "ENGINE_VERSION_UNSUPPORTED",
            Self::ProfileRevisionConflict => "PROFILE_REVISION_CONFLICT",
            Self::ModelNotConfigured => "MODEL_NOT_CONFIGURED",
            Self::ModelNotFound => "MODEL_NOT_FOUND",
            Self::ModelIncompatible => "MODEL_INCOMPATIBLE",
            Self::ModelDownloadBlocked => "MODEL_DOWNLOAD_BLOCKED",
            Self::DeviceConfigurationInvalid => "DEVICE_CONFIGURATION_INVALID",
            Self::MicPermissionDenied => "MIC_PERMISSION_DENIED",
            Self::MicDeviceUnavailable => "MIC_DEVICE_UNAVAILABLE",
            Self::AudioCaptureStartFailed => "AUDIO_CAPTURE_START_FAILED",
            Self::AudioCaptureOverrun => "AUDIO_CAPTURE_OVERRUN",
            Self::RecordingAlreadyActive => "RECORDING_ALREADY_ACTIVE",
            Self::RecordingNotFound => "RECORDING_NOT_FOUND",
            Self::RecordingTooShort => "RECORDING_TOO_SHORT",
            Self::RecordingLimitReached => "RECORDING_LIMIT_REACHED",
            Self::InputNotFound => "INPUT_NOT_FOUND",
            Self::InputTooLarge => "INPUT_TOO_LARGE",
            Self::UnsupportedMediaType => "UNSUPPORTED_MEDIA_TYPE",
            Self::PdfPageLimitExceeded => "PDF_PAGE_LIMIT_EXCEEDED",
            Self::ImagePixelLimitExceeded => "IMAGE_PIXEL_LIMIT_EXCEEDED",
            Self::NoTextDetected => "NO_TEXT_DETECTED",
            Self::NoSpeechDetected => "NO_SPEECH_DETECTED",
            Self::TtsTextTooLong => "TTS_TEXT_TOO_LONG",
            Self::PlaybackDeviceUnavailable => "PLAYBACK_DEVICE_UNAVAILABLE",
            Self::WorkerStartFailed => "WORKER_START_FAILED",
            Self::WorkerCrashed => "WORKER_CRASHED",
            Self::WorkerProtocolError => "WORKER_PROTOCOL_ERROR",
            Self::OperationCancelled => "OPERATION_CANCELLED",
            Self::OperationResultExpired => "OPERATION_RESULT_EXPIRED",
            Self::TempStorageFailed => "TEMP_STORAGE_FAILED",
            Self::TempCleanupFailed => "TEMP_CLEANUP_FAILED",
            Self::PaddleOnednnModelIncompatible => "PADDLE_ONEDNN_MODEL_INCOMPATIBLE",
            Self::ModelPathEncodingUnsupported => "MODEL_PATH_ENCODING_UNSUPPORTED",
            Self::TtsDataPathEncodingUnsupported => "TTS_DATA_PATH_ENCODING_UNSUPPORTED",
            Self::TtsPhonemizerDataUnavailable => "TTS_PHONEMIZER_DATA_UNAVAILABLE",
        }
    }

    /// Parse a code produced by the Python bridge. An unrecognized code is not coerced into a
    /// neighbour: the worker is speaking a protocol the host does not know.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|code| code.as_str() == value)
    }

    /// The locale key the frontend renders. Derived mechanically from the wire spelling so a new
    /// code cannot be added without a matching catalog entry being obvious.
    pub(crate) fn message_key(self) -> String {
        let mut key = String::from("localMedia.errors.");
        let mut capitalize = false;
        for character in self.as_str().chars() {
            if character == '_' {
                capitalize = true;
                continue;
            }
            let lowered = character.to_ascii_lowercase();
            if key.len() == "localMedia.errors.".len() {
                key.push(lowered);
            } else if capitalize {
                key.push(character.to_ascii_uppercase());
                capitalize = false;
            } else {
                key.push(lowered);
            }
        }
        key
    }

    pub(crate) const ALL: [Self; 44] = [
        Self::LocalMediaNativeOnly,
        Self::LocalMediaDisabled,
        Self::EngineDisabled,
        Self::EngineUnconfigured,
        Self::EngineBusy,
        Self::EngineUnavailable,
        Self::PythonNotFound,
        Self::PythonExecutionDenied,
        Self::EngineImportFailed,
        Self::EngineVersionUnsupported,
        Self::ProfileRevisionConflict,
        Self::ModelNotConfigured,
        Self::ModelNotFound,
        Self::ModelIncompatible,
        Self::ModelDownloadBlocked,
        Self::DeviceConfigurationInvalid,
        Self::MicPermissionDenied,
        Self::MicDeviceUnavailable,
        Self::AudioCaptureStartFailed,
        Self::AudioCaptureOverrun,
        Self::RecordingAlreadyActive,
        Self::RecordingNotFound,
        Self::RecordingTooShort,
        Self::RecordingLimitReached,
        Self::InputNotFound,
        Self::InputTooLarge,
        Self::UnsupportedMediaType,
        Self::PdfPageLimitExceeded,
        Self::ImagePixelLimitExceeded,
        Self::NoTextDetected,
        Self::NoSpeechDetected,
        Self::TtsTextTooLong,
        Self::PlaybackDeviceUnavailable,
        Self::WorkerStartFailed,
        Self::WorkerCrashed,
        Self::WorkerProtocolError,
        Self::OperationCancelled,
        Self::OperationResultExpired,
        Self::TempStorageFailed,
        Self::TempCleanupFailed,
        Self::PaddleOnednnModelIncompatible,
        Self::ModelPathEncodingUnsupported,
        Self::TtsDataPathEncodingUnsupported,
        Self::TtsPhonemizerDataUnavailable,
    ];
}

impl fmt::Display for LocalMediaErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A scalar attached to an error for diagnostics. Deliberately not `String`-open: a free-form
/// string field is where a path ends up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum SafeDetail {
    Text(String),
    Number(i64),
    Flag(bool),
}

/// The single error type crossing every local-media boundary.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LocalMediaError {
    code: LocalMediaErrorCode,
    details: BTreeMap<String, SafeDetail>,
}

impl LocalMediaError {
    pub(crate) fn new(code: LocalMediaErrorCode) -> Self {
        Self {
            code,
            details: BTreeMap::new(),
        }
    }

    pub(crate) fn with_text(mut self, key: &str, value: impl Into<String>) -> Self {
        let text = value.into();
        // 64 characters is enough for an engine name, a field name, or a version, and short enough
        // that a path cannot survive intact.
        if text.len() <= 64 && text.chars().all(|character| !character.is_control()) {
            self.details.insert(key.to_string(), SafeDetail::Text(text));
        }
        self
    }

    pub(crate) fn with_number(mut self, key: &str, value: i64) -> Self {
        self.details
            .insert(key.to_string(), SafeDetail::Number(value));
        self
    }

    pub(crate) fn with_flag(mut self, key: &str, value: bool) -> Self {
        self.details
            .insert(key.to_string(), SafeDetail::Flag(value));
        self
    }

    pub(crate) fn code(&self) -> LocalMediaErrorCode {
        self.code
    }

    /// Test-only.
    ///
    /// Nothing in production reads the details back: the command contract serializes a bare
    /// code, so the map's job is done at construction time, where `with_text` filters out anything
    /// that could carry a path. The accessor exists so the privacy tests can assert that filtering
    /// actually happened.
    #[cfg(test)]
    pub(crate) fn details(&self) -> &BTreeMap<String, SafeDetail> {
        &self.details
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.code == LocalMediaErrorCode::OperationCancelled
    }
}

impl fmt::Display for LocalMediaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Only the code. Anything richer would reach the unified log through `Display`.
        formatter.write_str(self.code.as_str())
    }
}

impl std::error::Error for LocalMediaError {}

impl From<LocalMediaErrorCode> for LocalMediaError {
    fn from(code: LocalMediaErrorCode) -> Self {
        Self::new(code)
    }
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
