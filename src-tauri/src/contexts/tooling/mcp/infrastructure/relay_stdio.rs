use super::relay_failure::RelayFailure;
use super::relay_jsonrpc::{CorrelatedRequest, JsonRpcCorrelation, RelayDirection};
use super::relay_observer::{RelayObserver, RelayRequest};
use super::relay_stdio_failure::{emit_pending_failures, emit_timeout, finish_correlated};
use super::relay_stdio_pump::{
    finish_pending, join_if_finished, spawn_pump, ClosableWriter, PumpEvent, PumpStop,
    StdioCorrelation,
};
use super::runtime_logging::{self, McpRuntimeLogContext};
use crate::contexts::tooling::mcp::application::{McpCancellation, McpLimits};
use crate::contexts::tooling::mcp::domain::McpFailureCode;
use crate::platform::process::{BlockingStderrDrain, ManagedChild};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const SUPERVISOR_POLL_INTERVAL: Duration = Duration::from_millis(10);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);

enum StopReason {
    ParentEof,
    UpstreamEof,
    ChildExit(std::process::ExitStatus),
    PumpFailure(String),
    Cancelled,
    RequestTimeout(CorrelatedRequest<Option<RelayRequest>>),
}

pub(super) fn run(
    executable: &str,
    args: &[String],
    env: &BTreeMap<String, String>,
    request_timeout: Duration,
    cancellation: McpCancellation,
    observer: Option<RelayObserver>,
    log_context: &McpRuntimeLogContext,
) -> Result<(), String> {
    runtime_logging::record_command_start(log_context);
    let child = ManagedChild::spawn(executable, args, env).map_err(|error| error.to_string())?;
    supervise(
        child,
        BufReader::new(std::io::stdin()),
        std::io::stdout(),
        request_timeout,
        cancellation,
        PumpStop::default(),
        observer,
        log_context,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn supervise<R, W>(
    mut child: ManagedChild,
    parent_input: R,
    parent_output: W,
    request_timeout: Duration,
    cancellation: McpCancellation,
    pump_stop: PumpStop,
    observer: Option<RelayObserver>,
    log_context: &McpRuntimeLogContext,
) -> Result<(), String>
where
    R: BufRead + Send + 'static,
    W: Write + Send + 'static,
{
    let child_stdin = child.take_stdin().map_err(|error| error.to_string())?;
    let child_stdout = child.take_stdout().map_err(|error| error.to_string())?;
    let child_stderr = child.take_stderr().map_err(|error| error.to_string())?;
    let child_input = ClosableWriter::new(child_stdin);
    let parent_output = ClosableWriter::new(parent_output);
    let correlation = Arc::new(Mutex::new(JsonRpcCorrelation::default()));
    let (events, receiver) = mpsc::channel();
    let input = spawn_pump(
        parent_input,
        child_input.clone(),
        RelayDirection::ParentToUpstream,
        observer.clone(),
        Arc::clone(&correlation),
        events.clone(),
    );
    let output = spawn_pump(
        BufReader::new(child_stdout),
        parent_output.clone(),
        RelayDirection::UpstreamToParent,
        observer.clone(),
        Arc::clone(&correlation),
        events,
    );
    let stderr = BlockingStderrDrain::spawn(child_stderr, McpLimits::DEFAULT.stderr_bytes);
    let request_timeout = request_timeout.max(Duration::from_millis(1));
    let reason = supervise_until_terminal(
        &mut child,
        &receiver,
        &correlation,
        request_timeout,
        &cancellation,
    )
    .unwrap_or_else(StopReason::PumpFailure);

    let request_emit_error = match &reason {
        StopReason::RequestTimeout(expired) => {
            emit_timeout(expired, &child_input, &parent_output).err()
        }
        _ => None,
    };
    let failure = reason.failure();
    let pending = correlation
        .lock()
        .map(|mut correlation| correlation.close_and_drain_correlated())
        .unwrap_or_default();
    let pending_emit_error =
        emit_pending_failures(failure, &pending, &child_input, &parent_output).err();
    if let StopReason::RequestTimeout(expired) = reason {
        finish_pending(
            observer.as_ref(),
            expired.pending,
            false,
            Some(failure.classification()),
        );
        finish_correlated(observer.as_ref(), pending, failure.classification());
        finish_supervision(
            child,
            child_input,
            pump_stop,
            input,
            output,
            stderr,
            log_context,
        )?;
        if let Some(error) = request_emit_error.or(pending_emit_error) {
            return Err(format!("MCP stdio timeout response failed: {error}"));
        }
        return Err("MCP stdio request timed out".to_string());
    }

    finish_correlated(observer.as_ref(), pending, failure.classification());
    let terminal_result = reason.result();
    finish_supervision(
        child,
        child_input,
        pump_stop,
        input,
        output,
        stderr,
        log_context,
    )?;
    if let Some(error) = request_emit_error.or(pending_emit_error) {
        return Err(format!("MCP stdio failure response failed: {error}"));
    }
    terminal_result
}

fn supervise_until_terminal(
    child: &mut ManagedChild,
    receiver: &mpsc::Receiver<PumpEvent>,
    correlation: &StdioCorrelation,
    request_timeout: Duration,
    cancellation: &McpCancellation,
) -> Result<StopReason, String> {
    loop {
        if let Some(status) = child
            .wait_until(Instant::now())
            .map_err(|error| error.to_string())?
        {
            return Ok(StopReason::ChildExit(status));
        }
        if cancellation.is_cancelled() {
            return Ok(StopReason::Cancelled);
        }
        if let Some(expired) = correlation
            .lock()
            .map_err(|_| "relay correlation state is unavailable".to_string())?
            .take_expired(Instant::now(), request_timeout)
        {
            return Ok(StopReason::RequestTimeout(expired));
        }
        let wait = next_poll_delay(correlation, request_timeout)?;
        match receiver.recv_timeout(wait) {
            Ok(PumpEvent::Ended(RelayDirection::ParentToUpstream, Ok(()))) => {
                return Ok(StopReason::ParentEof)
            }
            Ok(PumpEvent::Ended(RelayDirection::UpstreamToParent, Ok(()))) => {
                return Ok(StopReason::UpstreamEof)
            }
            Ok(PumpEvent::Ended(_, Err(error))) => return Ok(StopReason::PumpFailure(error)),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Ok(StopReason::PumpFailure(
                    "relay pumps disconnected".to_string(),
                ))
            }
        }
    }
}

fn next_poll_delay(
    correlation: &StdioCorrelation,
    request_timeout: Duration,
) -> Result<Duration, String> {
    let deadline = correlation
        .lock()
        .map_err(|_| "relay correlation state is unavailable".to_string())?
        .oldest_deadline(request_timeout);
    Ok(deadline.map_or(SUPERVISOR_POLL_INTERVAL, |deadline| {
        SUPERVISOR_POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now()))
    }))
}

fn finish_supervision<W>(
    mut child: ManagedChild,
    child_input: ClosableWriter<W>,
    pump_stop: PumpStop,
    input: JoinHandle<()>,
    output: JoinHandle<()>,
    stderr: BlockingStderrDrain,
    log_context: &McpRuntimeLogContext,
) -> Result<(), String> {
    pump_stop.stop();
    child_input.close();
    let shutdown = child.shutdown(Instant::now() + SHUTDOWN_TIMEOUT);
    drop(child);
    join_if_finished(input);
    join_if_finished(output);
    let capture = stderr.finish().map_err(|error| error.to_string())?;
    let status = shutdown.map_err(|error| error.to_string())?;
    runtime_logging::record_child_exit(
        log_context,
        status.code(),
        capture.observed_bytes(),
        capture.truncated(),
    );
    Ok(())
}

impl StopReason {
    fn failure(&self) -> RelayFailure {
        match self {
            Self::Cancelled => RelayFailure::new(McpFailureCode::Cancelled),
            Self::RequestTimeout(_) => RelayFailure::new(McpFailureCode::Timeout),
            Self::ParentEof | Self::UpstreamEof | Self::ChildExit(_) | Self::PumpFailure(_) => {
                RelayFailure::new(McpFailureCode::Transport)
            }
        }
    }

    fn result(self) -> Result<(), String> {
        match self {
            Self::ParentEof | Self::UpstreamEof => Ok(()),
            Self::ChildExit(status) if status.success() => Ok(()),
            Self::ChildExit(_) => Err("MCP stdio upstream exited unsuccessfully".to_string()),
            Self::PumpFailure(error) => Err(error),
            Self::Cancelled => Err("MCP stdio relay was cancelled".to_string()),
            Self::RequestTimeout(_) => Err("MCP stdio request timed out".to_string()),
        }
    }
}

#[cfg(test)]
#[path = "relay_stdio_tests.rs"]
mod tests;
