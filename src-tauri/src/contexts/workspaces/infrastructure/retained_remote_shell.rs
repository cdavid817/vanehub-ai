//! The remote half of a retained Session Shell.
//!
//! One SSH channel per Shell on a pooled transport. That is the whole design: the pool decides how
//! many TCP connections exist, and a Shell decides nothing about them. Closing one Shell closes one
//! channel, and the transport — along with every other Shell riding it — carries on.
//!
//! Reached only through `ssh_connections::api`. Nothing here knows what a transport is made of,
//! which is what keeps a Shell from acquiring one, tuning one, or outliving one.

use super::retained_shell_process::ShellWorker;
use crate::contexts::workspaces::application::{
    RemoteShellChannel, RemoteShellEvent, RemoteShellOpenFailure, RemoteShellTransport,
    SessionShellRuntimePort, ShellOutputSink, ShellRuntimeCloseOutcome, ShellRuntimeOpen,
    ShellRuntimeOpened,
};
use crate::contexts::workspaces::domain::{
    shell_reason, shell_reason_code, SessionShellError, SessionShellState, ShellCloseBudget,
    ShellForegroundProcessState, ShellGeneration, ShellId, ShellRuntimeDescriptor, ShellStream,
    TerminalDimensions,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

struct RemoteShell {
    generation: ShellGeneration,
    channel: Arc<dyn RemoteShellChannel>,
    closing: Arc<AtomicBool>,
    worker: Arc<ShellWorker>,
}

pub(crate) struct RetainedRemoteShellRuntime {
    transport: Arc<dyn RemoteShellTransport>,
    shells: Mutex<HashMap<String, RemoteShell>>,
}

impl RetainedRemoteShellRuntime {
    pub(crate) fn new(transport: Arc<dyn RemoteShellTransport>) -> Self {
        Self {
            transport,
            shells: Mutex::new(HashMap::new()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, RemoteShell>> {
        match self.shells.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn channel(
        &self,
        shell_id: &ShellId,
    ) -> Result<Arc<dyn RemoteShellChannel>, SessionShellError> {
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

/// Owns the newly opened channel until the Shell is committed to the map.
///
/// Only the channel. The pooled transport lease is explicitly *not* here: it belongs to the pool
/// and may be carrying other Shells, so a failed startup that dropped the connection would take
/// unrelated terminals down with it. Ending one channel is the whole extent of this rollback.
struct RemoteShellLaunchGuard {
    channel: Option<Arc<dyn RemoteShellChannel>>,
    closing: Arc<AtomicBool>,
}

impl RemoteShellLaunchGuard {
    fn commit(mut self) {
        self.channel.take();
    }
}

impl Drop for RemoteShellLaunchGuard {
    fn drop(&mut self) {
        let Some(channel) = self.channel.take() else {
            return;
        };
        // Marked first so a reader that did get started does not report the rollback as a
        // spontaneous exit of a Shell the caller never saw open.
        self.closing.store(true, Ordering::SeqCst);
        let _closed = tauri::async_runtime::block_on(async {
            tokio::time::timeout(ShellCloseBudget::default().terminate, channel.close()).await
        });
    }
}

/// Waits for the reader to report itself finished, then gives up its handle.
///
/// Polling rather than joining, because the thread being waited on is blocked inside an SSH read
/// that a closed channel is *expected* to end — and, when the transport is wedged, is exactly the
/// thread that will never end at all.
fn wait_for_worker(worker: &ShellWorker, budget: ShellCloseBudget) -> bool {
    let deadline = std::time::Instant::now() + budget.worker;
    loop {
        if worker.try_join() {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(budget.poll);
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
        let channel = tauri::async_runtime::block_on(self.transport.open_channel(
            &remote.connection_id,
            remote.profile_revision,
            request.dimensions.cols(),
            request.dimensions.rows(),
        ))
        .map_err(|failure| match failure {
            RemoteShellOpenFailure::ConnectionUnavailable => {
                unavailable("shell_remote_connection_unavailable")
            }
            RemoteShellOpenFailure::ChannelUnavailable => {
                unavailable("shell_remote_channel_unavailable")
            }
        })?;

        let closing = Arc::new(AtomicBool::new(false));
        let reader_channel = channel.clone();
        let reader_shell = request.shell_id.clone();
        let generation = request.generation;
        let reader_closing = closing.clone();
        // A guard from here on: the channel is open, and every `?` below has to close it rather
        // than return past it. `RemoteShellLaunchGuard` owns the channel until commit, and the
        // pooled transport lease stays untouched — closing one Shell's channel is not this Shell's
        // business to extend to the connection every other Shell is riding.
        let guard = RemoteShellLaunchGuard {
            channel: Some(channel.clone()),
            closing: closing.clone(),
        };
        let worker = Arc::new(ShellWorker::spawn(
            format!("vanehub-remote-shell-{}", request.shell_id.as_str()),
            move || {
                loop {
                    match tauri::async_runtime::block_on(reader_channel.next_event()) {
                        Ok(Some(RemoteShellEvent::Output(bytes))) => {
                            sink.on_output(&reader_shell, generation, ShellStream::Pty, &bytes);
                        }
                        Ok(Some(RemoteShellEvent::Exited { code })) => {
                            if !reader_closing.load(Ordering::SeqCst) {
                                sink.on_state(
                                    &reader_shell,
                                    generation,
                                    SessionShellState::Exited { code },
                                );
                            }
                            break;
                        }
                        // A remote program can close its output while the channel stays open and
                        // the user keeps typing. Ending here would tear down a live Shell.
                        Ok(Some(RemoteShellEvent::Eof)) => continue,
                        Ok(None) => {
                            if !reader_closing.load(Ordering::SeqCst) {
                                sink.on_state(
                                    &reader_shell,
                                    generation,
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
                                    generation,
                                    SessionShellState::Disconnected {
                                        reason: shell_reason("shell_remote_channel_lost"),
                                    },
                                );
                            }
                            break;
                        }
                    }
                }
            },
        )?);

        guard.commit();
        self.lock().insert(
            request.shell_id.as_str().to_string(),
            RemoteShell {
                generation: request.generation,
                channel,
                closing,
                worker,
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

    /// Closes one channel within a finite budget. The transport stays, and so does every other
    /// Shell riding it.
    ///
    /// The entry is not removed up front. Removing it and then closing is what makes a failed
    /// remote close unrecoverable: the channel is gone from the map, a retry has nothing to retry,
    /// and the routed runtime falls through to the *local* adapter for a Shell that never was.
    fn close(
        &self,
        shell_id: &ShellId,
        generation: ShellGeneration,
        budget: ShellCloseBudget,
    ) -> ShellRuntimeCloseOutcome {
        let held = {
            let shells = self.lock();
            let shell = shells.get(shell_id.as_str());
            match shell {
                Some(shell) if shell.generation == generation => Some((
                    shell.channel.clone(),
                    shell.closing.clone(),
                    shell.worker.clone(),
                )),
                _ => None,
            }
        };
        let Some((channel, closing, worker)) = held else {
            return ShellRuntimeCloseOutcome::NotHeld;
        };
        closing.store(true, Ordering::SeqCst);
        // One bounded wait for the channel-level close. `block_on` with no ceiling is how a stuck
        // SSH channel becomes an application that will not shut down.
        let closed = tauri::async_runtime::block_on(async {
            tokio::time::timeout(budget.terminate, channel.close())
                .await
                .map(|result| result.is_ok())
        });
        let confirmed = matches!(closed, Ok(true));
        if !confirmed {
            return ShellRuntimeCloseOutcome::Retained {
                reason: shell_reason(if closed.is_err() {
                    shell_reason_code::CLOSE_DEADLINE_REACHED
                } else {
                    shell_reason_code::TERMINATE_FAILED
                }),
                retryable: true,
            };
        }
        // The reader is blocked inside `next_event`; joining it unconditionally is the unbounded
        // wait this whole change exists to remove. It is joined only once it says it has finished.
        if !wait_for_worker(&worker, budget) {
            return ShellRuntimeCloseOutcome::Retained {
                reason: shell_reason(shell_reason_code::WORKER_COMPLETION_PENDING),
                retryable: true,
            };
        }
        let mut shells = self.lock();
        if shells
            .get(shell_id.as_str())
            .is_some_and(|shell| shell.generation == generation)
        {
            shells.remove(shell_id.as_str());
        }
        ShellRuntimeCloseOutcome::Confirmed
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
    /// Which runtime owns a Shell, and which life of it the entry describes.
    ///
    /// The generation is what makes a late close safe. A close for an old generation that arrives
    /// after the id was opened again must not delete the route the new Shell is using, and the
    /// consequence of getting that wrong is silent: the next write falls through to the *local*
    /// runtime for a remote Shell and reports "not found" for a terminal the user is looking at.
    routes: Mutex<HashMap<String, ShellRoute>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ShellRoute {
    generation: ShellGeneration,
    is_remote: bool,
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

    fn routes(&self) -> std::sync::MutexGuard<'_, HashMap<String, ShellRoute>> {
        match self.routes.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn route(&self, shell_id: &ShellId) -> Arc<dyn SessionShellRuntimePort> {
        let is_remote = self
            .routes()
            .get(shell_id.as_str())
            .is_some_and(|route| route.is_remote);
        if is_remote {
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
        // does not exist. Replacing an entry for an older generation is correct — that Shell is
        // gone — but an entry for a *newer* one is never overwritten by a late arrival.
        let mut routes = self.routes();
        let stale = routes
            .get(request.shell_id.as_str())
            .is_some_and(|route| route.generation > request.generation);
        if !stale {
            routes.insert(
                request.shell_id.as_str().to_string(),
                ShellRoute {
                    generation: request.generation,
                    is_remote,
                },
            );
        }
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

    /// Routes the close, and removes the route only on confirmation for the same generation.
    ///
    /// The removed-unconditionally version of this was the defect: a remote close that timed out
    /// deleted the route on its way out, so the retry the user pressed next went to the local
    /// runtime, found nothing, and reported success for a channel that was still open.
    fn close(
        &self,
        shell_id: &ShellId,
        generation: ShellGeneration,
        budget: ShellCloseBudget,
    ) -> ShellRuntimeCloseOutcome {
        let outcome = self.route(shell_id).close(shell_id, generation, budget);
        if outcome.is_released() {
            let mut routes = self.routes();
            if routes
                .get(shell_id.as_str())
                .is_some_and(|route| route.generation == generation)
            {
                routes.remove(shell_id.as_str());
            }
        }
        outcome
    }

    fn foreground_process(&self, shell_id: &ShellId) -> ShellForegroundProcessState {
        self.route(shell_id).foreground_process(shell_id)
    }
}
