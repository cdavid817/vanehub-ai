use super::bounded_stdio::{BoundedLineReader, BoundedLineStatus};
use super::runtime_logging::{self, McpRuntimeLogContext};
use crate::contexts::tooling::mcp::application::{McpExecutionControl, McpRuntimeError};
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use crate::platform::process::{ManagedTokioChild, TokioStderrDrain};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};
use tokio::process::{ChildStdin, ChildStdout};

const CLEANUP_RESERVE: Duration = Duration::from_millis(500);
const CANCELLATION_POLL: Duration = Duration::from_millis(10);

type ShutdownFuture = Pin<Box<dyn Future<Output = Result<(), McpRuntimeError>> + Send + 'static>>;
type HttpShutdown = Box<dyn FnOnce(Instant) -> ShutdownFuture + Send + 'static>;

pub(super) struct ManagedMcpSession<T> {
    operation: Option<tokio::task::JoinHandle<Result<T, McpRuntimeError>>>,
    resources: Option<OwnedSessionResources>,
}

enum OwnedSessionResources {
    Stdio {
        child: Box<ManagedTokioChild>,
        stderr_drain: TokioStderrDrain,
        frame_status: BoundedLineStatus,
        log_context: McpRuntimeLogContext,
    },
    Http {
        shutdown: HttpShutdown,
    },
}

impl<T: Send + 'static> ManagedMcpSession<T> {
    pub(super) fn spawn_stdio<F, Fut>(
        executable: &str,
        args: &[String],
        environment: &BTreeMap<String, String>,
        frame_limit: usize,
        stderr_limit: usize,
        log_context: McpRuntimeLogContext,
        operation: F,
    ) -> Result<Self, McpRuntimeError>
    where
        F: FnOnce(BoundedLineReader<ChildStdout>, ChildStdin) -> Fut,
        Fut: Future<Output = Result<T, McpRuntimeError>> + Send + 'static,
    {
        let mut child = ManagedTokioChild::spawn(executable, args, environment)
            .map_err(|error| runtime_error(McpFailureCode::Spawn, error))?;
        let stdout = child
            .take_stdout()
            .map_err(|error| runtime_error(McpFailureCode::Spawn, error))?;
        let stdin = child
            .take_stdin()
            .map_err(|error| runtime_error(McpFailureCode::Spawn, error))?;
        let stderr = child
            .take_stderr()
            .map_err(|error| runtime_error(McpFailureCode::Spawn, error))?;
        let stderr_drain = TokioStderrDrain::spawn(stderr, stderr_limit);
        let (stdout, frame_status) = BoundedLineReader::new(stdout, frame_limit);
        Ok(Self {
            operation: Some(tokio::spawn(operation(stdout, stdin))),
            resources: Some(OwnedSessionResources::Stdio {
                child: Box::new(child),
                stderr_drain,
                frame_status,
                log_context,
            }),
        })
    }

    pub(super) fn spawn_http<F, Fut, S, SFut>(operation: F, shutdown: S) -> Self
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, McpRuntimeError>> + Send + 'static,
        S: FnOnce(Instant) -> SFut + Send + 'static,
        SFut: Future<Output = Result<(), McpRuntimeError>> + Send + 'static,
    {
        Self {
            operation: Some(tokio::spawn(operation())),
            resources: Some(OwnedSessionResources::Http {
                shutdown: Box::new(move |deadline| Box::pin(shutdown(deadline))),
            }),
        }
    }

    pub(super) async fn run(mut self, control: &McpExecutionControl) -> Result<T, McpRuntimeError> {
        let mut result = self.await_operation(control).await;
        if self.protocol_limit_exceeded() {
            result = Err(McpRuntimeError::new(McpFailureCode::LimitExceeded));
        }
        let cleanup = self.shutdown(control).await;
        if result.is_ok() && control.is_cancelled() {
            result = Err(McpRuntimeError::new(McpFailureCode::Cancelled));
        }
        resolve_result(result, cleanup)
    }

    fn protocol_limit_exceeded(&self) -> bool {
        matches!(
            self.resources.as_ref(),
            Some(OwnedSessionResources::Stdio { frame_status, .. }) if frame_status.exceeded()
        )
    }

    async fn await_operation(
        &mut self,
        control: &McpExecutionControl,
    ) -> Result<T, McpRuntimeError> {
        let initial_remaining = control.deadline_remaining()?;
        let cleanup_reserve =
            CLEANUP_RESERVE.min(initial_remaining.saturating_sub(initial_remaining / 4));
        loop {
            if control.is_cancelled() {
                self.abort_operation().await;
                return Err(McpRuntimeError::new(McpFailureCode::Cancelled));
            }
            let remaining = control.deadline_remaining()?;
            if remaining <= cleanup_reserve {
                self.abort_operation().await;
                return Err(McpRuntimeError::new(McpFailureCode::Timeout));
            }
            let wait = CANCELLATION_POLL.min(remaining - cleanup_reserve);
            let operation = self
                .operation
                .as_mut()
                .ok_or_else(|| runtime_error(McpFailureCode::Cleanup, "operation unavailable"))?;
            match tokio::time::timeout(wait, operation).await {
                Ok(result) => {
                    self.operation.take();
                    return result.map_err(|error| {
                        runtime_error(McpFailureCode::Transport, error.to_string())
                    })?;
                }
                Err(_) => continue,
            }
        }
    }

    async fn abort_operation(&mut self) {
        if let Some(operation) = self.operation.take() {
            operation.abort();
            let _ = operation.await;
        }
    }

    async fn shutdown(&mut self, control: &McpExecutionControl) -> Result<(), McpRuntimeError> {
        self.abort_operation().await;
        match self
            .resources
            .take()
            .ok_or_else(|| runtime_error(McpFailureCode::Cleanup, "resources unavailable"))?
        {
            OwnedSessionResources::Stdio {
                child,
                stderr_drain,
                frame_status: _,
                log_context,
            } => shutdown_stdio(*child, stderr_drain, &log_context, control).await,
            OwnedSessionResources::Http { shutdown } => shutdown_http(shutdown, control).await,
        }
    }
}

async fn shutdown_stdio(
    mut child: ManagedTokioChild,
    stderr_drain: TokioStderrDrain,
    log_context: &McpRuntimeLogContext,
    control: &McpExecutionControl,
) -> Result<(), McpRuntimeError> {
    let deadline = Instant::now() + control.deadline_remaining().unwrap_or(Duration::ZERO);
    let child_result = child
        .shutdown(deadline)
        .await
        .map_err(|error| runtime_error(McpFailureCode::Cleanup, error));
    let drain_result = stderr_drain
        .finish(control.deadline_remaining().unwrap_or(Duration::ZERO))
        .await
        .map_err(|error| runtime_error(McpFailureCode::Cleanup, error));
    if let (Ok(status), Ok(capture)) = (&child_result, &drain_result) {
        runtime_logging::record_child_exit(
            log_context,
            status.code(),
            capture.observed_bytes(),
            capture.truncated(),
        );
    }
    child_result.map(|_| ())?;
    drain_result.map(|_| ())
}

async fn shutdown_http(
    shutdown: HttpShutdown,
    control: &McpExecutionControl,
) -> Result<(), McpRuntimeError> {
    let remaining = control.deadline_remaining().unwrap_or(Duration::ZERO);
    let deadline = Instant::now() + remaining;
    match tokio::time::timeout(remaining, shutdown(deadline)).await {
        Ok(result) => result,
        Err(_) => Err(McpRuntimeError::new(McpFailureCode::Cleanup)),
    }
}

pub(super) fn resolve_result<T>(
    result: Result<T, McpRuntimeError>,
    cleanup: Result<(), McpRuntimeError>,
) -> Result<T, McpRuntimeError> {
    match (result, cleanup) {
        (result, Ok(())) => result,
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(primary), Err(_)) => Err(primary),
    }
}

fn runtime_error(code: McpFailureCode, error: impl std::fmt::Display) -> McpRuntimeError {
    McpRuntimeError::with_diagnostic(code, error.to_string())
}
