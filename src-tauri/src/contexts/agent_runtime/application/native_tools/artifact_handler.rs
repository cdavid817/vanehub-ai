use super::{
    ArtifactPort, CanonicalToolResource, NativeToolDefinition, NativeToolExecutionContext,
    NativeToolHandler, NativeToolHandlerError, NativeToolLimitProfile, NativeToolOperation,
    NativeToolPermissionRequest, NativeToolPortRequest, NativeToolReadinessReasonCode,
    NativeToolResultEnvelope, ToolEligibility, ToolEligibilityContext, ToolResourceKind,
    ValidatedNativeToolInput, NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::permissions::api::{Action, Resource};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(crate) struct ArtifactNativeToolHandler {
    definition: NativeToolDefinition,
    port: Arc<dyn ArtifactPort>,
}

impl ArtifactNativeToolHandler {
    pub(crate) fn new(port: Arc<dyn ArtifactPort>) -> Self {
        Self {
            definition: definition(),
            port,
        }
    }
}

impl NativeToolHandler for ArtifactNativeToolHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility {
        match context.readiness.get("artifact") {
            Some(None) => ToolEligibility::Eligible,
            Some(Some(reason)) => ToolEligibility::Ineligible { reason: *reason },
            None => ToolEligibility::Ineligible {
                reason: NativeToolReadinessReasonCode::BackendUnavailable,
            },
        }
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        let object = input.as_object().ok_or_else(invalid_input)?;
        let operation_name = string_field(object, "operation")?;
        let (operation, allowed) = match operation_name {
            "list" => (
                NativeToolOperation::ArtifactRead,
                ["operation", "limit"].as_slice(),
            ),
            "metadata" => (
                NativeToolOperation::ArtifactRead,
                ["operation", "artifact_id"].as_slice(),
            ),
            "read_text" => (
                NativeToolOperation::ArtifactRead,
                ["operation", "artifact_id", "offset", "limit"].as_slice(),
            ),
            "publish" => (
                NativeToolOperation::ArtifactPublish,
                ["operation", "artifact_id", "visibility"].as_slice(),
            ),
            _ => return Err(invalid_input()),
        };
        reject_unknown_fields(object, allowed)?;
        if operation_name != "list" {
            validate_artifact_id(string_field(object, "artifact_id")?)?;
        }
        if operation_name == "publish"
            && !matches!(string_field(object, "visibility")?, "private" | "session")
        {
            return Err(invalid_input());
        }
        validate_optional_u64(object, "offset", 0, u64::MAX)?;
        validate_optional_u64(object, "limit", 1, 65_536)?;
        let artifact_id = object
            .get("artifact_id")
            .and_then(Value::as_str)
            .unwrap_or("collection");
        let canonical_id = format!("artifact/{artifact_id}");
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash: format!(
                "sha256:{}",
                hex_digest(&serde_json::to_vec(input).unwrap_or_default())
            ),
            operation,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::Artifact,
                canonical_id,
                attributes: BTreeMap::new(),
            },
        })
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        _context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        NativeToolPermissionRequest {
            action: Action::new(input.operation.as_str()),
            resource: Resource::new(input.resource.canonical_id.clone()),
            operation: input.operation,
            canonical_resource: input.resource.clone(),
            input_hash: input.input_hash.clone(),
        }
    }

    fn execute(
        &self,
        input: ValidatedNativeToolInput,
        context: NativeToolExecutionContext,
    ) -> NativeToolResultEnvelope {
        self.port
            .execute_artifact(NativeToolPortRequest { input, context })
    }
}

fn definition() -> NativeToolDefinition {
    NativeToolDefinition {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        name: "artifact".to_owned(),
        description: "Inspect, read, or publish immutable VaneHub Artifacts.".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "operation": {"type": "string", "enum": ["list", "metadata", "read_text", "publish"]},
                "artifact_id": {"type": "string", "pattern": "^artifact-[A-Za-z0-9-]+$", "maxLength": 64},
                "offset": {"type": "integer", "minimum": 0},
                "limit": {"type": "integer", "minimum": 1, "maximum": 65536},
                "visibility": {"type": "string", "enum": ["private", "session"]}
            },
            "required": ["operation"],
            "additionalProperties": false
        }),
        operations: vec![
            NativeToolOperation::ArtifactRead,
            NativeToolOperation::ArtifactPublish,
        ],
        plan_mode_compatible: true,
        limit_profile: NativeToolLimitProfile::bounded(16_384, 131_072, 30_000, 50),
    }
}

fn string_field<'a>(
    object: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, NativeToolHandlerError> {
    object
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(invalid_input)
}

fn validate_artifact_id(value: &str) -> Result<(), NativeToolHandlerError> {
    if !value.starts_with("artifact-")
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(invalid_input());
    }
    Ok(())
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

fn validate_optional_u64(
    object: &Map<String, Value>,
    name: &str,
    minimum: u64,
    maximum: u64,
) -> Result<(), NativeToolHandlerError> {
    if let Some(value) = object.get(name) {
        let value = value.as_u64().ok_or_else(invalid_input)?;
        if !(minimum..=maximum).contains(&value) {
            return Err(invalid_input());
        }
    }
    Ok(())
}

fn invalid_input() -> NativeToolHandlerError {
    NativeToolHandlerError::new(
        super::NativeToolErrorCode::InvalidInput,
        "The Artifact tool input is invalid.",
    )
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
#[path = "artifact_handler_tests.rs"]
mod tests;
