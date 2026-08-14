use super::governance::NATIVE_TOOL_LIMIT_PROFILE_VERSION;
use super::{
    CanonicalToolResource, NativeToolDefinition, NativeToolExecutionContext, NativeToolHandler,
    NativeToolHandlerError, NativeToolLimitProfile, NativeToolOperation,
    NativeToolPermissionRequest, NativeToolPortRequest, NativeToolReadinessReasonCode,
    NativeToolResultEnvelope, OcrInferencePort, ToolEligibility, ToolEligibilityContext,
    ToolResourceKind, ValidatedNativeToolInput, NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::permissions::api::{Action, Resource};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(crate) struct OcrNativeToolHandler {
    definition: NativeToolDefinition,
    port: Arc<dyn OcrInferencePort>,
}

impl OcrNativeToolHandler {
    pub(crate) fn new(port: Arc<dyn OcrInferencePort>) -> Self {
        Self {
            definition: definition(),
            port,
        }
    }
}

impl NativeToolHandler for OcrNativeToolHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility {
        match context.readiness.get("ocr") {
            Some(None) => ToolEligibility::Eligible,
            Some(Some(reason)) => ToolEligibility::Ineligible { reason: *reason },
            None => ToolEligibility::Ineligible {
                reason: NativeToolReadinessReasonCode::BackendUnavailable,
            },
        }
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        let object = input.as_object().ok_or_else(invalid_input)?;
        reject_unknown(object, &["artifact_id", "pages", "languages", "publish"])?;
        let artifact_id = string(object, "artifact_id", 128)?;
        if !artifact_id.starts_with("artifact-") {
            return Err(invalid_input());
        }
        validate_pages(object)?;
        validate_languages(object)?;
        let publish = object
            .get("publish")
            .map_or(Ok(false), |value| value.as_bool().ok_or_else(invalid_input))?;
        let operation = if publish {
            NativeToolOperation::ArtifactPublish
        } else {
            NativeToolOperation::OcrRead
        };
        let input_hash = hash(input)?;
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash,
            operation,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::Artifact,
                canonical_id: format!("artifact/{artifact_id}/ocr"),
                attributes: BTreeMap::from([
                    ("artifact_id".to_owned(), artifact_id.to_owned()),
                    ("publish".to_owned(), publish.to_string()),
                    ("options_hash".to_owned(), hash(input)?),
                ]),
            },
        })
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        _context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        let publish = input
            .resource
            .attributes
            .get("publish")
            .is_some_and(|v| v == "true");
        let operation = input.operation;
        let canonical = format!(
            "{}#publish={publish}&options={}",
            input.resource.canonical_id,
            input
                .resource
                .attributes
                .get("options_hash")
                .map(String::as_str)
                .unwrap_or("missing")
        );
        NativeToolPermissionRequest {
            action: Action::new(operation.as_str()),
            resource: Resource::new(canonical.clone()),
            operation,
            canonical_resource: CanonicalToolResource {
                kind: ToolResourceKind::Artifact,
                canonical_id: canonical,
                attributes: input.resource.attributes.clone(),
            },
            input_hash: input.input_hash.clone(),
        }
    }

    fn execute(
        &self,
        input: ValidatedNativeToolInput,
        context: NativeToolExecutionContext,
    ) -> NativeToolResultEnvelope {
        self.port
            .execute_ocr(NativeToolPortRequest { input, context })
    }
}

fn definition() -> NativeToolDefinition {
    NativeToolDefinition {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        name: "ocr".to_owned(),
        description: "Extract bounded structured text from an authorized image or PDF Artifact."
            .to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "artifact_id": {"type": "string", "maxLength": 128},
                "pages": {"type": "array", "maxItems": 32, "items": {"type": "integer", "minimum": 1}},
                "languages": {"type": "array", "minItems": 1, "maxItems": 8, "items": {"type": "string", "maxLength": 32}},
                "publish": {"type": "boolean"}
            },
            "required": ["artifact_id", "languages"],
            "additionalProperties": false
        }),
        operations: vec![
            NativeToolOperation::OcrRead,
            NativeToolOperation::ArtifactPublish,
        ],
        plan_mode_compatible: true,
        limit_profile: NativeToolLimitProfile {
            version: NATIVE_TOOL_LIMIT_PROFILE_VERSION,
            max_input_bytes: 8 * 1024,
            max_output_bytes: 2 * 1024 * 1024,
            max_duration_ms: 60_000,
            max_progress_events: 200,
            max_child_processes: 2,
            max_memory_bytes: Some(1024 * 1024 * 1024),
            max_disk_bytes: Some(128 * 1024 * 1024),
        },
    }
}

fn validate_pages(object: &Map<String, Value>) -> Result<(), NativeToolHandlerError> {
    let Some(pages) = object.get("pages") else {
        return Ok(());
    };
    let pages = pages.as_array().ok_or_else(invalid_input)?;
    if pages.len() > 32
        || pages.iter().any(|page| {
            page.as_u64()
                .is_none_or(|page| page == 0 || page > u64::from(u32::MAX))
        })
        || pages
            .windows(2)
            .any(|pair| pair[0].as_u64() >= pair[1].as_u64())
    {
        return Err(invalid_input());
    }
    Ok(())
}

fn validate_languages(object: &Map<String, Value>) -> Result<(), NativeToolHandlerError> {
    let languages = object
        .get("languages")
        .and_then(Value::as_array)
        .ok_or_else(invalid_input)?;
    if languages.is_empty()
        || languages.len() > 8
        || languages.iter().any(|value| {
            value.as_str().is_none_or(|value| {
                value.is_empty()
                    || value.len() > 32
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            })
        })
    {
        return Err(invalid_input());
    }
    Ok(())
}

fn string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    max: usize,
) -> Result<&'a str, NativeToolHandlerError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= max && !value.contains('\0'))
        .ok_or_else(invalid_input)
}

fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), NativeToolHandlerError> {
    let allowed = allowed.iter().copied().collect::<BTreeSet<_>>();
    if object.keys().any(|key| !allowed.contains(key.as_str())) {
        Err(invalid_input())
    } else {
        Ok(())
    }
}

fn hash(value: &Value) -> Result<String, NativeToolHandlerError> {
    let bytes = serde_json::to_vec(value).map_err(|_| invalid_input())?;
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(71);
    encoded.push_str("sha256:");
    for byte in digest {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(encoded)
}

fn invalid_input() -> NativeToolHandlerError {
    NativeToolHandlerError::new(
        super::NativeToolErrorCode::InvalidInput,
        "The OCR input is invalid.",
    )
}

#[cfg(test)]
#[path = "ocr_handler_tests.rs"]
mod tests;
