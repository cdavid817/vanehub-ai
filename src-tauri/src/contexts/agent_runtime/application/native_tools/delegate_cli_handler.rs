#![allow(dead_code)]

use super::{
    CanonicalToolResource, CliDelegationPort, NativeToolDefinition, NativeToolExecutionContext,
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

pub(crate) struct DelegateCliNativeToolHandler {
    definition: NativeToolDefinition,
    port: Arc<dyn CliDelegationPort>,
}

impl DelegateCliNativeToolHandler {
    pub(crate) fn new(port: Arc<dyn CliDelegationPort>) -> Self {
        Self {
            definition: definition(),
            port,
        }
    }
}

impl NativeToolHandler for DelegateCliNativeToolHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility {
        if context.execution_mode == NativeToolExecutionMode::Plan {
            return ineligible(NativeToolReadinessReasonCode::PolicyUnavailable);
        }
        if context.canonical_workspace.is_none() {
            return ineligible(NativeToolReadinessReasonCode::PolicyUnavailable);
        }
        match context.readiness.get("delegate_cli") {
            Some(None) => ToolEligibility::Eligible,
            Some(Some(reason)) => ineligible(*reason),
            None => ineligible(NativeToolReadinessReasonCode::BackendUnavailable),
        }
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        let object = input.as_object().ok_or_else(invalid_input)?;
        reject_unknown_fields(object)?;
        let target = bounded_string(object, "target", 32)?;
        if !matches!(target, "claude_code" | "codex_cli") {
            return Err(invalid_input());
        }
        let mode = bounded_string(object, "mode", 16)?;
        let operation = match mode {
            "analyze" => NativeToolOperation::DelegationAnalyze,
            "edit" => NativeToolOperation::DelegationEdit,
            _ => return Err(invalid_input()),
        };
        let task = bounded_string(object, "task", 32 * 1024)?;
        if task.trim() != task {
            return Err(invalid_input());
        }
        optional_string(object, "context_summary", 64 * 1024)?;
        validate_artifacts(object)?;
        let input_hash = hash_json(input)?;
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash: input_hash.clone(),
            operation,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::DelegationTarget,
                canonical_id: format!("delegation/{target}/{mode}/{input_hash}"),
                attributes: BTreeMap::from([
                    ("target".to_owned(), target.to_owned()),
                    ("mode".to_owned(), mode.to_owned()),
                ]),
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
            .execute_delegation(NativeToolPortRequest { input, context })
    }
}

fn definition() -> NativeToolDefinition {
    NativeToolDefinition {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        name: "delegate_cli".to_owned(),
        description: "Delegate bounded analysis or isolated edits to Claude Code or Codex CLI."
            .to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "target": {"type": "string", "enum": ["claude_code", "codex_cli"]},
                "mode": {"type": "string", "enum": ["analyze", "edit"]},
                "task": {"type": "string", "minLength": 1, "maxLength": 32768},
                "context_summary": {"type": "string", "maxLength": 65536},
                "artifact_ids": {"type": "array", "maxItems": 16, "items": {
                    "type": "string", "pattern": "^artifact-[A-Za-z0-9-]+$", "maxLength": 128
                }}
            },
            "required": ["target", "mode", "task"],
            "additionalProperties": false
        }),
        operations: vec![
            NativeToolOperation::DelegationAnalyze,
            NativeToolOperation::DelegationEdit,
        ],
        plan_mode_compatible: false,
        limit_profile: NativeToolLimitProfile::bounded(
            512 * 1024,
            2 * 1024 * 1024,
            30 * 60 * 1000,
            2_000,
        ),
    }
}

fn reject_unknown_fields(object: &Map<String, Value>) -> Result<(), NativeToolHandlerError> {
    let allowed = ["target", "mode", "task", "context_summary", "artifact_ids"]
        .into_iter()
        .collect::<BTreeSet<_>>();
    if object.keys().any(|key| !allowed.contains(key.as_str())) {
        return Err(invalid_input());
    }
    Ok(())
}

fn bounded_string<'a>(
    object: &'a Map<String, Value>,
    name: &str,
    maximum: usize,
) -> Result<&'a str, NativeToolHandlerError> {
    let value = object
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(invalid_input)?;
    if value.is_empty() || value.len() > maximum || value.contains('\0') {
        return Err(invalid_input());
    }
    Ok(value)
}

fn optional_string(
    object: &Map<String, Value>,
    name: &str,
    maximum: usize,
) -> Result<(), NativeToolHandlerError> {
    if object.contains_key(name) {
        let _ = bounded_string(object, name, maximum)?;
    }
    Ok(())
}

fn validate_artifacts(object: &Map<String, Value>) -> Result<(), NativeToolHandlerError> {
    let Some(values) = object.get("artifact_ids") else {
        return Ok(());
    };
    let values = values.as_array().ok_or_else(invalid_input)?;
    if values.len() > 16
        || values.iter().any(|value| {
            value.as_str().is_none_or(|id| {
                !id.starts_with("artifact-")
                    || id.len() > 128
                    || !id
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
        })
    {
        return Err(invalid_input());
    }
    Ok(())
}

fn hash_json(value: &Value) -> Result<String, NativeToolHandlerError> {
    let bytes = serde_json::to_vec(value).map_err(|_| invalid_input())?;
    let encoded = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("sha256:{encoded}"))
}

fn ineligible(reason: NativeToolReadinessReasonCode) -> ToolEligibility {
    ToolEligibility::Ineligible { reason }
}

fn invalid_input() -> NativeToolHandlerError {
    NativeToolHandlerError::new(
        super::NativeToolErrorCode::InvalidInput,
        "The CLI delegation input is invalid.",
    )
}

#[cfg(test)]
#[path = "delegate_cli_handler_tests.rs"]
mod tests;
