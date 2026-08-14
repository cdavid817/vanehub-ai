use super::governance::NATIVE_TOOL_LIMIT_PROFILE_VERSION;
use super::{
    CanonicalToolResource, CodeExecutionPort, NativeToolDefinition, NativeToolExecutionContext,
    NativeToolExecutionMode, NativeToolHandler, NativeToolHandlerError, NativeToolLimitProfile,
    NativeToolOperation, NativeToolPermissionRequest, NativeToolPortRequest,
    NativeToolReadinessReasonCode, NativeToolResultEnvelope, ToolEligibility,
    ToolEligibilityContext, ToolResourceKind, ValidatedNativeToolInput,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::permissions::api::{Action, Resource};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(crate) struct CodeExecutionNativeToolHandler {
    definition: NativeToolDefinition,
    port: Arc<dyn CodeExecutionPort>,
}

impl CodeExecutionNativeToolHandler {
    pub(crate) fn new(port: Arc<dyn CodeExecutionPort>) -> Self {
        Self {
            definition: definition(),
            port,
        }
    }
}

impl NativeToolHandler for CodeExecutionNativeToolHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility {
        if context.execution_mode == NativeToolExecutionMode::Plan {
            return ineligible(NativeToolReadinessReasonCode::PolicyUnavailable);
        }
        match context.readiness.get("code_execution") {
            Some(None) => ToolEligibility::Eligible,
            Some(Some(reason)) => ineligible(*reason),
            None => ineligible(NativeToolReadinessReasonCode::BackendUnavailable),
        }
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        let object = input.as_object().ok_or_else(invalid_input)?;
        reject_unknown_fields(
            object,
            &["runtime", "source", "arguments", "inputs", "limits"],
        )?;
        let runtime = required_string(object, "runtime", 16)?;
        if !matches!(runtime, "python" | "javascript") {
            return Err(invalid_input());
        }
        let source = required_string(object, "source", 128 * 1024)?;
        validate_arguments(object)?;
        validate_inputs(object)?;
        validate_limits(object)?;
        let source_hash = digest(source.as_bytes());
        let input_hash = hash_json(input)?;
        let input_artifact_hash = hash_json(object.get("inputs").unwrap_or(&Value::Null))?;
        let limits_hash = hash_json(object.get("limits").unwrap_or(&Value::Null))?;
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash,
            operation: NativeToolOperation::CodeExecute,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::Runtime,
                canonical_id: format!("code-runtime/{runtime}/source/{source_hash}"),
                attributes: BTreeMap::from([
                    ("runtime".to_owned(), runtime.to_owned()),
                    ("source_hash".to_owned(), source_hash),
                    ("input_artifact_hash".to_owned(), input_artifact_hash),
                    ("limits_hash".to_owned(), limits_hash),
                ]),
            },
        })
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        _context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        let attributes = &input.resource.attributes;
        let resource = format!(
            "{}#inputs={}&limits={}",
            input.resource.canonical_id,
            attributes
                .get("input_artifact_hash")
                .map(String::as_str)
                .unwrap_or("missing"),
            attributes
                .get("limits_hash")
                .map(String::as_str)
                .unwrap_or("missing")
        );
        NativeToolPermissionRequest {
            action: Action::new(NativeToolOperation::CodeExecute.as_str()),
            resource: Resource::new(resource.clone()),
            operation: NativeToolOperation::CodeExecute,
            canonical_resource: CanonicalToolResource {
                kind: ToolResourceKind::Runtime,
                canonical_id: resource,
                attributes: attributes.clone(),
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
            .execute_code(NativeToolPortRequest { input, context })
    }
}

fn definition() -> NativeToolDefinition {
    NativeToolDefinition {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        name: "code_execution".to_owned(),
        description: "Execute Python or JavaScript in a disposable, network-denied sandbox."
            .to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "runtime": {"type": "string", "enum": ["python", "javascript"]},
                "source": {"type": "string", "minLength": 1, "maxLength": 131072},
                "arguments": {"type": "array", "maxItems": 16, "items": {"type": "string", "maxLength": 256}},
                "inputs": {"type": "array", "maxItems": 16, "items": {
                    "type": "object",
                    "properties": {
                        "artifact_id": {"type": "string", "maxLength": 128},
                        "content_hash": {"type": "string", "minLength": 64, "maxLength": 64}
                    },
                    "required": ["artifact_id", "content_hash"],
                    "additionalProperties": false
                }},
                "limits": {"type": "object"}
            },
            "required": ["runtime", "source"],
            "additionalProperties": false
        }),
        operations: vec![NativeToolOperation::CodeExecute],
        plan_mode_compatible: false,
        limit_profile: NativeToolLimitProfile {
            version: NATIVE_TOOL_LIMIT_PROFILE_VERSION,
            max_input_bytes: 192 * 1024,
            max_output_bytes: 2 * 1024 * 1024,
            max_duration_ms: 35_000,
            max_progress_events: 200,
            max_child_processes: 2,
            max_memory_bytes: Some(256 * 1024 * 1024),
            max_disk_bytes: Some(64 * 1024 * 1024),
        },
    }
}

fn validate_arguments(object: &Map<String, Value>) -> Result<(), NativeToolHandlerError> {
    let Some(arguments) = object.get("arguments") else {
        return Ok(());
    };
    let arguments = arguments.as_array().ok_or_else(invalid_input)?;
    if arguments.len() > 16
        || arguments.iter().any(|value| {
            value
                .as_str()
                .is_none_or(|value| value.is_empty() || value.len() > 256 || value.contains('\0'))
        })
    {
        return Err(invalid_input());
    }
    Ok(())
}

fn validate_inputs(object: &Map<String, Value>) -> Result<(), NativeToolHandlerError> {
    let Some(inputs) = object.get("inputs") else {
        return Ok(());
    };
    let inputs = inputs.as_array().ok_or_else(invalid_input)?;
    if inputs.len() > 16 {
        return Err(invalid_input());
    }
    for input in inputs {
        let input = input.as_object().ok_or_else(invalid_input)?;
        reject_unknown_fields(input, &["artifact_id", "content_hash"])?;
        let id = required_string(input, "artifact_id", 128)?;
        let hash = required_string(input, "content_hash", 64)?;
        if !id.starts_with("artifact-")
            || hash.len() != 64
            || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(invalid_input());
        }
    }
    Ok(())
}

fn validate_limits(object: &Map<String, Value>) -> Result<(), NativeToolHandlerError> {
    let Some(limits) = object.get("limits") else {
        return Ok(());
    };
    let limits = limits.as_object().ok_or_else(invalid_input)?;
    let ceilings = BTreeMap::from([
        ("wall_time_ms", 30_000_u64),
        ("cpu_time_ms", 20_000),
        ("memory_bytes", 256 * 1024 * 1024),
        ("process_count", 2),
        ("stdout_bytes", 1024 * 1024),
        ("stderr_bytes", 1024 * 1024),
        ("filesystem_bytes", 64 * 1024 * 1024),
        ("file_count", 64),
        ("event_count", 200),
    ]);
    if limits
        .keys()
        .any(|key| !ceilings.contains_key(key.as_str()))
        || limits.iter().any(|(key, value)| {
            value
                .as_u64()
                .is_none_or(|value| value == 0 || value > ceilings[key.as_str()])
        })
    {
        return Err(invalid_input());
    }
    Ok(())
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    max_bytes: usize,
) -> Result<&'a str, NativeToolHandlerError> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(invalid_input)?;
    if value.is_empty() || value.len() > max_bytes || value.contains('\0') {
        return Err(invalid_input());
    }
    Ok(value)
}

fn reject_unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), NativeToolHandlerError> {
    let allowed = allowed.iter().copied().collect::<BTreeSet<_>>();
    if object.keys().any(|key| !allowed.contains(key.as_str())) {
        return Err(invalid_input());
    }
    Ok(())
}

fn hash_json(value: &Value) -> Result<String, NativeToolHandlerError> {
    serde_json::to_vec(value)
        .map(|bytes| format!("sha256:{}", digest(&bytes)))
        .map_err(|_| invalid_input())
}

fn digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn ineligible(reason: NativeToolReadinessReasonCode) -> ToolEligibility {
    ToolEligibility::Ineligible { reason }
}

fn invalid_input() -> NativeToolHandlerError {
    NativeToolHandlerError::new(
        super::NativeToolErrorCode::InvalidInput,
        "The code execution input is invalid.",
    )
}

#[cfg(test)]
#[path = "code_execution_handler_tests.rs"]
mod tests;
