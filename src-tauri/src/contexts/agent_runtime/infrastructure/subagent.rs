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
    child_reply_turns, run_child_turn, wire_format_for, REQUEST_TIMEOUT,
};
use super::tools::{execute_file, execute_glob, execute_grep, GrepRequest};
use crate::contexts::agent_runtime::application::{
    ApiAgentGateway, ApiCredentialPort, NativeToolErrorCode, NativeToolPortRequest,
    NativeToolResultEnvelope, NativeToolResultStatus, SubagentPort, ToolDefinition, ToolUseBlock,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::platform::network::blocking_http_client;
use serde_json::{json, Value};
use std::collections::BTreeMap;
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
    slots: ConcurrencySlots,
}

impl NativeSubagentExecutor {
    pub(crate) fn new(
        credentials: Arc<dyn ApiCredentialPort>,
        config: Arc<dyn ApiAgentGateway>,
    ) -> Self {
        Self {
            credentials,
            config,
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

        let catalog = child_tool_catalog();
        let mut turns = vec![json!({ "role": "user", "content": task })];
        let mut tool_calls_used = 0_u32;

        for _turn in 0..MAX_CHILD_TURNS {
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
                Ok(value) => value,
                Err(_) => {
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
                return succeeded(&text, tool_calls_used);
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

            let executed: Vec<(ToolUseBlock, String, bool)> = requested
                .into_iter()
                .map(|call| {
                    tool_calls_used += 1;
                    let (output, is_error) = execute_child_tool(&call, &workspace);
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

/// The child's whole tool surface. Read-only by construction: these three definitions are the only
/// ones offered, and `execute_child_tool` is the only dispatcher, so a child has no route to
/// anything else even if the model asks for it.
fn child_tool_catalog() -> Vec<ToolDefinition> {
    crate::contexts::agent_runtime::application::plan_mode_tool_catalog()
        .into_iter()
        .filter(|tool| matches!(tool.name.as_str(), "file" | "grep" | "glob"))
        .collect()
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
