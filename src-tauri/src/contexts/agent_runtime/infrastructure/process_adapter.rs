use super::providers::{
    add_codex_output_capture_args, build_invocation, output_parser_for, ProviderOutputEvent,
    ProviderPromptDelivery,
};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentProcessEventSink,
    AgentProcessGateway, AgentRuntimeApplicationError, GenerationProcessEvent,
    GenerationProcessRequest, StartedGenerationProcess, WorkflowLaunchOutcome,
    WorkflowLaunchRequest,
};
use crate::contexts::agent_runtime::domain::{AgentAvailability, InteractionMode};
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
}

impl RuntimeAgentProcessAdapter {
    pub(crate) fn new(logging: Arc<dyn AgentLoggingPort>, clock: Arc<dyn AgentClockPort>) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            process_ids: Arc::new(AtomicU64::new(0)),
            logging,
            clock,
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
        let mut spec = build_invocation(
            &request.agent.id,
            executable,
            &request.effective_prompt,
            request.session.runtime_session_id.as_deref(),
            &request.cli_profile.managed_args,
        )
        .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
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
        if let Some(folder) = request
            .session
            .folder
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            command.current_dir(folder);
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

        let mut child = command
            .spawn()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        if spec.prompt_delivery == ProviderPromptDelivery::Stdin {
            if let Some(mut stdin) = child.stdin.take() {
                if let Err(error) = stdin
                    .write_all(request.effective_prompt.as_bytes())
                    .and_then(|_| stdin.write_all(b"\n"))
                {
                    terminate_child(&mut child);
                    return Err(AgentRuntimeApplicationError::Process(error.to_string()));
                }
            }
        }
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                terminate_child(&mut child);
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
        let managed = ManagedProcess {
            child: Arc::new(Mutex::new(child)),
            stdout: Some(stdout),
            stderr,
            agent_id: request.agent.id,
            session_id: request.session.id,
            operation_id: request.operation_id,
            monitoring: false,
            final_output_path,
        };
        self.processes
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?
            .insert(process_id.clone(), managed);
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
            occurred_at: self.clock.now(),
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
        let (child, stdout, stderr, agent_id, session_id, operation_id, final_output_path) = {
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
            )
        };
        let processes = self.processes.clone();
        let process_id = process_id.to_string();
        let logging = self.logging.clone();
        let clock = self.clock.clone();
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
            }
            .run();
            if let Ok(mut processes) = processes.lock() {
                processes.remove(&process_id);
            }
        });
        Ok(())
    }

    fn stop_generation(&self, process_id: &str) -> Result<bool, AgentRuntimeApplicationError> {
        let managed = self
            .processes
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?
            .remove(process_id);
        let Some(managed) = managed else {
            return Ok(false);
        };
        let mut child = managed
            .child
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        if child
            .try_wait()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?
            .is_some()
        {
            return Ok(false);
        }
        child
            .kill()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        if !managed.monitoring {
            let _ = child.wait();
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
        } = self;
        let stderr_handle = thread::spawn(move || read_stderr(stderr));
        let parser = output_parser_for(&agent_id);
        let mut terminal_error = None;
        let mut emitted_content = false;
        for line in BufReader::new(stdout).lines() {
            let event = match line {
                Ok(line) => match parser.parse_line(&line) {
                    ProviderOutputEvent::Token(delta) => {
                        emitted_content = true;
                        Some(GenerationProcessEvent::Token(delta))
                    }
                    ProviderOutputEvent::Thinking(content) => {
                        Some(GenerationProcessEvent::Thinking(content))
                    }
                    ProviderOutputEvent::ToolUse(tool_use) => {
                        Some(GenerationProcessEvent::ToolUse(tool_use))
                    }
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
                    ProviderOutputEvent::Completed | ProviderOutputEvent::Empty => None,
                },
                Err(error) => {
                    terminal_error = Some(format!("Failed to read Agent CLI output: {error}"));
                    break;
                }
            };
            if let Some(event) = event {
                if let Err(error) = sink.handle(event) {
                    terminal_error =
                        Some(format!("Agent generation event handling failed: {error}"));
                    break;
                }
            }
        }
        let exit_status = child
            .lock()
            .map_err(|error| error.to_string())
            .and_then(|mut child| child.wait().map_err(|error| error.to_string()));
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
            }
        }
        let terminal = match (terminal_error, exit_status) {
            (Some(error), _) => GenerationProcessEvent::Failed(error),
            (None, Ok(status)) if status.success() => GenerationProcessEvent::Completed,
            (None, Ok(status)) => {
                GenerationProcessEvent::Failed(if stderr_output.trim().is_empty() {
                    format!("Agent CLI exited with status {status}.")
                } else {
                    stderr_output.trim().to_string()
                })
            }
            (None, Err(error)) => GenerationProcessEvent::Failed(error),
        };
        if let Err(error) = sink.handle(terminal) {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Error,
                category: "session.runtime.cli".to_string(),
                message: format!("Agent generation terminal event failed: {error}"),
                agent_id: Some(agent_id),
                session_id: Some(session_id),
                operation_id: Some(operation_id),
                occurred_at: clock.now(),
            });
        }
        cleanup_final_output(final_output_path.as_deref());
    }
}

fn read_final_output(path: Option<&Path>) -> Option<String> {
    let path = path?;
    fs::read_to_string(path)
        .ok()
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty())
}

fn cleanup_final_output(path: Option<&Path>) {
    if let Some(path) = path {
        let _ = fs::remove_file(path);
    }
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
    if agent_id != "opencode" {
        return executable.to_string();
    }
    resolve_opencode_npm_shim(executable).unwrap_or_else(|| executable.to_string())
}

fn resolve_opencode_npm_shim(executable: &str) -> Option<String> {
    let path = Path::new(executable);
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    if extension != "cmd" && extension != "ps1" {
        return None;
    }
    if path.file_stem()?.to_string_lossy().to_ascii_lowercase() != "opencode" {
        return None;
    }
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
