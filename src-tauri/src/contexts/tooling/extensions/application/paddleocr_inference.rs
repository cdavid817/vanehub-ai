use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) const PADDLEOCR_INFERENCE_PROTOCOL_VERSION: &str = "vanehub.paddleocr.inference.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PaddleOcrInputKind {
    Image,
    RenderedPdfPage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PaddleOcrInferenceInput {
    pub(crate) staged_path: PathBuf,
    pub(crate) source_sha256: String,
    pub(crate) kind: PaddleOcrInputKind,
    pub(crate) page_number: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PaddleOcrInferenceLimits {
    pub(crate) duration_ms: u64,
    pub(crate) input_bytes: u64,
    pub(crate) pixels: u64,
    pub(crate) output_blocks: u32,
    pub(crate) output_characters: u64,
}

impl PaddleOcrInferenceLimits {
    pub(crate) const HARD_CEILING: Self = Self {
        duration_ms: 60_000,
        input_bytes: 64 * 1024 * 1024,
        pixels: 80_000_000,
        output_blocks: 10_000,
        output_characters: 2_000_000,
    };

    fn valid(self) -> bool {
        self.duration_ms > 0
            && self.input_bytes > 0
            && self.pixels > 0
            && self.output_blocks > 0
            && self.output_characters > 0
            && self.duration_ms <= Self::HARD_CEILING.duration_ms
            && self.input_bytes <= Self::HARD_CEILING.input_bytes
            && self.pixels <= Self::HARD_CEILING.pixels
            && self.output_blocks <= Self::HARD_CEILING.output_blocks
            && self.output_characters <= Self::HARD_CEILING.output_characters
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PaddleOcrInferenceRequest {
    pub(crate) protocol_version: String,
    pub(crate) operation_id: String,
    pub(crate) inputs: Vec<PaddleOcrInferenceInput>,
    pub(crate) languages: Vec<String>,
    pub(crate) limits: PaddleOcrInferenceLimits,
}

impl PaddleOcrInferenceRequest {
    pub(crate) fn validate(&self) -> Result<(), PaddleOcrProtocolError> {
        if self.protocol_version != PADDLEOCR_INFERENCE_PROTOCOL_VERSION {
            return Err(PaddleOcrProtocolError::VersionMismatch);
        }
        if !safe_token(&self.operation_id, 128)
            || self.inputs.is_empty()
            || self.inputs.len() > 32
            || self.languages.is_empty()
            || self.languages.len() > 8
            || self
                .languages
                .iter()
                .any(|language| !safe_token(language, 32))
            || !self.limits.valid()
        {
            return Err(PaddleOcrProtocolError::InvalidRequest);
        }
        if self.inputs.iter().any(|input| {
            !input.staged_path.is_absolute()
                || !sha256(&input.source_sha256)
                || match input.kind {
                    PaddleOcrInputKind::Image => input.page_number.is_some(),
                    PaddleOcrInputKind::RenderedPdfPage => input.page_number.is_none(),
                }
        }) {
            return Err(PaddleOcrProtocolError::InvalidInput);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PaddleOcrPoint {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PaddleOcrEngineBlock {
    pub(crate) page_number: u32,
    pub(crate) order: u32,
    pub(crate) text: String,
    pub(crate) polygon: Option<Vec<PaddleOcrPoint>>,
    pub(crate) confidence: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PaddleOcrEngineResult {
    pub(crate) protocol_version: String,
    pub(crate) engine_name: String,
    pub(crate) engine_version: String,
    pub(crate) blocks: Vec<PaddleOcrEngineBlock>,
    pub(crate) warnings: Vec<String>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PaddleOcrProtocolError {
    VersionMismatch,
    InvalidRequest,
    InvalidInput,
}

fn safe_token(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> PaddleOcrInferenceRequest {
        PaddleOcrInferenceRequest {
            protocol_version: PADDLEOCR_INFERENCE_PROTOCOL_VERSION.to_owned(),
            operation_id: "ocr-operation-1".to_owned(),
            inputs: vec![PaddleOcrInferenceInput {
                staged_path: PathBuf::from("C:\\owned\\input.png"),
                source_sha256: "a".repeat(64),
                kind: PaddleOcrInputKind::Image,
                page_number: None,
            }],
            languages: vec!["ch".to_owned(), "en".to_owned()],
            limits: PaddleOcrInferenceLimits::HARD_CEILING,
        }
    }

    #[test]
    fn versioned_request_accepts_only_owned_bounded_inference_material() {
        assert_eq!(request().validate(), Ok(()));
        let mut wrong_version = request();
        wrong_version.protocol_version = "vanehub.paddleocr.inference.v2".to_owned();
        assert_eq!(
            wrong_version.validate(),
            Err(PaddleOcrProtocolError::VersionMismatch)
        );
        let mut relative = request();
        relative.inputs[0].staged_path = PathBuf::from("provider/path.png");
        assert_eq!(
            relative.validate(),
            Err(PaddleOcrProtocolError::InvalidInput)
        );
    }

    #[test]
    fn wire_contract_rejects_provider_selected_execution_fields() {
        let mut value = serde_json::to_value(request()).expect("request json");
        value["executable"] = serde_json::json!("python.exe");
        assert!(serde_json::from_value::<PaddleOcrInferenceRequest>(value).is_err());
        let mut raised = request();
        raised.limits.output_blocks += 1;
        assert_eq!(
            raised.validate(),
            Err(PaddleOcrProtocolError::InvalidRequest)
        );
    }
}
