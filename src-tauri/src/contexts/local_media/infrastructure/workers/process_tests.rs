use super::*;
use crate::contexts::local_media::application::worker_contract::SttWorkerRequest;
use crate::contexts::local_media::domain::{
    ComposerScopeId, LocalMediaOperationId, LocalMediaOperationKind, LocalMediaProfile,
};
use serde_json::json;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

/// What a scripted transport does when the session reads.
enum Step {
    /// Deliver one line.
    Line(Vec<u8>),
    /// Report a timeout for this poll, `count` times.
    Silence(usize),
    /// Report the process as gone.
    Dead,
}

struct ScriptedTransport {
    steps: VecDeque<Step>,
    sent: Vec<Vec<u8>>,
    terminated: bool,
    /// What the dead child is pretending to have written to stderr.
    diagnostics: Vec<u8>,
}

impl ScriptedTransport {
    fn new(steps: Vec<Step>) -> Self {
        Self {
            steps: steps.into(),
            sent: Vec::new(),
            terminated: false,
            diagnostics: Vec::new(),
        }
    }

    fn with_diagnostics(mut self, text: &str) -> Self {
        self.diagnostics = text.as_bytes().to_vec();
        self
    }
}

impl WorkerTransport for ScriptedTransport {
    fn send_line(&mut self, frame: &[u8]) -> Result<(), LocalMediaError> {
        self.sent.push(frame.to_vec());
        Ok(())
    }

    fn recv_line(&mut self, _timeout: Duration) -> Result<Option<Vec<u8>>, LocalMediaError> {
        match self.steps.front_mut() {
            None => Ok(None),
            Some(Step::Silence(count)) => {
                *count -= 1;
                if *count == 0 {
                    self.steps.pop_front();
                }
                Ok(None)
            }
            Some(Step::Dead) => Err(LocalMediaError::new(LocalMediaErrorCode::WorkerCrashed)),
            Some(Step::Line(_)) => {
                let Some(Step::Line(line)) = self.steps.pop_front() else {
                    return Ok(None);
                };
                Ok(Some(line))
            }
        }
    }

    fn terminate(&mut self) {
        self.terminated = true;
    }

    fn crash_diagnostics(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.diagnostics)
    }
}

/// A transport whose sent frames the test can read back after the session takes ownership.
struct SharedLog(Arc<Mutex<Vec<Vec<u8>>>>);

struct LoggingTransport {
    inner: ScriptedTransport,
    log: Arc<Mutex<Vec<Vec<u8>>>>,
    terminated: Arc<AtomicBool>,
}

impl WorkerTransport for LoggingTransport {
    fn send_line(&mut self, frame: &[u8]) -> Result<(), LocalMediaError> {
        self.log.lock().expect("log").push(frame.to_vec());
        self.inner.send_line(frame)
    }

    fn recv_line(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, LocalMediaError> {
        self.inner.recv_line(timeout)
    }

    fn terminate(&mut self) {
        self.terminated.store(true, Ordering::SeqCst);
        self.inner.terminate();
    }
}

fn hello_line(engine: &str, methods: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "v": 1, "type": "hello", "engine": engine, "workerVersion": "1",
        "packageVersion": "1.2.3", "capabilities": methods,
    }))
    .expect("encode hello")
}

fn stt_hello() -> Vec<u8> {
    hello_line(
        "faster-whisper",
        &["probe", "transcribe", "cancel", "shutdown"],
    )
}

fn transcribe_line(id: &str, text: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "v": 1, "type": "response", "id": id, "ok": true,
        "result": {"text": text, "noSpeechDetected": false}
    }))
    .expect("encode response")
}

fn snapshot() -> LocalMediaProfileSnapshot {
    let mut profile = LocalMediaProfile::disabled_default("2026-01-01T00:00:00Z".to_string());
    profile.revision = 2;
    profile.enabled = true;
    profile.stt.enabled = true;
    profile.stt.model_directory = "/models/whisper".to_string();
    LocalMediaProfileSnapshot::capture(
        LocalMediaOperationId::new("lmo-0123456789abcdef0123456789abcdef"),
        LocalMediaOperationKind::Stt,
        LocalMediaEngine::Stt,
        &profile,
        Some(ComposerScopeId::new("session-1")),
        "2026-01-01T00:00:01Z".to_string(),
    )
}

fn transcribe_call() -> WorkerCall {
    WorkerCall::Transcribe(SttWorkerRequest {
        audio_path: PathBuf::from("/tmp/local-media/recordings/lmr-1/input.wav"),
        bypass_voice_activity_filter: false,
    })
}

fn establish(
    steps: Vec<Step>,
) -> Result<(WorkerSession, SharedLog, Arc<AtomicBool>), LocalMediaError> {
    let log = Arc::new(Mutex::new(Vec::new()));
    let terminated = Arc::new(AtomicBool::new(false));
    let transport = LoggingTransport {
        inner: ScriptedTransport::new(steps),
        log: log.clone(),
        terminated: terminated.clone(),
    };
    let session = WorkerSession::establish(
        LocalMediaEngine::Stt,
        Box::new(transport),
        Duration::from_millis(50),
        2,
    )?;
    Ok((session, SharedLog(log), terminated))
}

fn never_cancelled() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

const CALL_TIMEOUT: Duration = Duration::from_millis(200);
const CANCEL_GRACE: Duration = Duration::from_millis(50);

// ------------------------------------------------------------- handshake ---

#[test]
fn a_worker_that_greets_correctly_is_ready() {
    let (session, _log, _terminated) = establish(vec![Step::Line(stt_hello())]).expect("handshake");
    assert_eq!(session.profile_revision(), 2);
    assert!(!session.is_poisoned());
}

#[test]
fn a_silent_worker_fails_the_handshake_deadline() {
    let Err(error) = establish(vec![Step::Silence(100)]) else {
        panic!("a silent worker must not complete the handshake");
    };
    assert_eq!(error.code(), LocalMediaErrorCode::WorkerStartFailed);
}

#[test]
fn a_worker_that_exits_during_the_handshake_fails_to_start() {
    let Err(error) = establish(vec![Step::Dead]) else {
        panic!("a dead worker must not complete the handshake");
    };
    assert_eq!(error.code(), LocalMediaErrorCode::WorkerStartFailed);
}

#[test]
fn stdout_contamination_before_the_hello_fails_the_handshake() {
    let Err(error) = establish(vec![Step::Line(b"Loading model weights...".to_vec())]) else {
        panic!("stdout contamination must not complete the handshake");
    };
    assert_eq!(error.code(), LocalMediaErrorCode::WorkerStartFailed);
}

#[test]
fn a_hello_for_the_wrong_engine_fails_the_handshake() {
    let wrong = hello_line("paddleocr", &["probe", "ocr", "cancel", "shutdown"]);
    assert!(establish(vec![Step::Line(wrong)]).is_err());
}

// ------------------------------------------------------------------ call ---

#[test]
fn a_successful_call_returns_a_typed_reply() {
    let (mut session, log, _terminated) = establish(vec![
        Step::Line(stt_hello()),
        Step::Line(transcribe_line(
            "lmo-0123456789abcdef0123456789abcdef-1",
            "hello",
        )),
    ])
    .expect("handshake");

    let reply = session
        .call(
            &snapshot(),
            &transcribe_call(),
            never_cancelled(),
            never_cancelled(),
            CALL_TIMEOUT,
            CANCEL_GRACE,
        )
        .expect("call");
    let WorkerReply::Transcribe(reply) = reply else {
        panic!("expected a transcription");
    };
    assert_eq!(reply.text, "hello");

    let sent = log.0.lock().expect("log");
    let request: serde_json::Value = serde_json::from_slice(&sent[0]).expect("decode request");
    assert_eq!(request["method"], "transcribe");
    assert_eq!(request["params"]["modelDirectory"], "/models/whisper");
    assert!(!session.is_poisoned());
}

#[test]
fn request_ids_are_unique_per_call() {
    let (mut session, log, _terminated) = establish(vec![
        Step::Line(stt_hello()),
        Step::Line(transcribe_line(
            "lmo-0123456789abcdef0123456789abcdef-1",
            "a",
        )),
        Step::Line(transcribe_line(
            "lmo-0123456789abcdef0123456789abcdef-2",
            "b",
        )),
    ])
    .expect("handshake");

    for _ in 0..2 {
        session
            .call(
                &snapshot(),
                &transcribe_call(),
                never_cancelled(),
                never_cancelled(),
                CALL_TIMEOUT,
                CANCEL_GRACE,
            )
            .expect("call");
    }
    let sent = log.0.lock().expect("log");
    let first: serde_json::Value = serde_json::from_slice(&sent[0]).expect("decode");
    let second: serde_json::Value = serde_json::from_slice(&sent[1]).expect("decode");
    assert_ne!(first["id"], second["id"]);
}

#[test]
fn a_mismatched_response_id_poisons_the_worker() {
    // There is no way to know which request the stream is now answering, so the session is unusable
    // and the supervisor must replace the process.
    let (mut session, _log, terminated) = establish(vec![
        Step::Line(stt_hello()),
        Step::Line(transcribe_line("some-other-request", "hello")),
    ])
    .expect("handshake");

    let error = session
        .call(
            &snapshot(),
            &transcribe_call(),
            never_cancelled(),
            never_cancelled(),
            CALL_TIMEOUT,
            CANCEL_GRACE,
        )
        .expect_err("mismatched id");
    assert_eq!(error.code(), LocalMediaErrorCode::WorkerProtocolError);
    assert!(session.is_poisoned());
    assert!(terminated.load(Ordering::SeqCst));
}

#[test]
fn stdout_contamination_during_a_call_poisons_the_worker() {
    let (mut session, _log, terminated) = establish(vec![
        Step::Line(stt_hello()),
        Step::Line(b"[WARNING] falling back to CPU".to_vec()),
    ])
    .expect("handshake");

    let error = session
        .call(
            &snapshot(),
            &transcribe_call(),
            never_cancelled(),
            never_cancelled(),
            CALL_TIMEOUT,
            CANCEL_GRACE,
        )
        .expect_err("contamination");
    assert_eq!(error.code(), LocalMediaErrorCode::WorkerProtocolError);
    assert!(session.is_poisoned());
    assert!(terminated.load(Ordering::SeqCst));
}

#[test]
fn a_worker_that_dies_mid_call_reports_a_crash() {
    let (mut session, _log, _terminated) =
        establish(vec![Step::Line(stt_hello()), Step::Dead]).expect("handshake");
    let error = session
        .call(
            &snapshot(),
            &transcribe_call(),
            never_cancelled(),
            never_cancelled(),
            CALL_TIMEOUT,
            CANCEL_GRACE,
        )
        .expect_err("crash");
    assert_eq!(error.code(), LocalMediaErrorCode::WorkerCrashed);
    assert!(session.is_poisoned());
}

#[test]
fn a_hung_worker_is_terminated_at_the_call_deadline() {
    let (mut session, _log, terminated) =
        establish(vec![Step::Line(stt_hello()), Step::Silence(100_000)]).expect("handshake");
    let error = session
        .call(
            &snapshot(),
            &transcribe_call(),
            never_cancelled(),
            never_cancelled(),
            Duration::from_millis(30),
            CANCEL_GRACE,
        )
        .expect_err("timeout");
    assert_eq!(error.code(), LocalMediaErrorCode::WorkerCrashed);
    assert!(terminated.load(Ordering::SeqCst));
}

#[test]
fn a_worker_error_response_does_not_poison_the_session() {
    // A model that is missing is the user's configuration problem, not a broken process: the
    // worker is still speaking the protocol correctly and can serve the next request.
    let frame = serde_json::to_vec(&json!({
        "v": 1, "type": "response", "id": "lmo-0123456789abcdef0123456789abcdef-1", "ok": false,
        "error": {"code": "MODEL_NOT_FOUND", "messageKey": "k", "retryable": false}
    }))
    .expect("encode");
    let (mut session, _log, terminated) =
        establish(vec![Step::Line(stt_hello()), Step::Line(frame)]).expect("handshake");

    let error = session
        .call(
            &snapshot(),
            &transcribe_call(),
            never_cancelled(),
            never_cancelled(),
            CALL_TIMEOUT,
            CANCEL_GRACE,
        )
        .expect_err("worker error");
    assert_eq!(error.code(), LocalMediaErrorCode::ModelNotFound);
    assert!(!session.is_poisoned());
    assert!(!terminated.load(Ordering::SeqCst));
}

// ----------------------------------------------------------- cancellation ---

#[test]
fn a_cooperative_cancel_sends_a_cancel_frame_and_settles_as_cancelled() {
    let cancelled_reply = serde_json::to_vec(&json!({
        "v": 1, "type": "response", "id": "lmo-0123456789abcdef0123456789abcdef-1", "ok": false,
        "error": {"code": "OPERATION_CANCELLED", "messageKey": "k", "retryable": false}
    }))
    .expect("encode");
    let (mut session, log, terminated) = establish(vec![
        Step::Line(stt_hello()),
        Step::Silence(1),
        Step::Line(cancelled_reply),
    ])
    .expect("handshake");

    let flag = Arc::new(AtomicBool::new(true));
    let error = session
        .call(
            &snapshot(),
            &transcribe_call(),
            flag,
            never_cancelled(),
            CALL_TIMEOUT,
            CANCEL_GRACE,
        )
        .expect_err("cancelled");
    assert_eq!(error.code(), LocalMediaErrorCode::OperationCancelled);

    let sent = log.0.lock().expect("log");
    let cancel: serde_json::Value =
        serde_json::from_slice(sent.last().expect("a frame")).expect("decode");
    assert_eq!(cancel["type"], "cancel");
    assert_eq!(cancel["id"], "lmo-0123456789abcdef0123456789abcdef-1");
    // A worker that acknowledged is healthy; only a non-cooperative one gets killed.
    assert!(!terminated.load(Ordering::SeqCst));
}

#[test]
fn a_non_cooperative_worker_is_terminated_after_the_grace_period() {
    let (mut session, _log, terminated) =
        establish(vec![Step::Line(stt_hello()), Step::Silence(100_000)]).expect("handshake");

    let flag = Arc::new(AtomicBool::new(true));
    let error = session
        .call(
            &snapshot(),
            &transcribe_call(),
            flag,
            never_cancelled(),
            CALL_TIMEOUT,
            Duration::from_millis(20),
        )
        .expect_err("non-cooperative cancel");
    assert_eq!(error.code(), LocalMediaErrorCode::OperationCancelled);
    assert!(session.is_poisoned());
    assert!(
        terminated.load(Ordering::SeqCst),
        "only this engine's worker is killed"
    );
}

#[test]
fn shutdown_asks_before_it_terminates() {
    let (mut session, log, terminated) =
        establish(vec![Step::Line(stt_hello())]).expect("handshake");
    session.shutdown(Duration::from_millis(20));

    let sent = log.0.lock().expect("log");
    let frame: serde_json::Value =
        serde_json::from_slice(sent.last().expect("a frame")).expect("decode");
    assert_eq!(frame["type"], "shutdown");
    assert!(terminated.load(Ordering::SeqCst));
}

// --------------------------------------------- phonemizer crash classification ---

/// sherpa-onnx calls `exit()` rather than raising when a voice needs phonemizer data it was not
/// given, so the only evidence that the crash was a configuration problem is what it printed.
fn crashed_tts_session(diagnostics: &str) -> LocalMediaError {
    let hello = hello_line(
        "sherpa-onnx",
        &["probe", "synthesize", "cancel", "shutdown"],
    );
    let transport =
        ScriptedTransport::new(vec![Step::Line(hello), Step::Dead]).with_diagnostics(diagnostics);
    let mut session = WorkerSession::establish(
        LocalMediaEngine::Tts,
        Box::new(transport),
        Duration::from_millis(50),
        2,
    )
    .expect("handshake");
    session
        .call(
            &snapshot(),
            &WorkerCall::Synthesize(
                crate::contexts::local_media::application::worker_contract::TtsWorkerRequest {
                    text: "a".to_string(),
                    output_path: std::path::PathBuf::from("/tmp/local-media/op/output.wav"),
                },
            ),
            never_cancelled(),
            never_cancelled(),
            Duration::from_millis(50),
            Duration::from_millis(10),
        )
        .expect_err("the worker died")
}

#[test]
fn a_phonemizer_signature_on_a_dead_tts_worker_is_classified() {
    let error = crashed_tts_session(
        "sherpa-onnx/csrc/offline-tts-vits-impl.h:InitFrontend:471 Not a model using characters \
         as modeling unit. Please provide --vits-lexicon if you leave --vits-data-dir empty",
    );
    assert_eq!(
        error.code(),
        LocalMediaErrorCode::TtsPhonemizerDataUnavailable
    );
    assert!(format!("{:?}", error.details()).contains("dataDir"));
}

#[test]
fn a_missing_espeak_directory_is_the_same_classification() {
    let error = crashed_tts_session("Error processing file '/usr/share/espeak-ng-data/phontab'.");
    assert_eq!(
        error.code(),
        LocalMediaErrorCode::TtsPhonemizerDataUnavailable
    );
}

#[test]
fn an_unrelated_tts_crash_stays_a_worker_crash() {
    // Guessing here would send the user to configure a data directory that has nothing to do with
    // why their worker died.
    let error = crashed_tts_session("MemoryError: cannot allocate 2 GiB");
    assert_eq!(error.code(), LocalMediaErrorCode::WorkerCrashed);
}

#[test]
fn a_crash_with_nothing_on_stderr_stays_a_worker_crash() {
    let error = crashed_tts_session("");
    assert_eq!(error.code(), LocalMediaErrorCode::WorkerCrashed);
}

#[test]
fn the_raw_diagnostics_never_reach_the_error() {
    let error = crashed_tts_session(
        "Error processing file '/home/someone/models/espeak-ng-data/phontab': No such file",
    );
    let rendered = format!("{:?}", error.details());
    assert!(!rendered.contains("/home/someone"));
    assert!(!rendered.contains("phontab"));
}
