use super::*;

fn identity() -> OcrResultIdentity<'static> {
    OcrResultIdentity {
        operation_id: "operation",
        artifact_id: "artifact-1",
        content_hash: HASH,
        engine_name: "paddleocr",
        engine_version: "3.2.0",
        languages: vec!["en".to_owned()],
        duration_ms: 12,
        truncated: false,
        warnings: Vec::new(),
    }
}

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn block(page: u32, order: u32, text: &str, confidence: Option<f32>) -> OcrResultBlock {
    OcrResultBlock {
        page_number: page,
        order,
        text: text.to_owned(),
        polygon: None,
        confidence,
    }
}

#[test]
fn normalization_orders_blocks_and_preserves_unknown_confidence() {
    let result = normalize_ocr_result(
        identity(),
        vec![block(2, 1, "later", None), block(1, 1, "first", Some(0.9))],
    )
    .expect("result");

    assert_eq!(result.text, "first\nlater");
    assert_eq!(result.pages, vec![1, 2]);
    assert_eq!(result.blocks[1].confidence, None);
    assert_eq!(result.contract_version, 1);
}

#[test]
fn normalization_preserves_an_empty_ocr_result() {
    let mut fields = identity();
    fields.warnings = vec!["no_text_detected".to_owned()];
    let result = normalize_ocr_result(fields, Vec::new()).expect("empty result");

    assert!(result.text.is_empty());
    assert!(result.blocks.is_empty());
    assert!(result.pages.is_empty());
    assert_eq!(result.warnings, vec!["no_text_detected"]);
}

#[test]
fn duplicate_block_positions_are_rejected() {
    // Two blocks claiming the same slot have no deterministic reading order, so the plain-text
    // projection would depend on sort stability rather than on the document.
    assert!(normalize_ocr_result(
        identity(),
        vec![block(1, 1, "a", None), block(1, 1, "b", None)]
    )
    .is_none());
}

#[test]
fn a_malformed_identity_is_rejected() {
    for broken in [
        OcrResultIdentity {
            operation_id: "",
            ..identity()
        },
        OcrResultIdentity {
            artifact_id: "",
            ..identity()
        },
        OcrResultIdentity {
            content_hash: "short",
            ..identity()
        },
        OcrResultIdentity {
            engine_name: "",
            ..identity()
        },
        OcrResultIdentity {
            engine_version: "",
            ..identity()
        },
    ] {
        assert!(normalize_ocr_result(broken, Vec::new()).is_none());
    }
}

#[test]
fn the_serialized_shape_is_unchanged_from_the_previous_owner() {
    // A shared runtime is an implementation change. An Agent reading this tool's output must not
    // be able to tell that PaddleOCR moved contexts.
    let result =
        normalize_ocr_result(identity(), vec![block(1, 1, "alpha", Some(0.5))]).expect("result");
    let json = serde_json::to_value(&result).expect("serialize");
    let keys: std::collections::BTreeSet<&str> = json
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "blocks",
            "contractVersion",
            "durationMs",
            "engineName",
            "engineVersion",
            "languages",
            "operationId",
            "pages",
            "sourceArtifactId",
            "sourceContentHash",
            "text",
            "truncated",
            "warnings",
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(json["blocks"][0]["pageNumber"], 1);
    assert_eq!(json["blocks"][0]["confidence"], 0.5);
}

#[test]
fn geometry_survives_when_the_engine_reported_it() {
    let mut positioned = block(1, 1, "alpha", None);
    positioned.polygon = Some(vec![
        OcrResultPoint { x: 0.0, y: 0.0 },
        OcrResultPoint { x: 10.0, y: 0.0 },
        OcrResultPoint { x: 10.0, y: 4.0 },
        OcrResultPoint { x: 0.0, y: 4.0 },
    ]);
    let result = normalize_ocr_result(identity(), vec![positioned]).expect("result");
    let json = serde_json::to_value(&result).expect("serialize");
    assert_eq!(json["blocks"][0]["polygon"][2]["x"], 10.0);
    assert_eq!(json["blocks"][0]["polygon"][2]["y"], 4.0);
}
