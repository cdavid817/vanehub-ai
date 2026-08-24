//! The local PTY and remote SSH halves of a retained Session Shell.
//!
//! Both live behind one application port, and both obey the same two rules. The map of live shells
//! is never held across a blocking call — a write, a resize, a close, or a worker join — because a
//! shell whose child stopped draining its pipe would otherwise stall every other shell in the
//! application. And a worker is always joined on close, because a reader thread outliving its PTY
//! is a thread reading a closed handle forever.

use crate::contexts::workspaces::application::{
    SessionShellRuntimePort, ShellOutputSink, ShellRuntimeOpen, ShellRuntimeOpened,
};
use crate::contexts::workspaces::domain::{
    shell_reason, SessionShellError, SessionShellState, ShellForegroundProcessState, ShellId,
    ShellRuntimeDescriptor, ShellStream, TerminalDimensions,
};
use crate::platform::filesystem::normalize_windows_extended_length_path;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Larger reads coalesce bursty output into fewer frames without adding latency: a read returns as
/// soon as any bytes are available, so interactive echo is unaffected.
const SHELL_READ_BUFFER_BYTES: usize = 64 * 1024;

/// The blocking halves of one shell, checked out of the map before use.
struct ShellIo {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
}

struct RetainedShell {
    io: Arc<ShellIo>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    killer: Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>,
    /// Set before the child is killed so the reader stops reporting a closed handle as a failure.
    closing: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

/// One shell, one PTY, retained until something closes it.
pub(crate) struct RetainedLocalShellRuntime {
    shells: Mutex<HashMap<String, RetainedShell>>,
}

impl Default for RetainedLocalShellRuntime {
    fn default() -> Self {
        Self {
            shells: Mutex::new(HashMap::new()),
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

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|_| unavailable("shell_reader_unavailable"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|_| unavailable("shell_writer_unavailable"))?;
        let killer = child.clone_killer();
        let child = Arc::new(Mutex::new(child));
        let closing = Arc::new(AtomicBool::new(false));

        let reader_shell = request.shell_id.clone();
        let reader_sink = sink.clone();
        let reader_worker = thread::Builder::new()
            .name(format!(
                "vanehub-session-shell-{}",
                request.shell_id.as_str()
            ))
            .spawn(move || loop {
                let mut buffer = [0u8; SHELL_READ_BUFFER_BYTES];
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    // Bytes go to the sink undecoded: splitting a UTF-8 sequence across two reads
                    // is normal, and only the retained buffer knows what it is still waiting on.
                    Ok(count) => {
                        reader_sink.on_output(&reader_shell, ShellStream::Pty, &buffer[..count])
                    }
                    Err(_) => break,
                }
            })
            .map_err(|_| unavailable("shell_reader_thread_unavailable"))?;

        let monitor_shell = request.shell_id.clone();
        let monitor_child = child.clone();
        let monitor_closing = closing.clone();
        let monitor_worker = thread::Builder::new()
            .name(format!(
                "vanehub-session-shell-exit-{}",
                request.shell_id.as_str()
            ))
            .spawn(move || {
                let status = loop {
                    let waited = {
                        let mut child = match monitor_child.lock() {
                            Ok(guard) => guard,
                            Err(poisoned) => poisoned.into_inner(),
                        };
                        child.try_wait()
                    };
                    match waited {
                        Ok(Some(status)) => break Some(status),
                        Ok(None) => thread::sleep(std::time::Duration::from_millis(100)),
                        Err(_) => break None,
                    }
                };
                // A close already told the store; reporting again would end one Shell twice.
                if monitor_closing.load(Ordering::SeqCst) {
                    return;
                }
                let code = status.map(|status| status.exit_code() as i32);
                sink.on_state(&monitor_shell, SessionShellState::Exited { code });
            })
            .map_err(|_| unavailable("shell_monitor_thread_unavailable"))?;

        self.lock().insert(
            request.shell_id.as_str().to_string(),
            RetainedShell {
                io: Arc::new(ShellIo {
                    master: Mutex::new(pair.master),
                    writer: Mutex::new(writer),
                }),
                child,
                killer: Mutex::new(killer),
                closing,
                workers: vec![reader_worker, monitor_worker],
            },
        );
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
        let master = match io.master.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        master
            .resize(terminal_size(dimensions))
            .map_err(|_| runtime_error("shell_resize_failed"))
    }

    /// Removes the shell from the map, then kills and joins outside the lock.
    ///
    /// Closing a shell the runtime does not hold is a success: a registry entry can outlive its
    /// process, and a close that failed on that would make cleanup unreliable exactly where it
    /// matters.
    fn close(&self, shell_id: &ShellId) -> Result<(), SessionShellError> {
        let Some(shell) = self.lock().remove(shell_id.as_str()) else {
            return Ok(());
        };
        shell.closing.store(true, Ordering::SeqCst);
        {
            let mut killer = match shell.killer.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let _ = killer.kill();
        }
        {
            let mut child = match shell.child.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            let _ = child.wait();
        }
        // Joined rather than detached: a reader left behind is a thread reading a closed handle,
        // and at shutdown it is a thread the process waits on forever.
        for worker in shell.workers {
            let _ = worker.join();
        }
        Ok(())
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
