use super::*;

fn page(number: u32, text: &str) -> OcrPage {
    OcrPage {
        page_number: number,
        text: text.to_string(),
        line_count: text.lines().filter(|line| !line.is_empty()).count() as u32,
        lines: text
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| OcrLine {
                text: line.to_string(),
                confidence: None,
                polygon: None,
            })
            .collect(),
    }
}

#[test]
fn pages_join_with_a_blank_line_and_lines_with_a_newline() {
    let pages = vec![page(1, "alpha\nbeta"), page(2, "gamma")];
    assert_eq!(derive_plain_text(&pages), "alpha\nbeta\n\ngamma");
}

#[test]
fn a_single_page_gains_no_separator() {
    assert_eq!(derive_plain_text(&[page(1, "only")]), "only");
}

#[test]
fn empty_pages_do_not_produce_runs_of_blank_lines() {
    // A scanned document with a blank page must not add a paragraph break the user then has to
    // delete; the page still appears in `pages` so the count stays honest.
    let pages = vec![page(1, "alpha"), page(2, ""), page(3, "gamma")];
    assert_eq!(derive_plain_text(&pages), "alpha\n\ngamma");
}

#[test]
fn no_pages_derive_to_empty_text() {
    assert_eq!(derive_plain_text(&[]), "");
    assert_eq!(derive_plain_text(&[page(1, "")]), "");
}

#[test]
fn page_order_is_preserved_exactly_as_given() {
    // The engine's reading order is authoritative; nothing here re-sorts it.
    let pages = vec![page(3, "third"), page(1, "first"), page(2, "second")];
    assert_eq!(derive_plain_text(&pages), "third\n\nfirst\n\nsecond");
}

#[test]
fn crlf_is_normalized_but_punctuation_is_untouched() {
    assert_eq!(
        normalize_recognized_text("alpha\r\nbeta\rgamma"),
        "alpha\nbeta\ngamma"
    );
    assert_eq!(
        normalize_recognized_text("  中文，标点。  "),
        "中文，标点。"
    );
    assert_eq!(normalize_recognized_text("a\u{0000}b"), "ab");
    assert_eq!(normalize_recognized_text("\n\n text \n\n"), "text");
}

#[test]
fn an_empty_recognition_is_reported_as_a_user_outcome_not_a_crash() {
    let result = OcrResult {
        source: OcrSourceSummary {
            display_name: "scan.png".to_string(),
            media_type: OcrMediaType::Image,
            page_count: 1,
        },
        plain_text: String::new(),
        pages: vec![page(1, "")],
        warnings: Vec::new(),
        provenance: provenance(),
        character_count: 0,
        truncated: false,
    };
    assert!(result.is_empty());
    assert_eq!(
        result.outcome_code(),
        Some(LocalMediaErrorCode::NoTextDetected)
    );
}

#[test]
fn a_non_empty_recognition_has_no_outcome_code() {
    let result = OcrResult {
        source: OcrSourceSummary {
            display_name: "scan.png".to_string(),
            media_type: OcrMediaType::Image,
            page_count: 1,
        },
        plain_text: "alpha".to_string(),
        pages: vec![page(1, "alpha")],
        warnings: Vec::new(),
        provenance: provenance(),
        character_count: 5,
        truncated: false,
    };
    assert!(!result.is_empty());
    assert_eq!(result.outcome_code(), None);
}

fn provenance() -> OcrProvenance {
    OcrProvenance {
        engine: "paddleocr".to_string(),
        engine_version: Some("3.0.1".to_string()),
        profile_revision: 4,
        language: "ch".to_string(),
        model_identity: Some("det+rec:ch".to_string()),
    }
}

#[test]
fn ocr_provenance_identifies_the_engine_and_revision_without_a_path() {
    let json = serde_json::to_value(provenance()).expect("serialize provenance");
    assert_eq!(json["engine"], "paddleocr");
    assert_eq!(json["profileRevision"], 4);
    assert_eq!(json["modelIdentity"], "det+rec:ch");
    let keys: std::collections::BTreeSet<&str> = json
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "engine",
            "engineVersion",
            "language",
            "modelIdentity",
            "profileRevision"
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn an_empty_transcript_is_no_speech_detected() {
    let mut transcription = TranscriptionResult {
        text: String::new(),
        detected_language: Some("zh".to_string()),
        language_probability: Some(0.98),
        duration_ms: Some(6_400),
        limit_reached: false,
        provenance: TranscriptionProvenance {
            engine: "faster-whisper".to_string(),
            engine_version: None,
            profile_revision: 4,
            device: "cpu".to_string(),
        },
    };
    assert!(transcription.is_empty());
    assert_eq!(
        transcription.outcome_code(),
        Some(LocalMediaErrorCode::NoSpeechDetected)
    );

    transcription.text = "  ".to_string();
    assert!(transcription.is_empty(), "whitespace-only is still empty");

    transcription.text = "hello".to_string();
    assert!(!transcription.is_empty());
    assert_eq!(transcription.outcome_code(), None);
}

#[test]
fn a_limit_reached_transcription_is_still_a_success() {
    let transcription = TranscriptionResult {
        text: "hello".to_string(),
        detected_language: None,
        language_probability: None,
        duration_ms: Some(120_000),
        limit_reached: true,
        provenance: TranscriptionProvenance {
            engine: "faster-whisper".to_string(),
            engine_version: None,
            profile_revision: 1,
            device: "cpu".to_string(),
        },
    };
    assert!(!transcription.is_empty());
    assert_eq!(transcription.outcome_code(), None);
    assert!(transcription.limit_reached);
}

#[test]
fn playback_results_expose_an_opaque_id_and_never_a_path() {
    let playback = SpeechPlaybackResult {
        playback_id: PlaybackId::new("lmp-0123456789abcdef0123456789abcdef"),
        sample_rate: 22_050,
        duration_ms: 2_967,
        device_summary: Some("default".to_string()),
    };
    let json = serde_json::to_value(&playback).expect("serialize playback");
    let keys: std::collections::BTreeSet<&str> = json
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        ["deviceSummary", "durationMs", "playbackId", "sampleRate"]
            .into_iter()
            .collect()
    );
    assert!(!json.to_string().contains(".wav"));
}

#[test]
fn the_result_union_is_discriminated_by_kind() {
    let playback = LocalMediaOperationResult::Tts(SpeechPlaybackResult {
        playback_id: PlaybackId::new("lmp-0123456789abcdef0123456789abcdef"),
        sample_rate: 22_050,
        duration_ms: 100,
        device_summary: None,
    });
    let json = serde_json::to_value(&playback).expect("serialize union");
    assert_eq!(json["kind"], "tts");
    assert_eq!(json["result"]["sampleRate"], 22_050);
}
