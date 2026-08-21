#[cfg(test)]
use super::providers::output_parser_for;
use super::providers::{
    output_parser_for_format, prepare_provider_session_capture, ProviderOutputEvent,
    ProviderOutputFramer, ProviderOutputStream, ProviderSessionCapture, ProviderSessionDiscovery,
};
use super::terminal_observability::{
    finish_terminal_execution_trace, start_terminal_execution_trace,
    TerminalExecutionObservability, TerminalExecutionTrace,
};
use super::terminal_usage_ledger::{
    ingest_claude_terminal_usage, ingest_codex_terminal_usage, ingest_gemini_terminal_usage,
    ingest_opencode_terminal_usage, record_unsupported_terminal_source,
};
use super::terminal_wrapper::{
    default_agent_terminal_shell, generate_agent_terminal_wrapper, AgentTerminalWrapperRequest,
};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentRuntimeApplicationError,
    AgentSessionGateway, AgentTerminalCapability, AgentTerminalEvent, AgentTerminalEventPort,
    AgentTerminalGateway, AgentTerminalInputRequest, AgentTerminalProcessRequest,
    AgentTerminalSession, AgentTerminalSize, AgentTerminalState,
    ProviderInteractiveInvocationRequest, ProviderRegistry, ResizeAgentTerminalRequest,
    StopAgentTerminalRequest,
};
use crate::contexts::agent_runtime::domain::{AgentLifecycle, ProviderCapability};
use crate::contexts::execution_observability::api::ExecutionStatus;
use crate::contexts::sessions::api::SessionsApi;
use crate::platform::filesystem::normalize_windows_extended_length_path;
use crate::platform::text::take_decodable_utf8;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const RETAINED_TERMINAL_TRANSCRIPT_BYTES: usize = 1024 * 1024;

/// Larger reads coalesce bursty PTY output into fewer IPC events without adding latency:
/// a read still returns as soon as any bytes are available, so interactive echo is
/// unaffected, while a flood of build output emits far fewer events than a 4 KiB buffer.
const TERMINAL_READ_BUFFER_BYTES: usize = 64 * 1024;

/// Upper bound on an unterminated parse line, so newline-less output (e.g. `\r` progress
/// bars) cannot grow the session-id parse buffer without bound.
#[cfg(test)]
const MAX_PARSE_LINE_BYTES: usize = 256 * 1024;
const PROVIDER_SESSION_DISCOVERY_INTERVAL: Duration = Duration::from_millis(250);

/// How often the interactive terminal re-reads the CLI's own usage data while the
/// session is still open. Wider than the session-discovery poll since it re-scans a
/// session log/DB rather than checking already-buffered output; a few seconds keeps
/// the Token Usage panel close to live without adding meaningful file/DB IO.
const TERMINAL_USAGE_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// The blocking halves of an Agent terminal, shared out of the registry so PTY writes and
/// resizes never run while the registry lock is held.
///
/// `write_all` on a PTY master blocks once the slave's input buffer fills, which is what a CLI
/// that stopped draining its stdin does. Performing that write under the registry lock would
/// stall input, resize, attach and idle cleanup for *every* other Agent terminal — and `stop()`
/// takes the same lock, so the wedged terminal could not even be cancelled. The workspace shell
/// runtime hit this first and solved it the same way (`workspaces::infrastructure::portable_pty`).
struct TerminalIo {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
}

struct ManagedAgentTerminal {
    terminal_id: String,
    session_id: String,
    agent_id: String,
    runtime_session_id: Option<String>,
    last_active_at: i64,
    size: AgentTerminalSize,
    io: Arc<TerminalIo>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    transcript: BoundedTextBuffer,
}

struct BoundedTextBuffer {
    chunks: VecDeque<String>,
    bytes: usize,
    max_bytes: usize,
}

impl BoundedTextBuffer {
    fn new(max_bytes: usize) -> Self {
        Self {
            chunks: VecDeque::new(),
            bytes: 0,
            max_bytes,
        }
    }

    fn append(&mut self, content: &str) {
        if content.is_empty() || self.max_bytes == 0 {
            return;
        }
        self.chunks.push_back(content.to_string());
        self.bytes += content.len();
        while self.bytes > self.max_bytes {
            let excess = self.bytes - self.max_bytes;
            let Some(front) = self.chunks.front_mut() else {
                self.bytes = 0;
                return;
            };
            if front.len() <= excess {
                self.bytes -= front.len();
                self.chunks.pop_front();
                continue;
            }
            let mut trim_to = excess;
            while !front.is_char_boundary(trim_to) {
                trim_to += 1;
            }
            front.drain(..trim_to);
            self.bytes -= trim_to;
        }
    }

    fn snapshot(&self) -> String {
        let mut snapshot = String::with_capacity(self.bytes);
        for chunk in &self.chunks {
            snapshot.push_str(chunk);
        }
        snapshot
    }
}

#[derive(Clone)]
pub(crate) struct PortablePtyAgentTerminalRuntime {
    terminals: Arc<Mutex<HashMap<String, ManagedAgentTerminal>>>,
    terminal_ids: Arc<AtomicU64>,
    events: Arc<dyn AgentTerminalEventPort>,
    sessions: Arc<dyn AgentSessionGateway>,
    accounting: SessionsApi,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    observability: TerminalExecutionObservability,
    wrapper_dir: PathBuf,
    providers: Arc<ProviderRegistry>,
}

impl PortablePtyAgentTerminalRuntime {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        events: Arc<dyn AgentTerminalEventPort>,
        sessions: Arc<dyn AgentSessionGateway>,
        accounting: SessionsApi,
        logging: Arc<dyn AgentLoggingPort>,
        clock: Arc<dyn AgentClockPort>,
        observability: TerminalExecutionObservability,
        wrapper_dir: PathBuf,
        providers: Arc<ProviderRegistry>,
    ) -> Self {
        Self {
            terminals: Arc::new(Mutex::new(HashMap::new())),
            terminal_ids: Arc::new(AtomicU64::new(0)),
            events,
            sessions,
            accounting,
            logging,
            clock,
            observability,
            wrapper_dir,
            providers,
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

    fn checkout_io(
        &self,
        terminal_id: &str,
    ) -> Result<Arc<TerminalIo>, AgentRuntimeApplicationError> {
        let now = now_timestamp(self.clock.as_ref());
        checkout_terminal_io(self.terminals.as_ref(), terminal_id, now)
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

    fn start_terminal_execution(
        &self,
        session_id: &str,
        agent_id: &str,
        provider_session_id: Option<&str>,
    ) -> TerminalExecutionTrace {
        let settings = match self.observability.execution_settings.load_settings() {
            Ok(settings) => settings,
            Err(error) => {
                self.record_log(
                    AgentLogLevel::Warn,
                    format!("Execution observability settings are unavailable: {error}"),
                    Some(agent_id),
                    Some(session_id),
                );
                Default::default()
            }
        };
        start_terminal_execution_trace(
            self.observability.execution_ids.as_ref(),
            self.observability.telemetry.as_ref(),
            self.clock.as_ref(),
            &settings,
            session_id,
            agent_id,
            provider_session_id,
        )
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
            let transcript = terminal.transcript.snapshot();
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
            let transcript = terminal.transcript.snapshot();
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
        let provider = self
            .providers
            .require(&request.agent.id, ProviderCapability::Terminal)?;
        if request.session.runtime_session_id.is_some() {
            self.providers
                .require(&request.agent.id, ProviderCapability::Resume)?;
        }
        let provider_session = self.providers.resolve_session(
            &request.agent.id,
            request.session.runtime_session_id.as_deref(),
        )?;
        let output_format = provider.output_format();
        let provider_session_capture = if non_empty_runtime_session_id(
            request.session.runtime_session_id.as_deref(),
        )
        .is_none()
            && matches!(request.agent.id.as_str(), "codex-cli" | "opencode")
        {
            let working_directory = request
                .session
                .folder
                .as_deref()
                .filter(|folder| !folder.trim().is_empty())
                .map(PathBuf::from)
                .or_else(|| std::env::current_dir().ok());
            match working_directory {
                Some(working_directory) => {
                    match prepare_provider_session_capture(&request.agent.id, working_directory) {
                        Ok(capture) => capture,
                        Err(error) => {
                            self.record_log(
                                AgentLogLevel::Warn,
                                format!("Provider session capture is unavailable: {error}"),
                                Some(&request.agent.id),
                                Some(&request.session.id),
                            );
                            None
                        }
                    }
                }
                None => {
                    self.record_log(
                        AgentLogLevel::Warn,
                        "Provider session capture is unavailable because the working directory could not be resolved."
                            .to_string(),
                        Some(&request.agent.id),
                        Some(&request.session.id),
                    );
                    None
                }
            }
        } else {
            None
        };
        let executable =
            normalize_interactive_executable(&request.agent.id, &request.cli_profile.executable);
        let invocation = provider
            .prepare_interactive(ProviderInteractiveInvocationRequest {
                executable,
                provider_session: provider_session.as_ref(),
                managed_args: &request.cli_profile.managed_args,
            })
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
        // `Set-Location`/`cd /d` and `CreateProcess`'s `lpCurrentDirectory` all reject or
        // mishandle a Windows extended-length path prefix (`\\?\`) — normalize once and
        // reuse for both the wrapper script's own directory change and the outer PTY
        // spawn's cwd below, so the CLI actually starts in the intended project folder.
        let normalized_folder = request
            .session
            .folder
            .as_deref()
            .filter(|folder| !folder.trim().is_empty())
            .map(normalize_windows_extended_length_path);
        let (shell, shell_executable) = default_agent_terminal_shell();
        let wrapper = generate_agent_terminal_wrapper(&AgentTerminalWrapperRequest {
            terminal_id: terminal_id.clone(),
            session_folder: normalized_folder.as_deref().map(PathBuf::from),
            executable: invocation.executable.clone(),
            args: invocation.args.clone(),
            shell,
            shell_executable,
            wrapper_dir: self.wrapper_dir.clone(),
            env: request.cli_profile.env.clone(),
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
        if let Some(folder) = normalized_folder.as_deref() {
            command.cwd(folder);
        }
        let mut child = pair.slave.spawn_command(command).map_err(|error| {
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
        let mut reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(error) => {
                cleanup_unattached_terminal_child(child.as_mut());
                let message = format!("Failed to attach Agent terminal reader: {error}");
                self.record_log(
                    AgentLogLevel::Error,
                    message.clone(),
                    Some(&request.agent.id),
                    Some(&request.session.id),
                );
                return Err(AgentRuntimeApplicationError::Process(message));
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(error) => {
                cleanup_unattached_terminal_child(child.as_mut());
                let message = format!("Failed to attach Agent terminal writer: {error}");
                self.record_log(
                    AgentLogLevel::Error,
                    message.clone(),
                    Some(&request.agent.id),
                    Some(&request.session.id),
                );
                return Err(AgentRuntimeApplicationError::Process(message));
            }
        };
        let child = Arc::new(Mutex::new(child));
        let runtime_session_id =
            non_empty_runtime_session_id(request.session.runtime_session_id.as_deref())
                .map(str::to_string)
                .or_else(|| invocation.assigned_runtime_session_id.clone());
        let terminal_execution = self.start_terminal_execution(
            &request.session.id,
            &request.agent.id,
            runtime_session_id.as_deref(),
        );
        let terminal = ManagedAgentTerminal {
            terminal_id: terminal_id.clone(),
            session_id: request.session.id.clone(),
            agent_id: request.agent.id.clone(),
            runtime_session_id: runtime_session_id.clone(),
            last_active_at: now_timestamp(self.clock.as_ref()),
            size: request.size.clone(),
            io: Arc::new(TerminalIo {
                master: Mutex::new(pair.master),
                writer: Mutex::new(writer),
            }),
            child: child.clone(),
            transcript: BoundedTextBuffer::new(RETAINED_TERMINAL_TRANSCRIPT_BYTES),
        };
        let response = agent_terminal_session(&terminal);
        terminal_registry.insert(request.session.id.clone(), terminal);

        let events = self.events.clone();
        let sessions = self.sessions.clone();
        let accounting = self.accounting.clone();
        let logging = self.logging.clone();
        let clock = self.clock.clone();
        let telemetry = self.observability.telemetry.clone();
        let terminals = self.terminals.clone();
        let session_id = request.session.id;
        let agent_id = request.agent.id;
        let session_folder = request
            .session
            .folder
            .as_deref()
            .filter(|f| !f.trim().is_empty())
            .map(str::to_string);
        let started_at_ms = now_timestamp_ms(self.clock.as_ref());
        let terminal_started_at = self.clock.now();
        let usage_tracking_enabled = matches!(
            agent_id.as_str(),
            "claude-code" | "opencode" | "codex-cli" | "gemini-cli" | "antigravity-cli"
        );
        // A CLI can go quiet on the PTY (idle, waiting for the next prompt) for several
        // seconds *after* it has finished streaming visible output but *before* it has
        // actually persisted that turn's usage to its own session log/DB — observed
        // directly: opencode's `session` row appeared several seconds before its token
        // columns were actually updated. Polling only when output happens to arrive would
        // miss that gap entirely and could never catch up until the next unrelated burst
        // of output (or exit). This timer thread is intentionally independent of PTY
        // activity so it keeps checking on a fixed cadence regardless.
        let usage_poll_alive = Arc::new(AtomicBool::new(true));
        let usage_poll_handle = if usage_tracking_enabled {
            let agent_id = agent_id.clone();
            let session_folder = session_folder.clone();
            let runtime_session_id = runtime_session_id.clone();
            let accounting = accounting.clone();
            let logging = logging.clone();
            let clock = clock.clone();
            let session_id = session_id.clone();
            let terminal_started_at = terminal_started_at.clone();
            let alive = usage_poll_alive.clone();
            Some(thread::spawn(move || {
                // Ticks in short increments so the thread notices the terminal has
                // exited promptly instead of sleeping a full interval before checking.
                const TICK: Duration = Duration::from_millis(250);
                let mut waited = Duration::ZERO;
                while alive.load(Ordering::Acquire) {
                    thread::sleep(TICK);
                    waited += TICK;
                    if waited < TERMINAL_USAGE_POLL_INTERVAL {
                        continue;
                    }
                    waited = Duration::ZERO;
                    if !alive.load(Ordering::Acquire) {
                        break;
                    }
                    let result = run_terminal_usage_ingestion(
                        &agent_id,
                        session_folder.as_deref(),
                        runtime_session_id.as_deref(),
                        started_at_ms,
                        &accounting,
                        &session_id,
                        &terminal_started_at,
                        clock.as_ref(),
                    );
                    if let Some(result) = result {
                        record_terminal_usage_log(
                            logging.as_ref(),
                            clock.as_ref(),
                            &agent_id,
                            &session_id,
                            "periodic poll",
                            &result,
                        );
                    }
                }
            }))
        } else {
            None
        };
        drop(terminal_registry);
        thread::spawn(move || {
            let parser = output_parser_for_format(output_format);
            let mut provider_session_capture = provider_session_capture;
            let mut last_capture_attempt: Option<Instant> = None;
            let mut capture_failure_logged = false;
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
            let mut output_framer = ProviderOutputFramer::new(262_144);
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        let (content, framed_lines) =
                            split_terminal_read(&buffer[..count], &mut pending, &mut output_framer);
                        if !content.is_empty() {
                            let _ = events.publish_terminal(AgentTerminalEvent::Output {
                                terminal_id: terminal_id.clone(),
                                session_id: session_id.clone(),
                                content: content.clone(),
                            });
                            if let Ok(mut terminals) = terminals.lock() {
                                if let Some(terminal) = terminals.get_mut(&session_id) {
                                    terminal.last_active_at = now_timestamp(clock.as_ref());
                                    terminal.transcript.append(&content);
                                }
                            }
                        }
                        for line in framed_lines {
                            if let ProviderOutputEvent::SessionId(runtime_session_id) =
                                parser.parse_line(&line)
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
                                provider_session_capture = None;
                            }
                        }
                        if provider_session_capture.is_some()
                            && last_capture_attempt.is_none_or(|attempt| {
                                attempt.elapsed() >= PROVIDER_SESSION_DISCOVERY_INTERVAL
                            })
                        {
                            last_capture_attempt = Some(Instant::now());
                            let discovery = provider_session_capture
                                .as_ref()
                                .map(ProviderSessionCapture::discover);
                            match discovery {
                                Some(Ok(ProviderSessionDiscovery::Found(runtime_session_id))) => {
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
                                    record_provider_session_capture_log(
                                        logging.as_ref(),
                                        clock.as_ref(),
                                        AgentLogLevel::Info,
                                        "Captured exact provider session id.".to_string(),
                                        &agent_id,
                                        &session_id,
                                    );
                                    provider_session_capture = None;
                                }
                                Some(Ok(ProviderSessionDiscovery::Ambiguous(count))) => {
                                    record_provider_session_capture_log(
                                        logging.as_ref(),
                                        clock.as_ref(),
                                        AgentLogLevel::Warn,
                                        format!(
                                            "Provider session capture skipped because {count} matching new sessions were found."
                                        ),
                                        &agent_id,
                                        &session_id,
                                    );
                                    provider_session_capture = None;
                                }
                                Some(Err(error)) if !capture_failure_logged => {
                                    record_provider_session_capture_log(
                                        logging.as_ref(),
                                        clock.as_ref(),
                                        AgentLogLevel::Warn,
                                        format!("Provider session capture failed: {error}"),
                                        &agent_id,
                                        &session_id,
                                    );
                                    capture_failure_logged = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(_) => break,
                }
            }

            // Stop the independent usage-poll thread now that the terminal has
            // definitely exited, so it doesn't race the exit-time read below.
            if stop_terminal_usage_poll(usage_poll_alive.as_ref(), usage_poll_handle) {
                record_provider_session_capture_log(
                    logging.as_ref(),
                    clock.as_ref(),
                    AgentLogLevel::Warn,
                    "Terminal usage polling thread panicked before shutdown.".to_string(),
                    &agent_id,
                    &session_id,
                );
            }
            // Reap the PTY child without holding the `child` lock across the blocking
            // wait: `stop()` -> `terminate_terminal_child` locks the same
            // `Arc<Mutex<…Child…>>` to kill a runaway terminal, so holding the lock
            // across `wait()` deadlocks cancellation whenever the PTY master hits EOF
            // (the reader breaks) while the child process itself is still alive.
            // Polling `try_wait()` with short lock holds lets a concurrent `stop()`
            // `kill()` proceed.
            let exit_result = reap_terminal_without_holding_lock(&child);
            let (state, error) = match exit_result {
                Ok(status) if status.success() => (AgentTerminalState::Stopped, None),
                Ok(status) => (
                    AgentTerminalState::Failed,
                    Some(format!("Agent terminal exited with status {status}.")),
                ),
                Err(error) => (AgentTerminalState::Failed, Some(error)),
            };
            // Read back the CLI's own persisted usage data regardless of exit status:
            // a clean exit (Stopped) is the normal case, but even a killed process
            // (Failed) has already flushed its usage data during the interactive turns.
            //
            // claude-code needs the runtime session id VaneHub assigned (it's the JSONL
            // file name); opencode and codex-cli look themselves up by working directory
            // + start time instead of depending on the live `ProviderSessionCapture` poll
            // — that poll only samples while fresh PTY output is arriving, so it can miss
            // a session whose own log/DB row appears after the last visible output but
            // before Stop.
            if usage_tracking_enabled {
                let result = run_terminal_usage_ingestion(
                    &agent_id,
                    session_folder.as_deref(),
                    runtime_session_id.as_deref(),
                    started_at_ms,
                    &accounting,
                    &session_id,
                    &terminal_started_at,
                    clock.as_ref(),
                );
                if let Some(result) = result {
                    record_terminal_usage_log(
                        logging.as_ref(),
                        clock.as_ref(),
                        &agent_id,
                        &session_id,
                        "session exit",
                        &result,
                    );
                }
            }
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
            let execution_status = match state {
                AgentTerminalState::Failed => ExecutionStatus::Failed,
                _ => ExecutionStatus::Succeeded,
            };
            let error_classification = error.as_ref().map(|_| "agent_terminal_process_failed");
            finish_terminal_execution_trace(
                telemetry.as_ref(),
                clock.as_ref(),
                &terminal_execution,
                execution_status,
                error_classification,
            );
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
                run_id: Some(terminal_execution.session.run_id.as_str().to_string()),
                trace_id: Some(terminal_execution.session.trace_id.as_str().to_string()),
                span_id: Some(terminal_execution.process.span_id.as_str().to_string()),
                occurred_at: clock.now(),
            });
        });

        Ok(response)
    }

    fn input(
        &self,
        request: AgentTerminalInputRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let io = self.checkout_io(&request.terminal_id)?;
        let mut writer = io
            .writer
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        writer
            .write_all(request.content.as_bytes())
            .and_then(|_| writer.flush())
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))
    }

    fn resize(
        &self,
        request: ResizeAgentTerminalRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let io = self.checkout_io(&request.terminal_id)?;
        io.master
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?
            .resize(terminal_size(&request.size))
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        // Recorded only once the PTY accepted it, so a rejected resize does not leave the
        // registry reporting a size the terminal never took.
        if let Ok(mut terminals) = self.lock_terminals() {
            if let Ok(terminal) = terminal_by_id_mut(&mut terminals, &request.terminal_id) {
                terminal.size = request.size;
            }
        }
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
        self.providers
            .require(&terminal.agent_id, ProviderCapability::Cancellation)?;
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

/// Dispatches to the CLI-specific usage reader for `agent_id`, or `None` for CLIs that
/// don't support terminal-mode usage tracking (or when claude-code's runtime session id
/// isn't known yet). Shared by both the periodic in-session poll and the final
/// exit-time read so the two call sites can't drift.
#[allow(clippy::too_many_arguments)]
fn run_terminal_usage_ingestion(
    agent_id: &str,
    session_folder: Option<&str>,
    runtime_session_id: Option<&str>,
    started_at_ms: i64,
    accounting: &SessionsApi,
    session_id: &str,
    terminal_started_at: &str,
    clock: &dyn AgentClockPort,
) -> Option<Result<bool, AgentRuntimeApplicationError>> {
    match agent_id {
        "claude-code" => runtime_session_id.map(|runtime_session_id| {
            ingest_claude_terminal_usage(
                session_folder,
                runtime_session_id,
                accounting,
                session_id,
                agent_id,
                terminal_started_at,
            )
        }),
        "opencode" => Some(ingest_opencode_terminal_usage(
            session_folder,
            started_at_ms,
            accounting,
            session_id,
            agent_id,
            terminal_started_at,
            clock,
        )),
        "codex-cli" => Some(ingest_codex_terminal_usage(
            session_folder,
            started_at_ms,
            accounting,
            session_id,
            agent_id,
            terminal_started_at,
            clock,
        )),
        "gemini-cli" => runtime_session_id.map(|runtime_session_id| {
            ingest_gemini_terminal_usage(
                session_folder,
                runtime_session_id,
                accounting,
                session_id,
                agent_id,
                terminal_started_at,
            )
        }),
        "antigravity-cli" => Some(record_unsupported_terminal_source(
            accounting,
            session_id,
            agent_id,
            terminal_started_at,
        )),
        _ => None,
    }
}

/// `context` distinguishes the frequent periodic poll (kept at Debug so a long session
/// doesn't spam Info) from the one-time exit read (kept at Info as the meaningful
/// "did the panel end up with real data" summary). Both log Warn on error.
fn record_terminal_usage_log(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    agent_id: &str,
    session_id: &str,
    context: &str,
    result: &Result<bool, AgentRuntimeApplicationError>,
) {
    let level = match result {
        Ok(true) if context == "session exit" => AgentLogLevel::Info,
        Ok(_) => AgentLogLevel::Debug,
        Err(_) => AgentLogLevel::Warn,
    };
    let _ = logging.record(AgentLog {
        level,
        category: "session.agent_terminal.usage".to_string(),
        message: format!(
            "terminal usage ingestion ({context}): persisted={} err={:?}",
            result.as_ref().unwrap_or(&false),
            result.as_ref().err(),
        ),
        agent_id: Some(agent_id.to_string()),
        session_id: Some(session_id.to_string()),
        operation_id: None,
        run_id: None,
        trace_id: None,
        span_id: None,
        occurred_at: clock.now(),
    });
}

fn record_provider_session_capture_log(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    level: AgentLogLevel,
    message: String,
    agent_id: &str,
    session_id: &str,
) {
    let _ = logging.record(AgentLog {
        level,
        category: "session.agent_terminal".to_string(),
        message,
        agent_id: Some(agent_id.to_string()),
        session_id: Some(session_id.to_string()),
        operation_id: None,
        run_id: None,
        trace_id: None,
        span_id: None,
        occurred_at: clock.now(),
    });
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

/// Resolves a terminal to its shared PTY handles and marks it active, releasing the registry
/// lock before the caller performs any blocking PTY operation. See `TerminalIo`.
fn checkout_terminal_io(
    terminals: &Mutex<HashMap<String, ManagedAgentTerminal>>,
    terminal_id: &str,
    now: i64,
) -> Result<Arc<TerminalIo>, AgentRuntimeApplicationError> {
    let mut terminals = terminals
        .lock()
        .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
    let terminal = terminal_by_id_mut(&mut terminals, terminal_id)?;
    terminal.last_active_at = now;
    Ok(terminal.io.clone())
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

/// Reaps a PTY child without holding the lock across the blocking wait.
///
/// Both the reader thread (on PTY EOF) and `terminate_terminal_child` (on `stop()`)
/// reach this on the same `Arc<Mutex<…Child…>>`. Holding the lock across `wait()`
/// — which blocks until the child exits — would serialize them and deadlock the
/// second caller whenever the child stays alive after the PTY/kill. Polling
/// `try_wait()` with short lock holds lets both callers make progress and lets a
/// concurrent `kill()` take effect.
fn reap_terminal_without_holding_lock(
    child: &Mutex<Box<dyn Child + Send + Sync>>,
) -> Result<portable_pty::ExitStatus, String> {
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

fn terminate_terminal_child(
    child: &Mutex<Box<dyn Child + Send + Sync>>,
) -> Result<(), AgentRuntimeApplicationError> {
    // Kill inside the lock, then release it before reaping. `wait()` blocks until the
    // child actually exits, and the reader thread reaches `reap_terminal_without_holding_lock`
    // on the same `Arc<Mutex<…>>` when the PTY master hits EOF — holding the lock across
    // `wait()` here would deadlock that reaper (and any concurrent `stop()`).
    {
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
    }
    let _ = reap_terminal_without_holding_lock(child);
    Ok(())
}

fn cleanup_unattached_terminal_child(child: &mut dyn Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn stop_terminal_usage_poll(alive: &AtomicBool, handle: Option<thread::JoinHandle<()>>) -> bool {
    alive.store(false, Ordering::Release);
    handle.map(|handle| handle.join().is_err()).unwrap_or(false)
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
/// Invokes `on_line` for each complete `\n`-terminated line in `line_buffer` (trailing
/// CR/LF stripped), leaving any unterminated remainder for the next read to finish.
/// An oversized newline-less remainder is discarded to keep the buffer bounded.
#[cfg(test)]
fn drain_complete_lines(line_buffer: &mut String, mut on_line: impl FnMut(&str)) {
    while let Some(newline) = line_buffer.find('\n') {
        let line: String = line_buffer.drain(..=newline).collect();
        on_line(line.trim_end_matches(['\n', '\r']));
    }
    if line_buffer.len() > MAX_PARSE_LINE_BYTES {
        line_buffer.clear();
    }
}

/// Splits one PTY read into the text the terminal view shows and the `\n`-terminated records
/// the provider parser consumes.
///
/// The two consumers need different framing but must both see every byte: `pending` carries an
/// incomplete UTF-8 tail forward so a character split across reads is never rendered as U+FFFD,
/// while the framer works on raw bytes and assembles them into lines. A read that decodes to
/// nothing displayable — the leading bytes of a multi-byte character arriving on their own — is
/// exactly the case where those two diverge, and it is still part of whatever line the parser is
/// assembling.
fn split_terminal_read(
    chunk: &[u8],
    pending: &mut Vec<u8>,
    framer: &mut ProviderOutputFramer,
) -> (String, Vec<String>) {
    pending.extend_from_slice(chunk);
    let content = take_decodable_utf8(pending);
    let lines = framer
        .push(ProviderOutputStream::Stdout, chunk)
        .unwrap_or_default();
    (content, lines)
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

fn now_timestamp_ms(clock: &dyn AgentClockPort) -> i64 {
    chrono::DateTime::parse_from_rfc3339(&clock.now())
        .map(|value| value.timestamp_millis())
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

fn non_empty_runtime_session_id(runtime_session_id: Option<&str>) -> Option<&str> {
    runtime_session_id.filter(|session_id| !session_id.trim().is_empty())
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
        managed_terminal_named("terminal-1", "session-1", runtime_session_id)
    }

    fn managed_terminal_named(
        terminal_id: &str,
        session_id: &str,
        runtime_session_id: Option<&str>,
    ) -> ManagedAgentTerminal {
        ManagedAgentTerminal {
            terminal_id: terminal_id.to_string(),
            session_id: session_id.to_string(),
            agent_id: "codex-cli".to_string(),
            runtime_session_id: runtime_session_id.map(str::to_string),
            last_active_at: 1,
            size: AgentTerminalSize { rows: 24, cols: 80 },
            io: Arc::new(TerminalIo {
                master: Mutex::new(
                    native_pty_system()
                        .openpty(terminal_size(&AgentTerminalSize { rows: 1, cols: 1 }))
                        .expect("pty")
                        .master,
                ),
                writer: Mutex::new(Box::new(Vec::<u8>::new())),
            }),
            child: Arc::new(Mutex::new(dummy_child())),
            transcript: BoundedTextBuffer::new(RETAINED_TERMINAL_TRANSCRIPT_BYTES),
        }
    }

    /// The Agent-terminal counterpart of the workspace shell runtime's
    /// `a_blocked_shell_writer_does_not_stall_other_shells`. A CLI that stopped draining its
    /// stdin blocks `write_all` indefinitely; if that write ran under the registry lock it would
    /// freeze every other terminal's input, resize and attach — and `stop()`, which takes the
    /// same lock, could not cancel the wedged one.
    #[test]
    fn a_blocked_terminal_writer_does_not_hold_the_registry_lock() {
        let mut registry = HashMap::new();
        registry.insert(
            "session-1".to_string(),
            managed_terminal_named("terminal-1", "session-1", None),
        );
        registry.insert(
            "session-2".to_string(),
            managed_terminal_named("terminal-2", "session-2", None),
        );
        let terminals = Arc::new(Mutex::new(registry));

        let blocked = checkout_terminal_io(terminals.as_ref(), "terminal-1", 10)
            .expect("first terminal checks out");
        // Stands in for a child that stopped reading: the writer stays held for the whole test.
        let _held = blocked.writer.lock().expect("hold the first writer");

        let other = checkout_terminal_io(terminals.as_ref(), "terminal-2", 20)
            .expect("second terminal checks out while the first writer is blocked");
        other
            .writer
            .lock()
            .expect("second writer is independent")
            .write_all(b"echo test\n")
            .expect("second terminal accepts input");
        other
            .master
            .lock()
            .expect("second master is independent")
            .resize(terminal_size(&AgentTerminalSize {
                rows: 30,
                cols: 100,
            }))
            .expect("second terminal resizes");

        // What `stop()` needs: the registry itself is never held across a blocking PTY write.
        let mut guard = terminals.lock().expect("registry is still lockable");
        assert!(
            guard.remove("session-1").is_some(),
            "the wedged terminal can still be cancelled"
        );
        assert_eq!(
            guard.get("session-2").expect("survivor").last_active_at,
            20,
            "checkout marks the terminal active"
        );
    }

    #[test]
    fn terminal_size_bounds_rows_and_cols_for_pty() {
        let size = terminal_size(&AgentTerminalSize { rows: 0, cols: 900 });

        assert_eq!(size.rows, 1);
        assert_eq!(size.cols, 500);
    }

    #[test]
    fn blank_runtime_session_id_is_treated_as_missing() {
        assert_eq!(non_empty_runtime_session_id(None), None);
        assert_eq!(non_empty_runtime_session_id(Some(" \t")), None);
        assert_eq!(
            non_empty_runtime_session_id(Some("provider-session-1")),
            Some("provider-session-1")
        );
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

    /// Unlike `session_marker_split_across_reads_is_parsed_once_the_newline_arrives`, which
    /// drives the test-only `drain_complete_lines` helper, this exercises the framing the reader
    /// thread actually runs: `split_terminal_read` feeding `ProviderOutputFramer`.
    #[test]
    fn a_read_that_decodes_to_no_text_still_reaches_the_provider_framer() {
        let parser = output_parser_for("claude-code");
        let mut framer = ProviderOutputFramer::new(4096);
        let mut pending: Vec<u8> = Vec::new();
        // A real `claude-code` init line carries the working directory, so a project path with
        // non-ASCII characters puts a multi-byte sequence inside the marker line itself.
        let marker =
            "{\"type\":\"system\",\"session_id\":\"claude-session\",\"cwd\":\"D:/项目\"}\n";
        let bytes = marker.as_bytes();
        let multibyte_at = marker
            .find('项')
            .expect("marker contains a multi-byte character");

        // Three reads, the middle one being just the first byte of '项' — a PTY hands back
        // whatever is available, so a single leading byte is a read the reader thread must
        // survive without losing it.
        let reads: [&[u8]; 3] = [
            &bytes[..multibyte_at],
            &bytes[multibyte_at..multibyte_at + 1],
            &bytes[multibyte_at + 1..],
        ];
        let mut session_ids: Vec<String> = Vec::new();
        let mut rendered = String::new();
        for read in reads {
            let (content, lines) = split_terminal_read(read, &mut pending, &mut framer);
            rendered.push_str(&content);
            for line in lines {
                if let ProviderOutputEvent::SessionId(id) = parser.parse_line(&line) {
                    session_ids.push(id);
                }
            }
        }

        assert_eq!(
            session_ids,
            vec!["claude-session".to_string()],
            "the session marker must survive a read that decodes to no displayable text"
        );
        assert_eq!(
            rendered, marker,
            "no byte may be dropped from the rendered text"
        );
        assert!(
            pending.is_empty(),
            "nothing is left buffered once the line completes"
        );
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
        let mut transcript = BoundedTextBuffer::new(RETAINED_TERMINAL_TRANSCRIPT_BYTES);
        transcript.append("older");
        transcript.append(&"好".repeat(RETAINED_TERMINAL_TRANSCRIPT_BYTES / "好".len() + 2));

        let snapshot = transcript.snapshot();
        assert!(transcript.bytes <= RETAINED_TERMINAL_TRANSCRIPT_BYTES);
        assert_eq!(snapshot.len(), transcript.bytes);
        assert!(!snapshot.contains("older"));
    }

    #[test]
    fn terminal_transcript_appends_independent_chunks_below_the_limit() {
        let mut transcript = BoundedTextBuffer::new(64);
        transcript.append("first");
        transcript.append("second");

        assert_eq!(transcript.chunks.len(), 2);
        assert_eq!(transcript.snapshot(), "firstsecond");
    }

    #[test]
    fn stopping_usage_poll_joins_before_returning() {
        let alive = Arc::new(AtomicBool::new(true));
        let finished = Arc::new(AtomicBool::new(false));
        let worker_alive = alive.clone();
        let worker_finished = finished.clone();
        let handle = thread::spawn(move || {
            while worker_alive.load(Ordering::Acquire) {
                thread::yield_now();
            }
            worker_finished.store(true, Ordering::Release);
        });

        assert!(!stop_terminal_usage_poll(alive.as_ref(), Some(handle)));
        assert!(finished.load(Ordering::Acquire));
    }

    #[test]
    fn unattached_child_cleanup_waits_for_process_exit() {
        let mut child = dummy_child();

        cleanup_unattached_terminal_child(child.as_mut());

        assert!(child.try_wait().expect("child status").is_some());
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
                    seats: Vec::new(),
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
                    origin: crate::contexts::agent_runtime::domain::AgentOrigin::Builtin,
                },
                cli_profile: CliProfileSnapshot {
                    executable: "codex".to_string(),
                    selections: BTreeMap::from([("model".to_string(), json!("gpt-5"))]),
                    managed_args: vec!["--model".to_string(), "gpt-5".to_string()],
                    env: BTreeMap::new(),
                },
                size: AgentTerminalSize { rows: 24, cols: 80 },
            },
        )
    }
}
