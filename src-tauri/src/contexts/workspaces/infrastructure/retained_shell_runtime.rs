//! The local PTY half of a retained Session Shell.
//!
//! Two rules govern this file, and both are about ownership rather than about terminals.
//!
//! The map of live shells is never held across a blocking call — a write, a resize, a close, a
//! worker handoff — because a shell whose child stopped draining its pipe would otherwise stall
//! every other shell in the application.
//!
//! And every operating-system resource has exactly one owner from the instant it exists. Startup
//! acquires five of them in sequence — a PTY pair, a child, a reader, a writer, two threads — and
//! any of the five can fail. A `?` there returns to a caller that never saw the child, which is a
//! live process with nobody left to kill it. So startup builds a guard that owns each piece as it
//! is acquired and either commits the whole set or ends it; and close runs a staged, bounded
//! sequence that hands what it cannot confirm back to the Reaper instead of dropping it.

use super::retained_shell_process::{
    close_process_bounded, CloseObservations, MonotonicDeadlineClock, ShellDeadlineClock,
    ShellProcessError, ShellProcessHandle, ShellPtyHandle, ShellWorker,
};
use crate::contexts::workspaces::application::{
    SessionShellRuntimePort, ShellOutputSink, ShellRuntimeCloseOutcome, ShellRuntimeOpen,
    ShellRuntimeOpened,
};
use crate::contexts::workspaces::domain::{
    shell_reason, SessionShellError, SessionShellState, ShellCloseBudget,
    ShellForegroundProcessState, ShellGeneration, ShellId, ShellRuntimeDescriptor, ShellStream,
    TerminalDimensions,
};
use crate::platform::filesystem::normalize_windows_extended_length_path;
use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Larger reads coalesce bursty output into fewer frames without adding latency: a read returns as
/// soon as any bytes are available, so interactive echo is unaffected.
const SHELL_READ_BUFFER_BYTES: usize = 64 * 1024;

/// How often the exit monitor asks whether the child is still there.
const EXIT_POLL_MILLIS: u64 = 100;

/// The blocking halves of one shell, checked out of the map before use.
///
/// The writer is an `Option` so closing input can take it and drop it. Dropping the write end is
/// what an interactive shell sees as end-of-input, and it is the one termination step that lets a
/// shell finish on its own terms rather than being killed.
struct ShellIo {
    master: Arc<dyn ShellPtyHandle>,
    writer: Mutex<Option<Box<dyn Write + Send>>>,
}

/// One live shell as the runtime owns it.
struct RetainedShell {
    generation: ShellGeneration,
    io: Arc<ShellIo>,
    process: Arc<dyn ShellProcessHandle>,
    /// Set before the child is signalled so the exit monitor stops reporting a requested close as
    /// a spontaneous exit.
    closing: Arc<AtomicBool>,
    workers: Vec<Arc<ShellWorker>>,
}

/// The handles one close attempt works with, taken while the map is locked and used after it is
/// released. Named rather than a tuple because the close sequence reads them by role.
struct CloseCheckout {
    io: Arc<ShellIo>,
    process: Arc<dyn ShellProcessHandle>,
    closing: Arc<AtomicBool>,
    workers: Vec<Arc<ShellWorker>>,
}

/// One shell, one PTY, retained until something confirms it is gone.
pub(crate) struct RetainedLocalShellRuntime {
    shells: Mutex<HashMap<String, RetainedShell>>,
    clock: Arc<dyn ShellDeadlineClock>,
    observations: CloseObservations,
}

impl Default for RetainedLocalShellRuntime {
    fn default() -> Self {
        Self {
            shells: Mutex::new(HashMap::new()),
            clock: Arc::new(MonotonicDeadlineClock::default()),
            observations: CloseObservations::default(),
        }
    }
}

impl RetainedLocalShellRuntime {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, RetainedShell>> {
        match self.shells.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Takes the blocking halves out of the map so the call that uses them runs unlocked.
    fn checkout(&self, shell_id: &ShellId) -> Result<Arc<ShellIo>, SessionShellError> {
        self.lock()
            .get(shell_id.as_str())
            .map(|shell| shell.io.clone())
            .ok_or(SessionShellError::NotFound)
    }

    /// Everything a close needs, cloned out of the map so the sequence itself runs unlocked.
    ///
    /// Nothing is removed here. The entry stays until a close is *confirmed*, which is what makes a
    /// timed-out close retryable at the same id instead of a Shell nobody can reach.
    fn checkout_for_close(
        &self,
        shell_id: &ShellId,
        generation: ShellGeneration,
    ) -> Option<CloseCheckout> {
        let shells = self.lock();
        let shell = shells.get(shell_id.as_str())?;
        if shell.generation != generation {
            return None;
        }
        Some(CloseCheckout {
            io: shell.io.clone(),
            process: shell.process.clone(),
            closing: shell.closing.clone(),
            workers: shell.workers.clone(),
        })
    }

    #[cfg(test)]
    pub(super) fn with_clock(clock: Arc<dyn ShellDeadlineClock>) -> Self {
        Self {
            shells: Mutex::new(HashMap::new()),
            clock,
            observations: CloseObservations::default(),
        }
    }

    #[cfg(test)]
    pub(super) fn observations(&self) -> &CloseObservations {
        &self.observations
    }

    #[cfg(test)]
    pub(super) fn holds(&self, shell_id: &ShellId) -> bool {
        self.lock().contains_key(shell_id.as_str())
    }

    /// Installs a shell built from test doubles, so the close sequence can be driven against a
    /// child that never dies without needing a real one.
    #[cfg(test)]
    pub(super) fn install_for_tests(
        &self,
        shell_id: &ShellId,
        generation: ShellGeneration,
        process: Arc<dyn ShellProcessHandle>,
        master: Arc<dyn ShellPtyHandle>,
        workers: Vec<Arc<ShellWorker>>,
    ) {
        self.lock().insert(
            shell_id.as_str().to_string(),
            RetainedShell {
                generation,
                io: Arc::new(ShellIo {
                    master,
                    writer: Mutex::new(None),
                }),
                process,
                closing: Arc::new(AtomicBool::new(false)),
                workers,
            },
        );
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

fn runtime_error(reason: &str) -> SessionShellError {
    SessionShellError::Runtime {
        reason: shell_reason(reason),
    }
}

fn unavailable(reason: &str) -> SessionShellError {
    SessionShellError::RuntimeUnavailable {
        reason: shell_reason(reason),
    }
}

/// The portable-pty child, behind the two questions a bounded close asks.
///
/// The killer is kept separately from the child because that is how portable-pty hands out the
/// ability to signal: the child itself is behind the same lock the exit monitor polls, and a close
/// that had to take that lock to kill would queue behind the monitor's own observation.
struct PortablePtyProcess {
    child: Mutex<Box<dyn Child + Send + Sync>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
}

impl ShellProcessHandle for PortablePtyProcess {
    fn try_reap(&self) -> Result<Option<i32>, ShellProcessError> {
        let mut child = match self.child.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        match child.try_wait() {
            Ok(Some(status)) => Ok(Some(status.exit_code() as i32)),
            Ok(None) => Ok(None),
            Err(_) => Err(ShellProcessError::Observe),
        }
    }

    fn terminate(&self) -> Result<(), ShellProcessError> {
        let mut killer = match self.killer.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        killer.kill().map_err(|_| ShellProcessError::Terminate)
    }
}

struct PortablePtyMaster(Mutex<Box<dyn MasterPty + Send>>);

impl ShellPtyHandle for PortablePtyMaster {
    fn resize(&self, dimensions: TerminalDimensions) -> Result<(), ()> {
        let master = match self.0.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        master.resize(terminal_size(dimensions)).map_err(|_| ())
    }
}

/// Owns everything a startup has acquired, until the whole set is committed.
///
/// The failure this exists for is not exotic: `try_clone_reader` can fail on a handle the operating
/// system has already reclaimed, and a thread spawn fails whenever the process is near its limit.
/// Either one used to return through a `?` that had no idea a child was running. Now the child is
/// inside the guard from the moment it exists, and a guard that falls out of scope without being
/// committed terminates what it holds.
struct LocalLaunchGuard {
    process: Option<Arc<dyn ShellProcessHandle>>,
    master: Option<Arc<dyn ShellPtyHandle>>,
    writer: Option<Box<dyn Write + Send>>,
    workers: Vec<Arc<ShellWorker>>,
    closing: Arc<AtomicBool>,
    committed: bool,
}

impl LocalLaunchGuard {
    fn new() -> Self {
        Self {
            process: None,
            master: None,
            writer: None,
            workers: Vec::new(),
            closing: Arc::new(AtomicBool::new(false)),
            committed: false,
        }
    }

    /// Hands the whole acquired set over as one retained shell. The only path that disarms the
    /// rollback, and it is unreachable unless every piece is present.
    fn commit(mut self, generation: ShellGeneration) -> Option<RetainedShell> {
        let (process, master) = (self.process.take()?, self.master.take()?);
        self.committed = true;
        Some(RetainedShell {
            generation,
            io: Arc::new(ShellIo {
                master,
                writer: Mutex::new(self.writer.take()),
            }),
            process,
            closing: self.closing.clone(),
            workers: std::mem::take(&mut self.workers),
        })
    }
}

impl Drop for LocalLaunchGuard {
    /// Ends what the failed startup started.
    ///
    /// Bounded like every other termination here, and best-effort by construction: a child that
    /// outlives this window has still been signalled, and the caller reports a startup whose
    /// cleanup is pending rather than a startup that quietly leaked. Silence would be the one
    /// unacceptable outcome, and it is the one this replaces.
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        self.closing.store(true, Ordering::SeqCst);
        // Input first: a shell that has just been started and immediately loses stdin usually ends
        // by itself, which is a cleaner ending than a kill and costs one dropped handle.
        self.writer.take();
        let Some(process) = self.process.take() else {
            return;
        };
        let clock = MonotonicDeadlineClock::default();
        let observations = CloseObservations::default();
        let _outcome = close_process_bounded(
            process.as_ref(),
            &self.workers,
            ShellCloseBudget::default(),
            &clock,
            &observations,
        );
    }
}

impl RetainedLocalShellRuntime {
    /// Acquires the PTY, the child, and the streams, in the order in which each becomes the next
    /// one's precondition, with the guard taking ownership at every step.
    fn launch(
        &self,
        request: &ShellRuntimeOpen,
        guard: &mut LocalLaunchGuard,
    ) -> Result<Box<dyn Read + Send>, SessionShellError> {
        let root = PathBuf::from(normalize_windows_extended_length_path(&request.root));
        let pair = native_pty_system()
            .openpty(terminal_size(request.dimensions))
            .map_err(|_| unavailable("shell_pty_unavailable"))?;
        let mut command = CommandBuilder::new(default_shell());
        command.cwd(&root);
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|_| unavailable("shell_process_launch_failed"))?;
        drop(pair.slave);
        // From here the guard owns the child. Every `?` below unwinds through its `Drop`.
        let killer = child.clone_killer();
        guard.process = Some(Arc::new(PortablePtyProcess {
            child: Mutex::new(child),
            killer: Mutex::new(killer),
        }));

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|_| unavailable("shell_reader_unavailable"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|_| unavailable("shell_writer_unavailable"))?;
        guard.writer = Some(writer);
        guard.master = Some(Arc::new(PortablePtyMaster(Mutex::new(pair.master))));
        Ok(reader)
    }

    /// Starts the reader and the exit monitor, both stamped with the generation they belong to.
    fn start_workers(
        &self,
        request: &ShellRuntimeOpen,
        sink: Arc<dyn ShellOutputSink>,
        mut reader: Box<dyn Read + Send>,
        guard: &mut LocalLaunchGuard,
    ) -> Result<(), SessionShellError> {
        let Some(process) = guard.process.clone() else {
            return Err(unavailable("shell_process_launch_failed"));
        };
        let reader_shell = request.shell_id.clone();
        let reader_generation = request.generation;
        let reader_sink = sink.clone();
        guard.workers.push(Arc::new(ShellWorker::spawn(
            format!("vanehub-session-shell-{}", request.shell_id.as_str()),
            move || loop {
                let mut buffer = [0u8; SHELL_READ_BUFFER_BYTES];
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    // Bytes go to the sink undecoded: splitting a UTF-8 sequence across two reads
                    // is normal, and only the retained buffer knows what it is still waiting on.
                    Ok(count) => reader_sink.on_output(
                        &reader_shell,
                        reader_generation,
                        ShellStream::Pty,
                        &buffer[..count],
                    ),
                    Err(_) => break,
                }
            },
        )?));

        let monitor_shell = request.shell_id.clone();
        let monitor_generation = request.generation;
        let monitor_closing = guard.closing.clone();
        guard.workers.push(Arc::new(ShellWorker::spawn(
            format!("vanehub-session-shell-exit-{}", request.shell_id.as_str()),
            move || {
                let code = loop {
                    match process.try_reap() {
                        Ok(Some(code)) => break Some(code),
                        Ok(None) => {
                            std::thread::sleep(std::time::Duration::from_millis(EXIT_POLL_MILLIS))
                        }
                        Err(_) => break None,
                    }
                };
                // A close already owns the ending. Reporting again would end one Shell twice, and
                // would report a requested close as a spontaneous exit.
                if monitor_closing.load(Ordering::SeqCst) {
                    return;
                }
                sink.on_state(
                    &monitor_shell,
                    monitor_generation,
                    SessionShellState::Exited { code },
                );
            },
        )?));
        Ok(())
    }
}

impl SessionShellRuntimePort for RetainedLocalShellRuntime {
    fn open(
        &self,
        request: &ShellRuntimeOpen,
        sink: Arc<dyn ShellOutputSink>,
    ) -> Result<ShellRuntimeOpened, SessionShellError> {
        // Remote shells belong to the SSH adapter. Opening a local PTY at a remote path would open
        // a shell on this machine and label it remote, which is worse than refusing.
        if request.remote.is_some() {
            return Err(unavailable("shell_remote_not_supported_locally"));
        }
        let mut guard = LocalLaunchGuard::new();
        let reader = self.launch(request, &mut guard)?;
        self.start_workers(request, sink, reader, &mut guard)?;
        let Some(shell) = guard.commit(request.generation) else {
            return Err(unavailable("shell_open_setup_failed"));
        };
        // The store already holds this Shell as `Opening`, so the workers above could publish into
        // it from their first byte. Inserting here transfers ownership of the handles, not the
        // right to be heard from.
        self.lock()
            .insert(request.shell_id.as_str().to_string(), shell);
        Ok(ShellRuntimeOpened {
            runtime: ShellRuntimeDescriptor::Native,
            state: SessionShellState::Running,
        })
    }

    fn write(&self, shell_id: &ShellId, content: &str) -> Result<(), SessionShellError> {
        let io = self.checkout(shell_id)?;
        let mut writer = match io.writer.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Absent once close has taken it. A write accepted after input was closed would report a
        // keystroke delivered to a shell that is being ended.
        let Some(writer) = writer.as_mut() else {
            return Err(runtime_error("shell_write_failed"));
        };
        writer
            .write_all(content.as_bytes())
            .and_then(|()| writer.flush())
            .map_err(|_| runtime_error("shell_write_failed"))
    }

    fn resize(
        &self,
        shell_id: &ShellId,
        dimensions: TerminalDimensions,
    ) -> Result<(), SessionShellError> {
        let io = self.checkout(shell_id)?;
        io.master
            .resize(dimensions)
            .map_err(|()| runtime_error("shell_resize_failed"))
    }

    /// Ends one shell within a finite budget, and gives up the entry only on confirmation.
    ///
    /// The entry is deliberately not removed first. Removing it and then killing is what makes a
    /// failed close unrecoverable: the handles are gone from the map, so a retry has nothing to
    /// retry, and the process it could not kill has no owner left.
    fn close(
        &self,
        shell_id: &ShellId,
        generation: ShellGeneration,
        budget: ShellCloseBudget,
    ) -> ShellRuntimeCloseOutcome {
        let Some(checkout) = self.checkout_for_close(shell_id, generation) else {
            return ShellRuntimeCloseOutcome::NotHeld;
        };
        checkout.closing.store(true, Ordering::SeqCst);
        // Stop input before signalling anything: a shell that loses stdin usually finishes on its
        // own, and an orderly exit beats a kill for the same result.
        {
            let mut writer = match checkout.io.writer.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            writer.take();
        }
        let outcome = close_process_bounded(
            checkout.process.as_ref(),
            &checkout.workers,
            budget,
            self.clock.as_ref(),
            &self.observations,
        );
        if matches!(outcome, ShellRuntimeCloseOutcome::Confirmed) {
            // Removed under the same generation check that let us in, so a close racing a newer
            // open cannot evict the newer shell.
            let mut shells = self.lock();
            if shells
                .get(shell_id.as_str())
                .is_some_and(|shell| shell.generation == generation)
            {
                shells.remove(shell_id.as_str());
            }
        }
        outcome
    }

    /// A local PTY exposes no reliable foreground marker, and guessing one from terminal text would
    /// be parsing output to invent a fact.
    fn foreground_process(&self, shell_id: &ShellId) -> ShellForegroundProcessState {
        if self.lock().contains_key(shell_id.as_str()) {
            ShellForegroundProcessState::Unknown
        } else {
            ShellForegroundProcessState::Absent
        }
    }
}
