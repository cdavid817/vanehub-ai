use super::{OcrExecutionError, PaddleOcrEngineBlock, PaddleOcrEngineResult, PaddleOcrPoint};
use serde::Serialize;
use std::collections::BTreeSet;

pub(crate) const OCR_RESULT_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OcrResultBlock {
    pub(crate) page_number: u32,
    pub(crate) order: u32,
    pub(crate) text: String,
    pub(crate) polygon: Option<Vec<PaddleOcrPoint>>,
    pub(crate) confidence: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NormalizedOcrResult {
    pub(crate) contract_version: u16,
    pub(crate) operation_id: String,
    pub(crate) source_artifact_id: String,
    pub(crate) source_content_hash: String,
    pub(crate) engine_name: String,
    pub(crate) engine_version: String,
    pub(crate) languages: Vec<String>,
    pub(crate) pages: Vec<u32>,
    pub(crate) blocks: Vec<OcrResultBlock>,
    pub(crate) text: String,
    pub(crate) truncated: bool,
    pub(crate) duration_ms: u64,
    pub(crate) warnings: Vec<String>,
}

pub(crate) fn normalize_ocr_result(
    operation_id: &str,
    artifact_id: &str,
    content_hash: &str,
    languages: Vec<String>,
    duration_ms: u64,
    engine: PaddleOcrEngineResult,
) -> Result<NormalizedOcrResult, OcrExecutionError> {
    if operation_id.is_empty()
        || artifact_id.is_empty()
        || content_hash.len() != 64
        || engine.engine_name.is_empty()
        || engine.engine_version.is_empty()
    {
        return Err(OcrExecutionError::ProtocolViolation);
    }
    let mut blocks = engine.blocks;
    blocks.sort_by_key(|block| (block.page_number, block.order));
    if blocks
        .windows(2)
        .any(|pair| (pair[0].page_number, pair[0].order) == (pair[1].page_number, pair[1].order))
    {
        return Err(OcrExecutionError::ProtocolViolation);
    }
    let pages = blocks
        .iter()
        .map(|block| block.page_number)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let text = blocks
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(NormalizedOcrResult {
        contract_version: OCR_RESULT_CONTRACT_VERSION,
        operation_id: operation_id.to_owned(),
        source_artifact_id: artifact_id.to_owned(),
        source_content_hash: content_hash.to_owned(),
        engine_name: engine.engine_name,
        engine_version: engine.engine_version,
        languages,
        pages,
        blocks: blocks.into_iter().map(normalize_block).collect(),
        text,
        truncated: engine.truncated,
        duration_ms,
        warnings: engine.warnings,
    })
}

fn normalize_block(block: PaddleOcrEngineBlock) -> OcrResultBlock {
    OcrResultBlock {
        page_number: block.page_number,
        order: block.order,
        text: block.text,
        polygon: block.polygon,
        confidence: block.confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::extensions::application::{
        PaddleOcrEngineBlock, PADDLEOCR_INFERENCE_PROTOCOL_VERSION,
    };

    #[test]
    fn normalization_orders_blocks_and_preserves_unknown_confidence() {
        let engine = PaddleOcrEngineResult {
            protocol_version: PADDLEOCR_INFERENCE_PROTOCOL_VERSION.to_owned(),
            engine_name: "paddleocr".to_owned(),
            engine_version: "3.2.0".to_owned(),
            blocks: vec![
                PaddleOcrEngineBlock {
                    page_number: 2,
                    order: 1,
                    text: "later".to_owned(),
                    polygon: None,
                    confidence: None,
                },
                PaddleOcrEngineBlock {
                    page_number: 1,
                    order: 1,
                    text: "first".to_owned(),
                    polygon: None,
                    confidence: Some(0.9),
                },
            ],
            warnings: Vec::new(),
            truncated: false,
        };
        let result = normalize_ocr_result(
            "operation",
            "artifact",
            &"a".repeat(64),
            vec!["en".to_owned()],
            12,
            engine,
        )
        .expect("result");
        assert_eq!(result.text, "first\nlater");
        assert_eq!(result.pages, vec![1, 2]);
        assert_eq!(result.blocks[1].confidence, None);
    }

    #[test]
    fn normalization_preserves_an_empty_ocr_result() {
        let engine = PaddleOcrEngineResult {
            protocol_version: PADDLEOCR_INFERENCE_PROTOCOL_VERSION.to_owned(),
            engine_name: "paddleocr".to_owned(),
            engine_version: "3.2.0".to_owned(),
            blocks: Vec::new(),
            warnings: vec!["no_text_detected".to_owned()],
            truncated: false,
        };
        let result = normalize_ocr_result(
            "operation",
            "artifact",
            &"a".repeat(64),
            vec!["en".to_owned()],
            1,
            engine,
        )
        .expect("empty result");
        assert!(result.text.is_empty());
        assert!(result.blocks.is_empty());
        assert!(result.pages.is_empty());
        assert_eq!(result.warnings, vec!["no_text_detected"]);
    }
}
