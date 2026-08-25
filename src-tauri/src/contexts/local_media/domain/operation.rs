//! Operation kinds, phases, controller limits, and the immutable per-operation profile snapshot.

use super::engine::LocalMediaEngine;
use super::ids::{ComposerScopeId, LocalMediaOperationId};
use super::profile::{
    FasterWhisperProfile, LocalMediaProfile, PaddleOcrProfile, SherpaOnnxTtsProfile,
};
use serde::{Deserialize, Serialize};

/// The four operation kinds this context registers. Spelled with the context prefix so an
/// operation list is readable without a lookup table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum LocalMediaOperationKind {
    Probe,
    Ocr,
    Stt,
    Tts,
}

impl LocalMediaOperationKind {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) const ALL: [Self; 4] = [Self::Probe, Self::Ocr, Self::Stt, Self::Tts];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Probe => "local-media.probe",
            Self::Ocr => "local-media.ocr",
            Self::Stt => "local-media.stt",
            Self::Tts => "local-media.tts",
        }
    }

    /// Exercised by this module's tests; production reads the state it derives, not this.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_str() == value)
    }

    /// The operation label shown in history. Generic by construction: a transcript fragment must
    /// never become an operation title.
    pub(crate) fn message_key(self) -> &'static str {
        match self {
            Self::Probe => "localMedia.operations.probe",
            Self::Ocr => "localMedia.operations.ocr",
            Self::Stt => "localMedia.operations.stt",
            Self::Tts => "localMedia.operations.tts",
        }
    }
}

/// Observable progress. `Playing` is terminal-adjacent rather than terminal: the operation is still
/// cancellable while audio is on the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum LocalMediaPhase {
    Accepted,
    FinalizingRecording,
    Queued,
    LoadingEngine,
    Processing,
    GeneratingAudio,
    Playing,
    Succeeded,
    Failed,
    Cancelled,
}

impl LocalMediaPhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::FinalizingRecording => "finalizing-recording",
            Self::Queued => "queued",
            Self::LoadingEngine => "loading-engine",
            Self::Processing => "processing",
            Self::GeneratingAudio => "generating-audio",
            Self::Playing => "playing",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

/// Controller-owned bounds. These are ceilings the profile can narrow but never raise, which is why
/// they live here rather than in the profile: a persisted row from a future build must not be able
/// to widen what this build is willing to process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdmissionLimits {
    pub(crate) max_input_bytes: u64,
    pub(crate) max_pdf_pages: u32,
    pub(crate) max_decoded_pixels: u64,
    pub(crate) max_output_characters: u32,
    pub(crate) engine_timeout_ms: u64,
}

impl AdmissionLimits {
    pub(crate) const HARD_CEILING: Self = Self {
        max_input_bytes: 50 * 1024 * 1024,
        max_pdf_pages: 50,
        max_decoded_pixels: 50_000_000,
        max_output_characters: 200_000,
        engine_timeout_ms: 300_000,
    };

    /// Narrow the ceiling with a profile's PDF-page preference. Values above the ceiling are
    /// clamped rather than rejected: the profile validator already refuses them at save time, and
    /// an operation is not the place to re-litigate stored data.
    pub(crate) fn for_ocr(profile: &PaddleOcrProfile) -> Self {
        Self {
            max_pdf_pages: profile
                .max_pdf_pages
                .clamp(1, Self::HARD_CEILING.max_pdf_pages),
            ..Self::HARD_CEILING
        }
    }
}

impl Default for AdmissionLimits {
    fn default() -> Self {
        Self::HARD_CEILING
    }
}

/// Everything an accepted operation is allowed to depend on.
///
/// Fields are private and there are no setters: a settings save while an operation runs must be
/// invisible to that operation. `capture` clones the sub-profile rather than borrowing it, so the
/// snapshot survives the profile being replaced in the repository.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LocalMediaProfileSnapshot {
    operation_id: LocalMediaOperationId,
    kind: LocalMediaOperationKind,
    engine: LocalMediaEngine,
    profile_revision: i64,
    created_at: String,
    composer_scope: Option<ComposerScopeId>,
    ocr: PaddleOcrProfile,
    stt: FasterWhisperProfile,
    tts: SherpaOnnxTtsProfile,
    limits: AdmissionLimits,
}

impl LocalMediaProfileSnapshot {
    pub(crate) fn capture(
        operation_id: LocalMediaOperationId,
        kind: LocalMediaOperationKind,
        engine: LocalMediaEngine,
        profile: &LocalMediaProfile,
        composer_scope: Option<ComposerScopeId>,
        created_at: String,
    ) -> Self {
        Self {
            operation_id,
            kind,
            engine,
            profile_revision: profile.revision,
            created_at,
            composer_scope,
            ocr: profile.ocr.clone(),
            stt: profile.stt.clone(),
            tts: profile.tts.clone(),
            limits: AdmissionLimits::for_ocr(&profile.ocr),
        }
    }

    pub(crate) fn operation_id(&self) -> &LocalMediaOperationId {
        &self.operation_id
    }

    /// Exercised by this module's tests; production reads the state it derives, not this.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn kind(&self) -> LocalMediaOperationKind {
        self.kind
    }

    pub(crate) fn engine(&self) -> LocalMediaEngine {
        self.engine
    }

    pub(crate) fn profile_revision(&self) -> i64 {
        self.profile_revision
    }

    /// Exercised by this module's tests; production reads the state it derives, not this.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn created_at(&self) -> &str {
        &self.created_at
    }

    /// Exercised by this module's tests; production reads the state it derives, not this.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn composer_scope(&self) -> Option<&ComposerScopeId> {
        self.composer_scope.as_ref()
    }

    pub(crate) fn ocr(&self) -> &PaddleOcrProfile {
        &self.ocr
    }

    pub(crate) fn stt(&self) -> &FasterWhisperProfile {
        &self.stt
    }

    pub(crate) fn tts(&self) -> &SherpaOnnxTtsProfile {
        &self.tts
    }

    pub(crate) fn limits(&self) -> AdmissionLimits {
        self.limits
    }

    /// The Python executable for this snapshot's engine. Resolved from the snapshot rather than the
    /// live profile so a mid-operation save cannot redirect a running worker.
    pub(crate) fn python_executable(&self) -> &str {
        match self.engine {
            LocalMediaEngine::Ocr => &self.ocr.python_executable,
            LocalMediaEngine::Stt => &self.stt.python_executable,
            LocalMediaEngine::Tts => &self.tts.python_executable,
        }
    }
}

#[cfg(test)]
#[path = "operation_tests.rs"]
mod tests;
