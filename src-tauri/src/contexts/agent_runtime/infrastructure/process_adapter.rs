use super::providers::{
    add_codex_output_capture_args, output_parser_for_format, BoundedProviderLines,
    ProviderOutputEvent, ProviderPromptDelivery, ProviderReportedUsage, ProviderToolEvent,
    ProviderToolPhase, ProviderUsageOverlap,
};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentProcessEventSink,
    AgentProcessGateway, AgentRunner, AgentRuntimeApplicationError, AgentUsageOverlap,
    GenerationProcessEvent, GenerationProcessFailure, GenerationProcessRequest,
    ProviderGenerationInvocationRequest, ProviderInvocationSpec, ProviderOutputFormat,
    ProviderRegistry, ReportedUsageTotals, RunnerEvent, RunnerHandle, RunnerLaunchSpec,
    RunnerPermissionContext, RunnerPermissionPort, RunnerSelection, StartedGenerationProcess,
    ToolLifecycleEvent, ToolLifecyclePhase, ToolUseBlock, WorkflowLaunchOutcome,
    WorkflowLaunchRequest,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, InteractionMode, ProviderCapability,
};
use crate::contexts::execution_observability::api::{
    ExecutionContext, ExecutionEvent, ExecutionFidelity, ExecutionIdentityPort, ExecutionSpan,
    ExecutionStatus, ExecutionTelemetryPort, SafeAttributeValue, SafeAttributes,
};
use crate::contexts::skill_evolution_evidence::application::{
    CliLifecycleFact, CliRuntimeKind, RuntimeEvidenceProjector,
};
use crate::contexts::skill_evolution_evidence::domain::{
    CliMountSnapshot, EnvelopeCommon, FailureClass, MountedSkillRevision, SourceFidelity,
    TerminalOutcome,
};
use crate::contexts::tooling::skills::api::{CliSkillEvidenceSnapshot, SkillApi};
use crate::platform::private_relay_fs::PreparedMcpRelayGuard;
use crate::platform::process;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct RuntimeAgentProcessAdapter {
    processes: Arc<Mutex<HashMap<String, ManagedProcess>>>,
    runner: Arc<dyn AgentRunner>,
    runner_permissions: Arc<dyn RunnerPermissionPort>,
    process_ids: Arc<AtomicU64>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    execution_ids: Arc<dyn ExecutionIdentityPort>,
    telemetry: Arc<dyn ExecutionTelemetryPort>,
    mcp_relay: Arc<dyn ManagedMcpRelayPort>,
    providers: Arc<ProviderRegistry>,
    event_sequence: Arc<AtomicU64>,
    evidence: RuntimeEvidenceProjector,
    skills: SkillApi,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedMcpRelay {
    pub(crate) invocation_args: Vec<String>,
    pub(crate) guard: Option<PreparedMcpRelayGuard>,
}

#[derive(Clone)]
pub(crate) struct RuntimeProcessEvidenceDependencies {
    pub(crate) evidence: RuntimeEvidenceProjector,
    pub(crate) skills: SkillApi,
}

#[derive(Clone)]
pub(crate) struct RuntimeAgentProcessDependencies {
    pub(crate) logging: Arc<dyn AgentLoggingPort>,
    pub(crate) clock: Arc<dyn AgentClockPort>,
    pub(crate) execution_ids: Arc<dyn ExecutionIdentityPort>,
    pub(crate) telemetry: Arc<dyn ExecutionTelemetryPort>,
    pub(crate) mcp_relay: Arc<dyn ManagedMcpRelayPort>,
    pub(crate) providers: Arc<ProviderRegistry>,
    pub(crate) runner: Arc<dyn AgentRunner>,
    pub(crate) runner_permissions: Arc<dyn RunnerPermissionPort>,
    pub(crate) evidence: RuntimeProcessEvidenceDependencies,
}

pub(crate) trait ManagedMcpRelayPort: Send + Sync {
    fn prepare(
        &self,
        agent_id: &str,
        project_path: Option<&str>,
        context: &ExecutionContext,
    ) -> Result<PreparedMcpRelay, String>;
}

struct ManagedProcess {
    runner_handle: RunnerHandle,
    agent_id: String,
    session_id: String,
    operation_id: String,
    monitoring: bool,
    final_output_path: Option<PathBuf>,
    relay_guard: Option<PreparedMcpRelayGuard>,
    execution_context: ExecutionContext,
    output_format: ProviderOutputFormat,
    message_id: String,
    workspace: Option<String>,
    mount_snapshot: Option<CliMountSnapshot>,
    configured_binding_ids: Vec<String>,
}

struct ProcessMonitor {
    runner: Arc<dyn AgentRunner>,
    runner_handle: RunnerHandle,
    agent_id: String,
    sink: Arc<dyn AgentProcessEventSink>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    session_id: String,
    operation_id: String,
    final_output_path: Option<PathBuf>,
    relay_guard: Option<PreparedMcpRelayGuard>,
    execution_context: ExecutionContext,
    telemetry: Arc<dyn ExecutionTelemetryPort>,
    event_sequence: Arc<AtomicU64>,
    output_format: ProviderOutputFormat,
    message_id: String,
    workspace: Option<String>,
    evidence: RuntimeEvidenceProjector,
    mount_snapshot: Option<CliMountSnapshot>,
    configured_binding_ids: Vec<String>,
}

impl RuntimeAgentProcessAdapter {
    pub(crate) fn new(dependencies: RuntimeAgentProcessDependencies) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            runner: dependencies.runner,
            runner_permissions: dependencies.runner_permissions,
            process_ids: Arc::new(AtomicU64::new(0)),
            logging: dependencies.logging,
            clock: dependencies.clock,
            execution_ids: dependencies.execution_ids,
            telemetry: dependencies.telemetry,
            mcp_relay: dependencies.mcp_relay,
            providers: dependencies.providers,
            event_sequence: Arc::new(AtomicU64::new(0)),
            evidence: dependencies.evidence.evidence,
            skills: dependencies.evidence.skills,
        }
    }

    fn start_cli_generation(
        &self,
        request: GenerationProcessRequest,
    ) -> Result<StartedGenerationProcess, AgentRuntimeApplicationError> {
        if request.agent.launch.kind != "cli" {
            return Err(AgentRuntimeApplicationError::Process(format!(
                "{} launch kind '{}' is unsupported for chat runtime.",
                request.agent.display_name, request.agent.launch.kind
            )));
        }
        let skill_snapshot = self
            .skills
            .cli_evidence_snapshot(&request.agent.id, request.session.folder.as_deref())
            .ok();
        let (mount_snapshot, configured_binding_ids) = evidence_cli_snapshot(skill_snapshot);
        let executable =
            normalize_generation_executable(&request.agent.id, &request.cli_profile.executable);
        let provider = self
            .providers
            .require(&request.agent.id, ProviderCapability::StructuredOutput)?;
        if request.session.runtime_session_id.is_some() {
            self.providers
                .require(&request.agent.id, ProviderCapability::Resume)?;
        }
        let provider_session = self.providers.resolve_session(
            &request.agent.id,
            request.session.runtime_session_id.as_deref(),
        )?;
        let output_format = provider.output_format();
        let mut spec = provider.prepare_generation(ProviderGenerationInvocationRequest {
            executable,
            prompt: &request.effective_prompt,
            provider_session: provider_session.as_ref(),
            managed_args: &request.cli_profile.managed_args,
            role_briefing: request.role_briefing.as_deref(),
        })?;
        let mut relay_guard = None;
        if request.execution_context.mcp_relay_enabled {
            match self.mcp_relay.prepare(
                &request.agent.id,
                request.session.folder.as_deref(),
                &request.execution_context,
            ) {
                Ok(prepared) => {
                    apply_mcp_relay_args(
                        &request.agent.id,
                        &mut spec.args,
                        prepared.invocation_args,
                    );
                    relay_guard = prepared.guard;
                }
                Err(error) => {
                    self.record_log(
                        AgentLogLevel::Warn,
                        "session.runtime.mcp_relay",
                        format!("managed MCP relay unavailable; continuing without relay: {error}"),
                        Some(&request.agent.id),
                        Some(&request.session.id),
                        Some(&request.operation_id),
                    );
                }
            }
        }
        let final_output_path = if request.agent.id == "codex-cli" {
            let path = codex_output_capture_path(&request.session.id, &request.operation_id);
            add_codex_output_capture_args(&mut spec.args, &path.to_string_lossy());
            Some(path)
        } else {
            None
        };
        let runner_spec = local_runner_launch_spec(
            &spec,
            Some(request.session.id.clone()),
            request.session.folder.clone(),
            request.execution_context.traceparent(),
        );
        self.record_log(
            AgentLogLevel::Info,
            "session.runtime.cli",
            provider_execution_summary(&spec),
            Some(&request.agent.id),
            Some(&request.session.id),
            Some(&request.operation_id),
        );

        let process_context = ExecutionContext {
            run_id: request.execution_context.run_id.clone(),
            trace_id: request.execution_context.trace_id.clone(),
            span_id: self.execution_ids.next_span_id(),
            capture_policy: request.execution_context.capture_policy,
            sampling_per_million: request.execution_context.sampling_per_million,
            mcp_relay_enabled: request.execution_context.mcp_relay_enabled,
        };
        let _ = self.telemetry.start_span(&ExecutionSpan {
            context: process_context.clone(),
            parent_span_id: Some(request.execution_context.span_id.clone()),
            name: format!("vanehub.process.run {}", request.agent.id),
            status: ExecutionStatus::Running,
            fidelity: ExecutionFidelity::Native,
            started_at: self.clock.now(),
            ended_at: None,
            error_classification: None,
            attributes: safe_attributes([
                (
                    "process.executable.name".to_string(),
                    SafeAttributeValue::String(executable_name(&spec.executable)),
                ),
                (
                    "vanehub.agent.id".to_string(),
                    SafeAttributeValue::String(request.agent.id.clone()),
                ),
            ]),
            links: Vec::new(),
        });
        let selection = request.runner.clone();
        let permission_context = RunnerPermissionContext {
            agent_id: request.agent.id.clone(),
            session_id: request.session.id.clone(),
            generation_id: request.operation_id.clone(),
            project_key: request.session.folder.clone().unwrap_or_default(),
            action: "shell.exec".to_string(),
            selection: selection.clone(),
        };
        self.record_runner_log(
            AgentLogLevel::Debug,
            "prepare",
            selection.kind,
            selection.target_id.as_deref().unwrap_or("local"),
            "started",
            &request,
        );
        let runner_handle = match prepare_and_spawn_authorized(
            self.runner.as_ref(),
            self.runner_permissions.as_ref(),
            &selection,
            &permission_context,
            runner_spec,
        ) {
            Ok(handle) => handle,
            Err(error) => {
                // The detail names the constraint that rejected the launch. Without it the log
                // says only `runner_invalid_launch`, which is not something anyone can act on.
                self.record_runner_log(
                    AgentLogLevel::Error,
                    "spawn",
                    selection.kind,
                    selection.target_id.as_deref().unwrap_or("local"),
                    &runner_failure_category(&error),
                    &request,
                );
                let _ = self.telemetry.finish_span(
                    &process_context.run_id,
                    &process_context.span_id,
                    ExecutionStatus::Failed,
                    &self.clock.now(),
                    Some("process_spawn_failed"),
                );
                return Err(runner_application_error(error));
            }
        };
        self.record_runner_log(
            AgentLogLevel::Info,
            "spawn",
            runner_handle.reference.kind,
            &runner_handle.reference.target_id,
            "succeeded",
            &request,
        );
        self.record_process_event(
            &process_context,
            "process.spawned",
            safe_attributes([(
                "vanehub.runner.handle".to_string(),
                SafeAttributeValue::String(runner_handle.id.clone()),
            )]),
        );
        if let Some(input) = provider_prompt_input(&spec, &request.effective_prompt) {
            if let Err(error) = self.runner.send_input(&runner_handle, &input) {
                let _ = self.runner.cancel(&runner_handle);
                let _ = self.runner.cleanup(&runner_handle);
                let _ = self.telemetry.finish_span(
                    &process_context.run_id,
                    &process_context.span_id,
                    ExecutionStatus::Failed,
                    &self.clock.now(),
                    Some("process_stdin_failed"),
                );
                return Err(runner_application_error(error));
            }
        }
        let process_id = format!(
            "agent-process-{}",
            self.process_ids.fetch_add(1, Ordering::Relaxed) + 1
        );
        let runner_reference = runner_handle.reference.clone();
        let process_reference = runner_handle.process_reference.clone();
        let mut processes = match self.processes.lock() {
            Ok(processes) => processes,
            Err(error) => {
                let _ = self.runner.cancel(&runner_handle);
                let _ = self.runner.cleanup(&runner_handle);
                let _ = self.telemetry.finish_span(
                    &process_context.run_id,
                    &process_context.span_id,
                    ExecutionStatus::Failed,
                    &self.clock.now(),
                    Some("process_registration_failed"),
                );
                return Err(AgentRuntimeApplicationError::Process(error.to_string()));
            }
        };
        let managed = ManagedProcess {
            runner_handle,
            agent_id: request.agent.id,
            session_id: request.session.id,
            operation_id: request.operation_id,
            monitoring: false,
            final_output_path,
            relay_guard,
            execution_context: process_context,
            output_format,
            message_id: request.message_id,
            workspace: self
                .evidence
                .workspace_scope(request.session.folder.as_deref()),
            mount_snapshot,
            configured_binding_ids,
        };
        processes.insert(process_id.clone(), managed);
        Ok(StartedGenerationProcess {
            process_id,
            runner_reference,
            process_reference,
        })
    }

    fn record_log(
        &self,
        level: AgentLogLevel,
        category: &str,
        message: String,
        agent_id: Option<&str>,
        session_id: Option<&str>,
        operation_id: Option<&str>,
    ) {
        let _ = self.logging.record(AgentLog {
            level,
            category: category.to_string(),
            message,
            agent_id: agent_id.map(str::to_string),
            session_id: session_id.map(str::to_string),
            operation_id: operation_id.map(str::to_string),
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: self.clock.now(),
        });
    }

    fn record_process_event(
        &self,
        context: &ExecutionContext,
        name: &str,
        attributes: SafeAttributes,
    ) {
        let _ = self.telemetry.record_event(&ExecutionEvent {
            run_id: context.run_id.clone(),
            span_id: context.span_id.clone(),
            sequence: self.event_sequence.fetch_add(1, Ordering::Relaxed) + 1,
            name: name.to_string(),
            timestamp: self.clock.now(),
            attributes,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn record_runner_log(
        &self,
        level: AgentLogLevel,
        action: &str,
        kind: crate::contexts::agent_runtime::application::RunnerKind,
        target_id: &str,
        category: &str,
        request: &GenerationProcessRequest,
    ) {
        record_runner_lifecycle(
            self.logging.as_ref(),
            self.clock.as_ref(),
            level,
            action,
            kind,
            target_id,
            category,
            &request.agent.id,
            &request.session.id,
            &request.operation_id,
            &request.execution_context,
        );
    }
}

fn combined_authority_witness(runner: &str, policy: &str) -> String {
    let digest = Sha256::digest(format!("v1\0{runner}\0{policy}").as_bytes());
    let encoded: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{encoded}")
}

fn prepare_and_spawn_authorized(
    runner: &dyn AgentRunner,
    permissions: &dyn RunnerPermissionPort,
    selection: &RunnerSelection,
    permission_context: &RunnerPermissionContext,
    spec: RunnerLaunchSpec,
) -> Result<RunnerHandle, crate::contexts::agent_runtime::application::RunnerError> {
    let permission_witness = permissions.authorize(permission_context)?;
    let mut prepared = runner.prepare(selection, spec)?;
    permissions.revalidate(permission_context, &permission_witness)?;
    prepared.reference.authority_witness = combined_authority_witness(
        &prepared.reference.authority_witness,
        &permission_witness.fingerprint,
    );
    runner.spawn(prepared)
}

#[allow(clippy::too_many_arguments)]
/// `code` alone, or `code (detail)` when the error names the constraint it tripped.
fn runner_failure_category(
    error: &crate::contexts::agent_runtime::application::RunnerError,
) -> String {
    match error.detail() {
        Some(detail) => format!("{} ({detail})", error.code()),
        None => error.code().to_string(),
    }
}

// Correlation identity (agent, session, operation, run) is what a lifecycle line is for, so the
// parameter list is long by construction; the wrapper above suppresses the same lint for the same
// list. Every caller is in this file, which is what keeps positional passing checkable.
#[allow(clippy::too_many_arguments)]
fn record_runner_lifecycle(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    level: AgentLogLevel,
    action: &str,
    kind: crate::contexts::agent_runtime::application::RunnerKind,
    target_id: &str,
    category: &str,
    agent_id: &str,
    session_id: &str,
    operation_id: &str,
    context: &ExecutionContext,
) {
    let _ = logging.record(AgentLog {
        level,
        category: "agent.runner.lifecycle".to_string(),
        message: format!(
            "action={action} runner={} target={target_id} category={category} attempt=1",
            kind.as_str()
        ),
        agent_id: Some(agent_id.to_string()),
        session_id: Some(session_id.to_string()),
        operation_id: Some(operation_id.to_string()),
        run_id: Some(context.run_id.as_str().to_string()),
        trace_id: Some(context.trace_id.as_str().to_string()),
        span_id: Some(context.span_id.as_str().to_string()),
        occurred_at: clock.now(),
    });
}

pub(in crate::contexts::agent_runtime::infrastructure) fn local_runner_launch_spec(
    provider: &ProviderInvocationSpec,
    session_id: Option<String>,
    cwd: Option<String>,
    traceparent: String,
) -> RunnerLaunchSpec {
    RunnerLaunchSpec {
        session_id,
        executable: provider.executable.clone(),
        arguments: provider.args.clone(),
        cwd: cwd.filter(|value| !value.trim().is_empty()),
        environment: std::collections::BTreeMap::from([("TRACEPARENT".to_string(), traceparent)]),
        pipe_stdin: provider.prompt_delivery == ProviderPromptDelivery::Stdin,
    }
}

pub(in crate::contexts::agent_runtime::infrastructure) fn provider_prompt_input(
    provider: &ProviderInvocationSpec,
    prompt: &str,
) -> Option<Vec<u8>> {
    (provider.prompt_delivery == ProviderPromptDelivery::Stdin).then(|| {
        let mut input = prompt.as_bytes().to_vec();
        input.push(b'\n');
        input
    })
}

fn provider_execution_summary(provider: &ProviderInvocationSpec) -> String {
    format!(
        "executing {} with {} arguments",
        executable_name(&provider.executable),
        provider.args.len()
    )
}

impl AgentProcessGateway for RuntimeAgentProcessAdapter {
    fn launch_workflow(
        &self,
        request: WorkflowLaunchRequest,
    ) -> Result<WorkflowLaunchOutcome, AgentRuntimeApplicationError> {
        if !request
            .agent
            .supported_interaction_modes
            .contains(&request.interaction_mode)
        {
            return Err(AgentRuntimeApplicationError::UnsupportedInteractionMode(
                request.interaction_mode.as_str().to_string(),
            ));
        }
        if request.agent.availability != AgentAvailability::Available {
            return Err(AgentRuntimeApplicationError::AgentUnavailable(
                request
                    .agent
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "Agent is not available.".to_string()),
            ));
        }
        let (adapter, message) = match request.interaction_mode {
            InteractionMode::Browser => {
                ("browser", "Browser workflow routed to Playwright adapter.")
            }
            InteractionMode::NativeDesktop => {
                launch_command(request.agent.launch.command.as_deref())?;
                (
                    "native-desktop",
                    "Native desktop workflow launch routed through Tauri adapter.",
                )
            }
            InteractionMode::Cli => {
                launch_command(request.agent.launch.command.as_deref())?;
                ("cli", "CLI workflow launch routed through Tauri adapter.")
            }
            InteractionMode::Api => {
                return Err(AgentRuntimeApplicationError::UnsupportedInteractionMode(
                    InteractionMode::Api.as_str().to_string(),
                ))
            }
        };
        self.record_log(
            AgentLogLevel::Info,
            "agent.launch",
            message.to_string(),
            Some(&request.agent.id),
            None,
            Some(&request.operation_id),
        );
        if request.interaction_mode == InteractionMode::Cli {
            let (mount_snapshot, configured_binding_ids) = evidence_cli_snapshot(
                self.skills
                    .cli_evidence_snapshot(&request.agent.id, None)
                    .ok(),
            );
            let _ = self.evidence.cli(CliLifecycleFact {
                kind: CliRuntimeKind::Interactive,
                common: EnvelopeCommon {
                    source_event_id: format!("cli-launch:{}", request.operation_id),
                    occurred_at: self.clock.now(),
                    stable_agent_id: Some(request.agent.id.clone()),
                    session_id: None,
                    message_id: None,
                    run_id: None,
                    attempt_id: Some(request.operation_id.clone()),
                    workspace: None,
                    fidelity: SourceFidelity::Opaque,
                    observed_skill_revisions: Vec::new(),
                },
                outcome: TerminalOutcome::Succeeded,
                failure_class: None,
                mount_snapshot,
                configured_binding_ids,
            });
        }
        Ok(WorkflowLaunchOutcome {
            adapter: adapter.to_string(),
            message: message.to_string(),
        })
    }

    fn start_generation(
        &self,
        request: GenerationProcessRequest,
    ) -> Result<StartedGenerationProcess, AgentRuntimeApplicationError> {
        self.start_cli_generation(request)
    }

    fn monitor_generation(
        &self,
        process_id: &str,
        sink: Arc<dyn AgentProcessEventSink>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let (
            runner_handle,
            agent_id,
            session_id,
            operation_id,
            final_output_path,
            relay_guard,
            execution_context,
            output_format,
            message_id,
            workspace,
            mount_snapshot,
            configured_binding_ids,
        ) = {
            let mut processes = self
                .processes
                .lock()
                .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
            let managed = processes.get_mut(process_id).ok_or_else(|| {
                AgentRuntimeApplicationError::Process(format!(
                    "Agent process {process_id} is not active."
                ))
            })?;
            if managed.monitoring {
                return Err(AgentRuntimeApplicationError::Process(format!(
                    "Agent process {process_id} is already monitored."
                )));
            }
            managed.monitoring = true;
            (
                managed.runner_handle.clone(),
                managed.agent_id.clone(),
                managed.session_id.clone(),
                managed.operation_id.clone(),
                managed.final_output_path.clone(),
                managed.relay_guard.clone(),
                managed.execution_context.clone(),
                managed.output_format,
                managed.message_id.clone(),
                managed.workspace.clone(),
                managed.mount_snapshot.clone(),
                managed.configured_binding_ids.clone(),
            )
        };
        let processes = self.processes.clone();
        let process_id = process_id.to_string();
        let logging = self.logging.clone();
        let clock = self.clock.clone();
        let telemetry = self.telemetry.clone();
        let event_sequence = self.event_sequence.clone();
        let evidence = self.evidence.clone();
        let runner = self.runner.clone();
        thread::spawn(move || {
            ProcessMonitor {
                runner,
                runner_handle,
                agent_id,
                sink,
                logging,
                clock,
                session_id,
                operation_id,
                final_output_path,
                relay_guard,
                execution_context,
                telemetry,
                event_sequence,
                output_format,
                message_id,
                workspace,
                evidence,
                mount_snapshot,
                configured_binding_ids,
            }
            .run();
            if let Ok(mut processes) = processes.lock() {
                processes.remove(&process_id);
            }
        });
        Ok(())
    }

    fn stop_generation(
        &self,
        process_id: &str,
        initiator: crate::contexts::agent_runtime::application::ProcessStopInitiator,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let managed = self
            .processes
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?
            .remove(process_id);
        let Some(managed) = managed else {
            return Ok(false);
        };
        self.providers
            .require(&managed.agent_id, ProviderCapability::Cancellation)?;
        let _ = self.telemetry.record_event(&ExecutionEvent {
            run_id: managed.execution_context.run_id.clone(),
            span_id: managed.execution_context.span_id.clone(),
            sequence: self.event_sequence.fetch_add(1, Ordering::Relaxed) + 1,
            name: "process.cancellation_requested".to_string(),
            timestamp: self.clock.now(),
            attributes: safe_attributes([(
                "vanehub.cancellation.initiator".to_string(),
                SafeAttributeValue::String(initiator.as_str().to_string()),
            )]),
        });
        let inspection = self
            .runner
            .inspect(&managed.runner_handle)
            .map_err(runner_application_error)?;
        record_runner_lifecycle(
            self.logging.as_ref(),
            self.clock.as_ref(),
            AgentLogLevel::Debug,
            "inspect",
            managed.runner_handle.reference.kind,
            &managed.runner_handle.reference.target_id,
            "completed",
            &managed.agent_id,
            &managed.session_id,
            &managed.operation_id,
            &managed.execution_context,
        );
        if let crate::contexts::agent_runtime::application::RunnerInspection::Exited(code) =
            inspection
        {
            if !managed.monitoring {
                let execution_status = if code == Some(0) {
                    ExecutionStatus::Succeeded
                } else {
                    ExecutionStatus::Failed
                };
                let _ = self.telemetry.finish_span(
                    &managed.execution_context.run_id,
                    &managed.execution_context.span_id,
                    execution_status,
                    &self.clock.now(),
                    (code != Some(0)).then_some("process_exit_failed"),
                );
                self.runner
                    .cleanup(&managed.runner_handle)
                    .map_err(runner_application_error)?;
            }
            return Ok(false);
        }
        if let Err(error) = self.runner.cancel(&managed.runner_handle) {
            record_runner_lifecycle(
                self.logging.as_ref(),
                self.clock.as_ref(),
                AgentLogLevel::Error,
                "cancel",
                managed.runner_handle.reference.kind,
                &managed.runner_handle.reference.target_id,
                error.code(),
                &managed.agent_id,
                &managed.session_id,
                &managed.operation_id,
                &managed.execution_context,
            );
            let _ = self.telemetry.finish_span(
                &managed.execution_context.run_id,
                &managed.execution_context.span_id,
                ExecutionStatus::Incomplete,
                &self.clock.now(),
                Some("process_cancel_failed"),
            );
            return Err(runner_application_error(error));
        }
        record_runner_lifecycle(
            self.logging.as_ref(),
            self.clock.as_ref(),
            AgentLogLevel::Info,
            "cancel",
            managed.runner_handle.reference.kind,
            &managed.runner_handle.reference.target_id,
            "succeeded",
            &managed.agent_id,
            &managed.session_id,
            &managed.operation_id,
            &managed.execution_context,
        );
        let (status, error_classification) = match initiator {
            crate::contexts::agent_runtime::application::ProcessStopInitiator::User => {
                (ExecutionStatus::Cancelled, "user_cancelled")
            }
            crate::contexts::agent_runtime::application::ProcessStopInitiator::RuntimeCleanup => {
                (ExecutionStatus::Failed, "runtime_cleanup")
            }
        };
        let _ = self.telemetry.finish_span(
            &managed.execution_context.run_id,
            &managed.execution_context.span_id,
            status,
            &self.clock.now(),
            Some(error_classification),
        );
        if !managed.monitoring {
            self.runner
                .cleanup(&managed.runner_handle)
                .map_err(runner_application_error)?;
            cleanup_final_output(managed.final_output_path.as_deref());
        }
        Ok(true)
    }
}

fn evidence_cli_snapshot(
    snapshot: Option<CliSkillEvidenceSnapshot>,
) -> (Option<CliMountSnapshot>, Vec<String>) {
    let Some(snapshot) = snapshot else {
        return (None, Vec::new());
    };
    let mount_snapshot = snapshot
        .manifest_hash
        .map(|manifest_hash| CliMountSnapshot {
            manifest_hash,
            skills: snapshot
                .mounted
                .into_iter()
                .map(|skill| MountedSkillRevision {
                    skill_id: skill.skill_id,
                    revision: skill.revision,
                })
                .collect(),
        });
    (mount_snapshot, snapshot.configured_binding_ids)
}

impl ProcessMonitor {
    fn run(self) {
        let ProcessMonitor {
            runner,
            runner_handle,
            agent_id,
            sink,
            logging,
            clock,
            session_id,
            operation_id,
            final_output_path,
            relay_guard,
            execution_context,
            telemetry,
            event_sequence,
            output_format,
            message_id,
            workspace,
            evidence,
            mount_snapshot,
            configured_binding_ids,
        } = self;
        let mut reader = RunnerEventReader::new(runner.clone(), runner_handle.clone());
        let parser = output_parser_for_format(output_format);
        let mut terminal_error = None;
        let mut emitted_content = false;
        let mut first_visible_output = false;
        // Captured from the CLI's own completion line (if it carried usage), mirroring
        // how `terminal_error` is captured across the read loop. Attached to the
        // terminal `GenerationProcessEvent::Completed` below rather than acted on
        // immediately, since the exit code (not this line) decides success/failure.
        let mut reported_usage: Option<ReportedUsageTotals> = None;
        for line in BoundedProviderLines::new(&mut reader, 256 * 1024) {
            let event = match line {
                Ok(line) => match parser.parse_line(&line) {
                    ProviderOutputEvent::Token(delta) => {
                        let delta = if emitted_content {
                            format!("\n{delta}")
                        } else {
                            delta
                        };
                        emitted_content = true;
                        Some(GenerationProcessEvent::Token(delta))
                    }
                    ProviderOutputEvent::Thinking(content) => {
                        Some(GenerationProcessEvent::Thinking(content))
                    }
                    ProviderOutputEvent::ToolLifecycle(tool) => Some(
                        GenerationProcessEvent::ToolLifecycle(normalize_provider_tool(
                            *tool,
                            event_sequence.fetch_add(1, Ordering::Relaxed) + 1,
                        )),
                    ),
                    ProviderOutputEvent::RichBlock(block) => {
                        Some(GenerationProcessEvent::RichBlock(block))
                    }
                    ProviderOutputEvent::SessionId(runtime_session_id) => {
                        Some(GenerationProcessEvent::RuntimeSessionId(runtime_session_id))
                    }
                    ProviderOutputEvent::Failed(error) => {
                        terminal_error = Some(error);
                        None
                    }
                    ProviderOutputEvent::Completed(usage) => {
                        if let Some(reason) = usage_degradation_reason(usage.as_ref()) {
                            let adapter_version = usage.as_ref().map_or_else(
                                || output_adapter_name(output_format),
                                |value| value.normalization_version,
                            );
                            let _ = logging.record(AgentLog {
                                level: AgentLogLevel::Warn,
                                category: "token.accounting.ingestion".to_string(),
                                message: usage_degradation_message(reason, adapter_version),
                                agent_id: Some(agent_id.clone()),
                                session_id: Some(session_id.clone()),
                                operation_id: Some(operation_id.clone()),
                                run_id: Some(execution_context.run_id.as_str().to_string()),
                                trace_id: Some(execution_context.trace_id.as_str().to_string()),
                                span_id: Some(execution_context.span_id.as_str().to_string()),
                                occurred_at: clock.now(),
                            });
                        }
                        reported_usage = usage.map(normalize_provider_usage);
                        None
                    }
                    ProviderOutputEvent::Empty => None,
                },
                Err(error) => {
                    terminal_error = Some(GenerationProcessFailure::retryable(format!(
                        "Provider output protocol error: {error}"
                    )));
                    break;
                }
            };
            if let Some(event) = event {
                if !first_visible_output {
                    first_visible_output = true;
                    let _ = telemetry.record_event(&ExecutionEvent {
                        run_id: execution_context.run_id.clone(),
                        span_id: execution_context.span_id.clone(),
                        sequence: event_sequence.fetch_add(1, Ordering::Relaxed) + 1,
                        name: "process.first_visible_output".to_string(),
                        timestamp: clock.now(),
                        attributes: SafeAttributes::default(),
                    });
                }
                if let Err(error) = sink.handle(event) {
                    terminal_error = Some(GenerationProcessFailure::retryable(format!(
                        "Agent generation event handling failed: {error}"
                    )));
                    break;
                }
            }
        }
        if reader.disconnected {
            record_runner_lifecycle(
                logging.as_ref(),
                clock.as_ref(),
                AgentLogLevel::Warn,
                "disconnect",
                runner_handle.reference.kind,
                &runner_handle.reference.target_id,
                "runner_disconnected",
                &agent_id,
                &session_id,
                &operation_id,
                &execution_context,
            );
        }
        if reader.exit_code.is_none() {
            let _ = runner.cancel(&runner_handle);
            reader.drain_to_exit();
        }
        let exit_status = runner_exit_outcome(reader.exit_code);
        let stderr_output = reader.stderr_output();
        if !stderr_output.trim().is_empty() {
            let _ = sink.handle(GenerationProcessEvent::Stderr(
                stderr_output.trim().to_string(),
            ));
        }
        if terminal_error.is_none()
            && matches!(&exit_status, Ok(ProcessExitOutcome::Success))
            && !emitted_content
        {
            if let Some(final_message) = read_final_output(final_output_path.as_deref()) {
                let _ = sink.handle(GenerationProcessEvent::Token(final_message));
                emitted_content = true;
            }
        }
        let exit_attributes = match reader.exit_code.flatten() {
            Some(code) => safe_attributes([(
                "process.exit.code".to_string(),
                SafeAttributeValue::Integer(i64::from(code)),
            )]),
            None => SafeAttributes::default(),
        };
        let terminal =
            compose_terminal_event(terminal_error, exit_status, &stderr_output, reported_usage);
        let (process_status, process_error) = match &terminal {
            GenerationProcessEvent::Completed(_) => (ExecutionStatus::Succeeded, None),
            GenerationProcessEvent::Failed(_) => {
                (ExecutionStatus::Failed, Some("process_exit_failed"))
            }
            _ => (
                ExecutionStatus::Incomplete,
                Some("process_terminal_unknown"),
            ),
        };
        let (outcome, failure_class) = match &terminal {
            GenerationProcessEvent::Completed(_) => (TerminalOutcome::Succeeded, None),
            GenerationProcessEvent::Failed(_) => {
                (TerminalOutcome::Failed, Some(FailureClass::Process))
            }
            _ => (TerminalOutcome::Incomplete, Some(FailureClass::Process)),
        };
        let _ = evidence.cli(CliLifecycleFact {
            kind: CliRuntimeKind::Managed,
            common: EnvelopeCommon {
                source_event_id: format!("cli:{}:terminal", execution_context.run_id.as_str()),
                occurred_at: clock.now(),
                stable_agent_id: Some(agent_id.clone()),
                session_id: Some(session_id.clone()),
                message_id: Some(message_id),
                run_id: Some(execution_context.run_id.as_str().to_string()),
                attempt_id: Some(operation_id.clone()),
                workspace,
                fidelity: SourceFidelity::Proxied,
                observed_skill_revisions: Vec::new(),
            },
            outcome,
            failure_class,
            mount_snapshot,
            configured_binding_ids,
        });
        let _ = telemetry.record_event(&ExecutionEvent {
            run_id: execution_context.run_id.clone(),
            span_id: execution_context.span_id.clone(),
            sequence: event_sequence.fetch_add(1, Ordering::Relaxed) + 1,
            name: if emitted_content {
                "process.exited".to_string()
            } else {
                "process.exited_without_output".to_string()
            },
            timestamp: clock.now(),
            attributes: exit_attributes,
        });
        let _ = telemetry.finish_span(
            &execution_context.run_id,
            &execution_context.span_id,
            process_status,
            &clock.now(),
            process_error,
        );
        if let Err(error) = sink.handle(terminal) {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Error,
                category: "session.runtime.cli".to_string(),
                message: format!("Agent generation terminal event failed: {error}"),
                agent_id: Some(agent_id.clone()),
                session_id: Some(session_id.clone()),
                operation_id: Some(operation_id.clone()),
                run_id: Some(execution_context.run_id.as_str().to_string()),
                trace_id: Some(execution_context.trace_id.as_str().to_string()),
                span_id: Some(execution_context.span_id.as_str().to_string()),
                occurred_at: clock.now(),
            });
        }
        cleanup_final_output(final_output_path.as_deref());
        let cleanup = runner.cleanup(&runner_handle);
        record_runner_lifecycle(
            logging.as_ref(),
            clock.as_ref(),
            if cleanup.is_ok() {
                AgentLogLevel::Debug
            } else {
                AgentLogLevel::Error
            },
            "cleanup",
            runner_handle.reference.kind,
            &runner_handle.reference.target_id,
            cleanup
                .as_ref()
                .map_or_else(|error| error.code(), |_| "succeeded"),
            &agent_id,
            &session_id,
            &operation_id,
            &execution_context,
        );
        if let Some(guard) = relay_guard {
            let _ = guard.cleanup();
        }
    }
}

struct RunnerEventReader {
    runner: Arc<dyn AgentRunner>,
    handle: RunnerHandle,
    pending: std::collections::VecDeque<u8>,
    stderr: Vec<u8>,
    stderr_truncated: bool,
    exit_code: Option<Option<i32>>,
    disconnected: bool,
}

impl RunnerEventReader {
    fn new(runner: Arc<dyn AgentRunner>, handle: RunnerHandle) -> Self {
        Self {
            runner,
            handle,
            pending: std::collections::VecDeque::new(),
            stderr: Vec::new(),
            stderr_truncated: false,
            exit_code: None,
            disconnected: false,
        }
    }

    fn drain_to_exit(&mut self) {
        while self.exit_code.is_none() {
            match self.runner.next_event(&self.handle) {
                Ok(Some(RunnerEvent::Stderr(chunk))) => self.push_stderr(&chunk),
                Ok(Some(RunnerEvent::Exited(code))) => self.exit_code = Some(code),
                Ok(Some(_)) => {}
                Ok(None) | Err(_) => break,
            }
        }
    }

    fn push_stderr(&mut self, chunk: &[u8]) {
        const LIMIT: usize = 65_536;
        let remaining = LIMIT.saturating_sub(self.stderr.len());
        self.stderr
            .extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        self.stderr_truncated |= chunk.len() > remaining;
    }

    fn stderr_output(&self) -> String {
        if self.stderr_truncated {
            return "provider stderr exceeded the bounded diagnostic limit".to_string();
        }
        String::from_utf8_lossy(&self.stderr).trim().to_string()
    }
}

impl Read for RunnerEventReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        while self.pending.is_empty() && self.exit_code.is_none() {
            match self.runner.next_event(&self.handle) {
                Ok(Some(RunnerEvent::Stdout(chunk))) => self.pending.extend(chunk),
                Ok(Some(RunnerEvent::Stderr(chunk))) => self.push_stderr(&chunk),
                Ok(Some(RunnerEvent::Exited(code))) => self.exit_code = Some(code),
                Ok(Some(RunnerEvent::Disconnected)) => {
                    self.disconnected = true;
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionAborted,
                        "Runner disconnected",
                    ));
                }
                Ok(None) => return Ok(0),
                Err(error) => {
                    return Err(std::io::Error::other(error.code()));
                }
            }
        }
        let length = output.len().min(self.pending.len());
        for slot in output.iter_mut().take(length) {
            *slot = self.pending.pop_front().unwrap_or_default();
        }
        Ok(length)
    }
}

fn runner_exit_outcome(exit_code: Option<Option<i32>>) -> Result<ProcessExitOutcome, String> {
    match exit_code {
        Some(Some(0)) => Ok(ProcessExitOutcome::Success),
        Some(code) => Ok(ProcessExitOutcome::Failure {
            status: code.map_or_else(|| "terminated".to_string(), |value| value.to_string()),
        }),
        None => Err("Runner event stream ended without an exit status".to_string()),
    }
}

/// Converts the infrastructure-owned per-CLI usage shape into the application layer's
/// own `ReportedUsageTotals`, at the same boundary `normalize_provider_tool` already
/// converts `ProviderToolEvent` — keeping `agent_runtime::application` free of a
/// concrete infrastructure type. See `add-reported-usage-ingestion` design.md Decision 0.
fn normalize_provider_usage(usage: ProviderReportedUsage) -> ReportedUsageTotals {
    let overlap = |value| match value {
        ProviderUsageOverlap::Subset => AgentUsageOverlap::Subset,
        ProviderUsageOverlap::Exclusive => AgentUsageOverlap::Exclusive,
        ProviderUsageOverlap::Unknown => AgentUsageOverlap::Unknown,
    };
    ReportedUsageTotals {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_read_tokens: usage.cache_read_tokens,
        cache_creation_tokens: usage.cache_creation_tokens,
        reasoning_output_tokens: usage.reasoning_output_tokens,
        provider_total_tokens: usage.provider_total_tokens,
        cache_overlap: overlap(usage.cache_overlap),
        reasoning_overlap: overlap(usage.reasoning_overlap),
        normalization_version: usage.normalization_version,
        model_id: usage.model_id,
        source_identity: usage.source_identity,
        source_revision: usage.source_revision,
    }
}

fn output_adapter_name(format: ProviderOutputFormat) -> &'static str {
    match format {
        ProviderOutputFormat::ClaudeStreamJson => "claude-stream-json",
        ProviderOutputFormat::StructuredJsonLines => "structured-json-lines",
        ProviderOutputFormat::AntigravityStreamJson => "antigravity-stream-json",
    }
}

fn provider_usage_diagnostic_reason(usage: Option<&ProviderReportedUsage>) -> Option<&'static str> {
    let usage = usage?;
    let mut derived = usage.input_tokens.checked_add(usage.output_tokens)?;
    if usage.cache_overlap == ProviderUsageOverlap::Exclusive {
        derived = derived.checked_add(usage.cache_read_tokens)?;
        derived = derived.checked_add(usage.cache_creation_tokens)?;
    }
    if usage.reasoning_overlap == ProviderUsageOverlap::Exclusive {
        derived = derived.checked_add(usage.reasoning_output_tokens)?;
    }
    usage
        .provider_total_tokens
        .filter(|total| *total != derived)
        .map(|_| "semantic_mismatch")
}

fn usage_degradation_reason(usage: Option<&ProviderReportedUsage>) -> Option<&'static str> {
    usage
        .is_none()
        .then_some("missing_or_unsupported_schema")
        .or_else(|| provider_usage_diagnostic_reason(usage))
}

fn usage_degradation_message(reason: &str, adapter_version: &str) -> String {
    format!("CLI usage degraded reason={reason} adapter={adapter_version}")
}

fn normalize_provider_tool(tool: ProviderToolEvent, sequence: u64) -> ToolLifecycleEvent {
    let call_id = tool
        .call_id
        .unwrap_or_else(|| format!("unidentified-tool-{sequence}"));
    let name = tool.name.unwrap_or_else(|| "unknown_tool".to_string());
    ToolLifecycleEvent {
        call_id: call_id.clone(),
        phase: match tool.phase {
            ProviderToolPhase::Started => ToolLifecyclePhase::Started,
            ProviderToolPhase::Updated => ToolLifecyclePhase::Updated,
            ProviderToolPhase::Completed => ToolLifecyclePhase::Completed,
            ProviderToolPhase::Failed => ToolLifecyclePhase::Failed,
        },
        provider_timestamp: tool.provider_timestamp,
        fidelity: tool.fidelity,
        parent_run_id: tool.parent_run_id,
        parent_trace_id: tool.parent_trace_id,
        parent_span_id: tool.parent_span_id,
        delegation_id: tool.delegation_id,
        attempt: tool.attempt,
        tool_use: ToolUseBlock {
            id: call_id,
            name,
            input: tool.input,
            output: tool.output,
            status: tool.status,
            skill_provenance: None,
        },
    }
}

fn read_final_output(path: Option<&Path>) -> Option<String> {
    let path = path?;
    fs::read_to_string(path)
        .ok()
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty())
}

fn executable_name(executable: &str) -> String {
    executable
        .rsplit(['/', '\\'])
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn safe_attributes(
    entries: impl IntoIterator<Item = (String, SafeAttributeValue)>,
) -> SafeAttributes {
    SafeAttributes::try_from_entries(entries).unwrap_or_default()
}

fn runner_application_error(
    error: crate::contexts::agent_runtime::application::RunnerError,
) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Process(error.code().to_string())
}

fn cleanup_final_output(path: Option<&Path>) {
    if let Some(path) = path {
        let _ = fs::remove_file(path);
    }
}

fn apply_mcp_relay_args(agent_id: &str, args: &mut Vec<String>, relay_args: Vec<String>) {
    let insertion_index = if agent_id == "codex-cli" {
        args.iter()
            .position(|argument| argument == "exec")
            .unwrap_or(args.len())
    } else {
        args.len()
    };
    args.splice(insertion_index..insertion_index, relay_args);
}

fn launch_command(command: Option<&str>) -> Result<(), AgentRuntimeApplicationError> {
    let Some(command) = command else {
        return Ok(());
    };
    if !process::command_exists(command, Duration::from_secs(2)) {
        return Err(AgentRuntimeApplicationError::Process(format!(
            "Command '{command}' was not found on PATH."
        )));
    }
    process::std_command(command)
        .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))
}

fn codex_output_capture_path(session_id: &str, operation_id: &str) -> PathBuf {
    let safe_session = safe_file_segment(session_id);
    let safe_operation = safe_file_segment(operation_id);
    std::env::temp_dir().join(format!(
        "vanehub-codex-last-message-{safe_session}-{safe_operation}.txt"
    ))
}

fn safe_file_segment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn normalize_generation_executable(agent_id: &str, executable: &str) -> String {
    match agent_id {
        "codex-cli" => resolve_codex_npm_shim(executable).unwrap_or_else(|| executable.to_string()),
        "opencode" => {
            resolve_opencode_npm_shim(executable).unwrap_or_else(|| executable.to_string())
        }
        _ => executable.to_string(),
    }
}

fn resolve_codex_npm_shim(executable: &str) -> Option<String> {
    let path = recognized_npm_shim(executable, "codex")?;
    let package_root = path
        .parent()?
        .join("node_modules")
        .join("@openai")
        .join("codex");
    let (platform_package, target_triple) = if cfg!(target_arch = "aarch64") {
        ("codex-win32-arm64", "aarch64-pc-windows-msvc")
    } else {
        ("codex-win32-x64", "x86_64-pc-windows-msvc")
    };
    let candidates = [
        package_root
            .join("node_modules")
            .join("@openai")
            .join(platform_package)
            .join("vendor")
            .join(target_triple)
            .join("bin")
            .join("codex.exe"),
        package_root
            .join("vendor")
            .join(target_triple)
            .join("bin")
            .join("codex.exe"),
        package_root.join("bin").join("codex.exe"),
        package_root.join("codex.exe"),
    ];
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().to_string())
}

fn recognized_npm_shim<'a>(executable: &'a str, expected_stem: &str) -> Option<&'a Path> {
    let path = Path::new(executable);
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    if extension != "cmd" && extension != "ps1" {
        return None;
    }
    (path.file_stem()?.to_string_lossy().to_ascii_lowercase() == expected_stem).then_some(path)
}

fn resolve_opencode_npm_shim(executable: &str) -> Option<String> {
    let path = recognized_npm_shim(executable, "opencode")?;
    let resolved = path
        .parent()?
        .join("node_modules")
        .join("opencode-ai")
        .join("bin")
        .join("opencode.exe");
    resolved
        .is_file()
        .then(|| resolved.to_string_lossy().to_string())
}

/// How a finished Agent process exited, carrying the rendered status so the fallback message can
/// still name it when nothing better is available.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ProcessExitOutcome {
    Success,
    Failure { status: String },
}

/// Chooses the terminal event for a finished Agent process.
///
/// Order matters: a diagnostic parsed from the provider's own output wins over anything derived
/// from the exit status. claude-code exits non-zero with empty stderr after stating the cause on
/// stdout, so deriving the message from the exit status discards the only useful information.
fn compose_terminal_event(
    terminal_error: Option<GenerationProcessFailure>,
    exit_outcome: Result<ProcessExitOutcome, String>,
    stderr_output: &str,
    reported_usage: Option<ReportedUsageTotals>,
) -> GenerationProcessEvent {
    match (terminal_error, exit_outcome) {
        (Some(error), _) => GenerationProcessEvent::Failed(error),
        (None, Ok(ProcessExitOutcome::Success)) => {
            GenerationProcessEvent::Completed(reported_usage)
        }
        (None, Ok(ProcessExitOutcome::Failure { status })) => GenerationProcessEvent::Failed(
            GenerationProcessFailure::retryable(if stderr_output.trim().is_empty() {
                format!("Agent CLI exited with status {status}.")
            } else {
                stderr_output.trim().to_string()
            }),
        ),
        (None, Err(error)) => {
            GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(error))
        }
    }
}

#[cfg(test)]
mod terminal_event_tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{
        GenerationProcessFailureKind, PreparedRunnerLaunch, RunnerCapabilities, RunnerError,
        RunnerErrorKind, RunnerInspection, RunnerKind, RunnerPolicyWitness, RunnerRecoveryMode,
        RunnerReference,
    };
    use crate::contexts::tooling::skills::api::CliSkillEvidenceEntry;
    use crate::test_support::TempDirectory;

    fn failure(message: &str) -> GenerationProcessFailure {
        GenerationProcessFailure::non_retryable(message)
    }

    #[test]
    fn cli_snapshot_projection_preserves_only_ids_revisions_and_manifest() {
        let (mount, configured) = evidence_cli_snapshot(Some(CliSkillEvidenceSnapshot {
            manifest_hash: Some("manifest-a".to_string()),
            mounted: vec![CliSkillEvidenceEntry {
                skill_id: "review".to_string(),
                revision: "revision-a".to_string(),
            }],
            configured_binding_ids: vec!["review".to_string(), "disabled".to_string()],
        }));

        assert_eq!(configured, ["review", "disabled"]);
        assert_eq!(mount.expect("mount").skills[0].skill_id, "review");
    }

    /// claude-code exits non-zero with empty stderr after reporting the cause on stdout. The
    /// parsed diagnostic must survive; reporting the exit status here is what left users with
    /// `Agent CLI exited with status exit code: 1.` and no reason.
    #[test]
    fn parsed_diagnostic_survives_a_non_zero_exit_with_empty_stderr() {
        let terminal = compose_terminal_event(
            Some(failure(
                "Failed to authenticate. API Error: 403 Request not allowed",
            )),
            Ok(ProcessExitOutcome::Failure {
                status: "exit code: 1".to_string(),
            }),
            "",
            None,
        );

        match terminal {
            GenerationProcessEvent::Failed(error) => {
                assert_eq!(
                    error.diagnostic,
                    "Failed to authenticate. API Error: 403 Request not allowed"
                );
                assert_eq!(error.kind, GenerationProcessFailureKind::NonRetryable);
            }
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn exit_status_is_reported_only_when_nothing_better_exists() {
        let terminal = compose_terminal_event(
            None,
            Ok(ProcessExitOutcome::Failure {
                status: "exit code: 1".to_string(),
            }),
            "",
            None,
        );

        match terminal {
            GenerationProcessEvent::Failed(error) => {
                assert!(
                    error.diagnostic.contains("exit code: 1"),
                    "with no diagnostic and no stderr the exit status is all we have, got {}",
                    error.diagnostic
                );
            }
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn stderr_is_used_when_present_and_nothing_was_parsed() {
        let terminal = compose_terminal_event(
            None,
            Ok(ProcessExitOutcome::Failure {
                status: "exit code: 1".to_string(),
            }),
            "  boom  ",
            None,
        );

        match terminal {
            GenerationProcessEvent::Failed(error) => assert_eq!(error.diagnostic, "boom"),
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    #[test]
    fn runner_log_summary_excludes_arguments_prompts_and_unrestricted_paths() {
        let provider = ProviderInvocationSpec {
            executable: "C:\\private\\fixture.exe".to_string(),
            args: vec![
                "--json".to_string(),
                "private prompt".to_string(),
                "--resume".to_string(),
                "session-1".to_string(),
            ],
            prompt_delivery: ProviderPromptDelivery::Argument,
        };

        let summary = provider_execution_summary(&provider);
        assert_eq!(summary, "executing fixture.exe with 4 arguments");
        for secret in ["private prompt", "session-1", "private\\", "--resume"] {
            assert!(!summary.contains(secret));
        }
    }

    #[test]
    fn a_successful_exit_completes_with_its_usage() {
        let terminal = compose_terminal_event(None, Ok(ProcessExitOutcome::Success), "", None);
        assert!(matches!(terminal, GenerationProcessEvent::Completed(None)));
    }

    #[test]
    fn usage_diagnostics_are_bounded_and_exclude_provider_content() {
        let usage = ProviderReportedUsage {
            input_tokens: 5,
            output_tokens: 3,
            provider_total_tokens: Some(99),
            cache_overlap: ProviderUsageOverlap::Subset,
            reasoning_overlap: ProviderUsageOverlap::Subset,
            normalization_version: "fixture-adapter-v1",
            model_id: Some("prompt=private response=private token=credential".to_string()),
            source_identity: Some("C:\\private\\raw-event.jsonl".to_string()),
            source_revision: Some("--api-key secret argument".to_string()),
            ..ProviderReportedUsage::default()
        };
        let reason = usage_degradation_reason(Some(&usage)).expect("diagnostic reason");
        let message = usage_degradation_message(reason, usage.normalization_version);

        assert_eq!(reason, "semantic_mismatch");
        assert!(message.len() <= 128);
        for secret in [
            "prompt=private",
            "response=private",
            "credential",
            "private\\raw-event",
            "api-key",
            "secret argument",
        ] {
            assert!(!message.contains(secret), "diagnostic leaked {secret}");
        }
    }

    #[test]
    fn absent_usage_reports_an_unsupported_schema_without_raw_event_data() {
        assert_eq!(
            usage_degradation_reason(None),
            Some("missing_or_unsupported_schema")
        );
        assert_eq!(
            usage_degradation_message(
                "missing_or_unsupported_schema",
                output_adapter_name(ProviderOutputFormat::StructuredJsonLines)
            ),
            "CLI usage degraded reason=missing_or_unsupported_schema adapter=structured-json-lines"
        );
    }

    #[test]
    fn codex_npm_shim_resolves_the_packaged_native_windows_binary() {
        let directory = TempDirectory::new("codex-generation-shim");
        let shim = directory.path().join("codex.cmd");
        let native = directory
            .path()
            .join("node_modules")
            .join("@openai")
            .join("codex")
            .join("node_modules")
            .join("@openai")
            .join(if cfg!(target_arch = "aarch64") {
                "codex-win32-arm64"
            } else {
                "codex-win32-x64"
            })
            .join("vendor")
            .join(if cfg!(target_arch = "aarch64") {
                "aarch64-pc-windows-msvc"
            } else {
                "x86_64-pc-windows-msvc"
            })
            .join("bin")
            .join("codex.exe");
        std::fs::write(&shim, "fixture").expect("shim");
        std::fs::create_dir_all(native.parent().expect("native parent")).expect("native dirs");
        std::fs::write(&native, "fixture").expect("native executable");

        assert_eq!(
            normalize_generation_executable("codex-cli", &shim.to_string_lossy()),
            native.to_string_lossy().to_string()
        );
    }

    #[test]
    fn codex_npm_shim_falls_back_when_the_native_package_is_missing() {
        let directory = TempDirectory::new("codex-generation-shim-missing");
        let shim = directory.path().join("codex.cmd");
        std::fs::write(&shim, "fixture").expect("shim");

        assert_eq!(
            normalize_generation_executable("codex-cli", &shim.to_string_lossy()),
            shim.to_string_lossy().to_string()
        );
    }

    struct OrderedPermissions {
        calls: Arc<Mutex<Vec<&'static str>>>,
        stale: bool,
    }

    impl RunnerPermissionPort for OrderedPermissions {
        fn authorize(
            &self,
            _context: &RunnerPermissionContext,
        ) -> Result<RunnerPolicyWitness, RunnerError> {
            self.calls.lock().expect("calls").push("authorize");
            Ok(RunnerPolicyWitness {
                fingerprint: "sha256:policy".into(),
            })
        }

        fn revalidate(
            &self,
            _context: &RunnerPermissionContext,
            _witness: &RunnerPolicyWitness,
        ) -> Result<(), RunnerError> {
            self.calls.lock().expect("calls").push("revalidate");
            if self.stale {
                Err(RunnerError::new(RunnerErrorKind::AuthorityStale))
            } else {
                Ok(())
            }
        }
    }

    struct OrderedRunner(Arc<Mutex<Vec<&'static str>>>);

    impl AgentRunner for OrderedRunner {
        fn kind(&self) -> RunnerKind {
            RunnerKind::Local
        }

        fn capabilities(&self) -> RunnerCapabilities {
            RunnerCapabilities {
                interactive_input: true,
                pty: false,
                cancellation: true,
                inspection: true,
                recovery: RunnerRecoveryMode::None,
            }
        }

        fn prepare(
            &self,
            _selection: &RunnerSelection,
            spec: RunnerLaunchSpec,
        ) -> Result<PreparedRunnerLaunch, RunnerError> {
            self.0.lock().expect("calls").push("prepare");
            Ok(PreparedRunnerLaunch {
                reference: RunnerReference {
                    kind: RunnerKind::Local,
                    target_id: "local".into(),
                    target_revision: None,
                    recovery: RunnerRecoveryMode::None,
                    authority_witness: "runner-v1".into(),
                },
                spec,
                preparation_id: None,
                admission_id: None,
            })
        }

        fn spawn(&self, prepared: PreparedRunnerLaunch) -> Result<RunnerHandle, RunnerError> {
            self.0.lock().expect("calls").push("spawn");
            assert!(prepared.reference.authority_witness.starts_with("sha256:"));
            Ok(RunnerHandle {
                id: "handle-1".into(),
                reference: prepared.reference,
                process_reference: None,
            })
        }

        fn send_input(&self, _handle: &RunnerHandle, _content: &[u8]) -> Result<(), RunnerError> {
            unreachable!()
        }

        fn next_event(&self, _handle: &RunnerHandle) -> Result<Option<RunnerEvent>, RunnerError> {
            unreachable!()
        }

        fn cancel(&self, _handle: &RunnerHandle) -> Result<bool, RunnerError> {
            unreachable!()
        }

        fn inspect(&self, _handle: &RunnerHandle) -> Result<RunnerInspection, RunnerError> {
            unreachable!()
        }

        fn cleanup(&self, _handle: &RunnerHandle) -> Result<(), RunnerError> {
            unreachable!()
        }

        fn recover(
            &self,
            _reference: &RunnerReference,
            _process_reference: Option<&str>,
        ) -> Result<RunnerInspection, RunnerError> {
            unreachable!()
        }
    }

    fn authorization_context() -> RunnerPermissionContext {
        RunnerPermissionContext {
            agent_id: "codex-cli".into(),
            session_id: "session-1".into(),
            generation_id: "generation-1".into(),
            project_key: "workspace".into(),
            action: "shell.exec".into(),
            selection: RunnerSelection::local(),
        }
    }

    fn runner_launch() -> RunnerLaunchSpec {
        RunnerLaunchSpec {
            session_id: Some("session-1".into()),
            executable: "codex".into(),
            arguments: vec!["exec".into()],
            cwd: Some("workspace".into()),
            environment: Default::default(),
            pipe_stdin: false,
        }
    }

    #[test]
    fn runner_permission_is_revalidated_immediately_before_spawn() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        prepare_and_spawn_authorized(
            &OrderedRunner(calls.clone()),
            &OrderedPermissions {
                calls: calls.clone(),
                stale: false,
            },
            &RunnerSelection::local(),
            &authorization_context(),
            runner_launch(),
        )
        .expect("spawn");
        assert_eq!(
            *calls.lock().expect("calls"),
            ["authorize", "prepare", "revalidate", "spawn"]
        );
    }

    #[test]
    fn stale_runner_permission_prevents_spawn() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let error = prepare_and_spawn_authorized(
            &OrderedRunner(calls.clone()),
            &OrderedPermissions {
                calls: calls.clone(),
                stale: true,
            },
            &RunnerSelection::local(),
            &authorization_context(),
            runner_launch(),
        )
        .expect_err("stale");
        assert_eq!(error.kind, RunnerErrorKind::AuthorityStale);
        assert_eq!(
            *calls.lock().expect("calls"),
            ["authorize", "prepare", "revalidate"]
        );
    }
}
