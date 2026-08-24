use crate::contexts::workspaces::application::{
    ShellEvent, ShellLaunch, WorkspaceApplicationError as AppError, WorkspaceLogLevel,
    WorkspaceShellEventPort, WorkspaceShellLogPort, WorkspaceShellRuntimePort,
};
use crate::contexts::workspaces::domain::{reset_directory_command, ShellHost, TerminalDimensions};
use crate::platform::filesystem::normalize_windows_extended_length_path;
use crate::platform::text::take_decodable_utf8;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use super::shell_termination::{
    log_termination, poll_until_exit, probe_shared_child, reap_shared_child, requires_adoption,
    settled_cleanup, write_shell_log, PendingReapRegistry, PollEnd, ShellTermination,
    ShutdownToken, TerminationOutcome, TerminationReport, MONITOR_INTERVAL_CEILING, REAP_DEADLINE,
    SHUTDOWN_REAP_DEADLINE,
};

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

/// Larger reads coalesce bursty PTY output into fewer IPC events without adding latency:
/// a read still returns as soon as any bytes are available, so interactive echo is
/// unaffected, while a flood of build output emits far fewer events than a 4 KiB buffer.
/// Matches the agent terminal's read width.
const SHELL_READ_BUFFER_BYTES: usize = 64 * 1024;

/// Everything the runtime owns for as long as any handle to it exists.
///
/// The manager used to be a bag of `Arc`s cloned by value, with a `Drop` that checked
/// `Arc::strong_count` and bailed out unless it happened to be the last one. That check was the
/// wrong shape twice over: monitor threads held clones of the same map, so the count was almost
/// never one, and a manager that "owns" its shells only when it can prove it is alone does not
/// really own them. Ownership is expressed here instead -- the handle is a thin
/// `Arc<ManagerCore>`, and `Drop for ManagerCore` runs exactly once, when the last handle goes,
/// with `&mut self` proving it. Monitors deliberately do *not* hold a `ManagerCore`; they hold
/// only the pieces they use, so they can never keep the manager alive.
struct ManagerCore {
    shells: Arc<Mutex<HashMap<String, ManagedShell>>>,
    pending: Arc<PendingReapRegistry>,
    monitors: Mutex<Vec<thread::JoinHandle<()>>>,
    shutdown: Arc<ShutdownToken>,
    events: Arc<dyn WorkspaceShellEventPort>,
    logging: Arc<dyn WorkspaceShellLogPort>,
}

#[derive(Clone)]
pub(crate) struct PortablePtyShellRuntime {
    core: Arc<ManagerCore>,
}

impl PortablePtyShellRuntime {
    pub(crate) fn new(
        events: Arc<dyn WorkspaceShellEventPort>,
        logging: Arc<dyn WorkspaceShellLogPort>,
    ) -> Self {
        Self {
            core: Arc::new(ManagerCore {
                shells: Arc::new(Mutex::new(HashMap::new())),
                pending: Arc::new(PendingReapRegistry::default()),
                monitors: Mutex::new(Vec::new()),
                shutdown: Arc::new(ShutdownToken::default()),
                events,
                logging,
            }),
        }
    }

    fn shells(&self) -> &Mutex<HashMap<String, ManagedShell>> {
        &self.core.shells
    }

    fn logging(&self) -> &dyn WorkspaceShellLogPort {
        self.core.logging.as_ref()
    }
}

/// Ends a registered shell within a bounded deadline, and hands its child to the pending
/// registry if the child outlives the attempt.
///
/// Single-flight: the first caller out of `Idle` is the owner and runs exactly one kill and one
/// reap loop. Every other caller is a follower and returns immediately with no outcome of its
/// own -- it does not queue on the child mutex, and it does not claim a result it did not
/// produce. Once the owner settles, later callers read that one final outcome.
///
/// The adoption is the part that closes the lifecycle: on `ReapTimedOut` or `KillFailed` the
/// child is transferred to `pending` *before* this returns, so the caller can release its
/// `ManagedShell` without dropping the last handle to a live process.
fn terminate_shell(
    shell: &ManagedShell,
    pending: &PendingReapRegistry,
    logging: &dyn WorkspaceShellLogPort,
    shell_id: &str,
    deadline: Instant,
    shutdown: bool,
) -> TerminationReport {
    if let Err(existing) = shell.termination.claim() {
        return existing;
    }
    let outcome = reap_shared_child(&shell.child, &shell.killer, deadline);
    if requires_adoption(outcome) {
        pending.adopt(
            &shell.session_id,
            shell_id,
            shell.child.clone(),
            shell.termination.clone(),
        );
    }
    let cleanup = settled_cleanup(outcome);
    shell.termination.settle(outcome, cleanup);
    let report = TerminationReport {
        outcome: Some(outcome),
        cleanup: shell.termination.cleanup(),
    };
    log_termination(logging, &shell.session_id, shell_id, report, shutdown);
    report
}

impl PortablePtyShellRuntime {
    /// Terminates a child that never made it into the registry, on `open_shell`'s failure
    /// paths. Same route and same ownership transfer as a registered shell — the only
    /// difference is that there is no map entry to remove.
    fn terminate_unregistered(
        &self,
        child: &Arc<Mutex<Box<dyn Child + Send + Sync>>>,
        killer: &Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>,
        termination: &Arc<ShellTermination>,
        session_id: &str,
        shell_id: &str,
    ) -> TerminationOutcome {
        if termination.claim().is_err() {
            return TerminationOutcome::Reaped;
        }
        let outcome = reap_shared_child(child, killer, Instant::now() + REAP_DEADLINE);
        if requires_adoption(outcome) {
            self.core
                .pending
                .adopt(session_id, shell_id, child.clone(), termination.clone());
        }
        termination.settle(outcome, settled_cleanup(outcome));
        log_termination(
            self.logging(),
            session_id,
            shell_id,
            TerminationReport {
                outcome: Some(outcome),
                cleanup: termination.cleanup(),
            },
            false,
        );
        outcome
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

impl PortablePtyShellRuntime {
    fn insert(&self, shell_id: String, shell: ManagedShell) -> Result<(), AppError> {
        // The guard is bound and dropped explicitly rather than left to a temporary's lifetime.
        // Both spellings release the lock before the terminate below, but only one of them says
        // so, and the property here -- no process work under the routing lock -- is one a later
        // edit must not be able to break by accident.
        let replaced = {
            let mut shells = self
                .shells()
                .lock()
                .map_err(|error| AppError::Storage(error.to_string()))?;
            shells.insert(shell_id, shell)
        };
        if let Some(replaced) = replaced {
            terminate_shell(
                &replaced,
                &self.core.pending,
                self.logging(),
                "replaced",
                Instant::now() + REAP_DEADLINE,
                false,
            );
        }
        Ok(())
    }

    /// Resolves a shell to its owning session and shared I/O handles, releasing the
    /// registry lock before the caller performs any blocking PTY operation.
    fn checkout(&self, shell_id: &str) -> Result<(String, Arc<ShellIo>), AppError> {
        let shells = self
            .shells()
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
            .shells()
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

    /// Starts the thread that notices a shell exiting on its own.
    ///
    /// The handle is kept so shutdown can unpark and join it. The thread holds only the pieces
    /// it uses -- never a `ManagerCore` -- so it can never be the reason the manager stays
    /// alive, which is precisely how the old `Arc::strong_count` guard ended up disabling
    /// shutdown entirely.
    fn start_exit_monitor(
        &self,
        shell_id: &str,
        session_id: &str,
        io: Arc<ShellIo>,
        child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
        termination: Arc<ShellTermination>,
    ) -> Result<(), AppError> {
        let shells = self.core.shells.clone();
        let logging = self.core.logging.clone();
        let events = self.core.events.clone();
        let shutdown = self.core.shutdown.clone();
        let shell_id = shell_id.to_owned();
        let session_id = session_id.to_owned();
        let handle = thread::Builder::new()
            .name(format!("vanehub-shell-monitor-{shell_id}"))
            .spawn(move || {
                // Waiting for a *natural* exit is legitimately open-ended -- a user may keep a
                // shell open for hours. What must be bounded is the lock and the lifetime: each
                // probe holds the child mutex for one `try_wait`, and the wait ends when a
                // terminate settles the shell or the manager starts shutting down. Without that
                // second exit, a shell nobody ever stops keeps its monitor alive forever.
                let ended = poll_until_exit(
                    || probe_shared_child(&child),
                    || !termination.is_settled() && !shutdown.is_signalled(),
                    None,
                    MONITOR_INTERVAL_CEILING,
                );
                if ended == PollEnd::Abandoned {
                    // Either a terminate path owns this child now and publishes its own state,
                    // or shutdown does. Neither wants a second opinion from here.
                    return;
                }
                let outcome = match ended {
                    PollEnd::Exited => TerminationOutcome::Reaped,
                    _ => TerminationOutcome::ReapFailed,
                };
                // A natural exit races the first `stop`. Only the winner reports.
                if termination.claim().is_err() {
                    return;
                }
                termination.settle(outcome, settled_cleanup(outcome));
                log_termination(
                    logging.as_ref(),
                    &session_id,
                    &shell_id,
                    TerminationReport {
                        outcome: Some(outcome),
                        cleanup: termination.cleanup(),
                    },
                    false,
                );
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
            .map_err(|error| AppError::Storage(error.to_string()))?;
        if let Ok(mut monitors) = self.core.monitors.lock() {
            monitors.push(handle);
        }
        Ok(())
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
                    self.logging(),
                    WorkspaceLogLevel::Error,
                    &launch.session_id,
                    &launch.shell_id,
                    "PTY creation failed.",
                );
                AppError::LaunchFailed(error.to_string())
            })?;
        let mut command = CommandBuilder::new(default_shell());
        command.cwd(&root);
        let child = pair.slave.spawn_command(command).map_err(|error| {
            write_shell_log(
                self.logging(),
                WorkspaceLogLevel::Error,
                &launch.session_id,
                &launch.shell_id,
                "Shell process launch failed.",
            );
            AppError::LaunchFailed(error.to_string())
        })?;
        drop(pair.slave);
        // Shared and registered before the fallible setup below, so the two failure paths end
        // the child through the same route as every other termination. Terminating it as a
        // locally owned handle would have been the one place a `kill_failed` could still drop
        // the last reference to a live process -- a hole in exactly the invariant this change
        // exists to close.
        let killer = Mutex::new(child.clone_killer());
        let child = Arc::new(Mutex::new(child));
        let termination = Arc::new(ShellTermination::default());
        let mut reader = match pair.master.try_clone_reader() {
            Ok(reader) => reader,
            Err(error) => {
                self.terminate_unregistered(
                    &child,
                    &killer,
                    &termination,
                    &launch.session_id,
                    &launch.shell_id,
                );
                return Err(AppError::Storage(error.to_string()));
            }
        };
        let writer = match pair.master.take_writer() {
            Ok(writer) => writer,
            Err(error) => {
                self.terminate_unregistered(
                    &child,
                    &killer,
                    &termination,
                    &launch.session_id,
                    &launch.shell_id,
                );
                return Err(AppError::Storage(error.to_string()));
            }
        };

        let events = self.core.events.clone();
        let reader_shell_id = launch.shell_id.clone();
        let reader_session_id = launch.session_id.clone();
        let io = Arc::new(ShellIo {
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
        });
        let monitor_io = io.clone();
        let monitor_child = child.clone();
        let monitor_termination = termination.clone();
        self.insert(
            launch.shell_id.clone(),
            ManagedShell {
                session_id: launch.session_id.clone(),
                root,
                io,
                child,
                killer,
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
                self.logging(),
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
                self.logging(),
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
                self.logging(),
                WorkspaceLogLevel::Warn,
                &session_id,
                shell_id,
                "Shell resize failed.",
            );
        }
        result
    }

    fn stop(&self, shell_id: &str) -> Result<Option<String>, AppError> {
        // Anything still owed from an earlier timeout gets one non-blocking look first, so a
        // child that has since exited is reclaimed on the next ordinary operation rather than
        // waiting for shutdown.
        self.core.pending.sweep(self.logging());
        let shell = {
            let mut shells = self
                .shells()
                .lock()
                .map_err(|error| AppError::Storage(error.to_string()))?;
            shells.remove(shell_id)
        };
        let Some(shell) = shell else {
            return Ok(None);
        };
        // `terminate_shell` hands the child to the pending registry before returning if it
        // outlives the attempt, so dropping `shell` here cannot drop the last handle to a live
        // process.
        terminate_shell(
            &shell,
            &self.core.pending,
            self.logging(),
            shell_id,
            Instant::now() + REAP_DEADLINE,
            false,
        );
        Ok(Some(shell.session_id))
    }

    fn stop_for_session(&self, session_id: &str) -> Result<Vec<(String, String)>, AppError> {
        let shell_ids = {
            let shells = self
                .shells()
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

impl Drop for ManagerCore {
    /// Shutdown, in the order the ownership requires.
    ///
    /// This runs exactly once, when the last handle to the runtime goes, because `&mut self` on
    /// an `Arc`'s contents can only happen then. The old code approximated that with an
    /// `Arc::strong_count(&self.shells) != 1` early return, which was not the same test: every
    /// monitor thread held a clone of that map, so the count was almost never one and shutdown
    /// almost never ran.
    fn drop(&mut self) {
        // 1. Say we are going down, so monitors stop waiting for natural exits, and
        // 2. wake them, so they notice now instead of at the end of a backoff.
        self.shutdown.signal();
        let handles: Vec<thread::JoinHandle<()>> = match self.monitors.lock() {
            Ok(mut monitors) => monitors.drain(..).collect(),
            Err(_) => Vec::new(),
        };
        for handle in &handles {
            handle.thread().unpark();
        }
        // Joining is bounded by construction rather than by a timer: an unparked monitor
        // re-probes, sees the shutdown flag, and returns without touching a lock we hold.
        for handle in handles {
            let _ = handle.join();
        }

        // 3. Drain, then release the routing lock before any process work. Terminating inside
        // the guard made one wedged child block shutdown for every other shell.
        let shells: Vec<(String, ManagedShell)> = match self.shells.lock() {
            Ok(mut shells) => shells.drain().collect(),
            Err(_) => Vec::new(),
        };

        // 4-5. One budget for the whole shutdown, not one per shell: ten wedged shells must not
        // multiply into ten deadlines.
        let deadline = Instant::now() + SHUTDOWN_REAP_DEADLINE;
        for (shell_id, shell) in &shells {
            terminate_shell(
                shell,
                &self.pending,
                self.logging.as_ref(),
                shell_id,
                deadline,
                true,
            );
        }

        // 6. One last non-blocking look, so a child that exited while we worked through the
        // others is recorded as reclaimed rather than as unresolved.
        self.pending.sweep(self.logging.as_ref());
        // 7. Whatever is still owed is named as such. An unreaped child is not erased by the
        // process that failed to reap it going away.
        self.pending.mark_unresolved(self.logging.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::super::shell_termination::{
        reap_shared_child as reap_child_of, ChildProbe, CleanupState, POLL_INTERVAL_CEILING,
    };
    use super::*;
    use crate::contexts::workspaces::application::ShellLog;
    use portable_pty::{ChildKiller, ExitStatus};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Barrier;
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
        /// Counted rather than flagged: "only the owner signalled" is a claim about *how many*
        /// kills happened, and a boolean cannot tell one from eight.
        kills: AtomicUsize,
        polls: AtomicUsize,
        /// Lets a test make a previously wedged child finally exit, so a later sweep has
        /// something real to observe.
        released: AtomicBool,
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

        fn kills(&self) -> usize {
            self.kills.load(Ordering::Acquire)
        }

        fn was_killed(&self) -> bool {
            self.kills() > 0
        }

        /// The child finally exits.
        fn release(&self) {
            self.released.store(true, Ordering::Release);
        }

        fn blocking_wait_reached(&self) -> bool {
            self.blocking_wait_reached.load(Ordering::Acquire)
        }
    }

    #[derive(Debug)]
    struct FakeChild(Arc<FakeChildState>);

    impl ChildKiller for FakeChild {
        fn kill(&mut self) -> io::Result<()> {
            self.0.kills.fetch_add(1, Ordering::AcqRel);
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
            if self.0.released.load(Ordering::Acquire) {
                return Ok(Some(ExitStatus::with_exit_code(0)));
            }
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
        let logging = Arc::new(CapturingLogs::default());
        let manager =
            PortablePtyShellRuntime::new(Arc::new(CapturingEvents::default()), logging.clone());
        // `FailingChild` refuses the kill and keeps reporting itself as running, so this is a
        // refused signal rather than a reap that ran out of time. One outcome, one warning: the
        // old pair of messages described the same event twice and named neither.
        let child: Arc<Mutex<Box<dyn Child + Send + Sync>>> =
            Arc::new(Mutex::new(Box::new(FailingChild)));
        let killer: Mutex<Box<dyn ChildKiller + Send + Sync>> = Mutex::new(Box::new(FailingChild));
        let termination = Arc::new(ShellTermination::default());
        let outcome = manager.terminate_unregistered(
            &child,
            &killer,
            &termination,
            "session-one",
            "shell-one",
        );
        assert_eq!(outcome, TerminationOutcome::KillFailed);
        // A refused kill leaves a live child, so even a shell that never reached the registry
        // hands its handle over rather than dropping it.
        assert_eq!(termination.cleanup(), CleanupState::Pending);

        let messages = logging
            .logs
            .lock()
            .expect("logs")
            .iter()
            .map(|log| log.message.clone())
            .collect::<Vec<_>>();
        assert_eq!(messages.len(), 1, "one outcome, one warning");
        assert!(messages[0].contains("kill_failed"), "{}", messages[0]);
        assert!(messages[0].contains("pending"), "{}", messages[0]);
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
        assert_eq!(manager.shells().lock().expect("shell map").len(), 2);
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
        assert!(manager.shells().lock().expect("shell map").is_empty());
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
            let shells = manager.shells().lock().expect("shell map");
            let wedged = shells.get("shell-one").expect("wedged shell");
            reap_child_of(
                &wedged.child,
                &wedged.killer,
                Instant::now() + TEST_DEADLINE,
            )
        };
        assert_eq!(outcome, TerminationOutcome::ReapTimedOut);
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
                TerminationOutcome::Reaped,
                "reaped",
            ),
            (
                FakeExit::AfterKill,
                false,
                TerminationOutcome::Reaped,
                "reaped",
            ),
            (
                FakeExit::Never,
                false,
                TerminationOutcome::ReapTimedOut,
                "reap_timed_out",
            ),
            (
                FakeExit::Never,
                true,
                TerminationOutcome::KillFailed,
                "kill_failed",
            ),
            (
                FakeExit::ProbeError,
                false,
                TerminationOutcome::ReapFailed,
                "reap_failed",
            ),
        ];

        for (exit, kill_refused, expected, code) in cases {
            let (shell, _slave, state) = scripted_shell("session", &root, exit, kill_refused);
            let outcome =
                reap_child_of(&shell.child, &shell.killer, Instant::now() + TEST_DEADLINE);
            assert_eq!(outcome, expected, "outcome for {exit:?}/{kill_refused}");
            assert_eq!(outcome.code(), code);
            assert!(
                !state.blocking_wait_reached(),
                "no outcome path may reach the blocking wait"
            );
        }
        remove_test_dir(&root);
    }

    #[test]
    fn a_timed_out_reap_keeps_the_child_instead_of_dropping_the_last_handle() {
        let root = temp_dir("timeout-ownership");
        std::fs::create_dir_all(&root).expect("root");
        let logging = CapturingLogs::default();
        let pending = PendingReapRegistry::default();
        let (shell, _slave, state) = scripted_shell("session-one", &root, FakeExit::Never, false);

        let report = terminate_shell(
            &shell,
            &pending,
            &logging,
            "shell-one",
            Instant::now() + TEST_DEADLINE,
            false,
        );

        assert_eq!(report.outcome, Some(TerminationOutcome::ReapTimedOut));
        assert_eq!(report.cleanup, CleanupState::Pending);
        assert_eq!(
            pending.len(),
            1,
            "the registry owns the child that outlived its termination"
        );
        assert!(state.was_killed());

        // The evidence has to name the child, not merely admit that something went wrong.
        let entries = logging.logs.lock().expect("logs");
        let entry = entries
            .first()
            .expect("a timed-out reap is worth recording");
        assert_eq!(entry.session_id, "session-one");
        assert_eq!(entry.shell_id, "shell-one");
        assert!(
            entry.message.contains("reap_timed_out"),
            "{}",
            entry.message
        );
        assert!(entry.message.contains("pending"), "{}", entry.message);
        drop(entries);
        remove_test_dir(&root);
    }

    #[test]
    fn a_child_that_exits_later_becomes_reaped_later_without_rewriting_the_timeout() {
        let root = temp_dir("reaped-later");
        std::fs::create_dir_all(&root).expect("root");
        let logging = CapturingLogs::default();
        let pending = PendingReapRegistry::default();
        let (shell, _slave, state) = scripted_shell("session-one", &root, FakeExit::Never, false);

        let report = terminate_shell(
            &shell,
            &pending,
            &logging,
            "shell-one",
            Instant::now() + TEST_DEADLINE,
            false,
        );
        assert_eq!(report.cleanup, CleanupState::Pending);

        // The child finally goes. A sweep is a pure `try_wait` pass -- a blocking wait here
        // would rebuild, inside the recovery path, the hang the recovery path exists for.
        state.release();
        assert_eq!(pending.sweep(&logging), 1);
        assert_eq!(pending.len(), 0);

        assert_eq!(shell.termination.cleanup(), CleanupState::ReapedLater);
        assert_eq!(
            shell.termination.outcome(),
            Some(TerminationOutcome::ReapTimedOut),
            "recovering later does not rewrite the history: it timed out, and then was reclaimed"
        );
        assert!(!state.blocking_wait_reached());
        remove_test_dir(&root);
    }

    #[test]
    fn a_refused_kill_on_a_live_child_also_keeps_ownership() {
        let root = temp_dir("kill-failed-ownership");
        std::fs::create_dir_all(&root).expect("root");
        let logging = CapturingLogs::default();
        let pending = PendingReapRegistry::default();
        let (shell, _slave, _state) = scripted_shell("session-one", &root, FakeExit::Never, true);

        let report = terminate_shell(
            &shell,
            &pending,
            &logging,
            "shell-one",
            Instant::now() + TEST_DEADLINE,
            false,
        );

        assert_eq!(report.outcome, Some(TerminationOutcome::KillFailed));
        assert_eq!(
            report.cleanup,
            CleanupState::Pending,
            "a signal that was refused leaves a live child, so ownership must not be dropped"
        );
        assert_eq!(pending.len(), 1);
        remove_test_dir(&root);
    }

    #[test]
    fn a_clean_reap_owes_nothing_and_is_not_a_diagnostic() {
        let root = temp_dir("clean-reap-quiet");
        std::fs::create_dir_all(&root).expect("root");
        let logging = CapturingLogs::default();
        let pending = PendingReapRegistry::default();
        let (shell, _slave, _state) =
            scripted_shell("session-one", &root, FakeExit::AfterKill, false);

        let report = terminate_shell(
            &shell,
            &pending,
            &logging,
            "shell-one",
            Instant::now() + TEST_DEADLINE,
            false,
        );

        assert_eq!(report.outcome, Some(TerminationOutcome::Reaped));
        assert_eq!(report.cleanup, CleanupState::NotRequired);
        assert_eq!(pending.len(), 0, "nothing is owed after a clean reap");
        assert!(
            logging.logs.lock().expect("logs").is_empty(),
            "a shell that shut down cleanly is not a warning"
        );
        remove_test_dir(&root);
    }

    #[test]
    fn concurrent_stops_elect_one_owner_and_the_rest_do_not_claim_a_result() {
        let root = temp_dir("stop-race");
        std::fs::create_dir_all(&root).expect("root");
        let logging = Arc::new(CapturingLogs::default());
        let pending = Arc::new(PendingReapRegistry::default());
        // Slow enough that followers genuinely arrive mid-flight rather than after the owner
        // has already settled, which is the interleaving worth testing.
        let (shell, _slave, state) = scripted_shell("session-one", &root, FakeExit::Never, false);
        let shell = Arc::new(shell);

        const RACERS: usize = 8;
        let barrier = Arc::new(Barrier::new(RACERS));
        let mut handles = Vec::with_capacity(RACERS);
        for _ in 0..RACERS {
            let shell = shell.clone();
            let pending = pending.clone();
            let logging = logging.clone();
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                terminate_shell(
                    &shell,
                    &pending,
                    logging.as_ref(),
                    "shell-one",
                    Instant::now() + TEST_DEADLINE,
                    false,
                )
            }));
        }
        let reports: Vec<TerminationReport> = handles
            .into_iter()
            .map(|handle| handle.join().expect("racer"))
            .collect();

        // Exactly one kill and exactly one reap loop, no matter how many callers asked.
        assert_eq!(state.kills(), 1, "only the owner signalled the child");
        let owners = reports
            .iter()
            .filter(|report| report.outcome == Some(TerminationOutcome::ReapTimedOut))
            .count();
        assert_eq!(owners, 1, "exactly one caller produced the outcome");

        // Followers report that a reap is in flight. They do not block on the child, and they
        // do not claim a success -- or any outcome -- that another thread produced.
        for report in &reports {
            match report.outcome {
                Some(TerminationOutcome::ReapTimedOut) => {}
                None => assert_eq!(report.cleanup, CleanupState::Reaping),
                other => panic!("a follower reported an outcome it did not produce: {other:?}"),
            }
        }
        assert_eq!(pending.len(), 1, "one owner, one adoption");
        assert!(!state.blocking_wait_reached());

        // Once settled there is exactly one final answer, and asking again returns it.
        let after = terminate_shell(
            &shell,
            &pending,
            logging.as_ref(),
            "shell-one",
            Instant::now() + TEST_DEADLINE,
            false,
        );
        assert_eq!(after.outcome, Some(TerminationOutcome::ReapTimedOut));
        assert_eq!(state.kills(), 1, "a later ask starts no second kill");
        remove_test_dir(&root);
    }

    #[test]
    fn a_repeated_stop_after_a_timeout_still_reports_the_timeout() {
        let root = temp_dir("timeout-repeat");
        std::fs::create_dir_all(&root).expect("root");
        let logging = CapturingLogs::default();
        let pending = PendingReapRegistry::default();
        let (shell, _slave, state) = scripted_shell("session-one", &root, FakeExit::Never, false);

        let first = terminate_shell(
            &shell,
            &pending,
            &logging,
            "shell-one",
            Instant::now() + TEST_DEADLINE,
            false,
        );
        assert_eq!(first.outcome, Some(TerminationOutcome::ReapTimedOut));

        // The settled outcome is carried, not replaced by a cheerful default, and the second
        // ask does not touch the child again.
        let polls_after_first = state.polls();
        let repeated = terminate_shell(
            &shell,
            &pending,
            &logging,
            "shell-one",
            Instant::now() + TEST_DEADLINE,
            false,
        );
        assert_eq!(repeated.outcome, Some(TerminationOutcome::ReapTimedOut));
        assert_eq!(state.polls(), polls_after_first);
        remove_test_dir(&root);
    }

    #[test]
    fn manager_drop_ends_within_the_deadline_and_leaves_no_monitor_running() {
        let root = temp_dir("drop-shutdown");
        std::fs::create_dir_all(&root).expect("root");
        let logging = Arc::new(CapturingLogs::default());
        let manager =
            PortablePtyShellRuntime::new(Arc::new(CapturingEvents::default()), logging.clone());

        // A child that never exits, with a monitor watching it. Before the shutdown token the
        // monitor waited for a natural exit that was never coming, and because it held a clone
        // of the shells map, `Drop`'s `strong_count` guard then made shutdown do nothing at all.
        let (shell, _slave, state) = scripted_shell("session-one", &root, FakeExit::Never, false);
        let io = shell.io.clone();
        let child = shell.child.clone();
        let termination = shell.termination.clone();
        manager
            .insert("shell-one".to_string(), shell)
            .expect("insert");
        manager
            .start_exit_monitor("shell-one", "session-one", io, child, termination)
            .expect("monitor");

        let started = Instant::now();
        drop(manager);
        let elapsed = started.elapsed();

        assert!(
            elapsed < SHUTDOWN_REAP_DEADLINE * 3,
            "shutdown must finish on its own budget, took {elapsed:?}"
        );
        assert!(state.was_killed(), "shutdown signalled the child");
        assert!(!state.blocking_wait_reached());

        // The child could not be reaped, so it is named as unresolved rather than forgotten.
        let messages = logging
            .logs
            .lock()
            .expect("logs")
            .iter()
            .map(|log| log.message.clone())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            messages.contains("unresolved_at_shutdown"),
            "an unreaped child survives the process that failed to reap it: {messages}"
        );
        remove_test_dir(&root);
    }

    #[test]
    fn one_shell_pending_cleanup_does_not_hold_up_another_shells_shutdown() {
        let root = temp_dir("drop-isolation");
        std::fs::create_dir_all(&root).expect("root");
        let logging = Arc::new(CapturingLogs::default());
        let manager =
            PortablePtyShellRuntime::new(Arc::new(CapturingEvents::default()), logging.clone());
        let (wedged, _wedged_slave, wedged_state) =
            scripted_shell("session-one", &root, FakeExit::Never, false);
        let (healthy, _healthy_slave, healthy_state) =
            scripted_shell("session-two", &root, FakeExit::AfterKill, false);
        manager
            .insert("shell-one".to_string(), wedged)
            .expect("insert wedged");
        manager
            .insert("shell-two".to_string(), healthy)
            .expect("insert healthy");

        drop(manager);

        // The wedged shell consumed the budget; the healthy one still got signalled and reaped.
        assert!(wedged_state.was_killed());
        assert!(healthy_state.was_killed());
        assert!(!wedged_state.blocking_wait_reached());
        assert!(!healthy_state.blocking_wait_reached());
        remove_test_dir(&root);
    }

    #[test]
    fn an_in_flight_reap_turns_a_concurrent_stop_away_rather_than_queueing_it() {
        let termination = ShellTermination::default();
        termination.claim().expect("first caller owns the reap");
        let follower = termination
            .claim()
            .expect_err("a second caller is a follower");
        assert_eq!(follower.outcome, None, "a follower produced no outcome");
        assert_eq!(follower.cleanup, CleanupState::Reaping);

        termination.settle(TerminationOutcome::Reaped, CleanupState::NotRequired);
        assert!(termination.is_settled());
        let after = termination.claim().expect_err("settled");
        assert_eq!(after.outcome, Some(TerminationOutcome::Reaped));
        assert_eq!(after.cleanup, CleanupState::NotRequired);
    }

    #[test]
    fn polling_stops_as_soon_as_a_terminate_path_takes_ownership() {
        let polls = AtomicUsize::new(0);
        let ended = poll_until_exit(
            || {
                polls.fetch_add(1, Ordering::AcqRel);
                ChildProbe::Running
            },
            // Stands in for the exit monitor seeing a terminate settle the shell, or shutdown
            // being signalled: an open-ended wait for a natural exit must still be able to end.
            || polls.load(Ordering::Acquire) < 3,
            None,
            POLL_INTERVAL_CEILING,
        );
        assert_eq!(ended, PollEnd::Abandoned);
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
            .shells()
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
            if manager.shells().lock().expect("shell map").is_empty() {
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
