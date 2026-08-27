use super::*;

#[test]
fn every_code_round_trips_through_its_wire_spelling() {
    for code in LocalMediaErrorCode::ALL {
        assert_eq!(
            LocalMediaErrorCode::parse(code.as_str()),
            Some(code),
            "{} did not round-trip",
            code.as_str()
        );
    }
}

#[test]
fn wire_spellings_are_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for code in LocalMediaErrorCode::ALL {
        assert!(
            seen.insert(code.as_str()),
            "duplicate wire spelling {}",
            code.as_str()
        );
    }
    assert_eq!(seen.len(), LocalMediaErrorCode::ALL.len());
}

#[test]
fn unknown_codes_are_rejected_rather_than_coerced() {
    assert_eq!(LocalMediaErrorCode::parse("MODEL_NOT_FOUND_"), None);
    assert_eq!(LocalMediaErrorCode::parse("model_not_found"), None);
    assert_eq!(LocalMediaErrorCode::parse(""), None);
}

#[test]
fn message_keys_are_camel_cased_under_one_namespace() {
    assert_eq!(
        LocalMediaErrorCode::ModelNotFound.message_key(),
        "localMedia.errors.modelNotFound"
    );
    assert_eq!(
        LocalMediaErrorCode::TtsTextTooLong.message_key(),
        "localMedia.errors.ttsTextTooLong"
    );
    assert_eq!(
        LocalMediaErrorCode::MicPermissionDenied.message_key(),
        "localMedia.errors.micPermissionDenied"
    );
    for code in LocalMediaErrorCode::ALL {
        let key = code.message_key();
        assert!(
            key.starts_with("localMedia.errors."),
            "{key} is outside the namespace"
        );
        assert!(!key.contains('_'), "{key} kept an underscore");
    }
}

#[test]
fn message_keys_are_unique() {
    let keys: std::collections::BTreeSet<String> = LocalMediaErrorCode::ALL
        .iter()
        .map(|code| code.message_key())
        .collect();
    assert_eq!(keys.len(), LocalMediaErrorCode::ALL.len());
}

#[test]
fn display_exposes_only_the_stable_code() {
    let error = LocalMediaError::new(LocalMediaErrorCode::ModelNotFound)
        .with_text("engine", "paddleocr")
        .with_number("limit", 20);
    assert_eq!(error.to_string(), "MODEL_NOT_FOUND");
}

#[test]
fn oversized_or_control_text_details_are_dropped_not_truncated() {
    let long_path =
        "C:\\Users\\someone\\models\\a-very-long-directory-name-that-keeps-going-and-going";
    let error = LocalMediaError::new(LocalMediaErrorCode::ModelNotFound)
        .with_text("engine", long_path)
        .with_text("field", "modelDirectory\n");
    assert!(
        error.details().get("engine").is_none(),
        "a long value must not be stored at all"
    );
    assert!(
        error.details().get("field").is_none(),
        "a control character must reject the value"
    );
}

#[test]
fn scalar_details_survive() {
    let error = LocalMediaError::new(LocalMediaErrorCode::PdfPageLimitExceeded)
        .with_text("engine", "paddleocr")
        .with_number("limit", 20)
        .with_flag("truncated", true);
    assert_eq!(
        error.details().get("engine"),
        Some(&SafeDetail::Text("paddleocr".into()))
    );
    assert_eq!(error.details().get("limit"), Some(&SafeDetail::Number(20)));
    assert_eq!(
        error.details().get("truncated"),
        Some(&SafeDetail::Flag(true))
    );
}

#[test]
fn cancellation_is_recognizable_without_string_matching() {
    assert!(LocalMediaError::new(LocalMediaErrorCode::OperationCancelled).is_cancelled());
    assert!(!LocalMediaError::new(LocalMediaErrorCode::WorkerCrashed).is_cancelled());
}

#[test]
fn codes_serialize_as_their_wire_spelling() {
    let json =
        serde_json::to_value(LocalMediaErrorCode::ProfileRevisionConflict).expect("serialize code");
    assert_eq!(json, serde_json::json!("PROFILE_REVISION_CONFLICT"));
}
