use crate::contexts::workspaces::application::{
    ShellEvent, ShellLaunch, ShellLog, WorkspaceApplicationError as AppError, WorkspaceLogLevel,
    WorkspaceShellEventPort, WorkspaceShellLogPort, WorkspaceShellRuntimePort,
};
use crate::contexts::workspaces::domain::{reset_directory_command, ShellHost, TerminalDimensions};
use crate::platform::filesystem::normalize_windows_extended_length_path;
use crate::platform::text::take_decodable_utf8;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// The blocking halves of a shell, shared out of the registry so PTY writes and resizes
/// never run while the registry lock is held. A shell whose child stopped draining its
/// pipe would otherwise stall input, resize and shutdown for *every* other shell.
struct ShellIo {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
}

struct ManagedShell {
    session_id: String,
    root: PathBuf,
    io: Arc<ShellIo>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    killer: Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>,
    termination: Arc<ShellTermination>,
}

/// How long a kill is given to become an *observed* exit. Past this the runtime reports
/// `reap_timed_out` and says so, rather than returning as though the child were gone.
const REAP_DEADLINE: Duration = Duration::from_secs(5);

/// Poll backoff floor and ceilings. Not a fixed sleep: every iteration reads the child's real
/// state through the non-blocking `try_wait`, and the loop ends on a real answer or a real
/// deadline. The interval only decides how often the truth is sampled. Starting at 1 ms means
/// the overwhelmingly common case -- a child that exits at once -- is observed in about a
/// millisecond, while a wedged one costs a few syscalls a second instead of a spin.
const POLL_INTERVAL_FLOOR: Duration = Duration::from_millis(1);
const POLL_INTERVAL_CEILING: Duration = Duration::from_millis(25);
/// The exit monitor waits for a *natural* exit, which is legitimately unbounded -- a user may
/// keep a shell open for hours. It settles at a slower cadence because a quarter second of
/// latency on a "disconnected" event is imperceptible, and unlike the previous implementation
/// it never holds the child lock while waiting.
const MONITOR_INTERVAL_CEILING: Duration = Duration::from_millis(250);

/// Every way terminating a managed shell can end. Closed on purpose: a timeout is a distinct
/// answer from a refused signal and from a failed probe, because each calls for a different
/// response, and none of them may be reported as a completed termination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellTerminationOutcome {
    /// The child was already gone before any signal was sent.
    AlreadyExited,
    /// The signal was delivered but the exit has not been observed yet.
    KillRequested,
    /// Another caller already owns the reap for this child.
    Reaping,
    /// The exit was observed within the deadline.
    Reaped,
    /// The deadline passed with the child still alive. A live process is still out there.
    ReapTimedOut,
    /// The signal itself was refused, and the child had not exited.
    KillFailed,
    /// Probing the child returned an error rather than a status.
    ReapFailed,
}

impl ShellTerminationOutcome {
    /// Stable codes. These reach diagnostics, so they are a vocabulary rather than prose.
    fn code(self) -> &'static str {
        match self {
            Self::AlreadyExited => "already_exited",
            Self::KillRequested => "kill_requested",
            Self::Reaping => "reaping",
            Self::Reaped => "reaped",
            Self::ReapTimedOut => "reap_timed_out",
            Self::KillFailed => "kill_failed",
            Self::ReapFailed => "reap_failed",
        }
    }

    /// True only when the child is known to be gone. `ReapTimedOut` is deliberately not here:
    /// that is the whole point of naming it separately.
    fn is_settled_exit(self) -> bool {
        matches!(self, Self::AlreadyExited | Self::Reaped)
    }

    fn as_state(self) -> u8 {
        SETTLED_BASE
            + match self {
                Self::AlreadyExited => 0,
                Self::KillRequested => 1,
                Self::Reaping => 2,
                Self::Reaped => 3,
                Self::ReapTimedOut => 4,
                Self::KillFailed => 5,
                Self::ReapFailed => 6,
            }
    }

    fn from_state(state: u8) -> Option<Self> {
        match state.checked_sub(SETTLED_BASE)? {
            0 => Some(Self::AlreadyExited),
            1 => Some(Self::KillRequested),
            2 => Some(Self::Reaping),
            3 => Some(Self::Reaped),
            4 => Some(Self::ReapTimedOut),
            5 => Some(Self::KillFailed),
            6 => Some(Self::ReapFailed),
            _ => None,
        }
    }
}

const STATE_IDLE: u8 = 0;
const STATE_IN_FLIGHT: u8 = 1;
/// Settled states carry the outcome itself, offset past the two live states so the two ranges
/// can never collide. Storing the outcome rather than a bare "done" flag is what lets a repeated
/// stop answer truthfully -- a shell that timed out reports `reap_timed_out` again, instead of
/// the second call inventing a success the first one never had.
const SETTLED_BASE: u8 = 2;

/// Single-flight termination state for one shell.
#[derive(Default)]
struct ShellTermination {
    state: AtomicU8,
}

impl ShellTermination {
    /// Claims the reap. `Ok(())` means this caller owns it; `Err(outcome)` means someone else
    /// does, or it already finished, and the caller must return that answer without touching
    /// the child. This is what stops a second `stop` from queueing behind the first on the
    /// child mutex, which is how repeated termination used to become a second way to block.
    fn claim(&self) -> Result<(), ShellTerminationOutcome> {
        match self.state.compare_exchange(
            STATE_IDLE,
            STATE_IN_FLIGHT,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => Ok(()),
            Err(STATE_IN_FLIGHT) => Err(ShellTerminationOutcome::Reaping),
            Err(settled) => Err(ShellTerminationOutcome::from_state(settled)
                .unwrap_or(ShellTerminationOutcome::Reaping)),
        }
    }

    fn settle(&self, outcome: ShellTerminationOutcome) {
        self.state.store(outcome.as_state(), Ordering::Release);
    }

    fn is_settled(&self) -> bool {
        self.state.load(Ordering::Acquire) >= SETTLED_BASE
    }
}

/// One non-blocking look at the child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildProbe {
    Exited,
    Running,
    Failed,
}

fn probe_child(child: &mut dyn Child) -> ChildProbe {
    match child.try_wait() {
        Ok(Some(_)) => ChildProbe::Exited,
        Ok(None) => ChildProbe::Running,
        Err(_) => ChildProbe::Failed,
    }
}

/// Polls until the child exits, the probe fails, or the deadline passes.
///
/// `deadline` of `None` is an unbounded wait for a *natural* exit and is only used by the exit
/// monitor, which stops as soon as a terminate path settles the shell. `park_timeout` rather
/// than `sleep` so an unpark can cut the wait short; a spurious wake just re-probes.
fn poll_until_exit(
    mut probe: impl FnMut() -> ChildProbe,
    mut keep_waiting: impl FnMut() -> bool,
    deadline: Option<Instant>,
    ceiling: Duration,
) -> ShellTerminationOutcome {
    let mut interval = POLL_INTERVAL_FLOOR;
    loop {
        match probe() {
            ChildProbe::Exited => return ShellTerminationOutcome::Reaped,
            ChildProbe::Failed => return ShellTerminationOutcome::ReapFailed,
            ChildProbe::Running => {}
        }
        if !keep_waiting() {
            return ShellTerminationOutcome::Reaping;
        }
        let mut wait_for = interval;
        if let Some(deadline) = deadline {
            let now = Instant::now();
            if now >= deadline {
                return ShellTerminationOutcome::ReapTimedOut;
            }
            wait_for = interval.min(deadline - now);
        }
        thread::park_timeout(wait_for);
        interval = interval.saturating_mul(2).min(ceiling);
    }
}

/// Larger reads coalesce bursty PTY output into fewer IPC events without adding latency:
/// a read still returns as soon as any bytes are available, so interactive echo is
/// unaffected, while a flood of build output emits far fewer events than a 4 KiB buffer.
/// Matches the agent terminal's read width.
const SHELL_READ_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub(crate) struct PortablePtyShellRuntime {
    shells: Arc<Mutex<HashMap<String, ManagedShell>>>,
    events: Arc<dyn WorkspaceShellEventPort>,
    logging: Arc<dyn WorkspaceShellLogPort>,
}

impl PortablePtyShellRuntime {
    pub(crate) fn new(
        events: Arc<dyn WorkspaceShellEventPort>,
        logging: Arc<dyn WorkspaceShellLogPort>,
    ) -> Self {
        Self {
            shells: Arc::new(Mutex::new(HashMap::new())),
            events,
            logging,
        }
    }
}

fn terminal_size(dimensions: TerminalDimensions) -> PtySize {
    PtySize {
        rows: dimensions.rows(),
        cols: dimensions.cols(),
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn default_shell() -> String {
    if cfg!(target_os = "windows") {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

fn shell_root_path(root: &str) -> PathBuf {
    PathBuf::from(normalize_windows_extended_length_path(root))
}

fn write_shell_log(
    logging: &dyn WorkspaceShellLogPort,
    level: WorkspaceLogLevel,
    session_id: &str,
    shell_id: &str,
    message: &str,
) {
    logging.write(ShellLog {
        level,
        session_id: session_id.to_string(),
        shell_id: shell_id.to_string(),
        message: message.to_string(),
    });
}

/// Writes the outcome of a termination, and only when it is worth writing. A clean reap is not
/// a diagnostic; a timeout is, because it means a live process was left behind and somebody has
/// to know that. The code is the whole message -- no raw command or PTY output goes near it.
fn log_termination_outcome(
    logging: &dyn WorkspaceShellLogPort,
    session_id: &str,
    shell_id: &str,
    outcome: ShellTerminationOutcome,
    shutdown: bool,
) {
    if outcome.is_settled_exit() || outcome == ShellTerminationOutcome::Reaping {
        return;
    }
    let phase = if shutdown { " during shutdown" } else { "" };
    let message = match outcome {
        ShellTerminationOutcome::ReapTimedOut => format!(
            "Shell process was not reaped within the termination deadline{phase} \
             (outcome: {}). The child may still be running and remains unreaped.",
            outcome.code()
        ),
        _ => format!(
            "Shell process termination did not complete{phase} (outcome: {}).",
            outcome.code()
        ),
    };
    write_shell_log(
        logging,
        WorkspaceLogLevel::Warn,
        session_id,
        shell_id,
        &message,
    );
}

/// Terminates a child the caller owns exclusively, used on `open_shell`'s failure paths before
/// the shell ever reaches the registry.
fn terminate_child(
    child: &mut dyn Child,
    logging: &dyn WorkspaceShellLogPort,
    session_id: &str,
    shell_id: &str,
    shutdown: bool,
) -> ShellTerminationOutcome {
    let outcome = reap_owned_child(child, Instant::now() + REAP_DEADLINE);
    log_termination_outcome(logging, session_id, shell_id, outcome, shutdown);
    outcome
}

fn reap_owned_child(child: &mut dyn Child, deadline: Instant) -> ShellTerminationOutcome {
    match probe_child(child) {
        ChildProbe::Exited => return ShellTerminationOutcome::AlreadyExited,
        ChildProbe::Failed => return ShellTerminationOutcome::ReapFailed,
        ChildProbe::Running => {}
    }
    if child.kill().is_err() {
        // A refused signal can also mean the child exited between the probe and the kill, so
        // ask once more before calling it a failure.
        return match probe_child(child) {
            ChildProbe::Exited => ShellTerminationOutcome::AlreadyExited,
            _ => ShellTerminationOutcome::KillFailed,
        };
    }
    poll_until_exit(
        || probe_child(child),
        || true,
        Some(deadline),
        POLL_INTERVAL_CEILING,
    )
}

/// Probes the shared child without ever holding its lock across a wait. Each call is one
/// `try_wait`, so the lock is held for microseconds and a terminate can always acquire it --
/// which is exactly what the previous implementation could not promise, because its monitor
/// thread held this lock for the whole of a blocking `wait()`.
fn probe_shared_child(child: &Mutex<Box<dyn Child + Send + Sync>>) -> ChildProbe {
    match child.lock() {
        Ok(mut child) => probe_child(&mut **child),
        Err(_) => ChildProbe::Failed,
    }
}

/// Terminates a registry-owned shell within a bounded deadline.
///
/// Single-flight: the first caller out of `Idle` owns the reap and every other caller is told
/// `reaping` immediately instead of queueing on the child mutex. Repeated termination is
/// therefore both idempotent and non-blocking, and a shell that timed out keeps reporting
/// `reap_timed_out` rather than a later call inventing a success the first one never had.
fn terminate_shell(
    shell: &ManagedShell,
    logging: &dyn WorkspaceShellLogPort,
    shell_id: &str,
    shutdown: bool,
) -> ShellTerminationOutcome {
    if let Err(existing) = shell.termination.claim() {
        return existing;
    }
    let outcome = reap_shared_child(shell, Instant::now() + REAP_DEADLINE);
    shell.termination.settle(outcome);
    log_termination_outcome(logging, &shell.session_id, shell_id, outcome, shutdown);
    outcome
}

fn reap_shared_child(shell: &ManagedShell, deadline: Instant) -> ShellTerminationOutcome {
    match probe_shared_child(&shell.child) {
        ChildProbe::Exited => return ShellTerminationOutcome::AlreadyExited,
        ChildProbe::Failed => return ShellTerminationOutcome::ReapFailed,
        ChildProbe::Running => {}
    }
    // portable-pty 0.9 inverts the Windows TerminateProcess result, so the signal's own return
    // value is not authoritative on its own. A refused kill is only reported as such once a
    // fresh probe confirms the child is still there.
    let kill_refused = match shell.killer.lock() {
        Ok(mut killer) => killer.kill().is_err(),
        Err(_) => true,
    };
    if kill_refused {
        return match probe_shared_child(&shell.child) {
            ChildProbe::Exited => ShellTerminationOutcome::AlreadyExited,
            ChildProbe::Failed => ShellTerminationOutcome::ReapFailed,
            ChildProbe::Running => ShellTerminationOutcome::KillFailed,
        };
    }
    poll_until_exit(
        || probe_shared_child(&shell.child),
        || true,
        Some(deadline),
        POLL_INTERVAL_CEILING,
    )
}

impl PortablePtyShellRuntime {
    fn insert(&self, shell_id: String, shell: ManagedShell) -> Result<(), AppError> {
        // The guard is bound and dropped explicitly rather than left to a temporary's lifetime.
        // Both spellings release the lock before the terminate below, but only one of them says
        // so, and the property here -- no process work under the routing lock -- is one a later
        // edit must not be able to break by accident.
        let replaced = {
            let mut shells = self
                .shells
                .lock()
                .map_err(|error| AppError::Storage(error.to_string()))?;
            shells.insert(shell_id, shell)
        };
        if let Some(replaced) = replaced {
            terminate_shell(&replaced, self.logging.as_ref(), "replaced", false);
        }
        Ok(())
    }

    /// Resolves a shell to its owning session and shared I/O handles, releasing the
    /// registry lock before the caller performs any blocking PTY operation.
    fn checkout(&self, shell_id: &str) -> Result<(String, Arc<ShellIo>), AppError> {
        let shells = self
            .shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let shell = shells
            .get(shell_id)
            .ok_or_else(|| AppError::Validation("Shell session is not connected.".to_string()))?;
        Ok((shell.session_id.clone(), shell.io.clone()))
    }

    /// Resolves a shell's working root alongside its I/O handles, for the directory reset
    /// command that has to be rendered from the root before anything is written.
    fn checkout_with_root(
        &self,
        shell_id: &str,
    ) -> Result<(String, PathBuf, Arc<ShellIo>), AppError> {
        let shells = self
            .shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let shell = shells
            .get(shell_id)
            .ok_or_else(|| AppError::Validation("Shell session is not connected.".to_string()))?;
        Ok((
            shell.session_id.clone(),
            shell.root.clone(),
            shell.io.clone(),
        ))
    }

    fn write_all(io: &ShellIo, bytes: &[u8]) -> Result<(), AppError> {
        let mut writer = io
            .writer
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        writer
            .write_all(bytes)
            .and_then(|_| writer.flush())
            .map_err(|error| AppError::Storage(error.to_string()))
    }

    fn start_exit_monitor(
        &self,
        shell_id: &str,
        session_id: &str,
        io: Arc<ShellIo>,
        child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
        termination: Arc<ShellTermination>,
    ) -> Result<(), AppError> {
        let shells = self.shells.clone();
        let logging = self.logging.clone();
        let events = self.events.clone();
        let shell_id = shell_id.to_owned();
        let session_id = session_id.to_owned();
        thread::Builder::new()
            .name(format!("vanehub-shell-monitor-{shell_id}"))
            .spawn(move || {
                // Waiting for a *natural* exit is legitimately open-ended -- a user may keep a
                // shell open for hours -- but it must never be open-ended while holding the
                // child lock, which is what made a wedged child able to park every terminate.
                // Each probe takes the lock for one `try_wait` and releases it, and the loop
                // ends the moment a terminate path settles the shell.
                let outcome = poll_until_exit(
                    || probe_shared_child(&child),
                    || !termination.is_settled(),
                    None,
                    MONITOR_INTERVAL_CEILING,
                );
                if outcome == ShellTerminationOutcome::Reaping {
                    // A terminate path owns this child now; it publishes its own state.
                    return;
                }
                if outcome == ShellTerminationOutcome::Reaped {
                    termination.settle(ShellTerminationOutcome::Reaped);
                } else {
                    termination.settle(outcome);
                    log_termination_outcome(
                        logging.as_ref(),
                        &session_id,
                        &shell_id,
                        outcome,
                        false,
                    );
                }
                if let Ok(mut shells) = shells.lock() {
                    let owns_entry = shells
                        .get(&shell_id)
                        .is_some_and(|shell| Arc::ptr_eq(&shell.io, &io));
                    if owns_entry {
                        shells.remove(&shell_id);
                    }
                }
                events.publish(ShellEvent::State {
                    shell_id,
                    session_id,
                    state: "disconnected",
                    error: None,
                });
            })
            .map(|_| ())
            .map_err(|error| AppError::Storage(error.to_string()))
    }
}

impl WorkspaceShellRuntimePort for PortablePtyShellRuntime {
    fn open_shell(&self, launch: &ShellLaunch) -> Result<(), AppError> {
        let root = shell_root_path(&launch.root);
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(terminal_size(launch.dimensions))
            .map_err(|error| {
                write_shell_log(
                    self.logging.as_ref(),
                    WorkspaceLogLevel::Error,
                    &launch.session_id,
                    &launch.shell_id,
                    "PTY creation failed.",
                );
                AppError::LaunchFailed(error.to_string())
            })?;
        let mut command = CommandBuilder::new(default_shell());
        command.cwd(&root);
        let mut child = pair.slave.spawn_command(command).map_err(|error| {
            write_shell_log(
                self.logging.as_ref(),
                WorkspaceLogLevel::Error,
                &launch.session_id,
                &launch.shell_id,
                "Shell process launch failed.",
            );
            AppError::LaunchFailed(error.to_string())
        })?;
        drop(pair.slave);
        let mut reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(error) => {
                terminate_child(
                    child.as_mut(),
                    self.logging.as_ref(),
                    &launch.session_id,
                    &launch.shell_id,
                    false,
                );
                return Err(AppError::Storage(error.to_string()));
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(error) => {
                terminate_child(
                    child.as_mut(),
                    self.logging.as_ref(),
                    &launch.session_id,
                    &launch.shell_id,
                    false,
                );
                return Err(AppError::Storage(error.to_string()));
            }
        };

        let killer = child.clone_killer();
        let child = Arc::new(Mutex::new(child));
        let events = self.events.clone();
        let reader_shell_id = launch.shell_id.clone();
        let reader_session_id = launch.session_id.clone();
        let io = Arc::new(ShellIo {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
        });
        let monitor_io = io.clone();
        let monitor_child = child.clone();
        let termination = Arc::new(ShellTermination::default());
        let monitor_termination = termination.clone();
        self.insert(
            launch.shell_id.clone(),
            ManagedShell {
                session_id: launch.session_id.clone(),
                root,
                io,
                child,
                killer: Mutex::new(killer),
                termination,
            },
        )?;
        let reader_worker = thread::Builder::new()
            .name(format!("vanehub-shell-reader-{}", launch.shell_id))
            .spawn(move || {
                let mut buffer = [0u8; SHELL_READ_BUFFER_BYTES];
                // Reads land on arbitrary byte boundaries, so a multi-byte UTF-8 sequence can be
                // split across two reads; carry the incomplete tail until the next read completes it.
                let mut pending: Vec<u8> = Vec::new();
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(count) => {
                            pending.extend_from_slice(&buffer[..count]);
                            let content = take_decodable_utf8(&mut pending);
                            if content.is_empty() {
                                continue;
                            }
                            events.publish(ShellEvent::Output {
                                shell_id: reader_shell_id.clone(),
                                session_id: reader_session_id.clone(),
                                content,
                            })
                        }
                        Err(_) => break,
                    }
                }
            });
        if let Err(error) = reader_worker {
            let _ = self.stop(&launch.shell_id);
            return Err(AppError::Storage(error.to_string()));
        }
        if let Err(error) = self.start_exit_monitor(
            &launch.shell_id,
            &launch.session_id,
            monitor_io,
            monitor_child,
            monitor_termination,
        ) {
            let _ = self.stop(&launch.shell_id);
            return Err(error);
        }
        Ok(())
    }

    fn write_input(&self, shell_id: &str, content: &str) -> Result<(), AppError> {
        let (session_id, io) = self.checkout(shell_id)?;
        let result = Self::write_all(&io, content.as_bytes());
        if result.is_err() {
            write_shell_log(
                self.logging.as_ref(),
                WorkspaceLogLevel::Warn,
                &session_id,
                shell_id,
                "Shell input failed.",
            );
        }
        result
    }

    fn reset_directory(&self, shell_id: &str) -> Result<(), AppError> {
        let (session_id, root, io) = self.checkout_with_root(shell_id)?;
        let host = if cfg!(target_os = "windows") {
            ShellHost::Windows
        } else {
            ShellHost::Unix
        };
        let command = reset_directory_command(&root.to_string_lossy(), host);
        let result = Self::write_all(&io, command.as_bytes());
        if result.is_err() {
            write_shell_log(
                self.logging.as_ref(),
                WorkspaceLogLevel::Warn,
                &session_id,
                shell_id,
                "Shell directory reset failed.",
            );
        }
        result
    }

    fn resize(&self, shell_id: &str, dimensions: TerminalDimensions) -> Result<(), AppError> {
        let (session_id, io) = self.checkout(shell_id)?;
        let result = io
            .master
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))
            .and_then(|master| {
                master
                    .resize(terminal_size(dimensions))
                    .map_err(|error| AppError::Storage(error.to_string()))
            });
        if result.is_err() {
            write_shell_log(
                self.logging.as_ref(),
                WorkspaceLogLevel::Warn,
                &session_id,
                shell_id,
                "Shell resize failed.",
            );
        }
        result
    }

    fn stop(&self, shell_id: &str) -> Result<Option<String>, AppError> {
        let shell = {
            let mut shells = self
                .shells
                .lock()
                .map_err(|error| AppError::Storage(error.to_string()))?;
            shells.remove(shell_id)
        };
        let Some(shell) = shell else {
            return Ok(None);
        };
        terminate_shell(&shell, self.logging.as_ref(), shell_id, false);
        Ok(Some(shell.session_id))
    }

    fn stop_for_session(&self, session_id: &str) -> Result<Vec<(String, String)>, AppError> {
        let shell_ids = {
            let shells = self
                .shells
                .lock()
                .map_err(|error| AppError::Storage(error.to_string()))?;
            shells
                .iter()
                .filter(|(_, shell)| shell.session_id == session_id)
                .map(|(shell_id, _)| shell_id.clone())
                .collect::<Vec<_>>()
        };
        let mut stopped = Vec::with_capacity(shell_ids.len());
        for shell_id in shell_ids {
            if let Some(owning_session_id) = self.stop(&shell_id)? {
                stopped.push((shell_id, owning_session_id));
            }
        }
        Ok(stopped)
    }
}

impl Drop for PortablePtyShellRuntime {
    fn drop(&mut self) {
        if Arc::strong_count(&self.shells) != 1 {
            return;
        }
        // Drain first, release the routing lock, then terminate. Terminating inside the guard
        // meant one wedged child blocked shutdown for every other shell -- and, before the
        // reap was bounded, blocked it permanently.
        let shells: Vec<ManagedShell> = match self.shells.lock() {
            Ok(mut shells) => shells.drain().map(|(_, shell)| shell).collect(),
            Err(_) => return,
        };
        for shell in &shells {
            terminate_shell(shell, self.logging.as_ref(), "shutdown", true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use portable_pty::{ChildKiller, ExitStatus};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("vanehub-shell-{label}-{suffix}"))
    }

    fn remove_test_dir(path: &Path) {
        let mut last_error = None;
        for _ in 0..20 {
            match std::fs::remove_dir_all(path) {
                Ok(()) => return,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return,
                Err(error) => {
                    last_error = Some(error);
                    std::thread::sleep(Duration::from_millis(25));
                }
            }
        }
        panic!("cleanup: {}", last_error.expect("cleanup error"));
    }

    #[derive(Debug)]
    struct FailingChild;

    #[derive(Default)]
    struct CapturingEvents {
        events: Mutex<Vec<ShellEvent>>,
    }

    impl WorkspaceShellEventPort for CapturingEvents {
        fn publish(&self, event: ShellEvent) {
            self.events.lock().expect("events").push(event);
        }
    }

    #[derive(Default)]
    struct CapturingLogs {
        logs: Mutex<Vec<ShellLog>>,
    }

    impl WorkspaceShellLogPort for CapturingLogs {
        fn write(&self, log: ShellLog) {
            self.logs.lock().expect("logs").push(log);
        }
    }

    fn runtime() -> (PortablePtyShellRuntime, Arc<CapturingLogs>) {
        let logging = Arc::new(CapturingLogs::default());
        (
            PortablePtyShellRuntime::new(Arc::new(CapturingEvents::default()), logging.clone()),
            logging,
        )
    }

    impl ChildKiller for FailingChild {
        fn kill(&mut self) -> io::Result<()> {
            Err(io::Error::other("secret kill detail"))
        }

        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            Box::new(Self)
        }
    }

    impl Child for FailingChild {
        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            Ok(None)
        }

        fn wait(&mut self) -> io::Result<ExitStatus> {
            Err(io::Error::other("secret wait detail"))
        }

        fn process_id(&self) -> Option<u32> {
            None
        }

        #[cfg(windows)]
        fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> {
            None
        }
    }

    fn managed_test_shell(session_id: &str, root: &Path) -> ManagedShell {
        let pair = native_pty_system()
            .openpty(terminal_size(TerminalDimensions::bounded(24, 80)))
            .expect("test pty");
        let mut command = CommandBuilder::new(default_shell());
        command.cwd(root);
        let child = pair.slave.spawn_command(command).expect("test shell");
        let killer = child.clone_killer();
        drop(pair.slave);
        let writer = pair.master.take_writer().expect("test writer");
        ManagedShell {
            session_id: session_id.to_string(),
            root: root.to_path_buf(),
            io: Arc::new(ShellIo {
                master: Mutex::new(pair.master),
                writer: Mutex::new(writer),
            }),
            child: Arc::new(Mutex::new(child)),
            killer: Mutex::new(killer),
            termination: Arc::new(ShellTermination::default()),
        }
    }

    /// What a fake child does when polled.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FakeExit {
        /// Never exits, however long it is polled. The wedged child this change exists for.
        Never,
        /// Already gone before anything is sent to it.
        Immediately,
        /// Exits once a kill has actually been delivered.
        AfterKill,
        /// `try_wait` itself errors.
        ProbeError,
    }

    #[derive(Debug, Default)]
    struct FakeChildState {
        exit: Option<FakeExit>,
        kill_refused: bool,
        killed: AtomicBool,
        polls: AtomicUsize,
        /// Set if the *blocking* wait is ever entered. No production path may reach it.
        blocking_wait_reached: AtomicBool,
    }

    impl FakeChildState {
        fn new(exit: FakeExit, kill_refused: bool) -> Arc<Self> {
            Arc::new(Self {
                exit: Some(exit),
                kill_refused,
                ..Self::default()
            })
        }

        fn polls(&self) -> usize {
            self.polls.load(Ordering::Acquire)
        }

        fn was_killed(&self) -> bool {
            self.killed.load(Ordering::Acquire)
        }

        fn blocking_wait_reached(&self) -> bool {
            self.blocking_wait_reached.load(Ordering::Acquire)
        }
    }

    #[derive(Debug)]
    struct FakeChild(Arc<FakeChildState>);

    impl ChildKiller for FakeChild {
        fn kill(&mut self) -> io::Result<()> {
            self.0.killed.store(true, Ordering::Release);
            if self.0.kill_refused {
                return Err(io::Error::other("fake kill refused"));
            }
            Ok(())
        }

        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            Box::new(FakeChild(self.0.clone()))
        }
    }

    impl Child for FakeChild {
        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            self.0.polls.fetch_add(1, Ordering::AcqRel);
            match self.0.exit {
                Some(FakeExit::Immediately) => Ok(Some(ExitStatus::with_exit_code(0))),
                Some(FakeExit::ProbeError) => Err(io::Error::other("fake probe failure")),
                Some(FakeExit::AfterKill) if self.0.was_killed() => {
                    Ok(Some(ExitStatus::with_exit_code(0)))
                }
                _ => Ok(None),
            }
        }

        /// The whole point of this fake. Bounding today's wait fixes today's hang; making the
        /// blocking call unreachable and proving it is what fixes the class. If a later edit
        /// reintroduces one, this fails by name instead of hanging until CI cancels the job.
        fn wait(&mut self) -> io::Result<ExitStatus> {
            self.0.blocking_wait_reached.store(true, Ordering::Release);
            panic!("a production path reached the blocking Child::wait; it must use try_wait");
        }

        fn process_id(&self) -> Option<u32> {
            None
        }

        #[cfg(windows)]
        fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> {
            None
        }
    }

    /// A managed shell with a real PTY for io and a scripted child for termination. The io half
    /// is real so `write_input` and `resize` exercise the actual master; the child half is fake
    /// so no test depends on how a given OS reaps a real process.
    fn scripted_shell(
        session_id: &str,
        root: &Path,
        exit: FakeExit,
        kill_refused: bool,
    ) -> (
        ManagedShell,
        Box<dyn portable_pty::SlavePty + Send>,
        Arc<FakeChildState>,
    ) {
        let pair = native_pty_system()
            .openpty(terminal_size(TerminalDimensions::bounded(24, 80)))
            .expect("test pty");
        let writer = pair.master.take_writer().expect("test writer");
        let state = FakeChildState::new(exit, kill_refused);
        let child = FakeChild(state.clone());
        let killer = child.clone_killer();
        let shell = ManagedShell {
            session_id: session_id.to_string(),
            root: root.to_path_buf(),
            io: Arc::new(ShellIo {
                master: Mutex::new(pair.master),
                writer: Mutex::new(writer),
            }),
            child: Arc::new(Mutex::new(Box::new(child) as Box<dyn Child + Send + Sync>)),
            killer: Mutex::new(killer),
            termination: Arc::new(ShellTermination::default()),
        };
        // The slave end is handed back so the caller keeps it open; dropping it would make
        // writes to the master fail for a reason that has nothing to do with the test.
        (shell, pair.slave, state)
    }

    /// Short enough that a regression fails in milliseconds. A test that proves a bound must
    /// not need an unbounded harness to notice that the bound is gone.
    const TEST_DEADLINE: Duration = Duration::from_millis(75);

    #[test]
    fn terminal_dimensions_are_bounded() {
        assert_eq!(terminal_size(TerminalDimensions::bounded(0, 0)).rows, 1);
        assert_eq!(
            terminal_size(TerminalDimensions::bounded(800, 900)).cols,
            500
        );
    }

    #[test]
    fn missing_shell_kill_is_idempotent_at_manager_level() {
        let (manager, _) = runtime();
        assert_eq!(manager.stop("missing").expect("first stop"), None);
        assert_eq!(manager.stop("missing").expect("second stop"), None);
    }

    #[test]
    fn child_shutdown_failures_write_generic_warnings() {
        let logging = CapturingLogs::default();
        // `FailingChild` refuses the kill and keeps reporting itself as running, so this is a
        // refused signal rather than a reap that ran out of time. One outcome, one warning:
        // the old pair of messages described the same event twice and named neither.
        let outcome = terminate_child(
            &mut FailingChild,
            &logging,
            "session-one",
            "shell-one",
            false,
        );
        assert_eq!(outcome, ShellTerminationOutcome::KillFailed);
        let messages = logging
            .logs
            .lock()
            .expect("logs")
            .iter()
            .map(|log| log.message.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            messages,
            vec!["Shell process termination did not complete (outcome: kill_failed)."]
        );
        // The child's own error text carries a secret; the outcome code carries none of it.
        assert!(!messages.join(" ").contains("secret"));
    }

    #[test]
    fn missing_shell_routes_return_validation_errors() {
        let (manager, _) = runtime();
        assert!(manager.write_input("missing", "echo test").is_err());
        assert!(manager.reset_directory("missing").is_err());
        assert!(manager
            .resize("missing", TerminalDimensions::bounded(24, 80))
            .is_err());
    }

    #[test]
    fn default_shell_and_cd_escaping_are_platform_specific() {
        assert!(!default_shell().trim().is_empty());
    }

    #[test]
    fn shell_roots_strip_windows_extended_length_prefixes_before_launch() {
        assert_eq!(
            shell_root_path(r"\\?\D:\cdavid\Documents\code\claude-code").to_string_lossy(),
            r"D:\cdavid\Documents\code\claude-code"
        );
        assert_eq!(
            shell_root_path(r"\\?\UNC\server\share\repo").to_string_lossy(),
            r"\\server\share\repo"
        );
    }

    #[test]
    fn manager_routes_input_resize_and_cleanup_by_shell_id() {
        let root = temp_dir("manager");
        std::fs::create_dir_all(&root).expect("root");
        let (manager, _) = runtime();
        // Real PTY io, scripted children. Spawning real shells here made this test depend on
        // how each OS reaps a killed process, which is what made it hang on macOS for two hours
        // and forty minutes rather than fail.
        let (first, _first_slave, first_state) =
            scripted_shell("session-one", &root, FakeExit::AfterKill, false);
        let (second, _second_slave, second_state) =
            scripted_shell("session-two", &root, FakeExit::AfterKill, false);
        manager
            .insert("shell-one".to_string(), first)
            .expect("insert first");
        manager
            .insert("shell-two".to_string(), second)
            .expect("insert second");
        assert_eq!(manager.shells.lock().expect("shell map").len(), 2);
        manager
            .write_input(
                "shell-one",
                if cfg!(windows) {
                    "echo test\r\n"
                } else {
                    "echo test\n"
                },
            )
            .expect("input");
        manager
            .resize("shell-two", TerminalDimensions::bounded(30, 100))
            .expect("resize");
        assert_eq!(
            manager.stop("shell-one").expect("stop first").as_deref(),
            Some("session-one")
        );
        assert_eq!(manager.stop("shell-one").expect("repeat stop"), None);
        assert_eq!(
            manager.stop("shell-two").expect("stop second").as_deref(),
            Some("session-two")
        );
        assert!(manager.shells.lock().expect("shell map").is_empty());
        for state in [&first_state, &second_state] {
            assert!(state.was_killed(), "each shell's child was signalled");
            assert!(
                !state.blocking_wait_reached(),
                "termination must reach the child through try_wait, never the blocking wait"
            );
        }
        remove_test_dir(&root);
    }

    #[test]
    fn a_blocked_shell_writer_does_not_stall_other_shells() {
        let root = temp_dir("writer-isolation");
        std::fs::create_dir_all(&root).expect("root");
        let (manager, _) = runtime();
        let (first, _first_slave, _first_state) =
            scripted_shell("session-one", &root, FakeExit::AfterKill, false);
        let (second, _second_slave, _second_state) =
            scripted_shell("session-two", &root, FakeExit::AfterKill, false);
        manager
            .insert("shell-one".to_string(), first)
            .expect("insert first");
        manager
            .insert("shell-two".to_string(), second)
            .expect("insert second");

        // Stands in for a child that stopped draining its pipe: shell-one's writer is held
        // for the whole test. The registry lock must not be part of that critical section.
        let (_, blocked) = manager.checkout("shell-one").expect("checkout first");
        let _held = blocked.writer.lock().expect("hold first writer");

        manager
            .resize("shell-two", TerminalDimensions::bounded(30, 100))
            .expect("second shell resizes while the first writer is blocked");
        manager
            .write_input("shell-two", if cfg!(windows) { "\r\n" } else { "\n" })
            .expect("second shell accepts input while the first writer is blocked");
        assert_eq!(
            manager.stop("shell-two").expect("stop second").as_deref(),
            Some("session-two")
        );

        drop(_held);
        manager.stop("shell-one").expect("stop first");
        remove_test_dir(&root);
    }

    #[test]
    fn a_wedged_child_does_not_stall_input_resize_or_cleanup_for_another_shell() {
        let root = temp_dir("wedged-isolation");
        std::fs::create_dir_all(&root).expect("root");
        let (manager, _) = runtime();
        // shell-one never exits, so its reap runs to the deadline. shell-two must stay fully
        // serviceable throughout -- that is the property the routing lock exists to protect,
        // and the one that terminating under the lock destroyed.
        let (wedged, _wedged_slave, wedged_state) =
            scripted_shell("session-one", &root, FakeExit::Never, false);
        let (healthy, _healthy_slave, _healthy_state) =
            scripted_shell("session-two", &root, FakeExit::AfterKill, false);
        manager
            .insert("shell-one".to_string(), wedged)
            .expect("insert wedged");
        manager
            .insert("shell-two".to_string(), healthy)
            .expect("insert healthy");

        let outcome = {
            let shells = manager.shells.lock().expect("shell map");
            let wedged = shells.get("shell-one").expect("wedged shell");
            reap_shared_child(wedged, Instant::now() + TEST_DEADLINE)
        };
        assert_eq!(outcome, ShellTerminationOutcome::ReapTimedOut);
        assert!(wedged_state.was_killed());
        assert!(
            wedged_state.polls() > 1,
            "the deadline was polled, not slept"
        );

        manager
            .resize("shell-two", TerminalDimensions::bounded(30, 100))
            .expect("healthy shell resizes while the other is wedged");
        manager
            .write_input("shell-two", if cfg!(windows) { "\r\n" } else { "\n" })
            .expect("healthy shell accepts input while the other is wedged");
        assert_eq!(
            manager.stop("shell-two").expect("stop healthy").as_deref(),
            Some("session-two")
        );
        manager.stop("shell-one").expect("stop wedged");
        remove_test_dir(&root);
    }

    #[test]
    fn every_termination_outcome_is_named_and_a_timeout_is_never_reported_as_terminated() {
        let root = temp_dir("outcomes");
        std::fs::create_dir_all(&root).expect("root");

        let cases = [
            (
                FakeExit::Immediately,
                false,
                ShellTerminationOutcome::AlreadyExited,
                "already_exited",
            ),
            (
                FakeExit::AfterKill,
                false,
                ShellTerminationOutcome::Reaped,
                "reaped",
            ),
            (
                FakeExit::Never,
                false,
                ShellTerminationOutcome::ReapTimedOut,
                "reap_timed_out",
            ),
            (
                FakeExit::Never,
                true,
                ShellTerminationOutcome::KillFailed,
                "kill_failed",
            ),
            (
                FakeExit::ProbeError,
                false,
                ShellTerminationOutcome::ReapFailed,
                "reap_failed",
            ),
        ];

        for (exit, kill_refused, expected, code) in cases {
            let (shell, _slave, state) = scripted_shell("session", &root, exit, kill_refused);
            let outcome = reap_shared_child(&shell, Instant::now() + TEST_DEADLINE);
            assert_eq!(outcome, expected, "outcome for {exit:?}/{kill_refused}");
            assert_eq!(outcome.code(), code);
            assert!(
                !state.blocking_wait_reached(),
                "no outcome path may reach the blocking wait"
            );
        }

        // The distinction the whole enum exists for.
        assert!(!ShellTerminationOutcome::ReapTimedOut.is_settled_exit());
        assert!(!ShellTerminationOutcome::KillFailed.is_settled_exit());
        assert!(!ShellTerminationOutcome::ReapFailed.is_settled_exit());
        assert!(ShellTerminationOutcome::Reaped.is_settled_exit());
        assert!(ShellTerminationOutcome::AlreadyExited.is_settled_exit());
        remove_test_dir(&root);
    }

    #[test]
    fn a_timed_out_reap_records_the_child_it_left_behind() {
        let root = temp_dir("timeout-evidence");
        std::fs::create_dir_all(&root).expect("root");
        let logging = CapturingLogs::default();
        let (shell, _slave, _state) = scripted_shell("session-one", &root, FakeExit::Never, false);

        log_termination_outcome(
            &logging,
            &shell.session_id,
            "shell-one",
            reap_shared_child(&shell, Instant::now() + TEST_DEADLINE),
            false,
        );

        let entries = logging.logs.lock().expect("logs");
        let entry = entries
            .first()
            .expect("a timed-out reap is worth recording");
        assert_eq!(entry.session_id, "session-one");
        assert_eq!(entry.shell_id, "shell-one");
        assert!(
            entry.message.contains("reap_timed_out"),
            "the stable code reaches diagnostics: {}",
            entry.message
        );
        assert!(
            entry.message.contains("remains unreaped"),
            "cleanup ownership of the surviving child stays visible: {}",
            entry.message
        );
        drop(entries);
        remove_test_dir(&root);
    }

    #[test]
    fn a_clean_reap_is_not_a_diagnostic() {
        let root = temp_dir("clean-reap-quiet");
        std::fs::create_dir_all(&root).expect("root");
        let logging = CapturingLogs::default();
        let (shell, _slave, _state) =
            scripted_shell("session-one", &root, FakeExit::AfterKill, false);

        log_termination_outcome(
            &logging,
            &shell.session_id,
            "shell-one",
            reap_shared_child(&shell, Instant::now() + TEST_DEADLINE),
            false,
        );

        assert!(
            logging.logs.lock().expect("logs").is_empty(),
            "a shell that shut down cleanly is not a warning"
        );
        remove_test_dir(&root);
    }

    #[test]
    fn repeated_termination_is_idempotent_and_starts_only_one_reap() {
        let root = temp_dir("single-flight");
        std::fs::create_dir_all(&root).expect("root");
        let logging = CapturingLogs::default();
        let (shell, _slave, state) =
            scripted_shell("session-one", &root, FakeExit::AfterKill, false);

        let first = terminate_shell(&shell, &logging, "shell-one", false);
        assert_eq!(first, ShellTerminationOutcome::Reaped);
        let polls_after_first = state.polls();

        // A second stop must answer from the settled state rather than touch the child again.
        let second = terminate_shell(&shell, &logging, "shell-one", false);
        assert_eq!(second, ShellTerminationOutcome::Reaped);
        assert_eq!(
            state.polls(),
            polls_after_first,
            "the second termination started no second reap"
        );
        remove_test_dir(&root);
    }

    #[test]
    fn a_repeated_stop_after_a_timeout_still_reports_the_timeout() {
        let root = temp_dir("timeout-repeat");
        std::fs::create_dir_all(&root).expect("root");
        let logging = CapturingLogs::default();
        let (shell, _slave, _state) = scripted_shell("session-one", &root, FakeExit::Never, false);
        shell
            .termination
            .settle(ShellTerminationOutcome::ReapTimedOut);

        // The settled outcome is carried, not replaced by a cheerful default. A shell whose
        // child was never reaped must not start reporting success on the second ask.
        let repeated = terminate_shell(&shell, &logging, "shell-one", false);
        assert_eq!(repeated, ShellTerminationOutcome::ReapTimedOut);
        assert!(!repeated.is_settled_exit());
        remove_test_dir(&root);
    }

    #[test]
    fn an_in_flight_reap_turns_a_concurrent_stop_away_rather_than_queueing_it() {
        let termination = ShellTermination::default();
        termination.claim().expect("first caller owns the reap");
        assert_eq!(
            termination.claim(),
            Err(ShellTerminationOutcome::Reaping),
            "a concurrent caller is told a reap is in flight instead of blocking on the child"
        );
        termination.settle(ShellTerminationOutcome::Reaped);
        assert!(termination.is_settled());
        assert_eq!(
            termination.claim(),
            Err(ShellTerminationOutcome::Reaped),
            "once settled, the recorded outcome is what every later caller sees"
        );
    }

    #[test]
    fn every_outcome_survives_the_round_trip_through_termination_state() {
        for outcome in [
            ShellTerminationOutcome::AlreadyExited,
            ShellTerminationOutcome::KillRequested,
            ShellTerminationOutcome::Reaping,
            ShellTerminationOutcome::Reaped,
            ShellTerminationOutcome::ReapTimedOut,
            ShellTerminationOutcome::KillFailed,
            ShellTerminationOutcome::ReapFailed,
        ] {
            let state = outcome.as_state();
            assert!(
                state >= SETTLED_BASE,
                "a settled state can never collide with idle or in-flight"
            );
            assert_eq!(ShellTerminationOutcome::from_state(state), Some(outcome));
        }
        assert_eq!(ShellTerminationOutcome::from_state(STATE_IDLE), None);
        assert_eq!(ShellTerminationOutcome::from_state(STATE_IN_FLIGHT), None);
    }

    #[test]
    fn polling_stops_as_soon_as_a_terminate_path_takes_ownership() {
        let polls = AtomicUsize::new(0);
        let outcome = poll_until_exit(
            || {
                polls.fetch_add(1, Ordering::AcqRel);
                ChildProbe::Running
            },
            // Stands in for the exit monitor seeing a terminate settle the shell: an unbounded
            // wait for a natural exit must still be able to stop.
            || polls.load(Ordering::Acquire) < 3,
            None,
            POLL_INTERVAL_CEILING,
        );
        assert_eq!(outcome, ShellTerminationOutcome::Reaping);
        assert_eq!(polls.load(Ordering::Acquire), 3);
    }

    #[test]
    fn manager_cleans_up_only_the_requested_session_shells() {
        let root = temp_dir("session-cleanup");
        std::fs::create_dir_all(&root).expect("root");
        let (manager, _) = runtime();
        manager
            .insert(
                "shell-one".to_string(),
                managed_test_shell("session-one", &root),
            )
            .expect("insert first");
        manager
            .insert(
                "shell-two".to_string(),
                managed_test_shell("session-two", &root),
            )
            .expect("insert second");

        let stopped = manager
            .stop_for_session("session-one")
            .expect("stop session");

        assert_eq!(
            stopped,
            vec![("shell-one".to_string(), "session-one".to_string())]
        );
        assert!(manager
            .shells
            .lock()
            .expect("shell map")
            .contains_key("shell-two"));
        assert_eq!(
            manager
                .stop_for_session("session-one")
                .expect("repeat cleanup"),
            Vec::new()
        );
        manager.stop("shell-two").expect("stop remaining");
        remove_test_dir(&root);
    }

    #[test]
    fn externally_exited_shell_is_removed_and_reaped() {
        let root = temp_dir("natural-exit");
        std::fs::create_dir_all(&root).expect("root");
        let (manager, _) = runtime();
        let pair = native_pty_system()
            .openpty(terminal_size(TerminalDimensions::bounded(24, 80)))
            .expect("test pty");
        let mut command = if cfg!(windows) {
            CommandBuilder::new("cmd.exe")
        } else {
            CommandBuilder::new("/bin/sh")
        };
        command.cwd(&root);
        if cfg!(windows) {
            command.arg("/C");
            command.arg("exit 0");
        } else {
            command.arg("-c");
            command.arg("exit 0");
        }
        let child = pair
            .slave
            .spawn_command(command)
            .expect("short-lived shell");
        let mut external_killer = child.clone_killer();
        let killer = child.clone_killer();
        let child = Arc::new(Mutex::new(child));
        drop(pair.slave);
        let writer = pair.master.take_writer().expect("test writer");
        let io = Arc::new(ShellIo {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
        });
        let termination = Arc::new(ShellTermination::default());
        manager
            .insert(
                "shell-natural-exit".to_owned(),
                ManagedShell {
                    session_id: "session-natural-exit".to_owned(),
                    root: root.clone(),
                    io: io.clone(),
                    child: child.clone(),
                    killer: Mutex::new(killer),
                    termination: termination.clone(),
                },
            )
            .expect("register shell");
        manager
            .start_exit_monitor(
                "shell-natural-exit",
                "session-natural-exit",
                io,
                child,
                termination,
            )
            .expect("start exit monitor");
        // portable-pty 0.9 reports an inverted Windows kill result. The waiter observes the
        // actual process exit, which is the lifecycle behavior this regression test exercises.
        let _ = external_killer.kill();

        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            if manager.shells.lock().expect("shell map").is_empty() {
                remove_test_dir(&root);
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("exited shell remained in the live registry");
    }

    #[test]
    fn invalid_shell_executable_fails_to_spawn() {
        let pair = native_pty_system()
            .openpty(terminal_size(TerminalDimensions::bounded(24, 80)))
            .expect("test pty");
        let command = CommandBuilder::new("vanehub-shell-executable-that-does-not-exist");
        assert!(pair.slave.spawn_command(command).is_err());
    }
}
