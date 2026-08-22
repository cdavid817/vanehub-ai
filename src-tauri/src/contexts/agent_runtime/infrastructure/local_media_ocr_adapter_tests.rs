use super::*;
use crate::contexts::local_media::api::OcrLine;

fn page(number: u32, lines: Vec<OcrLine>) -> OcrPage {
    OcrPage {
        page_number: number,
        text: lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        line_count: lines.len() as u32,
        lines,
    }
}

fn line(text: &str, confidence: Option<f32>) -> OcrLine {
    OcrLine {
        text: text.to_owned(),
        confidence,
        polygon: None,
    }
}

#[test]
fn pages_flatten_into_blocks_numbered_from_one_per_page() {
    let blocks = blocks_from_pages(&[
        page(1, vec![line("alpha", Some(0.9)), line("beta", None)]),
        page(2, vec![line("gamma", Some(0.4))]),
    ]);

    assert_eq!(blocks.len(), 3);
    assert_eq!((blocks[0].page_number, blocks[0].order), (1, 1));
    assert_eq!((blocks[1].page_number, blocks[1].order), (1, 2));
    assert_eq!((blocks[2].page_number, blocks[2].order), (2, 1));
    assert_eq!(blocks[0].confidence, Some(0.9));
    assert_eq!(blocks[1].confidence, None);
}

#[test]
fn geometry_is_carried_across_the_shared_boundary() {
    let positioned = OcrLine {
        text: "alpha".to_owned(),
        confidence: None,
        polygon: Some(vec![(0.0, 0.0), (8.0, 0.0), (8.0, 3.0), (0.0, 3.0)]),
    };
    let blocks = blocks_from_pages(&[page(1, vec![positioned])]);
    let polygon = blocks[0].polygon.as_ref().expect("polygon");
    assert_eq!(polygon.len(), 4);
    assert_eq!((polygon[1].x, polygon[1].y), (8.0, 0.0));
}

#[test]
fn a_page_with_no_lines_contributes_no_blocks() {
    assert!(blocks_from_pages(&[page(1, Vec::new())]).is_empty());
    assert!(blocks_from_pages(&[]).is_empty());
}

#[test]
fn the_tool_input_requires_an_artifact_id_and_languages() {
    let valid = serde_json::json!({"artifact_id": "artifact-1", "languages": ["en"]});
    let parsed = parse_input(&valid).expect("valid input");
    assert_eq!(parsed.artifact_id, "artifact-1");
    assert_eq!(parsed.languages, vec!["en".to_owned()]);
    assert!(!parsed.publish);

    for invalid in [
        serde_json::json!({"languages": ["en"]}),
        serde_json::json!({"artifact_id": "artifact-1"}),
        serde_json::json!({"artifact_id": 7, "languages": ["en"]}),
        serde_json::json!({"artifact_id": "artifact-1", "languages": [7]}),
        serde_json::json!("not an object"),
    ] {
        assert_eq!(
            parse_input(&invalid).err(),
            Some(OcrToolError::InvalidInput)
        );
    }
}

#[test]
fn a_host_path_cannot_be_supplied_through_the_tool_schema() {
    // Sharing the composer's runtime must not widen this input. A `path` field is simply not read,
    // so a caller that supplies one gets OCR of the artifact they named -- or nothing.
    let hostile = serde_json::json!({
        "path": "/etc/passwd",
        "source_path": "C:\\Users\\someone\\secrets.png",
        "languages": ["en"],
    });
    assert_eq!(
        parse_input(&hostile).err(),
        Some(OcrToolError::InvalidInput)
    );

    let with_extra = serde_json::json!({
        "artifact_id": "artifact-1",
        "languages": ["en"],
        "path": "/etc/passwd",
    });
    let parsed = parse_input(&with_extra).expect("artifact input");
    assert_eq!(parsed.artifact_id, "artifact-1");
}

#[test]
fn publish_defaults_to_false() {
    let parsed = parse_input(&serde_json::json!({
        "artifact_id": "artifact-1",
        "languages": ["en"]
    }))
    .expect("input");
    assert!(!parsed.publish);

    let explicit = parse_input(&serde_json::json!({
        "artifact_id": "artifact-1",
        "languages": ["en"],
        "publish": true
    }))
    .expect("input");
    assert!(explicit.publish);
}

#[test]
fn cancellation_and_limits_keep_their_own_envelope_status() {
    // A cancelled call must not read as a failure, and a deadline must not read as a crash: the
    // Agent's retry policy branches on these.
    assert_eq!(
        OcrToolError::Cancelled.envelope().status,
        NativeToolResultStatus::Cancelled
    );
    assert_eq!(
        OcrToolError::Limit.envelope().status,
        NativeToolResultStatus::LimitExceeded
    );
    assert_eq!(
        OcrToolError::Admission.envelope().error_code,
        Some(NativeToolErrorCode::IntegrityFailure)
    );
    assert_eq!(
        OcrToolError::InvalidInput.envelope().error_code,
        Some(NativeToolErrorCode::InvalidInput)
    );
    assert_eq!(
        OcrToolError::Execution.envelope().error_code,
        Some(NativeToolErrorCode::ExternalFailure)
    );
}

#[test]
fn an_error_envelope_carries_no_engine_detail() {
    for error in [
        OcrToolError::Execution,
        OcrToolError::Artifact,
        OcrToolError::Protocol,
        OcrToolError::Admission,
    ] {
        let envelope = error.envelope();
        let message = envelope.safe_error.unwrap_or_default();
        assert!(!message.contains('/'), "{message} leaks a path fragment");
        assert!(!message.contains('\\'), "{message} leaks a path fragment");
        assert!(envelope.output.is_none());
        assert!(envelope.metadata.is_empty());
    }
}
