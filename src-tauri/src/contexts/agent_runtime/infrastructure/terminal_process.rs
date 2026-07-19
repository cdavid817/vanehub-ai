use super::providers::{build_interactive_invocation, output_parser_for, ProviderOutputEvent};
use super::terminal_wrapper::{
    default_agent_terminal_shell, generate_agent_terminal_wrapper, AgentTerminalWrapperRequest,
};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentRuntimeApplicationError,
    AgentSessionGateway, AgentTerminalCapability, AgentTerminalEvent, AgentTerminalEventPort,
    AgentTerminalGateway, AgentTerminalInputRequest, AgentTerminalProcessRequest,
    AgentTerminalSession, AgentTerminalSize, AgentTerminalState, ResizeAgentTerminalRequest,
    StopAgentTerminalRequest,
};
use crate::contexts::agent_runtime::domain::AgentLifecycle;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

struct ManagedAgentTerminal {
    terminal_id: String,
    session_id: String,
    agent_id: String,
    runtime_session_id: Option<String>,
    last_active_at: i64,
    size: AgentTerminalSize,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
}

#[derive(Clone)]
pub(crate) struct PortablePtyAgentTerminalRuntime {
    terminals: Arc<Mutex<HashMap<String, ManagedAgentTerminal>>>,
    terminal_ids: Arc<AtomicU64>,
    events: Arc<dyn AgentTerminalEventPort>,
    sessions: Arc<dyn AgentSessionGateway>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    wrapper_dir: PathBuf,
}

impl PortablePtyAgentTerminalRuntime {
    pub(crate) fn new(
        events: Arc<dyn AgentTerminalEventPort>,
        sessions: Arc<dyn AgentSessionGateway>,
        logging: Arc<dyn AgentLoggingPort>,
        clock: Arc<dyn AgentClockPort>,
        wrapper_dir: PathBuf,
    ) -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
            terminal_ids: Arc::new(AtomicU64::new(0)),
            events,
            sessions,
            logging,
            clock,
            wrapper_dir,
        }
    }

    fn next_terminal_id(&self, session_id: &str) -> String {
        format!(
            "agent-terminal-{}-{}",
            safe_file_segment(session_id),
            self.terminal_ids.fetch_add(1, Ordering::Relaxed) + 1
        )
    }

    fn attach_existing(
        &self,
        session_id: &str,
    ) -> Result<Option<AgentTerminalSession>, AgentRuntimeApplicationError> {
        let now = now_timestamp(self.clock.as_ref());
        let mut terminals = self.lock_terminals()?;
        let Some(terminal) = terminals.get_mut(session_id) else {
            return Ok(None);
        };
        terminal.last_active_at = now;
        Ok(Some(agent_terminal_session(terminal)))
    }

    fn lock_terminals(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, HashMap<String, ManagedAgentTerminal>>,
        AgentRuntimeApplicationError,
    > {
        self.terminals
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))
    }

    fn record_log(
        &self,
        level: AgentLogLevel,
        message: String,
        agent_id: Option<&str>,
        session_id: Option<&str>,
    ) {
        let _ = self.logging.record(AgentLog {
            level,
            category: "session.agent_terminal".to_string(),
            message,
            agent_id: agent_id.map(str::to_string),
            session_id: session_id.map(str::to_string),
            operation_id: None,
            occurred_at: self.clock.now(),
        });
    }
}

impl AgentTerminalGateway for PortablePtyAgentTerminalRuntime {
    fn open_or_attach(
        &self,
        request: AgentTerminalProcessRequest,
    ) -> Result<AgentTerminalSession, AgentRuntimeApplicationError> {
        if let Some(session) = self.attach_existing(&request.session.id)? {
            self.record_log(
                AgentLogLevel::Info,
                "Attached retained Agent terminal process.".to_string(),
                Some(&request.agent.id),
                Some(&request.session.id),
            );
            return Ok(session);
        }

        let executable =
            normalize_interactive_executable(&request.agent.id, &request.cli_profile.executable);
        let invocation = build_interactive_invocation(
            &request.agent.id,
            executable,
            request.session.runtime_session_id.as_deref(),
            &request.cli_profile.managed_args,
        )
        .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        let terminal_id = self.next_terminal_id(&request.session.id);
        let (shell, shell_executable) = default_agent_terminal_shell();
        let wrapper = generate_agent_terminal_wrapper(&AgentTerminalWrapperRequest {
            terminal_id: terminal_id.clone(),
            session_folder: request
                .session
                .folder
                .as_ref()
                .filter(|folder| !folder.trim().is_empty())
                .map(PathBuf::from),
            executable: invocation.executable.clone(),
            args: invocation.args.clone(),
            shell,
            shell_executable,
            wrapper_dir: self.wrapper_dir.clone(),
        })
        .map_err(AgentRuntimeApplicationError::Process)?;

        self.record_log(
            AgentLogLevel::Info,
            format!("Starting Agent terminal: {}", wrapper.redacted_command),
            Some(&request.agent.id),
            Some(&request.session.id),
        );

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(terminal_size(&request.size))
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        let mut command = CommandBuilder::new(wrapper.executable);
        command.args(wrapper.args);
        if let Some(folder) = request
            .session
            .folder
            .as_deref()
            .filter(|folder| !folder.trim().is_empty())
        {
            command.cwd(folder);
        }
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        drop(pair.slave);
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        let child = Arc::new(Mutex::new(child));
        let terminal = ManagedAgentTerminal {
            terminal_id: terminal_id.clone(),
            session_id: request.session.id.clone(),
            agent_id: request.agent.id.clone(),
            runtime_session_id: request.session.runtime_session_id.clone(),
            last_active_at: now_timestamp(self.clock.as_ref()),
            size: request.size.clone(),
            master: pair.master,
            writer,
            child: child.clone(),
        };
        let response = agent_terminal_session(&terminal);
        self.lock_terminals()?
            .insert(request.session.id.clone(), terminal);

        let events = self.events.clone();
        let sessions = self.sessions.clone();
        let logging = self.logging.clone();
        let clock = self.clock.clone();
        let terminals = self.terminals.clone();
        let session_id = request.session.id;
        let agent_id = request.agent.id;
        thread::spawn(move || {
            let parser = output_parser_for(&agent_id);
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        let content = String::from_utf8_lossy(&buffer[..count]).to_string();
                        let _ = events.publish_terminal(AgentTerminalEvent::Output {
                            terminal_id: terminal_id.clone(),
                            session_id: session_id.clone(),
                            content: content.clone(),
                        });
                        if let Ok(mut terminals) = terminals.lock() {
                            if let Some(terminal) = terminals.get_mut(&session_id) {
                                terminal.last_active_at = now_timestamp(clock.as_ref());
                            }
                        }
                        for line in content.lines() {
                            if let ProviderOutputEvent::SessionId(runtime_session_id) =
                                parser.parse_line(line)
                            {
                                let _ = sessions
                                    .update_runtime_session_id(&session_id, &runtime_session_id);
                                if let Ok(mut terminals) = terminals.lock() {
                                    if let Some(terminal) = terminals.get_mut(&session_id) {
                                        terminal.runtime_session_id =
                                            Some(runtime_session_id.clone());
                                    }
                                }
                                let _ =
                                    events.publish_terminal(AgentTerminalEvent::RuntimeSessionId {
                                        terminal_id: terminal_id.clone(),
                                        session_id: session_id.clone(),
                                        runtime_session_id,
                                    });
                            }
                        }
                    }
                    Err(_) => break,
                }
            }

            let exit_result = child
                .lock()
                .map_err(|error| error.to_string())
                .and_then(|mut child| child.wait().map_err(|error| error.to_string()));
            let (state, error) = match exit_result {
                Ok(status) if status.success() => (AgentTerminalState::Stopped, None),
                Ok(status) => (
                    AgentTerminalState::Failed,
                    Some(format!("Agent terminal exited with status {status}.")),
                ),
                Err(error) => (AgentTerminalState::Failed, Some(error)),
            };
            if let Ok(mut terminals) = terminals.lock() {
                if terminals
                    .get(&session_id)
                    .is_some_and(|terminal| terminal.terminal_id == terminal_id)
                {
                    terminals.remove(&session_id);
                }
            }
            let lifecycle = match state {
                AgentTerminalState::Failed => AgentLifecycle::Failed,
                _ => AgentLifecycle::Stopped,
            };
            let _ = sessions.update_lifecycle(&session_id, lifecycle);
            let _ = events.publish_terminal(AgentTerminalEvent::State {
                terminal_id,
                session_id: session_id.clone(),
                state,
                error: error.clone(),
            });
            let _ = logging.record(AgentLog {
                level: if error.is_some() {
                    AgentLogLevel::Warn
                } else {
                    AgentLogLevel::Info
                },
                category: "session.agent_terminal".to_string(),
                message: error.unwrap_or_else(|| "Agent terminal process exited.".to_string()),
                agent_id: Some(agent_id),
                session_id: Some(session_id),
                operation_id: None,
                occurred_at: clock.now(),
            });
        });

        Ok(response)
    }

    fn input(
        &self,
        request: AgentTerminalInputRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut terminals = self.lock_terminals()?;
        let terminal = terminal_by_id_mut(&mut terminals, &request.terminal_id)?;
        terminal
            .writer
            .write_all(request.content.as_bytes())
            .and_then(|_| terminal.writer.flush())
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        terminal.last_active_at = now_timestamp(self.clock.as_ref());
        Ok(())
    }

    fn resize(
        &self,
        request: ResizeAgentTerminalRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut terminals = self.lock_terminals()?;
        let terminal = terminal_by_id_mut(&mut terminals, &request.terminal_id)?;
        terminal
            .master
            .resize(terminal_size(&request.size))
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        terminal.size = request.size;
        terminal.last_active_at = now_timestamp(self.clock.as_ref());
        Ok(())
    }

    fn stop(
        &self,
        request: StopAgentTerminalRequest,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let terminal = {
            let mut terminals = self.lock_terminals()?;
            let Some(session_id) = terminals.iter().find_map(|(session_id, terminal)| {
                (terminal.terminal_id == request.terminal_id).then(|| session_id.clone())
            }) else {
                return Ok(false);
            };
            terminals.remove(&session_id)
        };
        let Some(terminal) = terminal else {
            return Ok(false);
        };
        terminate_terminal_child(terminal.child.as_ref())?;
        self.sessions
            .update_lifecycle(&terminal.session_id, AgentLifecycle::Stopped)?;
        let _ = self.events.publish_terminal(AgentTerminalEvent::State {
            terminal_id: terminal.terminal_id,
            session_id: terminal.session_id.clone(),
            state: AgentTerminalState::Stopped,
            error: None,
        });
        self.record_log(
            AgentLogLevel::Info,
            "Agent terminal process stopped.".to_string(),
            Some(&terminal.agent_id),
            Some(&terminal.session_id),
        );
        Ok(true)
    }

    fn cleanup_idle(
        &self,
        idle_after_seconds: i64,
    ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        let now = now_timestamp(self.clock.as_ref());
        let terminal_ids = self
            .lock_terminals()?
            .values()
            .filter(|terminal| now.saturating_sub(terminal.last_active_at) > idle_after_seconds)
            .map(|terminal| terminal.terminal_id.clone())
            .collect::<Vec<_>>();
        let mut stopped = Vec::new();
        for terminal_id in terminal_ids {
            if self.stop(StopAgentTerminalRequest {
                terminal_id: terminal_id.clone(),
            })? {
                stopped.push(terminal_id);
            }
        }
        Ok(stopped)
    }

    fn shutdown(&self) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        let terminal_ids = self
            .lock_terminals()?
            .values()
            .map(|terminal| terminal.terminal_id.clone())
            .collect::<Vec<_>>();
        let mut stopped = Vec::new();
        for terminal_id in terminal_ids {
            if self.stop(StopAgentTerminalRequest {
                terminal_id: terminal_id.clone(),
            })? {
                stopped.push(terminal_id);
            }
        }
        Ok(stopped)
    }
}

fn terminal_by_id_mut<'a>(
    terminals: &'a mut HashMap<String, ManagedAgentTerminal>,
    terminal_id: &str,
) -> Result<&'a mut ManagedAgentTerminal, AgentRuntimeApplicationError> {
    terminals
        .values_mut()
        .find(|terminal| terminal.terminal_id == terminal_id)
        .ok_or_else(|| {
            AgentRuntimeApplicationError::Process("Agent terminal is not connected.".to_string())
        })
}

fn terminate_terminal_child(
    child: &Mutex<Box<dyn Child + Send + Sync>>,
) -> Result<(), AgentRuntimeApplicationError> {
    let mut child = child
        .lock()
        .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
    if child
        .try_wait()
        .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?
        .is_none()
    {
        child
            .kill()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
    }
    let _ = child.wait();
    Ok(())
}

fn agent_terminal_session(terminal: &ManagedAgentTerminal) -> AgentTerminalSession {
    AgentTerminalSession {
        terminal_id: terminal.terminal_id.clone(),
        session_id: terminal.session_id.clone(),
        agent_id: terminal.agent_id.clone(),
        state: AgentTerminalState::Running,
        capability: AgentTerminalCapability::Native,
        size: terminal.size.clone(),
        runtime_session_id: terminal.runtime_session_id.clone(),
        retained: true,
    }
}

fn terminal_size(size: &AgentTerminalSize) -> PtySize {
    PtySize {
        rows: size.rows.clamp(1, 200),
        cols: size.cols.clamp(1, 500),
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn now_timestamp(clock: &dyn AgentClockPort) -> i64 {
    chrono::DateTime::parse_from_rfc3339(&clock.now())
        .map(|value| value.timestamp())
        .unwrap_or(0)
}

fn normalize_interactive_executable(agent_id: &str, executable: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::CliProfileSnapshot;
    use crate::contexts::agent_runtime::domain::{AgentAvailability, InteractionMode};
    use crate::test_support::TempDirectory;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn terminal_size_bounds_rows_and_cols_for_pty() {
        let size = terminal_size(&AgentTerminalSize { rows: 0, cols: 900 });

        assert_eq!(size.rows, 1);
        assert_eq!(size.cols, 500);
    }

    #[test]
    fn opencode_windows_shim_resolves_to_real_binary_when_present() {
        let directory = TempDirectory::new("opencode-shim");
        let shim = directory.path().join("opencode.cmd");
        let binary = directory
            .path()
            .join("node_modules")
            .join("opencode-ai")
            .join("bin")
            .join("opencode.exe");
        std::fs::create_dir_all(binary.parent().expect("parent")).expect("dirs");
        std::fs::write(&binary, "fixture").expect("binary");

        assert_eq!(
            normalize_interactive_executable("opencode", &shim.to_string_lossy()),
            binary.to_string_lossy().to_string()
        );
    }

    #[test]
    fn agent_terminal_session_projection_uses_native_retained_running_state() {
        let terminal = ManagedAgentTerminal {
            terminal_id: "terminal-1".to_string(),
            session_id: "session-1".to_string(),
            agent_id: "codex-cli".to_string(),
            runtime_session_id: Some("runtime-1".to_string()),
            last_active_at: 1,
            size: AgentTerminalSize { rows: 24, cols: 80 },
            master: native_pty_system()
                .openpty(terminal_size(&AgentTerminalSize { rows: 1, cols: 1 }))
                .expect("pty")
                .master,
            writer: Box::new(Vec::<u8>::new()),
            child: Arc::new(Mutex::new(dummy_child())),
        };

        let session = agent_terminal_session(&terminal);

        assert_eq!(session.terminal_id, "terminal-1");
        assert_eq!(session.state, AgentTerminalState::Running);
        assert_eq!(session.capability, AgentTerminalCapability::Native);
        assert!(session.retained);
    }

    fn dummy_child() -> Box<dyn Child + Send + Sync> {
        let pair = native_pty_system()
            .openpty(terminal_size(&AgentTerminalSize { rows: 1, cols: 1 }))
            .expect("pty");
        let command = if cfg!(target_os = "windows") {
            CommandBuilder::new("cmd.exe")
        } else {
            CommandBuilder::new("/bin/sh")
        };
        pair.slave.spawn_command(command).expect("child")
    }

    #[allow(dead_code)]
    fn _request_fixture() -> (TempDirectory, AgentTerminalProcessRequest) {
        let directory = TempDirectory::new("agent-terminal-process-request");
        (
            directory,
            AgentTerminalProcessRequest {
                session: crate::contexts::agent_runtime::application::AgentSession {
                    id: "session-1".to_string(),
                    agent_id: "codex-cli".to_string(),
                    interaction_mode: InteractionMode::Cli,
                    lifecycle: AgentLifecycle::Idle,
                    folder: None,
                    runtime_session_id: None,
                    archived: false,
                },
                agent: crate::contexts::agent_runtime::application::AgentView {
                    id: "codex-cli".to_string(),
                    display_name: "Codex".to_string(),
                    provider: "openai".to_string(),
                    managed_sdk_dependency_id: None,
                    launch: crate::contexts::agent_runtime::application::AgentLaunchView {
                        kind: "cli".to_string(),
                        command: None,
                        url: None,
                        executable_name: Some("codex".to_string()),
                    },
                    supported_interaction_modes: vec![InteractionMode::Cli],
                    availability: AgentAvailability::Available,
                    unavailable_reason: None,
                    capability_tags: Vec::new(),
                },
                cli_profile: CliProfileSnapshot {
                    executable: "codex".to_string(),
                    selections: BTreeMap::from([("model".to_string(), json!("gpt-5"))]),
                    managed_args: vec!["--model".to_string(), "gpt-5".to_string()],
                },
                size: AgentTerminalSize { rows: 24, cols: 80 },
            },
        )
    }
}
