use super::providers::{
    add_codex_output_capture_args, output_parser_for_format, ProviderOutputEvent,
    ProviderPromptDelivery, ProviderReportedUsage, ProviderToolEvent, ProviderToolPhase,
    ProviderUsageOverlap,
};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentProcessEventSink,
    AgentProcessGateway, AgentRuntimeApplicationError, AgentUsageOverlap, GenerationProcessEvent,
    GenerationProcessFailure, GenerationProcessRequest, ProviderGenerationInvocationRequest,
    ProviderOutputFormat, ProviderRegistry, ReportedUsageTotals, StartedGenerationProcess,
    ToolLifecycleEvent, ToolLifecyclePhase, ToolUseBlock, WorkflowLaunchOutcome,
    WorkflowLaunchRequest,
};
use crate::contexts::agent_runtime::domain::{AgentAvailability, InteractionMode};
use crate::contexts::execution_observability::api::{
    ExecutionContext, ExecutionEvent, ExecutionFidelity, ExecutionIdentityPort, ExecutionSpan,
    ExecutionStatus, ExecutionTelemetryPort, SafeAttributeValue, SafeAttributes,
};
use crate::platform::filesystem::normalize_windows_extended_length_path;
use crate::platform::private_relay_fs::PreparedMcpRelayGuard;
use crate::platform::process;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct RuntimeAgentProcessAdapter {
    processes: Arc<Mutex<HashMap<String, ManagedProcess>>>,
    process_ids: Arc<AtomicU64>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    execution_ids: Arc<dyn ExecutionIdentityPort>,
    telemetry: Arc<dyn ExecutionTelemetryPort>,
    mcp_relay: Arc<dyn ManagedMcpRelayPort>,
    providers: Arc<ProviderRegistry>,
    event_sequence: Arc<AtomicU64>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedMcpRelay {
    pub(crate) invocation_args: Vec<String>,
    pub(crate) guard: Option<PreparedMcpRelayGuard>,
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
    child: Arc<Mutex<Child>>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
    agent_id: String,
    session_id: String,
    operation_id: String,
    monitoring: bool,
    final_output_path: Option<PathBuf>,
    relay_guard: Option<PreparedMcpRelayGuard>,
    execution_context: ExecutionContext,
    output_format: ProviderOutputFormat,
}

struct ProcessMonitor {
    child: Arc<Mutex<Child>>,
    stdout: ChildStdout,
    stderr: Option<ChildStderr>,
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
}

impl RuntimeAgentProcessAdapter {
    pub(crate) fn new(
        logging: Arc<dyn AgentLoggingPort>,
        clock: Arc<dyn AgentClockPort>,
        execution_ids: Arc<dyn ExecutionIdentityPort>,
        telemetry: Arc<dyn ExecutionTelemetryPort>,
        mcp_relay: Arc<dyn ManagedMcpRelayPort>,
        providers: Arc<ProviderRegistry>,
    ) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            process_ids: Arc::new(AtomicU64::new(0)),
            logging,
            clock,
            execution_ids,
            telemetry,
            mcp_relay,
            providers,
            event_sequence: Arc::new(AtomicU64::new(0)),
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
        let executable =
            normalize_generation_executable(&request.agent.id, &request.cli_profile.executable);
        let provider = self.providers.get(&request.agent.id)?;
        if !provider.capabilities().structured_output() {
            return Err(crate::contexts::agent_runtime::application::AgentProviderError::UnsupportedCapability {
                provider_id: request.agent.id.clone(),
                capability: "structured output".to_string(),
            }
            .into());
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
        let mut command = process::std_command(&spec.executable)
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        command.args(&spec.args);
        command.env("TRACEPARENT", request.execution_context.traceparent());
        if let Some(folder) = request
            .session
            .folder
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            command.current_dir(normalize_windows_extended_length_path(folder));
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        if spec.prompt_delivery == ProviderPromptDelivery::Stdin {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }
        let redacted_args = spec
            .args
            .iter()
            .map(|argument| {
                if argument == &request.effective_prompt {
                    "[prompt redacted]".to_string()
                } else {
                    argument.clone()
                }
            })
            .collect::<Vec<_>>();
        self.record_log(
            AgentLogLevel::Info,
            "session.runtime.cli",
            format!("executing {} {}", spec.executable, redacted_args.join(" ")),
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
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                let _ = self.telemetry.finish_span(
                    &process_context.run_id,
                    &process_context.span_id,
                    ExecutionStatus::Failed,
                    &self.clock.now(),
                    Some("process_spawn_failed"),
                );
                return Err(AgentRuntimeApplicationError::Process(error.to_string()));
            }
        };
        self.record_process_event(
            &process_context,
            "process.spawned",
            safe_attributes([(
                "process.pid".to_string(),
                SafeAttributeValue::Integer(i64::from(child.id())),
            )]),
        );
        if spec.prompt_delivery == ProviderPromptDelivery::Stdin {
            if let Some(mut stdin) = child.stdin.take() {
                if let Err(error) = stdin
                    .write_all(request.effective_prompt.as_bytes())
                    .and_then(|_| stdin.write_all(b"\n"))
                {
                    terminate_child(&mut child);
                    let _ = self.telemetry.finish_span(
                        &process_context.run_id,
                        &process_context.span_id,
                        ExecutionStatus::Failed,
                        &self.clock.now(),
                        Some("process_stdin_failed"),
                    );
                    return Err(AgentRuntimeApplicationError::Process(error.to_string()));
                }
            }
        }
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                terminate_child(&mut child);
                let _ = self.telemetry.finish_span(
                    &process_context.run_id,
                    &process_context.span_id,
                    ExecutionStatus::Failed,
                    &self.clock.now(),
                    Some("process_stdout_unavailable"),
                );
                return Err(AgentRuntimeApplicationError::Process(
                    "CLI process stdout unavailable.".to_string(),
                ));
            }
        };
        let stderr = child.stderr.take();
        let process_id = format!(
            "agent-process-{}-{}",
            child.id(),
            self.process_ids.fetch_add(1, Ordering::Relaxed) + 1
        );
        let mut processes = match self.processes.lock() {
            Ok(processes) => processes,
            Err(error) => {
                terminate_child(&mut child);
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
            child: Arc::new(Mutex::new(child)),
            stdout: Some(stdout),
            stderr,
            agent_id: request.agent.id,
            session_id: request.session.id,
            operation_id: request.operation_id,
            monitoring: false,
            final_output_path,
            relay_guard,
            execution_context: process_context,
            output_format,
        };
        processes.insert(process_id.clone(), managed);
        Ok(StartedGenerationProcess { process_id })
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
            child,
            stdout,
            stderr,
            agent_id,
            session_id,
            operation_id,
            final_output_path,
            relay_guard,
            execution_context,
            output_format,
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
                managed.child.clone(),
                managed.stdout.take().ok_or_else(|| {
                    AgentRuntimeApplicationError::Process(
                        "CLI process stdout unavailable.".to_string(),
                    )
                })?,
                managed.stderr.take(),
                managed.agent_id.clone(),
                managed.session_id.clone(),
                managed.operation_id.clone(),
                managed.final_output_path.clone(),
                managed.relay_guard.clone(),
                managed.execution_context.clone(),
                managed.output_format,
            )
        };
        let processes = self.processes.clone();
        let process_id = process_id.to_string();
        let logging = self.logging.clone();
        let clock = self.clock.clone();
        let telemetry = self.telemetry.clone();
        let event_sequence = self.event_sequence.clone();
        thread::spawn(move || {
            ProcessMonitor {
                child,
                stdout,
                stderr,
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
        let mut child = managed
            .child
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        if let Some(status) = child
            .try_wait()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?
        {
            if !managed.monitoring {
                let execution_status = if status.success() {
                    ExecutionStatus::Succeeded
                } else {
                    ExecutionStatus::Failed
                };
                let _ = self.telemetry.finish_span(
                    &managed.execution_context.run_id,
                    &managed.execution_context.span_id,
                    execution_status,
                    &self.clock.now(),
                    (!status.success()).then_some("process_exit_failed"),
                );
            }
            return Ok(false);
        }
        if let Err(error) = child.kill() {
            let _ = self.telemetry.finish_span(
                &managed.execution_context.run_id,
                &managed.execution_context.span_id,
                ExecutionStatus::Incomplete,
                &self.clock.now(),
                Some("process_cancel_failed"),
            );
            return Err(AgentRuntimeApplicationError::Process(error.to_string()));
        }
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
            let _ = child.wait();
            cleanup_final_output(managed.final_output_path.as_deref());
        }
        Ok(true)
    }
}

impl ProcessMonitor {
    fn run(self) {
        let ProcessMonitor {
            child,
            stdout,
            stderr,
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
        } = self;
        let stderr_handle = thread::spawn(move || read_stderr(stderr));
        let parser = output_parser_for_format(output_format);
        let mut terminal_error = None;
        let mut emitted_content = false;
        let mut first_visible_output = false;
        // Captured from the CLI's own completion line (if it carried usage), mirroring
        // how `terminal_error` is captured across the read loop. Attached to the
        // terminal `GenerationProcessEvent::Completed` below rather than acted on
        // immediately, since the exit code (not this line) decides success/failure.
        let mut reported_usage: Option<ReportedUsageTotals> = None;
        for line in BufReader::new(stdout).lines() {
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
                        "Failed to read Agent CLI output: {error}"
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
        // Reap the child *without* holding the `child` lock across the blocking wait.
        // `stop_generation` locks the same `Arc<Mutex<Child>>` to kill the process, so
        // holding the lock across `wait()` — which blocks until the process actually
        // exits — deadlocks any user-initiated cancellation when a CLI closes stdout but
        // keeps running (daemonized / detached grandchildren). Poll `try_wait()` with
        // short holds of the lock so a concurrent `stop_generation` kill can proceed.
        let exit_status = reap_without_holding_child_lock(&child);
        let stderr_output = stderr_handle.join().unwrap_or_default();
        if !stderr_output.trim().is_empty() {
            let _ = sink.handle(GenerationProcessEvent::Stderr(
                stderr_output.trim().to_string(),
            ));
        }
        if terminal_error.is_none()
            && exit_status.as_ref().is_ok_and(|status| status.success())
            && !emitted_content
        {
            if let Some(final_message) = read_final_output(final_output_path.as_deref()) {
                let _ = sink.handle(GenerationProcessEvent::Token(final_message));
                emitted_content = true;
            }
        }
        let exit_attributes = match &exit_status {
            Ok(status) => safe_attributes([(
                "process.exit.code".to_string(),
                SafeAttributeValue::Integer(i64::from(status.code().unwrap_or(-1))),
            )]),
            Err(_) => SafeAttributes::default(),
        };
        let terminal = compose_terminal_event(
            terminal_error,
            exit_status
                .as_ref()
                .map(|status| {
                    if status.success() {
                        ProcessExitOutcome::Success
                    } else {
                        ProcessExitOutcome::Failure {
                            status: status.to_string(),
                        }
                    }
                })
                .map_err(|error| error.clone()),
            &stderr_output,
            reported_usage,
        );
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
                agent_id: Some(agent_id),
                session_id: Some(session_id),
                operation_id: Some(operation_id),
                run_id: Some(execution_context.run_id.as_str().to_string()),
                trace_id: Some(execution_context.trace_id.as_str().to_string()),
                span_id: Some(execution_context.span_id.as_str().to_string()),
                occurred_at: clock.now(),
            });
        }
        cleanup_final_output(final_output_path.as_deref());
        if let Some(guard) = relay_guard {
            let _ = guard.cleanup();
        }
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
    Path::new(executable)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn safe_attributes(
    entries: impl IntoIterator<Item = (String, SafeAttributeValue)>,
) -> SafeAttributes {
    SafeAttributes::try_from_entries(entries).unwrap_or_default()
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

fn read_stderr(stderr: Option<ChildStderr>) -> String {
    let Some(stderr) = stderr else {
        return String::new();
    };
    BufReader::new(stderr)
        .lines()
        .map_while(Result::ok)
        .collect::<Vec<_>>()
        .join("\n")
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

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// Reaps a managed child process without holding the lock across the blocking wait.
///
/// `stop_generation` locks the same `Arc<Mutex<Child>>` to kill a runaway CLI, so a
/// monitor that holds the lock across `wait()` — which blocks until the process
/// actually exits — deadlocks cancellation whenever a CLI closes stdout but keeps
/// running (daemonized / detached grandchildren). Polling `try_wait()` with short lock
/// holds lets a concurrent `stop_generation` `kill()` proceed; once it has (or the
/// process exits on its own), `try_wait()` returns the exit status.
fn reap_without_holding_child_lock(
    child: &Arc<Mutex<Child>>,
) -> Result<std::process::ExitStatus, String> {
    const POLL_INTERVAL: Duration = Duration::from_millis(50);
    loop {
        let status = child
            .lock()
            .map_err(|error| error.to_string())?
            .try_wait()
            .map_err(|error| error.to_string())?;
        if let Some(status) = status {
            return Ok(status);
        }
        thread::sleep(POLL_INTERVAL);
    }
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
    use crate::contexts::agent_runtime::application::GenerationProcessFailureKind;
    use crate::test_support::TempDirectory;

    fn failure(message: &str) -> GenerationProcessFailure {
        GenerationProcessFailure::non_retryable(message)
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
}
