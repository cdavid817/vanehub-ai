use super::json_rpc_actor::{
    spawn_json_rpc_actor, JsonRpcActorLimits, JsonRpcClient, JsonRpcEvents, ServerRequestHandler,
};
use super::lsp_framing::{FrameLimits, LspFrameError, LspFrameReader, LspFrameWriter};
use crate::platform::process::{ManagedTokioChild, TokioStderrDrain};
use serde_json::Value;
use std::collections::BTreeMap;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::task::JoinHandle;

const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(20);
const STDERR_FINISH_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LspStdioError {
    #[error("managed LSP process could not start")]
    Spawn,
    #[error("managed LSP process operation failed")]
    Process,
    #[error("managed LSP protocol failed: {0}")]
    Protocol(LspFrameError),
    #[error("managed LSP actor stopped")]
    Actor,
    #[error("managed LSP stderr drain failed")]
    Stderr,
}

#[derive(Debug)]
pub(crate) struct LspStderrSummary {
    pub(crate) observed_bytes: u64,
    pub(crate) truncated: bool,
}

#[derive(Debug)]
pub(crate) struct ManagedLspExit {
    pub(crate) status: ExitStatus,
    pub(crate) stderr: LspStderrSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LspShutdownDisposition {
    Graceful,
    Forced,
}

#[derive(Debug)]
pub(crate) struct LspShutdownOutcome {
    pub(crate) disposition: LspShutdownDisposition,
    pub(crate) exit: ManagedLspExit,
}

pub(crate) struct ManagedLspStdio {
    child: ManagedTokioChild,
    reader: Option<JoinHandle<Result<(), LspStdioError>>>,
    writer: Option<JoinHandle<Result<(), LspStdioError>>>,
    stderr: Option<TokioStderrDrain>,
    reaped: bool,
    protocol_tasks_finished: bool,
}

impl ManagedLspStdio {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn(
        executable: &str,
        args: &[String],
        environment: &BTreeMap<String, String>,
        frame_limits: FrameLimits,
        stderr_limit: usize,
        actor_limits: JsonRpcActorLimits,
        handler: Arc<dyn ServerRequestHandler>,
    ) -> Result<(JsonRpcClient, JsonRpcEvents, Self), LspStdioError> {
        let mut child = ManagedTokioChild::spawn(executable, args, environment)
            .map_err(|_| LspStdioError::Spawn)?;
        let stdin = child.take_stdin().map_err(|_| LspStdioError::Spawn)?;
        let stdout = child.take_stdout().map_err(|_| LspStdioError::Spawn)?;
        let stderr = child.take_stderr().map_err(|_| LspStdioError::Spawn)?;
        let stderr = TokioStderrDrain::spawn(stderr, stderr_limit);
        let (client, transport) = spawn_json_rpc_actor(actor_limits, handler);
        let (inbound, mut outbound, events) = transport.into_parts();
        let reader = tokio::spawn(async move {
            let mut reader = LspFrameReader::new(stdout, frame_limits);
            while let Some(payload) = reader.read_frame().await.map_err(LspStdioError::Protocol)? {
                inbound
                    .send(payload)
                    .await
                    .map_err(|_| LspStdioError::Actor)?;
            }
            Ok(())
        });
        let writer = tokio::spawn(async move {
            let mut writer = LspFrameWriter::new(stdin, frame_limits.max_payload_bytes())
                .map_err(LspStdioError::Protocol)?;
            while let Some(payload) = outbound.recv().await {
                writer
                    .write_frame(&payload)
                    .await
                    .map_err(LspStdioError::Protocol)?;
            }
            Ok(())
        });
        Ok((
            client,
            events,
            Self {
                child,
                reader: Some(reader),
                writer: Some(writer),
                stderr: Some(stderr),
                reaped: false,
                protocol_tasks_finished: false,
            },
        ))
    }

    pub(crate) async fn wait_until(
        &mut self,
        deadline: Instant,
    ) -> Result<Option<ManagedLspExit>, LspStdioError> {
        loop {
            if let Some(result) = take_finished_task(&mut self.reader).await {
                match result {
                    Ok(()) => {}
                    Err(error) => return Err(self.terminate_after(error, deadline).await),
                }
            }
            if let Some(Err(error)) = take_finished_task(&mut self.writer).await {
                return Err(self.terminate_after(error, deadline).await);
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(None);
            }
            let poll_deadline = deadline.min(now + PROCESS_POLL_INTERVAL);
            match self
                .child
                .wait_until(poll_deadline)
                .await
                .map_err(|_| LspStdioError::Process)?
            {
                Some(status) => return self.finish_exit(status, deadline).await.map(Some),
                None => tokio::task::yield_now().await,
            }
        }
    }

    pub(crate) fn is_reaped(&self) -> bool {
        self.reaped
    }

    pub(crate) fn protocol_tasks_finished(&self) -> bool {
        self.protocol_tasks_finished
    }

    pub(crate) async fn shutdown_protocol(
        &mut self,
        client: &JsonRpcClient,
        deadline: Instant,
    ) -> Result<LspShutdownOutcome, LspStdioError> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let protocol_budget = remaining / 2;
        let graceful = !protocol_budget.is_zero()
            && tokio::time::timeout(protocol_budget, async {
                client
                    .request::<_, Value>("shutdown", Value::Null)
                    .await
                    .map_err(|_| LspStdioError::Actor)?;
                client
                    .notify("exit", Value::Null)
                    .await
                    .map_err(|_| LspStdioError::Actor)
            })
            .await
            .is_ok_and(|result| result.is_ok());
        if graceful {
            if let Ok(Some(exit)) = self.wait_until(deadline).await {
                return Ok(LspShutdownOutcome {
                    disposition: LspShutdownDisposition::Graceful,
                    exit,
                });
            }
        }
        self.force_shutdown(deadline)
            .await
            .map(|exit| LspShutdownOutcome {
                disposition: LspShutdownDisposition::Forced,
                exit,
            })
    }

    pub(crate) async fn force_shutdown(
        &mut self,
        deadline: Instant,
    ) -> Result<ManagedLspExit, LspStdioError> {
        self.abort_protocol_tasks().await;
        let status = self
            .child
            .shutdown(deadline)
            .await
            .map_err(|_| LspStdioError::Process)?;
        self.reaped = true;
        let stderr = self.finish_stderr().await.ok_or(LspStdioError::Stderr)?;
        Ok(ManagedLspExit { status, stderr })
    }

    async fn terminate_after(&mut self, error: LspStdioError, deadline: Instant) -> LspStdioError {
        self.abort_protocol_tasks().await;
        if self.child.shutdown(deadline).await.is_ok() {
            self.reaped = true;
        }
        self.finish_stderr().await;
        error
    }

    async fn finish_exit(
        &mut self,
        status: ExitStatus,
        deadline: Instant,
    ) -> Result<ManagedLspExit, LspStdioError> {
        self.reaped = true;
        self.finish_protocol_tasks(deadline).await;
        let stderr = self.finish_stderr().await.ok_or(LspStdioError::Stderr)?;
        Ok(ManagedLspExit { status, stderr })
    }

    async fn finish_protocol_tasks(&mut self, deadline: Instant) {
        finish_or_abort_task(&mut self.reader, deadline).await;
        finish_or_abort_task(&mut self.writer, deadline).await;
        self.protocol_tasks_finished = true;
    }

    async fn abort_protocol_tasks(&mut self) {
        abort_task(&mut self.reader).await;
        abort_task(&mut self.writer).await;
        self.protocol_tasks_finished = true;
    }

    async fn finish_stderr(&mut self) -> Option<LspStderrSummary> {
        let drain = self.stderr.take()?;
        let capture = drain.finish(STDERR_FINISH_TIMEOUT).await.ok()?;
        Some(LspStderrSummary {
            observed_bytes: capture.observed_bytes(),
            truncated: capture.truncated(),
        })
    }
}

impl Drop for ManagedLspStdio {
    fn drop(&mut self) {
        if let Some(task) = self.reader.take() {
            task.abort();
        }
        if let Some(task) = self.writer.take() {
            task.abort();
        }
    }
}

async fn take_finished_task(
    task: &mut Option<JoinHandle<Result<(), LspStdioError>>>,
) -> Option<Result<(), LspStdioError>> {
    if !task.as_ref().is_some_and(|task| task.is_finished()) {
        return None;
    }
    let task = task.take()?;
    Some(task.await.unwrap_or(Err(LspStdioError::Process)))
}

async fn finish_or_abort_task(
    task: &mut Option<JoinHandle<Result<(), LspStdioError>>>,
    deadline: Instant,
) {
    let Some(mut task) = task.take() else {
        return;
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    if tokio::time::timeout(remaining, &mut task).await.is_err() {
        task.abort();
        let _ = task.await;
    }
}

async fn abort_task(task: &mut Option<JoinHandle<Result<(), LspStdioError>>>) {
    if let Some(task) = task.take() {
        task.abort();
        let _ = task.await;
    }
}
