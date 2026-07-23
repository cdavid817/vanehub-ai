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

const RETAINED_TERMINAL_TRANSCRIPT_BYTES: usize = 1_000_000;

/// Larger reads coalesce bursty PTY output into fewer IPC events without adding latency:
/// a read still returns as soon as any bytes are available, so interactive echo is
/// unaffected, while a flood of build output emits far fewer events than a 4 KiB buffer.
const TERMINAL_READ_BUFFER_BYTES: usize = 64 * 1024;

/// Upper bound on an unterminated parse line, so newline-less output (e.g. `\r` progress
/// bars) cannot grow the session-id parse buffer without bound.
const MAX_PARSE_LINE_BYTES: usize = 256 * 1024;

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
    transcript: String,
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
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: self.clock.now(),
        });
    }
}

impl AgentTerminalGateway for PortablePtyAgentTerminalRuntime {
    fn attach_retained(
        &self,
        session_id: &str,
    ) -> Result<Option<AgentTerminalSession>, AgentRuntimeApplicationError> {
        let mut terminal_registry = self.lock_terminals()?;
        if let Some(terminal) = terminal_registry.get_mut(session_id) {
            terminal.last_active_at = now_timestamp(self.clock.as_ref());
            let session = agent_terminal_session(terminal);
            let transcript = terminal.transcript.clone();
            let agent_id = terminal.agent_id.clone();
            drop(terminal_registry);
            self.record_log(
                AgentLogLevel::Info,
                "Attached retained Agent terminal process.".to_string(),
                Some(&agent_id),
                Some(session_id),
            );
            let _ = self.events.publish_terminal(AgentTerminalEvent::State {
                terminal_id: session.terminal_id.clone(),
                session_id: session.session_id.clone(),
                state: AgentTerminalState::Running,
                error: None,
            });
            if !transcript.is_empty() {
                let _ = self.events.publish_terminal(AgentTerminalEvent::Output {
                    terminal_id: session.terminal_id.clone(),
                    session_id: session.session_id.clone(),
                    content: transcript,
                });
            }
            return Ok(Some(session));
        }
        Ok(None)
    }

    fn open_or_attach(
        &self,
        request: AgentTerminalProcessRequest,
    ) -> Result<AgentTerminalSession, AgentRuntimeApplicationError> {
        let mut terminal_registry = self.lock_terminals()?;
        if let Some(terminal) = terminal_registry.get_mut(&request.session.id) {
            terminal.last_active_at = now_timestamp(self.clock.as_ref());
            let session = agent_terminal_session(terminal);
            let transcript = terminal.transcript.clone();
            drop(terminal_registry);
            self.record_log(
                AgentLogLevel::Info,
                "Attached retained Agent terminal process.".to_string(),
                Some(&request.agent.id),
                Some(&request.session.id),
            );
            let _ = self.events.publish_terminal(AgentTerminalEvent::State {
                terminal_id: session.terminal_id.clone(),
                session_id: session.session_id.clone(),
                state: AgentTerminalState::Running,
                error: None,
            });
            if !transcript.is_empty() {
                let _ = self.events.publish_terminal(AgentTerminalEvent::Output {
                    terminal_id: session.terminal_id.clone(),
                    session_id: session.session_id.clone(),
                    content: transcript,
                });
            }
            return Ok(session);
        }
        let agent_id_for_error = request.agent.id.clone();
        let session_id_for_error = request.session.id.clone();
        let executable =
            normalize_interactive_executable(&request.agent.id, &request.cli_profile.executable);
        let invocation = build_interactive_invocation(
            &request.agent.id,
            executable,
            request.session.runtime_session_id.as_deref(),
            &request.cli_profile.managed_args,
        )
        .map_err(|error| {
            let message = format!("Failed to prepare Agent terminal invocation: {error}");
            self.record_log(
                AgentLogLevel::Error,
                message.clone(),
                Some(&agent_id_for_error),
                Some(&session_id_for_error),
            );
            AgentRuntimeApplicationError::Process(message)
        })?;
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
        .map_err(|error| {
            let message = format!("Failed to prepare Agent terminal wrapper: {error}");
            self.record_log(
                AgentLogLevel::Error,
                message.clone(),
                Some(&request.agent.id),
                Some(&request.session.id),
            );
            AgentRuntimeApplicationError::Process(message)
        })?;

        self.record_log(
            AgentLogLevel::Info,
            format!("Starting Agent terminal: {}", wrapper.redacted_command),
            Some(&request.agent.id),
            Some(&request.session.id),
        );

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(terminal_size(&request.size))
            .map_err(|error| {
                let message = format!("Failed to allocate Agent terminal PTY: {error}");
                self.record_log(
                    AgentLogLevel::Error,
                    message.clone(),
                    Some(&request.agent.id),
                    Some(&request.session.id),
                );
                AgentRuntimeApplicationError::Process(message)
            })?;
        let redacted_command = wrapper.redacted_command.clone();
        let mut command = CommandBuilder::new(wrapper.executable.clone());
        command.args(wrapper.args.clone());
        if let Some(folder) = request
            .session
            .folder
            .as_deref()
            .filter(|folder| !folder.trim().is_empty())
        {
            command.cwd(folder);
        }
        let child = pair.slave.spawn_command(command).map_err(|error| {
            let message = format!(
                "Failed to spawn Agent terminal process for {}: {error}",
                redacted_command
            );
            self.record_log(
                AgentLogLevel::Error,
                message.clone(),
                Some(&request.agent.id),
                Some(&request.session.id),
            );
            AgentRuntimeApplicationError::Process(message)
        })?;
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().map_err(|error| {
            let message = format!("Failed to attach Agent terminal reader: {error}");
            self.record_log(
                AgentLogLevel::Error,
                message.clone(),
                Some(&request.agent.id),
                Some(&request.session.id),
            );
            AgentRuntimeApplicationError::Process(message)
        })?;
        let writer = pair.master.take_writer().map_err(|error| {
            let message = format!("Failed to attach Agent terminal writer: {error}");
            self.record_log(
                AgentLogLevel::Error,
                message.clone(),
                Some(&request.agent.id),
                Some(&request.session.id),
            );
            AgentRuntimeApplicationError::Process(message)
        })?;
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
            transcript: String::new(),
        };
        let response = agent_terminal_session(&terminal);
        terminal_registry.insert(request.session.id.clone(), terminal);

        let events = self.events.clone();
        let sessions = self.sessions.clone();
        let logging = self.logging.clone();
        let clock = self.clock.clone();
        let terminals = self.terminals.clone();
        let session_id = request.session.id;
        let agent_id = request.agent.id;
        drop(terminal_registry);
        thread::spawn(move || {
            let parser = output_parser_for(&agent_id);
            let mut buffer = [0u8; TERMINAL_READ_BUFFER_BYTES];
            // Reads land on arbitrary byte boundaries, so a multi-byte UTF-8 sequence
            // (中文 / emoji / TUI box-drawing) can be split across two reads. Carry the
            // incomplete trailing bytes until the next read completes them, otherwise
            // `from_utf8_lossy` would emit U+FFFD in the middle of valid output.
            let mut pending: Vec<u8> = Vec::new();
            // The provider parser only recognises whole `\n`-terminated lines (a session
            // marker is line-delimited JSON), so accumulate across reads and keep the
            // trailing partial line until its newline arrives — otherwise a marker split
            // across two reads is silently dropped.
            let mut line_buffer = String::new();
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        pending.extend_from_slice(&buffer[..count]);
                        let content = take_decodable_utf8(&mut pending);
                        if content.is_empty() {
                            continue;
                        }
                        let _ = events.publish_terminal(AgentTerminalEvent::Output {
                            terminal_id: terminal_id.clone(),
                            session_id: session_id.clone(),
                            content: content.clone(),
                        });
                        if let Ok(mut terminals) = terminals.lock() {
                            if let Some(terminal) = terminals.get_mut(&session_id) {
                                terminal.last_active_at = now_timestamp(clock.as_ref());
                                append_terminal_transcript(&mut terminal.transcript, &content);
                            }
                        }
                        line_buffer.push_str(&content);
                        drain_complete_lines(&mut line_buffer, |line| {
                            if let ProviderOutputEvent::SessionId(runtime_session_id) =
                                parser.parse_line(line)
                            {
                                let event = record_runtime_session_id(
                                    terminals.as_ref(),
                                    &terminal_id,
                                    &session_id,
                                    runtime_session_id,
                                    |session_id, runtime_session_id| {
                                        let _ = sessions.update_runtime_session_id(
                                            session_id,
                                            runtime_session_id,
                                        );
                                    },
                                );
                                let _ = events.publish_terminal(event);
                            }
                        });
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
                run_id: None,
                trace_id: None,
                span_id: None,
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

fn record_runtime_session_id(
    terminals: &Mutex<HashMap<String, ManagedAgentTerminal>>,
    terminal_id: &str,
    session_id: &str,
    runtime_session_id: String,
    persist: impl FnOnce(&str, &str),
) -> AgentTerminalEvent {
    persist(session_id, &runtime_session_id);
    if let Ok(mut terminals) = terminals.lock() {
        if let Some(terminal) = terminals.get_mut(session_id) {
            terminal.runtime_session_id = Some(runtime_session_id.clone());
        }
    }
    AgentTerminalEvent::RuntimeSessionId {
        terminal_id: terminal_id.to_string(),
        session_id: session_id.to_string(),
        runtime_session_id,
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

/// Splits off the longest valid UTF-8 prefix of `pending`, leaving any incomplete
/// trailing multi-byte sequence in place for the next read to complete. A genuinely
/// invalid byte (not just a truncated tail) is decoded lossily so the reader never
/// wedges on malformed output.
fn take_decodable_utf8(pending: &mut Vec<u8>) -> String {
    let valid_up_to = match std::str::from_utf8(pending) {
        Ok(_) => pending.len(),
        // `error_len() == None` means the bytes after `valid_up_to` are an incomplete
        // sequence at the end of the buffer — keep them for the next read.
        Err(error) if error.error_len().is_none() => error.valid_up_to(),
        Err(_) => {
            let content = String::from_utf8_lossy(pending).to_string();
            pending.clear();
            return content;
        }
    };
    let tail = pending.split_off(valid_up_to);
    let head = std::mem::replace(pending, tail);
    // `head` is guaranteed valid UTF-8 by the check above; fall back rather than panic.
    String::from_utf8(head).unwrap_or_default()
}

/// Invokes `on_line` for each complete `\n`-terminated line in `line_buffer` (trailing
/// CR/LF stripped), leaving any unterminated remainder for the next read to finish.
/// An oversized newline-less remainder is discarded to keep the buffer bounded.
fn drain_complete_lines(line_buffer: &mut String, mut on_line: impl FnMut(&str)) {
    while let Some(newline) = line_buffer.find('\n') {
        let line: String = line_buffer.drain(..=newline).collect();
        on_line(line.trim_end_matches(['\n', '\r']));
    }
    if line_buffer.len() > MAX_PARSE_LINE_BYTES {
        line_buffer.clear();
    }
}

fn append_terminal_transcript(transcript: &mut String, content: &str) {
    transcript.push_str(content);
    if transcript.len() <= RETAINED_TERMINAL_TRANSCRIPT_BYTES {
        return;
    }
    let mut trim_to = transcript.len() - RETAINED_TERMINAL_TRANSCRIPT_BYTES;
    while !transcript.is_char_boundary(trim_to) {
        trim_to += 1;
    }
    transcript.drain(..trim_to);
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
    resolve_windows_npm_shim(agent_id, executable).unwrap_or_else(|| executable.to_string())
}

fn resolve_windows_npm_shim(agent_id: &str, executable: &str) -> Option<String> {
    let path = Path::new(executable);
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    if extension != "cmd" && extension != "ps1" {
        return None;
    }
    let stem = path.file_stem()?.to_string_lossy().to_ascii_lowercase();
    if expected_windows_shim_stem(agent_id) != Some(stem.as_str()) {
        return None;
    }
    if extension == "ps1" {
        let sibling_cmd = path.with_extension("cmd");
        if sibling_cmd.is_file() {
            return Some(sibling_cmd.to_string_lossy().to_string());
        }
    }
    let parent = path.parent()?;
    managed_windows_binary_candidates(agent_id, parent)
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().to_string())
}

fn expected_windows_shim_stem(agent_id: &str) -> Option<&'static str> {
    match agent_id {
        "claude-code" => Some("claude"),
        "codex-cli" => Some("codex"),
        "opencode" => Some("opencode"),
        _ => None,
    }
}

fn managed_windows_binary_candidates(agent_id: &str, shim_parent: &Path) -> Vec<PathBuf> {
    let node_modules = shim_parent.join("node_modules");
    match agent_id {
        "claude-code" => vec![
            node_modules
                .join("@anthropic-ai")
                .join("claude-code")
                .join("claude.exe"),
            node_modules
                .join("@anthropic-ai")
                .join("claude-code")
                .join("bin")
                .join("claude.exe"),
            node_modules
                .join("@anthropic-ai")
                .join("claude-code")
                .join("vendor")
                .join("claude.exe"),
        ],
        "codex-cli" => vec![
            node_modules
                .join("@openai")
                .join("codex")
                .join("bin")
                .join("codex.exe"),
            node_modules
                .join("@openai")
                .join("codex")
                .join("bin")
                .join("codex-x86_64-pc-windows-msvc.exe"),
            node_modules.join("@openai").join("codex").join("codex.exe"),
        ],
        "opencode" => vec![node_modules
            .join("opencode-ai")
            .join("bin")
            .join("opencode.exe")],
        _ => Vec::new(),
    }
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

    fn managed_terminal(runtime_session_id: Option<&str>) -> ManagedAgentTerminal {
        ManagedAgentTerminal {
            terminal_id: "terminal-1".to_string(),
            session_id: "session-1".to_string(),
            agent_id: "codex-cli".to_string(),
            runtime_session_id: runtime_session_id.map(str::to_string),
            last_active_at: 1,
            size: AgentTerminalSize { rows: 24, cols: 80 },
            master: native_pty_system()
                .openpty(terminal_size(&AgentTerminalSize { rows: 1, cols: 1 }))
                .expect("pty")
                .master,
            writer: Box::new(Vec::<u8>::new()),
            child: Arc::new(Mutex::new(dummy_child())),
            transcript: String::new(),
        }
    }

    #[test]
    fn terminal_size_bounds_rows_and_cols_for_pty() {
        let size = terminal_size(&AgentTerminalSize { rows: 0, cols: 900 });

        assert_eq!(size.rows, 1);
        assert_eq!(size.cols, 500);
    }

    #[test]
    fn windows_ps1_shim_prefers_sibling_cmd_for_interactive_launch() {
        let directory = TempDirectory::new("agent-terminal-ps1-shim");
        let shim = directory.path().join("codex.ps1");
        let cmd = directory.path().join("codex.cmd");
        std::fs::write(&shim, "fixture").expect("shim");
        std::fs::write(&cmd, "fixture").expect("cmd");

        assert_eq!(
            normalize_interactive_executable("codex-cli", &shim.to_string_lossy()),
            cmd.to_string_lossy().to_string()
        );
    }

    #[test]
    fn windows_shims_resolve_known_managed_cli_binaries_when_present() {
        let cases: [(&str, &str, &[&str]); 3] = [
            (
                "claude-code",
                "claude.cmd",
                &[
                    "node_modules",
                    "@anthropic-ai",
                    "claude-code",
                    "bin",
                    "claude.exe",
                ],
            ),
            (
                "codex-cli",
                "codex.cmd",
                &["node_modules", "@openai", "codex", "bin", "codex.exe"],
            ),
            (
                "opencode",
                "opencode.cmd",
                &["node_modules", "opencode-ai", "bin", "opencode.exe"],
            ),
        ];
        for (agent_id, shim_name, binary_parts) in cases {
            let directory = TempDirectory::new("agent-terminal-shim");
            let shim = directory.path().join(shim_name);
            let binary = binary_parts
                .iter()
                .fold(directory.path().to_path_buf(), |path, part| path.join(part));
            std::fs::write(&shim, "fixture").expect("shim");
            std::fs::create_dir_all(binary.parent().expect("parent")).expect("dirs");
            std::fs::write(&binary, "fixture").expect("binary");

            assert_eq!(
                normalize_interactive_executable(agent_id, &shim.to_string_lossy()),
                binary.to_string_lossy().to_string()
            );
        }
    }

    #[test]
    fn windows_shim_resolution_falls_back_for_unknown_or_missing_targets() {
        let directory = TempDirectory::new("agent-terminal-shim-fallback");
        let shim = directory.path().join("codex.cmd");
        std::fs::write(&shim, "fixture").expect("shim");

        assert_eq!(
            normalize_interactive_executable("codex-cli", &shim.to_string_lossy()),
            shim.to_string_lossy().to_string()
        );
        assert_eq!(
            normalize_interactive_executable("gemini-cli", &shim.to_string_lossy()),
            shim.to_string_lossy().to_string()
        );
    }

    #[test]
    fn agent_terminal_session_projection_uses_native_retained_running_state() {
        let terminal = managed_terminal(Some("runtime-1"));

        let session = agent_terminal_session(&terminal);

        assert_eq!(session.terminal_id, "terminal-1");
        assert_eq!(session.state, AgentTerminalState::Running);
        assert_eq!(session.capability, AgentTerminalCapability::Native);
        assert!(session.retained);
    }

    #[test]
    fn runtime_session_id_event_persists_refreshes_retained_terminal_and_publishes_latest_id() {
        let terminals = Mutex::new(HashMap::from([(
            "session-1".to_string(),
            managed_terminal(Some("runtime-1")),
        )]));
        let persisted = Mutex::new(Vec::<(String, String)>::new());

        let event = record_runtime_session_id(
            &terminals,
            "terminal-1",
            "session-1",
            "runtime-2".to_string(),
            |session_id, runtime_session_id| {
                persisted
                    .lock()
                    .expect("persisted ids")
                    .push((session_id.to_string(), runtime_session_id.to_string()));
            },
        );

        assert_eq!(
            *persisted.lock().expect("persisted ids"),
            vec![("session-1".to_string(), "runtime-2".to_string())]
        );
        assert_eq!(
            terminals.lock().expect("terminals")["session-1"]
                .runtime_session_id
                .as_deref(),
            Some("runtime-2")
        );
        assert!(matches!(
            event,
            AgentTerminalEvent::RuntimeSessionId {
                terminal_id,
                session_id,
                runtime_session_id,
            } if terminal_id == "terminal-1"
                && session_id == "session-1"
                && runtime_session_id == "runtime-2"
        ));
    }

    #[test]
    fn split_multibyte_utf8_is_buffered_until_the_sequence_completes() {
        // "好" is E5 A5 BD; a read that ends after E5 A5 must not emit a replacement char.
        let bytes = "已好".as_bytes().to_vec();
        let split = bytes.len() - 1;
        let mut pending = bytes[..split].to_vec();

        let first = take_decodable_utf8(&mut pending);
        assert_eq!(first, "已");
        assert!(!first.contains('\u{FFFD}'));
        assert!(!pending.is_empty(), "incomplete tail is retained");

        pending.extend_from_slice(&bytes[split..]);
        let second = take_decodable_utf8(&mut pending);
        assert_eq!(second, "好");
        assert!(pending.is_empty());
    }

    #[test]
    fn complete_utf8_is_returned_whole_and_drains_pending() {
        let mut pending = "ready ✅".as_bytes().to_vec();

        assert_eq!(take_decodable_utf8(&mut pending), "ready ✅");
        assert!(pending.is_empty());
    }

    #[test]
    fn drain_yields_complete_lines_strips_crlf_and_retains_partial() {
        let mut line_buffer = String::from("one\r\ntwo\nthr");
        let mut lines: Vec<String> = Vec::new();

        drain_complete_lines(&mut line_buffer, |line| lines.push(line.to_string()));

        assert_eq!(lines, vec!["one".to_string(), "two".to_string()]);
        assert_eq!(line_buffer, "thr", "unterminated remainder is retained");
    }

    #[test]
    fn session_marker_split_across_reads_is_parsed_once_the_newline_arrives() {
        let parser = output_parser_for("codex-cli");
        let mut line_buffer = String::new();
        let mut session_ids: Vec<String> = Vec::new();

        // First read cuts the JSON marker in half — nothing parseable yet.
        line_buffer.push_str("{\"type\":\"session_init\",\"sessi");
        drain_complete_lines(&mut line_buffer, |line| {
            if let ProviderOutputEvent::SessionId(id) = parser.parse_line(line) {
                session_ids.push(id);
            }
        });
        assert!(session_ids.is_empty());

        // Second read completes the line; the marker is now recovered.
        line_buffer.push_str("on_id\":\"codex-session\"}\n");
        drain_complete_lines(&mut line_buffer, |line| {
            if let ProviderOutputEvent::SessionId(id) = parser.parse_line(line) {
                session_ids.push(id);
            }
        });
        assert_eq!(session_ids, vec!["codex-session".to_string()]);
        assert!(line_buffer.is_empty());
    }

    #[test]
    fn newline_less_output_is_bounded_and_never_yields_a_line() {
        let mut line_buffer = String::new();
        let mut lines = 0_usize;
        for _ in 0..8 {
            line_buffer.push_str(&"progress\r".repeat(MAX_PARSE_LINE_BYTES / 4));
            drain_complete_lines(&mut line_buffer, |_| lines += 1);
        }

        assert_eq!(lines, 0, "no newline means no complete line");
        assert!(
            line_buffer.len() <= MAX_PARSE_LINE_BYTES,
            "buffer stays bounded"
        );
    }

    #[test]
    fn terminal_transcript_retention_keeps_recent_utf8_content() {
        let mut transcript = String::new();
        append_terminal_transcript(&mut transcript, "older");
        append_terminal_transcript(
            &mut transcript,
            &"好".repeat(RETAINED_TERMINAL_TRANSCRIPT_BYTES / "好".len() + 2),
        );

        assert!(transcript.len() <= RETAINED_TERMINAL_TRANSCRIPT_BYTES);
        assert!(transcript.is_char_boundary(0));
        assert!(!transcript.contains("older"));
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
                    read_only: false,
                    loop_ownership: None,
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
