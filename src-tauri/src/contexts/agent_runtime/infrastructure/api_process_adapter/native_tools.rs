//! Native tool implementations: skills, registered tools, shell, code intelligence, memory.

use super::super::agent_image::{prepare_image, AgentImage, MAX_IMAGES_PER_REQUEST};
use super::super::code_intelligence_tool_output::{
    diagnostics_outcome, hover_outcome, locations_outcome,
};
use super::super::tools::{
    background_shell_registry, execute_edit, execute_file, execute_glob, execute_grep,
    execute_notebook, execute_shell, is_reviewed_image_path, render_task_list, task_list_store,
    validate_task_list, BackgroundStartError, GrepRequest, KillOutcome, NotebookRequest,
    ToolExecutionOutcome, MAX_BACKGROUND_COMMANDS_PER_SESSION, OUTPUT_MODE_FILES,
};
use super::super::SqliteNativeToolRepository;
use super::interactive::{await_approval, plan_mode_denial, ApprovalOutcome};
use super::{failed_non_retryable, failed_retryable, PendingApprovals, REQUEST_TIMEOUT};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentCodeIntelligenceContext, AgentCodeIntelligencePort,
    AgentCodeRetrievalOutcome, AgentDocumentInput, AgentDocumentPositionInput, AgentLog,
    AgentLogLevel, AgentLoggingPort, AgentMcpToolPort, AgentPermissionPort, AgentProcessEventSink,
    AgentRetrievalOutcome, AgentRetrievalPort, AgentSkillPort, AgentSkillReadRequest,
    AgentWorkspaceMutation, AgentWorkspaceMutationPort, ExistingToolHandler,
    ExistingToolHandlerRegistry, GenerationProcessEvent, GenerationProcessRequest,
    NativeToolAuthorizationStatus, NativeToolDispatchRequest, NativeToolDispatcher,
    NativeToolExecutionContext, NativeToolExecutionMode, NativeToolProgress,
    NativeToolProgressPhase, NativeToolProgressSink, NativeToolRegistry, NativeToolResultEnvelope,
    NativeToolResultStatus, StoredToolOperation, StoredToolOperationStatus, ToolEligibilityContext,
    ToolUseBlock, UtilityDelegationApplicationService, DELEGATE_UTILITY_SKILL_TOOL_NAME,
    FILE_TOOL_NAME, FIND_DEFINITION_TOOL_NAME, FIND_REFERENCES_TOOL_NAME,
    GET_DIAGNOSTICS_TOOL_NAME, GET_HOVER_TOOL_NAME, IMAGE_ARTIFACT_METADATA_KEY,
    LIST_SKILLS_TOOL_NAME, LOAD_SKILL_TOOL_NAME, READ_SKILL_RESOURCE_TOOL_NAME,
};
use crate::contexts::agent_runtime::domain::{UtilityDelegationLimits, UtilityDelegationRequest};
use crate::contexts::artifacts::application::ArtifactService;
use crate::platform::filesystem::BoundedFilesystem;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Emitter;

/// Parses a tool-call argument that should be an absent-or-non-negative integer (`offset`,
/// `limit`, `context`, `head_limit`), accepting a JSON number that arrived as either an integer
/// or an integral float -- some OpenAI-compatible providers serialize every number as a float on
/// the wire, so `100` and `100.0` must parse identically instead of the float silently falling
/// through `Value::as_u64` (which only recognizes the integer encoding) and being reinterpreted
/// as "absent". Returns `Ok(None)` when the field is absent or JSON `null`, which callers must
/// keep distinct from `Ok(Some(0))` for an explicit zero -- `grep`'s `head_limit == Some(0)` and
/// `file`'s `limit == Some(0)` guards reject the latter as degenerate input rather than reading it
/// as "unbounded" (`None`'s meaning). A value that is present but not a non-negative integer
/// (negative, fractional, or non-numeric) is rejected with the same clear-error shape the tool
/// modules themselves already use for degenerate input, instead of silently collapsing into
/// `None` and widening the effective bound.
pub(super) fn parse_optional_non_negative_integer_arg(
    input: &Value,
    field: &str,
) -> Result<Option<usize>, ToolExecutionOutcome> {
    match input.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => match non_negative_integer(value) {
            Some(number) => Ok(Some(number)),
            None => Err(ToolExecutionOutcome {
                output: format!("{field} must be a non-negative integer (received {value})."),
                is_error: true,
            }),
        },
    }
}

/// Reads a JSON number as a non-negative integer regardless of whether it was encoded as an
/// integer (`5`) or an integral float (`5.0`) -- `Value::as_u64` alone only recognizes the
/// former. Negative, fractional, non-finite, and non-numeric values all yield `None`.
fn non_negative_integer(value: &Value) -> Option<usize> {
    if let Some(integer) = value.as_u64() {
        return Some(integer as usize);
    }
    let float = value.as_f64()?;
    (float.is_finite() && float >= 0.0 && float.fract() == 0.0).then_some(float as u64 as usize)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListSkillsInput {
    query: Option<String>,
    #[serde(rename = "type")]
    skill_type: Option<String>,
    delivery: Option<String>,
    availability: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoadSkillInput {
    id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadSkillResourceInput {
    uri: String,
    revision: String,
}

fn invalid_skill_tool_input(name: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: json!({
            "status": "error",
            "error": {
                "code": "invalid-input",
                "message": format!("Invalid input for {name}.")
            }
        })
        .to_string(),
        is_error: true,
    }
}

fn valid_skill_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_skill_resource_uri(value: &str) -> bool {
    if value.len() > 512 || value.contains(['\\', '%']) || value.chars().any(char::is_control) {
        return false;
    }
    let Some(path) = value.strip_prefix("skill://") else {
        return false;
    };
    let mut components = path.split('/');
    let Some(id) = components.next() else {
        return false;
    };
    let Some(directory) = components.next() else {
        return false;
    };
    let resources = components.collect::<Vec<_>>();
    valid_skill_identifier(id)
        && matches!(directory, "scripts" | "references" | "templates" | "assets")
        && !resources.is_empty()
        && resources.iter().all(|component| {
            !component.is_empty()
                && *component != "."
                && *component != ".."
                && !component.starts_with('.')
                && component.chars().count() <= 240
        })
}

fn execute_skill_read(
    name: &str,
    input: &Value,
    workspace_folder: Option<&str>,
    skills: &dyn AgentSkillPort,
) -> ToolExecutionOutcome {
    let request = match name {
        LIST_SKILLS_TOOL_NAME => {
            let Ok(input) = serde_json::from_value::<ListSkillsInput>(input.clone()) else {
                return invalid_skill_tool_input(name);
            };
            let valid = input
                .query
                .as_deref()
                .is_none_or(|query| query.chars().count() <= 80)
                && input.limit.is_none_or(|limit| (1..=100).contains(&limit))
                && input
                    .skill_type
                    .as_deref()
                    .is_none_or(|value| matches!(value, "role" | "utility"))
                && input
                    .delivery
                    .as_deref()
                    .is_none_or(|value| matches!(value, "eager" | "on-demand"))
                && input.availability.as_deref().is_none_or(|value| {
                    matches!(
                        value,
                        "available" | "disabled" | "invalid" | "conflicting" | "unsupported"
                    )
                });
            if !valid {
                return invalid_skill_tool_input(name);
            }
            AgentSkillReadRequest::List {
                workspace_path: workspace_folder.map(str::to_string),
                query: input.query,
                skill_type: input.skill_type,
                delivery: input.delivery,
                availability: input.availability,
                limit: input.limit,
            }
        }
        LOAD_SKILL_TOOL_NAME => {
            let Ok(input) = serde_json::from_value::<LoadSkillInput>(input.clone()) else {
                return invalid_skill_tool_input(name);
            };
            if !valid_skill_identifier(&input.id) {
                return invalid_skill_tool_input(name);
            }
            AgentSkillReadRequest::Load {
                workspace_path: workspace_folder.map(str::to_string),
                id_or_alias: input.id,
            }
        }
        READ_SKILL_RESOURCE_TOOL_NAME => {
            let Ok(input) = serde_json::from_value::<ReadSkillResourceInput>(input.clone()) else {
                return invalid_skill_tool_input(name);
            };
            if !valid_skill_resource_uri(&input.uri)
                || input.revision.is_empty()
                || input.revision.len() > 128
                || input.revision.chars().any(char::is_control)
            {
                return invalid_skill_tool_input(name);
            }
            AgentSkillReadRequest::ReadResource {
                workspace_path: workspace_folder.map(str::to_string),
                uri: input.uri,
                revision: input.revision,
            }
        }
        _ => return invalid_skill_tool_input(name),
    };
    let outcome = skills.execute_read(request);
    ToolExecutionOutcome {
        output: outcome.output,
        is_error: outcome.is_error,
    }
}

struct NativeToolOperationRecorder {
    repository: Option<SqliteNativeToolRepository>,
    events: Option<tauri::AppHandle>,
    record: Mutex<StoredToolOperation>,
}

impl std::fmt::Debug for NativeToolOperationRecorder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeToolOperationRecorder")
            .finish_non_exhaustive()
    }
}

impl NativeToolOperationRecorder {
    fn new(
        repository: Option<&SqliteNativeToolRepository>,
        events: Option<&tauri::AppHandle>,
        request: &GenerationProcessRequest,
        tool_use: &ToolUseBlock,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let recorder = Self {
            repository: repository.cloned(),
            events: events.cloned(),
            record: Mutex::new(StoredToolOperation {
                contract_version: 1,
                id: tool_use.id.clone(),
                session_id: request.session.id.clone(),
                generation_id: request.operation_id.clone(),
                tool_name: tool_use.name.clone(),
                status: StoredToolOperationStatus::Queued,
                progress_sequence: 0,
                progress_message: None,
                result_artifact_ids: Vec::new(),
                error_code: None,
                created_at: now.clone(),
                updated_at: now,
            }),
        };
        recorder.persist();
        recorder
    }

    fn transition(
        &self,
        status: StoredToolOperationStatus,
        message: Option<String>,
        artifact_ids: Vec<String>,
        error_code: Option<String>,
    ) {
        if let Ok(mut record) = self.record.lock() {
            record.progress_sequence = record.progress_sequence.saturating_add(1);
            record.status = status;
            record.progress_message = message;
            record.result_artifact_ids = artifact_ids;
            record.error_code = error_code;
            record.updated_at = chrono::Utc::now().to_rfc3339();
        }
        self.persist();
    }

    fn persist(&self) {
        let Ok(record) = self.record.lock().map(|record| record.clone()) else {
            return;
        };
        if let Some(repository) = &self.repository {
            let _ = repository.save_operation(&record);
        }
        if let Some(events) = &self.events {
            let _ = events.emit("builtin-tool-operation", operation_event(&record));
        }
    }
}

impl NativeToolProgressSink for NativeToolOperationRecorder {
    fn publish(&self, progress: NativeToolProgress) {
        if let Ok(mut record) = self.record.lock() {
            record.progress_sequence = record
                .progress_sequence
                .saturating_add(1)
                .max(progress.sequence.saturating_add(2));
            record.status = if progress.phase == NativeToolProgressPhase::AwaitingHuman {
                StoredToolOperationStatus::AwaitingHuman
            } else {
                StoredToolOperationStatus::Running
            };
            record.progress_message = progress.message;
            record.updated_at = chrono::Utc::now().to_rfc3339();
        }
        self.persist();
    }
}

#[allow(clippy::too_many_arguments, clippy::result_large_err)]
pub(super) fn execute_registered_native_tool(
    tool_use: &mut ToolUseBlock,
    input: &Value,
    request: &GenerationProcessRequest,
    cancelled: Arc<AtomicBool>,
    registry: &NativeToolRegistry,
    operations: Option<&SqliteNativeToolRepository>,
    events: Option<&tauri::AppHandle>,
    permissions: &dyn AgentPermissionPort,
    pending_approvals: &PendingApprovals,
    sink: &dyn AgentProcessEventSink,
    plan_mode: bool,
) -> Result<(ToolExecutionOutcome, Option<String>), GenerationProcessEvent> {
    let recorder = Arc::new(NativeToolOperationRecorder::new(
        operations, events, request, tool_use,
    ));
    let authority = ToolEligibilityContext {
        agent_id: request.agent.id.clone(),
        session_id: request.session.id.clone(),
        generation_id: request.operation_id.clone(),
        canonical_workspace: request.session.folder.as_deref().map(Into::into),
        execution_mode: if plan_mode {
            NativeToolExecutionMode::Plan
        } else {
            NativeToolExecutionMode::Execute
        },
        readiness: registry.readiness_snapshot(),
    };
    let execution = NativeToolExecutionContext {
        call_id: tool_use.id.clone(),
        session_id: authority.session_id.clone(),
        generation_id: authority.generation_id.clone(),
        agent_id: authority.agent_id.clone(),
        canonical_workspace: authority.canonical_workspace.clone(),
        deadline: Instant::now() + REQUEST_TIMEOUT,
        cancelled: cancelled.clone(),
        progress: recorder.clone(),
    };
    let dispatcher = NativeToolDispatcher::new(registry.clone());
    let prepared = match dispatcher.prepare(NativeToolDispatchRequest {
        tool_name: tool_use.name.clone(),
        input: input.clone(),
        authority,
        execution,
    }) {
        Ok(prepared) => prepared,
        Err(error) => {
            recorder.transition(
                StoredToolOperationStatus::Failed,
                None,
                Vec::new(),
                Some(error.code.as_str().to_owned()),
            );
            return Ok((native_dispatch_error(error.safe_message), None));
        }
    };
    let project_key = request.session.folder.as_deref().unwrap_or("");
    let mut witness = match dispatcher.authorize(&prepared, permissions, project_key) {
        Ok(witness) => witness,
        Err(error) => {
            recorder.transition(
                StoredToolOperationStatus::Failed,
                None,
                Vec::new(),
                Some(error.code.as_str().to_owned()),
            );
            return Ok((native_dispatch_error(error.safe_message), None));
        }
    };
    if witness.status == NativeToolAuthorizationStatus::AwaitingApproval {
        recorder.transition(
            StoredToolOperationStatus::AwaitingApproval,
            None,
            Vec::new(),
            None,
        );
        tool_use.status = "awaiting_approval".to_owned();
        if sink
            .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
            .is_err()
        {
            return Err(failed_retryable("Agent generation event handling failed."));
        }
        match await_approval(&tool_use.id, &cancelled, pending_approvals) {
            ApprovalOutcome::Approved => {
                witness.status = NativeToolAuthorizationStatus::Allowed;
            }
            // An answer delivered here means the approval and question resolution paths were
            // crossed; fail closed rather than treat it as consent.
            ApprovalOutcome::Denied | ApprovalOutcome::Answered(_) => {
                recorder.transition(
                    StoredToolOperationStatus::Failed,
                    None,
                    Vec::new(),
                    Some("permission_denied".to_owned()),
                );
                return Ok((native_dispatch_error("Denied by user.".to_owned()), None));
            }
            ApprovalOutcome::Cancelled => {
                recorder.transition(
                    StoredToolOperationStatus::Cancelled,
                    None,
                    Vec::new(),
                    Some("cancelled".to_owned()),
                );
                return Err(failed_non_retryable(
                    "Generation was cancelled while a tool call was awaiting approval.",
                ));
            }
        }
    }
    recorder.transition(StoredToolOperationStatus::Running, None, Vec::new(), None);
    tool_use.status = "running".to_owned();
    if sink
        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
        .is_err()
    {
        return Err(failed_retryable("Agent generation event handling failed."));
    }
    let result = match dispatcher.execute_authorized(prepared, &witness) {
        Ok(result) => result,
        Err(error) => {
            recorder.transition(
                StoredToolOperationStatus::Failed,
                None,
                Vec::new(),
                Some(error.code.as_str().to_owned()),
            );
            return Ok((native_dispatch_error(error.safe_message), None));
        }
    };
    let is_error = result.status != NativeToolResultStatus::Succeeded;
    recorder.transition(
        stored_status(&result),
        None,
        artifact_ids(&result),
        result.error_code.map(|code| code.as_str().to_owned()),
    );
    let image_artifact_id = result
        .metadata
        .get(IMAGE_ARTIFACT_METADATA_KEY)
        .and_then(Value::as_str)
        .map(str::to_owned);
    let output = match (result.output, result.safe_error) {
        (Some(value), _) => serde_json::to_string(&value)
            .unwrap_or_else(|_| "The native tool result could not be encoded.".to_owned()),
        (None, Some(message)) => message,
        (None, None) => "The native tool returned no result.".to_owned(),
    };
    Ok((ToolExecutionOutcome { output, is_error }, image_artifact_id))
}

/// Resolves the Artifact a native tool named as its image and prepares it for the wire.
///
/// Returns `None` whenever the image cannot be attached -- the model does not accept images, the
/// per-request budget is spent, the Artifact cannot be read, or its bytes are not a reviewed image
/// type. Every one of those degrades to the tool's existing non-image result rather than failing
/// the call: a model choice or a budget must never turn a working tool into an error
/// (`add-onepiece-visual-tool-returns`).
pub(super) fn resolve_tool_image(
    artifacts: Option<&ArtifactService>,
    artifact_id: &str,
    images_supported: bool,
    images_in_request: usize,
) -> Option<AgentImage> {
    if !images_supported || images_in_request >= MAX_IMAGES_PER_REQUEST {
        return None;
    }
    let (bytes, media_type) = artifacts?.read_bytes(artifact_id).ok()?;
    prepare_image(&bytes, Some(&media_type)).ok()
}

fn stored_status(result: &NativeToolResultEnvelope) -> StoredToolOperationStatus {
    match result.status {
        NativeToolResultStatus::Succeeded => StoredToolOperationStatus::Succeeded,
        NativeToolResultStatus::Cancelled => StoredToolOperationStatus::Cancelled,
        _ => StoredToolOperationStatus::Failed,
    }
}

pub(super) fn artifact_ids(result: &NativeToolResultEnvelope) -> Vec<String> {
    fn visit(value: &Value, ids: &mut Vec<String>) {
        if ids.len() >= 64 {
            return;
        }
        match value {
            Value::String(value) if value.starts_with("artifact-") => {
                if !ids.contains(value) {
                    ids.push(value.clone());
                }
            }
            Value::Array(values) => values.iter().for_each(|value| visit(value, ids)),
            Value::Object(values) => values.values().for_each(|value| visit(value, ids)),
            _ => {}
        }
    }

    let mut ids = Vec::new();
    if let Some(output) = &result.output {
        visit(output, &mut ids);
    }
    ids
}

pub(super) fn operation_event(record: &StoredToolOperation) -> Value {
    let progress = record.progress_message.as_ref().map(|message| {
        json!({
            "phase": message,
            "completedUnits": record.progress_sequence,
            "totalUnits": Value::Null,
            "messageCode": Value::Null
        })
    });
    json!({
        "kind": "snapshot",
        "operation": {
            "id": record.id,
            "agentId": "onepiece",
            "sessionId": record.session_id,
            "capability": native_tool_capability(&record.tool_name),
            "operation": record.tool_name,
            "status": match record.status {
                StoredToolOperationStatus::AwaitingApproval => "queued",
                other => other.as_str(),
            },
            "progress": progress,
            "artifactIds": record.result_artifact_ids,
            "errorCode": record.error_code,
            "simulated": false,
            "createdAt": record.created_at,
            "updatedAt": record.updated_at
        }
    })
}

fn native_tool_capability(tool_name: &str) -> &'static str {
    match tool_name {
        "browser" => "browser",
        "web_search" | "web_fetch" => "web",
        "code_execution" => "code_execution",
        "ocr" => "ocr",
        "artifact" => "artifact",
        "delegate_cli" | "apply_delegation_changes" => "delegation",
        _ => "filesystem",
    }
}

fn native_dispatch_error(message: String) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: message,
        is_error: true,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_tool_call_with_runtime_ports(
    name: &str,
    input: &Value,
    workspace_folder: Option<&str>,
    cancelled: Arc<AtomicBool>,
    mcp: &dyn AgentMcpToolPort,
    retrieval: &dyn AgentRetrievalPort,
    code_intelligence: &dyn AgentCodeIntelligencePort,
    workspace_mutations: &dyn AgentWorkspaceMutationPort,
    plan_mode: bool,
    skills: &dyn AgentSkillPort,
    utility_delegation: Option<&UtilityDelegationApplicationService>,
    generation: &GenerationProcessRequest,
) -> ToolExecutionOutcome {
    if name == DELEGATE_UTILITY_SKILL_TOOL_NAME {
        return execute_utility_delegation(input, cancelled, utility_delegation, generation);
    }
    execute_tool_call_impl(
        name,
        input,
        workspace_folder,
        cancelled,
        mcp,
        retrieval,
        Some(code_intelligence),
        Some(workspace_mutations),
        plan_mode,
        skills,
        Some(generation.session.id.as_str()),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UtilityDelegationToolInput {
    skill_id: String,
    task: String,
    duration_ms: Option<u64>,
    tool_calls: Option<u32>,
    approvals: Option<u32>,
    result_chars: Option<usize>,
}

fn execute_utility_delegation(
    input: &Value,
    cancelled: Arc<AtomicBool>,
    service: Option<&UtilityDelegationApplicationService>,
    generation: &GenerationProcessRequest,
) -> ToolExecutionOutcome {
    let Some(service) = service else {
        return ToolExecutionOutcome {
            output: json!({"status":"refused","reason":"native-runtime-unavailable"}).to_string(),
            is_error: true,
        };
    };
    let parsed: UtilityDelegationToolInput = match serde_json::from_value(input.clone()) {
        Ok(value) => value,
        Err(_) => {
            return ToolExecutionOutcome {
                output: json!({"status":"refused","reason":"invalid-input"}).to_string(),
                is_error: true,
            }
        }
    };
    let defaults = UtilityDelegationLimits::default();
    let limits = match UtilityDelegationLimits::bounded(
        parsed.duration_ms.unwrap_or(defaults.duration_ms),
        parsed.tool_calls.unwrap_or(defaults.tool_calls),
        parsed.approvals.unwrap_or(defaults.approvals),
        parsed.result_chars.unwrap_or(defaults.result_chars),
    ) {
        Ok(value) => value,
        Err(_) => {
            return ToolExecutionOutcome {
                output: json!({"status":"refused","reason":"invalid-limits"}).to_string(),
                is_error: true,
            }
        }
    };
    let request = UtilityDelegationRequest {
        agent_id: generation.agent.id.clone(),
        skill_id: parsed.skill_id,
        task: parsed.task,
        parent_run_id: generation.execution_context.run_id.as_str().to_string(),
        parent_span_id: generation.execution_context.span_id.as_str().to_string(),
        session_id: generation.session.id.clone(),
        message_id: generation.message_id.clone(),
        canonical_workspace: generation.session.folder.clone(),
        depth: 0,
        limits,
    };
    match service.execute(request, cancelled) {
        Ok(result) => ToolExecutionOutcome {
            output: json!({
                "status": result.terminal.as_str(),
                "delegationId": result.delegation_id,
                "attemptId": result.attempt_id,
                "skillId": result.skill_id,
                "revision": result.revision,
                "summary": result.summary,
                "durationMs": result.duration_ms,
                "toolCount": result.counts.tool_calls,
                "approvalCount": result.counts.approvals,
                "limitReason": result.limit_reason,
            })
            .to_string(),
            is_error: result.terminal
                != crate::contexts::agent_runtime::domain::UtilityDelegationTerminal::Succeeded,
        },
        Err(_) => ToolExecutionOutcome {
            output: json!({"status":"refused","reason":"utility-resolution-failed"}).to_string(),
            is_error: true,
        },
    }
}

#[allow(clippy::too_many_arguments)]
/// The shared refusal for a background-command tool used by a session the runtime could not
/// identify. Background state is keyed by session, so without one there is no safe scope to read
/// or terminate within -- failing closed beats guessing an owner.
fn background_unavailable(reason: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: format!("Background commands are unavailable for this session: {reason}."),
        is_error: true,
    }
}

/// Whether this tool call is a `file` read of a reviewed image type. Both halves matter: the
/// file tool's other operations are unaffected, and a non-image read must not detour through the
/// image path (`add-agent-image-input`).
pub(super) fn is_image_read_request(tool_name: &str, input: &Value) -> bool {
    if tool_name != FILE_TOOL_NAME {
        return false;
    }
    if input.get("operation").and_then(Value::as_str) != Some("read") {
        return false;
    }
    input
        .get("path")
        .and_then(Value::as_str)
        .is_some_and(is_reviewed_image_path)
}

/// Records that an image was attached, carrying its hash, media type, dimensions, and byte count
/// only. The bytes never reach a durable log: a single screenshot base64-encodes to more than the
/// whole log-line budget, so this is a size constraint as much as a privacy one.
pub(super) fn log_image_attachment(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    call_id: &str,
    image: &AgentImage,
) {
    let _ = logging.record(AgentLog {
        level: AgentLogLevel::Debug,
        category: "session.runtime.api.image".to_string(),
        message: format!(
            "Attached image to tool call {call_id}: {} {}x{} {} bytes sha256:{}",
            image.media_type().as_str(),
            image.width(),
            image.height(),
            image.byte_len(),
            image.content_hash()
        ),
        agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        operation_id: Some(request.operation_id.clone()),
        run_id: None,
        trace_id: None,
        span_id: None,
        occurred_at: clock.now(),
    });
}

fn execute_todo_write(input: &Value, session_id: Option<&str>) -> ToolExecutionOutcome {
    let Some(todos) = input.get("todos").and_then(Value::as_array) else {
        return ToolExecutionOutcome {
            output: "todos must be an array of {content, status} objects.".to_string(),
            is_error: true,
        };
    };
    let mut submitted = Vec::with_capacity(todos.len());
    for (index, todo) in todos.iter().enumerate() {
        let Some(content) = todo.get("content").and_then(Value::as_str) else {
            return ToolExecutionOutcome {
                output: format!("Task {} is missing a string content field.", index + 1),
                is_error: true,
            };
        };
        let Some(status) = todo.get("status").and_then(Value::as_str) else {
            return ToolExecutionOutcome {
                output: format!("Task {} is missing a string status field.", index + 1),
                is_error: true,
            };
        };
        submitted.push((content.to_owned(), status.to_owned()));
    }
    // Validation happens before any store access, so a rejected write provably leaves the
    // previous list untouched rather than half-applied.
    let items = match validate_task_list(&submitted) {
        Ok(items) => items,
        Err(error) => {
            return ToolExecutionOutcome {
                output: error.message(),
                is_error: true,
            }
        }
    };
    let Some(session_id) = session_id else {
        return ToolExecutionOutcome {
            output: "The task list is unavailable because this session has no runtime identity."
                .to_string(),
            is_error: true,
        };
    };
    let stored = task_list_store().replace(session_id, items);
    ToolExecutionOutcome {
        output: if stored.is_empty() {
            "Task list cleared.".to_string()
        } else {
            format!("Task list updated.\n{}", render_task_list(&stored))
        },
        is_error: false,
    }
}

/// Shortens a command for a one-line status header. Cuts on a character boundary so a multi-byte
/// character is never split into replacement characters.
fn truncate_for_label(command: &str) -> String {
    const MAX_LABEL_CHARS: usize = 100;
    if command.chars().count() <= MAX_LABEL_CHARS {
        return command.to_owned();
    }
    let head: String = command.chars().take(MAX_LABEL_CHARS).collect();
    format!("{head}...")
}

fn required_handle_arg(input: &Value) -> Result<&str, ToolExecutionOutcome> {
    match input.get("shell_id").and_then(Value::as_str) {
        Some(handle) if !handle.trim().is_empty() => Ok(handle),
        _ => Err(ToolExecutionOutcome {
            output: "shell_id must be the handle string returned when the background command was started.".to_string(),
            is_error: true,
        }),
    }
}

fn execute_shell_in_background(
    command: &str,
    workspace_folder: &str,
    session_id: Option<&str>,
) -> ToolExecutionOutcome {
    let Some(session_id) = session_id else {
        return background_unavailable("this session has no runtime identity");
    };
    match background_shell_registry().start(session_id, command, workspace_folder) {
        Ok(handle) => ToolExecutionOutcome {
            output: format!(
                "Started background command {handle}. It keeps running after this tool call \
                 returns. Read its output with shell_output(shell_id: \"{handle}\") and stop it \
                 with shell_kill(shell_id: \"{handle}\")."
            ),
            is_error: false,
        },
        Err(BackgroundStartError::SessionLimitReached) => ToolExecutionOutcome {
            output: format!(
                "This session already has {MAX_BACKGROUND_COMMANDS_PER_SESSION} background \
                 commands running. Stop one with shell_kill before starting another."
            ),
            is_error: true,
        },
        Err(BackgroundStartError::Spawn) => ToolExecutionOutcome {
            output: "The background command could not be started.".to_string(),
            is_error: true,
        },
    }
}

fn execute_shell_output(input: &Value, session_id: Option<&str>) -> ToolExecutionOutcome {
    let handle = match required_handle_arg(input) {
        Ok(handle) => handle,
        Err(outcome) => return outcome,
    };
    let Some(session_id) = session_id else {
        return background_unavailable("this session has no runtime identity");
    };
    let registry = background_shell_registry();
    let command = registry.command_label(session_id, handle);
    let Ok(output) = registry.take_output(session_id, handle) else {
        return unknown_background_handle(handle);
    };

    // Naming the command in the header matters once several handles are in flight: a status line
    // that says only "bg_3 running" leaves the model to remember which of them is the build.
    let mut report = match command {
        Some(command) => format!(
            "[{handle}] {} — {}",
            output.status.label(),
            truncate_for_label(&command)
        ),
        None => format!("[{handle}] {}", output.status.label()),
    };
    if output.dropped_bytes > 0 {
        report.push_str(&format!(
            "\n[{} earlier bytes were dropped: the command produced output faster than it was read]",
            output.dropped_bytes
        ));
    }
    if output.remaining_bytes > 0 {
        report.push_str(&format!(
            "\n[{} more bytes are buffered; call shell_output again to continue reading]",
            output.remaining_bytes
        ));
    }
    if output.text.is_empty() {
        report.push_str("\n(no new output)");
    } else {
        report.push('\n');
        report.push_str(&output.text);
    }
    ToolExecutionOutcome {
        output: report,
        // A non-zero exit is information about the command, not a failure of this tool: reporting
        // it as a tool error would make a failing build indistinguishable from a broken handle.
        is_error: false,
    }
}

fn execute_shell_kill(input: &Value, session_id: Option<&str>) -> ToolExecutionOutcome {
    let handle = match required_handle_arg(input) {
        Ok(handle) => handle,
        Err(outcome) => return outcome,
    };
    let Some(session_id) = session_id else {
        return background_unavailable("this session has no runtime identity");
    };
    match background_shell_registry().kill(session_id, handle) {
        Ok(KillOutcome::Terminated(status)) => ToolExecutionOutcome {
            output: format!("Background command {handle} and its child processes were terminated. Status: {}.", status.label()),
            is_error: false,
        },
        Ok(KillOutcome::AlreadyFinished(status)) => ToolExecutionOutcome {
            output: format!(
                "Background command {handle} had already finished, so nothing was terminated. Status: {}.",
                status.label()
            ),
            is_error: false,
        },
        Err(_) => unknown_background_handle(handle),
    }
}

fn unknown_background_handle(handle: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: format!(
            "No background command {handle} belongs to this session. Handles do not survive a \
             desktop restart and cannot be used from another session."
        ),
        is_error: true,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_tool_call_impl(
    name: &str,
    input: &Value,
    workspace_folder: Option<&str>,
    cancelled: Arc<AtomicBool>,
    mcp: &dyn AgentMcpToolPort,
    retrieval: &dyn AgentRetrievalPort,
    code_intelligence: Option<&dyn AgentCodeIntelligencePort>,
    workspace_mutations: Option<&dyn AgentWorkspaceMutationPort>,
    plan_mode: bool,
    skills: &dyn AgentSkillPort,
    // Owning session for background commands (`add-background-shell-execution`). Never
    // model-supplied: a handle resolves only within the session that started it, so accepting a
    // session id as a tool argument would let one session read or kill another's processes.
    session_id: Option<&str>,
) -> ToolExecutionOutcome {
    let registered_handler = ExistingToolHandlerRegistry::resolve(name);
    if registered_handler == Some(ExistingToolHandler::SkillRead) {
        return execute_skill_read(name, input, workspace_folder, skills);
    }
    // `remember` is not dispatched here. It became a personalization operation when model-
    // originated writes started producing candidates, and this dispatcher has no snapshot to judge
    // one against — so the generation loop handles it before reaching this point. Reaching it here
    // would mean the interception was removed, and a proposal would silently become a write again.
    if registered_handler == Some(ExistingToolHandler::Remember) {
        return ToolExecutionOutcome {
            output: "Memory proposals are not available on this path.".to_string(),
            is_error: true,
        };
    }
    // `recall` is handled in the same spot for the same reason: it only ever reads this app's own
    // storage, never the workspace filesystem, so it needs neither a workspace folder nor a
    // plan-mode restriction. It also needs no `agent_id`/`workspace_folder`: memories are one
    // host-level shared pool (`agent-memory-shared-pool`), so there is no slice of it to name.
    if registered_handler == Some(ExistingToolHandler::Recall) {
        return execute_recall(input, retrieval);
    }
    // Handled beside remember/recall for the same reason: it touches only VaneHub-internal
    // session state, so it needs neither a workspace folder nor a plan-mode restriction.
    if registered_handler == Some(ExistingToolHandler::TodoWrite) {
        return execute_todo_write(input, session_id);
    }
    if registered_handler == Some(ExistingToolHandler::SearchCode) {
        let Some(folder) = workspace_folder else {
            return ToolExecutionOutcome {
                output: "Code search is unavailable because this session has no workspace folder."
                    .to_string(),
                is_error: true,
            };
        };
        let Some(code_retrieval) = retrieval.code_retrieval() else {
            return ToolExecutionOutcome {
                output: "Code search is not enabled for this workspace.".to_string(),
                is_error: true,
            };
        };
        return execute_search_code(input, folder, code_retrieval);
    }
    // Plan mode (`add-agent-chat-configuration`) excludes MCP-sourced tools and `shell` from the
    // catalog entirely, and narrows `file` to `read` — but the catalog only shapes what the model
    // is *told* it can do. This is the actual enforcement boundary: nothing stops a model from
    // requesting a tool/operation it was never offered (hallucination, or prompt injection from
    // earlier tool output), so every one of these is re-checked here regardless of the catalog.
    if plan_mode && registered_handler == Some(ExistingToolHandler::Mcp) {
        return plan_mode_denial("MCP tools");
    }
    // MCP tools are similarly folder-independent: a user-scoped MCP server has no project
    // affiliation at all, so a folder-less session can still reach it (`add-agent-mcp-tools`).
    // `mcp.call_tool` re-derives visibility itself (`workspace_folder.unwrap_or_default()` mirrors
    // the CLI relay's own `project_path.unwrap_or_default()` precedent), so no separate check here.
    if registered_handler == Some(ExistingToolHandler::Mcp) {
        let outcome = mcp.call_tool(workspace_folder.unwrap_or_default(), name, input, cancelled);
        return ToolExecutionOutcome {
            output: outcome.output,
            is_error: outcome.is_error,
        };
    }
    if plan_mode && registered_handler == Some(ExistingToolHandler::Shell) {
        return plan_mode_denial("Shell commands");
    }
    if plan_mode && registered_handler == Some(ExistingToolHandler::ShellKill) {
        return plan_mode_denial("Terminating background commands");
    }
    // Reading a background command's output needs no workspace folder: the command was started
    // with one, and retrieval only touches this process's own buffers. Handled before the
    // folder gate for the same reason `remember`/`recall` are.
    if registered_handler == Some(ExistingToolHandler::ShellOutput) {
        return execute_shell_output(input, session_id);
    }
    if registered_handler == Some(ExistingToolHandler::ShellKill) {
        return execute_shell_kill(input, session_id);
    }
    if plan_mode && registered_handler == Some(ExistingToolHandler::Edit) {
        return plan_mode_denial("Editing files");
    }
    // The plan-mode catalog offers a read-only notebook, but the catalog only shapes what the model
    // is told; this is the boundary that holds if it asks for an operation it was never offered.
    if plan_mode
        && registered_handler == Some(ExistingToolHandler::Notebook)
        && input.get("operation").and_then(Value::as_str) != Some("read")
    {
        return plan_mode_denial("Editing notebooks");
    }
    let Some(folder) = workspace_folder else {
        return ToolExecutionOutcome {
            output: "This session has no workspace folder configured.".to_string(),
            is_error: true,
        };
    };
    if registered_handler == Some(ExistingToolHandler::CodeIntelligence) {
        let Some(code_intelligence) = code_intelligence else {
            return ToolExecutionOutcome {
                output: "Code intelligence is unavailable for this session.".to_owned(),
                is_error: true,
            };
        };
        return execute_code_intelligence_tool(name, input, folder, cancelled, code_intelligence);
    }
    match registered_handler {
        Some(ExistingToolHandler::Shell) => {
            let command = input
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if input
                .get("run_in_background")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return execute_shell_in_background(command, folder, session_id);
            }
            let timeout_ms = match parse_optional_non_negative_integer_arg(input, "timeout_ms") {
                Ok(timeout_ms) => timeout_ms.map(|value| value as u64),
                Err(outcome) => return outcome,
            };
            execute_shell(command, folder, cancelled, timeout_ms)
        }
        Some(ExistingToolHandler::File) => {
            let operation = input
                .get("operation")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if plan_mode && operation != "read" {
                return plan_mode_denial("Writing files");
            }
            let path = input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let content = input.get("content").and_then(Value::as_str);
            let offset = match parse_optional_non_negative_integer_arg(input, "offset") {
                Ok(offset) => offset,
                Err(outcome) => return outcome,
            };
            let limit = match parse_optional_non_negative_integer_arg(input, "limit") {
                Ok(limit) => limit,
                Err(outcome) => return outcome,
            };
            let outcome = execute_file(operation, path, content, offset, limit, folder);
            if operation == "write" && !outcome.is_error {
                publish_workspace_mutation(folder, path, workspace_mutations);
            }
            outcome
        }
        Some(ExistingToolHandler::Grep) => {
            let context = match parse_optional_non_negative_integer_arg(input, "context") {
                Ok(context) => context.unwrap_or(0),
                Err(outcome) => return outcome,
            };
            let head_limit = match parse_optional_non_negative_integer_arg(input, "head_limit") {
                Ok(head_limit) => head_limit,
                Err(outcome) => return outcome,
            };
            execute_grep(
                GrepRequest {
                    pattern: input
                        .get("pattern")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    glob: input.get("glob").and_then(Value::as_str),
                    path: input.get("path").and_then(Value::as_str),
                    output_mode: input
                        .get("output_mode")
                        .and_then(Value::as_str)
                        .unwrap_or(OUTPUT_MODE_FILES),
                    context,
                    case_insensitive: input
                        .get("case_insensitive")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    head_limit,
                },
                folder,
                cancelled,
            )
        }
        Some(ExistingToolHandler::Glob) => execute_glob(
            input
                .get("pattern")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            input.get("path").and_then(Value::as_str),
            folder,
            cancelled,
        ),
        Some(ExistingToolHandler::Notebook) => {
            let path = input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let operation = input
                .get("operation")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let outcome = execute_notebook(
                NotebookRequest {
                    operation,
                    path,
                    cell_id: input.get("cell_id").and_then(Value::as_str),
                    cell_index: input
                        .get("cell_index")
                        .and_then(Value::as_u64)
                        .and_then(|index| usize::try_from(index).ok()),
                    source: input.get("source").and_then(Value::as_str),
                    cell_type: input.get("cell_type").and_then(Value::as_str),
                    position: input.get("position").and_then(Value::as_str),
                },
                folder,
            );
            if !outcome.is_error && operation != "read" {
                publish_workspace_mutation(folder, path, workspace_mutations);
            }
            outcome
        }
        Some(ExistingToolHandler::Edit) => {
            let path = input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let outcome = execute_edit(
                path,
                input
                    .get("old_string")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                input
                    .get("new_string")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                input
                    .get("replace_all")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                folder,
            );
            if !outcome.is_error {
                publish_workspace_mutation(folder, path, workspace_mutations);
            }
            outcome
        }
        _ => ToolExecutionOutcome {
            output: format!("Unknown tool \"{name}\"."),
            is_error: true,
        },
    }
}

fn publish_workspace_mutation(
    workspace_folder: &str,
    relative_path: &str,
    workspace_mutations: Option<&dyn AgentWorkspaceMutationPort>,
) {
    let Some(workspace_mutations) = workspace_mutations else {
        return;
    };
    let Ok(boundary) = BoundedFilesystem::new(Path::new(workspace_folder)) else {
        return;
    };
    let Ok(relative_path) = boundary.validate_relative(relative_path) else {
        return;
    };
    let Ok(canonical_workspace) = Path::new(workspace_folder).canonicalize() else {
        return;
    };
    workspace_mutations.publish(AgentWorkspaceMutation {
        canonical_workspace,
        relative_path: relative_path.to_string_lossy().replace('\\', "/"),
    });
}

fn execute_code_intelligence_tool(
    name: &str,
    input: &Value,
    folder: &str,
    cancelled: Arc<AtomicBool>,
    code_intelligence: &dyn AgentCodeIntelligencePort,
) -> ToolExecutionOutcome {
    let relative_path = input
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if relative_path.is_empty() {
        return invalid_code_intelligence_input("path must be a non-empty relative string");
    }
    let context = AgentCodeIntelligenceContext::from_session_workspace(folder);
    if name == GET_DIAGNOSTICS_TOOL_NAME {
        return diagnostics_outcome(code_intelligence.get_diagnostics(
            &context,
            &AgentDocumentInput { relative_path },
            cancelled,
        ));
    }
    let Some(line) = one_based_u32(input, "line") else {
        return invalid_code_intelligence_input("line must be a one-based integer");
    };
    let Some(column) = one_based_u32(input, "column") else {
        return invalid_code_intelligence_input("column must be a one-based integer");
    };
    let position = AgentDocumentPositionInput {
        relative_path,
        line,
        column,
    };
    match name {
        FIND_DEFINITION_TOOL_NAME => locations_outcome(
            "definitions",
            code_intelligence.find_definition(&context, &position, cancelled),
            20,
        ),
        FIND_REFERENCES_TOOL_NAME => locations_outcome(
            "references",
            code_intelligence.find_references(&context, &position, cancelled),
            50,
        ),
        GET_HOVER_TOOL_NAME => {
            hover_outcome(code_intelligence.get_hover(&context, &position, cancelled))
        }
        _ => invalid_code_intelligence_input("unsupported code-intelligence operation"),
    }
}

fn one_based_u32(input: &Value, field: &str) -> Option<u32> {
    let value = input.get(field)?.as_u64()?;
    (value > 0).then(|| u32::try_from(value).ok()).flatten()
}

fn invalid_code_intelligence_input(message: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: message.to_owned(),
        is_error: true,
    }
}

/// Retrieval failure **never** returns `Err` here — it returns a normal tool result telling the
/// model that recall is temporarily unavailable, so generation continues. Bubbling an optional
/// enhancement's failure up as a generation failure is unacceptable (design.md §8.1): the model
/// must never confuse "search failed" with "no such memory exists".
fn execute_recall(input: &Value, retrieval: &dyn AgentRetrievalPort) -> ToolExecutionOutcome {
    let query = input
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if query.is_empty() {
        return ToolExecutionOutcome {
            output: "No query was provided to recall.".to_string(),
            is_error: true,
        };
    }
    let limit = input
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 20) as usize;
    match retrieval.search(query, limit) {
        Ok(outcome) => ToolExecutionOutcome {
            output: serde_json::to_string(&recall_payload(&outcome))
                .unwrap_or_else(|_| "{\"results\":[]}".to_string()),
            is_error: false,
        },
        Err(_) => ToolExecutionOutcome {
            output: "Memory search is temporarily unavailable. Continue without it.".to_string(),
            is_error: false,
        },
    }
}

/// Projects `outcome` into exactly what the model should see: `content`/`created_at`/
/// `matched_via` per hit, `degraded` only when present. `source_id`/`score` are internal — no
/// decision value to the model, and raw material for hallucination if included
/// (`AgentRetrievalHit` doesn't even carry them, so there is nothing here to accidentally leak).
fn recall_payload(outcome: &AgentRetrievalOutcome) -> Value {
    let hits: Vec<Value> = outcome
        .hits
        .iter()
        .map(|hit| {
            json!({
                "content": hit.content,
                "created_at": hit.created_at,
                "matched_via": hit.matched_via,
            })
        })
        .collect();
    match &outcome.degraded {
        Some(degraded) => json!({ "results": hits, "degraded": degraded }),
        None => json!({ "results": hits }),
    }
}

fn execute_search_code(
    input: &Value,
    workspace_folder: &str,
    retrieval: &dyn crate::contexts::agent_runtime::application::AgentCodeRetrievalPort,
) -> ToolExecutionOutcome {
    let query = input
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if query.is_empty() {
        return ToolExecutionOutcome {
            output: "No query was provided to search_code.".to_string(),
            is_error: true,
        };
    }
    let limit = input
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 20) as usize;
    match retrieval.search_code(workspace_folder, query, limit) {
        Ok(outcome) => ToolExecutionOutcome {
            output: serde_json::to_string(&code_search_payload(&outcome))
                .unwrap_or_else(|_| "{\"results\":[]}".to_string()),
            is_error: false,
        },
        Err(_) => ToolExecutionOutcome {
            output: "Code search is temporarily unavailable. Continue without it.".to_string(),
            is_error: false,
        },
    }
}

fn code_search_payload(outcome: &AgentCodeRetrievalOutcome) -> Value {
    let hits = outcome
        .hits
        .iter()
        .map(|hit| {
            json!({
                "file_path": hit.file_path,
                "start_line": hit.start_line,
                "end_line": hit.end_line,
                "language": hit.language,
                "symbol_name": hit.symbol_name,
                "symbol_kind": hit.symbol_kind,
                "snippet": hit.snippet,
                "matched_via": hit.matched_via,
            })
        })
        .collect::<Vec<_>>();
    let mut payload = json!({ "results": hits });
    if let Some(degraded) = &outcome.degraded {
        payload["degraded"] = Value::String(degraded.clone());
    }
    payload
}
