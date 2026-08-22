//! One engine worker: handshake, request/reply correlation, cancellation, shutdown.
//!
//! The process is behind a transport trait so the lifecycle above it -- what happens on a
//! mismatched id, on contamination, on a hang, on a cancel the worker ignores -- is testable
//! without Python, a model, or a machine that happens to have the right packages installed. The
//! child-process implementation of that trait is thin by design.

use std::collections::BTreeMap;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use super::environment::worker_environment;
use super::protocol::{
    encode_control, encode_request, parse_response, request_params, validate_hello,
    MAX_REQUEST_FRAME_BYTES, MAX_RESPONSE_FRAME_BYTES,
};
use crate::contexts::local_media::application::worker_contract::{WorkerCall, WorkerReply};
use crate::contexts::local_media::domain::{
    LocalMediaEngine, LocalMediaError, LocalMediaErrorCode, LocalMediaProfileSnapshot,
};
use crate::platform::process::{BlockingStderrDrain, ManagedChild};

/// How often the read loop wakes to re-check cancellation while waiting for a reply.
const POLL_SLICE: Duration = Duration::from_millis(25);
const STDERR_LIMIT: usize = 16 * 1024;

pub(super) trait WorkerTransport: Send {
    fn send_line(&mut self, frame: &[u8]) -> Result<(), LocalMediaError>;
    /// `Ok(None)` means the slice elapsed with nothing to read; `Err` means the process is gone.
    fn recv_line(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, LocalMediaError>;
    fn terminate(&mut self);
}

pub(super) struct WorkerSession {
    engine: LocalMediaEngine,
    transport: Box<dyn WorkerTransport>,
    profile_revision: i64,
    poisoned: bool,
    request_counter: u64,
    operation_prefix: String,
}

impl WorkerSession {
    /// Complete the handshake or refuse the worker.
    ///
    /// Every failure here maps to `WORKER_START_FAILED` rather than to its cause. The distinction
    /// between "did not greet in time", "greeted as the wrong engine", and "printed a progress bar"
    /// matters to a developer reading redacted diagnostics, not to a user reading a settings badge.
    pub(super) fn establish(
        engine: LocalMediaEngine,
        mut transport: Box<dyn WorkerTransport>,
        handshake_timeout: Duration,
        profile_revision: i64,
    ) -> Result<Self, LocalMediaError> {
        let deadline = Instant::now() + handshake_timeout;
        loop {
            if Instant::now() >= deadline {
                transport.terminate();
                return Err(LocalMediaError::new(LocalMediaErrorCode::WorkerStartFailed)
                    .with_text("engine", engine.as_str()));
            }
            match transport.recv_line(POLL_SLICE.min(handshake_timeout)) {
                Ok(Some(line)) => {
                    // The hello frame is validated for shape and engine identity; its version
                    // field is not retained: engine status reports the probe's answer, which is
                    // the one the settings page shows.
                    let Ok(_hello) = validate_hello(&line, engine) else {
                        transport.terminate();
                        return Err(LocalMediaError::new(LocalMediaErrorCode::WorkerStartFailed)
                            .with_text("engine", engine.as_str()));
                    };
                    return Ok(Self {
                        engine,
                        transport,
                        profile_revision,
                        poisoned: false,
                        request_counter: 0,
                        operation_prefix: String::new(),
                    });
                }
                Ok(None) => continue,
                Err(_) => {
                    transport.terminate();
                    return Err(LocalMediaError::new(LocalMediaErrorCode::WorkerStartFailed)
                        .with_text("engine", engine.as_str()));
                }
            }
        }
    }

    pub(super) fn profile_revision(&self) -> i64 {
        self.profile_revision
    }

    /// True when the stream can no longer be trusted. The supervisor drops a poisoned session and
    /// starts a fresh worker for the next request.
    pub(super) fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub(super) fn call(
        &mut self,
        snapshot: &LocalMediaProfileSnapshot,
        call: &WorkerCall,
        cancelled: Arc<AtomicBool>,
        timeout: Duration,
        cancel_grace: Duration,
    ) -> Result<WorkerReply, LocalMediaError> {
        self.request_counter += 1;
        self.operation_prefix = snapshot.operation_id().as_str().to_string();
        let request_id = format!("{}-{}", self.operation_prefix, self.request_counter);
        let method = call.method(self.engine);
        let params = request_params(snapshot, call);

        let frame = encode_request(&request_id, method, params);
        if frame.len() > MAX_REQUEST_FRAME_BYTES {
            // Refusing here rather than leaving the bound to the reader keeps an oversized
            // request from reaching a worker that would have to reject it mid-stream.
            return Err(self.poison(LocalMediaErrorCode::WorkerProtocolError));
        }
        if self.transport.send_line(&frame).is_err() {
            return Err(self.poison(LocalMediaErrorCode::WorkerCrashed));
        }

        let deadline = Instant::now() + timeout;
        let mut cancel_deadline: Option<Instant> = None;

        loop {
            if cancel_deadline.is_none() && cancelled.load(Ordering::SeqCst) {
                // Cooperative first: the worker may be between pages or segments and can stop
                // cleanly. Only if it does not answer within the grace period is it killed.
                let _ = self
                    .transport
                    .send_line(&encode_control("cancel", &request_id));
                cancel_deadline = Some(Instant::now() + cancel_grace);
            }
            if cancel_deadline.is_some_and(|at| Instant::now() >= at) {
                self.transport.terminate();
                self.poisoned = true;
                return Err(
                    LocalMediaError::new(LocalMediaErrorCode::OperationCancelled)
                        .with_text("engine", self.engine.as_str()),
                );
            }
            if cancel_deadline.is_none() && Instant::now() >= deadline {
                return Err(self.poison(LocalMediaErrorCode::WorkerCrashed));
            }

            match self.transport.recv_line(POLL_SLICE) {
                Ok(Some(line)) => {
                    return match parse_response(&line, &request_id, call) {
                        Ok(reply) => Ok(reply),
                        Err(error) if error.code() == LocalMediaErrorCode::WorkerProtocolError => {
                            Err(self.poison(LocalMediaErrorCode::WorkerProtocolError))
                        }
                        // A mapped engine error means the worker is healthy and answering; the
                        // failure belongs to the request, not to the process.
                        Err(error) => Err(error),
                    };
                }
                Ok(None) => continue,
                Err(_) => return Err(self.poison(LocalMediaErrorCode::WorkerCrashed)),
            }
        }
    }

    pub(super) fn shutdown(&mut self, grace: Duration) {
        let _ = self
            .transport
            .send_line(&encode_control("shutdown", "shutdown"));
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline {
            match self.transport.recv_line(POLL_SLICE) {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) => continue,
            }
        }
        self.transport.terminate();
        self.poisoned = true;
    }

    fn poison(&mut self, code: LocalMediaErrorCode) -> LocalMediaError {
        self.poisoned = true;
        self.transport.terminate();
        LocalMediaError::new(code).with_text("engine", self.engine.as_str())
    }
}

/// The real transport: a child process speaking JSON Lines over stdio.
pub(super) struct ChildProcessTransport {
    child: ManagedChild,
    stdin: std::process::ChildStdin,
    inbound: Receiver<Result<Vec<u8>, ()>>,
    stderr: Option<BlockingStderrDrain>,
}

impl ChildProcessTransport {
    /// Launch the bundled bridge with the configured interpreter.
    ///
    /// Arguments go through the process API, never a shell string. That is what makes an
    /// interpreter path containing spaces or non-ASCII characters work identically on all three
    /// platforms, and it is why `cmd.exe` never appears anywhere in this path.
    pub(super) fn spawn(
        engine: LocalMediaEngine,
        python_executable: &str,
        bridge_root: &Path,
        media_root: &Path,
        working_directory: &Path,
        parent_environment: &BTreeMap<String, String>,
    ) -> Result<Self, LocalMediaError> {
        let start_failed = || {
            LocalMediaError::new(LocalMediaErrorCode::WorkerStartFailed)
                .with_text("engine", engine.as_str())
        };
        if !Path::new(python_executable).is_file() {
            return Err(LocalMediaError::new(LocalMediaErrorCode::PythonNotFound)
                .with_text("engine", engine.as_str()));
        }
        let args = vec![
            "-u".to_string(),
            "-m".to_string(),
            "vane_local_media_worker".to_string(),
            "--engine".to_string(),
            engine.worker_id().to_string(),
            "--protocol".to_string(),
            "1".to_string(),
        ];
        let environment = worker_environment(bridge_root, media_root, parent_environment);
        let mut child = ManagedChild::spawn_isolated(
            python_executable,
            &args,
            &environment,
            Some(working_directory),
        )
        .map_err(|_| start_failed())?;

        let stdin = child.take_stdin().map_err(|_| start_failed())?;
        let stdout = child.take_stdout().map_err(|_| start_failed())?;
        let stderr = child.take_stderr().map_err(|_| start_failed())?;
        Ok(Self {
            child,
            stdin,
            inbound: spawn_bounded_reader(stdout),
            stderr: Some(BlockingStderrDrain::spawn(stderr, STDERR_LIMIT)),
        })
    }
}

impl WorkerTransport for ChildProcessTransport {
    fn send_line(&mut self, frame: &[u8]) -> Result<(), LocalMediaError> {
        let mut line = frame.to_vec();
        line.push(b'\n');
        self.stdin
            .write_all(&line)
            .and_then(|_| self.stdin.flush())
            .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::WorkerCrashed))
    }

    fn recv_line(&mut self, timeout: Duration) -> Result<Option<Vec<u8>>, LocalMediaError> {
        match self.inbound.recv_timeout(timeout) {
            Ok(Ok(line)) => Ok(Some(line)),
            Ok(Err(())) => Err(LocalMediaError::new(
                LocalMediaErrorCode::WorkerProtocolError,
            )),
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) => {
                Err(LocalMediaError::new(LocalMediaErrorCode::WorkerCrashed))
            }
        }
    }

    fn terminate(&mut self) {
        let _ = self.child.shutdown(Instant::now() + Duration::from_secs(2));
        if let Some(stderr) = self.stderr.take() {
            // Draining is what keeps a worker's diagnostics from being lost on the kill path; the
            // capture is discarded here because only the supervisor is allowed to redact it.
            let _ = stderr.finish(Duration::from_millis(500));
        }
    }
}

/// Read newline-delimited frames on a dedicated thread with a hard per-line bound.
///
/// The bound is the reason this is not `BufRead::read_line`: a worker that never emits a newline
/// would otherwise grow one allocation until the process dies.
fn spawn_bounded_reader(stdout: std::process::ChildStdout) -> Receiver<Result<Vec<u8>, ()>> {
    let (sender, receiver) = mpsc::sync_channel(4);
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = Vec::with_capacity(8 * 1024);
        let mut byte = [0u8; 1];
        loop {
            match reader.read(&mut byte) {
                Ok(0) => break,
                Ok(_) if byte[0] == b'\n' => {
                    let frame = std::mem::take(&mut line);
                    if sender.send(Ok(frame)).is_err() {
                        break;
                    }
                    line = Vec::with_capacity(8 * 1024);
                }
                Ok(_) if line.len() >= MAX_RESPONSE_FRAME_BYTES => {
                    let _ = sender.send(Err(()));
                    break;
                }
                Ok(_) => line.push(byte[0]),
                Err(_) => break,
            }
        }
    });
    receiver
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
