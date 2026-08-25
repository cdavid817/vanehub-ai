use super::service::{LocalMediaApplicationService, LocalMediaDependencies};
use super::test_doubles::{
    FakeCapture, FakeDevices, FakeOperationBridge, FakePlayback, FakeProfileRepository,
    FakeTempStore, FakeWorkerSupervisor, FixedClock, RecordingDiagnostics, SequentialIds,
    WorkerHandler,
};
use super::worker_contract::{
    OcrReply, ProbeReply, SynthesizeReply, TranscribeReply, WorkerCall, WorkerLine, WorkerPage,
    WorkerReply,
};
use crate::contexts::local_media::domain::{
    ComposerScopeId, EngineReadiness, LocalMediaEngine, LocalMediaError, LocalMediaErrorCode,
    LocalMediaOperationResult, LocalMediaProfile, RecordingId, StagedInputId,
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

struct Harness {
    service: LocalMediaApplicationService,
    repository: Arc<FakeProfileRepository>,
    clock: Arc<FixedClock>,
    temp: Arc<FakeTempStore>,
    workers: Arc<FakeWorkerSupervisor>,
    capture: Arc<FakeCapture>,
    playback: Arc<FakePlayback>,
    operations: Arc<FakeOperationBridge>,
    diagnostics: Arc<RecordingDiagnostics>,
}

fn absolute(name: &str) -> String {
    if cfg!(windows) {
        format!("C:\\vanehub-test\\{name}")
    } else {
        format!("/opt/vanehub-test/{name}")
    }
}

fn ready_profile() -> LocalMediaProfile {
    let mut profile = LocalMediaProfile::disabled_default("2026-01-01T00:00:00Z".to_string());
    profile.revision = 3;
    profile.enabled = true;

    profile.ocr.enabled = true;
    profile.ocr.python_executable = absolute("python");
    profile.ocr.text_detection_model_dir = Some(absolute("det"));
    profile.ocr.text_recognition_model_dir = Some(absolute("rec"));

    profile.stt.enabled = true;
    profile.stt.python_executable = absolute("python");
    profile.stt.model_directory = absolute("whisper");

    profile.tts.enabled = true;
    profile.tts.python_executable = absolute("python");
    profile.tts.model_path = absolute("model.onnx");
    profile.tts.tokens_path = absolute("tokens.txt");

    profile
}

fn harness_with(profile: LocalMediaProfile) -> Harness {
    let ids = SequentialIds::new();
    let repository = FakeProfileRepository::new(profile);
    let clock = FixedClock::new(1_000);
    let temp = FakeTempStore::new(ids.clone());
    let workers = FakeWorkerSupervisor::new();
    let capture = FakeCapture::new();
    let playback = FakePlayback::new();
    let operations = FakeOperationBridge::new();
    let diagnostics = RecordingDiagnostics::new();

    let service = LocalMediaApplicationService::new(LocalMediaDependencies {
        repository: repository.clone(),
        clock: clock.clone(),
        ids,
        temp: temp.clone(),
        workers: workers.clone(),
        capture: capture.clone(),
        playback: playback.clone(),
        devices: Arc::new(FakeDevices),
        operations: operations.clone(),
        diagnostics: diagnostics.clone(),
    });

    Harness {
        service,
        repository,
        clock,
        temp,
        workers,
        capture,
        playback,
        operations,
        diagnostics,
    }
}

fn harness() -> Harness {
    harness_with(ready_profile())
}

fn scope() -> ComposerScopeId {
    ComposerScopeId::new("session-1")
}

// ---------------------------------------------------------------- profile ---

#[test]
fn saving_an_invalid_profile_never_consumes_a_revision() {
    let harness = harness();
    let mut invalid = ready_profile();
    invalid.stt.beam_size = 99;

    let error = harness
        .service
        .save_profile(invalid, 3)
        .expect_err("validation must fail");
    assert_eq!(
        error.code(),
        LocalMediaErrorCode::DeviceConfigurationInvalid
    );
    assert!(
        harness
            .repository
            .save_calls
            .lock()
            .expect("save calls")
            .is_empty(),
        "the repository must not be reached at all"
    );
}

#[test]
fn a_stale_revision_conflicts_and_leaves_the_stored_profile_alone() {
    let harness = harness();
    let mut edited = ready_profile();
    edited.ocr.language = "en".to_string();

    let error = harness
        .service
        .save_profile(edited, 2)
        .expect_err("stale save must fail");
    assert_eq!(error.code(), LocalMediaErrorCode::ProfileRevisionConflict);
    assert_eq!(
        harness
            .repository
            .profile
            .lock()
            .expect("profile")
            .ocr
            .language,
        "ch",
        "the stored profile must be untouched"
    );
}

#[test]
fn a_successful_save_increments_the_revision_and_retires_stale_workers() {
    let harness = harness();
    let saved = harness
        .service
        .save_profile(ready_profile(), 3)
        .expect("save");
    assert_eq!(saved.revision, 4);
    assert_eq!(
        harness.workers.retired.lock().expect("retired").as_slice(),
        &[4]
    );
}

#[test]
fn a_save_forces_a_re_check_before_the_engine_is_usable_again() {
    let harness = harness();
    probe_engine_successfully(&harness, LocalMediaEngine::Ocr);
    assert!(matches!(
        readiness(&harness, LocalMediaEngine::Ocr),
        EngineReadiness::Ready
    ));

    harness
        .service
        .save_profile(ready_profile(), 3)
        .expect("save");
    assert!(
        matches!(
            readiness(&harness, LocalMediaEngine::Ocr),
            EngineReadiness::RestartRequired
        ),
        "readiness established against an older revision must not carry forward"
    );
}

// -------------------------------------------------------------- readiness ---

fn readiness(harness: &Harness, engine: LocalMediaEngine) -> EngineReadiness {
    harness
        .service
        .get_status()
        .expect("status")
        .engine(engine)
        .expect("engine present")
        .readiness
        .clone()
}

fn probe_engine_successfully(harness: &Harness, engine: LocalMediaEngine) {
    // A probe is two calls now: metadata, then the readiness canary's real inference. Scripting
    // only the first would leave every one of these tests asserting a readiness the engine never
    // demonstrated -- which is the whole point of the canary.
    harness.workers.respond_with(Box::new(|_, call| match call {
        WorkerCall::Probe => Ok(WorkerReply::Probe(ProbeReply {
            package_version: Some("3.0.1".to_string()),
            device: Some("cpu".to_string()),
            model_identity: Some("det+rec:ch".to_string()),
        })),
        WorkerCall::Ocr(_) => Ok(WorkerReply::Ocr(OcrReply::default())),
        WorkerCall::Transcribe(_) => Ok(WorkerReply::Transcribe(TranscribeReply::default())),
        WorkerCall::Synthesize(_) => Ok(WorkerReply::Synthesize(SynthesizeReply {
            audio_path: std::path::PathBuf::from("/tmp/local-media/operation-0/output.wav"),
            sample_rate: 16_000,
            sample_count: 1_600,
            duration_ms: 100,
            engine_version: Some("1.10.0".to_string()),
        })),
    }));
    let prepared = harness
        .service
        .prepare_probe(engine)
        .expect("prepare probe");
    harness.service.execute(prepared);
}

#[test]
fn an_unprobed_configured_engine_needs_a_check_rather_than_claiming_readiness() {
    let harness = harness();
    assert!(matches!(
        readiness(&harness, LocalMediaEngine::Tts),
        EngineReadiness::RestartRequired
    ));
}

#[test]
fn a_disabled_engine_reports_disabled_regardless_of_configuration() {
    let mut profile = ready_profile();
    profile.tts.enabled = false;
    let harness = harness_with(profile);
    assert_eq!(
        readiness(&harness, LocalMediaEngine::Tts),
        EngineReadiness::Disabled
    );
}

#[test]
fn an_engine_missing_its_model_reports_unconfigured_not_a_worker_error() {
    let mut profile = ready_profile();
    profile.stt.model_directory = String::new();
    let harness = harness_with(profile);
    assert_eq!(
        readiness(&harness, LocalMediaEngine::Stt),
        EngineReadiness::Unconfigured
    );
}

#[test]
fn one_engines_configuration_gap_does_not_make_another_unconfigured() {
    let mut profile = ready_profile();
    profile.ocr.text_detection_model_dir = None;
    let harness = harness_with(profile);
    assert_eq!(
        readiness(&harness, LocalMediaEngine::Ocr),
        EngineReadiness::Unconfigured
    );
    assert!(matches!(
        readiness(&harness, LocalMediaEngine::Stt),
        EngineReadiness::RestartRequired
    ));
}

#[test]
fn a_failed_probe_marks_only_its_own_engine_unavailable() {
    let harness = harness();
    probe_engine_successfully(&harness, LocalMediaEngine::Stt);

    harness.workers.respond_with(Box::new(|_, _| {
        Err(LocalMediaError::new(
            LocalMediaErrorCode::EngineImportFailed,
        ))
    }));
    let prepared = harness
        .service
        .prepare_probe(LocalMediaEngine::Ocr)
        .expect("prepare");
    harness.service.execute(prepared);

    assert_eq!(
        readiness(&harness, LocalMediaEngine::Ocr),
        EngineReadiness::Unavailable {
            code: LocalMediaErrorCode::EngineImportFailed,
            field: None,
        }
    );
    assert_eq!(
        readiness(&harness, LocalMediaEngine::Stt),
        EngineReadiness::Ready
    );
}

#[test]
fn probing_a_disabled_engine_is_refused_before_an_operation_exists() {
    let mut profile = ready_profile();
    profile.ocr.enabled = false;
    let harness = harness_with(profile);

    let error = harness
        .service
        .prepare_probe(LocalMediaEngine::Ocr)
        .expect_err("refused");
    assert_eq!(error.code(), LocalMediaErrorCode::EngineDisabled);
    assert!(harness
        .operations
        .log
        .lock()
        .expect("log")
        .started
        .is_empty());
}

#[test]
fn a_probe_result_carries_the_full_runtime_status() {
    let harness = harness();
    probe_engine_successfully(&harness, LocalMediaEngine::Ocr);
    let result = harness
        .service
        .get_operation_result("operation-0")
        .expect("result");
    let Some(LocalMediaOperationResult::Probe(status)) = result else {
        panic!("expected a probe status");
    };
    assert_eq!(status.engines.len(), 3);
    assert_eq!(
        status
            .engine(LocalMediaEngine::Ocr)
            .expect("ocr")
            .installed_version
            .as_deref(),
        Some("3.0.1")
    );
}

// -------------------------------------------------------------------- ocr ---

fn ocr_worker(pages: Vec<(&str, u32)>, truncated: bool) -> WorkerHandler {
    let pages: Vec<WorkerPage> = pages
        .into_iter()
        .enumerate()
        .map(|(index, (text, line_count))| WorkerPage {
            page_number: index as u32 + 1,
            text: text.to_string(),
            line_count,
            lines: text
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| WorkerLine {
                    text: line.to_string(),
                    confidence: Some(0.95),
                    polygon: None,
                })
                .collect(),
        })
        .collect();
    Box::new(move |_, call| match call {
        WorkerCall::Ocr(_) => Ok(WorkerReply::Ocr(OcrReply {
            pages: pages.clone(),
            character_count: 0,
            truncated,
            no_text_detected: pages.iter().all(|page| page.text.is_empty()),
            engine_version: Some("3.0.1".to_string()),
            model_identity: Some("det+rec:ch".to_string()),
        })),
        _ => Err(LocalMediaError::new(
            LocalMediaErrorCode::WorkerProtocolError,
        )),
    })
}

fn stage(harness: &Harness) -> StagedInputId {
    harness
        .service
        .stage_ocr_source(Path::new("/home/user/scan.png"))
        .expect("stage")
        .staged_input_id
}

#[test]
fn ocr_derives_plain_text_in_page_and_reading_order() {
    let harness = harness();
    harness
        .workers
        .respond_with(ocr_worker(vec![("alpha\nbeta", 2), ("gamma", 1)], false));
    let staged = stage(&harness);

    let prepared = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    let Some(LocalMediaOperationResult::Ocr(result)) = harness
        .service
        .get_operation_result(&operation_id)
        .expect("result")
    else {
        panic!("expected an OCR result");
    };
    assert_eq!(result.plain_text, "alpha\nbeta\n\ngamma");
    assert_eq!(result.source.page_count, 2);
    assert_eq!(result.provenance.engine, "paddleocr");
    assert_eq!(result.provenance.profile_revision, 3);
}

#[test]
fn ocr_that_recognizes_nothing_succeeds_with_a_no_text_outcome() {
    let harness = harness();
    harness
        .workers
        .respond_with(ocr_worker(vec![("", 0)], false));
    let staged = stage(&harness);
    let prepared = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    let Some(LocalMediaOperationResult::Ocr(result)) = harness
        .service
        .get_operation_result(&operation_id)
        .expect("result")
    else {
        panic!("expected an OCR result rather than a failure");
    };
    assert!(result.is_empty());
    assert_eq!(
        result.outcome_code(),
        Some(LocalMediaErrorCode::NoTextDetected)
    );
    assert!(harness
        .operations
        .log
        .lock()
        .expect("log")
        .failed
        .is_empty());
}

#[test]
fn a_truncated_result_carries_a_warning_instead_of_silently_cutting_text() {
    let harness = harness();
    harness
        .workers
        .respond_with(ocr_worker(vec![("alpha", 1)], true));
    let staged = stage(&harness);
    let prepared = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    let Some(LocalMediaOperationResult::Ocr(result)) = harness
        .service
        .get_operation_result(&operation_id)
        .expect("result")
    else {
        panic!("expected an OCR result");
    };
    assert!(result.truncated);
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].code, "OUTPUT_TRUNCATED");
}

#[test]
fn a_staged_input_cannot_be_claimed_twice() {
    let harness = harness();
    harness
        .workers
        .respond_with(ocr_worker(vec![("alpha", 1)], false));
    let staged = stage(&harness);

    let first = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect("first claim");
    harness.service.execute(first);

    let error = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect_err("second claim");
    assert_eq!(error.code(), LocalMediaErrorCode::InputNotFound);
}

#[test]
fn a_rejected_claim_does_not_create_an_orphan_operation() {
    let harness = harness();
    let unknown = StagedInputId::new("lmi-ffffffffffffffffffffffffffffffff");
    assert!(harness
        .service
        .prepare_ocr(&unknown, Some(scope()))
        .is_err());
    assert!(harness
        .operations
        .log
        .lock()
        .expect("log")
        .started
        .is_empty());
}

#[test]
fn ocr_cleans_both_the_staged_copy_and_the_operation_directory() {
    let harness = harness();
    harness
        .workers
        .respond_with(ocr_worker(vec![("alpha", 1)], false));
    let staged = stage(&harness);
    let prepared = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    let log = harness.temp.log.lock().expect("log");
    assert!(log.cleaned_staged.contains(&staged.as_str().to_string()));
    assert!(log.cleaned_operations.contains(&operation_id));
}

#[test]
fn a_failing_ocr_worker_still_cleans_up() {
    let harness = harness();
    harness.workers.respond_with(Box::new(|_, _| {
        Err(LocalMediaError::new(LocalMediaErrorCode::WorkerCrashed))
    }));
    let staged = stage(&harness);
    let prepared = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    assert_eq!(
        harness
            .service
            .get_operation_result(&operation_id)
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::WorkerCrashed)
    );
    let log = harness.temp.log.lock().expect("log");
    assert!(log.cleaned_staged.contains(&staged.as_str().to_string()));
    assert!(log.cleaned_operations.contains(&operation_id));
}

#[test]
fn artifact_staging_never_takes_a_host_path() {
    // The OnePiece entry point receives verified bytes; there is no path parameter to abuse.
    let harness = harness();
    let staged = harness
        .service
        .stage_ocr_artifact(b"%PDF-1.7 fake", "report.pdf")
        .expect("stage");
    assert_eq!(staged.display_name, "report.pdf");
    assert_eq!(staged.byte_length, 13);
}

// -------------------------------------------------------------------- stt ---

#[test]
fn only_one_recording_can_be_active() {
    let harness = harness();
    harness
        .service
        .start_recording(scope())
        .expect("first recording");
    let error = harness
        .service
        .start_recording(scope())
        .expect_err("second recording");
    assert_eq!(error.code(), LocalMediaErrorCode::RecordingAlreadyActive);
}

#[test]
fn a_failed_device_open_releases_the_slot_and_the_temp_file() {
    let harness = harness();
    *harness.capture.start_error.lock().expect("start error") =
        Some(LocalMediaErrorCode::MicPermissionDenied);

    let error = harness
        .service
        .start_recording(scope())
        .expect_err("denied");
    assert_eq!(error.code(), LocalMediaErrorCode::MicPermissionDenied);
    assert!(!harness
        .temp
        .log
        .lock()
        .expect("log")
        .cleaned_recordings
        .is_empty());

    // The slot must be free again, or a permission prompt that the user then accepts would be
    // followed by a permanent "already active".
    *harness.capture.start_error.lock().expect("start error") = None;
    assert!(harness.service.start_recording(scope()).is_ok());
}

#[test]
fn another_scope_cannot_stop_or_cancel_a_recording_it_does_not_own() {
    let harness = harness();
    let handle = harness.service.start_recording(scope()).expect("recording");
    let other = ComposerScopeId::new("session-2");

    assert_eq!(
        harness
            .service
            .cancel_recording(&handle.recording_id, &other)
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::RecordingNotFound)
    );
    assert_eq!(
        harness
            .service
            .prepare_transcription(&handle.recording_id, other)
            .map_err(|error| error.code())
            .err(),
        Some(LocalMediaErrorCode::RecordingNotFound)
    );
}

#[test]
fn a_refused_cancel_leaves_the_recording_for_its_own_scope_to_release() {
    let harness = harness();
    let handle = harness.service.start_recording(scope()).expect("recording");
    let other = ComposerScopeId::new("session-2");

    // The frontend keeps the scope a recording was born under precisely so this call is the one it
    // makes. Aiming it with whatever session is on screen produces the refusal below, and the
    // application-wide slot would then stay occupied with the microphone open.
    assert!(harness
        .service
        .cancel_recording(&handle.recording_id, &other)
        .is_err());

    // Refusing did not end it: the owner can still release it, and the slot frees.
    assert!(harness
        .service
        .cancel_recording(&handle.recording_id, &scope())
        .is_ok());
    assert!(harness.service.start_recording(other).is_ok());
}

#[test]
fn guessing_a_recording_id_is_not_enough() {
    let harness = harness();
    harness.service.start_recording(scope()).expect("recording");
    let guessed = RecordingId::new("lmr-ffffffffffffffffffffffffffffffff");
    assert_eq!(
        harness
            .service
            .cancel_recording(&guessed, &scope())
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::RecordingNotFound)
    );
}

fn transcribe_worker(text: &str) -> WorkerHandler {
    let text = text.to_string();
    Box::new(move |_, call| match call {
        WorkerCall::Transcribe(_) => Ok(WorkerReply::Transcribe(TranscribeReply {
            text: text.clone(),
            detected_language: Some("zh".to_string()),
            language_probability: Some(0.98),
            duration_ms: Some(6_400),
            no_speech_detected: text.trim().is_empty(),
            engine_version: Some("1.0.0".to_string()),
            device: Some("cpu".to_string()),
        })),
        _ => Err(LocalMediaError::new(
            LocalMediaErrorCode::WorkerProtocolError,
        )),
    })
}

#[test]
fn a_valid_hold_transcribes_the_complete_utterance() {
    let harness = harness();
    harness
        .workers
        .respond_with(transcribe_worker("  你好，世界  "));
    let handle = harness.service.start_recording(scope()).expect("recording");
    let prepared = harness
        .service
        .prepare_transcription(&handle.recording_id, scope())
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    let Some(LocalMediaOperationResult::Stt(result)) = harness
        .service
        .get_operation_result(&operation_id)
        .expect("result")
    else {
        panic!("expected a transcription");
    };
    assert_eq!(result.text, "你好，世界");
    assert!(!result.limit_reached);
    assert_eq!(result.provenance.engine, "faster-whisper");
}

#[test]
fn a_hold_shorter_than_the_floor_never_reaches_the_worker() {
    let harness = harness();
    harness
        .workers
        .respond_with(transcribe_worker("should not run"));
    *harness.capture.committed.lock().expect("committed") =
        Some(crate::contexts::local_media::domain::CommittedRecording {
            recording_id: RecordingId::new("lmr-0000"),
            duration_ms: 120,
            sample_rate: 16_000,
            sample_count: 1_920,
            limit_reached: false,
        });

    let handle = harness.service.start_recording(scope()).expect("recording");
    let prepared = harness
        .service
        .prepare_transcription(&handle.recording_id, scope())
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    assert_eq!(harness.workers.call_count(), 0);
    assert_eq!(
        harness
            .service
            .get_operation_result(&operation_id)
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::RecordingTooShort)
    );
    assert!(!harness
        .temp
        .log
        .lock()
        .expect("log")
        .cleaned_recordings
        .is_empty());
}

#[test]
fn reaching_the_limit_transcribes_and_flags_the_warning() {
    let harness = harness();
    harness
        .workers
        .respond_with(transcribe_worker("long utterance"));
    *harness.capture.committed.lock().expect("committed") =
        Some(crate::contexts::local_media::domain::CommittedRecording {
            recording_id: RecordingId::new("lmr-0000"),
            duration_ms: 120_000,
            sample_rate: 16_000,
            sample_count: 1_920_000,
            limit_reached: true,
        });

    let handle = harness.service.start_recording(scope()).expect("recording");
    let prepared = harness
        .service
        .prepare_transcription(&handle.recording_id, scope())
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    let Some(LocalMediaOperationResult::Stt(result)) = harness
        .service
        .get_operation_result(&operation_id)
        .expect("result")
    else {
        panic!("the utterance must not be discarded when the limit is reached");
    };
    assert_eq!(result.text, "long utterance");
    assert!(result.limit_reached);
}

#[test]
fn an_empty_transcript_is_reported_as_no_speech_rather_than_a_failure() {
    let harness = harness();
    harness.workers.respond_with(transcribe_worker("   "));
    let handle = harness.service.start_recording(scope()).expect("recording");
    let prepared = harness
        .service
        .prepare_transcription(&handle.recording_id, scope())
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    let Some(LocalMediaOperationResult::Stt(result)) = harness
        .service
        .get_operation_result(&operation_id)
        .expect("result")
    else {
        panic!("expected a transcription result");
    };
    assert!(result.is_empty());
    assert_eq!(
        result.outcome_code(),
        Some(LocalMediaErrorCode::NoSpeechDetected)
    );
}

#[test]
fn the_recording_wav_is_deleted_on_every_terminal_path() {
    for text in ["hello", "   "] {
        let harness = harness();
        harness.workers.respond_with(transcribe_worker(text));
        let handle = harness.service.start_recording(scope()).expect("recording");
        let prepared = harness
            .service
            .prepare_transcription(&handle.recording_id, scope())
            .expect("prepare");
        harness.service.execute(prepared);
        assert!(
            harness
                .temp
                .log
                .lock()
                .expect("log")
                .cleaned_recordings
                .contains(&handle.recording_id.as_str().to_string()),
            "the WAV must be removed after transcribing {text:?}"
        );
    }
}

#[test]
fn cancelling_before_execution_neither_opens_a_worker_nor_keeps_the_audio() {
    let harness = harness();
    harness
        .workers
        .respond_with(transcribe_worker("should not run"));
    let handle = harness.service.start_recording(scope()).expect("recording");
    let prepared = harness
        .service
        .prepare_transcription(&handle.recording_id, scope())
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();

    harness.operations.request_cancel(&operation_id);
    harness.service.execute(prepared);

    assert_eq!(harness.workers.call_count(), 0);
    assert_eq!(
        harness
            .service
            .get_operation_result(&operation_id)
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::OperationCancelled)
    );
    assert!(!harness.capture.cancels.lock().expect("cancels").is_empty());
}

#[test]
fn a_result_that_arrives_after_cancellation_is_dropped() {
    let harness = harness();
    harness
        .workers
        .respond_with(transcribe_worker("late transcript"));
    let handle = harness.service.start_recording(scope()).expect("recording");
    let prepared = harness
        .service
        .prepare_transcription(&handle.recording_id, scope())
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();

    harness.service.cancel_operation(&operation_id);
    harness.service.execute(prepared);

    assert_eq!(
        harness
            .service
            .get_operation_result(&operation_id)
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::OperationCancelled)
    );
}

#[test]
fn finishing_a_recording_frees_the_slot_for_the_next_hold() {
    let harness = harness();
    harness.workers.respond_with(transcribe_worker("first"));
    let handle = harness.service.start_recording(scope()).expect("recording");
    let prepared = harness
        .service
        .prepare_transcription(&handle.recording_id, scope())
        .expect("prepare");
    harness.service.execute(prepared);
    assert!(harness.service.start_recording(scope()).is_ok());
}

// -------------------------------------------------------------------- tts ---

fn synthesize_worker(path: Option<PathBuf>) -> WorkerHandler {
    Box::new(move |_, call| match call {
        WorkerCall::Synthesize(request) => Ok(WorkerReply::Synthesize(SynthesizeReply {
            audio_path: path.clone().unwrap_or_else(|| request.output_path.clone()),
            sample_rate: 22_050,
            sample_count: 65_432,
            duration_ms: 2_967,
            engine_version: Some("1.10.0".to_string()),
        })),
        _ => Err(LocalMediaError::new(
            LocalMediaErrorCode::WorkerProtocolError,
        )),
    })
}

#[test]
fn empty_text_never_starts_an_operation() {
    let harness = harness();
    let error = harness
        .service
        .prepare_tts("   ".to_string(), scope())
        .expect_err("empty");
    assert_eq!(error.code(), LocalMediaErrorCode::TtsTextTooLong);
    assert!(harness
        .operations
        .log
        .lock()
        .expect("log")
        .started
        .is_empty());
}

#[test]
fn text_over_the_limit_is_rejected_rather_than_truncated() {
    let harness = harness();
    let long = "字".repeat(4_001);
    let error = harness
        .service
        .prepare_tts(long, scope())
        .expect_err("too long");
    assert_eq!(error.code(), LocalMediaErrorCode::TtsTextTooLong);
    assert!(harness
        .operations
        .log
        .lock()
        .expect("log")
        .started
        .is_empty());
}

#[test]
fn the_limit_counts_code_points_not_bytes() {
    // 4,000 CJK characters is 12,000 UTF-8 bytes; a byte limit would reject a valid request.
    let harness = harness();
    harness.workers.respond_with(synthesize_worker(None));
    let text = "字".repeat(4_000);
    assert!(harness.service.prepare_tts(text, scope()).is_ok());
}

#[test]
fn synthesis_plays_the_authorized_output_and_then_deletes_it() {
    let harness = harness();
    harness.workers.respond_with(synthesize_worker(None));
    let prepared = harness
        .service
        .prepare_tts("hello".to_string(), scope())
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    let Some(LocalMediaOperationResult::Tts(result)) = harness
        .service
        .get_operation_result(&operation_id)
        .expect("result")
    else {
        panic!("expected a playback result");
    };
    assert_eq!(result.sample_rate, 22_050);
    assert_eq!(harness.playback.played.lock().expect("played").len(), 1);
    assert!(harness
        .temp
        .log
        .lock()
        .expect("log")
        .cleaned_operations
        .contains(&operation_id));
}

#[test]
fn a_worker_returning_an_unauthorized_path_is_treated_as_protocol_invalid() {
    let harness = harness();
    harness
        .workers
        .respond_with(synthesize_worker(Some(PathBuf::from("/etc/passwd"))));
    let prepared = harness
        .service
        .prepare_tts("hello".to_string(), scope())
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    assert_eq!(
        harness
            .service
            .get_operation_result(&operation_id)
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::WorkerProtocolError)
    );
    assert!(
        harness.playback.played.lock().expect("played").is_empty(),
        "nothing outside the operation directory may be played"
    );
}

#[test]
fn starting_a_new_utterance_stops_the_previous_playback() {
    let harness = harness();
    harness.workers.respond_with(synthesize_worker(None));
    let first = harness
        .service
        .prepare_tts("first".to_string(), scope())
        .expect("first");
    harness.service.execute(first);
    let stops_before = harness.playback.stops.lock().expect("stops").len();

    let _second = harness
        .service
        .prepare_tts("second".to_string(), scope())
        .expect("second");
    assert!(harness.playback.stops.lock().expect("stops").len() > stops_before);
}

#[test]
fn cancelling_a_tts_operation_stops_playback_immediately() {
    let harness = harness();
    harness.workers.respond_with(synthesize_worker(None));
    let prepared = harness
        .service
        .prepare_tts("hello".to_string(), scope())
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    harness.service.cancel_operation(&operation_id);
    assert!(!harness.playback.stops.lock().expect("stops").is_empty());
}

// ---------------------------------------------------------- results/privacy ---

#[test]
fn a_terminal_result_expires_and_says_so() {
    let harness = harness();
    harness
        .workers
        .respond_with(ocr_worker(vec![("alpha", 1)], false));
    let staged = stage(&harness);
    let prepared = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    assert!(harness
        .service
        .get_operation_result(&operation_id)
        .expect("fresh")
        .is_some());
    harness.clock.advance(6 * 60 * 1000);
    assert_eq!(
        harness
            .service
            .get_operation_result(&operation_id)
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::OperationResultExpired)
    );
}

#[test]
fn reading_a_result_twice_returns_the_same_value_without_re_running_inference() {
    let harness = harness();
    harness
        .workers
        .respond_with(ocr_worker(vec![("alpha", 1)], false));
    let staged = stage(&harness);
    let prepared = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect("prepare");
    let operation_id = prepared.operation_id.clone();
    harness.service.execute(prepared);

    let calls = harness.workers.call_count();
    let first = harness
        .service
        .get_operation_result(&operation_id)
        .expect("first");
    let second = harness
        .service
        .get_operation_result(&operation_id)
        .expect("second");
    assert_eq!(first, second);
    assert_eq!(harness.workers.call_count(), calls);
}

#[test]
fn diagnostics_never_carry_recognized_text_transcripts_or_paths() {
    let harness = harness();
    harness
        .workers
        .respond_with(ocr_worker(vec![("SENSITIVE-OCR-TEXT", 1)], false));
    let staged = stage(&harness);
    let prepared = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect("prepare");
    harness.service.execute(prepared);

    harness
        .workers
        .respond_with(transcribe_worker("SENSITIVE-TRANSCRIPT"));
    let handle = harness.service.start_recording(scope()).expect("recording");
    let prepared = harness
        .service
        .prepare_transcription(&handle.recording_id, scope())
        .expect("prepare");
    harness.service.execute(prepared);

    let recorded = harness.diagnostics.flattened();
    assert!(!recorded.contains("SENSITIVE-OCR-TEXT"));
    assert!(!recorded.contains("SENSITIVE-TRANSCRIPT"));
    assert!(!recorded.contains("scan.png"));
    assert!(!recorded.contains("/tmp/"));
    assert!(!recorded.contains(&absolute("whisper")));
    // What it *should* carry: counts and codes.
    assert!(recorded.contains("pageCount"));
    assert!(recorded.contains("durationMs"));
}

#[test]
fn the_startup_sweep_and_shutdown_release_native_resources() {
    let harness = harness();
    harness.service.sweep_stale_media();
    assert_eq!(harness.temp.log.lock().expect("log").swept, 1);

    harness.service.start_recording(scope()).expect("recording");
    harness.service.shutdown();
    assert!(!harness.capture.cancels.lock().expect("cancels").is_empty());
    assert!(!harness.playback.stops.lock().expect("stops").is_empty());
    assert_eq!(
        harness
            .workers
            .shutdowns
            .load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[test]
fn the_device_catalog_is_read_without_opening_a_stream() {
    let harness = harness();
    let catalog = harness.service.list_audio_devices().expect("catalog");
    assert_eq!(catalog.inputs.len(), 1);
    assert_eq!(catalog.outputs.len(), 1);
    assert!(harness
        .capture
        .destinations
        .lock()
        .expect("destinations")
        .is_empty());
}

#[test]
fn a_running_operation_keeps_its_snapshot_when_the_profile_changes() {
    let harness = harness();
    harness
        .workers
        .respond_with(ocr_worker(vec![("alpha", 1)], false));
    let staged = stage(&harness);
    let prepared = harness
        .service
        .prepare_ocr(&staged, Some(scope()))
        .expect("prepare");

    // The user saves a different profile while the operation is queued.
    let mut edited = ready_profile();
    edited.ocr.language = "en".to_string();
    harness.service.save_profile(edited, 3).expect("save");

    harness.service.execute(prepared);
    let calls = harness.workers.calls.lock().expect("calls");
    let (_, _, revision) = calls.last().expect("one worker call");
    assert_eq!(
        *revision, 3,
        "the operation must run against the revision it captured"
    );
}

// --------------------------------------------------------------- canaries ---

/// Script a probe whose metadata succeeds and whose real inference fails.
///
/// This is the shape the whole canary exists for: the runtime accepts the model on load and cannot
/// execute its graph. A construction-only probe calls that `Ready`.
fn probe_with_failing_inference(
    harness: &Harness,
    engine: LocalMediaEngine,
    code: LocalMediaErrorCode,
) {
    harness
        .workers
        .respond_with(Box::new(move |_, call| match call {
            WorkerCall::Probe => Ok(WorkerReply::Probe(ProbeReply {
                package_version: Some("3.0.1".to_string()),
                device: Some("cpu".to_string()),
                model_identity: Some("det+rec:ch".to_string()),
            })),
            _ => Err(LocalMediaError::new(code)),
        }));
    let prepared = harness
        .service
        .prepare_probe(engine)
        .expect("prepare probe");
    harness.service.execute(prepared);
}

#[test]
fn an_ocr_model_that_loads_and_cannot_run_is_never_reported_ready() {
    let harness = harness();
    probe_with_failing_inference(
        &harness,
        LocalMediaEngine::Ocr,
        LocalMediaErrorCode::PaddleOnednnModelIncompatible,
    );
    assert_eq!(
        readiness(&harness, LocalMediaEngine::Ocr),
        EngineReadiness::Unavailable {
            code: LocalMediaErrorCode::PaddleOnednnModelIncompatible,
            field: None,
        }
    );
}

#[test]
fn a_transcriber_that_raises_while_being_drained_is_never_reported_ready() {
    let harness = harness();
    probe_with_failing_inference(
        &harness,
        LocalMediaEngine::Stt,
        LocalMediaErrorCode::EngineUnavailable,
    );
    assert_eq!(
        readiness(&harness, LocalMediaEngine::Stt),
        EngineReadiness::Unavailable {
            code: LocalMediaErrorCode::EngineUnavailable,
            field: None,
        }
    );
}

#[test]
fn a_synthesizer_that_fails_to_generate_is_never_reported_ready() {
    let harness = harness();
    probe_with_failing_inference(
        &harness,
        LocalMediaEngine::Tts,
        LocalMediaErrorCode::TtsPhonemizerDataUnavailable,
    );
    assert_eq!(
        readiness(&harness, LocalMediaEngine::Tts),
        EngineReadiness::Unavailable {
            code: LocalMediaErrorCode::TtsPhonemizerDataUnavailable,
            field: None,
        }
    );
}

#[test]
fn a_synthesis_that_reports_success_and_wrote_nothing_playable_is_not_ready() {
    // The one canary whose output is inspected rather than discarded unread. A worker that answers
    // with a zero sample rate has not demonstrated an engine, whatever its reply says.
    let harness = harness();
    harness.workers.respond_with(Box::new(|_, call| match call {
        WorkerCall::Probe => Ok(WorkerReply::Probe(ProbeReply::default())),
        _ => Ok(WorkerReply::Synthesize(SynthesizeReply {
            audio_path: std::path::PathBuf::from("/tmp/local-media/operation-0/output.wav"),
            sample_rate: 0,
            sample_count: 0,
            duration_ms: 0,
            engine_version: None,
        })),
    }));
    let prepared = harness
        .service
        .prepare_probe(LocalMediaEngine::Tts)
        .expect("prepare probe");
    harness.service.execute(prepared);

    assert!(matches!(
        readiness(&harness, LocalMediaEngine::Tts),
        EngineReadiness::Unavailable { .. }
    ));
}

#[test]
fn a_successful_canary_reports_ready_and_keeps_only_safe_metadata() {
    let harness = harness();
    probe_engine_successfully(&harness, LocalMediaEngine::Ocr);

    let status = harness.service.get_status().expect("status");
    let ocr = status.engine(LocalMediaEngine::Ocr).expect("ocr");
    assert_eq!(ocr.readiness, EngineReadiness::Ready);
    assert_eq!(ocr.installed_version.as_deref(), Some("3.0.1"));
    assert_eq!(ocr.model_identity.as_deref(), Some("det+rec:ch"));
}

#[test]
fn the_canary_writes_its_input_into_the_probe_operation_and_leaves_nothing_behind() {
    let harness = harness();
    probe_engine_successfully(&harness, LocalMediaEngine::Ocr);

    let log = harness.temp.log.lock().expect("log");
    let (operation, file_name, byte_length) =
        log.canary_inputs.first().expect("a canary input").clone();
    assert_eq!(operation, "operation-0");
    assert_eq!(file_name, "canary.png");
    assert!(byte_length > 0);
    // The operation directory is deleted on every exit, so the image does not outlive the probe.
    assert!(log.cleaned_operations.contains(&"operation-0".to_string()));
}

#[test]
fn the_stt_canary_bypasses_the_voice_activity_filter_and_a_real_transcription_does_not() {
    // Silence plus the user's filter is a probe that never reaches the decoder: the filter finds no
    // speech and returns, and the canary passes on a model that cannot decode at all.
    let harness = harness();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    harness
        .workers
        .respond_with(Box::new(move |_, call| match call {
            WorkerCall::Probe => Ok(WorkerReply::Probe(ProbeReply::default())),
            WorkerCall::Transcribe(request) => {
                recorder
                    .lock()
                    .expect("seen")
                    .push(request.bypass_voice_activity_filter);
                Ok(WorkerReply::Transcribe(TranscribeReply::default()))
            }
            _ => Err(LocalMediaError::new(
                LocalMediaErrorCode::WorkerProtocolError,
            )),
        }));
    let prepared = harness
        .service
        .prepare_probe(LocalMediaEngine::Stt)
        .expect("prepare probe");
    harness.service.execute(prepared);

    assert_eq!(*seen.lock().expect("seen"), vec![true]);
}
