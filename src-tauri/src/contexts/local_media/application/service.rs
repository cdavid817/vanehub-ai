//! The composed local-media application service.
//!
//! Every long-running entry point splits into `prepare_*` (synchronous, allocates the operation id)
//! and `execute` (blocking, run on a background thread by the command layer). That split is what
//! lets a command return a stable id without waiting for a model to load, and it keeps
//! `tauri::async_runtime` out of the application layer.

use super::operation_store::LocalMediaOperationStore;
use super::ports::{
    AudioCapturePort, AudioDeviceCatalogPort, AudioPlaybackPort, ClaimedInput, LocalMediaClock,
    LocalMediaDiagnostics, LocalMediaProfileRepository, MediaTempStore, OpaqueIdFactory,
    OperationBridge, WorkerSupervisorPort,
};
use crate::contexts::local_media::domain::{
    classify_model_paths, first_error, validate_profile, AudioDeviceCatalog, EngineReadiness,
    EngineStatus, LocalMediaEngine, LocalMediaError, LocalMediaErrorCode, LocalMediaOperationKind,
    LocalMediaOperationResult, LocalMediaProfile, LocalMediaProfileSnapshot,
    LocalMediaRuntimeStatus, PlatformSupport, PlaybackId, ProfileFieldIssue, RecordingId,
    RecordingSummary, StagedInputId,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Typed results are retained for five minutes. Long enough for a user to come back to a review
/// dialog; short enough that recognized text is not sitting in memory for a whole session.
const RESULT_RETENTION_MS: u64 = 5 * 60 * 1000;
const RESULT_CAPACITY: usize = 32;
/// Startup sweep window from the ephemeral-media requirement.
pub(crate) const STALE_SWEEP_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Default)]
pub(super) struct EngineRuntimeState {
    pub(super) readiness: Option<EngineReadiness>,
    pub(super) installed_version: Option<String>,
    pub(super) model_identity: Option<String>,
    pub(super) device_summary: Option<String>,
    pub(super) last_checked_at: Option<String>,
    pub(super) checked_revision: Option<i64>,
}

/// One accepted, not-yet-executed unit of work.
#[derive(Debug)]
pub(super) enum LocalMediaJob {
    Probe {
        snapshot: LocalMediaProfileSnapshot,
    },
    Ocr {
        snapshot: LocalMediaProfileSnapshot,
        claimed: ClaimedInput,
    },
    Stt {
        snapshot: LocalMediaProfileSnapshot,
        recording_id: RecordingId,
    },
    Tts {
        snapshot: LocalMediaProfileSnapshot,
        text: String,
        playback_id: PlaybackId,
    },
}

/// What a `prepare_*` call returns. The id is already registered with the operations context, so a
/// caller may report it before `execute` has done anything.
#[derive(Debug)]
pub(crate) struct PreparedLocalMediaOperation {
    pub(crate) operation_id: String,
    pub(crate) kind: LocalMediaOperationKind,
    pub(crate) accepted_at: String,
    pub(super) job: LocalMediaJob,
}

pub(super) struct Inner {
    pub(super) repository: Arc<dyn LocalMediaProfileRepository>,
    pub(super) clock: Arc<dyn LocalMediaClock>,
    pub(super) ids: Arc<dyn OpaqueIdFactory>,
    pub(super) temp: Arc<dyn MediaTempStore>,
    pub(super) workers: Arc<dyn WorkerSupervisorPort>,
    pub(super) capture: Arc<dyn AudioCapturePort>,
    pub(super) playback: Arc<dyn AudioPlaybackPort>,
    pub(super) devices: Arc<dyn AudioDeviceCatalogPort>,
    pub(super) operations: Arc<dyn OperationBridge>,
    pub(super) diagnostics: Arc<dyn LocalMediaDiagnostics>,
    pub(super) store: LocalMediaOperationStore,
    pub(super) engine_state: Mutex<HashMap<LocalMediaEngine, EngineRuntimeState>>,
    /// The one recording allowed application-wide, together with the scope that owns it. Held here
    /// rather than in the capture port because ownership is a use-case rule, not a device concern.
    pub(super) active_recording: Mutex<Option<RecordingSummary>>,
    pub(super) platform: PlatformSupport,
}

#[derive(Clone)]
pub(crate) struct LocalMediaApplicationService {
    pub(super) inner: Arc<Inner>,
}

pub(crate) struct LocalMediaDependencies {
    pub(crate) repository: Arc<dyn LocalMediaProfileRepository>,
    pub(crate) clock: Arc<dyn LocalMediaClock>,
    pub(crate) ids: Arc<dyn OpaqueIdFactory>,
    pub(crate) temp: Arc<dyn MediaTempStore>,
    pub(crate) workers: Arc<dyn WorkerSupervisorPort>,
    pub(crate) capture: Arc<dyn AudioCapturePort>,
    pub(crate) playback: Arc<dyn AudioPlaybackPort>,
    pub(crate) devices: Arc<dyn AudioDeviceCatalogPort>,
    pub(crate) operations: Arc<dyn OperationBridge>,
    pub(crate) diagnostics: Arc<dyn LocalMediaDiagnostics>,
}

impl LocalMediaApplicationService {
    pub(crate) fn new(dependencies: LocalMediaDependencies) -> Self {
        Self {
            inner: Arc::new(Inner {
                repository: dependencies.repository,
                clock: dependencies.clock,
                ids: dependencies.ids,
                temp: dependencies.temp,
                workers: dependencies.workers,
                capture: dependencies.capture,
                playback: dependencies.playback,
                devices: dependencies.devices,
                operations: dependencies.operations,
                diagnostics: dependencies.diagnostics,
                store: LocalMediaOperationStore::new(RESULT_RETENTION_MS, RESULT_CAPACITY),
                engine_state: Mutex::new(HashMap::new()),
                active_recording: Mutex::new(None),
                platform: PlatformSupport::current(),
            }),
        }
    }

    pub(crate) fn get_profile(&self) -> Result<LocalMediaProfile, LocalMediaError> {
        self.inner.repository.load()
    }

    /// Validate then commit. Validation runs before the repository sees the value so a rejected
    /// profile never consumes a revision.
    pub(crate) fn save_profile(
        &self,
        profile: LocalMediaProfile,
        expected_revision: i64,
    ) -> Result<LocalMediaProfile, LocalMediaError> {
        let issues = validate_profile(&profile);
        if let Some(error) = first_error(&issues) {
            return Err(error);
        }
        let mut candidate = profile;
        candidate.updated_at = self.inner.clock.now_iso();
        let saved = self.inner.repository.save(&candidate, expected_revision)?;

        // A saved profile invalidates readiness: the next use must re-probe rather than trust a
        // check performed against different paths.
        self.reset_readiness_after_save(saved.revision);
        self.inner.workers.retire_stale(saved.revision);
        self.inner.diagnostics.record(
            "profile.saved",
            &[("profileRevision", saved.revision.to_string())],
        );
        Ok(saved)
    }

    pub(crate) fn validate(&self, profile: &LocalMediaProfile) -> Vec<ProfileFieldIssue> {
        validate_profile(profile)
    }

    pub(crate) fn get_status(&self) -> Result<LocalMediaRuntimeStatus, LocalMediaError> {
        let profile = self.inner.repository.load()?;
        Ok(self.status_for(&profile))
    }

    pub(crate) fn list_audio_devices(&self) -> Result<AudioDeviceCatalog, LocalMediaError> {
        self.inner.devices.catalog()
    }

    pub(crate) fn get_operation_result(
        &self,
        operation_id: &str,
    ) -> Result<Option<LocalMediaOperationResult>, LocalMediaError> {
        self.inner
            .store
            .result(operation_id, self.inner.clock.now_ms())
    }

    /// Cancel through the operations context and stop whatever native resource the operation owns.
    /// Playback stops immediately; a worker gets a cooperative cancel first.
    pub(crate) fn cancel_operation(&self, operation_id: &str) {
        self.inner.operations.cancel(operation_id);
        self.inner
            .store
            .cancel(operation_id, self.inner.clock.now_ms());
        if self.inner.store.kind(operation_id) == Some(LocalMediaOperationKind::Tts) {
            self.inner.playback.stop(None);
        }
        self.inner.temp.cleanup_operation(operation_id);
    }

    pub(crate) fn stop_playback(&self, playback_id: Option<&PlaybackId>) {
        self.inner.playback.stop(playback_id);
    }

    /// Remove ephemeral media left behind by a previous run. Bounded and best effort: a sweep
    /// failure is a redacted warning, never a startup failure.
    pub(crate) fn sweep_stale_media(&self) {
        let removed = self.inner.temp.sweep_stale(STALE_SWEEP_MS);
        if removed > 0 {
            self.inner
                .diagnostics
                .record("cleanup.sweep", &[("count", removed.to_string())]);
        }
    }

    pub(crate) fn shutdown(&self) {
        self.inner.playback.stop(None);
        if let Some(active) = self.inner.capture.active() {
            self.inner.capture.cancel(&active.recording_id);
        }
        self.inner.workers.shutdown_all();
    }

    pub(crate) fn cleanup_staged(&self, staged_input_id: &StagedInputId) {
        self.inner.temp.cleanup_staged(staged_input_id);
    }

    pub(super) fn status_for(&self, profile: &LocalMediaProfile) -> LocalMediaRuntimeStatus {
        let engines = LocalMediaEngine::ALL
            .iter()
            .map(|engine| self.engine_status(*engine, profile))
            .collect();
        LocalMediaRuntimeStatus {
            native_available: true,
            platform_support: self.inner.platform,
            enabled: profile.enabled,
            profile_revision: profile.revision,
            engines,
            path_classifications: classify_model_paths(profile),
        }
    }

    fn engine_status(&self, engine: LocalMediaEngine, profile: &LocalMediaProfile) -> EngineStatus {
        let worker_state = self.inner.workers.state(engine);
        if !profile.engine_enabled(engine) {
            return EngineStatus::disabled(engine, profile.revision);
        }
        if self.inner.platform == PlatformSupport::Unsupported {
            return EngineStatus {
                readiness: EngineReadiness::Unavailable {
                    code: LocalMediaErrorCode::EngineUnavailable,
                    // A platform this feature does not run on is not one field's fault.
                    field: None,
                },
                worker_state,
                ..EngineStatus::disabled(engine, profile.revision)
            };
        }

        let recorded = self.engine_state_for(engine);
        let configured = self.engine_is_configured(engine, profile);
        let readiness = Self::resolve_readiness(&recorded, configured, profile.revision);

        EngineStatus {
            engine,
            readiness,
            profile_revision: profile.revision,
            worker_state,
            installed_version: recorded.installed_version.clone(),
            model_identity: recorded.model_identity.clone(),
            device_summary: recorded.device_summary.clone(),
            last_checked_at: recorded.last_checked_at.clone(),
        }
    }

    /// Readiness is only meaningful for the revision it was established against. A recorded `Ready`
    /// from an older revision becomes `RestartRequired`, which the settings page renders as "Needs
    /// check" when no worker is alive and "Restart required" when one is.
    fn resolve_readiness(
        recorded: &EngineRuntimeState,
        configured: bool,
        revision: i64,
    ) -> EngineReadiness {
        if !configured {
            return EngineReadiness::Unconfigured;
        }
        match (&recorded.readiness, recorded.checked_revision) {
            (Some(EngineReadiness::Checking), _) => EngineReadiness::Checking,
            (Some(readiness), Some(checked)) if checked == revision => readiness.clone(),
            (Some(_), _) => EngineReadiness::RestartRequired,
            (None, _) => EngineReadiness::RestartRequired,
        }
    }

    fn engine_is_configured(&self, engine: LocalMediaEngine, profile: &LocalMediaProfile) -> bool {
        let mut candidate = profile.clone();
        // Validation reports every enabled engine's issues at once; narrowing to one engine keeps
        // an unconfigured OCR section from marking a fully configured TTS section unconfigured.
        match engine {
            LocalMediaEngine::Ocr => {
                candidate.stt.enabled = false;
                candidate.tts.enabled = false;
            }
            LocalMediaEngine::Stt => {
                candidate.ocr.enabled = false;
                candidate.tts.enabled = false;
            }
            LocalMediaEngine::Tts => {
                candidate.ocr.enabled = false;
                candidate.stt.enabled = false;
            }
        }
        validate_profile(&candidate).is_empty()
    }

    pub(super) fn engine_state_for(&self, engine: LocalMediaEngine) -> EngineRuntimeState {
        let Ok(states) = self.inner.engine_state.lock() else {
            return EngineRuntimeState::default();
        };
        states.get(&engine).cloned().unwrap_or_default()
    }

    pub(super) fn record_engine_state(&self, engine: LocalMediaEngine, state: EngineRuntimeState) {
        let Ok(mut states) = self.inner.engine_state.lock() else {
            return;
        };
        states.insert(engine, state);
    }

    fn reset_readiness_after_save(&self, revision: i64) {
        let Ok(mut states) = self.inner.engine_state.lock() else {
            return;
        };
        for state in states.values_mut() {
            if state.checked_revision != Some(revision) {
                state.readiness = Some(EngineReadiness::RestartRequired);
            }
        }
    }

    /// Refuse work the runtime cannot honour, before an operation id is allocated.
    pub(super) fn ensure_engine_usable(
        &self,
        engine: LocalMediaEngine,
        profile: &LocalMediaProfile,
    ) -> Result<(), LocalMediaError> {
        if !self.inner.platform.permits_operation() {
            return Err(LocalMediaError::new(LocalMediaErrorCode::EngineUnavailable)
                .with_text("engine", engine.as_str()));
        }
        if !profile.enabled {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::LocalMediaDisabled,
            ));
        }
        if !profile.engine_enabled(engine) {
            return Err(LocalMediaError::new(LocalMediaErrorCode::EngineDisabled)
                .with_text("engine", engine.as_str()));
        }
        if !self.engine_is_configured(engine, profile) {
            return Err(
                LocalMediaError::new(LocalMediaErrorCode::EngineUnconfigured)
                    .with_text("engine", engine.as_str()),
            );
        }
        Ok(())
    }

    /// Register the operation and capture the snapshot it will run against.
    pub(super) fn accept_operation(
        &self,
        kind: LocalMediaOperationKind,
        engine: LocalMediaEngine,
        profile: &LocalMediaProfile,
        composer_scope: Option<crate::contexts::local_media::domain::ComposerScopeId>,
    ) -> Result<(String, LocalMediaProfileSnapshot, String), LocalMediaError> {
        let operation_id = self
            .inner
            .operations
            .start(kind.as_str(), kind.message_key())?;
        let accepted_at = self.inner.clock.now_iso();
        self.inner
            .store
            .accept(&operation_id, kind, self.inner.clock.now_ms());
        let snapshot = LocalMediaProfileSnapshot::capture(
            crate::contexts::local_media::domain::LocalMediaOperationId::new(operation_id.clone()),
            kind,
            engine,
            profile,
            composer_scope,
            accepted_at.clone(),
        );
        Ok((operation_id, snapshot, accepted_at))
    }
}
