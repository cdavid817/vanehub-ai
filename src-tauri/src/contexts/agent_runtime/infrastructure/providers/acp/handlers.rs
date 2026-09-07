//! Agent-originated requests, decided by policy.
//!
//! Every method the host advertises is handled here; every other method with an id is answered
//! with `method not found`. A handler either replies now or defers: a deferred request becomes a
//! backend-owned pending interaction and is answered when the user (or the deadline, or a
//! cancellation) decides. Nothing here executes an instruction carried inside a payload -- a
//! plan, a question, or a tool title is untrusted content and is shown, never obeyed.

use super::definitions::AcpLaunchGrammar;
use super::interactions::{
    select_permission_option, InteractionKind, PermissionOption, PermissionOptionKind,
};
use super::jsonrpc::RpcError;
use super::proxy_fs::{read_text_file, write_text_file, AuthorizedRoots};
use super::proxy_terminal::{TerminalCreateRequest, TerminalOwner, TerminalRegistry};
use crate::contexts::permissions::api::Effect;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// How the tool call the agent asks about maps onto the permission model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PermissionSubject {
    pub(crate) action: &'static str,
    pub(crate) resource: String,
    pub(crate) tool_call_id: String,
    pub(crate) title: String,
    pub(crate) kind: String,
}

/// The tool kinds ACP defines, classified by whether they can change anything.
fn action_for_tool_kind(kind: &str) -> &'static str {
    match kind {
        "read" | "search" | "think" => "file.read",
        "edit" | "delete" | "move" => "file.write",
        // `execute` runs a command; `fetch` reaches the network; anything unknown is treated as
        // the most consequential thing it could be rather than the least.
        _ => "shell.exec",
    }
}

pub(crate) fn permission_subject(params: &Value) -> Result<PermissionSubject, RpcError> {
    let tool_call = params
        .get("toolCall")
        .ok_or_else(|| RpcError::invalid_params("toolCall is required"))?;
    let tool_call_id = tool_call
        .get("toolCallId")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .ok_or_else(|| RpcError::invalid_params("toolCall.toolCallId is required"))?;
    let kind = tool_call
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("other")
        .to_string();
    let resource = tool_call
        .get("locations")
        .and_then(Value::as_array)
        .and_then(|locations| locations.first())
        .and_then(|location| location.get("path"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| "workspace".to_string());
    Ok(PermissionSubject {
        action: action_for_tool_kind(&kind),
        resource,
        tool_call_id: tool_call_id.to_string(),
        title: tool_call
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("tool call")
            .to_string(),
        kind,
    })
}

pub(crate) fn permission_options(params: &Value) -> Result<Vec<PermissionOption>, RpcError> {
    let options = params
        .get("options")
        .and_then(Value::as_array)
        .ok_or_else(|| RpcError::invalid_params("options are required"))?;
    let mut parsed = Vec::new();
    for option in options {
        let option_id = option
            .get("optionId")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| RpcError::invalid_params("options[].optionId is required"))?;
        parsed.push(PermissionOption {
            option_id: option_id.to_string(),
            name: option
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(option_id)
                .to_string(),
            kind: PermissionOptionKind::parse(
                option.get("kind").and_then(Value::as_str).unwrap_or(""),
            ),
        });
    }
    if parsed.is_empty() {
        return Err(RpcError::invalid_params("at least one option is required"));
    }
    Ok(parsed)
}

pub(crate) fn permission_reply(options: &[PermissionOption], approve: bool) -> Value {
    match select_permission_option(options, approve) {
        Some(option) => json!({"outcome": {"outcome": "selected", "optionId": option.option_id}}),
        // The agent offered nothing with the decided scope: answering cancelled is the only reply
        // that neither widens an approval nor invents a rejection option.
        None => cancelled_outcome(),
    }
}

pub(crate) fn cancelled_outcome() -> Value {
    json!({"outcome": {"outcome": "cancelled"}})
}

/// What a handler decided.
#[derive(Debug, PartialEq)]
pub(crate) enum HandlerOutcome {
    Reply(Result<Value, RpcError>),
    /// Answer later, once a person decides. The interaction is registered by the caller.
    Defer {
        kind: InteractionKind,
        ui: DeferredUi,
        deadline: Option<Duration>,
    },
}

/// How a deferred request is shown.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DeferredUi {
    Approval {
        tool_call_id: String,
        tool_name: String,
        action: &'static str,
        resource: String,
        input: Value,
    },
    Question {
        tool_call_id: String,
        tool_name: String,
        question: String,
        options: Vec<String>,
    },
}

/// The pieces of a binding a handler consults. Borrowed rather than owned so the driver keeps
/// ownership of the connection state.
pub(crate) struct HandlerContext<'a> {
    pub(crate) session_id: &'a str,
    pub(crate) external_session_id: &'a str,
    pub(crate) epoch: u64,
    pub(crate) grammar: &'a AcpLaunchGrammar,
    pub(crate) roots: &'a AuthorizedRoots,
    pub(crate) terminals: &'a TerminalRegistry,
    pub(crate) workspace: &'a str,
    /// Whether a human can answer. An unattended run rejects rather than waits.
    pub(crate) interactive: bool,
    pub(crate) advertised_fs: bool,
    pub(crate) advertised_terminal: bool,
    pub(crate) child_environment: &'a BTreeMap<String, String>,
    /// The turn's cancel flag. A blocking proxy wait polls it so a cancel is not held behind a
    /// command the agent chose to wait on.
    pub(crate) cancel: &'a AtomicBool,
}

/// Decides one inbound request. `evaluate` is the permission decision point.
pub(crate) fn handle_request(
    context: &HandlerContext<'_>,
    method: &str,
    params: &Value,
    evaluate: &dyn Fn(&str, &str) -> Effect,
) -> HandlerOutcome {
    if let Some(session_id) = params.get("sessionId").and_then(Value::as_str) {
        if session_id != context.external_session_id {
            return HandlerOutcome::Reply(Err(RpcError::refused(
                "request names a session this connection does not own",
            )));
        }
    }
    match method {
        "session/request_permission" => handle_permission(context, params, evaluate),
        "fs/read_text_file" if context.advertised_fs => handle_read(context, params, evaluate),
        "fs/write_text_file" if context.advertised_fs => handle_write(context, params, evaluate),
        "terminal/create" if context.advertised_terminal => {
            handle_terminal_create(context, params, evaluate)
        }
        "terminal/output" | "terminal/wait_for_exit" | "terminal/kill" | "terminal/release"
            if context.advertised_terminal =>
        {
            handle_terminal_lifecycle(context, method, params)
        }
        "fs/read_text_file"
        | "fs/write_text_file"
        | "terminal/create"
        | "terminal/output"
        | "terminal/wait_for_exit"
        | "terminal/kill"
        | "terminal/release" => {
            // Not advertised at initialize. A capability is never opened because the peer
            // asked for it after the fact.
            HandlerOutcome::Reply(Err(RpcError::method_not_found(method)))
        }
        "cursor/ask_question" if context.grammar.blocking_extensions.contains(&method) => {
            handle_cursor_question(context, params)
        }
        "cursor/create_plan" if context.grammar.blocking_extensions.contains(&method) => {
            handle_cursor_plan(context, params)
        }
        _ => HandlerOutcome::Reply(Err(RpcError::method_not_found(method))),
    }
}

fn handle_permission(
    context: &HandlerContext<'_>,
    params: &Value,
    evaluate: &dyn Fn(&str, &str) -> Effect,
) -> HandlerOutcome {
    let subject = match permission_subject(params) {
        Ok(subject) => subject,
        Err(error) => return HandlerOutcome::Reply(Err(error)),
    };
    let options = match permission_options(params) {
        Ok(options) => options,
        Err(error) => return HandlerOutcome::Reply(Err(error)),
    };
    match evaluate(subject.action, &subject.resource) {
        Effect::Allow => HandlerOutcome::Reply(Ok(permission_reply(&options, true))),
        Effect::Deny => HandlerOutcome::Reply(Ok(permission_reply(&options, false))),
        Effect::Ask if !context.interactive => {
            // Unattended: nobody can answer, and waiting would only burn the deadline. The
            // documented outcome is rejection, recorded by the driver as needs-intervention.
            HandlerOutcome::Reply(Ok(permission_reply(&options, false)))
        }
        Effect::Ask => HandlerOutcome::Defer {
            ui: DeferredUi::Approval {
                tool_call_id: subject.tool_call_id.clone(),
                tool_name: subject.title.clone(),
                action: subject.action,
                resource: subject.resource.clone(),
                input: json!({
                    "kind": subject.kind,
                    "title": subject.title,
                    "resource": subject.resource,
                    "rawInput": params.get("toolCall").and_then(|call| call.get("rawInput")).cloned().unwrap_or(Value::Null),
                }),
            },
            kind: InteractionKind::Permission {
                tool_call_id: subject.tool_call_id,
                options,
                action: subject.action.to_string(),
                resource: subject.resource,
            },
            deadline: None,
        },
    }
}

fn handle_read(
    context: &HandlerContext<'_>,
    params: &Value,
    evaluate: &dyn Fn(&str, &str) -> Effect,
) -> HandlerOutcome {
    let Some(path) = params.get("path").and_then(Value::as_str) else {
        return HandlerOutcome::Reply(Err(RpcError::invalid_params("path is required")));
    };
    if evaluate("file.read", path) == Effect::Deny {
        return HandlerOutcome::Reply(Err(RpcError::refused("read denied by policy")));
    }
    let line = params.get("line").and_then(Value::as_u64);
    let limit = params.get("limit").and_then(Value::as_u64);
    HandlerOutcome::Reply(
        read_text_file(context.roots, path, line, limit)
            .map(|content| json!({"content": content}))
            .map_err(|error| RpcError::refused(error.message())),
    )
}

fn handle_write(
    context: &HandlerContext<'_>,
    params: &Value,
    evaluate: &dyn Fn(&str, &str) -> Effect,
) -> HandlerOutcome {
    let (Some(path), Some(content)) = (
        params.get("path").and_then(Value::as_str),
        params.get("content").and_then(Value::as_str),
    ) else {
        return HandlerOutcome::Reply(Err(RpcError::invalid_params(
            "path and content are required",
        )));
    };
    // Validate the path before asking anyone: an escape is refused outright, not offered.
    if let Err(error) = context.roots.resolve(path) {
        return HandlerOutcome::Reply(Err(RpcError::refused(error.message())));
    }
    match evaluate("file.write", path) {
        Effect::Allow => HandlerOutcome::Reply(apply_write(context.roots, path, content)),
        Effect::Deny => HandlerOutcome::Reply(Err(RpcError::refused("write denied by policy"))),
        Effect::Ask if !context.interactive => {
            HandlerOutcome::Reply(Err(RpcError::refused("write requires a human approval")))
        }
        Effect::Ask => HandlerOutcome::Defer {
            ui: DeferredUi::Approval {
                tool_call_id: format!("acp-fs-write-{}", stable_token(path)),
                tool_name: "fs/write_text_file".to_string(),
                action: "file.write",
                resource: path.to_string(),
                input: json!({"path": path, "bytes": content.len()}),
            },
            kind: InteractionKind::FileWrite {
                path: path.to_string(),
                content: content.to_string(),
            },
            deadline: None,
        },
    }
}

pub(crate) fn apply_write(
    roots: &AuthorizedRoots,
    path: &str,
    content: &str,
) -> Result<Value, RpcError> {
    write_text_file(roots, path, content)
        .map(|()| Value::Null)
        .map_err(|error| RpcError::refused(error.message()))
}

fn terminal_request(
    context: &HandlerContext<'_>,
    params: &Value,
) -> Result<TerminalCreateRequest, RpcError> {
    let command = params
        .get("command")
        .and_then(Value::as_str)
        .filter(|command| !command.trim().is_empty())
        .ok_or_else(|| RpcError::invalid_params("command is required"))?;
    let args = params
        .get("args")
        .and_then(Value::as_array)
        .map(|args| {
            args.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let cwd = match params.get("cwd").and_then(Value::as_str) {
        Some(cwd) => context
            .roots
            .resolve(cwd)
            .map_err(|error| RpcError::refused(error.message()))?
            .to_string_lossy()
            .to_string(),
        None => context.workspace.to_string(),
    };
    // The agent may add variables but never replace the scrubbed base environment.
    let mut environment = context.child_environment.clone();
    if let Some(entries) = params.get("env").and_then(Value::as_array) {
        for entry in entries {
            if let (Some(name), Some(value)) = (
                entry.get("name").and_then(Value::as_str),
                entry.get("value").and_then(Value::as_str),
            ) {
                environment.insert(name.to_string(), value.to_string());
            }
        }
    }
    Ok(TerminalCreateRequest {
        command: command.to_string(),
        args,
        environment,
        cwd,
        output_byte_limit: params
            .get("outputByteLimit")
            .and_then(Value::as_u64)
            .map(|limit| usize::try_from(limit).unwrap_or(usize::MAX)),
    })
}

fn handle_terminal_create(
    context: &HandlerContext<'_>,
    params: &Value,
    evaluate: &dyn Fn(&str, &str) -> Effect,
) -> HandlerOutcome {
    let request = match terminal_request(context, params) {
        Ok(request) => request,
        Err(error) => return HandlerOutcome::Reply(Err(error)),
    };
    match evaluate("shell.exec", "workspace") {
        Effect::Allow => HandlerOutcome::Reply(apply_terminal_create(context, &request)),
        Effect::Deny => {
            HandlerOutcome::Reply(Err(RpcError::refused("command execution denied by policy")))
        }
        Effect::Ask if !context.interactive => HandlerOutcome::Reply(Err(RpcError::refused(
            "command execution requires a human approval",
        ))),
        Effect::Ask => HandlerOutcome::Defer {
            ui: DeferredUi::Approval {
                tool_call_id: format!("acp-terminal-{}", stable_token(&request.command)),
                tool_name: "terminal/create".to_string(),
                action: "shell.exec",
                resource: "workspace".to_string(),
                input: json!({"command": request.command, "args": request.args, "cwd": request.cwd}),
            },
            kind: InteractionKind::TerminalCreate {
                request: serde_json::to_value(TerminalCreateRecord::from(&request))
                    .unwrap_or(Value::Null),
            },
            deadline: None,
        },
    }
}

/// The serializable form of a terminal request held in a pending interaction.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct TerminalCreateRecord {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) cwd: String,
    pub(crate) output_byte_limit: Option<usize>,
}

impl From<&TerminalCreateRequest> for TerminalCreateRecord {
    fn from(request: &TerminalCreateRequest) -> Self {
        Self {
            command: request.command.clone(),
            args: request.args.clone(),
            environment: request.environment.clone(),
            cwd: request.cwd.clone(),
            output_byte_limit: request.output_byte_limit,
        }
    }
}

impl From<TerminalCreateRecord> for TerminalCreateRequest {
    fn from(record: TerminalCreateRecord) -> Self {
        Self {
            command: record.command,
            args: record.args,
            environment: record.environment,
            cwd: record.cwd,
            output_byte_limit: record.output_byte_limit,
        }
    }
}

pub(crate) fn apply_terminal_create(
    context: &HandlerContext<'_>,
    request: &TerminalCreateRequest,
) -> Result<Value, RpcError> {
    context
        .terminals
        .create(
            TerminalOwner {
                epoch: context.epoch,
                session_id: context.session_id,
            },
            request,
        )
        .map(|terminal_id| json!({"terminalId": terminal_id}))
        .map_err(|error| RpcError::refused(error.message()))
}

fn handle_terminal_lifecycle(
    context: &HandlerContext<'_>,
    method: &str,
    params: &Value,
) -> HandlerOutcome {
    let Some(terminal_id) = params.get("terminalId").and_then(Value::as_str) else {
        return HandlerOutcome::Reply(Err(RpcError::invalid_params("terminalId is required")));
    };
    let owner = TerminalOwner {
        epoch: context.epoch,
        session_id: context.session_id,
    };
    let reply = match method {
        "terminal/output" => context.terminals.output(owner, terminal_id).map(|output| {
            let mut document = json!({"output": output.output, "truncated": output.truncated});
            if let Some(status) = output.exit_status {
                document["exitStatus"] =
                    json!({"exitCode": status.exit_code, "signal": status.signal});
            }
            document
        }),
        // Bounded so the driver stays responsive, and abandoned on cancel so the turn can end;
        // the agent may ask again.
        "terminal/wait_for_exit" => context
            .terminals
            .wait_for_exit(owner, terminal_id, Duration::from_secs(60), &|| {
                context.cancel.load(Ordering::SeqCst)
            })
            .map(|status| match status {
                Some(status) => json!({"exitCode": status.exit_code, "signal": status.signal}),
                None => json!({"exitCode": null, "signal": "timeout"}),
            }),
        "terminal/kill" => context
            .terminals
            .kill(owner, terminal_id)
            .map(|()| Value::Null),
        "terminal/release" => context
            .terminals
            .release(owner, terminal_id)
            .map(|()| Value::Null),
        _ => return HandlerOutcome::Reply(Err(RpcError::method_not_found(method))),
    };
    HandlerOutcome::Reply(reply.map_err(|error| RpcError::refused(error.message())))
}

fn handle_cursor_question(context: &HandlerContext<'_>, params: &Value) -> HandlerOutcome {
    let Some(tool_call_id) = params.get("toolCallId").and_then(Value::as_str) else {
        return HandlerOutcome::Reply(Err(RpcError::invalid_params("toolCallId is required")));
    };
    let questions: Vec<&Value> = params
        .get("questions")
        .and_then(Value::as_array)
        .map(|questions| questions.iter().collect())
        .unwrap_or_default();
    let Some(first) = questions.first() else {
        return HandlerOutcome::Reply(Err(RpcError::invalid_params("questions must not be empty")));
    };
    if !context.interactive {
        return HandlerOutcome::Reply(Ok(json!({"outcome": "skipped"})));
    }
    let question_ids: Vec<String> = questions
        .iter()
        .filter_map(|question| question.get("id").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    let prompt = first
        .get("prompt")
        .and_then(Value::as_str)
        .unwrap_or("The agent has a question.")
        .to_string();
    let options: Vec<String> = first
        .get("options")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    option.as_str().map(str::to_string).or_else(|| {
                        option
                            .get("label")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    HandlerOutcome::Defer {
        ui: DeferredUi::Question {
            tool_call_id: tool_call_id.to_string(),
            tool_name: "cursor/ask_question".to_string(),
            question: prompt,
            options,
        },
        kind: InteractionKind::Question {
            tool_call_id: tool_call_id.to_string(),
            question_ids,
        },
        deadline: None,
    }
}

fn handle_cursor_plan(context: &HandlerContext<'_>, params: &Value) -> HandlerOutcome {
    let Some(tool_call_id) = params.get("toolCallId").and_then(Value::as_str) else {
        return HandlerOutcome::Reply(Err(RpcError::invalid_params("toolCallId is required")));
    };
    if !context.interactive {
        return HandlerOutcome::Reply(Ok(json!({"outcome": "cancelled"})));
    }
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Plan")
        .to_string();
    HandlerOutcome::Defer {
        ui: DeferredUi::Question {
            tool_call_id: tool_call_id.to_string(),
            tool_name: "cursor/create_plan".to_string(),
            question: format!("Accept the proposed plan \"{name}\"?"),
            options: vec!["accept".to_string(), "reject".to_string()],
        },
        kind: InteractionKind::Plan {
            tool_call_id: tool_call_id.to_string(),
        },
        deadline: None,
    }
}

/// The reply for a resolved question or plan. Answers are keyed by question id; a free-text
/// answer applies to every question the agent asked, which is the only mapping available when
/// the card shows one prompt.
pub(crate) fn question_reply(question_ids: &[String], answer: &str) -> Value {
    let answers: serde_json::Map<String, Value> = question_ids
        .iter()
        .map(|id| (id.clone(), json!([answer])))
        .collect();
    json!({"outcome": "answered", "answers": answers})
}

pub(crate) fn plan_reply(answer: &str) -> Value {
    let accepted = matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "accept" | "accepted" | "yes" | "approve" | "approved" | "接受" | "同意"
    );
    json!({"outcome": if accepted { "accepted" } else { "rejected" }})
}

fn stable_token(input: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn grammar() -> AcpLaunchGrammar {
        super::super::definitions::definition("cursor-agent-cli")
            .and_then(|definition| definition.acp)
            .expect("cursor grammar")
    }

    fn context<'a>(
        workspace: &'a str,
        roots: &'a AuthorizedRoots,
        terminals: &'a TerminalRegistry,
        grammar: &'a AcpLaunchGrammar,
        environment: &'a BTreeMap<String, String>,
        interactive: bool,
    ) -> HandlerContext<'a> {
        HandlerContext {
            session_id: "session-1",
            external_session_id: "ext-1",
            epoch: 5,
            grammar,
            roots,
            terminals,
            workspace,
            interactive,
            advertised_fs: true,
            advertised_terminal: true,
            child_environment: environment,
            cancel: &NEVER_CANCELLED,
        }
    }

    static NEVER_CANCELLED: AtomicBool = AtomicBool::new(false);

    fn permission_params(kind: &str) -> Value {
        json!({
            "sessionId": "ext-1",
            "toolCall": {"toolCallId": "call-9", "title": "Write file", "kind": kind, "locations": [{"path": "/w/a.txt"}], "rawInput": {"path": "/w/a.txt"}},
            "options": [
                {"optionId": "allow-always", "name": "Always", "kind": "allow_always"},
                {"optionId": "allow-once", "name": "Once", "kind": "allow_once"},
                {"optionId": "reject-once", "name": "No", "kind": "reject_once"}
            ]
        })
    }

    #[test]
    fn permission_requests_follow_policy_and_never_widen_scope() {
        let workspace = TempDirectory::new("acp-handlers");
        let roots = AuthorizedRoots::new([workspace.path().to_path_buf()]);
        let terminals = TerminalRegistry::default();
        let grammar = grammar();
        let environment = BTreeMap::new();
        let workspace_text = workspace.path().to_string_lossy().to_string();
        let context = context(
            &workspace_text,
            &roots,
            &terminals,
            &grammar,
            &environment,
            true,
        );

        let allow = handle_request(
            &context,
            "session/request_permission",
            &permission_params("edit"),
            &|_, _| Effect::Allow,
        );
        assert_eq!(
            allow,
            HandlerOutcome::Reply(Ok(
                json!({"outcome": {"outcome": "selected", "optionId": "allow-once"}})
            ))
        );
        let deny = handle_request(
            &context,
            "session/request_permission",
            &permission_params("execute"),
            &|action, _| {
                assert_eq!(action, "shell.exec");
                Effect::Deny
            },
        );
        assert_eq!(
            deny,
            HandlerOutcome::Reply(Ok(
                json!({"outcome": {"outcome": "selected", "optionId": "reject-once"}})
            ))
        );
        let ask = handle_request(
            &context,
            "session/request_permission",
            &permission_params("delete"),
            &|action, resource| {
                assert_eq!((action, resource), ("file.write", "/w/a.txt"));
                Effect::Ask
            },
        );
        match ask {
            HandlerOutcome::Defer {
                kind:
                    InteractionKind::Permission {
                        tool_call_id,
                        options,
                        action,
                        resource,
                    },
                ui: DeferredUi::Approval { tool_name, .. },
                ..
            } => {
                assert_eq!(tool_call_id, "call-9");
                assert_eq!(options.len(), 3);
                assert_eq!(action, "file.write");
                assert_eq!(resource, "/w/a.txt");
                assert_eq!(tool_name, "Write file");
            }
            other => panic!("unexpected {other:?}"),
        }
        // Only allow_always offered: an approval must not be widened into it.
        let only_always = json!({"sessionId":"ext-1","toolCall":{"toolCallId":"c","kind":"read"},"options":[{"optionId":"a","name":"Always","kind":"allow_always"}]});
        assert_eq!(
            handle_request(
                &context,
                "session/request_permission",
                &only_always,
                &|_, _| Effect::Allow
            ),
            HandlerOutcome::Reply(Ok(cancelled_outcome()))
        );
        // Wrong session id is refused before policy is consulted.
        let foreign = json!({"sessionId":"someone-else","toolCall":{"toolCallId":"c","kind":"read"},"options":[{"optionId":"a","name":"A","kind":"allow_once"}]});
        assert!(matches!(
            handle_request(&context, "session/request_permission", &foreign, &|_, _| panic!("not consulted")),
            HandlerOutcome::Reply(Err(error)) if error.code == super::super::jsonrpc::HOST_REFUSED
        ));
    }

    #[test]
    fn unattended_runs_reject_instead_of_waiting() {
        let workspace = TempDirectory::new("acp-handlers-unattended");
        let roots = AuthorizedRoots::new([workspace.path().to_path_buf()]);
        let terminals = TerminalRegistry::default();
        let grammar = grammar();
        let environment = BTreeMap::new();
        let workspace_text = workspace.path().to_string_lossy().to_string();
        let context = context(
            &workspace_text,
            &roots,
            &terminals,
            &grammar,
            &environment,
            false,
        );
        assert_eq!(
            handle_request(
                &context,
                "session/request_permission",
                &permission_params("edit"),
                &|_, _| Effect::Ask
            ),
            HandlerOutcome::Reply(Ok(
                json!({"outcome": {"outcome": "selected", "optionId": "reject-once"}})
            ))
        );
        let question = json!({"toolCallId":"q1","questions":[{"id":"q","prompt":"Which?","options":["a","b"]}]});
        assert_eq!(
            handle_request(&context, "cursor/ask_question", &question, &|_, _| {
                Effect::Ask
            }),
            HandlerOutcome::Reply(Ok(json!({"outcome": "skipped"})))
        );
        let plan = json!({"toolCallId":"p1","name":"Refactor","plan":"..."});
        assert_eq!(
            handle_request(&context, "cursor/create_plan", &plan, &|_, _| Effect::Ask),
            HandlerOutcome::Reply(Ok(json!({"outcome": "cancelled"})))
        );
        let target = workspace.path().join("unattended.txt");
        assert!(matches!(
            handle_request(
                &context,
                "fs/write_text_file",
                &json!({"path": target.to_string_lossy(), "content": "x"}),
                &|_, _| Effect::Ask
            ),
            HandlerOutcome::Reply(Err(_))
        ));
        assert!(!target.exists());
    }

    #[test]
    fn file_proxy_is_policy_gated_and_root_bound() {
        let workspace = TempDirectory::new("acp-handlers-fs");
        let outside = TempDirectory::new("acp-handlers-fs-outside");
        let roots = AuthorizedRoots::new([workspace.path().to_path_buf()]);
        let terminals = TerminalRegistry::default();
        let grammar = grammar();
        let environment = BTreeMap::new();
        let workspace_text = workspace.path().to_string_lossy().to_string();
        let context = context(
            &workspace_text,
            &roots,
            &terminals,
            &grammar,
            &environment,
            true,
        );
        let inside = workspace.path().join("note.txt");
        let inside_text = inside.to_string_lossy().to_string();

        assert_eq!(
            handle_request(
                &context,
                "fs/write_text_file",
                &json!({"path": inside_text, "content": "你好"}),
                &|_, _| Effect::Allow
            ),
            HandlerOutcome::Reply(Ok(Value::Null))
        );
        assert_eq!(std::fs::read_to_string(&inside).expect("written"), "你好");
        assert_eq!(
            handle_request(
                &context,
                "fs/read_text_file",
                &json!({"path": inside_text}),
                &|_, _| Effect::Allow
            ),
            HandlerOutcome::Reply(Ok(json!({"content": "你好"})))
        );
        // Deny leaves the file untouched.
        assert!(matches!(
            handle_request(
                &context,
                "fs/write_text_file",
                &json!({"path": inside_text, "content": "changed"}),
                &|_, _| Effect::Deny
            ),
            HandlerOutcome::Reply(Err(_))
        ));
        assert_eq!(std::fs::read_to_string(&inside).expect("unchanged"), "你好");
        // Ask defers with the content held for later.
        match handle_request(
            &context,
            "fs/write_text_file",
            &json!({"path": inside_text, "content": "later"}),
            &|_, _| Effect::Ask,
        ) {
            HandlerOutcome::Defer {
                kind: InteractionKind::FileWrite { path, content },
                ..
            } => {
                assert_eq!(path, inside_text);
                assert_eq!(content, "later");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(
            std::fs::read_to_string(&inside).expect("still unchanged"),
            "你好"
        );
        // Outside the roots: refused even under Allow, and never asked about.
        let escape = outside
            .path()
            .join("victim.txt")
            .to_string_lossy()
            .to_string();
        assert!(matches!(
            handle_request(
                &context,
                "fs/write_text_file",
                &json!({"path": escape, "content": "x"}),
                &|_, _| Effect::Allow
            ),
            HandlerOutcome::Reply(Err(_))
        ));
        assert!(!outside.path().join("victim.txt").exists());
    }

    #[test]
    fn unadvertised_and_unknown_methods_are_refused_promptly() {
        let workspace = TempDirectory::new("acp-handlers-unknown");
        let roots = AuthorizedRoots::new([workspace.path().to_path_buf()]);
        let terminals = TerminalRegistry::default();
        let mut grammar = grammar();
        grammar.blocking_extensions = &[];
        let environment = BTreeMap::new();
        let workspace_text = workspace.path().to_string_lossy().to_string();
        let mut context = context(
            &workspace_text,
            &roots,
            &terminals,
            &grammar,
            &environment,
            true,
        );
        context.advertised_fs = false;
        context.advertised_terminal = false;
        for method in [
            "fs/read_text_file",
            "terminal/create",
            "terminal/output",
            "cursor/ask_question",
            "vendor/execute_instruction",
        ] {
            let outcome = handle_request(
                &context,
                method,
                &json!({"path": "/x", "toolCallId": "t", "questions": [{"id":"q","prompt":"?"}], "instruction": "rm -rf /"}),
                &|_, _| Effect::Allow,
            );
            assert!(
                matches!(outcome, HandlerOutcome::Reply(Err(error)) if error.code == super::super::jsonrpc::METHOD_NOT_FOUND),
                "{method}"
            );
        }
    }

    #[test]
    fn cursor_extensions_defer_with_validated_shapes() {
        let workspace = TempDirectory::new("acp-handlers-cursor");
        let roots = AuthorizedRoots::new([workspace.path().to_path_buf()]);
        let terminals = TerminalRegistry::default();
        let grammar = grammar();
        let environment = BTreeMap::new();
        let workspace_text = workspace.path().to_string_lossy().to_string();
        let context = context(
            &workspace_text,
            &roots,
            &terminals,
            &grammar,
            &environment,
            true,
        );
        let question = json!({"toolCallId":"q1","title":"Choose","questions":[{"id":"q-a","prompt":"Which framework?","options":[{"label":"React"},"Vue"]}]});
        match handle_request(&context, "cursor/ask_question", &question, &|_, _| {
            Effect::Ask
        }) {
            HandlerOutcome::Defer {
                kind:
                    InteractionKind::Question {
                        tool_call_id,
                        question_ids,
                    },
                ui:
                    DeferredUi::Question {
                        question, options, ..
                    },
                ..
            } => {
                assert_eq!(tool_call_id, "q1");
                assert_eq!(question_ids, vec!["q-a".to_string()]);
                assert_eq!(question, "Which framework?");
                assert_eq!(options, vec!["React".to_string(), "Vue".to_string()]);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            handle_request(
                &context,
                "cursor/ask_question",
                &json!({"toolCallId":"q2","questions":[]}),
                &|_, _| Effect::Ask
            ),
            HandlerOutcome::Reply(Err(_))
        ));
        match handle_request(
            &context,
            "cursor/create_plan",
            &json!({"toolCallId":"p1","name":"Migrate","plan":"do things"}),
            &|_, _| Effect::Ask,
        ) {
            HandlerOutcome::Defer {
                kind: InteractionKind::Plan { tool_call_id },
                ui: DeferredUi::Question { options, .. },
                ..
            } => {
                assert_eq!(tool_call_id, "p1");
                assert_eq!(options, vec!["accept".to_string(), "reject".to_string()]);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(
            question_reply(&["q-a".to_string()], "React"),
            json!({"outcome": "answered", "answers": {"q-a": ["React"]}})
        );
        assert_eq!(plan_reply("accept"), json!({"outcome": "accepted"}));
        assert_eq!(plan_reply("no thanks"), json!({"outcome": "rejected"}));
    }

    #[cfg(unix)]
    #[test]
    fn terminal_create_is_policy_gated_and_lifecycle_is_owner_scoped() {
        let workspace = TempDirectory::new("acp-handlers-terminal");
        let roots = AuthorizedRoots::new([workspace.path().to_path_buf()]);
        let terminals = TerminalRegistry::default();
        let grammar = grammar();
        let environment = BTreeMap::from([(
            "PATH".to_string(),
            std::env::var("PATH").unwrap_or_default(),
        )]);
        let workspace_text = workspace.path().to_string_lossy().to_string();
        let context = context(
            &workspace_text,
            &roots,
            &terminals,
            &grammar,
            &environment,
            true,
        );
        let marker = workspace.path().join("marker");
        let create = json!({"sessionId":"ext-1","command":"sh","args":["-c", format!("touch {}", marker.display())]});
        assert!(matches!(
            handle_request(&context, "terminal/create", &create, &|_, _| Effect::Deny),
            HandlerOutcome::Reply(Err(_))
        ));
        assert!(!marker.exists(), "denied command must not run");
        assert!(matches!(
            handle_request(&context, "terminal/create", &create, &|_, _| Effect::Ask),
            HandlerOutcome::Defer {
                kind: InteractionKind::TerminalCreate { .. },
                ..
            }
        ));
        assert!(
            !marker.exists(),
            "deferred command must not run before a decision"
        );
        let created = handle_request(&context, "terminal/create", &create, &|_, _| Effect::Allow);
        let HandlerOutcome::Reply(Ok(reply)) = created else {
            panic!("expected terminal id, got {created:?}");
        };
        let terminal_id = reply["terminalId"].as_str().expect("id").to_string();
        let waited = handle_request(
            &context,
            "terminal/wait_for_exit",
            &json!({"terminalId": terminal_id}),
            &|_, _| Effect::Allow,
        );
        assert!(
            matches!(waited, HandlerOutcome::Reply(Ok(status)) if status["exitCode"] == json!(0))
        );
        assert!(marker.exists());
        let mut other = context;
        other.session_id = "another-session";
        assert!(matches!(
            handle_request(
                &other,
                "terminal/output",
                &json!({"terminalId": terminal_id}),
                &|_, _| Effect::Allow
            ),
            HandlerOutcome::Reply(Err(_))
        ));
        assert!(matches!(
            handle_request(
                &other,
                "terminal/release",
                &json!({"terminalId": terminal_id}),
                &|_, _| Effect::Allow
            ),
            HandlerOutcome::Reply(Err(_))
        ));
        assert_eq!(terminals.open_count(), 1);
        // cwd outside the roots is refused before spawning.
        let escape = json!({"command":"sh","args":["-c","true"],"cwd":"/"});
        assert!(matches!(
            handle_request(&other, "terminal/create", &escape, &|_, _| Effect::Allow),
            HandlerOutcome::Reply(Err(_))
        ));
        terminals.release_epoch(5);
    }
}
