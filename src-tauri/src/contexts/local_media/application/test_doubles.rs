//! Deterministic doubles for every local-media port.
//!
//! Shared across the application tests so a use case can be exercised without Python, a
//! microphone, an audio device, or SQLite. Each double records what it was asked to do, because
//! most of the interesting assertions here are about what did *not* happen -- no worker call after
//! cancellation, no draft-bound result after a scope change, no file left behind.

use super::ports::{
    AudioCapturePort, AudioDeviceCatalogPort, AudioPlaybackPort, ClaimedInput, LocalMediaClock,
    LocalMediaDiagnostics, LocalMediaProfileRepository, MediaTempStore, OpaqueIdFactory,
    OperationBridge, StartCaptureRequest, WorkerSupervisorPort,
};
use super::worker_contract::{WorkerCall, WorkerReply};
use crate::contexts::local_media::domain::{
    AudioDevice, AudioDeviceCatalog, CommittedRecording, ComposerScopeId, LocalMediaEngine,
    LocalMediaError, LocalMediaErrorCode, LocalMediaProfile, LocalMediaProfileSnapshot,
    OcrMediaType, PlaybackId, RecordingId, RecordingSummary, StagedInputId, StagedOcrSource,
    WorkerState,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub(super) struct FakeProfileRepository {
    pub(super) profile: Mutex<LocalMediaProfile>,
    pub(super) save_calls: Mutex<Vec<i64>>,
    pub(super) fail_with: Mutex<Option<LocalMediaErrorCode>>,
}

impl FakeProfileRepository {
    pub(super) fn new(profile: LocalMediaProfile) -> Arc<Self> {
        Arc::new(Self {
            profile: Mutex::new(profile),
            save_calls: Mutex::new(Vec::new()),
            fail_with: Mutex::new(None),
        })
    }
}

impl LocalMediaProfileRepository for FakeProfileRepository {
    fn load(&self) -> Result<LocalMediaProfile, LocalMediaError> {
        Ok(self.profile.lock().expect("profile lock").clone())
    }

    fn save(
        &self,
        profile: &LocalMediaProfile,
        expected_revision: i64,
    ) -> Result<LocalMediaProfile, LocalMediaError> {
        self.save_calls
            .lock()
            .expect("save calls")
            .push(expected_revision);
        if let Some(code) = *self.fail_with.lock().expect("fail with") {
            return Err(LocalMediaError::new(code));
        }
        let mut stored = self.profile.lock().expect("profile lock");
        if stored.revision != expected_revision {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::ProfileRevisionConflict,
            ));
        }
        let mut next = profile.clone();
        next.revision = stored.revision + 1;
        *stored = next.clone();
        Ok(next)
    }
}

pub(super) struct FixedClock {
    pub(super) millis: AtomicU64,
}

impl FixedClock {
    pub(super) fn new(millis: u64) -> Arc<Self> {
        Arc::new(Self {
            millis: AtomicU64::new(millis),
        })
    }

    pub(super) fn advance(&self, delta: u64) {
        self.millis.fetch_add(delta, Ordering::SeqCst);
    }
}

impl LocalMediaClock for FixedClock {
    fn now_iso(&self) -> String {
        format!(
            "2026-01-01T00:00:{:02}Z",
            self.millis.load(Ordering::SeqCst) / 1000 % 60
        )
    }

    fn now_ms(&self) -> u64 {
        self.millis.load(Ordering::SeqCst)
    }
}

pub(super) struct SequentialIds {
    counter: AtomicU64,
}

impl SequentialIds {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            counter: AtomicU64::new(0),
        })
    }
}

impl OpaqueIdFactory for SequentialIds {
    fn next(&self, prefix: &str) -> String {
        let index = self.counter.fetch_add(1, Ordering::SeqCst);
        format!("{prefix}{index:032x}")
    }
}

#[derive(Default)]
pub(super) struct TempStoreLog {
    pub(super) cleaned_operations: Vec<String>,
    pub(super) cleaned_staged: Vec<String>,
    pub(super) cleaned_recordings: Vec<String>,
    pub(super) authorized_outputs: Vec<String>,
    pub(super) swept: usize,
}

pub(super) struct FakeTempStore {
    pub(super) log: Mutex<TempStoreLog>,
    pub(super) staged: Mutex<HashMap<String, StagedOcrSource>>,
    pub(super) claimed: Mutex<Vec<String>>,
    pub(super) stage_error: Mutex<Option<LocalMediaErrorCode>>,
    pub(super) verify_error: Mutex<Option<LocalMediaErrorCode>>,
    ids: Arc<SequentialIds>,
}

impl FakeTempStore {
    pub(super) fn new(ids: Arc<SequentialIds>) -> Arc<Self> {
        Arc::new(Self {
            log: Mutex::new(TempStoreLog::default()),
            staged: Mutex::new(HashMap::new()),
            claimed: Mutex::new(Vec::new()),
            stage_error: Mutex::new(None),
            verify_error: Mutex::new(None),
            ids,
        })
    }

    fn record(&self, source: StagedOcrSource) -> StagedOcrSource {
        self.staged
            .lock()
            .expect("staged lock")
            .insert(source.staged_input_id.as_str().to_string(), source.clone());
        source
    }
}

impl MediaTempStore for FakeTempStore {
    fn stage_ocr_source(&self, source: &Path) -> Result<StagedOcrSource, LocalMediaError> {
        if let Some(code) = *self.stage_error.lock().expect("stage error") {
            return Err(LocalMediaError::new(code));
        }
        let name = source
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file");
        Ok(self.record(StagedOcrSource {
            staged_input_id: StagedInputId::new(self.ids.next(StagedInputId::PREFIX)),
            display_name: name.to_string(),
            media_type: OcrMediaType::Image,
            byte_length: 1024,
        }))
    }

    fn stage_bytes(
        &self,
        bytes: &[u8],
        display_name: &str,
    ) -> Result<StagedOcrSource, LocalMediaError> {
        if let Some(code) = *self.stage_error.lock().expect("stage error") {
            return Err(LocalMediaError::new(code));
        }
        Ok(self.record(StagedOcrSource {
            staged_input_id: StagedInputId::new(self.ids.next(StagedInputId::PREFIX)),
            display_name: display_name.to_string(),
            media_type: OcrMediaType::Pdf,
            byte_length: bytes.len() as u64,
        }))
    }

    fn claim(&self, staged_input_id: &StagedInputId) -> Result<ClaimedInput, LocalMediaError> {
        let key = staged_input_id.as_str().to_string();
        let mut claimed = self.claimed.lock().expect("claimed lock");
        if claimed.contains(&key) {
            return Err(LocalMediaError::new(LocalMediaErrorCode::InputNotFound));
        }
        let staged = self.staged.lock().expect("staged lock");
        let Some(source) = staged.get(&key).cloned() else {
            return Err(LocalMediaError::new(LocalMediaErrorCode::InputNotFound));
        };
        claimed.push(key.clone());
        Ok(ClaimedInput {
            staged_input_id: staged_input_id.clone(),
            source,
            path: PathBuf::from(format!("/tmp/local-media/{key}/source.bin")),
        })
    }

    fn authorize_recording_wav(
        &self,
        recording_id: &RecordingId,
    ) -> Result<PathBuf, LocalMediaError> {
        Ok(PathBuf::from(format!(
            "/tmp/local-media/{}/input.wav",
            recording_id.as_str()
        )))
    }

    fn cleanup_recording(&self, recording_id: &RecordingId) {
        self.log
            .lock()
            .expect("log lock")
            .cleaned_recordings
            .push(recording_id.as_str().to_string());
    }

    fn authorize_output_wav(&self, operation_id: &str) -> Result<PathBuf, LocalMediaError> {
        self.log
            .lock()
            .expect("log lock")
            .authorized_outputs
            .push(operation_id.to_string());
        Ok(PathBuf::from(format!(
            "/tmp/local-media/{operation_id}/output.wav"
        )))
    }

    fn verify_output_wav(
        &self,
        operation_id: &str,
        candidate: &Path,
    ) -> Result<u64, LocalMediaError> {
        if let Some(code) = *self.verify_error.lock().expect("verify error") {
            return Err(LocalMediaError::new(code));
        }
        let expected = PathBuf::from(format!("/tmp/local-media/{operation_id}/output.wav"));
        if candidate != expected {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::WorkerProtocolError,
            ));
        }
        Ok(4096)
    }

    fn cleanup_operation(&self, operation_id: &str) {
        self.log
            .lock()
            .expect("log lock")
            .cleaned_operations
            .push(operation_id.to_string());
    }

    fn cleanup_staged(&self, staged_input_id: &StagedInputId) {
        self.log
            .lock()
            .expect("log lock")
            .cleaned_staged
            .push(staged_input_id.as_str().to_string());
    }

    fn sweep_stale(&self, _older_than_ms: u64) -> usize {
        let mut log = self.log.lock().expect("log lock");
        log.swept += 1;
        log.swept
    }
}

/// A scripted worker response.
///
/// Aliased because clippy's complexity threshold counts the boxed closure, and every test
/// helper that builds one would otherwise repeat the same four-line signature.
pub(super) type WorkerHandler = Box<
    dyn Fn(&LocalMediaProfileSnapshot, &WorkerCall) -> Result<WorkerReply, LocalMediaError>
        + Send
        + Sync,
>;

pub(super) struct FakeWorkerSupervisor {
    pub(super) handler: Mutex<Option<WorkerHandler>>,
    pub(super) calls: Mutex<Vec<(LocalMediaEngine, String, i64)>>,
    pub(super) retired: Mutex<Vec<i64>>,
    pub(super) shutdowns: AtomicU64,
    pub(super) state: Mutex<HashMap<LocalMediaEngine, WorkerState>>,
}

impl FakeWorkerSupervisor {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            handler: Mutex::new(None),
            calls: Mutex::new(Vec::new()),
            retired: Mutex::new(Vec::new()),
            shutdowns: AtomicU64::new(0),
            state: Mutex::new(HashMap::new()),
        })
    }

    pub(super) fn respond_with(&self, handler: WorkerHandler) {
        *self.handler.lock().expect("handler lock") = Some(handler);
    }

    pub(super) fn call_count(&self) -> usize {
        self.calls.lock().expect("calls lock").len()
    }
}

impl WorkerSupervisorPort for FakeWorkerSupervisor {
    fn call(
        &self,
        snapshot: &LocalMediaProfileSnapshot,
        call: WorkerCall,
        cancelled: Arc<AtomicBool>,
    ) -> Result<WorkerReply, LocalMediaError> {
        self.calls.lock().expect("calls lock").push((
            snapshot.engine(),
            call.method(snapshot.engine()).to_string(),
            snapshot.profile_revision(),
        ));
        if cancelled.load(Ordering::SeqCst) {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::OperationCancelled,
            ));
        }
        let handler = self.handler.lock().expect("handler lock");
        match handler.as_ref() {
            Some(handler) => handler(snapshot, &call),
            None => Err(LocalMediaError::new(LocalMediaErrorCode::EngineUnavailable)),
        }
    }

    fn state(&self, engine: LocalMediaEngine) -> WorkerState {
        self.state
            .lock()
            .expect("state lock")
            .get(&engine)
            .copied()
            .unwrap_or(WorkerState::Stopped)
    }

    fn retire_stale(&self, current_revision: i64) {
        self.retired
            .lock()
            .expect("retired lock")
            .push(current_revision);
    }

    fn shutdown_all(&self) {
        self.shutdowns.fetch_add(1, Ordering::SeqCst);
    }
}

pub(super) struct FakeCapture {
    pub(super) active: Mutex<Option<RecordingSummary>>,
    pub(super) committed: Mutex<Option<CommittedRecording>>,
    pub(super) start_error: Mutex<Option<LocalMediaErrorCode>>,
    pub(super) finish_error: Mutex<Option<LocalMediaErrorCode>>,
    pub(super) cancels: Mutex<Vec<String>>,
    pub(super) destinations: Mutex<Vec<PathBuf>>,
}

impl FakeCapture {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            active: Mutex::new(None),
            committed: Mutex::new(None),
            start_error: Mutex::new(None),
            finish_error: Mutex::new(None),
            cancels: Mutex::new(Vec::new()),
            destinations: Mutex::new(Vec::new()),
        })
    }
}

impl AudioCapturePort for FakeCapture {
    fn start(&self, request: StartCaptureRequest) -> Result<u32, LocalMediaError> {
        if let Some(code) = *self.start_error.lock().expect("start error") {
            return Err(LocalMediaError::new(code));
        }
        self.destinations
            .lock()
            .expect("destinations")
            .push(request.destination);
        *self.active.lock().expect("active lock") = Some(RecordingSummary {
            recording_id: request.recording_id,
            composer_scope: ComposerScopeId::new("unset"),
            started_at_ms: 0,
            max_duration_ms: request.max_duration_ms,
        });
        Ok(16_000)
    }

    fn finish(&self, recording_id: &RecordingId) -> Result<CommittedRecording, LocalMediaError> {
        *self.active.lock().expect("active lock") = None;
        if let Some(code) = *self.finish_error.lock().expect("finish error") {
            return Err(LocalMediaError::new(code));
        }
        Ok(self
            .committed
            .lock()
            .expect("committed lock")
            .clone()
            .unwrap_or(CommittedRecording {
                recording_id: recording_id.clone(),
                duration_ms: 6_400,
                sample_rate: 16_000,
                sample_count: 102_400,
                limit_reached: false,
            }))
    }

    fn cancel(&self, recording_id: &RecordingId) {
        *self.active.lock().expect("active lock") = None;
        self.cancels
            .lock()
            .expect("cancels")
            .push(recording_id.as_str().to_string());
    }

    fn active(&self) -> Option<RecordingSummary> {
        self.active.lock().expect("active lock").clone()
    }
}

pub(super) struct FakePlayback {
    pub(super) played: Mutex<Vec<PathBuf>>,
    pub(super) stops: Mutex<Vec<Option<String>>>,
    pub(super) error: Mutex<Option<LocalMediaErrorCode>>,
    pub(super) active: Mutex<Option<PlaybackId>>,
}

impl FakePlayback {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            played: Mutex::new(Vec::new()),
            stops: Mutex::new(Vec::new()),
            error: Mutex::new(None),
            active: Mutex::new(None),
        })
    }
}

impl AudioPlaybackPort for FakePlayback {
    fn play_blocking(
        &self,
        playback_id: &PlaybackId,
        path: &Path,
        _device_id: Option<&str>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<u64, LocalMediaError> {
        self.played
            .lock()
            .expect("played lock")
            .push(path.to_path_buf());
        if let Some(code) = *self.error.lock().expect("error lock") {
            return Err(LocalMediaError::new(code));
        }
        if cancelled.load(Ordering::SeqCst) {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::OperationCancelled,
            ));
        }
        *self.active.lock().expect("active lock") = Some(playback_id.clone());
        Ok(2_967)
    }

    fn stop(&self, playback_id: Option<&PlaybackId>) {
        self.stops
            .lock()
            .expect("stops lock")
            .push(playback_id.map(|id| id.as_str().to_string()));
        *self.active.lock().expect("active lock") = None;
    }
}

pub(super) struct FakeDevices;

impl AudioDeviceCatalogPort for FakeDevices {
    fn catalog(&self) -> Result<AudioDeviceCatalog, LocalMediaError> {
        Ok(AudioDeviceCatalog {
            inputs: vec![AudioDevice {
                device_id: "input-0".to_string(),
                label: "Default input".to_string(),
                is_default: true,
            }],
            outputs: vec![AudioDevice {
                device_id: "output-0".to_string(),
                label: "Default output".to_string(),
                is_default: true,
            }],
        })
    }
}

#[derive(Default)]
pub(super) struct OperationLog {
    pub(super) started: Vec<(String, String)>,
    pub(super) phases: Vec<(String, String)>,
    pub(super) succeeded: Vec<String>,
    pub(super) failed: Vec<(String, String)>,
    pub(super) cancelled: Vec<String>,
}

pub(super) struct FakeOperationBridge {
    pub(super) log: Mutex<OperationLog>,
    pub(super) flags: Mutex<HashMap<String, Arc<AtomicBool>>>,
    counter: AtomicU64,
}

impl FakeOperationBridge {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            log: Mutex::new(OperationLog::default()),
            flags: Mutex::new(HashMap::new()),
            counter: AtomicU64::new(0),
        })
    }

    /// Flip the cancellation flag the way the operations context would when the user cancels.
    pub(super) fn request_cancel(&self, operation_id: &str) {
        if let Some(flag) = self.flags.lock().expect("flags lock").get(operation_id) {
            flag.store(true, Ordering::SeqCst);
        }
    }
}

impl OperationBridge for FakeOperationBridge {
    fn start(&self, kind: &str, message_key: &str) -> Result<String, LocalMediaError> {
        let index = self.counter.fetch_add(1, Ordering::SeqCst);
        let id = format!("operation-{index}");
        self.log
            .lock()
            .expect("log lock")
            .started
            .push((kind.to_string(), message_key.to_string()));
        self.flags
            .lock()
            .expect("flags lock")
            .insert(id.clone(), Arc::new(AtomicBool::new(false)));
        Ok(id)
    }

    fn phase(&self, operation_id: &str, phase: &str) {
        self.log
            .lock()
            .expect("log lock")
            .phases
            .push((operation_id.to_string(), phase.to_string()));
    }

    fn succeed(&self, operation_id: &str) {
        self.log
            .lock()
            .expect("log lock")
            .succeeded
            .push(operation_id.to_string());
    }

    fn fail(&self, operation_id: &str, code: &str) {
        self.log
            .lock()
            .expect("log lock")
            .failed
            .push((operation_id.to_string(), code.to_string()));
    }

    fn cancel(&self, operation_id: &str) {
        self.log
            .lock()
            .expect("log lock")
            .cancelled
            .push(operation_id.to_string());
        self.request_cancel(operation_id);
    }

    fn cancellation_flag(&self, operation_id: &str) -> Arc<AtomicBool> {
        self.flags
            .lock()
            .expect("flags lock")
            .entry(operation_id.to_string())
            .or_insert_with(|| Arc::new(AtomicBool::new(false)))
            .clone()
    }

    fn is_cancelled(&self, operation_id: &str) -> bool {
        self.flags
            .lock()
            .expect("flags lock")
            .get(operation_id)
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
    }
}

/// One recorded diagnostic: the event name and its allowlisted scalar fields.
type RecordedDiagnostic = (String, Vec<(String, String)>);

#[derive(Default)]
pub(super) struct RecordingDiagnostics {
    pub(super) events: Mutex<Vec<RecordedDiagnostic>>,
}

impl RecordingDiagnostics {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Everything the diagnostics port was handed, flattened. Used by the privacy tests to assert
    /// that no recognized text, transcript, or path ever reached it.
    pub(super) fn flattened(&self) -> String {
        self.events
            .lock()
            .expect("events lock")
            .iter()
            .map(|(event, fields)| {
                let pairs: Vec<String> = fields
                    .iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect();
                format!("{event} {}", pairs.join(" "))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl LocalMediaDiagnostics for RecordingDiagnostics {
    fn record(&self, event: &str, fields: &[(&str, String)]) {
        self.events.lock().expect("events lock").push((
            event.to_string(),
            fields
                .iter()
                .map(|(key, value)| ((*key).to_string(), value.clone()))
                .collect(),
        ));
    }
}
