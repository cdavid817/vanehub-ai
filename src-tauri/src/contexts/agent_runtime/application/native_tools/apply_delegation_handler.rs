#![allow(dead_code)]

use super::{
    CanonicalToolResource, ChangeSetApplyPort, NativeToolDefinition, NativeToolExecutionContext,
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

pub(crate) struct ApplyDelegationChangesNativeToolHandler {
    definition: NativeToolDefinition,
    port: Arc<dyn ChangeSetApplyPort>,
}

impl ApplyDelegationChangesNativeToolHandler {
    pub(crate) fn new(port: Arc<dyn ChangeSetApplyPort>) -> Self {
        Self {
            definition: definition(),
            port,
        }
    }
}

impl NativeToolHandler for ApplyDelegationChangesNativeToolHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility {
        match context.readiness.get("apply_delegation_changes") {
            Some(None) => ToolEligibility::Eligible,
            Some(Some(reason)) => ToolEligibility::Ineligible { reason: *reason },
            None => ToolEligibility::Ineligible {
                reason: NativeToolReadinessReasonCode::BackendUnavailable,
            },
        }
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        let object = input.as_object().ok_or_else(invalid_input)?;
        reject_unknown_fields(object)?;
        if object.get("acknowledged") != Some(&Value::Bool(true)) {
            return Err(invalid_input());
        }
        let artifact_id = bounded_string(object, "artifact_id", 128)?;
        let content_hash = sha256_string(object, "content_hash")?;
        let diff_hash = sha256_string(object, "diff_hash")?;
        let repository_identity = bounded_string(object, "target_repository_identity", 4096)?;
        let base_commit = bounded_string(object, "base_commit", 64)?;
        if !matches!(base_commit.len(), 40 | 64)
            || !base_commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(invalid_input());
        }
        let canonical_id = format!(
            "changeset/{artifact_id}/{content_hash}/{diff_hash}/{repository_identity}/{base_commit}"
        );
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash: sha256(&serde_json::to_vec(input).map_err(|_| invalid_input())?),
            operation: NativeToolOperation::DelegationApply,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::ChangeSet,
                canonical_id,
                attributes: BTreeMap::from([
                    ("artifact_hash".into(), content_hash.into()),
                    ("diff_hash".into(), diff_hash.into()),
                    ("repository_identity".into(), repository_identity.into()),
                    ("base_commit".into(), base_commit.into()),
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
            action: Action::new(NativeToolOperation::DelegationApply.as_str()),
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
            .execute_change_set_apply(NativeToolPortRequest { input, context })
    }
}

fn definition() -> NativeToolDefinition {
    NativeToolDefinition {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        name: "apply_delegation_changes".into(),
        description: "Apply one complete, reviewed delegated ChangeSet exactly once.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "artifact_id": {"type": "string", "minLength": 1, "maxLength": 128},
                "content_hash": {"type": "string", "pattern": "^sha256:[0-9a-fA-F]{64}$"},
                "diff_hash": {"type": "string", "pattern": "^sha256:[0-9a-fA-F]{64}$"},
                "target_repository_identity": {"type": "string", "minLength": 1, "maxLength": 4096},
                "base_commit": {"type": "string", "pattern": "^(?:[0-9a-fA-F]{40}|[0-9a-fA-F]{64})$"},
                "acknowledged": {"const": true}
            },
            "required": ["artifact_id", "content_hash", "diff_hash", "target_repository_identity", "base_commit", "acknowledged"],
            "additionalProperties": false
        }),
        operations: vec![NativeToolOperation::DelegationApply],
        plan_mode_compatible: false,
        limit_profile: NativeToolLimitProfile::bounded(16_384, 65_536, 300_000, 100),
    }
}

fn bounded_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    maximum: usize,
) -> Result<&'a str, NativeToolHandlerError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty() && value.len() <= maximum)
        .ok_or_else(invalid_input)
}

fn sha256_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, NativeToolHandlerError> {
    let value = bounded_string(object, key, 71)?;
    if value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        Ok(value)
    } else {
        Err(invalid_input())
    }
}

fn reject_unknown_fields(object: &Map<String, Value>) -> Result<(), NativeToolHandlerError> {
    let allowed = BTreeSet::from([
        "artifact_id",
        "content_hash",
        "diff_hash",
        "target_repository_identity",
        "base_commit",
        "acknowledged",
    ]);
    if object.keys().any(|key| !allowed.contains(key.as_str())) {
        return Err(invalid_input());
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    let hex = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

fn invalid_input() -> NativeToolHandlerError {
    NativeToolHandlerError::new(
        super::NativeToolErrorCode::InvalidInput,
        "The delegated ChangeSet application request is invalid.",
    )
}

#[cfg(test)]
#[path = "apply_delegation_handler_tests.rs"]
mod tests;
