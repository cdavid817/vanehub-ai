use super::*;

use crate::contexts::local_media::application::worker_contract::{
    OcrWorkerRequest, SttWorkerRequest, TtsWorkerRequest,
};
use crate::contexts::local_media::domain::{
    ComposerScopeId, LocalMediaOperationId, LocalMediaOperationKind, LocalMediaProfile,
    OcrCpuAcceleration, OcrMediaType, TtsModelKind,
};

fn snapshot(engine: LocalMediaEngine) -> LocalMediaProfileSnapshot {
    let mut profile = LocalMediaProfile::disabled_default("2026-01-01T00:00:00Z".to_string());
    profile.revision = 5;
    profile.enabled = true;

    profile.ocr.enabled = true;
    profile.ocr.python_executable = "/usr/bin/python3".to_string();
    profile.ocr.text_detection_model_dir = Some("/models/det".to_string());
    profile.ocr.text_recognition_model_dir = Some("/models/rec".to_string());
    profile.ocr.language = "ch".to_string();
    profile.ocr.max_pdf_pages = 9;

    profile.stt.enabled = true;
    profile.stt.model_directory = "/models/whisper".to_string();
    profile.stt.beam_size = 4;
    profile.stt.vad_filter = false;

    profile.tts.enabled = true;
    profile.tts.model_kind = TtsModelKind::Kokoro;
    profile.tts.model_path = "/models/kokoro.onnx".to_string();
    profile.tts.tokens_path = "/models/tokens.txt".to_string();
    profile.tts.voices_path = Some("/models/voices.bin".to_string());
    profile.tts.rule_fsts = vec!["/models/a.fst".to_string()];
    profile.tts.speaker_id = 3;
    profile.tts.num_threads = 2;

    LocalMediaProfileSnapshot::capture(
        LocalMediaOperationId::new("lmo-0123456789abcdef0123456789abcdef"),
        LocalMediaOperationKind::Probe,
        engine,
        &profile,
        Some(ComposerScopeId::new("session-1")),
        "2026-01-01T00:00:01Z".to_string(),
    )
}

fn hello(engine: &str, capabilities: &[&str]) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "v": 1,
        "type": "hello",
        "engine": engine,
        "workerVersion": "1",
        "packageVersion": "3.0.1",
        "capabilities": capabilities,
    }))
    .expect("encode hello")
}

// ------------------------------------------------------------- handshake ---

#[test]
fn a_matching_hello_is_accepted() {
    let frame = hello("paddleocr", &["probe", "ocr", "cancel", "shutdown"]);
    let parsed = validate_hello(&frame, LocalMediaEngine::Ocr).expect("hello");
    assert_eq!(parsed.package_version.as_deref(), Some("3.0.1"));
}

#[test]
fn a_hello_from_the_wrong_engine_is_refused() {
    // Routing an OCR request into a speech worker is worse than failing to start one.
    let frame = hello(
        "faster-whisper",
        &["probe", "transcribe", "cancel", "shutdown"],
    );
    assert_eq!(
        validate_hello(&frame, LocalMediaEngine::Ocr)
            .map(|_| ())
            .map_err(|e| e.code()),
        Err(LocalMediaErrorCode::WorkerProtocolError)
    );
}

#[test]
fn a_hello_missing_a_required_capability_is_refused() {
    let frame = hello("faster-whisper", &["probe", "cancel", "shutdown"]);
    assert!(validate_hello(&frame, LocalMediaEngine::Stt).is_err());
}

#[test]
fn a_hello_with_an_unknown_protocol_version_is_refused() {
    let frame = serde_json::to_vec(&json!({
        "v": 2, "type": "hello", "engine": "paddleocr",
        "capabilities": ["probe", "ocr", "cancel", "shutdown"],
    }))
    .expect("encode");
    assert!(validate_hello(&frame, LocalMediaEngine::Ocr).is_err());
}

#[test]
fn non_protocol_stdout_is_refused_as_a_handshake() {
    for contamination in [
        &b"Downloading model files: 12%"[..],
        &b"{ not json"[..],
        &b""[..],
        &b"{\"v\":1,\"type\":\"response\",\"id\":\"x\",\"ok\":true}"[..],
    ] {
        assert!(
            validate_hello(contamination, LocalMediaEngine::Ocr).is_err(),
            "stdout contamination must not pass as a handshake"
        );
    }
}

// -------------------------------------------------------------- responses ---

fn ocr_call() -> WorkerCall {
    WorkerCall::Ocr(OcrWorkerRequest {
        source_path: PathBuf::from("/tmp/local-media/staging/lmi-1/source.png"),
        media_type: OcrMediaType::Image,
        max_pdf_pages: 9,
        max_output_characters: 1_000,
    })
}

#[test]
fn a_matching_response_parses_into_a_typed_reply() {
    let frame = serde_json::to_vec(&json!({
        "v": 1, "type": "response", "id": "req-1", "ok": true,
        "result": {
            "pages": [{"pageNumber": 1, "text": "alpha", "lineCount": 1}],
            "characterCount": 5, "truncated": false, "noTextDetected": false,
            "engineVersion": "3.0.1", "modelIdentity": "det+rec:ch"
        }
    }))
    .expect("encode");

    let WorkerReply::Ocr(reply) = parse_response(&frame, "req-1", &ocr_call()).expect("parse")
    else {
        panic!("expected an OCR reply");
    };
    assert_eq!(reply.pages.len(), 1);
    assert_eq!(reply.pages[0].text, "alpha");
    assert_eq!(reply.engine_version.as_deref(), Some("3.0.1"));
}

#[test]
fn a_response_for_a_different_request_is_a_protocol_error() {
    // The stream is now out of step with the queue; there is no correct way to resynchronize.
    let frame = serde_json::to_vec(&json!({
        "v": 1, "type": "response", "id": "req-OTHER", "ok": true, "result": {"pages": []}
    }))
    .expect("encode");
    assert_eq!(
        parse_response(&frame, "req-1", &ocr_call())
            .map(|_| ())
            .map_err(|e| e.code()),
        Err(LocalMediaErrorCode::WorkerProtocolError)
    );
}

#[test]
fn a_response_with_no_id_is_a_protocol_error() {
    let frame = serde_json::to_vec(&json!({
        "v": 1, "type": "response", "ok": true, "result": {"pages": []}
    }))
    .expect("encode");
    assert!(parse_response(&frame, "req-1", &ocr_call()).is_err());
}

#[test]
fn a_successful_response_with_no_result_is_a_protocol_error() {
    let frame = serde_json::to_vec(&json!({
        "v": 1, "type": "response", "id": "req-1", "ok": true
    }))
    .expect("encode");
    assert!(parse_response(&frame, "req-1", &ocr_call()).is_err());
}

#[test]
fn a_worker_error_maps_to_its_stable_code_with_safe_details() {
    let frame = serde_json::to_vec(&json!({
        "v": 1, "type": "response", "id": "req-1", "ok": false,
        "error": {
            "code": "MODEL_NOT_FOUND",
            "messageKey": "localMedia.errors.modelNotFound",
            "retryable": false,
            "safeDetails": {"engine": "paddleocr", "limit": 20, "truncated": true}
        }
    }))
    .expect("encode");

    let error = parse_response(&frame, "req-1", &ocr_call()).expect_err("mapped error");
    assert_eq!(error.code(), LocalMediaErrorCode::ModelNotFound);
    assert_eq!(error.details().len(), 3);
}

#[test]
fn an_unknown_error_code_is_not_passed_through() {
    // Passing it through would render as a missing translation in the composer.
    let frame = serde_json::to_vec(&json!({
        "v": 1, "type": "response", "id": "req-1", "ok": false,
        "error": {"code": "SOMETHING_NEW", "messageKey": "x", "retryable": false}
    }))
    .expect("encode");
    assert_eq!(
        parse_response(&frame, "req-1", &ocr_call())
            .map(|_| ())
            .map_err(|e| e.code()),
        Err(LocalMediaErrorCode::WorkerProtocolError)
    );
}

#[test]
fn a_synthesis_reply_without_an_audio_path_is_a_protocol_error() {
    let call = WorkerCall::Synthesize(TtsWorkerRequest {
        text: "hello".to_string(),
        output_path: PathBuf::from("/tmp/local-media/operations/op-1/output.wav"),
    });
    let frame = serde_json::to_vec(&json!({
        "v": 1, "type": "response", "id": "req-1", "ok": true,
        "result": {"sampleRate": 22050, "sampleCount": 100, "durationMs": 5}
    }))
    .expect("encode");
    assert!(parse_response(&frame, "req-1", &call).is_err());
}

#[test]
fn a_transcription_reply_keeps_its_language_metadata() {
    let call = WorkerCall::Transcribe(SttWorkerRequest {
        audio_path: PathBuf::from("/tmp/local-media/recordings/lmr-1/input.wav"),
        bypass_voice_activity_filter: false,
    });
    let frame = serde_json::to_vec(&json!({
        "v": 1, "type": "response", "id": "req-1", "ok": true,
        "result": {
            "text": "你好", "detectedLanguage": "zh", "languageProbability": 0.98,
            "durationMs": 6432, "noSpeechDetected": false, "device": "cpu"
        }
    }))
    .expect("encode");
    let WorkerReply::Transcribe(reply) = parse_response(&frame, "req-1", &call).expect("parse")
    else {
        panic!("expected a transcription reply");
    };
    assert_eq!(reply.text, "你好");
    assert_eq!(reply.detected_language.as_deref(), Some("zh"));
    assert_eq!(reply.duration_ms, Some(6432));
}

// ----------------------------------------------------------------- params ---

#[test]
fn ocr_params_carry_the_staged_path_and_explicit_model_directories() {
    let snapshot = snapshot(LocalMediaEngine::Ocr);
    let params = request_params(&snapshot, &ocr_call());
    assert_eq!(
        params["sourcePath"],
        "/tmp/local-media/staging/lmi-1/source.png"
    );
    assert_eq!(params["textDetectionModelDir"], "/models/det");
    assert_eq!(params["textRecognitionModelDir"], "/models/rec");
    assert_eq!(params["mediaType"], "image");
    assert_eq!(params["maxPdfPages"], 9);
    assert_eq!(params["language"], "ch");
}

#[test]
fn an_unset_optional_model_path_is_omitted_rather_than_sent_as_empty() {
    // An empty string would reach PaddleOCR as a configured-but-invalid path; omission is what
    // makes the worker disable the stage instead of resolving a default checkpoint.
    let snapshot = snapshot(LocalMediaEngine::Ocr);
    let params = request_params(&snapshot, &ocr_call());
    assert!(params.get("textLineOrientationModelDir").is_none());
    assert!(params.get("paddleXConfigPath").is_none());
}

#[test]
fn stt_params_mirror_the_snapshot_not_the_live_profile() {
    let snapshot = snapshot(LocalMediaEngine::Stt);
    let params = request_params(
        &snapshot,
        &WorkerCall::Transcribe(SttWorkerRequest {
            audio_path: PathBuf::from("/tmp/local-media/recordings/lmr-1/input.wav"),
            bypass_voice_activity_filter: false,
        }),
    );
    assert_eq!(params["modelDirectory"], "/models/whisper");
    assert_eq!(params["beamSize"], 4);
    assert_eq!(params["vadFilter"], false);
    assert_eq!(
        params["audioPath"],
        "/tmp/local-media/recordings/lmr-1/input.wav"
    );
}

#[test]
fn tts_params_include_the_model_kind_specific_files() {
    let snapshot = snapshot(LocalMediaEngine::Tts);
    let params = request_params(
        &snapshot,
        &WorkerCall::Synthesize(TtsWorkerRequest {
            text: "hello".to_string(),
            output_path: PathBuf::from("/tmp/local-media/operations/op-1/output.wav"),
        }),
    );
    assert_eq!(params["modelKind"], "kokoro");
    assert_eq!(params["voicesPath"], "/models/voices.bin");
    assert_eq!(params["ruleFsts"][0], "/models/a.fst");
    assert_eq!(params["speakerId"], 3);
    assert_eq!(
        params["outputPath"],
        "/tmp/local-media/operations/op-1/output.wav"
    );
    assert_eq!(params["text"], "hello");
}

#[test]
fn a_probe_carries_configuration_but_no_media_path() {
    for engine in LocalMediaEngine::ALL {
        let params = request_params(&snapshot(engine), &WorkerCall::Probe);
        for key in ["sourcePath", "audioPath", "outputPath", "text"] {
            assert!(params.get(key).is_none(), "{engine:?} probe leaked {key}");
        }
    }
}

// ----------------------------------------------------------------- frames ---

#[test]
fn an_encoded_request_is_one_line_of_protocol_v1() {
    let encoded = encode_request("req-1", "ocr", json!({"sourcePath": "/tmp/x"}));
    let value: Value = serde_json::from_slice(&encoded).expect("decode");
    assert_eq!(value["v"], 1);
    assert_eq!(value["type"], "request");
    assert_eq!(value["id"], "req-1");
    assert_eq!(value["method"], "ocr");
    assert!(
        !encoded.contains(&b'\n'),
        "the newline is added by the writer, not the encoder"
    );
}

#[test]
fn control_frames_reference_the_request_they_act_on() {
    for frame_type in ["cancel", "shutdown"] {
        let encoded = encode_control(frame_type, "req-7");
        let value: Value = serde_json::from_slice(&encoded).expect("decode");
        assert_eq!(value["type"], frame_type);
        assert_eq!(value["id"], "req-7");
        assert_eq!(value["v"], 1);
    }
}

// The OCR output ceiling is 200,000 characters; the response bound has to clear that even at four
// bytes per character plus JSON overhead. A `const` assertion rather than a `#[test]` one: both
// operands are compile-time constants, so this belongs to the build, not to the suite.
const _: () = assert!(MAX_RESPONSE_FRAME_BYTES > 200_000 * 4);
const _: () = assert!(MAX_REQUEST_FRAME_BYTES < MAX_RESPONSE_FRAME_BYTES);

#[test]
fn ocr_params_carry_the_acceleration_mode_from_the_snapshot() {
    // The mode travels with the snapshot rather than being read live, so a settings change while an
    // operation is in flight cannot alter the acceleration that operation was accepted under.
    let mut snapshot = snapshot(LocalMediaEngine::Ocr);
    assert_eq!(
        request_params(&snapshot, &ocr_call())["cpuAcceleration"],
        "library-default"
    );

    snapshot = snapshot_with_acceleration(OcrCpuAcceleration::Disabled);
    assert_eq!(
        request_params(&snapshot, &ocr_call())["cpuAcceleration"],
        "disabled"
    );

    snapshot = snapshot_with_acceleration(OcrCpuAcceleration::Enabled);
    assert_eq!(
        request_params(&snapshot, &ocr_call())["cpuAcceleration"],
        "enabled"
    );
}

#[test]
fn the_acceleration_mode_is_always_present_so_the_worker_never_guesses() {
    // An absent key would leave the worker deciding for itself, which is how a mode that was
    // explicitly chosen becomes a mode nobody can account for afterwards.
    let snapshot = snapshot(LocalMediaEngine::Ocr);
    assert!(request_params(&snapshot, &ocr_call())
        .get("cpuAcceleration")
        .is_some());
}

fn snapshot_with_acceleration(mode: OcrCpuAcceleration) -> LocalMediaProfileSnapshot {
    let mut profile = LocalMediaProfile::disabled_default("2026-01-01T00:00:00Z".to_string());
    profile.revision = 5;
    profile.enabled = true;
    profile.ocr.enabled = true;
    profile.ocr.python_executable = "/usr/bin/python3".to_string();
    profile.ocr.text_detection_model_dir = Some("/models/det".to_string());
    profile.ocr.text_recognition_model_dir = Some("/models/rec".to_string());
    profile.ocr.cpu_acceleration = mode;
    LocalMediaProfileSnapshot::capture(
        LocalMediaOperationId::new("lmo-0123456789abcdef0123456789abcdef"),
        LocalMediaOperationKind::Probe,
        LocalMediaEngine::Ocr,
        &profile,
        Some(ComposerScopeId::new("session-1")),
        "2026-01-01T00:00:01Z".to_string(),
    )
}
