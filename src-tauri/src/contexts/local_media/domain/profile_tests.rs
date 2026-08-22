use super::super::engine::LocalMediaEngine;
use super::super::error::LocalMediaErrorCode;
use super::super::validation::{first_error, is_valid_path, validate_profile, ProfileFieldIssue};
use super::*;

/// Absolute on the host that is running the test. Hard-coding one spelling would make the suite
/// pass on Windows and fail on the Linux CI runner for a reason unrelated to the assertion.
fn absolute(name: &str) -> String {
    if cfg!(windows) {
        format!("C:\\vanehub-test\\{name}")
    } else {
        format!("/opt/vanehub-test/{name}")
    }
}

fn base() -> LocalMediaProfile {
    LocalMediaProfile::disabled_default("2026-01-01T00:00:00Z".to_string())
}

fn configured_ocr() -> PaddleOcrProfile {
    PaddleOcrProfile {
        enabled: true,
        python_executable: absolute("python"),
        text_detection_model_dir: Some(absolute("det")),
        text_recognition_model_dir: Some(absolute("rec")),
        ..PaddleOcrProfile::default()
    }
}

fn configured_stt() -> FasterWhisperProfile {
    FasterWhisperProfile {
        enabled: true,
        python_executable: absolute("python"),
        model_directory: absolute("whisper"),
        ..FasterWhisperProfile::default()
    }
}

fn configured_tts() -> SherpaOnnxTtsProfile {
    SherpaOnnxTtsProfile {
        enabled: true,
        python_executable: absolute("python"),
        model_path: absolute("model.onnx"),
        tokens_path: absolute("tokens.txt"),
        ..SherpaOnnxTtsProfile::default()
    }
}

fn fields(issues: &[ProfileFieldIssue]) -> Vec<&str> {
    issues.iter().map(|issue| issue.field.as_str()).collect()
}

#[test]
fn the_default_profile_is_off_and_unconfigured() {
    let profile = base();
    assert_eq!(profile.profile_id, DEFAULT_PROFILE_ID);
    assert_eq!(profile.revision, 0);
    assert!(!profile.enabled);
    assert!(!profile.ocr.enabled);
    assert!(!profile.stt.enabled);
    assert!(!profile.tts.enabled);
    assert!(profile.ocr.python_executable.is_empty());
    assert!(profile.stt.model_directory.is_empty());
    assert!(profile.tts.model_path.is_empty());
}

#[test]
fn a_fully_disabled_profile_validates() {
    assert!(validate_profile(&base()).is_empty());
    assert!(first_error(&validate_profile(&base())).is_none());
}

#[test]
fn a_disabled_engine_with_empty_paths_does_not_block_saving() {
    // The user configures one engine at a time; a half-finished section that is switched off must
    // not hold the save hostage.
    let mut profile = base();
    profile.enabled = true;
    profile.stt = configured_stt();
    profile.ocr.enabled = false;
    profile.tts.enabled = false;
    assert!(validate_profile(&profile).is_empty());
}

#[test]
fn an_enabled_engine_without_an_executable_reports_engine_unconfigured() {
    let mut profile = base();
    profile.ocr = PaddleOcrProfile {
        python_executable: String::new(),
        ..configured_ocr()
    };
    let issues = validate_profile(&profile);
    assert!(issues.iter().any(|issue| issue.field == "pythonExecutable"
        && issue.code == LocalMediaErrorCode::EngineUnconfigured));
    assert_eq!(issues[0].engine, Some(LocalMediaEngine::Ocr));
}

#[test]
fn ocr_accepts_either_a_paddlex_config_or_explicit_model_directories() {
    let mut with_paddle_x = base();
    with_paddle_x.ocr = PaddleOcrProfile {
        enabled: true,
        python_executable: absolute("python"),
        paddle_x_config_path: Some(absolute("pipeline.yaml")),
        ..PaddleOcrProfile::default()
    };
    assert!(validate_profile(&with_paddle_x).is_empty());

    let mut with_directories = base();
    with_directories.ocr = configured_ocr();
    assert!(validate_profile(&with_directories).is_empty());
}

#[test]
fn ocr_with_neither_model_form_points_at_the_paddlex_field() {
    let mut profile = base();
    profile.ocr = PaddleOcrProfile {
        enabled: true,
        python_executable: absolute("python"),
        ..PaddleOcrProfile::default()
    };
    let issues = validate_profile(&profile);
    assert_eq!(fields(&issues), vec!["paddleXConfigPath"]);
    assert_eq!(issues[0].code, LocalMediaErrorCode::ModelNotConfigured);
}

#[test]
fn a_half_filled_directory_pair_names_the_missing_half() {
    let mut profile = base();
    profile.ocr = PaddleOcrProfile {
        text_recognition_model_dir: None,
        ..configured_ocr()
    };
    let issues = validate_profile(&profile);
    assert_eq!(fields(&issues), vec!["textRecognitionModelDir"]);
}

#[test]
fn an_omitted_orientation_model_is_not_an_error() {
    let mut profile = base();
    profile.ocr = PaddleOcrProfile {
        text_line_orientation_model_dir: None,
        ..configured_ocr()
    };
    assert!(validate_profile(&profile).is_empty());
}

#[test]
fn a_relative_or_scheme_qualified_path_is_rejected() {
    assert!(!is_valid_path("models/det"));
    assert!(!is_valid_path("./models"));
    assert!(!is_valid_path("file:///opt/models"));
    assert!(!is_valid_path("http://example.invalid/model.onnx"));
    assert!(!is_valid_path("smb://host/share"));
    assert!(!is_valid_path(""));
    assert!(!is_valid_path("   "));
    assert!(!is_valid_path("/opt/mo\ndels"));
    assert!(is_valid_path(&absolute("det")));
}

#[test]
fn a_windows_drive_letter_is_not_mistaken_for_a_url_scheme() {
    // `C:` reaches the scheme check with a colon at index 1; only a longer prefix is a scheme.
    let windows_path = "C:\\models\\det";
    assert_eq!(is_valid_path(windows_path), cfg!(windows));
}

#[test]
fn numeric_bounds_are_enforced_per_field() {
    let mut profile = base();
    profile.stt = FasterWhisperProfile {
        beam_size: 11,
        ..configured_stt()
    };
    assert!(fields(&validate_profile(&profile)).contains(&"beamSize"));

    profile.stt = FasterWhisperProfile {
        beam_size: 0,
        ..configured_stt()
    };
    assert!(fields(&validate_profile(&profile)).contains(&"beamSize"));

    profile.stt = FasterWhisperProfile {
        max_recording_seconds: 121,
        ..configured_stt()
    };
    assert!(fields(&validate_profile(&profile)).contains(&"maxRecordingSeconds"));

    profile.stt = FasterWhisperProfile {
        max_recording_seconds: 4,
        ..configured_stt()
    };
    assert!(fields(&validate_profile(&profile)).contains(&"maxRecordingSeconds"));

    profile.stt = configured_stt();
    profile.tts = SherpaOnnxTtsProfile {
        speed: 2.5,
        ..configured_tts()
    };
    assert!(fields(&validate_profile(&profile)).contains(&"speed"));

    profile.tts = SherpaOnnxTtsProfile {
        num_threads: 17,
        ..configured_tts()
    };
    assert!(fields(&validate_profile(&profile)).contains(&"numThreads"));

    profile.tts = configured_tts();
    profile.ocr = PaddleOcrProfile {
        max_pdf_pages: 0,
        ..configured_ocr()
    };
    assert!(fields(&validate_profile(&profile)).contains(&"maxPdfPages"));

    profile.ocr = PaddleOcrProfile {
        max_pdf_pages: 51,
        ..configured_ocr()
    };
    assert!(fields(&validate_profile(&profile)).contains(&"maxPdfPages"));
}

#[test]
fn kokoro_requires_voices_and_matcha_requires_a_vocoder() {
    let mut profile = base();

    profile.tts = SherpaOnnxTtsProfile {
        model_kind: TtsModelKind::Kokoro,
        ..configured_tts()
    };
    assert!(fields(&validate_profile(&profile)).contains(&"voicesPath"));

    profile.tts = SherpaOnnxTtsProfile {
        model_kind: TtsModelKind::Kokoro,
        voices_path: Some(absolute("voices.bin")),
        ..configured_tts()
    };
    assert!(validate_profile(&profile).is_empty());

    profile.tts = SherpaOnnxTtsProfile {
        model_kind: TtsModelKind::Matcha,
        ..configured_tts()
    };
    assert!(fields(&validate_profile(&profile)).contains(&"vocoderPath"));

    profile.tts = SherpaOnnxTtsProfile {
        model_kind: TtsModelKind::Matcha,
        vocoder_path: Some(absolute("vocoder.onnx")),
        ..configured_tts()
    };
    assert!(validate_profile(&profile).is_empty());
}

#[test]
fn vits_and_piper_need_neither_voices_nor_a_vocoder() {
    for kind in [TtsModelKind::Vits, TtsModelKind::Piper] {
        let mut profile = base();
        profile.tts = SherpaOnnxTtsProfile {
            model_kind: kind,
            ..configured_tts()
        };
        assert!(
            validate_profile(&profile).is_empty(),
            "{kind:?} should validate"
        );
    }
}

#[test]
fn a_relative_rule_fst_is_rejected_once_not_per_entry() {
    let mut profile = base();
    profile.tts = SherpaOnnxTtsProfile {
        rule_fsts: vec!["rules/a.fst".to_string(), "rules/b.fst".to_string()],
        ..configured_tts()
    };
    let issues = validate_profile(&profile);
    assert_eq!(
        issues
            .iter()
            .filter(|issue| issue.field == "ruleFsts")
            .count(),
        1
    );
}

#[test]
fn a_non_default_profile_id_is_rejected_in_v1() {
    let mut profile = base();
    profile.profile_id = "secondary".to_string();
    assert!(fields(&validate_profile(&profile)).contains(&"profileId"));
}

#[test]
fn the_master_switch_gates_engine_enablement() {
    let mut profile = base();
    profile.ocr = configured_ocr();
    profile.enabled = false;
    assert!(!profile.engine_enabled(LocalMediaEngine::Ocr));
    profile.enabled = true;
    assert!(profile.engine_enabled(LocalMediaEngine::Ocr));
    assert!(!profile.engine_enabled(LocalMediaEngine::Stt));
}

#[test]
fn a_stored_recording_limit_cannot_exceed_the_hard_ceiling() {
    let mut profile = base();
    profile.stt.max_recording_seconds = 600;
    assert_eq!(profile.recording_limit_seconds(), MAX_RECORDING_SECONDS);
    profile.stt.max_recording_seconds = 1;
    assert_eq!(profile.recording_limit_seconds(), MIN_RECORDING_SECONDS);
    profile.stt.max_recording_seconds = 45;
    assert_eq!(profile.recording_limit_seconds(), 45);
}

#[test]
fn first_error_carries_the_field_and_engine_but_no_path() {
    let mut profile = base();
    profile.tts = SherpaOnnxTtsProfile {
        model_path: String::new(),
        ..configured_tts()
    };
    let error = first_error(&validate_profile(&profile)).expect("an issue");
    assert_eq!(error.code(), LocalMediaErrorCode::ModelNotConfigured);
    assert_eq!(
        error.details().get("field"),
        Some(&super::super::error::SafeDetail::Text("modelPath".into()))
    );
    assert_eq!(
        error.details().get("engine"),
        Some(&super::super::error::SafeDetail::Text("tts".into()))
    );
}

#[test]
fn the_profile_round_trips_through_json_with_camel_case_keys() {
    let mut profile = base();
    profile.ocr = configured_ocr();
    let json = serde_json::to_value(&profile).expect("serialize profile");
    assert_eq!(json["profileId"], DEFAULT_PROFILE_ID);
    assert_eq!(json["ocr"]["maxPdfPages"], 20);
    assert!(json["ocr"].get("max_pdf_pages").is_none());
    let restored: LocalMediaProfile = serde_json::from_value(json).expect("deserialize profile");
    assert_eq!(restored, profile);
}

#[test]
fn missing_json_fields_fall_back_to_disabled_defaults() {
    // A row written by an older build must not fail to load; `#[serde(default)]` on each engine
    // struct is what keeps a schema addition backward compatible.
    let json = serde_json::json!({
        "profileId": "default",
        "revision": 4,
        "enabled": true,
        "ocr": {},
        "stt": {},
        "tts": {},
        "updatedAt": "2026-01-01T00:00:00Z"
    });
    let profile: LocalMediaProfile = serde_json::from_value(json).expect("deserialize");
    assert_eq!(profile.revision, 4);
    assert!(!profile.ocr.enabled);
    assert_eq!(profile.ocr.max_pdf_pages, 20);
    assert_eq!(profile.stt.beam_size, 5);
    assert_eq!(profile.tts.model_kind, TtsModelKind::Vits);
}
