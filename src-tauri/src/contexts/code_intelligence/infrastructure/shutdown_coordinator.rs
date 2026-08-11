use super::json_rpc_actor::JsonRpcClient;
use super::lsp_stdio_child::{
    LspShutdownDisposition, LspShutdownOutcome, LspStdioError, ManagedLspExit, ManagedLspStdio,
};
use futures_util::stream::{FuturesUnordered, StreamExt};
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, Notify};

#[derive(Clone)]
pub(crate) struct ActiveLspProcess {
    client: JsonRpcClient,
    process: Arc<Mutex<Option<ManagedLspStdio>>>,
}

#[derive(Debug)]
pub(crate) enum ActiveLspProcessWait {
    Pending,
    Exited(ManagedLspExit),
    Gone,
}

impl fmt::Debug for ActiveLspProcess {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ActiveLspProcess { managed: true }")
    }
}

impl ActiveLspProcess {
    pub(crate) fn new(client: JsonRpcClient, process: ManagedLspStdio) -> Self {
        Self {
            client,
            process: Arc::new(Mutex::new(Some(process))),
        }
    }

    pub(crate) fn client(&self) -> JsonRpcClient {
        self.client.clone()
    }

    pub(crate) async fn shutdown(
        &self,
        deadline: Instant,
    ) -> Result<Option<LspShutdownOutcome>, LspStdioError> {
        let Some(mut process) = self.process.lock().await.take() else {
            return Ok(None);
        };
        process
            .shutdown_protocol(&self.client, deadline)
            .await
            .map(Some)
    }

    pub(crate) async fn wait_until(
        &self,
        deadline: Instant,
    ) -> Result<ActiveLspProcessWait, LspStdioError> {
        let mut process = self.process.lock().await;
        let Some(active) = process.as_mut() else {
            return Ok(ActiveLspProcessWait::Gone);
        };
        match active.wait_until(deadline).await {
            Ok(Some(exit)) => {
                process.take();
                Ok(ActiveLspProcessWait::Exited(exit))
            }
            Ok(None) => Ok(ActiveLspProcessWait::Pending),
            Err(error) => {
                process.take();
                Err(error)
            }
        }
    }

    pub(crate) async fn force_shutdown(
        &self,
        deadline: Instant,
    ) -> Result<Option<ManagedLspExit>, LspStdioError> {
        let Some(mut process) = self.process.lock().await.take() else {
            return Ok(None);
        };
        process.force_shutdown(deadline).await.map(Some)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LspShutdownSummary {
    pub(crate) total: usize,
    pub(crate) graceful: usize,
    pub(crate) forced: usize,
    pub(crate) failed: usize,
}

struct CoordinatorState {
    processes: Vec<ActiveLspProcess>,
    shutting_down: bool,
    completed: Option<LspShutdownSummary>,
}

struct CoordinatorInner {
    accepting: AtomicBool,
    state: Mutex<CoordinatorState>,
    completed: Notify,
}

#[derive(Clone)]
pub(crate) struct LspShutdownCoordinator {
    inner: Arc<CoordinatorInner>,
}

impl LspShutdownCoordinator {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(CoordinatorInner {
                accepting: AtomicBool::new(true),
                state: Mutex::new(CoordinatorState {
                    processes: Vec::new(),
                    shutting_down: false,
                    completed: None,
                }),
                completed: Notify::new(),
            }),
        }
    }

    pub(crate) fn is_accepting(&self) -> bool {
        self.inner.accepting.load(Ordering::Acquire)
    }

    pub(crate) async fn register(&self, process: ActiveLspProcess) -> Result<(), ActiveLspProcess> {
        let mut state = self.inner.state.lock().await;
        if !self.inner.accepting.load(Ordering::Acquire) || state.shutting_down {
            return Err(process);
        }
        state.processes.push(process);
        Ok(())
    }

    pub(crate) async fn shutdown_all(&self, deadline: Instant) -> LspShutdownSummary {
        let processes = loop {
            let notified = self.inner.completed.notified();
            let mut state = self.inner.state.lock().await;
            if let Some(summary) = state.completed {
                return summary;
            }
            if !state.shutting_down {
                state.shutting_down = true;
                self.inner.accepting.store(false, Ordering::Release);
                break std::mem::take(&mut state.processes);
            }
            drop(state);
            if tokio::time::timeout_at(deadline.into(), notified)
                .await
                .is_err()
            {
                return LspShutdownSummary::default();
            }
        };

        let mut pending = FuturesUnordered::new();
        for process in processes {
            pending.push(async move { process.shutdown(deadline).await });
        }
        let mut summary = LspShutdownSummary::default();
        while let Some(outcome) = pending.next().await {
            match outcome {
                Ok(Some(outcome)) if outcome.disposition == LspShutdownDisposition::Graceful => {
                    summary.total += 1;
                    summary.graceful += 1;
                }
                Ok(Some(_)) => {
                    summary.total += 1;
                    summary.forced += 1;
                }
                Ok(None) => {}
                Err(_) => {
                    summary.total += 1;
                    summary.failed += 1;
                }
            }
        }

        let mut state = self.inner.state.lock().await;
        state.completed = Some(summary);
        drop(state);
        self.inner.completed.notify_waiters();
        summary
    }
}

impl Default for LspShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
