//! The bounded child OnePiece attempt behind `delegate_subagent` (`add-onepiece-subagents`).
//!
//! The child's authority is structural, not a filter. It dispatches to exactly three functions --
//! bounded file *reads*, content search, and filename search -- so there is no code path from a
//! child to a write, a process, the network, the user, or another child. An allowlist would be a
//! rule that can be got wrong; this cannot express the forbidden call at all.
//!
//! The child never sees the parent's transcript. It gets its task text and whatever it reads
//! itself, which is the entire point: exploring in the parent's context is what makes exploring
//! expensive.

use super::api_process_adapter::{
    begin_child_invocation, child_reply_turns, finish_child_invocation, run_child_turn,
    wire_format_for, ChildInvocationIdentity, REQUEST_TIMEOUT,
};
use super::subagent_worktree::{CapturedFile, ChildWorktree, WorktreeError};
use super::tools::{execute_edit, execute_file, execute_glob, execute_grep, GrepRequest};
use super::SqliteNativeToolRepository;
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLoggingPort, ApiAgentGateway, ApiCredentialPort, NativeToolErrorCode,
    NativeToolPortRequest, NativeToolProgress, NativeToolProgressPhase, NativeToolResultEnvelope,
    NativeToolResultStatus, SubagentPort, ToolDefinition, ToolUseBlock,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::agent_runtime::application::{
    ChangeSetFileRecord, ChangeSetRecord, FileChangeKind,
};
use crate::contexts::artifacts::api::{
    ArtifactCreateRequest, ArtifactCreator, ArtifactEvidenceKind, ArtifactService,
    ArtifactVisibility,
};
use crate::contexts::sessions::api::{SessionsApi, UsageStatus};
use crate::platform::network::blocking_http_client;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// Model turns a child may take. Each turn is one provider round trip, so this bounds both the
/// child's spend and how long the parent's tool call blocks.
const MAX_CHILD_TURNS: u32 = 12;

/// Tool calls a child may execute across all its turns.
const MAX_CHILD_TOOL_CALLS: u32 = 40;

/// The child's answer enters the parent's context, so it is the one bound the parent pays for.
const MAX_CHILD_RESULT_CHARS: usize = 4_000;

/// Running children one parent session may own at once.
const MAX_CONCURRENT_CHILDREN: usize = 4;

const CHILD_INSTRUCTIONS: &str = "\
You are a bounded investigator working on behalf of another agent. You have read-only tools: \
`file` (read only), `grep`, and `glob`. You cannot write files, run commands, reach the network, \
ask questions, or delegate.

Investigate the task and answer it. Read only what you need. When you have the answer, reply with \
it directly and stop calling tools. Your reply is the entire result the caller receives -- they \
cannot see anything you read, so state findings concretely, with file paths and line numbers \
where they matter. If the task cannot be answered from this workspace, say so and say why.";

/// Per-session running-child counters. Split out from the executor so the cap can be tested
/// without standing up provider ports it does not touch.
#[derive(Debug, Default, Clone)]
struct ConcurrencySlots {
    running: Arc<Mutex<BTreeMap<String, usize>>>,
}

impl ConcurrencySlots {
    /// Claims a slot for `session_id`, or reports that the session is already at its cap. A
    /// refused claim never terminates a running child to make room.
    fn claim(&self, session_id: &str) -> bool {
        let Ok(mut running) = self.running.lock() else {
            return false;
        };
        let count = running.entry(session_id.to_owned()).or_insert(0);
        if *count >= MAX_CONCURRENT_CHILDREN {
            return false;
        }
        *count += 1;
        true
    }

    fn release(&self, session_id: &str) {
        if let Ok(mut running) = self.running.lock() {
            if let Some(count) = running.get_mut(session_id) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    running.remove(session_id);
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct NativeSubagentExecutor {
    credentials: Arc<dyn ApiCredentialPort>,
    config: Arc<dyn ApiAgentGateway>,
    /// Child turns spend real tokens. Without this they would spend them invisibly, which is worse
    /// than mis-attributing them (`add-onepiece-subagents`).
    accounting: Option<SessionsApi>,
    clock: Arc<dyn AgentClockPort>,
    logging: Arc<dyn AgentLoggingPort>,
    artifacts: Arc<ArtifactService>,
    operations: Arc<SqliteNativeToolRepository>,
    operations_root: PathBuf,
    slots: ConcurrencySlots,
}

/// What a child attempt runs against. Bundled because these travel together and only the subagent
/// executor uses them.
pub(crate) struct SubagentRuntime {
    pub(crate) credentials: Arc<dyn ApiCredentialPort>,
    pub(crate) config: Arc<dyn ApiAgentGateway>,
    pub(crate) accounting: Option<SessionsApi>,
    pub(crate) clock: Arc<dyn AgentClockPort>,
    pub(crate) logging: Arc<dyn AgentLoggingPort>,
    pub(crate) artifacts: Arc<ArtifactService>,
    pub(crate) operations: Arc<SqliteNativeToolRepository>,
    pub(crate) operations_root: PathBuf,
}

impl NativeSubagentExecutor {
    pub(crate) fn new(runtime: SubagentRuntime) -> Self {
        let SubagentRuntime {
            credentials,
            config,
            accounting,
            clock,
            logging,
            artifacts,
            operations,
            operations_root,
        } = runtime;
        Self {
            credentials,
            config,
            accounting,
            clock,
            logging,
            artifacts,
            operations,
            operations_root,
            slots: ConcurrencySlots::default(),
        }
    }
}

impl SubagentPort for NativeSubagentExecutor {
    fn execute_subagent(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        let session_id = request.context.session_id.clone();
        if !self.slots.claim(&session_id) {
            return failure(
                NativeToolErrorCode::LimitExceeded,
                format!(
                    "This session already has {MAX_CONCURRENT_CHILDREN} investigations running. Wait for one to finish."
                ),
            );
        }
        let outcome = self.run(&request);
        self.slots.release(&session_id);
        outcome
    }
}

impl NativeSubagentExecutor {
    fn run(&self, request: &NativeToolPortRequest) -> NativeToolResultEnvelope {
        let context = &request.context;
        let Some(workspace) = context
            .canonical_workspace
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned())
        else {
            return failure(
                NativeToolErrorCode::Ineligible,
                "A subagent needs a workspace to investigate.".to_owned(),
            );
        };
        let Some(task) = request.input.value.get("task").and_then(Value::as_str) else {
            return failure(
                NativeToolErrorCode::InvalidInput,
                "A subagent needs a task.".to_owned(),
            );
        };

        // The credential is used through the existing boundary and never copied into the child's
        // prompt, its result, or any record of the attempt.
        let Ok(Some(api_key)) = self.credentials.fetch(&context.agent_id) else {
            return failure(
                NativeToolErrorCode::Unavailable,
                "No credential is available for this agent.".to_owned(),
            );
        };
        let Ok(Some(config)) = self.config.provider_config(&context.agent_id) else {
            return failure(
                NativeToolErrorCode::Unavailable,
                "No provider configuration is available for this agent.".to_owned(),
            );
        };
        let Ok(wire_format) = wire_format_for(&config) else {
            return failure(
                NativeToolErrorCode::Unavailable,
                "This provider's interface format is unsupported.".to_owned(),
            );
        };
        let Ok(client) = blocking_http_client(REQUEST_TIMEOUT) else {
            return failure(
                NativeToolErrorCode::Unavailable,
                "The provider client could not be created.".to_owned(),
            );
        };

        let mutating = request
            .input
            .value
            .get("change_files")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        // Provisioned before the loop so every exit below drops it, and `ChildWorktree::drop`
        // reaps. A child that is cancelled mid-edit leaves nothing behind.
        let worktree = if mutating {
            match ChildWorktree::provision(std::path::Path::new(&workspace), &self.operations_root)
            {
                Ok(worktree) => Some(worktree),
                Err(error) => {
                    return failure(NativeToolErrorCode::Ineligible, error.message().to_owned())
                }
            }
        } else {
            None
        };
        // A mutating child edits its own copy; a read-only child never leaves the parent's.
        let child_root = worktree.as_ref().map_or_else(
            || workspace.clone(),
            |tree| tree.path().to_string_lossy().into_owned(),
        );
        let catalog = child_tool_catalog(mutating);
        let mut turns = vec![json!({ "role": "user", "content": task })];
        let mut tool_calls_used = 0_u32;
        let identity = ChildInvocationIdentity {
            call_id: &context.call_id,
            session_id: &context.session_id,
            agent_id: &context.agent_id,
            operation_id: &context.generation_id,
        };

        publish_progress(
            context,
            0,
            NativeToolProgressPhase::Started,
            "Investigating.".to_owned(),
            0,
        );
        for turn_index in 0..MAX_CHILD_TURNS {
            if context.is_cancelled() {
                return terminal(NativeToolResultStatus::Cancelled, None, tool_calls_used);
            }
            if context.deadline_reached() {
                return terminal(
                    NativeToolResultStatus::LimitExceeded,
                    Some("The investigation ran out of time.".to_owned()),
                    tool_calls_used,
                );
            }
            let invocation = begin_child_invocation(
                self.accounting.as_ref(),
                identity,
                &config,
                turn_index,
                self.clock.as_ref(),
            );
            let turn = run_child_turn(
                &wire_format,
                &client,
                &api_key,
                &config.model_id,
                Some(CHILD_INSTRUCTIONS),
                &turns,
                &catalog,
                &context.cancelled,
            );
            let (text, requested) = match turn {
                Ok((text, requested, usage)) => {
                    finish_child_invocation(
                        self.accounting.as_ref(),
                        invocation.as_ref(),
                        &context.session_id,
                        usage.as_ref(),
                        UsageStatus::Succeeded,
                        self.clock.as_ref(),
                        self.logging.as_ref(),
                    );
                    (text, requested)
                }
                Err(_) => {
                    finish_child_invocation(
                        self.accounting.as_ref(),
                        invocation.as_ref(),
                        &context.session_id,
                        None,
                        if context.is_cancelled() {
                            UsageStatus::Cancelled
                        } else {
                            UsageStatus::Failed
                        },
                        self.clock.as_ref(),
                        self.logging.as_ref(),
                    );
                    // The provider's own diagnostic is not forwarded: it can carry endpoint and
                    // credential detail, and the parent only needs to know the child failed.
                    return terminal(
                        NativeToolResultStatus::Failed,
                        Some("The investigation could not be completed.".to_owned()),
                        tool_calls_used,
                    );
                }
            };
            if requested.is_empty() {
                return match worktree.as_ref() {
                    Some(worktree) => self.seal(worktree, &text, context, tool_calls_used),
                    None => succeeded(&text, tool_calls_used),
                };
            }
            if tool_calls_used.saturating_add(requested.len() as u32) > MAX_CHILD_TOOL_CALLS {
                return terminal(
                    NativeToolResultStatus::LimitExceeded,
                    Some(bounded(&text).unwrap_or_else(|| {
                        "The investigation reached its tool-call limit.".to_owned()
                    })),
                    tool_calls_used,
                );
            }

            // What the child is reading, not what it read: the parent is blocked inside this
            // call and cannot see anything until it returns, so this is for the user watching a
            // long investigation advance.
            publish_progress(
                context,
                turn_index.saturating_add(1),
                NativeToolProgressPhase::Updated,
                format!(
                    "Reading {} more source{}.",
                    requested.len(),
                    if requested.len() == 1 { "" } else { "s" }
                ),
                tool_calls_used,
            );
            let executed: Vec<(ToolUseBlock, String, bool)> = requested
                .into_iter()
                .map(|call| {
                    tool_calls_used += 1;
                    let (output, is_error) = if mutating {
                        execute_mutating_child_tool(&call, &child_root)
                    } else {
                        execute_child_tool(&call, &child_root)
                    };
                    (call, output, is_error)
                })
                .collect();
            turns.extend(child_reply_turns(&wire_format, &text, &executed));
        }

        terminal(
            NativeToolResultStatus::LimitExceeded,
            Some("The investigation reached its turn limit without concluding.".to_owned()),
            tool_calls_used,
        )
    }
}

impl NativeSubagentExecutor {
    /// Captures the child's work, seals the diff as an Artifact, and records the ChangeSet, in
    /// that order and before the worktree is dropped. A capture that is not sealed would leave the
    /// edits in a directory about to be reaped and have the model report work that no longer
    /// exists (`add-onepiece-subagents` D8).
    fn seal(
        &self,
        worktree: &ChildWorktree,
        text: &str,
        context: &crate::contexts::agent_runtime::application::NativeToolExecutionContext,
        tool_calls: u32,
    ) -> NativeToolResultEnvelope {
        let files = match worktree.capture() {
            Ok(files) => files,
            Err(error) => {
                return failure(
                    NativeToolErrorCode::LimitExceeded,
                    error.message().to_owned(),
                )
            }
        };
        if files.is_empty() {
            // Nothing changed: report the child's answer rather than an empty change set the user
            // would have to open to discover it is empty.
            return succeeded(text, tool_calls);
        }
        let Ok(diff) = worktree.diff() else {
            return failure(
                NativeToolErrorCode::ExternalFailure,
                WorktreeError::GitFailure.message().to_owned(),
            );
        };

        let created_at = self.clock.now();
        let artifact = self.artifacts.create_bytes(
            ArtifactCreateRequest {
                operation_id: context.call_id.clone(),
                display_name: "subagent-change-set.patch".to_owned(),
                media_type: "text/x-patch".to_owned(),
                creator: ArtifactCreator {
                    kind: "subagent".to_owned(),
                    id: context.agent_id.clone(),
                },
                // The child is a model, not the host: its output is reviewed, never trusted.
                evidence_kind: ArtifactEvidenceKind::UntrustedExternal,
                visibility: ArtifactVisibility::Session,
                source_artifact_ids: Vec::new(),
                created_at: created_at.clone(),
                expires_at: None,
            },
            &diff,
        );
        let Ok(artifact) = artifact else {
            return failure(
                NativeToolErrorCode::ExternalFailure,
                "The subagent's change set could not be stored.".to_owned(),
            );
        };

        let record = ChangeSetRecord {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            artifact_id: artifact.id.clone(),
            content_hash: artifact.content_hash.clone(),
            repository_identity: worktree.repository_identity().to_owned(),
            base_commit: worktree.base_commit().to_owned(),
            attempt_id: context.call_id.clone(),
            files: files.iter().map(change_set_file).collect(),
            warnings: Vec::new(),
            created_at,
        };
        if self.operations.insert_change_set(&record).is_err() {
            return failure(
                NativeToolErrorCode::ExternalFailure,
                "The subagent's change set could not be recorded.".to_owned(),
            );
        }

        let summary = bounded(text).unwrap_or_else(|| "The subagent made changes.".to_owned());
        NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: Some(json!({
                "summary": summary,
                "changeSetArtifactId": artifact.id,
                "changedFiles": files.iter().map(|file| file.path.clone()).collect::<Vec<_>>(),
            })),
            error_code: None,
            safe_error: None,
            truncated: text.chars().count() > MAX_CHILD_RESULT_CHARS,
            metadata: metadata(tool_calls),
        }
    }
}

fn change_set_file(file: &CapturedFile) -> ChangeSetFileRecord {
    ChangeSetFileRecord {
        path: file.path.clone(),
        change_kind: match file.status {
            '?' | 'A' => FileChangeKind::Add,
            'D' => FileChangeKind::Delete,
            'R' => FileChangeKind::Rename,
            _ => FileChangeKind::Modify,
        },
        old_hash: None,
        new_hash: file.new_hash.clone(),
        binary: file.binary,
        mode: None,
    }
}

/// The mutating child's dispatcher, deliberately separate from the read-only one rather than a
/// flag inside it (`add-onepiece-subagents` D8). Keeping them apart means the read-only path
/// still cannot express a write no matter what any caller passes.
fn execute_mutating_child_tool(call: &ToolUseBlock, worktree_root: &str) -> (String, bool) {
    let input = call.input.clone().unwrap_or(Value::Null);
    let string = |field: &str| {
        input
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    match call.name.as_str() {
        "edit" => {
            let outcome = execute_edit(
                &string("path"),
                &string("old_string"),
                &string("new_string"),
                input
                    .get("replace_all")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                worktree_root,
            );
            (outcome.output, outcome.is_error)
        }
        "file" if input.get("operation").and_then(Value::as_str) == Some("write") => {
            let outcome = execute_file(
                "write",
                &string("path"),
                input.get("content").and_then(Value::as_str),
                None,
                None,
                worktree_root,
            );
            (outcome.output, outcome.is_error)
        }
        // Everything else, including a `file` read, falls through to the read-only dispatcher, so
        // the reading half has exactly one implementation.
        _ => execute_child_tool(call, worktree_root),
    }
}

/// Emits a bounded progress event. Carries counts and a fixed phrase only -- never a file path, a
/// search pattern, or anything the child read, since progress is a user-facing signal and the
/// child's reading is exactly what stays inside its own context.
fn publish_progress(
    context: &crate::contexts::agent_runtime::application::NativeToolExecutionContext,
    sequence: u32,
    phase: NativeToolProgressPhase,
    message: String,
    tool_calls: u32,
) {
    context.progress.publish(NativeToolProgress {
        sequence,
        phase,
        message: Some(message),
        metadata: BTreeMap::from([("tool_calls".to_owned(), json!(tool_calls))]),
    });
}

/// The child's whole tool surface. Read-only by construction: these three definitions are the only
/// ones offered, and `execute_child_tool` is the only dispatcher, so a child has no route to
/// anything else even if the model asks for it.
fn child_tool_catalog(mutating: bool) -> Vec<ToolDefinition> {
    let mut catalog: Vec<ToolDefinition> =
        crate::contexts::agent_runtime::application::plan_mode_tool_catalog()
            .into_iter()
            .filter(|tool| matches!(tool.name.as_str(), "file" | "grep" | "glob"))
            .collect();
    if !mutating {
        return catalog;
    }
    // A mutating child needs the full `file` definition (its plan-mode copy cannot write) plus
    // `edit`. Both are scoped to the child's own worktree by the dispatcher, never the parent's.
    catalog.retain(|tool| tool.name != "file");
    catalog.extend(
        crate::contexts::agent_runtime::application::tool_catalog()
            .into_iter()
            .filter(|tool| matches!(tool.name.as_str(), "file" | "edit")),
    );
    catalog
}

fn execute_child_tool(call: &ToolUseBlock, workspace: &str) -> (String, bool) {
    let input = call.input.clone().unwrap_or(Value::Null);
    let string = |field: &str| {
        input
            .get(field)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let number = |field: &str| {
        input
            .get(field)
            .and_then(Value::as_u64)
            .map(|value| value as usize)
    };
    let outcome = match call.name.as_str() {
        // Reads only. The write operation is unreachable from here regardless of what the model
        // puts in `operation`.
        "file" => execute_file(
            "read",
            &string("path"),
            None,
            number("offset"),
            number("limit"),
            workspace,
        ),
        "grep" => execute_grep(
            GrepRequest {
                pattern: &string("pattern"),
                glob: input.get("glob").and_then(Value::as_str),
                path: input.get("path").and_then(Value::as_str),
                output_mode: input
                    .get("output_mode")
                    .and_then(Value::as_str)
                    .unwrap_or(super::tools::OUTPUT_MODE_FILES),
                context: number("context").unwrap_or(0),
                case_insensitive: input
                    .get("case_insensitive")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                head_limit: number("head_limit"),
            },
            workspace,
            Arc::new(AtomicBool::new(false)),
        ),
        "glob" => execute_glob(
            &string("pattern"),
            input.get("path").and_then(Value::as_str),
            workspace,
            Arc::new(AtomicBool::new(false)),
        ),
        other => {
            return (
                format!("{other} is not available to a subagent. You have file, grep, and glob."),
                true,
            )
        }
    };
    (outcome.output, outcome.is_error)
}

fn succeeded(text: &str, tool_calls: u32) -> NativeToolResultEnvelope {
    let Some(summary) = bounded(text) else {
        return terminal(
            NativeToolResultStatus::Failed,
            Some("The investigation produced no answer.".to_owned()),
            tool_calls,
        );
    };
    let truncated = text.chars().count() > MAX_CHILD_RESULT_CHARS;
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status: NativeToolResultStatus::Succeeded,
        output: Some(json!({ "summary": summary })),
        error_code: None,
        safe_error: None,
        truncated,
        metadata: metadata(tool_calls),
    }
}

/// Trims and caps the child's answer. `None` when there is nothing left, which is a failure rather
/// than an empty success -- an empty answer would read to the parent as "investigated, found
/// nothing".
fn bounded(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(MAX_CHILD_RESULT_CHARS).collect())
}

fn terminal(
    status: NativeToolResultStatus,
    message: Option<String>,
    tool_calls: u32,
) -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status,
        output: message
            .as_ref()
            .map(|summary| json!({ "summary": summary })),
        error_code: match status {
            NativeToolResultStatus::LimitExceeded => Some(NativeToolErrorCode::LimitExceeded),
            NativeToolResultStatus::Cancelled => Some(NativeToolErrorCode::Cancelled),
            NativeToolResultStatus::Failed => Some(NativeToolErrorCode::ExternalFailure),
            _ => None,
        },
        safe_error: message,
        truncated: false,
        metadata: metadata(tool_calls),
    }
}

fn failure(code: NativeToolErrorCode, message: String) -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status: NativeToolResultStatus::Unavailable,
        output: None,
        error_code: Some(code),
        safe_error: Some(message),
        truncated: false,
        metadata: BTreeMap::new(),
    }
}

/// Counts and timing only. The child's turns, its tool inputs, and its tool outputs never appear
/// here or anywhere the parent can read them.
fn metadata(tool_calls: u32) -> BTreeMap<String, Value> {
    BTreeMap::from([("tool_calls".to_owned(), json!(tool_calls))])
}

#[cfg(test)]
#[path = "subagent_tests.rs"]
mod tests;
