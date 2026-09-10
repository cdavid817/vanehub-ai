//! Agent-originated requests, decided by policy.
//!
//! Every method the host advertises is handled here; every other method with an id is answered
//! with `method not found`. A handler either replies now or defers: a deferred request becomes a
//! backend-owned pending interaction and is answered when the user (or the deadline, or a
//! cancellation) decides. Nothing here executes an instruction carried inside a payload -- a
//! plan, a question, or a tool title is untrusted content and is shown, never obeyed.

use super::definitions::AcpLaunchGrammar;
use super::interactions::{
    select_permission_option, CursorOption, CursorQuestion, InteractionKind, PermissionOption,
    PermissionOptionKind,
};
use super::jsonrpc::RpcError;
use super::proxy_fs::{read_text_file, write_text_file, AuthorizedRoots};
use super::proxy_terminal::{
    TerminalCreateRequest, TerminalOwner, TerminalProxyError, TerminalRegistry,
};
use crate::contexts::agent_runtime::application::LoopScopeGuard;
use crate::contexts::agent_runtime::domain::LoopSideEffectChannel;
use crate::contexts::permissions::api::{Effect, PermissionVerdict};
use serde_json::{json, Value};

/// Line/limit bounds accepted for a proxied read, checked before any policy or file access.
pub(crate) const MAX_READ_LIMIT: u64 = 20_000;
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
    /// The frozen Loop scope for a Loop-owned session. Admission through it precedes policy.
    pub(crate) scope: Option<&'a dyn LoopScopeGuard>,
}

/// Decides one inbound request. `evaluate` is the permission decision point. It may return a
/// bare `Effect` (tests, fixed policies) or a `PermissionVerdict` carrying evaluation health.
pub(crate) fn handle_request<E: Into<PermissionVerdict>>(
    context: &HandlerContext<'_>,
    method: &str,
    params: &Value,
    evaluate: &dyn Fn(&str, &str) -> E,
) -> HandlerOutcome {
    let verdict =
        |action: &str, resource: &str| -> PermissionVerdict { evaluate(action, resource).into() };
    let evaluate: &dyn Fn(&str, &str) -> PermissionVerdict = &verdict;
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
    evaluate: &dyn Fn(&str, &str) -> PermissionVerdict,
) -> HandlerOutcome {
    let subject = match permission_subject(params) {
        Ok(subject) => subject,
        Err(error) => return HandlerOutcome::Reply(Err(error)),
    };
    let options = match permission_options(params) {
        Ok(options) => options,
        Err(error) => return HandlerOutcome::Reply(Err(error)),
    };
    // An agent-internal mutation is an opaque channel: a Loop role that does not admit it is
    // answered with the rejection option before policy is even consulted.
    if let Some(scope) = context.scope {
        if subject.action != "file.read"
            && scope
                .admit_channel(LoopSideEffectChannel::CliInternal)
                .is_err()
        {
            return HandlerOutcome::Reply(Ok(permission_reply(&options, false)));
        }
    }
    let verdict = evaluate(subject.action, &subject.resource);
    if !verdict.healthy {
        return HandlerOutcome::Reply(Ok(permission_reply(&options, false)));
    }
    match verdict.effect {
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

/// Validates the read request's shape before any policy or filesystem access.
pub(crate) fn read_request(params: &Value) -> Result<(String, Option<u64>, Option<u64>), RpcError> {
    let Some(path) = params.get("path").and_then(Value::as_str) else {
        return Err(RpcError::invalid_params("path is required"));
    };
    if path.trim().is_empty() || path.chars().any(char::is_control) || path.len() > 4096 {
        return Err(RpcError::invalid_params("path is invalid"));
    }
    let line = match params.get("line") {
        None | Some(Value::Null) => None,
        Some(value) => match value.as_u64() {
            Some(line) if line >= 1 => Some(line),
            _ => return Err(RpcError::invalid_params("line must be a positive integer")),
        },
    };
    let limit = match params.get("limit") {
        None | Some(Value::Null) => None,
        Some(value) => match value.as_u64() {
            Some(limit) if (1..=MAX_READ_LIMIT).contains(&limit) => Some(limit),
            _ => {
                return Err(RpcError::invalid_params(
                    "limit must be between 1 and the proxy maximum",
                ))
            }
        },
    };
    Ok((path.to_string(), line, limit))
}

/// Every effect is explicit. Allow reads after safe resolution; Deny answers without touching
/// the file; a healthy Ask defers to durable approval without reading anything first; an
/// unanswerable or unhealthy Ask is refused.
fn handle_read(
    context: &HandlerContext<'_>,
    params: &Value,
    evaluate: &dyn Fn(&str, &str) -> PermissionVerdict,
) -> HandlerOutcome {
    let (path, line, limit) = match read_request(params) {
        Ok(request) => request,
        Err(error) => return HandlerOutcome::Reply(Err(error)),
    };
    let resolved = match context.roots.resolve(&path) {
        Ok(resolved) => resolved,
        Err(error) => return HandlerOutcome::Reply(Err(RpcError::refused(error.message()))),
    };
    let verdict = evaluate("file.read", &path);
    if !verdict.healthy {
        return HandlerOutcome::Reply(Err(RpcError::refused(
            "read policy could not be evaluated; refusing without reading",
        )));
    }
    match verdict.effect {
        Effect::Deny => HandlerOutcome::Reply(Err(RpcError::refused("read denied by policy"))),
        Effect::Allow => HandlerOutcome::Reply(apply_read(context.roots, &path, line, limit)),
        Effect::Ask if !context.interactive => HandlerOutcome::Reply(Err(RpcError::refused(
            "read requires a human approval and none is available",
        ))),
        Effect::Ask => HandlerOutcome::Defer {
            ui: DeferredUi::Approval {
                tool_call_id: format!("acp-fs-read-{}", stable_token(&path)),
                tool_name: "fs/read_text_file".to_string(),
                action: "file.read",
                resource: path.clone(),
                input: json!({"path": path, "line": line, "limit": limit}),
            },
            kind: InteractionKind::FileRead {
                identity: read_identity(&resolved),
                path,
                line,
                limit,
            },
            deadline: None,
        },
    }
}

/// `(device, inode, ctime)` of an existing target; `None` when it cannot be stated, in which case
/// delivery falls back to path resolution alone.
fn read_identity(resolved: &std::path::Path) -> Option<(u64, u64, i64)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::symlink_metadata(resolved)
            .ok()
            .filter(|metadata| metadata.is_file())
            .map(|metadata| {
                (
                    metadata.dev(),
                    metadata.ino(),
                    metadata
                        .ctime()
                        .saturating_mul(1_000_000_000)
                        .saturating_add(metadata.ctime_nsec()),
                )
            })
    }
    #[cfg(not(unix))]
    {
        let _ = resolved;
        None
    }
}

/// Delivers a read the person approved while it was pending. The target is resolved again and
/// must still be the file that was deferred: a replaced, moved or relinked target is refused
/// without opening anything, so an approval never reads a file the person did not see.
pub(crate) fn deliver_read(
    roots: &AuthorizedRoots,
    path: &str,
    line: Option<u64>,
    limit: Option<u64>,
    identity: Option<(u64, u64, i64)>,
) -> Result<Value, RpcError> {
    let resolved = roots
        .resolve(path)
        .map_err(|_| RpcError::refused("read target is no longer inside the authorized roots"))?;
    if let Some(expected) = identity {
        if read_identity(&resolved) != Some(expected) {
            return Err(RpcError::refused(
                "read target was replaced while the approval was pending; request it again",
            ));
        }
    }
    apply_read(roots, path, line, limit)
}

pub(crate) fn apply_read(
    roots: &AuthorizedRoots,
    path: &str,
    line: Option<u64>,
    limit: Option<u64>,
) -> Result<Value, RpcError> {
    read_text_file(roots, path, line, limit)
        .map(|content| json!({"content": content}))
        .map_err(|error| RpcError::refused(error.message()))
}

fn handle_write(
    context: &HandlerContext<'_>,
    params: &Value,
    evaluate: &dyn Fn(&str, &str) -> PermissionVerdict,
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
    // Scope admission precedes policy: an out-of-bound write is never offered for approval.
    if let Some(scope) = context.scope {
        if let Err(reason) = scope.admit_write(path) {
            return HandlerOutcome::Reply(Err(RpcError::refused(&reason)));
        }
    }
    let verdict = evaluate("file.write", path);
    if !verdict.healthy {
        return HandlerOutcome::Reply(Err(RpcError::refused(
            "write policy could not be evaluated; refusing",
        )));
    }
    match verdict.effect {
        Effect::Allow => HandlerOutcome::Reply(apply_scoped_write(context, path, content)),
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

/// Delivers through the Loop guard when the session is Loop-owned, otherwise through the
/// authorized roots. The guard re-admits the real resource at delivery time.
pub(crate) fn apply_scoped_write(
    context: &HandlerContext<'_>,
    path: &str,
    content: &str,
) -> Result<Value, RpcError> {
    match context.scope {
        Some(scope) => scope
            .write(path, content.as_bytes())
            .map(|()| Value::Null)
            .map_err(|reason| RpcError::refused(&reason)),
        None => apply_write(context.roots, path, content),
    }
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
    evaluate: &dyn Fn(&str, &str) -> PermissionVerdict,
) -> HandlerOutcome {
    let request = match terminal_request(context, params) {
        Ok(request) => request,
        Err(error) => return HandlerOutcome::Reply(Err(error)),
    };
    if let Some(scope) = context.scope {
        if let Err(reason) = scope.admit_channel(LoopSideEffectChannel::Terminal) {
            return HandlerOutcome::Reply(Err(RpcError::refused(&reason)));
        }
    }
    let verdict = evaluate("shell.exec", "workspace");
    if !verdict.healthy {
        return HandlerOutcome::Reply(Err(RpcError::refused(
            "command policy could not be evaluated; refusing",
        )));
    }
    match verdict.effect {
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
        // Bounded so the driver stays responsive, and abandoned on cancel so the turn can end.
        // A wait that outlives the bound is refused, not answered: the protocol's reply means
        // "the command exited", and a fabricated status for a still-running process would send
        // the agent on to read output that is not final. It may ask again.
        "terminal/wait_for_exit" => context
            .terminals
            .wait_for_exit(owner, terminal_id, TERMINAL_WAIT_BOUND, &|| {
                context.cancel.load(Ordering::SeqCst)
            })
            .and_then(|status| {
                status.ok_or_else(|| {
                    if context.cancel.load(Ordering::SeqCst) {
                        TerminalProxyError::Io("wait abandoned: the turn was cancelled".to_string())
                    } else {
                        TerminalProxyError::Io(format!(
                            "the command is still running after the host's {}s wait bound; ask again",
                            TERMINAL_WAIT_BOUND.as_secs()
                        ))
                    }
                })
            })
            .map(|status| json!({"exitCode": status.exit_code, "signal": status.signal})),
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

/// Bound on one `terminal/wait_for_exit`. The turn driver answers requests one at a time, so a
/// wait this long holds every later request (output, kill) behind it; long enough for a build
/// step, short enough that the agent's own timeouts still see the host answer.
const TERMINAL_WAIT_BOUND: Duration = Duration::from_secs(60);

fn handle_cursor_question(context: &HandlerContext<'_>, params: &Value) -> HandlerOutcome {
    let Some(tool_call_id) = params.get("toolCallId").and_then(Value::as_str) else {
        return HandlerOutcome::Reply(Err(RpcError::invalid_params("toolCallId is required")));
    };
    let questions: Vec<CursorQuestion> = params
        .get("questions")
        .and_then(Value::as_array)
        .map(|questions| questions.iter().map(parse_cursor_question).collect())
        .unwrap_or_default();
    if questions.is_empty() {
        return HandlerOutcome::Reply(Err(RpcError::invalid_params("questions must not be empty")));
    }
    if !context.interactive {
        return HandlerOutcome::Reply(Ok(cursor_outcome("skipped", None)));
    }
    let (prompt, options) = question_card(&questions);
    HandlerOutcome::Defer {
        ui: DeferredUi::Question {
            tool_call_id: tool_call_id.to_string(),
            tool_name: "cursor/ask_question".to_string(),
            question: prompt,
            options,
        },
        kind: InteractionKind::Question {
            tool_call_id: tool_call_id.to_string(),
            questions,
        },
        deadline: None,
    }
}

/// Cursor's schema gives every option an `id` and a `label`. A bare string (older agents, and
/// the fixtures) is both at once.
fn parse_cursor_question(question: &Value) -> CursorQuestion {
    let options = question
        .get("options")
        .and_then(Value::as_array)
        .map(|options| {
            options
                .iter()
                .filter_map(|option| {
                    if let Some(text) = option.as_str() {
                        return Some(CursorOption {
                            id: text.to_string(),
                            label: text.to_string(),
                        });
                    }
                    let label = option.get("label").and_then(Value::as_str);
                    let id = option.get("id").and_then(Value::as_str).or(label)?;
                    Some(CursorOption {
                        id: id.to_string(),
                        label: label.unwrap_or(id).to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    CursorQuestion {
        id: question
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        prompt: question
            .get("prompt")
            .and_then(Value::as_str)
            .unwrap_or("The agent has a question.")
            .to_string(),
        options,
        allow_multiple: question
            .get("allowMultiple")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

/// What the card shows. One question shows its prompt and its option labels as choices; several
/// are listed in order with their options inline, and the person answers one line per question
/// -- every question the agent asked is on the card, so none is answered on their behalf.
fn question_card(questions: &[CursorQuestion]) -> (String, Vec<String>) {
    if let [only] = questions {
        let mut prompt = only.prompt.clone();
        if only.allow_multiple && !only.options.is_empty() {
            prompt.push_str(" (several may apply; separate choices with commas)");
        }
        let options = only
            .options
            .iter()
            .map(|option| option.label.clone())
            .collect();
        return (prompt, options);
    }
    let mut prompt =
        String::from("The agent asks several questions. Answer one per line, in order:");
    for (index, question) in questions.iter().enumerate() {
        prompt.push_str(&format!("\n{}. {}", index + 1, question.prompt));
        if !question.options.is_empty() {
            let labels: Vec<&str> = question
                .options
                .iter()
                .map(|option| option.label.as_str())
                .collect();
            prompt.push_str(&format!(" [{}]", labels.join(" / ")));
            if question.allow_multiple {
                prompt.push_str(" (several may apply)");
            }
        }
    }
    (prompt, Vec::new())
}

fn handle_cursor_plan(context: &HandlerContext<'_>, params: &Value) -> HandlerOutcome {
    let Some(tool_call_id) = params.get("toolCallId").and_then(Value::as_str) else {
        return HandlerOutcome::Reply(Err(RpcError::invalid_params("toolCallId is required")));
    };
    if !context.interactive {
        return HandlerOutcome::Reply(Ok(cursor_outcome("cancelled", None)));
    }
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Plan")
        .to_string();
    // The person decides on the plan itself, so the card carries it: overview, the plan text,
    // and the todo list, bounded so a long plan cannot blow the card up.
    let mut question = format!("Accept the proposed plan \"{name}\"?");
    if let Some(overview) = params.get("overview").and_then(Value::as_str) {
        if !overview.trim().is_empty() {
            question.push_str(&format!("\n\n{}", overview.trim()));
        }
    }
    if let Some(plan) = params.get("plan").and_then(Value::as_str) {
        if !plan.trim().is_empty() {
            question.push_str(&format!(
                "\n\n{}",
                truncate_chars(plan.trim(), PLAN_CARD_CHARS)
            ));
        }
    }
    if let Some(todos) = params.get("todos").and_then(Value::as_array) {
        let items: Vec<String> = todos
            .iter()
            .filter_map(|todo| todo.get("content").and_then(Value::as_str))
            .map(|content| format!("- {content}"))
            .collect();
        if !items.is_empty() {
            question.push_str(&format!("\n\n{}", items.join("\n")));
        }
    }
    HandlerOutcome::Defer {
        ui: DeferredUi::Question {
            tool_call_id: tool_call_id.to_string(),
            tool_name: "cursor/create_plan".to_string(),
            question,
            options: vec!["accept".to_string(), "reject".to_string()],
        },
        kind: InteractionKind::Plan {
            tool_call_id: tool_call_id.to_string(),
        },
        deadline: None,
    }
}

const PLAN_CARD_CHARS: usize = 4_000;

fn truncate_chars(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut shortened: String = text.chars().take(limit).collect();
    shortened.push_str(" [...]");
    shortened
}

/// Cursor's response envelope: the outcome is an object under `outcome`, whose own `outcome`
/// field names the variant (`answered`, `skipped`, `cancelled`, `accepted`, `rejected`).
pub(crate) fn cursor_outcome(variant: &str, extra: Option<(&str, Value)>) -> Value {
    let mut outcome = serde_json::Map::new();
    outcome.insert("outcome".to_string(), json!(variant));
    if let Some((key, value)) = extra {
        outcome.insert(key.to_string(), value);
    }
    json!({ "outcome": Value::Object(outcome) })
}

/// The `answered` reply: one entry per question that received an answer, naming the option ids
/// the agent issued. The card asked for one line per question in order; a line names an option
/// by its id or its label (case-insensitively) or its position, several separated by commas
/// when the question allows more than one. A question whose line names nothing it offered is
/// left unanswered rather than answered with a guess; if nothing at all could be mapped, the
/// reply is `skipped` with the reason, so the agent never receives an answer nobody gave.
pub(crate) fn question_reply(questions: &[CursorQuestion], answer: &str) -> Value {
    let lines: Vec<&str> = if questions.len() == 1 {
        vec![answer.trim()]
    } else {
        answer
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect()
    };
    let answers: Vec<Value> = questions
        .iter()
        .zip(lines.iter())
        .filter_map(|(question, line)| {
            let selected = select_option_ids(question, line);
            (!selected.is_empty())
                .then(|| json!({"questionId": question.id, "selectedOptionIds": selected}))
        })
        .collect();
    if answers.is_empty() {
        return cursor_outcome(
            "skipped",
            Some((
                "reason",
                json!("the reply named none of the offered options"),
            )),
        );
    }
    cursor_outcome("answered", Some(("answers", Value::Array(answers))))
}

fn select_option_ids(question: &CursorQuestion, line: &str) -> Vec<String> {
    let tokens: Vec<&str> = if question.allow_multiple {
        line.split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .collect()
    } else {
        vec![line.trim()]
    };
    let mut selected = Vec::new();
    for token in tokens {
        // A question without options takes the text itself as the answer id: the agent offered
        // nothing to pick from, so there is nothing to mismatch.
        if question.options.is_empty() {
            if !token.is_empty() {
                selected.push(token.to_string());
            }
            continue;
        }
        // A numbered choice ("2") counts as the option at that position on the card.
        let by_position = token
            .parse::<usize>()
            .ok()
            .and_then(|number| number.checked_sub(1))
            .and_then(|index| question.options.get(index));
        let found = question
            .options
            .iter()
            .find(|option| option.id == token)
            .or_else(|| {
                question
                    .options
                    .iter()
                    .find(|option| option.label.eq_ignore_ascii_case(token))
            })
            .or(by_position);
        if let Some(option) = found {
            if !selected.contains(&option.id) {
                selected.push(option.id.clone());
            }
        }
        if !question.allow_multiple {
            break;
        }
    }
    selected
}

pub(crate) fn plan_reply(answer: &str) -> Value {
    let accepted = matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "accept" | "accepted" | "yes" | "approve" | "approved" | "接受" | "同意"
    );
    if accepted {
        cursor_outcome("accepted", None)
    } else {
        cursor_outcome("rejected", Some(("reason", json!(answer.trim()))))
    }
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
    use crate::contexts::agent_runtime::application::LoopGuardRole;
    use crate::contexts::agent_runtime::domain::LoopRequestedMode;
    use crate::contexts::agent_runtime::infrastructure::loop_scope_platform::test_guard;
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
            scope: None,
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
            handle_request(&context, "session/request_permission", &foreign, &|_, _| -> Effect { panic!("not consulted") }),
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
            HandlerOutcome::Reply(Ok(json!({"outcome": {"outcome": "skipped"}})))
        );
        let plan = json!({"toolCallId":"p1","name":"Refactor","plan":"..."});
        assert_eq!(
            handle_request(&context, "cursor/create_plan", &plan, &|_, _| Effect::Ask),
            HandlerOutcome::Reply(Ok(json!({"outcome": {"outcome": "cancelled"}})))
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
        let question = json!({"toolCallId":"q1","title":"Choose","questions":[{"id":"q-a","prompt":"Which framework?","options":[{"id":"react","label":"React"},"Vue"]}]});
        let parsed = match handle_request(&context, "cursor/ask_question", &question, &|_, _| {
            Effect::Ask
        }) {
            HandlerOutcome::Defer {
                kind:
                    InteractionKind::Question {
                        tool_call_id,
                        questions,
                    },
                ui:
                    DeferredUi::Question {
                        question, options, ..
                    },
                ..
            } => {
                assert_eq!(tool_call_id, "q1");
                assert_eq!(questions.len(), 1);
                assert_eq!(questions[0].id, "q-a");
                assert_eq!(questions[0].options[0].id, "react");
                assert_eq!(questions[0].options[1].id, "Vue");
                assert_eq!(question, "Which framework?");
                assert_eq!(options, vec!["React".to_string(), "Vue".to_string()]);
                questions
            }
            other => panic!("unexpected {other:?}"),
        };
        // The reply follows Cursor's schema: a nested outcome naming option ids, never labels.
        let answered = |ids: Vec<&str>| json!({"outcome": {"outcome": "answered", "answers": [{"questionId": "q-a", "selectedOptionIds": ids}]}});
        assert_eq!(question_reply(&parsed, "react"), answered(vec!["react"]));
        assert_eq!(question_reply(&parsed, "React"), answered(vec!["react"]));
        assert_eq!(question_reply(&parsed, "2"), answered(vec!["Vue"]));
        assert_eq!(
            question_reply(&parsed, "Svelte")["outcome"]["outcome"],
            json!("skipped")
        );

        // Several questions: each shows on the card, each is answered from its own line, and a
        // multi-select question takes comma-separated choices. Nothing is copied across.
        let several = json!({"toolCallId":"q3","questions":[
            {"id":"lang","prompt":"Language?","options":[{"id":"ts","label":"TypeScript"},{"id":"rs","label":"Rust"}]},
            {"id":"tools","prompt":"Tools?","options":[{"id":"lint","label":"Lint"},{"id":"fmt","label":"Format"},{"id":"test","label":"Test"}],"allowMultiple":true}
        ]});
        let parsed = match handle_request(&context, "cursor/ask_question", &several, &|_, _| {
            Effect::Ask
        }) {
            HandlerOutcome::Defer {
                kind: InteractionKind::Question { questions, .. },
                ui:
                    DeferredUi::Question {
                        question, options, ..
                    },
                ..
            } => {
                assert!(question.contains("1. Language?") && question.contains("2. Tools?"));
                assert!(question.contains("Lint / Format / Test"));
                assert!(options.is_empty());
                questions
            }
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(
            question_reply(&parsed, "Rust\nlint, Test"),
            json!({"outcome": {"outcome": "answered", "answers": [
                {"questionId": "lang", "selectedOptionIds": ["rs"]},
                {"questionId": "tools", "selectedOptionIds": ["lint", "test"]}
            ]}})
        );
        // Only the first line given: the second question stays unanswered, not guessed.
        assert_eq!(
            question_reply(&parsed, "ts"),
            json!({"outcome": {"outcome": "answered", "answers": [
                {"questionId": "lang", "selectedOptionIds": ["ts"]}
            ]}})
        );
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
            &json!({"toolCallId":"p1","name":"Migrate","overview":"Move to v2","plan":"do things","todos":[{"id":"t1","content":"Inspect","status":"pending"}]}),
            &|_, _| Effect::Ask,
        ) {
            HandlerOutcome::Defer {
                kind: InteractionKind::Plan { tool_call_id },
                ui:
                    DeferredUi::Question {
                        question, options, ..
                    },
                ..
            } => {
                assert_eq!(tool_call_id, "p1");
                assert_eq!(options, vec!["accept".to_string(), "reject".to_string()]);
                // The person judges the plan itself, so the card carries it.
                assert!(question.contains("Move to v2") && question.contains("do things"));
                assert!(question.contains("- Inspect"));
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(
            plan_reply("accept"),
            json!({"outcome": {"outcome": "accepted"}})
        );
        assert_eq!(
            plan_reply("no thanks"),
            json!({"outcome": {"outcome": "rejected", "reason": "no thanks"}})
        );
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

    // ---- ACP file.read three-state behaviour (acceptance AC-01, AC-02, AC-03, AC-04, AC-05, AC-08) ----

    fn sentinel(label: &str) -> String {
        format!(
            "SENTINEL-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        )
    }

    fn read_params(path: &str, line: Option<u64>, limit: Option<u64>) -> Value {
        json!({"sessionId": "ext-1", "path": path, "line": line, "limit": limit})
    }

    struct ReadFixture {
        workspace: TempDirectory,
        roots: AuthorizedRoots,
        terminals: TerminalRegistry,
        grammar: AcpLaunchGrammar,
        environment: BTreeMap<String, String>,
        workspace_text: String,
        file: String,
        sentinel: String,
    }

    impl ReadFixture {
        fn new(label: &str) -> Self {
            let workspace = TempDirectory::new(label);
            let sentinel = sentinel(label);
            let file = workspace
                .write("notes.txt", &format!("alpha\n{sentinel}\ngamma\n"))
                .to_string_lossy()
                .to_string();
            let roots = AuthorizedRoots::new([workspace.path().to_path_buf()]);
            let workspace_text = workspace.path().to_string_lossy().to_string();
            Self {
                workspace,
                roots,
                terminals: TerminalRegistry::default(),
                grammar: grammar(),
                environment: BTreeMap::new(),
                workspace_text,
                file,
                sentinel,
            }
        }

        fn context(&self, interactive: bool) -> HandlerContext<'_> {
            context(
                &self.workspace_text,
                &self.roots,
                &self.terminals,
                &self.grammar,
                &self.environment,
                interactive,
            )
        }
    }

    #[test]
    fn file_read_allow_returns_the_requested_window_after_safe_resolution() {
        let fixture = ReadFixture::new("acp-read-allow");
        let context = fixture.context(true);
        let consulted = std::cell::Cell::new(0);
        let outcome = handle_request(
            &context,
            "fs/read_text_file",
            &read_params(&fixture.file, Some(2), Some(1)),
            &|action, resource| {
                consulted.set(consulted.get() + 1);
                assert_eq!(action, "file.read");
                assert_eq!(resource, fixture.file);
                Effect::Allow
            },
        );
        assert_eq!(consulted.get(), 1, "policy is consulted exactly once");
        assert_eq!(
            outcome,
            HandlerOutcome::Reply(Ok(json!({"content": fixture.sentinel})))
        );
        let whole = handle_request(
            &context,
            "fs/read_text_file",
            &read_params(&fixture.file, None, None),
            &|_, _| Effect::Allow,
        );
        assert_eq!(
            whole,
            HandlerOutcome::Reply(Ok(
                json!({"content": format!("alpha\n{}\ngamma\n", fixture.sentinel)})
            ))
        );
        let _ = &fixture.workspace;
    }

    #[test]
    fn file_read_deny_answers_without_content_or_pending_interaction() {
        let fixture = ReadFixture::new("acp-read-deny");
        let context = fixture.context(true);
        let outcome = handle_request(
            &context,
            "fs/read_text_file",
            &read_params(&fixture.file, None, None),
            &|_, _| Effect::Deny,
        );
        let HandlerOutcome::Reply(Err(error)) = &outcome else {
            panic!("deny must reply with an error, got {outcome:?}");
        };
        assert_eq!(error.code, super::super::jsonrpc::HOST_REFUSED);
        assert!(!format!("{outcome:?}").contains(&fixture.sentinel));
    }

    #[test]
    fn file_read_ask_defers_without_reading_and_refuses_when_unattended() {
        let fixture = ReadFixture::new("acp-read-ask");
        let interactive = fixture.context(true);
        let outcome = handle_request(
            &interactive,
            "fs/read_text_file",
            &read_params(&fixture.file, Some(1), Some(2)),
            &|_, _| Effect::Ask,
        );
        match &outcome {
            HandlerOutcome::Defer {
                kind:
                    InteractionKind::FileRead {
                        path,
                        line,
                        limit,
                        identity,
                    },
                ui:
                    DeferredUi::Approval {
                        tool_name,
                        action,
                        resource,
                        input,
                        ..
                    },
                ..
            } => {
                assert_eq!(path, &fixture.file);
                assert_eq!((*line, *limit), (Some(1), Some(2)));
                assert!(
                    identity.is_some(),
                    "the deferred read binds the target identity"
                );
                assert_eq!(tool_name, "fs/read_text_file");
                assert_eq!(*action, "file.read");
                assert_eq!(resource, &fixture.file);
                assert_eq!(input["path"], json!(fixture.file));
            }
            other => panic!("healthy Ask must defer, got {other:?}"),
        }
        assert!(
            !format!("{outcome:?}").contains(&fixture.sentinel),
            "nothing is read before the approval commits"
        );

        let unattended = fixture.context(false);
        let refused = handle_request(
            &unattended,
            "fs/read_text_file",
            &read_params(&fixture.file, None, None),
            &|_, _| Effect::Ask,
        );
        assert!(matches!(
            &refused,
            HandlerOutcome::Reply(Err(error)) if error.code == super::super::jsonrpc::HOST_REFUSED
        ));
        assert!(!format!("{refused:?}").contains(&fixture.sentinel));
    }

    #[test]
    fn file_read_fails_closed_when_policy_evaluation_is_unhealthy() {
        let fixture = ReadFixture::new("acp-read-unhealthy");
        let context = fixture.context(true);
        for effect in [Effect::Allow, Effect::Ask, Effect::Deny] {
            let outcome = handle_request(
                &context,
                "fs/read_text_file",
                &read_params(&fixture.file, None, None),
                &|_, _| PermissionVerdict {
                    effect,
                    healthy: false,
                },
            );
            assert!(
                matches!(&outcome, HandlerOutcome::Reply(Err(error)) if error.code == super::super::jsonrpc::HOST_REFUSED),
                "an evaluator fault is never an Allow and never a pending Ask: {outcome:?}"
            );
            assert!(!format!("{outcome:?}").contains(&fixture.sentinel));
        }
    }

    #[test]
    fn file_read_rejects_bad_windows_and_escapes_before_consulting_policy() {
        let fixture = ReadFixture::new("acp-read-invalid");
        let context = fixture.context(true);
        let never = |_: &str, _: &str| -> Effect { panic!("policy must not be consulted") };
        for params in [
            json!({"sessionId": "ext-1", "path": fixture.file, "line": 0}),
            json!({"sessionId": "ext-1", "path": fixture.file, "limit": 0}),
            json!({"sessionId": "ext-1", "path": fixture.file, "limit": MAX_READ_LIMIT + 1}),
            json!({"sessionId": "ext-1", "path": fixture.file, "line": -3}),
            json!({"sessionId": "ext-1"}),
        ] {
            let outcome = handle_request(&context, "fs/read_text_file", &params, &never);
            assert!(
                matches!(&outcome, HandlerOutcome::Reply(Err(_))),
                "{outcome:?}"
            );
            assert!(!format!("{outcome:?}").contains(&fixture.sentinel));
        }
        let outside = TempDirectory::new("acp-read-outside");
        let secret = sentinel("outside");
        let outside_file = outside
            .write("secret.txt", &secret)
            .to_string_lossy()
            .to_string();
        let outcome = handle_request(
            &context,
            "fs/read_text_file",
            &read_params(&outside_file, None, None),
            &never,
        );
        assert!(matches!(&outcome, HandlerOutcome::Reply(Err(_))));
        assert!(!format!("{outcome:?}").contains(&secret));
    }

    // ---- Loop scope admission on ACP writes (acceptance SC-01, SC-02, SC-12) ----

    #[cfg(unix)]
    #[test]
    fn scoped_writes_are_admitted_before_policy_and_never_widened_by_approval() {
        let workspace_directory = TempDirectory::new("acp-scope-write");
        let storage = TempDirectory::new("acp-scope-write-store");
        let outside_sentinel = sentinel("docs");
        let protected_sentinel = sentinel("generated");
        workspace_directory.write("docs/readme.md", &outside_sentinel);
        workspace_directory.write("src/generated/a.ts", &protected_sentinel);
        // The guard binds the canonical root; requests use the same spelling so absolute paths
        // inside the worktree resolve as in-root.
        let root = std::fs::canonicalize(workspace_directory.path()).expect("canonical root");
        let workspace = root.as_path();
        let roots = AuthorizedRoots::new([root.clone()]);
        let terminals = TerminalRegistry::default();
        let grammar = grammar();
        let environment = BTreeMap::new();
        let workspace_text = root.to_string_lossy().to_string();
        let worker = test_guard(
            workspace,
            storage.path(),
            &["src"],
            &["src/generated"],
            LoopRequestedMode::PreventiveRequired,
            LoopGuardRole::Worker,
        );
        let mut context = context(
            &workspace_text,
            &roots,
            &terminals,
            &grammar,
            &environment,
            true,
        );
        context.scope = Some(worker.as_ref());
        let target = |relative: &str| workspace.join(relative).to_string_lossy().to_string();
        let write_params = |relative: &str| json!({"sessionId": "ext-1", "path": target(relative), "content": "mutated"});

        // In scope: policy Allow delivers through the guard.
        let allowed = handle_request(
            &context,
            "fs/write_text_file",
            &write_params("src/app.ts"),
            &|_, _| Effect::Allow,
        );
        assert_eq!(allowed, HandlerOutcome::Reply(Ok(Value::Null)));
        assert_eq!(
            std::fs::read_to_string(workspace.join("src/app.ts")).expect("written"),
            "mutated"
        );

        // Out of scope and protected: refused before policy, even a global Allow cannot widen it.
        let never = |_: &str, _: &str| -> Effect {
            panic!("policy must not be consulted for an out-of-scope write")
        };
        for relative in ["docs/readme.md", "src/generated/a.ts"] {
            let refused = handle_request(
                &context,
                "fs/write_text_file",
                &write_params(relative),
                &never,
            );
            assert!(
                matches!(&refused, HandlerOutcome::Reply(Err(error)) if error.code == super::super::jsonrpc::HOST_REFUSED),
                "{refused:?}"
            );
        }
        assert_eq!(
            std::fs::read_to_string(workspace.join("docs/readme.md")).expect("kept"),
            outside_sentinel
        );
        assert_eq!(
            std::fs::read_to_string(workspace.join("src/generated/a.ts")).expect("kept"),
            protected_sentinel
        );
        // An Ask for an in-scope write is still deferred: scope admission never replaces policy.
        let deferred = handle_request(
            &context,
            "fs/write_text_file",
            &write_params("src/other.ts"),
            &|_, _| Effect::Ask,
        );
        assert!(matches!(
            deferred,
            HandlerOutcome::Defer {
                kind: InteractionKind::FileWrite { .. },
                ..
            }
        ));
        assert!(!workspace.join("src/other.ts").exists());

        // A Verifier is read-only on every channel: an Allow verdict still cannot write.
        let verifier = test_guard(
            workspace,
            storage.path(),
            &["src"],
            &["src/generated"],
            LoopRequestedMode::PreventiveRequired,
            LoopGuardRole::Verifier,
        );
        context.scope = Some(verifier.as_ref());
        let refused = handle_request(
            &context,
            "fs/write_text_file",
            &write_params("src/app.ts"),
            &never,
        );
        assert!(matches!(&refused, HandlerOutcome::Reply(Err(_))));
        assert_eq!(
            std::fs::read_to_string(workspace.join("src/app.ts")).expect("unchanged"),
            "mutated"
        );
        let terminal = handle_request(
            &context,
            "terminal/create",
            &json!({"sessionId": "ext-1", "command": "touch", "args": ["src/app.ts"]}),
            &|_, _| Effect::Allow,
        );
        assert!(
            matches!(&terminal, HandlerOutcome::Reply(Err(_))),
            "{terminal:?}"
        );
        terminals.release_epoch(5);
    }

    // ---- deferred read delivery binds the target identity (acceptance AC-06, AC-07) ----

    #[cfg(unix)]
    #[test]
    fn approved_read_delivers_the_deferred_target_once_and_refuses_a_replaced_or_relinked_target() {
        let fixture = ReadFixture::new("acp-read-delivery");
        let context = fixture.context(true);
        let deferred = handle_request(
            &context,
            "fs/read_text_file",
            &read_params(&fixture.file, None, None),
            &|_, _| Effect::Ask,
        );
        let HandlerOutcome::Defer {
            kind:
                InteractionKind::FileRead {
                    path,
                    line,
                    limit,
                    identity,
                },
            ..
        } = deferred
        else {
            panic!("expected a deferred read");
        };
        assert!(identity.is_some());

        // The approval delivers exactly the content that was pending.
        let delivered =
            deliver_read(&fixture.roots, &path, line, limit, identity).expect("delivered");
        assert_eq!(
            delivered,
            json!({"content": format!("alpha\n{}\ngamma\n", fixture.sentinel)})
        );

        // The target is replaced by a different file at the same path: the old approval must
        // not read it.
        let replacement = sentinel("replacement");
        std::fs::remove_file(&path).expect("remove");
        std::fs::write(&path, &replacement).expect("replace");
        let refused = deliver_read(&fixture.roots, &path, line, limit, identity)
            .expect_err("replaced target is refused");
        assert!(!format!("{refused:?}").contains(&replacement));
        assert!(!format!("{refused:?}").contains(&fixture.sentinel));

        // The path is turned into a link to a file outside the roots: refused before reading.
        let outside = TempDirectory::new("acp-read-delivery-outside");
        let secret = sentinel("secret");
        let secret_path = outside.write("secret.txt", &secret);
        std::fs::remove_file(&path).expect("remove replacement");
        std::os::unix::fs::symlink(&secret_path, &path).expect("relink");
        let refused = deliver_read(&fixture.roots, &path, line, limit, identity)
            .expect_err("relinked target is refused");
        assert!(!format!("{refused:?}").contains(&secret));

        // A pending read from before the deferral rules cannot be widened either: without a
        // bound identity delivery still requires the path to resolve inside the roots.
        let refused =
            deliver_read(&fixture.roots, &path, line, limit, None).expect_err("outside root");
        assert!(!format!("{refused:?}").contains(&secret));
        let _ = &fixture.workspace;
    }
}
