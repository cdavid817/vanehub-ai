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
    SessionShellRuntimePort, ShellLifecycleDiagnosticsPort, ShellOutputSink,
    ShellRuntimeCloseOutcome, ShellRuntimeOpen, ShellRuntimeOpened,
};
use crate::contexts::workspaces::domain::{
    shell_reason, shell_reason_code, SessionShellError, SessionShellState, ShellCloseBudget,
    ShellForegroundProcessState, ShellGeneration, ShellId, ShellRuntimeDescriptor, ShellStream,
    TerminalDimensions,
};
use crate::platform::filesystem::normalize_windows_extended_length_path;
use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
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
    /// Where a startup rollback that could not confirm cleanup is recorded.
    diagnostics: Arc<dyn ShellLifecycleDiagnosticsPort>,
    /// How a local terminal is acquired. A seam, so the failures between spawning a child and
    /// owning its streams can be staged rather than only reasoned about.
    pty: Arc<dyn LocalPtyFactory>,
}

impl RetainedLocalShellRuntime {
    pub(crate) fn new(diagnostics: Arc<dyn ShellLifecycleDiagnosticsPort>) -> Self {
        Self {
            shells: Mutex::new(HashMap::new()),
            clock: Arc::new(MonotonicDeadlineClock::default()),
            observations: CloseObservations::default(),
            diagnostics,
            pty: Arc::new(PortablePtyFactory),
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
            diagnostics: Arc::new(DiscardedDiagnostics),
            pty: Arc::new(PortablePtyFactory),
        }
    }

    /// A diagnostics sink for the tests that are not about diagnostics.
    ///
    /// The recording double lives with the application tests; these exercise a real PTY, and what
    /// they need from this port is that it exists rather than what it received.
    #[cfg(test)]
    pub(super) fn for_test() -> Self {
        Self {
            shells: Mutex::new(HashMap::new()),
            clock: Arc::new(MonotonicDeadlineClock::default()),
            observations: CloseObservations::default(),
            diagnostics: Arc::new(DiscardedDiagnostics),
            pty: Arc::new(PortablePtyFactory),
        }
    }

    /// A runtime whose terminal acquisition is staged by the test.
    #[cfg(test)]
    pub(super) fn with_pty(
        pty: Arc<dyn LocalPtyFactory>,
        diagnostics: Arc<dyn ShellLifecycleDiagnosticsPort>,
    ) -> Self {
        Self {
            shells: Mutex::new(HashMap::new()),
            clock: Arc::new(MonotonicDeadlineClock::default()),
            observations: CloseObservations::default(),
            diagnostics,
            pty,
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
/// Runs the guard's rollback now and says which failure the caller is looking at.
///
/// Dropping the guard would do the same cleanup and report the same error either way, which is what
/// hid the distinction. What the caller needs is whether anything may still be running: a startup
/// that cleaned up after itself is a slot to return, and one that could not is a slot to keep and a
/// child to hand to the Reaper.
fn rollback_error(guard: LocalLaunchGuard, error: SessionShellError) -> SessionShellError {
    if guard.roll_back() {
        return error;
    }
    runtime_error(shell_reason_code::STARTUP_CLEANUP_PENDING)
}

/// Swallows what it is told, for tests whose subject is elsewhere.
#[cfg(test)]
pub(super) struct DiscardedDiagnostics;

#[cfg(test)]
impl ShellLifecycleDiagnosticsPort for DiscardedDiagnostics {
    fn stale_reaper_completion(&self, _shell_id: &str, _attempted: u64, _current: u64) {}
    fn orphaned_reaper_completion(&self, _shell_id: &str, _attempted: u64) {}
    fn startup_rollback_unconfirmed(&self, _shell_id: &str, _generation: u64, _reason: &str) {}
}

struct LocalLaunchGuard {
    process: Option<Arc<dyn ShellProcessHandle>>,
    master: Option<Arc<dyn ShellPtyHandle>>,
    writer: Option<Box<dyn Write + Send>>,
    workers: Vec<Arc<ShellWorker>>,
    closing: Arc<AtomicBool>,
    committed: bool,
    /// Whether the rollback confirmed the child had ended.
    ///
    /// `true` until a rollback says otherwise, because a guard that never acquired anything has
    /// nothing outstanding — and a `false` default would report every clean startup as one that
    /// left something behind.
    cleanup_confirmed: bool,
    /// Which Shell this startup is for, so a rollback that could not confirm can name it.
    shell_id: String,
    generation: ShellGeneration,
    diagnostics: Arc<dyn ShellLifecycleDiagnosticsPort>,
}

impl LocalLaunchGuard {
    fn new(
        shell_id: String,
        generation: ShellGeneration,
        diagnostics: Arc<dyn ShellLifecycleDiagnosticsPort>,
    ) -> Self {
        Self {
            process: None,
            master: None,
            writer: None,
            workers: Vec::new(),
            closing: Arc::new(AtomicBool::new(false)),
            committed: false,
            cleanup_confirmed: true,
            shell_id,
            generation,
            diagnostics,
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

impl LocalLaunchGuard {
    /// Ends what the failed startup started, and says whether it confirmed.
    ///
    /// Consuming, so a guard cannot be rolled back twice: `Drop` sees `committed` and returns.
    /// `true` means nothing is left running.
    fn roll_back(mut self) -> bool {
        self.terminate_acquired();
        // Disarms `Drop`. The work is done and the outcome has been reported; running it again on
        // the way out would signal a child that is either already gone or already recorded.
        self.committed = true;
        self.cleanup_confirmed
    }

    /// The termination itself, shared by the explicit rollback and the `Drop` backstop.
    ///
    /// Records its answer in `cleanup_confirmed` rather than returning it, because one of its two
    /// callers is a destructor with nobody to answer. A returned value there could only be
    /// discarded, and a discarded lifecycle result is how a child ends up with no owner and no
    /// trace — the exact shape this whole change exists to remove.
    fn terminate_acquired(&mut self) {
        self.closing.store(true, Ordering::SeqCst);
        // Input first: a shell that has just been started and immediately loses stdin usually ends
        // by itself, which is a cleaner ending than a kill and costs one dropped handle.
        self.writer.take();
        let Some(process) = self.process.take() else {
            // Nothing was acquired, so nothing is left running.
            self.cleanup_confirmed = true;
            return;
        };
        let clock = MonotonicDeadlineClock::default();
        let observations = CloseObservations::default();
        let outcome = close_process_bounded(
            process.as_ref(),
            &self.workers,
            ShellCloseBudget::default(),
            &clock,
            &observations,
        );
        // Recorded rather than discarded. The rollback is best-effort by construction — blocking
        // here would hold the create path open on a child refusing to die — and the outcome is the
        // only evidence that a child outlived its own startup.
        if !outcome.is_released() {
            self.diagnostics.startup_rollback_unconfirmed(
                &self.shell_id,
                self.generation.value(),
                shell_reason_code::STARTUP_CLEANUP_PENDING,
            );
            self.cleanup_confirmed = false;
            return;
        }
        self.cleanup_confirmed = true;
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
        // The backstop, for the paths that do not roll back explicitly — a panic, or a future
        // branch somebody adds without one. The ordinary failure goes through `roll_back`, which
        // does the same work and reads the answer it left behind. Nothing is discarded here: an
        // unconfirmed cleanup has already been reported by the time this returns.
        self.terminate_acquired();
    }
}

/// Acquiring a local terminal, one step at a time.
///
/// Stepwise rather than all-at-once, and that is the whole reason it exists. The failure this shape
/// makes reachable is the one that matters: the child is spawned and *then* the reader cannot be
/// cloned, so a live process exists with nothing owning it yet. A factory that returned a finished
/// terminal could not express that, and a fake built on one would prove only the happy path.
///
/// Each method is called once, in the order declared, and each is the next one's precondition.
pub(super) trait LocalPtySession: Send {
    /// Starts the shell. From here something is running.
    fn spawn(&mut self, root: &Path) -> Result<Arc<dyn ShellProcessHandle>, SessionShellError>;
    fn reader(&mut self) -> Result<Box<dyn Read + Send>, SessionShellError>;
    fn writer(&mut self) -> Result<Box<dyn Write + Send>, SessionShellError>;
    /// Hands over the master, which is what a resize goes through.
    fn master(&mut self) -> Result<Arc<dyn ShellPtyHandle>, SessionShellError>;
}

pub(super) trait LocalPtyFactory: Send + Sync {
    fn open(
        &self,
        dimensions: TerminalDimensions,
    ) -> Result<Box<dyn LocalPtySession>, SessionShellError>;
}

/// The real one, over `portable-pty`.
struct PortablePtyFactory;

impl LocalPtyFactory for PortablePtyFactory {
    fn open(
        &self,
        dimensions: TerminalDimensions,
    ) -> Result<Box<dyn LocalPtySession>, SessionShellError> {
        let pair = native_pty_system()
            .openpty(terminal_size(dimensions))
            .map_err(|_| unavailable("shell_pty_unavailable"))?;
        Ok(Box::new(PortablePtySession {
            slave: Some(pair.slave),
            master: Some(pair.master),
        }))
    }
}

struct PortablePtySession {
    slave: Option<Box<dyn portable_pty::SlavePty + Send>>,
    master: Option<Box<dyn portable_pty::MasterPty + Send>>,
}

impl LocalPtySession for PortablePtySession {
    fn spawn(&mut self, root: &Path) -> Result<Arc<dyn ShellProcessHandle>, SessionShellError> {
        let Some(slave) = self.slave.take() else {
            return Err(unavailable("shell_process_launch_failed"));
        };
        let mut command = CommandBuilder::new(default_shell());
        command.cwd(root);
        let child = slave
            .spawn_command(command)
            .map_err(|_| unavailable("shell_process_launch_failed"))?;
        // Dropped as soon as the child holds it. A slave kept open here would stop the master's
        // reader ever seeing EOF, so a shell that exited would look like one still running.
        drop(slave);
        let killer = child.clone_killer();
        Ok(Arc::new(PortablePtyProcess {
            child: Mutex::new(child),
            killer: Mutex::new(killer),
        }))
    }

    fn reader(&mut self) -> Result<Box<dyn Read + Send>, SessionShellError> {
        self.master
            .as_ref()
            .ok_or_else(|| unavailable("shell_reader_unavailable"))?
            .try_clone_reader()
            .map_err(|_| unavailable("shell_reader_unavailable"))
    }

    fn writer(&mut self) -> Result<Box<dyn Write + Send>, SessionShellError> {
        self.master
            .as_ref()
            .ok_or_else(|| unavailable("shell_writer_unavailable"))?
            .take_writer()
            .map_err(|_| unavailable("shell_writer_unavailable"))
    }

    fn master(&mut self) -> Result<Arc<dyn ShellPtyHandle>, SessionShellError> {
        let Some(master) = self.master.take() else {
            return Err(unavailable("shell_pty_unavailable"));
        };
        Ok(Arc::new(PortablePtyMaster(Mutex::new(master))))
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
        let mut session = self.pty.open(request.dimensions)?;
        // From here the guard owns the child. Every `?` below unwinds through its rollback.
        guard.process = Some(session.spawn(&root)?);
        let reader = session.reader()?;
        guard.writer = Some(session.writer()?);
        guard.master = Some(session.master()?);
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
        let mut guard = LocalLaunchGuard::new(
            request.shell_id.as_str().to_string(),
            request.generation,
            self.diagnostics.clone(),
        );
        // Rolled back explicitly rather than by unwinding, because the *caller* has to know which
        // of the two failures this was. A `?` here would pick the error before the guard ran, and
        // the registry would finalize a Shell whose child may still be alive — returning its slot
        // while the process holds a thread, which is the defect one step earlier.
        if let Err(error) = self
            .launch(request, &mut guard)
            .and_then(|reader| self.start_workers(request, sink, reader, &mut guard))
        {
            return Err(rollback_error(guard, error));
        }
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
