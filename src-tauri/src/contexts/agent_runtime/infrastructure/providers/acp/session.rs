//! The ACP session lifecycle on top of a connection: initialize, `session/new` or `session/load`,
//! and the prompt turn driver.
//!
//! A turn is one `session/prompt` request. While its response is outstanding the driver keeps
//! draining the connection's inbound queue, so a permission request the agent sends mid-turn is
//! decided (or deferred to a person) and answered without anyone waiting on anyone. The turn ends
//! on the prompt response's stop reason, never on process exit; a healthy connection is returned
//! to its binding for the next turn.

use super::blocks::{self, Tone};
use super::budget::{CANCEL_GRACE, DRIVER_TICK, HANDSHAKE_TIMEOUT};
use super::connection::{AcpConnection, AcpError, InboundEvent};
use super::definitions::AcpLaunchGrammar;
use super::handlers::{
    apply_terminal_create, cancelled_outcome, cursor_outcome, handle_request, permission_reply,
    plan_reply, question_reply, DeferredUi, HandlerContext, HandlerOutcome, TerminalCreateRecord,
};
use super::interactions::{
    InteractionKind, InteractionRejection, InteractionScope, PendingInteraction,
    PendingInteractionStore,
};
use super::jsonrpc::{RpcError, RpcId};
use super::proxy_fs::AuthorizedRoots;
use super::proxy_terminal::TerminalRegistry;
use crate::contexts::agent_runtime::application::{
    AgentPermissionPort, AgentProcessEventSink, GenerationProcessEvent, ToolApprovalDecision,
    ToolLifecycleEvent, ToolLifecyclePhase, ToolUseBlock,
};
use crate::contexts::agent_runtime::domain::LoopSideEffectChannel;
use crate::contexts::execution_observability::api::ExecutionFidelity;
use crate::contexts::permissions::api::{Action, Effect, PermissionVerdict, Resource};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub(crate) const ACP_PROTOCOL_VERSION: u64 = 1;

/// What the peer declared at `initialize`, normalized. Anything omitted is unsupported, as the
/// protocol specifies -- never assumed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct NegotiatedCapabilities {
    pub(crate) protocol_version: u64,
    pub(crate) load_session: bool,
    pub(crate) prompt_image: bool,
    pub(crate) prompt_audio: bool,
    pub(crate) prompt_embedded_context: bool,
    pub(crate) mcp_http: bool,
    pub(crate) mcp_sse: bool,
    pub(crate) auth_methods: Vec<String>,
    pub(crate) agent_name: Option<String>,
    pub(crate) agent_version: Option<String>,
}

impl NegotiatedCapabilities {
    pub(crate) fn summary(&self) -> Value {
        json!({
            "protocolVersion": self.protocol_version,
            "loadSession": self.load_session,
            "promptCapabilities": {"image": self.prompt_image, "audio": self.prompt_audio, "embeddedContext": self.prompt_embedded_context},
            "mcpCapabilities": {"http": self.mcp_http, "sse": self.mcp_sse},
            "authMethods": self.auth_methods,
            "agentName": self.agent_name,
            "agentVersion": self.agent_version,
        })
    }
}

/// What this host advertises. Only capabilities with a complete implementation behind them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HostCapabilities {
    pub(crate) fs: bool,
    pub(crate) terminal: bool,
}

pub(crate) fn initialize(
    connection: &AcpConnection,
    host: HostCapabilities,
) -> Result<NegotiatedCapabilities, AcpError> {
    let params = json!({
        "protocolVersion": ACP_PROTOCOL_VERSION,
        "clientCapabilities": {
            "fs": {"readTextFile": host.fs, "writeTextFile": host.fs},
            "terminal": host.terminal,
        },
        "clientInfo": {"name": "vanehub-ai", "title": "VaneHub AI", "version": env!("CARGO_PKG_VERSION")},
    });
    let pending = connection.request("initialize", params)?;
    let result = pending
        .recv_timeout(HANDSHAKE_TIMEOUT, "initialize")?
        .map_err(|error| AcpError::Protocol {
            reason_code: "acp-initialize-rejected",
            detail: error.to_string(),
        })?;
    let protocol_version = result
        .get("protocolVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| AcpError::Protocol {
            reason_code: "acp-initialize-malformed",
            detail: "protocolVersion missing".to_string(),
        })?;
    if protocol_version != ACP_PROTOCOL_VERSION {
        return Err(AcpError::Protocol {
            reason_code: "acp-protocol-version-unsupported",
            detail: format!("peer offered protocol version {protocol_version}"),
        });
    }
    let capabilities = result
        .get("agentCapabilities")
        .cloned()
        .unwrap_or(Value::Null);
    let flag = |path: &[&str]| -> bool {
        let mut current = &capabilities;
        for segment in path {
            current = match current.get(*segment) {
                Some(value) => value,
                None => return false,
            };
        }
        current.as_bool().unwrap_or(false)
    };
    let info = result.get("agentInfo").cloned().unwrap_or(Value::Null);
    Ok(NegotiatedCapabilities {
        protocol_version,
        load_session: flag(&["loadSession"]),
        prompt_image: flag(&["promptCapabilities", "image"]),
        prompt_audio: flag(&["promptCapabilities", "audio"]),
        prompt_embedded_context: flag(&["promptCapabilities", "embeddedContext"]),
        mcp_http: flag(&["mcpCapabilities", "http"]),
        mcp_sse: flag(&["mcpCapabilities", "sse"]),
        auth_methods: result
            .get("authMethods")
            .and_then(Value::as_array)
            .map(|methods| {
                methods
                    .iter()
                    .filter_map(|method| method.get("id").and_then(Value::as_str))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        agent_name: info.get("name").and_then(Value::as_str).map(str::to_string),
        agent_version: info
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

/// JSON-RPC `-32000` is what ACP agents answer when no account is signed in. Every installed
/// agent in the live evidence uses it (`Authentication required …`), so it gets its own error
/// instead of being folded into "the peer violated the protocol".
const ACP_AUTH_REQUIRED_CODE: i64 = -32000;

fn session_error(reason_code: &'static str, error: RpcError) -> AcpError {
    if error.code == ACP_AUTH_REQUIRED_CODE {
        return AcpError::AuthRequired {
            detail: error.message,
        };
    }
    AcpError::Protocol {
        reason_code,
        detail: error.to_string(),
    }
}

pub(crate) fn new_session(connection: &AcpConnection, cwd: &str) -> Result<String, AcpError> {
    let pending = connection.request("session/new", json!({"cwd": cwd, "mcpServers": []}))?;
    let result = pending
        .recv_timeout(HANDSHAKE_TIMEOUT, "session/new")?
        .map_err(|error| session_error("acp-session-new-rejected", error))?;
    result
        .get("sessionId")
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| AcpError::Protocol {
            reason_code: "acp-session-new-malformed",
            detail: "sessionId missing".to_string(),
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ReplaySummary {
    pub(crate) replayed_updates: usize,
    pub(crate) refused_requests: usize,
}

/// `session/load`: the agent replays history as `session/update` notifications before answering.
/// Replayed content is counted and discarded -- the local transcript already holds it, and
/// re-emitting it would duplicate messages and re-run tool projections. Any request the agent
/// makes during load is refused: nothing it could ask for is a replay.
pub(crate) fn load_session(
    connection: &AcpConnection,
    external_session_id: &str,
    cwd: &str,
) -> Result<ReplaySummary, AcpError> {
    let pending = connection.request(
        "session/load",
        json!({"sessionId": external_session_id, "cwd": cwd, "mcpServers": []}),
    )?;
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let mut summary = ReplaySummary::default();
    loop {
        if let Some(outcome) = pending.try_recv() {
            // Responses bypass the inbound queue, so the replay the agent wrote before its
            // response can still be sitting there when the response lands. The reader thread
            // queued every earlier frame before it delivered this one, so a non-blocking sweep
            // now sees the whole replay -- and the next turn starts with a clean queue instead
            // of projecting old history as its own output.
            drain_replay(connection, &mut summary);
            return match outcome? {
                Ok(_) => Ok(summary),
                Err(error) => Err(AcpError::Protocol {
                    reason_code: "acp-session-load-rejected",
                    detail: error.to_string(),
                }),
            };
        }
        if Instant::now() >= deadline {
            return Err(AcpError::Timeout("session/load"));
        }
        match connection.next_inbound(DRIVER_TICK) {
            Ok(InboundEvent::Notification { .. }) => summary.replayed_updates += 1,
            Ok(InboundEvent::Request { id, .. }) => {
                summary.refused_requests += 1;
                let _ = connection.respond(&id, Err(RpcError::cancelled()));
            }
            Ok(InboundEvent::Closed(failure)) => return Err(AcpError::Closed(failure)),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(AcpError::Closed(connection.failure().unwrap_or_else(
                    || super::connection::ConnectionFailure {
                        reason_code: "acp-connection-closed",
                        detail: "inbound queue closed".to_string(),
                    },
                )))
            }
        }
    }
}

fn drain_replay(connection: &AcpConnection, summary: &mut ReplaySummary) {
    while let Ok(event) = connection.next_inbound(Duration::ZERO) {
        match event {
            InboundEvent::Notification { .. } => summary.replayed_updates += 1,
            InboundEvent::Request { id, .. } => {
                summary.refused_requests += 1;
                let _ = connection.respond(&id, Err(RpcError::cancelled()));
            }
            InboundEvent::Closed(_) => break,
        }
    }
}

/// The reviewed stop reasons, plus a catch-all that keeps the raw value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StopReason {
    EndTurn,
    MaxTokens,
    MaxTurnRequests,
    Refusal,
    Cancelled,
    Other(String),
}

impl StopReason {
    pub(crate) fn parse(value: &str) -> Self {
        match value {
            "end_turn" => Self::EndTurn,
            "max_tokens" => Self::MaxTokens,
            "max_turn_requests" => Self::MaxTurnRequests,
            "refusal" => Self::Refusal,
            "cancelled" => Self::Cancelled,
            other => Self::Other(other.to_string()),
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::EndTurn => "end_turn",
            Self::MaxTokens => "max_tokens",
            Self::MaxTurnRequests => "max_turn_requests",
            Self::Refusal => "refusal",
            Self::Cancelled => "cancelled",
            Self::Other(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TurnOutcome {
    Completed {
        stop_reason: StopReason,
    },
    /// The host cancelled and the agent answered `cancelled`; the connection is reusable.
    Cancelled,
    /// The host cancelled, the agent did not answer in time, and the process tree was reaped.
    ForcedCancel,
    /// The connection went away mid-turn. Effects the agent may have produced are unknown.
    Interrupted {
        reason_code: &'static str,
        detail: String,
    },
    Failed {
        reason_code: &'static str,
        detail: String,
    },
}

/// Everything one execution binding holds between turns.
pub(crate) struct BindingState {
    pub(crate) connection: Arc<AcpConnection>,
    pub(crate) external_session_id: String,
    pub(crate) negotiated: NegotiatedCapabilities,
    pub(crate) host: HostCapabilities,
    pub(crate) grammar: AcpLaunchGrammar,
    pub(crate) roots: AuthorizedRoots,
    pub(crate) workspace: String,
    pub(crate) child_environment: BTreeMap<String, String>,
    pub(crate) session_id: String,
}

impl BindingState {
    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }
}

/// The parts of a turn that both the driver thread and a decision arriving from the UI need.
pub(crate) struct TurnShared {
    pub(crate) turn_id: String,
    pub(crate) session_id: String,
    pub(crate) agent_id: String,
    pub(crate) operation_id: String,
    pub(crate) project_key: String,
    pub(crate) interactive: bool,
    pub(crate) binding: Arc<BindingState>,
    pub(crate) sink: Arc<dyn AgentProcessEventSink>,
    pub(crate) permissions: Arc<dyn AgentPermissionPort>,
    pub(crate) interactions: Mutex<PendingInteractionStore>,
    pub(crate) terminals: Arc<TerminalRegistry>,
    pub(crate) cancel: AtomicBool,
    pub(crate) sequence: AtomicU64,
    /// The Loop scope guard for a Loop-owned session; `None` for ordinary sessions.
    pub(crate) scope: Option<Arc<dyn crate::contexts::agent_runtime::application::LoopScopeGuard>>,
}

impl TurnShared {
    fn evaluate(&self, action: &str, resource: &str) -> PermissionVerdict {
        self.permissions.evaluate_checked(
            &self.agent_id,
            Action::new(action),
            Resource::new(resource),
            &self.session_id,
            &self.operation_id,
            &self.project_key,
        )
    }

    /// A re-evaluation before an approved effect: a healthy non-Deny keeps the approval usable;
    /// anything else refuses without the effect.
    fn still_permitted(&self, action: &str, resource: &str) -> bool {
        let verdict = self.evaluate(action, resource);
        verdict.healthy && verdict.effect != Effect::Deny
    }

    fn handler_context(&self) -> HandlerContext<'_> {
        HandlerContext {
            session_id: &self.session_id,
            external_session_id: &self.binding.external_session_id,
            epoch: self.binding.connection.epoch(),
            grammar: &self.binding.grammar,
            roots: &self.binding.roots,
            terminals: &self.terminals,
            workspace: &self.binding.workspace,
            interactive: self.interactive,
            advertised_fs: self.binding.host.fs,
            advertised_terminal: self.binding.host.terminal,
            child_environment: &self.binding.child_environment,
            cancel: &self.cancel,
            scope: self.scope.as_deref(),
        }
    }

    fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::Relaxed) + 1
    }

    fn emit(&self, event: GenerationProcessEvent) -> bool {
        self.sink.handle(event).is_ok()
    }

    fn emit_tool(
        &self,
        call_id: &str,
        name: &str,
        phase: ToolLifecyclePhase,
        status: &str,
        input: Option<Value>,
        output: Option<Value>,
    ) -> bool {
        self.emit(GenerationProcessEvent::ToolLifecycle(ToolLifecycleEvent {
            call_id: call_id.to_string(),
            phase,
            provider_timestamp: None,
            fidelity: ExecutionFidelity::Native,
            parent_run_id: None,
            parent_trace_id: None,
            parent_span_id: None,
            delegation_id: None,
            attempt: None,
            tool_use: ToolUseBlock {
                id: call_id.to_string(),
                name: name.to_string(),
                input,
                output,
                status: status.to_string(),
                skill_provenance: None,
            },
        }))
    }

    /// Applies a decision the UI delivered. `Ok(true)` when a pending interaction was consumed.
    pub(crate) fn resolve(
        &self,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, String> {
        let epoch = self.binding.connection.epoch();
        let consumed = {
            let mut store = lock(&self.interactions);
            let Some(pending) = store.get(call_id) else {
                return Ok(false);
            };
            if !decision_fits(&pending.kind, &decision) {
                return Err(format!(
                    "decision does not fit the pending interaction for {call_id}"
                ));
            }
            match store.consume(call_id, &self.session_id, epoch, decision) {
                Ok(consumed) => consumed,
                Err(InteractionRejection::NotFound) => return Ok(false),
                Err(InteractionRejection::WrongSession) => {
                    return Err("interaction belongs to another session".to_string())
                }
                Err(InteractionRejection::StaleEpoch) => {
                    return Err("interaction belongs to a replaced connection".to_string())
                }
            }
        };
        let interaction = consumed.interaction;
        let decision = consumed.decision;
        let reply = self.reply_for(&interaction, &decision);
        self.binding
            .connection
            .respond(&interaction.rpc_id, reply)
            .map_err(|error| error.to_string())?;
        Ok(true)
    }

    fn reply_for(
        &self,
        interaction: &PendingInteraction,
        decision: &ToolApprovalDecision,
    ) -> Result<Value, RpcError> {
        let context = self.handler_context();
        match (&interaction.kind, decision) {
            (
                InteractionKind::Permission {
                    options,
                    action,
                    resource,
                    ..
                },
                ToolApprovalDecision::Approved,
            ) => {
                // The policy may have tightened while the request waited. A user's yes cannot
                // override a template that now says no.
                let approve = self.still_permitted(action, resource);
                Ok(permission_reply(options, approve))
            }
            (InteractionKind::Permission { options, .. }, ToolApprovalDecision::Denied) => {
                Ok(permission_reply(options, false))
            }
            (InteractionKind::FileWrite { path, content }, ToolApprovalDecision::Approved) => {
                let outcome = if !self.still_permitted("file.write", path) {
                    Err(RpcError::refused("write denied by policy"))
                } else {
                    super::handlers::apply_scoped_write(&context, path, content)
                };
                self.emit_tool(
                    &interaction.call_id,
                    "fs/write_text_file",
                    phase_of(&outcome),
                    status_of(&outcome),
                    None,
                    None,
                );
                outcome
            }
            (InteractionKind::FileWrite { .. }, ToolApprovalDecision::Denied) => {
                self.emit_tool(
                    &interaction.call_id,
                    "fs/write_text_file",
                    ToolLifecyclePhase::Cancelled,
                    "cancelled",
                    None,
                    None,
                );
                Err(RpcError::refused("write denied by user"))
            }
            (
                InteractionKind::FileRead {
                    path,
                    line,
                    limit,
                    identity,
                },
                ToolApprovalDecision::Approved,
            ) => {
                // Delivery re-checks current policy and the bound target identity; the file is
                // opened only here, after the resolution committed and matched this pending read.
                let outcome = if !self.still_permitted("file.read", path) {
                    Err(RpcError::refused("read denied by policy"))
                } else {
                    super::handlers::deliver_read(
                        &self.binding.roots,
                        path,
                        *line,
                        *limit,
                        *identity,
                    )
                };
                self.emit_tool(
                    &interaction.call_id,
                    "fs/read_text_file",
                    phase_of(&outcome),
                    status_of(&outcome),
                    None,
                    None,
                );
                outcome
            }
            (InteractionKind::FileRead { .. }, ToolApprovalDecision::Denied) => {
                self.emit_tool(
                    &interaction.call_id,
                    "fs/read_text_file",
                    ToolLifecyclePhase::Cancelled,
                    "cancelled",
                    None,
                    None,
                );
                Err(RpcError::refused("read denied by user"))
            }
            (InteractionKind::TerminalCreate { request }, ToolApprovalDecision::Approved) => {
                let outcome = if !self.still_permitted("shell.exec", "workspace") {
                    Err(RpcError::refused("command execution denied by policy"))
                } else if context.scope.is_some_and(|scope| {
                    scope
                        .admit_channel(LoopSideEffectChannel::Terminal)
                        .is_err()
                }) {
                    Err(RpcError::refused(
                        "terminal channel is not admitted for this Loop role",
                    ))
                } else {
                    serde_json::from_value::<TerminalCreateRecord>(request.clone())
                        .map_err(|_| RpcError::refused("terminal request is no longer readable"))
                        .and_then(|record| apply_terminal_create(&context, &record.into()))
                };
                self.emit_tool(
                    &interaction.call_id,
                    "terminal/create",
                    phase_of(&outcome),
                    status_of(&outcome),
                    None,
                    None,
                );
                outcome
            }
            (InteractionKind::TerminalCreate { .. }, ToolApprovalDecision::Denied) => {
                self.emit_tool(
                    &interaction.call_id,
                    "terminal/create",
                    ToolLifecyclePhase::Cancelled,
                    "cancelled",
                    None,
                    None,
                );
                Err(RpcError::refused("command execution denied by user"))
            }
            (
                InteractionKind::Question { questions, .. },
                ToolApprovalDecision::Answered(answer),
            ) => {
                let reply = question_reply(questions, answer);
                self.emit_tool(
                    &interaction.call_id,
                    "cursor/ask_question",
                    ToolLifecyclePhase::Completed,
                    "completed",
                    None,
                    Some(json!({"answer": answer, "reply": reply.clone()})),
                );
                Ok(reply)
            }
            (InteractionKind::Question { .. }, ToolApprovalDecision::Denied) => {
                self.emit_tool(
                    &interaction.call_id,
                    "cursor/ask_question",
                    ToolLifecyclePhase::Cancelled,
                    "cancelled",
                    None,
                    None,
                );
                Ok(cursor_outcome("skipped", None))
            }
            (InteractionKind::Plan { .. }, ToolApprovalDecision::Answered(answer)) => {
                let reply = plan_reply(answer);
                self.emit_tool(
                    &interaction.call_id,
                    "cursor/create_plan",
                    ToolLifecyclePhase::Completed,
                    "completed",
                    None,
                    Some(reply.clone()),
                );
                Ok(reply)
            }
            (InteractionKind::Plan { .. }, ToolApprovalDecision::Approved) => {
                self.emit_tool(
                    &interaction.call_id,
                    "cursor/create_plan",
                    ToolLifecyclePhase::Completed,
                    "completed",
                    None,
                    None,
                );
                Ok(cursor_outcome("accepted", None))
            }
            (InteractionKind::Plan { .. }, ToolApprovalDecision::Denied) => {
                self.emit_tool(
                    &interaction.call_id,
                    "cursor/create_plan",
                    ToolLifecyclePhase::Cancelled,
                    "cancelled",
                    None,
                    None,
                );
                Ok(cursor_outcome("rejected", None))
            }
            _ => Err(RpcError::cancelled()),
        }
    }

    /// Answers every pending interaction of this turn with the protocol's cancelled outcome.
    fn cancel_pending(&self, reason: &str) {
        let taken = lock(&self.interactions).take_for_turn(&self.turn_id);
        for interaction in taken {
            let _ = self
                .binding
                .connection
                .respond(&interaction.rpc_id, cancelled_reply(&interaction.kind));
            let (name, _) = interaction_display(&interaction.kind);
            self.emit_tool(
                &interaction.call_id,
                name,
                ToolLifecyclePhase::Cancelled,
                "cancelled",
                None,
                Some(json!({"reason": reason})),
            );
        }
    }

    fn expire_pending(&self) {
        let expired = lock(&self.interactions).take_expired(Instant::now());
        for interaction in expired {
            let _ = self
                .binding
                .connection
                .respond(&interaction.rpc_id, cancelled_reply(&interaction.kind));
            let (name, _) = interaction_display(&interaction.kind);
            self.emit_tool(
                &interaction.call_id,
                name,
                ToolLifecyclePhase::Cancelled,
                "cancelled",
                None,
                Some(json!({"reason": "interaction-deadline"})),
            );
        }
    }

    pub(crate) fn pending_count(&self) -> usize {
        lock(&self.interactions).list().len()
    }
}

fn decision_fits(kind: &InteractionKind, decision: &ToolApprovalDecision) -> bool {
    !matches!(
        (kind, decision),
        (
            InteractionKind::Permission { .. }
                | InteractionKind::FileWrite { .. }
                | InteractionKind::FileRead { .. }
                | InteractionKind::TerminalCreate { .. },
            ToolApprovalDecision::Answered(_),
        ) | (
            InteractionKind::Question { .. },
            ToolApprovalDecision::Approved
        )
    )
}

fn cancelled_reply(kind: &InteractionKind) -> Result<Value, RpcError> {
    match kind {
        InteractionKind::Permission { .. } => Ok(cancelled_outcome()),
        InteractionKind::Question { .. } | InteractionKind::Plan { .. } => {
            Ok(cursor_outcome("cancelled", None))
        }
        InteractionKind::FileWrite { .. }
        | InteractionKind::FileRead { .. }
        | InteractionKind::TerminalCreate { .. } => Err(RpcError::cancelled()),
    }
}

fn interaction_display(kind: &InteractionKind) -> (&'static str, ToolLifecyclePhase) {
    match kind {
        InteractionKind::Permission { .. } => (
            "session/request_permission",
            ToolLifecyclePhase::AwaitingApproval,
        ),
        InteractionKind::FileWrite { .. } => {
            ("fs/write_text_file", ToolLifecyclePhase::AwaitingApproval)
        }
        InteractionKind::FileRead { .. } => {
            ("fs/read_text_file", ToolLifecyclePhase::AwaitingApproval)
        }
        InteractionKind::TerminalCreate { .. } => {
            ("terminal/create", ToolLifecyclePhase::AwaitingApproval)
        }
        InteractionKind::Question { .. } => {
            ("cursor/ask_question", ToolLifecyclePhase::AwaitingInput)
        }
        InteractionKind::Plan { .. } => ("cursor/create_plan", ToolLifecyclePhase::AwaitingInput),
    }
}

fn phase_of(outcome: &Result<Value, RpcError>) -> ToolLifecyclePhase {
    if outcome.is_ok() {
        ToolLifecyclePhase::Completed
    } else {
        ToolLifecyclePhase::Failed
    }
}

fn status_of(outcome: &Result<Value, RpcError>) -> &'static str {
    if outcome.is_ok() {
        "completed"
    } else {
        "failed"
    }
}

/// Runs one prompt turn to its outcome. Emits streaming events to the sink as they arrive; the
/// caller emits the terminal event from the returned outcome.
pub(crate) fn run_turn(shared: &Arc<TurnShared>, prompt: &str) -> TurnOutcome {
    let connection = shared.binding.connection.clone();
    let prompt_request = match connection.request(
        "session/prompt",
        json!({
            "sessionId": shared.binding.external_session_id,
            "prompt": [{"type": "text", "text": prompt}],
        }),
    ) {
        Ok(pending) => pending,
        Err(error) => {
            return TurnOutcome::Failed {
                reason_code: error.reason_code(),
                detail: error.to_string(),
            }
        }
    };
    let mut cancel_sent_at: Option<Instant> = None;
    let mut tool_activity = false;
    loop {
        if let Some(outcome) = prompt_request.try_recv() {
            // Responses bypass the inbound queue, so updates the agent sent before its final
            // response may still be queued. They are projected first: buffered text and tool
            // events belong to this turn, and an interruption's "effects unknown" must see them.
            tool_activity |= drain_queued_inbound(shared, &connection);
            return match outcome {
                Ok(Ok(result)) => {
                    let stop_reason = result
                        .get("stopReason")
                        .and_then(Value::as_str)
                        .map(StopReason::parse)
                        .unwrap_or(StopReason::Other("missing".to_string()));
                    // Late arrivals for this turn are refused by the store: the turn is over.
                    shared.cancel_pending("turn-completed");
                    if stop_reason == StopReason::Cancelled {
                        TurnOutcome::Cancelled
                    } else {
                        TurnOutcome::Completed { stop_reason }
                    }
                }
                Ok(Err(error)) => {
                    shared.cancel_pending("turn-failed");
                    TurnOutcome::Failed {
                        reason_code: "acp-prompt-rejected",
                        detail: error.to_string(),
                    }
                }
                Err(error) => {
                    shared.cancel_pending("connection-lost");
                    interrupted(&connection, error, tool_activity)
                }
            };
        }
        if shared.cancel.load(Ordering::SeqCst) {
            match cancel_sent_at {
                None => {
                    let _ = connection.notify(
                        "session/cancel",
                        json!({"sessionId": shared.binding.external_session_id}),
                    );
                    shared.cancel_pending("cancelled");
                    cancel_sent_at = Some(Instant::now());
                }
                Some(sent_at) if sent_at.elapsed() >= CANCEL_GRACE => {
                    let _ = connection
                        .terminate("acp-cancel-escalated", "agent ignored session/cancel");
                    shared.terminals.release_epoch(connection.epoch());
                    return TurnOutcome::ForcedCancel;
                }
                Some(_) => {}
            }
        }
        shared.expire_pending();
        match connection.next_inbound(DRIVER_TICK) {
            Ok(InboundEvent::Notification { method, params }) => {
                if method == "session/update" {
                    tool_activity |= project_update(shared, &params);
                } else if shared
                    .binding
                    .grammar
                    .notification_extensions
                    .contains(&method.as_str())
                {
                    let payload = bounded_payload(&params);
                    shared.emit(GenerationProcessEvent::RichBlock(blocks::card(
                        "acp_extension_notification",
                        "Provider notification",
                        blocks::fenced_json(&payload),
                        Tone::Info,
                        vec![("Method", method.clone())],
                        json!({ "method": method, "payload": payload }),
                    )));
                }
                // Any other notification is ignored: notifications never get a reply.
            }
            Ok(InboundEvent::Request { id, method, params }) => {
                tool_activity |= dispatch_request(shared, id, &method, &params);
            }
            Ok(InboundEvent::Closed(failure)) => {
                shared.cancel_pending("connection-lost");
                shared.terminals.release_epoch(connection.epoch());
                return interrupted(&connection, AcpError::Closed(failure), tool_activity);
            }
            Err(RecvTimeoutError::Timeout) => {
                if let Some(code) = connection.poll_child_exit() {
                    shared.cancel_pending("process-exited");
                    shared.terminals.release_epoch(connection.epoch());
                    return TurnOutcome::Interrupted {
                        reason_code: "acp-process-exited",
                        detail: format!("agent process exited with {code:?} during the turn"),
                    };
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                shared.cancel_pending("connection-lost");
                shared.terminals.release_epoch(connection.epoch());
                return interrupted(
                    &connection,
                    AcpError::Closed(connection.failure().unwrap_or_else(|| {
                        super::connection::ConnectionFailure {
                            reason_code: "acp-connection-closed",
                            detail: "inbound queue closed".to_string(),
                        }
                    })),
                    tool_activity,
                );
            }
        }
    }
}

/// Projects everything already queued without waiting for more. Returns whether any of it was
/// tool activity.
fn drain_queued_inbound(shared: &Arc<TurnShared>, connection: &AcpConnection) -> bool {
    let mut tool_activity = false;
    while let Ok(event) = connection.next_inbound(Duration::ZERO) {
        match event {
            InboundEvent::Notification { method, params } if method == "session/update" => {
                tool_activity |= project_update(shared, &params);
            }
            InboundEvent::Notification { .. } => {}
            InboundEvent::Request { id, method, params } => {
                tool_activity |= dispatch_request(shared, id, &method, &params);
            }
            InboundEvent::Closed(_) => break,
        }
    }
    tool_activity
}

fn interrupted(connection: &AcpConnection, error: AcpError, tool_activity: bool) -> TurnOutcome {
    let stderr = connection.stderr_summary();
    let mut detail = error.to_string();
    if !stderr.redacted_tail.trim().is_empty() {
        detail.push_str(" | stderr: ");
        detail.push_str(stderr.redacted_tail.trim());
        if stderr.truncated {
            detail.push_str(" [truncated]");
        }
    }
    if tool_activity {
        detail.push_str(" | effects unknown: tool activity was observed before the interruption");
    }
    TurnOutcome::Interrupted {
        reason_code: error.reason_code(),
        detail,
    }
}

/// Handles one agent request. Returns whether it was a tool-related interaction.
fn dispatch_request(shared: &Arc<TurnShared>, id: RpcId, method: &str, params: &Value) -> bool {
    let outcome = {
        let context = shared.handler_context();
        handle_request(&context, method, params, &|action, resource| {
            shared.evaluate(action, resource)
        })
    };
    match outcome {
        HandlerOutcome::Reply(reply) => {
            let is_tool = matches!(
                method,
                "session/request_permission"
                    | "fs/write_text_file"
                    | "fs/read_text_file"
                    | "terminal/create"
            );
            if method == "session/request_permission" && !shared.interactive {
                if let Ok(reply) = &reply {
                    if reply["outcome"]["outcome"] != json!("selected")
                        || reply["outcome"]["optionId"]
                            .as_str()
                            .is_some_and(|option| option.contains("reject"))
                    {
                        let tool_call_id = params["toolCall"]["toolCallId"].clone();
                        shared.emit(GenerationProcessEvent::RichBlock(blocks::card(
                            "acp_unattended_rejection",
                            "Permission request rejected (unattended)",
                            "This run is unattended and its policy rejects interactive permission requests, so the agent's request was refused without asking anyone.",
                            Tone::Warning,
                            vec![
                                ("Method", method.to_string()),
                                ("Tool call", tool_call_id.as_str().unwrap_or("-").to_string()),
                            ],
                            json!({ "method": method, "toolCallId": tool_call_id }),
                        )));
                    }
                }
            }
            // A failed write here means the connection is closed (an oversized reply is
            // already answered with a bounded error inside `respond`); the driver observes the
            // closed connection on its next tick and ends the turn with the failure.
            let _ = shared.binding.connection.respond(&id, reply);
            is_tool
        }
        HandlerOutcome::Defer { kind, ui, deadline } => {
            // Host-minted ids (a token of the path or command) are stable across retries, so two
            // outstanding requests for the same target would share one id and the earlier one
            // would be silently overwritten and never answered. A second registration under a
            // live id gets a per-turn suffix; the UI resolves whichever id the event carried.
            let requested_id = match &ui {
                DeferredUi::Approval { tool_call_id, .. }
                | DeferredUi::Question { tool_call_id, .. } => tool_call_id.clone(),
            };
            let call_id = if lock(&shared.interactions).get(&requested_id).is_some() {
                format!("{requested_id}-{}", shared.next_sequence())
            } else {
                requested_id
            };
            let registered_ok = match &ui {
                DeferredUi::Approval {
                    tool_name,
                    action,
                    resource,
                    input,
                    ..
                } => {
                    let registration = shared.permissions.create_pending_approval(
                        &shared.agent_id,
                        Action::new(*action),
                        Resource::new(resource.clone()),
                        &shared.session_id,
                        &shared.operation_id,
                        &call_id,
                        &shared.project_key,
                    );
                    if registration.is_ok() {
                        shared.emit_tool(
                            &call_id,
                            tool_name,
                            ToolLifecyclePhase::AwaitingApproval,
                            "awaiting_approval",
                            Some(input.clone()),
                            None,
                        );
                    }
                    registration.is_ok()
                }
                DeferredUi::Question {
                    tool_name,
                    question,
                    options,
                    ..
                } => {
                    shared.emit_tool(
                        &call_id,
                        tool_name,
                        ToolLifecyclePhase::AwaitingInput,
                        "awaiting_input",
                        Some(json!({"question": question, "options": options})),
                        None,
                    );
                    true
                }
            };
            if !registered_ok {
                // The approval record could not be created, so nobody could ever answer it.
                let _ = shared
                    .binding
                    .connection
                    .respond(&id, cancelled_reply(&kind));
                return true;
            }
            lock(&shared.interactions).register(
                call_id,
                id,
                InteractionScope {
                    epoch: shared.binding.connection.epoch(),
                    session_id: &shared.session_id,
                    turn_id: &shared.turn_id,
                },
                kind,
                deadline,
            );
            true
        }
    }
}

/// Projects one `session/update` onto the runtime event model. Returns whether it was a tool
/// event, which is what makes a later interruption "effects unknown".
fn project_update(shared: &Arc<TurnShared>, params: &Value) -> bool {
    let Some(update) = params.get("update") else {
        return false;
    };
    let kind = update
        .get("sessionUpdate")
        .and_then(Value::as_str)
        .unwrap_or("");
    let provenance = provider_subagent_provenance(update);
    match kind {
        "agent_message_chunk" => {
            if let Some(text) = update
                .get("content")
                .and_then(|content| content.get("text"))
                .and_then(Value::as_str)
            {
                if let Some(provenance) = provenance.as_ref() {
                    shared.emit(GenerationProcessEvent::RichBlock(blocks::card(
                        "acp_provider_subagent_text",
                        "Provider sub-agent output",
                        text,
                        Tone::Info,
                        vec![("Provenance", provenance.to_string())],
                        json!({ "provenance": provenance, "text": text }),
                    )));
                } else {
                    shared.emit(GenerationProcessEvent::Token(text.to_string()));
                }
            } else if let Some(content) = update.get("content") {
                let content = bounded_payload(content);
                let content_type = content["type"].as_str().unwrap_or("unknown").to_string();
                shared.emit(GenerationProcessEvent::RichBlock(blocks::card(
                    "acp_content",
                    "Non-text agent content",
                    blocks::fenced_json(&content),
                    Tone::Info,
                    vec![("Content type", content_type)],
                    json!({ "content": content }),
                )));
            }
            false
        }
        "agent_thought_chunk" => {
            if let Some(text) = update
                .get("content")
                .and_then(|content| content.get("text"))
                .and_then(Value::as_str)
            {
                shared.emit(GenerationProcessEvent::Thinking(text.to_string()));
            }
            false
        }
        "tool_call" | "tool_call_update" => {
            let call_id = update
                .get("toolCallId")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("acp-tool-{}", shared.next_sequence()));
            let status =
                update
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or(if kind == "tool_call" {
                        "pending"
                    } else {
                        "in_progress"
                    });
            let (phase, normalized_status) = match status {
                "completed" => (ToolLifecyclePhase::Completed, "completed"),
                "failed" => (ToolLifecyclePhase::Failed, "failed"),
                _ if kind == "tool_call" => (ToolLifecyclePhase::Started, "running"),
                _ => (ToolLifecyclePhase::Updated, "running"),
            };
            let name = update
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string();
            let mut input = update.get("rawInput").cloned();
            if let Some(provenance) = provenance {
                input = Some(json!({"provenance": provenance, "rawInput": input}));
            }
            let output = update
                .get("rawOutput")
                .cloned()
                .or_else(|| update.get("content").cloned());
            shared.emit_tool(&call_id, &name, phase, normalized_status, input, output);
            true
        }
        "plan" => {
            let entries = bounded_payload(update.get("entries").unwrap_or(&Value::Null));
            shared.emit(GenerationProcessEvent::RichBlock(blocks::card(
                "acp_plan",
                "Agent plan",
                plan_markdown(&entries),
                Tone::Info,
                vec![(
                    "Entries",
                    entries.as_array().map_or(0, Vec::len).to_string(),
                )],
                json!({ "entries": entries }),
            )));
            false
        }
        "available_commands_update" => {
            let commands = bounded_payload(update.get("availableCommands").unwrap_or(&Value::Null));
            let names: Vec<String> = commands
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item["name"].as_str())
                        .map(|name| format!("`{name}`"))
                        .collect()
                })
                .unwrap_or_default();
            shared.emit(GenerationProcessEvent::RichBlock(blocks::card(
                "acp_available_commands",
                "Available agent commands",
                names.join(", "),
                Tone::Info,
                vec![("Count", names.len().to_string())],
                json!({ "commands": commands }),
            )));
            false
        }
        "current_mode_update" => {
            let mode_id = update.get("currentModeId").cloned().unwrap_or(Value::Null);
            shared.emit(GenerationProcessEvent::RichBlock(blocks::card(
                "acp_current_mode",
                "Agent mode changed",
                "",
                Tone::Info,
                vec![("Mode", mode_id.as_str().unwrap_or("-").to_string())],
                json!({ "modeId": mode_id }),
            )));
            false
        }
        // Replayed user text during a live turn has nothing to add to the transcript.
        "user_message_chunk" => false,
        _ => false,
    }
}

/// CodeBuddy tags team-member activity in `_meta`. The member is attributed to the owning seat
/// rather than becoming a seat of its own.
fn provider_subagent_provenance(update: &Value) -> Option<Value> {
    let meta = update.get("_meta")?;
    let member = meta
        .get("codebuddy.ai/memberEvent")
        .or_else(|| meta.get("codebuddy.ai/teamUpdate"))?;
    Some(json!({"vendor": "codebuddy", "member": bounded_payload(member)}))
}

/// Renders ACP plan entries as a markdown checklist; unknown shapes fall back to fenced JSON.
fn plan_markdown(entries: &Value) -> String {
    let Some(items) = entries.as_array() else {
        return blocks::fenced_json(entries);
    };
    items
        .iter()
        .map(|entry| {
            let done = entry["status"].as_str() == Some("completed");
            let content = entry["content"].as_str().unwrap_or("(untitled step)");
            let priority = entry["priority"].as_str().unwrap_or("");
            let suffix = if priority.is_empty() {
                String::new()
            } else {
                format!(" _({priority})_")
            };
            format!("- [{}] {content}{suffix}", if done { 'x' } else { ' ' })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Keeps a projected payload bounded: at most 8 KiB of serialized JSON.
fn bounded_payload(value: &Value) -> Value {
    let serialized = value.to_string();
    if serialized.len() <= 8 * 1024 {
        value.clone()
    } else {
        json!({"truncated": true, "bytes": serialized.len()})
    }
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
