//! Consuming-side contracts for everything outside the context's own reasoning.
//!
//! Each port is behaviour-oriented and returns domain types. None of them exposes a
//! `rusqlite::Connection`, an `AppHandle`, a process handle, or an untyped map, which is what keeps
//! the use cases testable with doubles instead of a real Python environment and a real microphone.

use super::super::domain::{
    AudioDeviceCatalog, CommittedRecording, LocalMediaEngine, LocalMediaError, LocalMediaProfile,
    LocalMediaProfileSnapshot, PlaybackId, PythonEnvironmentDiscovery, RecordingId,
    RecordingSummary, StagedInputId, StagedOcrSource, WorkerState,
};
use super::worker_contract::{WorkerCall, WorkerReply};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Persistence for the single versioned profile.
pub(crate) trait LocalMediaProfileRepository: Send + Sync {
    /// Returns the stored profile, inserting disabled defaults when no row exists. Reading must
    /// never fail because of a missing row: first use is the common case.
    fn load(&self) -> Result<LocalMediaProfile, LocalMediaError>;

    /// Commit a new revision. Fails with `PROFILE_REVISION_CONFLICT` when `expected_revision` does
    /// not match what is stored, leaving the stored row untouched.
    fn save(
        &self,
        profile: &LocalMediaProfile,
        expected_revision: i64,
    ) -> Result<LocalMediaProfile, LocalMediaError>;
}

/// Bounded host inspection used only by the settings surface.
pub(crate) trait PythonEnvironmentDiscoveryPort: Send + Sync {
    fn discover(&self, configured_paths: &[PathBuf]) -> PythonEnvironmentDiscovery;
}

pub(crate) trait LocalMediaClock: Send + Sync {
    fn now_iso(&self) -> String;
    /// Monotonic-enough milliseconds for TTL and duration arithmetic. Not a wall clock.
    fn now_ms(&self) -> u64;
}

/// Mints the opaque identifiers this context hands out.
pub(crate) trait OpaqueIdFactory: Send + Sync {
    fn next(&self, prefix: &str) -> String;
}

/// A staged file that an operation has taken ownership of.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClaimedInput {
    pub(crate) staged_input_id: StagedInputId,
    pub(crate) source: StagedOcrSource,
    pub(crate) path: PathBuf,
}

/// Ephemeral media on disk. The only component allowed to name a file.
pub(crate) trait MediaTempStore: Send + Sync {
    /// Validate, sniff, bound, and copy a user-selected file into a fresh staging directory.
    fn stage_ocr_source(&self, source: &Path) -> Result<StagedOcrSource, LocalMediaError>;

    /// Copy already-verified bytes (a managed artifact) into staging without touching a host path.
    fn stage_bytes(
        &self,
        bytes: &[u8],
        display_name: &str,
    ) -> Result<StagedOcrSource, LocalMediaError>;

    /// Atomically transfer one staged input to an operation. A second call for the same id fails
    /// with `INPUT_NOT_FOUND`; that is the race guard, not a diagnostic.
    fn claim(&self, staged_input_id: &StagedInputId) -> Result<ClaimedInput, LocalMediaError>;

    /// Reserve the recording-owned WAV path before capture opens the device.
    fn authorize_recording_wav(
        &self,
        recording_id: &RecordingId,
    ) -> Result<PathBuf, LocalMediaError>;

    /// Delete a recording's directory. Called on cancel, on a too-short hold, and after
    /// transcription regardless of outcome.
    fn cleanup_recording(&self, recording_id: &RecordingId);

    /// Reserve the operation-owned output path for synthesized speech.
    fn authorize_output_wav(&self, operation_id: &str) -> Result<PathBuf, LocalMediaError>;

    /// Write a readiness canary's input into the probe operation's own directory.
    ///
    /// Separate from `stage_bytes` because that one sniffs content against the OCR admission list,
    /// which is right for a file the user picked and wrong for bytes this context authored: an
    /// audio canary is not an image, and running it through image admission would reject it.
    /// `cleanup_operation` removes it, so a canary leaves nothing behind even if the probe fails.
    fn authorize_canary_input(
        &self,
        operation_id: &str,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, LocalMediaError>;

    /// Confirm a worker-returned path is exactly the authorized one and holds a bounded WAV.
    fn verify_output_wav(
        &self,
        operation_id: &str,
        candidate: &Path,
    ) -> Result<u64, LocalMediaError>;

    /// Delete everything owned by one operation. Idempotent; a missing directory is success.
    fn cleanup_operation(&self, operation_id: &str);

    /// Delete an unclaimed staged input.
    fn cleanup_staged(&self, staged_input_id: &StagedInputId);

    /// Bounded removal of entries older than the retention window. Runs once at startup.
    fn sweep_stale(&self, older_than_ms: u64) -> usize;
}

/// Supervised Python engine workers.
pub(crate) trait WorkerSupervisorPort: Send + Sync {
    /// Run one call against the engine named by the snapshot. Blocking; the caller is already on a
    /// background thread. `cancelled` is polled cooperatively and, after the grace period, causes
    /// only this engine's worker to be terminated.
    fn call(
        &self,
        snapshot: &LocalMediaProfileSnapshot,
        call: WorkerCall,
        cancelled: Arc<AtomicBool>,
    ) -> Result<WorkerReply, LocalMediaError>;

    fn state(&self, engine: LocalMediaEngine) -> WorkerState;

    /// Stop idle workers whose captured revision is older than the newly saved one.
    fn retire_stale(&self, current_revision: i64);

    fn shutdown_all(&self);
}

pub(crate) struct StartCaptureRequest {
    pub(crate) recording_id: RecordingId,
    pub(crate) device_id: Option<String>,
    pub(crate) max_duration_ms: u64,
    pub(crate) destination: PathBuf,
}

/// Native microphone capture. Samples never leave the implementation.
pub(crate) trait AudioCapturePort: Send + Sync {
    fn start(&self, request: StartCaptureRequest) -> Result<u32, LocalMediaError>;
    /// Stop, drain, finalize the WAV header, and report counts.
    fn finish(&self, recording_id: &RecordingId) -> Result<CommittedRecording, LocalMediaError>;
    /// Stop and discard. The partial file is removed by the caller's cleanup guard.
    fn cancel(&self, recording_id: &RecordingId);
    fn active(&self) -> Option<RecordingSummary>;
}

/// Native playback of one generated WAV at a time.
pub(crate) trait AudioPlaybackPort: Send + Sync {
    /// Start playback and block until it finishes, is stopped, or fails. Returns the duration
    /// actually played.
    fn play_blocking(
        &self,
        playback_id: &PlaybackId,
        path: &Path,
        device_id: Option<&str>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<u64, LocalMediaError>;

    fn stop(&self, playback_id: Option<&PlaybackId>);
}

pub(crate) trait AudioDeviceCatalogPort: Send + Sync {
    fn catalog(&self) -> Result<AudioDeviceCatalog, LocalMediaError>;
}

/// Bridge to the operations context. Keeps this context from writing another context's tables.
pub(crate) trait OperationBridge: Send + Sync {
    /// Allocate a stable id immediately and register the operation as accepted.
    fn start(&self, kind: &str, message_key: &str) -> Result<String, LocalMediaError>;
    fn phase(&self, operation_id: &str, phase: &str);
    fn succeed(&self, operation_id: &str);
    fn fail(&self, operation_id: &str, code: &str);
    fn cancel(&self, operation_id: &str);
    fn cancellation_flag(&self, operation_id: &str) -> Arc<AtomicBool>;
    fn is_cancelled(&self, operation_id: &str) -> bool;
}

/// Redacted diagnostics. Every argument is already an allowlisted scalar; there is no message
/// parameter, because a message is where content leaks.
pub(crate) trait LocalMediaDiagnostics: Send + Sync {
    fn record(&self, event: &str, fields: &[(&str, String)]);
}
