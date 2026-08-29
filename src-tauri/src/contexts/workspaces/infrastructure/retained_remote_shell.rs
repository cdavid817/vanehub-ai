//! The remote half of a retained Session Shell.
//!
//! One SSH channel per Shell on a pooled transport. That is the whole design: the pool decides how
//! many TCP connections exist, and a Shell decides nothing about them. Closing one Shell closes one
//! channel, and the transport — along with every other Shell riding it — carries on.
//!
//! Reached only through `ssh_connections::api`. Nothing here knows what a transport is made of,
//! which is what keeps a Shell from acquiring one, tuning one, or outliving one.

use crate::contexts::ssh_connections::api::{
    SshConnectionsApi, SshExecutionChannel, SshExecutionChannelEvent,
};
use crate::contexts::workspaces::application::{
    SessionShellRuntimePort, ShellOutputSink, ShellRuntimeOpen, ShellRuntimeOpened,
};
use crate::contexts::workspaces::domain::{
    shell_reason, SessionShellError, SessionShellState, ShellForegroundProcessState, ShellId,
    ShellRuntimeDescriptor, ShellStream, TerminalDimensions,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

struct RemoteShell {
    channel: Arc<SshExecutionChannel>,
    closing: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

pub(crate) struct RetainedRemoteShellRuntime {
    ssh: SshConnectionsApi,
    shells: Mutex<HashMap<String, RemoteShell>>,
}

impl RetainedRemoteShellRuntime {
    pub(crate) fn new(ssh: SshConnectionsApi) -> Self {
        Self {
            ssh,
            shells: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, RemoteShell>> {
        match self.shells.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn channel(&self, shell_id: &ShellId) -> Result<Arc<SshExecutionChannel>, SessionShellError> {
        self.lock()
            .get(shell_id.as_str())
            .map(|shell| shell.channel.clone())
            .ok_or(SessionShellError::NotFound)
    }
}

fn unavailable(reason: &str) -> SessionShellError {
    SessionShellError::RuntimeUnavailable {
        reason: shell_reason(reason),
    }
}

fn runtime_error(reason: &str) -> SessionShellError {
    SessionShellError::Runtime {
        reason: shell_reason(reason),
    }
}

impl SessionShellRuntimePort for RetainedRemoteShellRuntime {
    fn open(
        &self,
        request: &ShellRuntimeOpen,
        sink: Arc<dyn ShellOutputSink>,
    ) -> Result<ShellRuntimeOpened, SessionShellError> {
        let Some(remote) = request.remote.as_ref() else {
            return Err(unavailable("shell_local_not_supported_remotely"));
        };
        // The revision is checked by `acquire_execution`, which refuses a stale one. A Shell opened
        // against a profile the user has since edited would connect somewhere they did not choose.
        let lease = tauri::async_runtime::block_on(
            self.ssh
                .acquire_execution(&remote.connection_id, remote.profile_revision),
        )
        .map_err(|_| unavailable("shell_remote_connection_unavailable"))?;
        let channel = tauri::async_runtime::block_on(
            lease.open_pty(request.dimensions.cols(), request.dimensions.rows()),
        )
        .map_err(|_| unavailable("shell_remote_channel_unavailable"))?;
        let channel = Arc::new(channel);

        let closing = Arc::new(AtomicBool::new(false));
        let reader_channel = channel.clone();
        let reader_shell = request.shell_id.clone();
        let reader_closing = closing.clone();
        let worker = thread::Builder::new()
            .name(format!(
                "vanehub-remote-shell-{}",
                request.shell_id.as_str()
            ))
            .spawn(move || {
                loop {
                    match tauri::async_runtime::block_on(reader_channel.next_event()) {
                        Ok(Some(SshExecutionChannelEvent::Output(bytes)))
                        | Ok(Some(SshExecutionChannelEvent::ExtendedOutput {
                            content: bytes,
                            ..
                        })) => {
                            // One merged stream from the reader's point of view: an SSH PTY
                            // interleaves them, and labelling either separately would claim a
                            // separation nobody made.
                            sink.on_output(&reader_shell, ShellStream::Pty, &bytes);
                        }
                        Ok(Some(SshExecutionChannelEvent::ExitStatus(code))) => {
                            if !reader_closing.load(Ordering::SeqCst) {
                                sink.on_state(
                                    &reader_shell,
                                    SessionShellState::Exited {
                                        code: Some(code as i32),
                                    },
                                );
                            }
                            break;
                        }
                        Ok(Some(SshExecutionChannelEvent::ExitSignal(_))) => {
                            // Killed by a signal. The runtime saw it end and did not see a code,
                            // so the code stays absent rather than becoming a zero that would
                            // read as a clean exit.
                            if !reader_closing.load(Ordering::SeqCst) {
                                sink.on_state(
                                    &reader_shell,
                                    SessionShellState::Exited { code: None },
                                );
                            }
                            break;
                        }
                        Ok(Some(SshExecutionChannelEvent::Eof)) => continue,
                        Ok(Some(SshExecutionChannelEvent::Closed)) | Ok(None) => {
                            if !reader_closing.load(Ordering::SeqCst) {
                                sink.on_state(
                                    &reader_shell,
                                    SessionShellState::Exited { code: None },
                                );
                            }
                            break;
                        }
                        Err(_) => {
                            // A transport failure is a state, not a silence. Reporting it lets the
                            // UI keep the replay it holds and say why nothing more is arriving,
                            // rather than showing a live terminal that stopped answering.
                            if !reader_closing.load(Ordering::SeqCst) {
                                sink.on_state(
                                    &reader_shell,
                                    SessionShellState::Disconnected {
                                        reason: shell_reason("shell_remote_channel_lost"),
                                    },
                                );
                            }
                            break;
                        }
                    }
                }
            })
            .map_err(|_| unavailable("shell_remote_worker_unavailable"))?;

        self.lock().insert(
            request.shell_id.as_str().to_string(),
            RemoteShell {
                channel,
                closing,
                worker: Some(worker),
            },
        );
        Ok(ShellRuntimeOpened {
            runtime: ShellRuntimeDescriptor::Remote {
                connection_id: remote.connection_id.clone(),
                profile_revision: remote.profile_revision,
                // No automatic reconnect. A channel that reopened by itself would replay nothing
                // and inherit no shell state, so the user would be typing into a fresh shell that
                // looks like the one they had.
                supports_reconnect: false,
            },
            state: SessionShellState::Running,
        })
    }

    fn write(&self, shell_id: &ShellId, content: &str) -> Result<(), SessionShellError> {
        let channel = self.channel(shell_id)?;
        tauri::async_runtime::block_on(channel.write(content.as_bytes()))
            .map_err(|_| runtime_error("shell_remote_write_failed"))
    }

    fn resize(
        &self,
        shell_id: &ShellId,
        dimensions: TerminalDimensions,
    ) -> Result<(), SessionShellError> {
        let channel = self.channel(shell_id)?;
        tauri::async_runtime::block_on(channel.resize(dimensions.cols(), dimensions.rows()))
            .map_err(|_| runtime_error("shell_remote_resize_failed"))
    }

    /// Closes one channel and joins its worker. The transport stays, and so does every other Shell
    /// riding it.
    fn close(&self, shell_id: &ShellId) -> Result<(), SessionShellError> {
        let Some(mut shell) = self.lock().remove(shell_id.as_str()) else {
            return Ok(());
        };
        shell.closing.store(true, Ordering::SeqCst);
        let _ = tauri::async_runtime::block_on(shell.channel.close());
        if let Some(worker) = shell.worker.take() {
            let _ = worker.join();
        }
        Ok(())
    }

    /// An SSH PTY exposes no reliable foreground marker either, and inferring one from output would
    /// be reading terminal text to invent a fact.
    fn foreground_process(&self, shell_id: &ShellId) -> ShellForegroundProcessState {
        if self.lock().contains_key(shell_id.as_str()) {
            ShellForegroundProcessState::Unknown
        } else {
            ShellForegroundProcessState::Absent
        }
    }
}

/// Sends a Shell to whichever runtime owns its workspace.
///
/// One port, two implementations, and the choice made once at open time from the resolved
/// workspace rather than at every call: a Shell that could change which runtime it belongs to
/// midway would be two shells sharing an id.
pub(crate) struct RoutedShellRuntime {
    local: Arc<dyn SessionShellRuntimePort>,
    remote: Arc<dyn SessionShellRuntimePort>,
    routes: Mutex<HashMap<String, bool>>,
}

impl RoutedShellRuntime {
    pub(crate) fn new(
        local: Arc<dyn SessionShellRuntimePort>,
        remote: Arc<dyn SessionShellRuntimePort>,
    ) -> Self {
        Self {
            local,
            remote,
            routes: Mutex::new(HashMap::new()),
        }
    }

    fn route(&self, shell_id: &ShellId) -> Arc<dyn SessionShellRuntimePort> {
        let routes = match self.routes.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if routes.get(shell_id.as_str()).copied().unwrap_or(false) {
            self.remote.clone()
        } else {
            self.local.clone()
        }
    }
}

impl SessionShellRuntimePort for RoutedShellRuntime {
    fn open(
        &self,
        request: &ShellRuntimeOpen,
        sink: Arc<dyn ShellOutputSink>,
    ) -> Result<ShellRuntimeOpened, SessionShellError> {
        let is_remote = request.remote.is_some();
        let runtime = if is_remote {
            self.remote.clone()
        } else {
            self.local.clone()
        };
        let opened = runtime.open(request, sink)?;
        // Recorded only after the open succeeded, so a failed open leaves no route to a Shell that
        // does not exist.
        match self.routes.lock() {
            Ok(mut routes) => routes.insert(request.shell_id.as_str().to_string(), is_remote),
            Err(poisoned) => poisoned
                .into_inner()
                .insert(request.shell_id.as_str().to_string(), is_remote),
        };
        Ok(opened)
    }

    fn write(&self, shell_id: &ShellId, content: &str) -> Result<(), SessionShellError> {
        self.route(shell_id).write(shell_id, content)
    }

    fn resize(
        &self,
        shell_id: &ShellId,
        dimensions: TerminalDimensions,
    ) -> Result<(), SessionShellError> {
        self.route(shell_id).resize(shell_id, dimensions)
    }

    fn close(&self, shell_id: &ShellId) -> Result<(), SessionShellError> {
        let result = self.route(shell_id).close(shell_id);
        match self.routes.lock() {
            Ok(mut routes) => routes.remove(shell_id.as_str()),
            Err(poisoned) => poisoned.into_inner().remove(shell_id.as_str()),
        };
        result
    }

    fn foreground_process(&self, shell_id: &ShellId) -> ShellForegroundProcessState {
        self.route(shell_id).foreground_process(shell_id)
    }
}
