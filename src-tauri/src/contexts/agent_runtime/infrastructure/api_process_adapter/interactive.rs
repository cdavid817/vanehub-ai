//! Permission mapping and the tools that block on a user decision.

use super::super::memory_directory::is_within_memory_directory;
use super::super::tools::ToolExecutionOutcome;
use super::{failed_non_retryable, failed_retryable, PendingApprovals, APPROVAL_POLL_INTERVAL};
use crate::contexts::agent_runtime::application::{
    AgentPermissionPort, AgentProcessEventSink, GenerationProcessEvent, GenerationProcessRequest,
    ToolApprovalDecision, ToolUseBlock, ASK_USER_QUESTION_TOOL_NAME, EDIT_TOOL_NAME,
    EXIT_PLAN_MODE_TOOL_NAME, FILE_TOOL_NAME, FIND_DEFINITION_TOOL_NAME, FIND_REFERENCES_TOOL_NAME,
    GET_DIAGNOSTICS_TOOL_NAME, GET_HOVER_TOOL_NAME, GLOB_TOOL_NAME, GREP_TOOL_NAME,
    LIST_SKILLS_TOOL_NAME, LOAD_SKILL_TOOL_NAME, MAX_PLAN_CHARS, MAX_QUESTION_CHARS,
    MAX_QUESTION_OPTIONS, MAX_QUESTION_OPTION_CHARS, MCP_TOOL_NAME_PREFIX, MIN_QUESTION_OPTIONS,
    NOTEBOOK_TOOL_NAME, READ_SKILL_RESOURCE_TOOL_NAME, RECALL_TOOL_NAME, REMEMBER_TOOL_NAME,
    SEARCH_CODE_TOOL_NAME, SHELL_KILL_TOOL_NAME, SHELL_OUTPUT_TOOL_NAME, SHELL_TOOL_NAME,
    TODO_WRITE_TOOL_NAME,
};
use crate::contexts::permissions::domain::{Action, Effect, Resource};
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};

/// Maps every built-in tool to the established permission action whose policy behavior matches
/// that tool. A name outside the built-in catalog maps to a synthetic action no template declares
/// a rule for, so hallucinated calls still fail closed to `Ask`.
pub(super) fn permission_action_and_resource(tool_name: &str, input: &Value) -> (Action, Resource) {
    match tool_name {
        // Background start is deliberately not a weaker classification than a foreground call:
        // same command, same workspace, same effects -- only the wait differs
        // (`add-background-shell-execution`).
        SHELL_TOOL_NAME => (Action::shell_exec(), Resource::workspace()),
        // Reading a background command's output observes already-approved work, so it is
        // classified like the other read-only tools. Terminating one only *reduces* the effects
        // of work the user already approved, and can act on nothing else -- a handle resolves
        // solely within its own session -- so gating it behind another prompt would make stopping
        // a runaway process harder than starting it was.
        SHELL_OUTPUT_TOOL_NAME | SHELL_KILL_TOOL_NAME => {
            (Action::file_read(), Resource::new(tool_name))
        }
        // Writes only VaneHub-internal session state, with no workspace, process, or network
        // effect -- the same no-approval classification the fixed Skill tools use.
        TODO_WRITE_TOOL_NAME => (Action::file_read(), Resource::new(tool_name)),
        // The user's answer is itself the authorization; a separate approval prompt in front of a
        // question would ask permission to ask permission.
        ASK_USER_QUESTION_TOOL_NAME => (Action::file_read(), Resource::new(tool_name)),
        // Same reasoning: the decision the tool blocks on *is* the authorization, and it authorizes
        // a session mode rather than an action on a resource, so it must not classify as one
        // (`add-agent-plan-exit-request` D2).
        EXIT_PLAN_MODE_TOOL_NAME => (Action::file_read(), Resource::new(tool_name)),
        FILE_TOOL_NAME => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            let reading = input.get("operation").and_then(Value::as_str) == Some("read");
            // A generic file tool aimed at the memory directory is a memory operation, not a
            // workspace one (`migrate-agent-memory-to-file-store`): it maps onto the same
            // action/resource pair as `remember` and `recall`, so correcting or retracting a memory
            // is auto-approved exactly as saving one already was. Paths outside keep whatever
            // approval they required before.
            if is_within_memory_directory(path) {
                let action = if reading {
                    Action::file_read()
                } else {
                    Action::memory_write()
                };
                return (action, Resource::memory());
            }
            let resource = Resource::file_path(path);
            if reading {
                (Action::file_read(), resource)
            } else {
                (Action::file_write(), resource)
            }
        }
        GREP_TOOL_NAME | GLOB_TOOL_NAME | SEARCH_CODE_TOOL_NAME => {
            (Action::file_read(), Resource::workspace())
        }
        EDIT_TOOL_NAME => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            if is_within_memory_directory(path) {
                return (Action::memory_write(), Resource::memory());
            }
            (Action::file_write(), Resource::file_path(path))
        }
        // Classified per operation, like the file tool: reading a notebook is a read, and the three
        // that rewrite it are writes against the same path.
        NOTEBOOK_TOOL_NAME => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            let resource = Resource::file_path(path);
            match input.get("operation").and_then(Value::as_str) {
                Some("read") => (Action::file_read(), resource),
                _ => (Action::file_write(), resource),
            }
        }
        FIND_DEFINITION_TOOL_NAME
        | FIND_REFERENCES_TOOL_NAME
        | GET_HOVER_TOOL_NAME
        | GET_DIAGNOSTICS_TOOL_NAME => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            (Action::file_read(), Resource::file_path(path))
        }
        REMEMBER_TOOL_NAME => (Action::memory_write(), Resource::memory()),
        RECALL_TOOL_NAME => (Action::file_read(), Resource::memory()),
        LIST_SKILLS_TOOL_NAME | LOAD_SKILL_TOOL_NAME | READ_SKILL_RESOURCE_TOOL_NAME => {
            (Action::file_read(), Resource::new(tool_name))
        }
        name if name.starts_with(MCP_TOOL_NAME_PREFIX) => (Action::mcp_tool(), Resource::new(name)),
        name => (Action::new(format!("unknown:{name}")), Resource::new(name)),
    }
}

/// Validates a question, publishes it, and blocks until the user answers.
///
/// Validation happens before anything is published, so a malformed call neither renders a card
/// nor blocks the generation. The non-interactive refusal is repeated here rather than left to the
/// catalog because the catalog only shapes what the model is *told* -- nothing stops it requesting
/// a tool it was never offered, and in an unattended attempt that request would hang until the
/// attempt's ceiling fired (`add-agent-user-question` D4).
#[allow(clippy::result_large_err)]
pub(super) fn ask_user_question(
    tool_use: &mut ToolUseBlock,
    input: &Value,
    interactive: bool,
    cancelled: &AtomicBool,
    pending_approvals: &PendingApprovals,
    sink: &dyn AgentProcessEventSink,
) -> Result<ToolExecutionOutcome, GenerationProcessEvent> {
    if !interactive {
        return Ok(ToolExecutionOutcome {
            output: "There is no interactive user in this execution context, so a question cannot \
                     be answered here. Decide using the information you have, state the assumption \
                     you made, and continue."
                .to_string(),
            is_error: true,
        });
    }
    if let Err(message) = validate_question_input(input) {
        return Ok(ToolExecutionOutcome {
            output: message,
            is_error: true,
        });
    }

    tool_use.status = "awaiting_input".to_string();
    if sink
        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
        .is_err()
    {
        return Err(failed_retryable("Agent generation event handling failed."));
    }
    match await_approval(&tool_use.id, cancelled, pending_approvals) {
        ApprovalOutcome::Answered(answer) => Ok(ToolExecutionOutcome {
            output: answer,
            is_error: false,
        }),
        ApprovalOutcome::Cancelled => Err(failed_non_retryable(
            "Generation was cancelled while a question was awaiting an answer.",
        )),
        // Approve/deny arriving for a question means the two resolution paths were crossed. There
        // is no answer to return, so the call fails rather than inventing one.
        ApprovalOutcome::Approved | ApprovalOutcome::Denied => Ok(ToolExecutionOutcome {
            output: "The question was dismissed without an answer.".to_string(),
            is_error: true,
        }),
    }
}

/// Blocks on the user's decision to leave plan mode. Shaped like `ask_user_question` -- publish,
/// wait, report -- but resolved as an approval rather than an answer, because an answer is a string
/// the model interprets and would leave every later generation still resolving the read-only
/// catalog (`add-agent-plan-exit-request` D1).
#[allow(clippy::result_large_err)]
pub(super) fn request_plan_exit(
    tool_use: &mut ToolUseBlock,
    input: &Value,
    interactive: bool,
    plan_mode: bool,
    cancelled: &AtomicBool,
    pending_approvals: &PendingApprovals,
    sink: &dyn AgentProcessEventSink,
) -> Result<ToolExecutionOutcome, GenerationProcessEvent> {
    // Reachable even though the catalog only offers this in plan mode: a model can name any tool,
    // and a stale turn can replay one. Outside plan mode there is nothing to leave.
    if !plan_mode {
        return Ok(ToolExecutionOutcome {
            output: "This session is not in plan mode, so there is nothing to leave. You already \
                     have your full tool set; continue with the work."
                .to_string(),
            is_error: true,
        });
    }
    if !interactive {
        return Ok(ToolExecutionOutcome {
            output:
                "There is no interactive user in this execution context, so no one can approve \
                     leaving plan mode. Finish the planning you were asked for and report it."
                    .to_string(),
            is_error: true,
        });
    }
    if let Err(message) = validate_plan_input(input) {
        return Ok(ToolExecutionOutcome {
            output: message,
            is_error: true,
        });
    }

    tool_use.status = "awaiting_input".to_string();
    if sink
        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
        .is_err()
    {
        return Err(failed_retryable("Agent generation event handling failed."));
    }
    match await_approval(&tool_use.id, cancelled, pending_approvals) {
        // The catalog for this generation was resolved before the call and is not re-resolved, so
        // the write tools are genuinely absent until the next turn -- say so rather than let the
        // model discover it by calling a tool it was never given (D3).
        ApprovalOutcome::Approved => Ok(ToolExecutionOutcome {
            output: "The user approved your plan and this session has left plan mode. \
                     Write-capable tools become available on your next turn, not this one, so end \
                     your turn now instead of trying to start the work here."
                .to_string(),
            is_error: false,
        }),
        ApprovalOutcome::Denied => Ok(ToolExecutionOutcome {
            output:
                "The user did not approve this plan. The session is still in plan mode. Revise \
                     the plan based on what they have told you rather than asking again unchanged."
                    .to_string(),
            is_error: true,
        }),
        ApprovalOutcome::Cancelled => Err(failed_non_retryable(
            "Generation was cancelled while a plan was awaiting approval.",
        )),
        // An answer arriving for an approval means the two resolution paths were crossed. There is
        // no decision to act on, so the call fails rather than inventing one.
        ApprovalOutcome::Answered(_) => Ok(ToolExecutionOutcome {
            output: "The plan approval was dismissed without a decision.".to_string(),
            is_error: true,
        }),
    }
}

fn validate_plan_input(input: &Value) -> Result<(), String> {
    let plan = input
        .get("plan")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if plan.is_empty() {
        return Err("plan must be a non-empty string describing what you will do.".to_string());
    }
    if plan.chars().count() > MAX_PLAN_CHARS {
        return Err(format!(
            "plan is {} characters; the maximum is {MAX_PLAN_CHARS}. Summarize it to what the user \
             needs in order to decide.",
            plan.chars().count()
        ));
    }
    Ok(())
}

pub(super) fn validate_question_input(input: &Value) -> Result<(), String> {
    let question = input
        .get("question")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if question.is_empty() {
        return Err("question must be a non-empty string.".to_string());
    }
    if question.chars().count() > MAX_QUESTION_CHARS {
        return Err(format!(
            "question is {} characters; the maximum is {MAX_QUESTION_CHARS}.",
            question.chars().count()
        ));
    }
    let Some(options) = input.get("options").and_then(Value::as_array) else {
        return Err("options must be an array of strings.".to_string());
    };
    if options.len() < MIN_QUESTION_OPTIONS || options.len() > MAX_QUESTION_OPTIONS {
        return Err(format!(
            "options must contain between {MIN_QUESTION_OPTIONS} and {MAX_QUESTION_OPTIONS} entries, but {} were given.",
            options.len()
        ));
    }
    for (index, option) in options.iter().enumerate() {
        let Some(text) = option.as_str() else {
            return Err(format!("option {} must be a string.", index + 1));
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(format!("option {} is empty.", index + 1));
        }
        if trimmed.chars().count() > MAX_QUESTION_OPTION_CHARS {
            return Err(format!(
                "option {} is {} characters; the maximum is {MAX_QUESTION_OPTION_CHARS}.",
                index + 1,
                trimmed.chars().count()
            ));
        }
    }
    Ok(())
}

pub(super) enum ApprovalOutcome {
    Approved,
    Denied,
    Cancelled,
    /// A question resolved with the user's answer. Reaching this from the approval gate would mean
    /// an answer was delivered to a call that asked for permission, so that path treats it as a
    /// denial rather than silently proceeding (`add-agent-user-question` D1).
    Answered(String),
}

/// The three things the permission gate can conclude about one tool call.
// `large_enum_variant`: `Failed` carries the terminal `GenerationProcessEvent` unboxed, on purpose.
// Boxing it would be the only heap allocation this decomposition introduced, on the one path that
// exists to hand the event back to the caller unchanged.
#[allow(clippy::large_enum_variant)]
pub(super) enum ToolAuthorization {
    Allowed,
    /// The call was refused with this text as its output. The caller records it as a failed call
    /// and moves to the next one — a refusal is data the model sees, not an error.
    Denied(String),
    /// The whole generation ends with this event.
    Failed(GenerationProcessEvent),
}

/// Evaluates one tool call against policy and, when policy asks, blocks until the user decides.
///
/// The `tool_use` block is mutated in place on the paths that emit it — `awaiting_approval` while
/// the prompt is open, then `failed` with the denial text — so the caller's copy carries the same
/// state it did when this ran inline.
#[allow(clippy::too_many_arguments)]
pub(super) fn authorize_tool_call(
    tool_use: &mut ToolUseBlock,
    input: &Value,
    agent_id: &str,
    request: &GenerationProcessRequest,
    permissions: &dyn AgentPermissionPort,
    pending_approvals: &PendingApprovals,
    sink: &dyn AgentProcessEventSink,
    cancelled: &AtomicBool,
) -> ToolAuthorization {
    let (permission_action, permission_resource) =
        permission_action_and_resource(&tool_use.name, input);
    let project_key = request.session.folder.as_deref().unwrap_or("");
    let effect = permissions.evaluate(
        agent_id,
        permission_action.clone(),
        permission_resource.clone(),
        &request.session.id,
        &request.operation_id,
        project_key,
    );
    match effect {
        Effect::Allow => {}
        Effect::Deny => {
            let denial = "Denied by policy.".to_string();
            tool_use.status = "failed".to_string();
            tool_use.output = Some(Value::String(denial.clone()));
            if sink
                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                .is_err()
            {
                return ToolAuthorization::Failed(failed_retryable(
                    "Agent generation event handling failed.",
                ));
            }
            return ToolAuthorization::Denied(denial);
        }
        Effect::Ask => {
            tool_use.status = "awaiting_approval".to_string();
            if sink
                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                .is_err()
            {
                return ToolAuthorization::Failed(failed_retryable(
                    "Agent generation event handling failed.",
                ));
            }
            if let Err(error) = permissions.create_pending_approval(
                agent_id,
                permission_action,
                permission_resource,
                &request.session.id,
                &request.operation_id,
                &tool_use.id,
                project_key,
            ) {
                return ToolAuthorization::Failed(failed_non_retryable(&error.to_string()));
            }
            match await_approval(&tool_use.id, cancelled, pending_approvals) {
                ApprovalOutcome::Approved => {}
                ApprovalOutcome::Denied => {
                    let denial = "Denied by user.".to_string();
                    tool_use.status = "failed".to_string();
                    tool_use.output = Some(Value::String(denial.clone()));
                    if sink
                        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                        .is_err()
                    {
                        return ToolAuthorization::Failed(failed_retryable(
                            "Agent generation event handling failed.",
                        ));
                    }
                    return ToolAuthorization::Denied(denial);
                }
                ApprovalOutcome::Cancelled => {
                    return ToolAuthorization::Failed(failed_non_retryable(
                        "Generation was cancelled while a tool call was awaiting approval.",
                    ));
                }
                // An answer delivered to a call that asked for permission means the two
                // resolutions were crossed; fail closed rather than treat it as consent.
                ApprovalOutcome::Answered(_) => {
                    let denial = "Denied by user.".to_string();
                    tool_use.status = "failed".to_string();
                    tool_use.output = Some(Value::String(denial.clone()));
                    if sink
                        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                        .is_err()
                    {
                        return ToolAuthorization::Failed(failed_retryable(
                            "Agent generation event handling failed.",
                        ));
                    }
                    return ToolAuthorization::Denied(denial);
                }
            }
        }
    }
    ToolAuthorization::Allowed
}

pub(super) fn await_approval(
    call_id: &str,
    cancelled: &AtomicBool,
    pending_approvals: &PendingApprovals,
) -> ApprovalOutcome {
    let rx = {
        let (tx, rx) = mpsc::channel();
        match pending_approvals.lock() {
            Ok(mut pending) => {
                pending.insert(call_id.to_string(), tx);
            }
            Err(_) => return ApprovalOutcome::Cancelled,
        }
        rx
    };
    loop {
        if cancelled.load(Ordering::SeqCst) {
            if let Ok(mut pending) = pending_approvals.lock() {
                pending.remove(call_id);
            }
            return ApprovalOutcome::Cancelled;
        }
        match rx.recv_timeout(APPROVAL_POLL_INTERVAL) {
            Ok(ToolApprovalDecision::Approved) => return ApprovalOutcome::Approved,
            Ok(ToolApprovalDecision::Denied) => return ApprovalOutcome::Denied,
            Ok(ToolApprovalDecision::Answered(answer)) => return ApprovalOutcome::Answered(answer),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => return ApprovalOutcome::Cancelled,
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// Formats the fixed rejection message used for every plan-mode enforcement gate below — the
/// same message shape regardless of which tool/operation was denied.
pub(super) fn plan_mode_denial(what: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: format!("{what} is disabled in plan mode."),
        is_error: true,
    }
}
