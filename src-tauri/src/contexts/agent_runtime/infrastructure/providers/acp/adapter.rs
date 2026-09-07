//! The `AgentProcessGateway` for ACP providers.
//!
//! One execution binding (a session, or a seat inside one) owns one agent process. A turn is a
//! `session/prompt` on that process; the process survives the turn and answers the next one.
//! Cancellation is the protocol's `session/cancel` first and the owned process tree only after
//! the grace period. Decisions from the UI reach the turn through the same `ToolApprovalPort`
//! the native tool loop uses, keyed by the tool call id, so the existing approval and question
//! cards work unchanged.

use super::binding::{
    BindingCompatibility, ExecutionBindingRecord, SqliteExecutionBindingRepository,
};
use super::blocks;
use super::budget::SHUTDOWN_DEADLINE;
use super::connection::{AcpConnection, AcpError, AcpLaunchSpec};
use super::definitions;
use super::environment::child_environment;
use super::interactions::PendingInteractionStore;
use super::proxy_fs::AuthorizedRoots;
use super::proxy_terminal::TerminalRegistry;
use super::session::{
    initialize, load_session, new_session, run_turn, BindingState, HostCapabilities,
    NegotiatedCapabilities, StopReason, TurnOutcome, TurnShared,
};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentPermissionPort,
    AgentProcessEventSink, AgentProcessGateway, AgentProviderError, AgentRuntimeApplicationError,
    GenerationProcessEvent, GenerationProcessFailure, GenerationProcessRequest,
    ManagedConnectionCheckReport, ManagedConnectionCheckRequest, ManagedConnectionControlPort,
    ProcessStopInitiator, ProviderAcpInvocationRequest, ProviderRegistry, RunnerPermissionContext,
    RunnerPermissionPort, RunnerReference, RunnerSelection, StartedGenerationProcess,
    ToolApprovalDecision, ToolApprovalPort, WorkflowLaunchOutcome, WorkflowLaunchRequest,
};
use crate::contexts::agent_runtime::domain::ProviderTransport;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) const ACP_PROCESS_PREFIX: &str = "agent-acp-process-";
/// A binding with no active turn for this long is torn down by the janitor.
const IDLE_BINDING_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// Spawns the agent process. Injected so tests can put an in-process fake on the far end.
pub(crate) trait AcpLauncher: Send + Sync {
    fn launch(&self, spec: &AcpLaunchSpec) -> Result<Arc<AcpConnection>, AcpError>;
}

pub(crate) struct ProcessLauncher;

impl AcpLauncher for ProcessLauncher {
    fn launch(&self, spec: &AcpLaunchSpec) -> Result<Arc<AcpConnection>, AcpError> {
        AcpConnection::spawn(spec)
    }
}

pub(crate) struct AcpAgentProcessDependencies {
    pub(crate) providers: Arc<ProviderRegistry>,
    pub(crate) permissions: Arc<dyn AgentPermissionPort>,
    pub(crate) runner_permissions: Arc<dyn RunnerPermissionPort>,
    pub(crate) logging: Arc<dyn AgentLoggingPort>,
    pub(crate) clock: Arc<dyn AgentClockPort>,
    pub(crate) bindings: Option<SqliteExecutionBindingRepository>,
    pub(crate) launcher: Arc<dyn AcpLauncher>,
}

struct Binding {
    state: Arc<BindingState>,
    installation_fingerprint: String,
    adapter_revision: &'static str,
    account_profile: Option<String>,
    last_used: Instant,
    active_turn: Option<Arc<TurnShared>>,
}

struct ActiveProcess {
    binding_key: String,
    turn: Arc<TurnShared>,
    prompt: String,
    monitoring: bool,
}

#[derive(Clone)]
pub(crate) struct AcpAgentProcessAdapter {
    providers: Arc<ProviderRegistry>,
    permissions: Arc<dyn AgentPermissionPort>,
    runner_permissions: Arc<dyn RunnerPermissionPort>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    bindings_store: Option<SqliteExecutionBindingRepository>,
    launcher: Arc<dyn AcpLauncher>,
    bindings: Arc<Mutex<HashMap<String, Binding>>>,
    processes: Arc<Mutex<HashMap<String, ActiveProcess>>>,
    terminals: Arc<TerminalRegistry>,
    process_ids: Arc<AtomicU64>,
    host: HostCapabilities,
    shutting_down: Arc<AtomicBool>,
}

impl AcpAgentProcessAdapter {
    pub(crate) fn new(dependencies: AcpAgentProcessDependencies) -> Self {
        let adapter = Self {
            providers: dependencies.providers,
            permissions: dependencies.permissions,
            runner_permissions: dependencies.runner_permissions,
            logging: dependencies.logging,
            clock: dependencies.clock,
            bindings_store: dependencies.bindings,
            launcher: dependencies.launcher,
            bindings: Arc::new(Mutex::new(HashMap::new())),
            processes: Arc::new(Mutex::new(HashMap::new())),
            terminals: Arc::new(TerminalRegistry::default()),
            process_ids: Arc::new(AtomicU64::new(0)),
            // Both proxies are fully implemented (create/output/wait/kill/release; read/write),
            // so both are advertised. Flip one to false and the handshake stops advertising it
            // and the handler refuses its methods -- there is no partial state.
            host: HostCapabilities {
                fs: true,
                terminal: true,
            },
            shutting_down: Arc::new(AtomicBool::new(false)),
        };
        adapter.spawn_janitor();
        adapter
    }

    /// Whether `agent_id` is driven through this gateway.
    #[cfg(test)]
    pub(crate) fn handles(&self, agent_id: &str) -> bool {
        self.providers
            .get(agent_id)
            .map(|provider| {
                provider.capabilities().managed_transport() == Some(ProviderTransport::AcpStdio)
            })
            .unwrap_or(false)
    }
}

impl ManagedConnectionControlPort for AcpAgentProcessAdapter {
    fn check_connection(
        &self,
        request: ManagedConnectionCheckRequest,
    ) -> Result<ManagedConnectionCheckReport, String> {
        let provider = self
            .providers
            .get(&request.agent_id)
            .map_err(|error| error.to_string())?;
        if provider.capabilities().managed_transport() != Some(ProviderTransport::AcpStdio) {
            return Err(format!(
                "{} has no managed ACP transport to check.",
                request.agent_id
            ));
        }
        let definition = definitions::definition(&request.agent_id)
            .filter(|definition| definition.acp.is_some())
            .ok_or_else(|| format!("{} has no reviewed ACP grammar.", request.agent_id))?;
        let account_profile = account_profile_for(definition.id, request.provider_id.as_deref());
        let spec = provider
            .prepare_acp(ProviderAcpInvocationRequest {
                executable: request.executable.clone(),
                global_args: &[],
                invocation_args: &[],
                account_profile: account_profile.as_deref(),
            })
            .map_err(|error| error.to_string())?;
        // The same launch boundary a session crosses: a check is still a program start.
        self.runner_permissions
            .authorize(&RunnerPermissionContext {
                agent_id: request.agent_id.clone(),
                session_id: "cli-management".to_string(),
                generation_id: "connection-check".to_string(),
                project_key: request.workspace.clone(),
                action: "shell.exec".to_string(),
                selection: RunnerSelection::local(),
            })
            .map_err(|error| format!("acp-launch:{}", error.code()))?;
        let environment = child_environment(&request.agent_id, std::env::vars(), &spec.environment);
        let launch = AcpLaunchSpec {
            executable: spec.executable.clone(),
            args: spec.args.clone(),
            environment,
            cwd: Some(request.workspace.clone()),
        };
        let started = Instant::now();
        let connection = self
            .launcher
            .launch(&launch)
            .map_err(|error| error.to_string())?;
        let negotiated = initialize(&connection, self.host);
        // Handshake only. Whatever the answer, the process is released here: no session/new,
        // no prompt, nothing left running for a later turn to find.
        let _ = connection.terminate("acp-connection-check-complete", "connection check finished");
        let negotiated = negotiated.map_err(|error| {
            format!(
                "{} ({})",
                with_stderr(&connection, &error),
                error.reason_code()
            )
        })?;
        Ok(ManagedConnectionCheckReport {
            agent_id: request.agent_id,
            transport: ProviderTransport::AcpStdio.as_str().to_string(),
            protocol_version: negotiated.protocol_version,
            load_session: negotiated.load_session,
            agent_name: negotiated.agent_name,
            agent_version: negotiated.agent_version,
            auth_methods: negotiated.auth_methods,
            elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        })
    }

    /// Terminates every binding. Called at application shutdown so no agent outlives the host.
    fn shutdown_all(&self) -> Vec<String> {
        self.shutting_down.store(true, Ordering::SeqCst);
        let drained: Vec<(String, Binding)> = lock(&self.bindings).drain().collect();
        let mut keys = Vec::new();
        for (key, binding) in drained {
            if let Some(turn) = &binding.active_turn {
                turn.cancel.store(true, Ordering::SeqCst);
            }
            let _ = binding
                .state
                .connection
                .terminate("acp-host-shutdown", "application shutdown");
            self.terminals
                .release_epoch(binding.state.connection.epoch());
            keys.push(key);
        }
        keys
    }

    /// Releases every binding of one session: its connections, pending interactions, and proxied
    /// terminals. Nothing outside the session is touched.
    fn release_session(&self, session_id: &str) -> usize {
        let removed: Vec<Binding> = {
            let mut bindings = lock(&self.bindings);
            let keys: Vec<String> = bindings
                .iter()
                .filter(|(_, binding)| binding.state.session_id() == session_id)
                .map(|(key, _)| key.clone())
                .collect();
            keys.iter().filter_map(|key| bindings.remove(key)).collect()
        };
        for binding in &removed {
            if let Some(turn) = &binding.active_turn {
                turn.cancel.store(true, Ordering::SeqCst);
            }
            let _ = binding
                .state
                .connection
                .terminate("acp-session-released", "session released");
            self.terminals
                .release_epoch(binding.state.connection.epoch());
        }
        if let Some(store) = &self.bindings_store {
            let _ = store.delete_session(session_id);
        }
        removed.len()
    }
}

impl AcpAgentProcessAdapter {
    fn spawn_janitor(&self) {
        let bindings = self.bindings.clone();
        let terminals = self.terminals.clone();
        let shutting_down = self.shutting_down.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(60));
            if shutting_down.load(Ordering::SeqCst) {
                return;
            }
            let idle: Vec<Binding> = {
                let mut guard = lock(&bindings);
                let keys: Vec<String> = guard
                    .iter()
                    .filter(|(_, binding)| {
                        binding.active_turn.is_none()
                            && binding.last_used.elapsed() >= IDLE_BINDING_TIMEOUT
                    })
                    .map(|(key, _)| key.clone())
                    .collect();
                keys.iter().filter_map(|key| guard.remove(key)).collect()
            };
            for binding in idle {
                let _ = binding
                    .state
                    .connection
                    .terminate("acp-idle-timeout", "binding idle");
                terminals.release_epoch(binding.state.connection.epoch());
            }
        });
    }

    fn record_log(
        &self,
        level: AgentLogLevel,
        category: &str,
        message: String,
        request: &GenerationProcessRequest,
    ) {
        let _ = self.logging.record(AgentLog {
            level,
            category: category.to_string(),
            message,
            agent_id: Some(request.agent.id.clone()),
            session_id: Some(request.session.id.clone()),
            operation_id: Some(request.operation_id.clone()),
            run_id: Some(request.execution_context.run_id.as_str().to_string()),
            trace_id: Some(request.execution_context.trace_id.as_str().to_string()),
            span_id: Some(request.execution_context.span_id.as_str().to_string()),
            occurred_at: self.clock.now(),
        });
    }

    /// Establishes (or reuses) the binding for this request and returns it with the external
    /// session id the turn will prompt.
    fn bind(
        &self,
        request: &GenerationProcessRequest,
        binding_key: &str,
        seat_id: Option<&str>,
    ) -> Result<Arc<BindingState>, AgentRuntimeApplicationError> {
        let provider = self.providers.get(&request.agent.id)?;
        if provider.capabilities().managed_transport() != Some(ProviderTransport::AcpStdio) {
            return Err(AgentProviderError::UnsupportedCapability {
                provider_id: request.agent.id.clone(),
                capability: "acp-stdio".to_string(),
            }
            .into());
        }
        let definition = definitions::definition(&request.agent.id).ok_or_else(|| {
            AgentRuntimeApplicationError::Process(format!(
                "{} has no ACP grammar in the built-in catalog.",
                request.agent.id
            ))
        })?;
        let grammar = definition.acp.ok_or_else(|| {
            AgentRuntimeApplicationError::Process(format!(
                "{} declares ACP without a reviewed grammar.",
                request.agent.id
            ))
        })?;
        let workspace = canonical_workspace(request.session.folder.as_deref())?;
        let account_profile = account_profile_for(
            &request.agent.id,
            request.configuration.provider_id.as_deref(),
        );
        let spec = provider.prepare_acp(ProviderAcpInvocationRequest {
            executable: request.cli_profile.executable.clone(),
            global_args: &request.cli_profile.global_args,
            invocation_args: &request.cli_profile.invocation_args,
            account_profile: account_profile.as_deref(),
        })?;
        let fingerprint = installation_fingerprint(&spec.executable);

        // Reuse a healthy binding for the same program. A changed fingerprint means the user's
        // installation moved or was upgraded under a live session; the old process is retired
        // rather than trusted to still be the reviewed one.
        {
            let mut bindings = lock(&self.bindings);
            if let Some(existing) = bindings.get_mut(binding_key) {
                let reusable = existing.state.connection.is_open()
                    && existing.state.connection.poll_child_exit().is_none()
                    && existing.installation_fingerprint == fingerprint
                    && existing.adapter_revision == spec.adapter_revision
                    && existing.account_profile == account_profile
                    && existing.state.workspace == workspace
                    && existing.active_turn.is_none()
                    && request.resume_thread_id.as_deref()
                        == Some(existing.state.external_session_id.as_str());
                if reusable {
                    existing.last_used = Instant::now();
                    return Ok(existing.state.clone());
                }
                let retired = bindings.remove(binding_key);
                if let Some(retired) = retired {
                    if retired.active_turn.is_some() {
                        return Err(AgentRuntimeApplicationError::GenerationConflict(
                            "the session's ACP agent is still busy with the previous turn"
                                .to_string(),
                        ));
                    }
                    let _ = retired
                        .state
                        .connection
                        .terminate("acp-binding-retired", "binding replaced");
                    self.terminals
                        .release_epoch(retired.state.connection.epoch());
                }
            }
        }

        // Launch preflight through the same permission boundary every runner launch crosses.
        self.runner_permissions
            .authorize(&RunnerPermissionContext {
                agent_id: request.agent.id.clone(),
                session_id: request.session.id.clone(),
                generation_id: request.operation_id.clone(),
                project_key: workspace.clone(),
                action: "shell.exec".to_string(),
                selection: request.runner.clone(),
            })
            .map_err(|error| AgentRuntimeApplicationError::PolicyDenied {
                session_id: request.session.id.clone(),
                action: format!("acp-launch:{}", error.code()),
            })?;

        let mut environment =
            child_environment(&request.agent.id, std::env::vars(), &spec.environment);
        for (key, value) in &request.cli_profile.env {
            environment.insert(key.clone(), value.clone());
        }
        environment.insert(
            "TRACEPARENT".to_string(),
            request.execution_context.traceparent(),
        );
        let launch = AcpLaunchSpec {
            executable: spec.executable.clone(),
            args: spec.args.clone(),
            environment: environment.clone(),
            cwd: Some(workspace.clone()),
        };
        self.record_log(
            AgentLogLevel::Info,
            "session.runtime.acp",
            format!(
                "starting {} as an ACP agent with {} arguments",
                executable_name(&spec.executable),
                spec.args.len()
            ),
            request,
        );
        let connection = self
            .launcher
            .launch(&launch)
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        let negotiated = match initialize(&connection, self.host) {
            Ok(negotiated) => negotiated,
            Err(error) => {
                let _ = connection.terminate(error.reason_code(), &error.to_string());
                return Err(AgentRuntimeApplicationError::Process(format!(
                    "{} ACP initialize failed ({}): {}",
                    request.agent.id,
                    error.reason_code(),
                    with_stderr(&connection, &error)
                )));
            }
        };

        // Resume only against an exact, compatible binding. Anything else is a new session with
        // the local history kept; the reason is logged and surfaced, never guessed around.
        let stored = self
            .bindings_store
            .as_ref()
            .and_then(|store| store.find(binding_key).ok().flatten());
        let external_session_id = match request.resume_thread_id.as_deref() {
            Some(requested) => {
                let compatibility = stored
                    .as_ref()
                    .filter(|record| record.external_session_id.as_deref() == Some(requested))
                    .map(|record| {
                        record.compatibility(
                            &request.agent.id,
                            ProviderTransport::AcpStdio.as_str(),
                            &fingerprint,
                            spec.adapter_revision,
                            &workspace,
                            account_profile.as_deref(),
                        )
                    })
                    .unwrap_or(BindingCompatibility::Incompatible {
                        reason_code: "binding-record-missing",
                    });
                let reason = match compatibility {
                    BindingCompatibility::Compatible if negotiated.load_session => None,
                    BindingCompatibility::Compatible => Some("acp-peer-cannot-load"),
                    BindingCompatibility::Incompatible { reason_code } => Some(reason_code),
                };
                match reason {
                    None => match load_session(&connection, requested, &workspace) {
                        Ok(summary) => {
                            self.record_log(
                                AgentLogLevel::Info,
                                "session.runtime.acp",
                                format!(
                                    "resumed external session with {} replayed updates",
                                    summary.replayed_updates
                                ),
                                request,
                            );
                            requested.to_string()
                        }
                        Err(error) => {
                            let _ = connection.terminate(error.reason_code(), &error.to_string());
                            return Err(AgentRuntimeApplicationError::Process(format!(
                                "{} could not resume its recorded session ({}): {}",
                                request.agent.id,
                                error.reason_code(),
                                with_stderr(&connection, &error)
                            )));
                        }
                    },
                    Some(reason_code) => {
                        let _ = connection.terminate(reason_code, "resume refused");
                        return Err(AgentRuntimeApplicationError::Process(format!(
                            "{} cannot resume the recorded session ({reason_code}); start a new session to continue.",
                            request.agent.id
                        )));
                    }
                }
            }
            None => match new_session(&connection, &workspace) {
                Ok(id) => id,
                Err(AcpError::AuthRequired { detail }) => {
                    // The program answered correctly; the account is missing. Say so in the
                    // words a person can act on, and never start a sign-in on their behalf.
                    let _ = connection.terminate("acp-authentication-required", &detail);
                    return Err(AgentRuntimeApplicationError::Process(format!(
                        "{} is not signed in ({}). Sign in with the CLI in a terminal, then try again.",
                        request.agent.id, detail
                    )));
                }
                Err(error) => {
                    let _ = connection.terminate(error.reason_code(), &error.to_string());
                    return Err(AgentRuntimeApplicationError::Process(format!(
                        "{} ACP session/new failed ({}): {}",
                        request.agent.id,
                        error.reason_code(),
                        with_stderr(&connection, &error)
                    )));
                }
            },
        };

        let state = Arc::new(BindingState {
            connection,
            external_session_id: external_session_id.clone(),
            negotiated: negotiated.clone(),
            host: self.host,
            grammar,
            roots: AuthorizedRoots::new([Path::new(&workspace).to_path_buf()]),
            workspace: workspace.clone(),
            child_environment: environment,
            session_id: request.session.id.clone(),
        });
        if let Some(store) = &self.bindings_store {
            let record = ExecutionBindingRecord {
                binding_key: binding_key.to_string(),
                session_id: request.session.id.clone(),
                seat_id: seat_id.map(str::to_string),
                provider_id: request.agent.id.clone(),
                transport: ProviderTransport::AcpStdio.as_str().to_string(),
                executable_path: spec.executable.clone(),
                installation_fingerprint: fingerprint.clone(),
                distribution: None,
                adapter_revision: spec.adapter_revision.to_string(),
                external_session_id: Some(external_session_id),
                workspace: workspace.clone(),
                account_profile: account_profile.clone(),
                protocol_version: Some(negotiated.protocol_version as i64),
                peer_load_session: negotiated.load_session,
                last_handshake: Some(negotiated.summary().to_string()),
            };
            if let Err(error) = store.upsert(&record, &self.clock.now()) {
                self.record_log(
                    AgentLogLevel::Warn,
                    "session.runtime.acp",
                    format!("execution binding could not be persisted: {error}"),
                    request,
                );
            }
        }
        lock(&self.bindings).insert(
            binding_key.to_string(),
            Binding {
                state: state.clone(),
                installation_fingerprint: fingerprint,
                adapter_revision: spec.adapter_revision,
                account_profile,
                last_used: Instant::now(),
                active_turn: None,
            },
        );
        Ok(state)
    }

    fn terminal_event(
        &self,
        outcome: &TurnOutcome,
        negotiated: &NegotiatedCapabilities,
    ) -> Vec<GenerationProcessEvent> {
        match outcome {
            TurnOutcome::Completed { stop_reason } => {
                let mut events = Vec::new();
                if *stop_reason != StopReason::EndTurn {
                    events.push(GenerationProcessEvent::RichBlock(blocks::card(
                        "acp_stop_reason",
                        "Agent stopped before end of turn",
                        "The agent reported a stop reason other than `end_turn`. This is not a verified task success.",
                        blocks::Tone::Warning,
                        vec![("Stop reason", stop_reason.as_str().to_string())],
                        json!({
                            "stopReason": stop_reason.as_str(),
                            "verifiedTaskSuccess": false,
                        }),
                    )));
                }
                // Usage is unavailable on this transport by declaration: `None`, never zero.
                events.push(GenerationProcessEvent::Completed(None));
                let _ = negotiated;
                events
            }
            TurnOutcome::Cancelled => vec![GenerationProcessEvent::Failed(
                GenerationProcessFailure::non_retryable("ACP turn cancelled (stopReason=cancelled).")
                    .with_safe_error("cancelled"),
            )],
            TurnOutcome::ForcedCancel => vec![GenerationProcessEvent::Failed(
                GenerationProcessFailure::non_retryable(
                    "ACP agent ignored session/cancel; its process tree was terminated.",
                )
                .with_safe_error("cancelled"),
            )],
            TurnOutcome::Interrupted { reason_code, detail } => vec![
                GenerationProcessEvent::RichBlock(blocks::card(
                    "acp_interrupted",
                    "Agent connection interrupted",
                    "The connection dropped mid-turn. Any effects the agent had already applied are unknown, and the prompt was not resent.",
                    blocks::Tone::Danger,
                    vec![("Reason", reason_code.to_string())],
                    json!({ "reasonCode": reason_code, "effectsUnknown": true }),
                )),
                GenerationProcessEvent::Failed(
                    GenerationProcessFailure::non_retryable(format!(
                        "ACP connection interrupted ({reason_code}): {detail}"
                    ))
                    .with_safe_error("The agent connection was interrupted; its effects are unknown and the prompt was not resent."),
                ),
            ],
            TurnOutcome::Failed { reason_code, detail } => vec![GenerationProcessEvent::Failed(
                GenerationProcessFailure::non_retryable(format!("ACP turn failed ({reason_code}): {detail}")),
            )],
        }
    }
}

impl AgentProcessGateway for AcpAgentProcessAdapter {
    fn launch_workflow(
        &self,
        request: WorkflowLaunchRequest,
    ) -> Result<WorkflowLaunchOutcome, AgentRuntimeApplicationError> {
        // Interactive launches are the terminal path, which the CLI adapter owns.
        Err(AgentRuntimeApplicationError::UnsupportedInteractionMode(
            request.interaction_mode.as_str().to_string(),
        ))
    }

    fn start_generation(
        &self,
        request: GenerationProcessRequest,
    ) -> Result<StartedGenerationProcess, AgentRuntimeApplicationError> {
        if self.shutting_down.load(Ordering::SeqCst) {
            return Err(AgentRuntimeApplicationError::Process(
                "the runtime is shutting down".to_string(),
            ));
        }
        let seat_id = seat_for(&request);
        let binding_key = match &seat_id {
            Some(seat) => format!("{}/{seat}", request.session.id),
            None => format!("{}/{}", request.session.id, request.agent.id),
        };
        let state = self.bind(&request, &binding_key, seat_id.as_deref())?;
        let turn = Arc::new(TurnShared {
            turn_id: request.operation_id.clone(),
            session_id: request.session.id.clone(),
            agent_id: request.agent.id.clone(),
            operation_id: request.operation_id.clone(),
            project_key: state.workspace.clone(),
            interactive: request.interactive,
            binding: state.clone(),
            sink: Arc::new(NullSink),
            permissions: self.permissions.clone(),
            interactions: Mutex::new(PendingInteractionStore::default()),
            terminals: self.terminals.clone(),
            cancel: AtomicBool::new(false),
            sequence: AtomicU64::new(0),
        });
        {
            let mut bindings = lock(&self.bindings);
            let Some(binding) = bindings.get_mut(&binding_key) else {
                return Err(AgentRuntimeApplicationError::Process(
                    "the ACP binding disappeared before the turn started".to_string(),
                ));
            };
            if binding.active_turn.is_some() {
                return Err(AgentRuntimeApplicationError::GenerationConflict(
                    "the session's ACP agent already has an active turn".to_string(),
                ));
            }
            binding.active_turn = Some(turn.clone());
            binding.last_used = Instant::now();
        }
        let process_id = format!(
            "{ACP_PROCESS_PREFIX}{}",
            self.process_ids.fetch_add(1, Ordering::Relaxed) + 1
        );
        let process_reference = state.connection.process_id().map(|pid| pid.to_string());
        lock(&self.processes).insert(
            process_id.clone(),
            ActiveProcess {
                binding_key,
                turn,
                prompt: request.effective_prompt.clone(),
                monitoring: false,
            },
        );
        Ok(StartedGenerationProcess {
            process_id,
            runner_reference: RunnerReference::local(),
            process_reference,
        })
    }

    fn monitor_generation(
        &self,
        process_id: &str,
        sink: Arc<dyn AgentProcessEventSink>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let (binding_key, turn, prompt) = {
            let mut processes = lock(&self.processes);
            let process = processes.get_mut(process_id).ok_or_else(|| {
                AgentRuntimeApplicationError::Process(format!(
                    "Agent process {process_id} is not active."
                ))
            })?;
            if process.monitoring {
                return Err(AgentRuntimeApplicationError::Process(format!(
                    "Agent process {process_id} is already monitored."
                )));
            }
            process.monitoring = true;
            (
                process.binding_key.clone(),
                process.turn.clone(),
                process.prompt.clone(),
            )
        };
        // The sink arrives with monitoring, after the turn was created; rebind it here.
        let turn = Arc::new(TurnShared {
            turn_id: turn.turn_id.clone(),
            session_id: turn.session_id.clone(),
            agent_id: turn.agent_id.clone(),
            operation_id: turn.operation_id.clone(),
            project_key: turn.project_key.clone(),
            interactive: turn.interactive,
            binding: turn.binding.clone(),
            sink,
            permissions: turn.permissions.clone(),
            interactions: Mutex::new(PendingInteractionStore::default()),
            terminals: turn.terminals.clone(),
            cancel: AtomicBool::new(turn.cancel.load(Ordering::SeqCst)),
            sequence: AtomicU64::new(0),
        });
        {
            let mut processes = lock(&self.processes);
            if let Some(process) = processes.get_mut(process_id) {
                process.turn = turn.clone();
            }
            let mut bindings = lock(&self.bindings);
            if let Some(binding) = bindings.get_mut(&binding_key) {
                binding.active_turn = Some(turn.clone());
            }
        }
        let adapter = self.clone();
        let process_id = process_id.to_string();
        thread::spawn(move || {
            // The external id is recorded before the first token so a crash mid-turn still
            // leaves the seat bound to the right session.
            let _ = turn.sink.handle(GenerationProcessEvent::RuntimeSessionId(
                turn.binding.external_session_id.clone(),
            ));
            let outcome = run_turn(&turn, &prompt);
            let connection_open = turn.binding.connection.is_open();
            for event in adapter.terminal_event(&outcome, &turn.binding.negotiated) {
                let _ = turn.sink.handle(event);
            }
            let _ = adapter.logging.record(AgentLog {
                level: match &outcome {
                    TurnOutcome::Completed { .. } | TurnOutcome::Cancelled => AgentLogLevel::Info,
                    _ => AgentLogLevel::Warn,
                },
                category: "session.runtime.acp".to_string(),
                message: format!(
                    "turn finished: {} (pending interactions: {}, late responses: {})",
                    outcome_label(&outcome),
                    turn.pending_count(),
                    turn.binding.connection.late_responses()
                ),
                agent_id: Some(turn.agent_id.clone()),
                session_id: Some(turn.session_id.clone()),
                operation_id: Some(turn.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: adapter.clock.now(),
            });
            lock(&adapter.processes).remove(&process_id);
            let mut bindings = lock(&adapter.bindings);
            if let Some(binding) = bindings.get_mut(&binding_key) {
                binding.active_turn = None;
                binding.last_used = Instant::now();
            }
            if !connection_open {
                if let Some(retired) = bindings.remove(&binding_key) {
                    adapter
                        .terminals
                        .release_epoch(retired.state.connection.epoch());
                }
            }
        });
        Ok(())
    }

    fn stop_generation(
        &self,
        process_id: &str,
        initiator: ProcessStopInitiator,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let (turn, monitoring, binding_key) = {
            let processes = lock(&self.processes);
            let Some(process) = processes.get(process_id) else {
                return Ok(false);
            };
            (
                process.turn.clone(),
                process.monitoring,
                process.binding_key.clone(),
            )
        };
        turn.cancel.store(true, Ordering::SeqCst);
        if !monitoring {
            // Nothing is driving the turn, so nothing will honour the flag: retire the binding.
            lock(&self.processes).remove(process_id);
            if let Some(binding) = lock(&self.bindings).remove(&binding_key) {
                let _ = binding
                    .state
                    .connection
                    .terminate("acp-stop-before-monitor", initiator.as_str());
                self.terminals
                    .release_epoch(binding.state.connection.epoch());
            }
            return Ok(true);
        }
        if initiator == ProcessStopInitiator::RuntimeCleanup {
            // Shutdown cannot wait for the cooperative grace: terminate now, the driver observes
            // the closed connection and finishes.
            let _ = turn
                .binding
                .connection
                .terminate("acp-runtime-cleanup", initiator.as_str());
        }
        let deadline = Instant::now() + SHUTDOWN_DEADLINE;
        while Instant::now() < deadline {
            if !lock(&self.processes).contains_key(process_id) {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        Ok(true)
    }
}

impl ToolApprovalPort for AcpAgentProcessAdapter {
    fn resolve(
        &self,
        process_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let turn = {
            let processes = lock(&self.processes);
            let Some(process) = processes.get(process_id) else {
                return Ok(false);
            };
            process.turn.clone()
        };
        turn.resolve(call_id, decision)
            .map_err(AgentRuntimeApplicationError::Permission)
    }
}

/// A sink for the window between `start_generation` and `monitor_generation`, when no consumer
/// exists yet. Nothing is emitted in that window.
struct NullSink;

impl AgentProcessEventSink for NullSink {
    fn handle(&self, _event: GenerationProcessEvent) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

fn outcome_label(outcome: &TurnOutcome) -> String {
    match outcome {
        TurnOutcome::Completed { stop_reason } => format!("completed ({})", stop_reason.as_str()),
        TurnOutcome::Cancelled => "cancelled".to_string(),
        TurnOutcome::ForcedCancel => "forced-cancel".to_string(),
        TurnOutcome::Interrupted { reason_code, .. } => format!("interrupted ({reason_code})"),
        TurnOutcome::Failed { reason_code, .. } => format!("failed ({reason_code})"),
    }
}

fn with_stderr(connection: &AcpConnection, error: &AcpError) -> String {
    let stderr = connection.stderr_summary();
    if stderr.redacted_tail.trim().is_empty() {
        error.to_string()
    } else {
        format!("{error} | stderr: {}", stderr.redacted_tail.trim())
    }
}

fn executable_name(executable: &str) -> String {
    Path::new(executable)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| executable.to_string())
}

/// The workspace an ACP session is created in. The protocol requires an absolute path, and the
/// fs/terminal proxies use it as their authorized root, so a session without one cannot start.
fn canonical_workspace(folder: Option<&str>) -> Result<String, AgentRuntimeApplicationError> {
    let folder = folder
        .map(str::trim)
        .filter(|folder| !folder.is_empty())
        .ok_or_else(|| {
            AgentRuntimeApplicationError::Process(
                "an ACP conversation requires a workspace directory".to_string(),
            )
        })?;
    let path = Path::new(folder);
    if !path.is_absolute() {
        return Err(AgentRuntimeApplicationError::Process(
            "the ACP workspace directory must be absolute".to_string(),
        ));
    }
    let canonical = std::fs::canonicalize(path).map_err(|error| {
        AgentRuntimeApplicationError::Process(format!(
            "the ACP workspace directory is not accessible: {error}"
        ))
    })?;
    Ok(
        crate::platform::filesystem::normalize_windows_extended_length_path(
            &canonical.to_string_lossy(),
        ),
    )
}

/// A cheap identity for the executable actually launched: path, size, and modification time.
/// Enough to notice an upgrade or a moved installation between turns; not a content hash, which
/// would cost a full read of a multi-megabyte binary per turn.
pub(crate) fn installation_fingerprint(executable: &str) -> String {
    let metadata = std::fs::metadata(executable).ok();
    let size = metadata.as_ref().map(|meta| meta.len()).unwrap_or(0);
    let modified = metadata
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    crate::platform::hashing::sha256_tagged(format!("v1\0{executable}\0{size}\0{modified}"))
}

/// The seat this turn speaks for, when the session has seats. Matches by agent and by the
/// thread being resumed, so two seats of the same provider bind to two processes.
fn seat_for(request: &GenerationProcessRequest) -> Option<String> {
    let seats: Vec<_> = request
        .session
        .seats
        .iter()
        .filter(|seat| seat.agent_id == request.agent.id && seat.left_at.is_none())
        .collect();
    if seats.is_empty() {
        return None;
    }
    if let Some(thread) = request.resume_thread_id.as_deref() {
        if let Some(seat) = seats
            .iter()
            .find(|seat| seat.provider_thread_id.as_deref() == Some(thread))
        {
            return Some(seat.seat_id.clone());
        }
    }
    seats
        .iter()
        .find(|seat| seat.provider_thread_id.is_none())
        .or(seats.first())
        .map(|seat| seat.seat_id.clone())
}

/// The reviewed account profile a chat configuration selects. Only CodeBuddy distinguishes
/// environments; its provider id carries the choice (`codebuddy-china`, `codebuddy-ioa`), and
/// anything else is the vendor default.
fn account_profile_for(agent_id: &str, provider_id: Option<&str>) -> Option<String> {
    if agent_id != "codebuddy-code" {
        return None;
    }
    match provider_id {
        Some("codebuddy-china") => Some("china".to_string()),
        Some("codebuddy-ioa") => Some("ioa".to_string()),
        _ => Some("international".to_string()),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
pub(crate) fn test_dependencies(
    providers: Arc<ProviderRegistry>,
    permissions: Arc<dyn AgentPermissionPort>,
    runner_permissions: Arc<dyn RunnerPermissionPort>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    launcher: Arc<dyn AcpLauncher>,
) -> AcpAgentProcessDependencies {
    AcpAgentProcessDependencies {
        providers,
        permissions,
        runner_permissions,
        logging,
        clock,
        bindings: None,
        launcher,
    }
}
