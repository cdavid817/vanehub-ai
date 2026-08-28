//! Microphone capture as the domain sees it: durations, ownership, and the commit decision.
//!
//! No sample data appears in this module. The bytes stay in infrastructure and reach the worker as
//! a file path; what crosses this boundary is counts and identifiers.

use super::ids::{ComposerScopeId, RecordingId};
use super::profile::MIN_RECORDING_MILLIS;
use serde::{Deserialize, Serialize};

/// What the frontend receives when capture starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecordingHandle {
    pub(crate) recording_id: RecordingId,
    pub(crate) started_at: String,
    pub(crate) max_duration_ms: u64,
}

/// A finalized WAV, described without naming it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommittedRecording {
    pub(crate) recording_id: RecordingId,
    pub(crate) duration_ms: u64,
    pub(crate) sample_rate: u32,
    pub(crate) sample_count: u64,
    /// True when capture stopped because it hit the ceiling rather than because the user released.
    /// A warning on a successful transcription, never a failure.
    pub(crate) limit_reached: bool,
}

/// Host-side bookkeeping for the one recording that may be active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordingSummary {
    pub(crate) recording_id: RecordingId,
    pub(crate) composer_scope: ComposerScopeId,
    pub(crate) started_at_ms: u64,
    pub(crate) max_duration_ms: u64,
}

impl RecordingSummary {
    /// Both the id and the scope must match.
    ///
    /// The id alone would be enough to stop a recording if ids were unguessable, but they travel
    /// over IPC to a renderer that hosts more than one composer; requiring the scope means a stale
    /// controller cannot cancel the recording a different one just started.
    pub(crate) fn owned_by(&self, recording_id: &RecordingId, scope: &ComposerScopeId) -> bool {
        &self.recording_id == recording_id && &self.composer_scope == scope
    }
}

/// The commit decision for a finalized capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordingOutcome {
    Committed { limit_reached: bool },
    TooShort,
}

impl RecordingOutcome {
    pub(crate) fn evaluate(duration_ms: u64, max_duration_ms: u64) -> Self {
        if duration_ms < MIN_RECORDING_MILLIS {
            return Self::TooShort;
        }
        Self::Committed {
            limit_reached: duration_ms >= max_duration_ms,
        }
    }
}

/// Milliseconds for a sample count at a rate. A zero rate yields zero rather than panicking: an
/// input device that reports no sample rate is a device failure the caller maps to a stable error,
/// and a divide-by-zero would take the process with it.
pub(crate) fn duration_ms_for(sample_count: u64, sample_rate: u32) -> u64 {
    if sample_rate == 0 {
        return 0;
    }
    sample_count.saturating_mul(1_000) / u64::from(sample_rate)
}

#[cfg(test)]
#[path = "recording_tests.rs"]
mod tests;
