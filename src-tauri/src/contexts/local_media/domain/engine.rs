//! Engine identity, readiness, worker state, and platform support.

use super::error::LocalMediaErrorCode;
use serde::{Deserialize, Serialize};

/// The three local engines, named by the capability they provide rather than by the package that
/// implements it. The package name is an implementation detail the frontend never branches on;
/// the capability is what the composer enables a control for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LocalMediaEngine {
    Ocr,
    Stt,
    Tts,
}

impl LocalMediaEngine {
    pub(crate) const ALL: [Self; 3] = [Self::Ocr, Self::Stt, Self::Tts];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ocr => "ocr",
            Self::Stt => "stt",
            Self::Tts => "tts",
        }
    }

    /// The identifier the Python bridge answers to. This is the `--engine` argument and the value
    /// in the worker's hello frame; a mismatch means the supervisor launched the wrong module.
    pub(crate) fn worker_id(self) -> &'static str {
        match self {
            Self::Ocr => "paddleocr",
            Self::Stt => "faster-whisper",
            Self::Tts => "sherpa-onnx",
        }
    }

    /// The one inference method each worker exposes beyond `probe`/`cancel`/`shutdown`.
    pub(crate) fn inference_method(self) -> &'static str {
        match self {
            Self::Ocr => "ocr",
            Self::Stt => "transcribe",
            Self::Tts => "synthesize",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "ocr" => Some(Self::Ocr),
            "stt" => Some(Self::Stt),
            "tts" => Some(Self::Tts),
            _ => None,
        }
    }
}

/// Per-engine readiness.
///
/// The settings page shows seven badges but there are six states: `RestartRequired` renders as
/// "Needs check" when no worker has run under the current revision and as "Restart required" when
/// one is still alive on an older one. Splitting that into two domain states would make the two
/// spellings a persisted fact instead of a rendering decision derived from `WorkerState`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub(crate) enum EngineReadiness {
    Disabled,
    Unconfigured,
    Checking,
    Ready,
    #[serde(rename_all = "camelCase")]
    Unavailable {
        code: LocalMediaErrorCode,
        /// The single profile field the failure is attributable to, when the engine named one.
        ///
        /// `None` rather than a guess: with several paths configured, blaming the first one in a
        /// list sends the user to edit a field that is fine. Only a field the worker itself
        /// reported, or one the host can prove opened the failure, appears here.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        field: Option<String>,
    },
    RestartRequired,
}

impl EngineReadiness {
    /// Whether a composer control backed by this engine may be activated. Deliberately narrower
    /// than "not an error": probing and restart-required are transient but still not usable.
    pub(crate) fn permits_operation(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Supervisor-visible lifecycle of one engine's worker slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum WorkerState {
    Stopped,
    Starting,
    Idle,
    Busy,
    Restarting,
    /// Repeated startup failures. The slot stays here until the user probes again or saves a new
    /// profile, so a broken environment cannot turn into a respawn loop.
    Quarantined,
}

impl WorkerState {
    /// Exercised by this module's tests; production reads the state it derives, not this.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn is_running(self) -> bool {
        matches!(self, Self::Idle | Self::Busy)
    }
}

/// How well the current platform is expected to support local media.
///
/// A boolean would collapse "we test this" and "this compiles and might work", which is the
/// distinction a user needs before they invest in installing three inference stacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PlatformSupport {
    Supported,
    Experimental,
    Unsupported,
}

impl PlatformSupport {
    /// Resolve from the compile-time target. Tier 1 is Windows x64, Linux x64, and Apple Silicon;
    /// Intel macOS and Linux arm64 build and are expected to work but are not part of the verified
    /// matrix; anything else is refused rather than half-offered.
    pub(crate) fn current() -> Self {
        Self::for_target(std::env::consts::OS, std::env::consts::ARCH)
    }

    pub(crate) fn for_target(os: &str, arch: &str) -> Self {
        match (os, arch) {
            ("windows", "x86_64") | ("linux", "x86_64") | ("macos", "aarch64") => Self::Supported,
            ("macos", "x86_64") | ("linux", "aarch64") => Self::Experimental,
            _ => Self::Unsupported,
        }
    }

    pub(crate) fn permits_operation(self) -> bool {
        matches!(self, Self::Supported | Self::Experimental)
    }
}

/// Runtime status for one engine. Carries only safe metadata: no executable or model path reaches
/// this struct, because it is serialized into status responses and operation diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EngineStatus {
    pub(crate) engine: LocalMediaEngine,
    pub(crate) readiness: EngineReadiness,
    pub(crate) profile_revision: i64,
    pub(crate) worker_state: WorkerState,
    pub(crate) installed_version: Option<String>,
    pub(crate) model_identity: Option<String>,
    pub(crate) device_summary: Option<String>,
    pub(crate) last_checked_at: Option<String>,
}

impl EngineStatus {
    pub(crate) fn disabled(engine: LocalMediaEngine, profile_revision: i64) -> Self {
        Self {
            engine,
            readiness: EngineReadiness::Disabled,
            profile_revision,
            worker_state: WorkerState::Stopped,
            installed_version: None,
            model_identity: None,
            device_summary: None,
            last_checked_at: None,
        }
    }
}

/// The whole-context status the settings page and composer both read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalMediaRuntimeStatus {
    /// False in Web/mock mode. The composer keeps its controls visible and disabled rather than
    /// hiding them, so this is a rendering input, not a feature flag.
    pub(crate) native_available: bool,
    pub(crate) platform_support: PlatformSupport,
    pub(crate) enabled: bool,
    pub(crate) profile_revision: i64,
    pub(crate) engines: Vec<EngineStatus>,
    /// Every model-related field's path shape, whether or not anything is wrong with it.
    ///
    /// Reported unconditionally rather than only on failure: a non-ASCII path is a description and
    /// not a verdict -- faster-whisper reads them -- so the settings page needs the shape available
    /// even when the engine is `Ready`.
    #[serde(default)]
    pub(crate) path_classifications: Vec<crate::contexts::local_media::domain::PathClassification>,
}

impl LocalMediaRuntimeStatus {
    pub(crate) fn engine(&self, engine: LocalMediaEngine) -> Option<&EngineStatus> {
        self.engines.iter().find(|status| status.engine == engine)
    }

    /// Whether the given capability can start an operation right now. One engine's failure never
    /// participates in another's answer.
    pub(crate) fn permits(&self, engine: LocalMediaEngine) -> bool {
        self.native_available
            && self.enabled
            && self.platform_support.permits_operation()
            && self
                .engine(engine)
                .is_some_and(|status| status.readiness.permits_operation())
    }
}

/// One selectable audio device. The identifier is opaque to the frontend and the label is whatever
/// the OS reported, truncated by the adapter before it gets here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioDevice {
    pub(crate) device_id: String,
    pub(crate) label: String,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioDeviceCatalog {
    pub(crate) inputs: Vec<AudioDevice>,
    pub(crate) outputs: Vec<AudioDevice>,
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
