//! Session-scoped background command execution for the native tool loop's `shell` tool
//! (`add-background-shell-execution`).
//!
//! Foreground `shell` runs through `platform::process`'s one-shot bounded execution. Background
//! commands cannot: they must outlive the tool call that started them, so this module owns the
//! child process itself. It deliberately reuses `ManagedChild`, not a raw `Command` -- that type
//! already carries the two properties background execution depends on: a Windows job object with
//! `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and a Unix process-group leader. Together they mean
//! killing a command kills the tree it spawned, and desktop exit reaps every tree even if this
//! registry's own cleanup never runs.
//!
//! State here is intentionally in-memory and never persisted. A handle from a previous desktop
//! run resolves to "unknown", which is the honest answer -- the alternative would be either
//! resurrecting processes across restarts or storing rows that can only ever say "this is gone".

use super::shell_tool::shell_invocation;
use super::MAX_TOOL_OUTPUT_BYTES;
use crate::platform::process::{audit_command, ManagedChild};
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

/// Running commands one session may own at once. Enough for a dev server plus parallel checks;
/// low enough that a tool loop cannot turn the registry into a fork bomb.
pub(crate) const MAX_BACKGROUND_COMMANDS_PER_SESSION: usize = 8;

/// Rolling buffer per command. Four times the 64KB single-result cap, so a caller polling at any
/// reasonable rate never loses output while an unpolled chatty process still cannot grow without
/// bound.
pub(crate) const MAX_BUFFERED_OUTPUT_BYTES: usize = 256 * 1024;

/// Longer than any check in this repository, short enough that a forgotten process is not
/// permanent.
pub(crate) const MAX_BACKGROUND_LIFETIME: Duration = Duration::from_secs(30 * 60);

/// Terminal entries are kept after exit so `shell_output` can still report an exit code. This
/// bounds how many a single session accumulates -- without it, a loop that starts and finishes
/// commands would grow the map forever even though the *running* limit is never exceeded.
const MAX_RETAINED_TERMINAL_PER_SESSION: usize = 8;

const SUPERVISOR_POLL_INTERVAL: Duration = Duration::from_millis(50);
const TERMINATION_GRACE: Duration = Duration::from_secs(5);
const READ_CHUNK_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundStatus {
    Running,
    Exited(Option<i32>),
    Killed,
    LifetimeExceeded,
}

impl BackgroundStatus {
    pub(crate) const fn is_terminal(self) -> bool {
        !matches!(self, Self::Running)
    }

    pub(crate) fn label(self) -> String {
        match self {
            Self::Running => "running".to_owned(),
            Self::Exited(Some(code)) => format!("exited (code {code})"),
            Self::Exited(None) => "exited (code unavailable)".to_owned(),
            Self::Killed => "terminated".to_owned(),
            Self::LifetimeExceeded => {
                "terminated (exceeded maximum background lifetime)".to_owned()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundStartError {
    SessionLimitReached,
    Spawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnknownHandle;

/// Distinguishes "this call stopped it" from "it had already finished". Both are successes, but
/// reporting the second as a termination would tell the model it interrupted work that had in
/// fact completed -- and possibly succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KillOutcome {
    Terminated(BackgroundStatus),
    AlreadyFinished(BackgroundStatus),
}

impl KillOutcome {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn status(self) -> BackgroundStatus {
        match self {
            Self::Terminated(status) | Self::AlreadyFinished(status) => status,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackgroundOutput {
    pub(crate) text: String,
    pub(crate) dropped_bytes: u64,
    pub(crate) remaining_bytes: usize,
    pub(crate) status: BackgroundStatus,
}

#[derive(Debug, Default)]
struct OutputBuffer {
    bytes: Vec<u8>,
    dropped: u64,
}

impl OutputBuffer {
    fn append(&mut self, chunk: &[u8]) {
        self.bytes.extend_from_slice(chunk);
        if self.bytes.len() > MAX_BUFFERED_OUTPUT_BYTES {
            let overflow = self.bytes.len() - MAX_BUFFERED_OUTPUT_BYTES;
            self.bytes.drain(..overflow);
            self.dropped += overflow as u64;
        }
    }

    /// Takes up to `max` bytes from the *front*, leaving the rest buffered for the next call.
    /// Front-first is what keeps retrieval lossless: the alternative -- returning the tail and
    /// discarding the head -- would silently drop output the caller never got a chance to read.
    fn take_front(&mut self, max: usize) -> (String, u64, usize) {
        let split = safe_split_point(&self.bytes, max);
        let taken: Vec<u8> = self.bytes.drain(..split).collect();
        let dropped = std::mem::take(&mut self.dropped);
        (
            String::from_utf8_lossy(&taken).into_owned(),
            dropped,
            self.bytes.len(),
        )
    }
}

/// Backs off `max` to the nearest UTF-8 sequence boundary so a multi-byte character is never
/// split across two retrievals (which `from_utf8_lossy` would turn into replacement characters on
/// both sides).
fn safe_split_point(bytes: &[u8], max: usize) -> usize {
    let mut split = max.min(bytes.len());
    // `split == bytes.len()` takes everything, so there is no boundary to land on -- and indexing
    // there would be out of bounds.
    while split > 0 && split < bytes.len() && (bytes[split] & 0b1100_0000) == 0b1000_0000 {
        split -= 1;
    }
    split
}

#[derive(Debug)]
struct BackgroundCommand {
    sequence: u64,
    session_id: String,
    command: String,
    output: Mutex<OutputBuffer>,
    status: Mutex<BackgroundStatus>,
    kill_requested: Arc<AtomicBool>,
}

impl BackgroundCommand {
    fn status(&self) -> BackgroundStatus {
        self.status
            .lock()
            .map_or(BackgroundStatus::Running, |status| *status)
    }

    fn set_status(&self, next: BackgroundStatus) {
        if let Ok(mut status) = self.status.lock() {
            *status = next;
        }
    }
}

#[derive(Debug)]
pub(crate) struct BackgroundShellRegistry {
    commands: Mutex<HashMap<String, Arc<BackgroundCommand>>>,
    sequence: AtomicU64,
    /// Injectable so the lifetime bound is exercised by a test that finishes in milliseconds
    /// rather than being asserted only by reading the supervisor loop.
    lifetime: Duration,
}

impl Default for BackgroundShellRegistry {
    fn default() -> Self {
        Self {
            commands: Mutex::new(HashMap::new()),
            sequence: AtomicU64::new(0),
            lifetime: MAX_BACKGROUND_LIFETIME,
        }
    }
}

pub(crate) fn registry() -> &'static BackgroundShellRegistry {
    static REGISTRY: OnceLock<BackgroundShellRegistry> = OnceLock::new();
    REGISTRY.get_or_init(BackgroundShellRegistry::default)
}

impl BackgroundShellRegistry {
    pub(crate) fn start(
        &self,
        session_id: &str,
        command: &str,
        workspace_folder: &str,
    ) -> Result<String, BackgroundStartError> {
        self.prune_terminal(session_id);
        if self.running_count(session_id) >= MAX_BACKGROUND_COMMANDS_PER_SESSION {
            return Err(BackgroundStartError::SessionLimitReached);
        }

        let (executable, flag) = shell_invocation();
        let args = vec![flag.to_owned(), command.to_owned()];
        audit_command("agent_runtime.tool.shell.background", executable, &args);

        let mut child = ManagedChild::spawn_in(
            executable,
            &args,
            &BTreeMap::new(),
            Some(Path::new(workspace_folder)),
        )
        .map_err(|_| BackgroundStartError::Spawn)?;
        let stdout = child
            .take_stdout()
            .map_err(|_| BackgroundStartError::Spawn)?;
        let stderr = child
            .take_stderr()
            .map_err(|_| BackgroundStartError::Spawn)?;

        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let handle = format!("bg_{sequence}");
        let entry = Arc::new(BackgroundCommand {
            sequence,
            session_id: session_id.to_owned(),
            command: command.to_owned(),
            output: Mutex::new(OutputBuffer::default()),
            status: Mutex::new(BackgroundStatus::Running),
            kill_requested: Arc::new(AtomicBool::new(false)),
        });
        match self.commands.lock() {
            Ok(mut commands) => {
                commands.insert(handle.clone(), entry.clone());
            }
            Err(_) => return Err(BackgroundStartError::Spawn),
        }

        spawn_reader(stdout, entry.clone());
        spawn_reader(stderr, entry.clone());
        spawn_supervisor(child, entry, self.lifetime);
        Ok(handle)
    }

    #[cfg(test)]
    fn with_lifetime(lifetime: Duration) -> Self {
        Self {
            lifetime,
            ..Self::default()
        }
    }

    pub(crate) fn take_output(
        &self,
        session_id: &str,
        handle: &str,
    ) -> Result<BackgroundOutput, UnknownHandle> {
        let entry = self.lookup(session_id, handle)?;
        let mut buffer = entry.output.lock().map_err(|_| UnknownHandle)?;
        let (text, dropped_bytes, remaining_bytes) = buffer.take_front(MAX_TOOL_OUTPUT_BYTES);
        Ok(BackgroundOutput {
            text,
            dropped_bytes,
            remaining_bytes,
            status: entry.status(),
        })
    }

    /// Requests termination and waits, bounded, for the supervisor to settle the status. Waiting
    /// is what makes the tool's answer definite: an immediate "termination requested" would leave
    /// the model unable to distinguish a killed process from one that ignored the request.
    pub(crate) fn kill(
        &self,
        session_id: &str,
        handle: &str,
    ) -> Result<KillOutcome, UnknownHandle> {
        let entry = self.lookup(session_id, handle)?;
        let existing = entry.status();
        if existing.is_terminal() {
            return Ok(KillOutcome::AlreadyFinished(existing));
        }
        entry.kill_requested.store(true, Ordering::Release);
        let deadline = Instant::now() + TERMINATION_GRACE * 2;
        while Instant::now() < deadline {
            let status = entry.status();
            if status.is_terminal() {
                return Ok(KillOutcome::Terminated(status));
            }
            thread::sleep(SUPERVISOR_POLL_INTERVAL);
        }
        Ok(KillOutcome::Terminated(entry.status()))
    }

    pub(crate) fn command_label(&self, session_id: &str, handle: &str) -> Option<String> {
        self.lookup(session_id, handle)
            .ok()
            .map(|entry| entry.command.clone())
    }

    /// Terminates and forgets every remaining command, whatever session owns it. Called on
    /// desktop shutdown. On Windows the job object would reap these anyway once the process
    /// handle closed, but an orphaned Unix process group would not -- so this is the guarantee,
    /// not a convenience.
    pub(crate) fn reap_all(&self) {
        let owned: Vec<Arc<BackgroundCommand>> = match self.commands.lock() {
            Ok(mut commands) => commands.drain().map(|(_, entry)| entry).collect(),
            Err(_) => return,
        };
        for entry in owned {
            entry.kill_requested.store(true, Ordering::Release);
        }
    }

    /// Terminates and forgets every command a session owns. Called when the session ends; the
    /// process-group/job-object containment is the backstop if this never runs.
    pub(crate) fn reap_session(&self, session_id: &str) {
        let owned: Vec<Arc<BackgroundCommand>> = match self.commands.lock() {
            Ok(mut commands) => {
                let handles: Vec<String> = commands
                    .iter()
                    .filter(|(_, entry)| entry.session_id == session_id)
                    .map(|(handle, _)| handle.clone())
                    .collect();
                handles
                    .into_iter()
                    .filter_map(|handle| commands.remove(&handle))
                    .collect()
            }
            Err(_) => return,
        };
        for entry in owned {
            entry.kill_requested.store(true, Ordering::Release);
        }
    }

    fn lookup(
        &self,
        session_id: &str,
        handle: &str,
    ) -> Result<Arc<BackgroundCommand>, UnknownHandle> {
        let commands = self.commands.lock().map_err(|_| UnknownHandle)?;
        commands
            .get(handle)
            .filter(|entry| entry.session_id == session_id)
            .cloned()
            .ok_or(UnknownHandle)
    }

    fn running_count(&self, session_id: &str) -> usize {
        self.commands.lock().map_or(0, |commands| {
            commands
                .values()
                .filter(|entry| entry.session_id == session_id && !entry.status().is_terminal())
                .count()
        })
    }

    fn prune_terminal(&self, session_id: &str) {
        let Ok(mut commands) = self.commands.lock() else {
            return;
        };
        let mut terminal: Vec<(u64, String)> = commands
            .iter()
            .filter(|(_, entry)| entry.session_id == session_id && entry.status().is_terminal())
            .map(|(handle, entry)| (entry.sequence, handle.clone()))
            .collect();
        if terminal.len() <= MAX_RETAINED_TERMINAL_PER_SESSION {
            return;
        }
        terminal.sort_unstable();
        let excess = terminal.len() - MAX_RETAINED_TERMINAL_PER_SESSION;
        for (_, handle) in terminal.into_iter().take(excess) {
            commands.remove(&handle);
        }
    }

    #[cfg(test)]
    fn entry_count(&self) -> usize {
        self.commands.lock().map_or(0, |commands| commands.len())
    }
}

fn spawn_reader<R: Read + Send + 'static>(mut source: R, entry: Arc<BackgroundCommand>) {
    thread::spawn(move || {
        let mut chunk = vec![0_u8; READ_CHUNK_BYTES];
        loop {
            match source.read(&mut chunk) {
                Ok(0) | Err(_) => return,
                Ok(read) => {
                    if let Ok(mut buffer) = entry.output.lock() {
                        buffer.append(&chunk[..read]);
                    }
                }
            }
        }
    });
}

/// Owns the `ManagedChild` outright rather than sharing it behind a mutex. `shell_kill` signals
/// through an atomic flag instead, so a caller can never block on a supervisor that is itself
/// parked inside `wait_until`.
fn spawn_supervisor(mut child: ManagedChild, entry: Arc<BackgroundCommand>, lifetime: Duration) {
    thread::spawn(move || {
        let lifetime_deadline = Instant::now() + lifetime;
        loop {
            if entry.kill_requested.load(Ordering::Acquire) {
                let _ = child.shutdown(Instant::now() + TERMINATION_GRACE);
                entry.set_status(BackgroundStatus::Killed);
                return;
            }
            if Instant::now() >= lifetime_deadline {
                let _ = child.shutdown(Instant::now() + TERMINATION_GRACE);
                entry.set_status(BackgroundStatus::LifetimeExceeded);
                return;
            }
            match child.wait_until(Instant::now() + SUPERVISOR_POLL_INTERVAL) {
                Ok(Some(status)) => {
                    entry.set_status(BackgroundStatus::Exited(status.code()));
                    return;
                }
                Ok(None) => continue,
                Err(_) => {
                    entry.set_status(BackgroundStatus::Exited(None));
                    return;
                }
            }
        }
    });
}

#[cfg(test)]
#[path = "background_shell_tests.rs"]
mod tests;
