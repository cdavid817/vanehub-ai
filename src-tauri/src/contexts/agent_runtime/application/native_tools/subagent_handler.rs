//! `delegate_subagent`: run one bounded child OnePiece attempt with its own context
//! (`add-onepiece-subagents`).
//!
//! The value is context economy, not capability. A search across a codebase costs the main session
//! every file it read on the way to the answer, and that accumulated transcript is what drives
//! compaction -- so exploring degrades the session that needed the answer. A child pays that cost
//! in its own window and returns a paragraph.
//!
//! This delivery is read-only. A child gets exploration tools and nothing that writes, starts a
//! process, reaches the network, or asks the user. Mutating children need an isolated worktree and
//! a sealed ChangeSet, which is a separate change.
//!
//! What is here is the governance half: the tool identity, its OnePiece-only eligibility, its
//! input contract, and its approval classification. The child attempt executor is not, so the
//! handler is registered against the unavailable port and reports `backend_unavailable`. The
//! per-attempt tool-call and per-session concurrency ceilings land with that executor rather than
//! being declared here, because a bound nothing enforces is a claim, not a limit.

use super::governance::NATIVE_TOOL_LIMIT_PROFILE_VERSION;
use super::{
    CanonicalToolResource, NativeToolDefinition, NativeToolExecutionContext,
    NativeToolExecutionMode, NativeToolHandler, NativeToolHandlerError, NativeToolLimitProfile,
    NativeToolOperation, NativeToolPermissionRequest, NativeToolPortRequest,
    NativeToolReadinessReasonCode, NativeToolResultEnvelope, SubagentPort, ToolEligibility,
    ToolEligibilityContext, ToolResourceKind, ValidatedNativeToolInput,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::permissions::api::{Action, Resource};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The task a child is given. Long enough for real context, short enough that the caller has to
/// state a question rather than forward its whole transcript.
pub(crate) const MAX_SUBAGENT_TASK_CHARS: usize = 4_000;

/// The child's answer enters the parent's context, so it is the one bound the parent pays for
/// directly.
pub(crate) const MAX_SUBAGENT_RESULT_CHARS: usize = 4_000;

/// Must equal the native-tool execution deadline the dispatcher actually applies. Declaring more
/// than that would advertise time a child never gets; `subagent_declared_duration_matches_the_
/// enforced_deadline` pins them together.
pub(crate) const MAX_SUBAGENT_DURATION_MS: u64 = 120_000;

pub(crate) struct SubagentNativeToolHandler {
    definition: NativeToolDefinition,
    port: Arc<dyn SubagentPort>,
}

impl SubagentNativeToolHandler {
    pub(crate) fn new(port: Arc<dyn SubagentPort>) -> Self {
        Self {
            definition: definition(),
            port,
        }
    }
}

impl NativeToolHandler for SubagentNativeToolHandler {
    fn definition(&self) -> &NativeToolDefinition {
        &self.definition
    }

    fn eligibility(&self, context: &ToolEligibilityContext) -> ToolEligibility {
        // Plan mode is read-only exploration the *parent* performs; spawning a child there would
        // move the same work behind a boundary the user cannot watch, for no gain.
        if context.execution_mode == NativeToolExecutionMode::Plan {
            return ineligible(NativeToolReadinessReasonCode::PolicyUnavailable);
        }
        // A child explores a workspace. Without one there is nothing for it to explore, and its
        // read-only tools would all fail on the first call.
        if context.canonical_workspace.is_none() {
            return ineligible(NativeToolReadinessReasonCode::PolicyUnavailable);
        }
        match context.readiness.get("delegate_subagent") {
            Some(None) => ToolEligibility::Eligible,
            Some(Some(reason)) => ineligible(*reason),
            None => ineligible(NativeToolReadinessReasonCode::BackendUnavailable),
        }
    }

    fn validate(&self, input: &Value) -> Result<ValidatedNativeToolInput, NativeToolHandlerError> {
        let object = input.as_object().ok_or_else(invalid_input)?;
        reject_unknown_fields(object, &["task"])?;
        let task = required_string(object, "task", MAX_SUBAGENT_TASK_CHARS)?;
        let input_hash = hash_json(input)?;
        Ok(ValidatedNativeToolInput {
            value: input.clone(),
            input_hash,
            operation: NativeToolOperation::SubagentDelegate,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::Subagent,
                canonical_id: format!("subagent/task/{}", digest(task.as_bytes())),
                attributes: BTreeMap::from([("task_hash".to_owned(), digest(task.as_bytes()))]),
            },
        })
    }

    fn permission_request(
        &self,
        input: &ValidatedNativeToolInput,
        _context: &ToolEligibilityContext,
    ) -> NativeToolPermissionRequest {
        NativeToolPermissionRequest {
            // Classified as its own delegation start rather than as the child's individual
            // effects: approving the delegation does not widen what the child may do, because the
            // child's pool is read-only regardless.
            action: Action::new("subagent:delegate"),
            resource: Resource::new("subagent"),
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
            .execute_subagent(NativeToolPortRequest { input, context })
    }
}

fn definition() -> NativeToolDefinition {
    NativeToolDefinition {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        name: "delegate_subagent".to_owned(),
        description: format!(
            "Run one bounded read-only investigation in a separate context and get back only its \
             conclusion. Use it when finding the answer would cost far more reading than the \
             answer is worth carrying -- tracing where something is handled across a codebase, \
             surveying how a pattern is used. The child sees your task text and nothing else of \
             this conversation, explores with read-only tools only, and cannot write files, run \
             commands, reach the network, ask you anything, or delegate further. State exactly \
             what you want to know; you get at most {MAX_SUBAGENT_RESULT_CHARS} characters back."
        ),
        input_schema: json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "maxLength": MAX_SUBAGENT_TASK_CHARS,
                    "description": "The question to investigate, stated so it can be answered without further instruction. The child cannot ask you to clarify."
                }
            },
            "required": ["task"],
            "additionalProperties": false
        }),
        operations: vec![NativeToolOperation::SubagentDelegate],
        plan_mode_compatible: false,
        limit_profile: NativeToolLimitProfile {
            version: NATIVE_TOOL_LIMIT_PROFILE_VERSION,
            max_input_bytes: MAX_SUBAGENT_TASK_CHARS as u64 * 4,
            max_output_bytes: MAX_SUBAGENT_RESULT_CHARS as u64 * 4,
            max_duration_ms: MAX_SUBAGENT_DURATION_MS,
            max_progress_events: 64,
            max_child_processes: 0,
            max_memory_bytes: None,
            max_disk_bytes: None,
        },
    }
}

const fn ineligible(reason: NativeToolReadinessReasonCode) -> ToolEligibility {
    ToolEligibility::Ineligible { reason }
}

fn invalid_input() -> NativeToolHandlerError {
    NativeToolHandlerError::new(
        super::NativeToolErrorCode::InvalidInput,
        "delegate_subagent requires a single bounded task string.",
    )
}

fn reject_unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), NativeToolHandlerError> {
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid_input());
    }
    Ok(())
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    max_chars: usize,
) -> Result<&'a str, NativeToolHandlerError> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(invalid_input)?;
    let trimmed = value.trim();
    if trimmed.is_empty() || value.chars().count() > max_chars {
        return Err(invalid_input());
    }
    Ok(value)
}

fn hash_json(value: &Value) -> Result<String, NativeToolHandlerError> {
    let encoded = serde_json::to_vec(value).map_err(|_| invalid_input())?;
    Ok(digest(&encoded))
}

fn digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
#[path = "subagent_handler_tests.rs"]
mod tests;
