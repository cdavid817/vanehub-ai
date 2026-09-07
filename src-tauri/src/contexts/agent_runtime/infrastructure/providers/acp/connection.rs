//! One ACP stdio connection: a reader thread, a serialized writer, request correlation, and the
//! owned child process behind them.
//!
//! The reader never waits on anything but the pipe. Responses are delivered straight to the
//! waiter that sent the request; requests and notifications from the agent go onto a bounded
//! queue the turn driver drains. That split is what prevents the deadlock the protocol warns
//! about: while the host waits for `session/prompt` to answer, the agent's
//! `session/request_permission` still arrives, still gets queued, and still gets answered.
//!
//! Every connection has an epoch. Anything stamped with an older epoch -- a late reply, a stale
//! approval, a terminal id -- is refused, so a reconnection can never be steered by messages that
//! belonged to the process before it.

use super::budget::{
    INBOUND_QUEUE_CAPACITY, MAX_PENDING_OUTBOUND, SHUTDOWN_DEADLINE, STDERR_TAIL_BYTES,
};
use super::framing::{encode_frame, FramingError, NdjsonDecoder};
use super::jsonrpc::{
    classify, notification_document, request_document, response_document, InboundMessage, RpcError,
    RpcId,
};
use crate::platform::process::ManagedChild;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

static NEXT_EPOCH: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectionFailure {
    pub(crate) reason_code: &'static str,
    pub(crate) detail: String,
}

impl ConnectionFailure {
    fn new(reason_code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            reason_code,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AcpError {
    Spawn(String),
    /// The connection is closed; the failure says why (EOF, framing, spawn, termination).
    Closed(ConnectionFailure),
    /// Too many outbound requests are still unanswered.
    Backpressure {
        pending: usize,
    },
    Io(String),
    Framing(FramingError),
    /// The peer answered in a way the host cannot accept.
    Protocol {
        reason_code: &'static str,
        detail: String,
    },
    Timeout(&'static str),
    /// The agent refused because nobody is signed in (JSON-RPC `-32000`, the code ACP reserves
    /// for `auth_required`). Not a protocol violation: the program works, the account does not.
    AuthRequired {
        detail: String,
    },
}

impl std::fmt::Display for AcpError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(detail) => write!(formatter, "ACP agent could not be started: {detail}"),
            Self::Closed(failure) => write!(
                formatter,
                "ACP connection closed ({}): {}",
                failure.reason_code, failure.detail
            ),
            Self::Backpressure { pending } => {
                write!(
                    formatter,
                    "ACP connection has {pending} unanswered requests"
                )
            }
            Self::Io(detail) => write!(formatter, "ACP transport I/O failed: {detail}"),
            Self::Framing(error) => write!(formatter, "{error}"),
            Self::AuthRequired { detail } => {
                write!(formatter, "ACP agent requires sign-in: {detail}")
            }
            Self::Protocol {
                reason_code,
                detail,
            } => write!(
                formatter,
                "ACP protocol violation ({reason_code}): {detail}"
            ),
            Self::Timeout(stage) => write!(formatter, "ACP {stage} timed out"),
        }
    }
}

impl AcpError {
    pub(crate) fn reason_code(&self) -> &'static str {
        match self {
            Self::Spawn(_) => "acp-spawn-failed",
            Self::Closed(failure) => failure.reason_code,
            Self::Backpressure { .. } => "acp-backpressure",
            Self::Io(_) => "acp-io-failed",
            Self::Framing(error) => error.reason_code(),
            Self::Protocol { reason_code, .. } => reason_code,
            Self::Timeout(_) => "acp-timeout",
            Self::AuthRequired { .. } => "acp-authentication-required",
        }
    }
}

/// What the reader hands to the turn driver.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum InboundEvent {
    Request {
        id: RpcId,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
    /// The reader stopped. Delivered exactly once, after which the queue is closed.
    Closed(ConnectionFailure),
}

/// The process (or, in tests, the thread) behind a connection.
pub(crate) trait AcpChildControl: Send {
    /// `Some(code)` once the child has exited; `None` while it runs.
    fn poll_exit(&mut self) -> Option<Option<i32>>;
    /// Terminates the owned process tree and waits until `deadline`. Never touches processes the
    /// host did not spawn.
    fn terminate(&mut self, deadline: Instant) -> Result<Option<i32>, String>;
    fn process_id(&self) -> Option<u32>;
}

struct ManagedChildControl(ManagedChild);

impl AcpChildControl for ManagedChildControl {
    fn poll_exit(&mut self) -> Option<Option<i32>> {
        match self.0.wait_until(Instant::now()) {
            Ok(Some(status)) => Some(status.code()),
            Ok(None) => None,
            Err(_) => Some(None),
        }
    }

    fn terminate(&mut self, deadline: Instant) -> Result<Option<i32>, String> {
        self.0
            .shutdown(deadline)
            .map(|status| status.code())
            .map_err(|error| error.to_string())
    }

    fn process_id(&self) -> Option<u32> {
        self.0.id()
    }
}

/// A child that is a thread rather than a process: it exits when its pipe closes.
#[cfg(test)]
pub(crate) struct DetachedChildControl;

#[cfg(test)]
impl AcpChildControl for DetachedChildControl {
    fn poll_exit(&mut self) -> Option<Option<i32>> {
        None
    }

    fn terminate(&mut self, _deadline: Instant) -> Result<Option<i32>, String> {
        Ok(None)
    }

    fn process_id(&self) -> Option<u32> {
        None
    }
}

#[derive(Debug, Default)]
struct StderrTail {
    retained: Vec<u8>,
    truncated: bool,
    observed_bytes: u64,
}

impl StderrTail {
    fn push(&mut self, chunk: &[u8]) {
        self.observed_bytes = self
            .observed_bytes
            .saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
        self.retained.extend_from_slice(chunk);
        if self.retained.len() > STDERR_TAIL_BYTES {
            let excess = self.retained.len() - STDERR_TAIL_BYTES;
            self.retained.drain(..excess);
            self.truncated = true;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StderrSummary {
    pub(crate) redacted_tail: String,
    pub(crate) truncated: bool,
    pub(crate) observed_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConnectionState {
    Open,
    Closed(ConnectionFailure),
}

pub(crate) struct AcpLaunchSpec {
    pub(crate) executable: String,
    pub(crate) args: Vec<String>,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) cwd: Option<String>,
}

pub(crate) struct AcpConnection {
    epoch: u64,
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    pending: Mutex<HashMap<u64, mpsc::Sender<Result<Value, RpcError>>>>,
    next_id: AtomicU64,
    inbound: Mutex<Receiver<InboundEvent>>,
    child: Mutex<Box<dyn AcpChildControl>>,
    state: Mutex<ConnectionState>,
    stderr: Arc<Mutex<StderrTail>>,
    late_responses: AtomicU64,
}

/// A response the connection will deliver when the peer answers.
pub(crate) struct PendingResponse {
    #[cfg_attr(not(test), allow(dead_code))]
    id: u64,
    receiver: Receiver<Result<Value, RpcError>>,
}

impl PendingResponse {
    #[cfg(test)]
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    pub(crate) fn try_recv(&self) -> Option<Result<Result<Value, RpcError>, AcpError>> {
        match self.receiver.try_recv() {
            Ok(outcome) => Some(Ok(outcome)),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(AcpError::Closed(
                ConnectionFailure::new("acp-connection-closed", "response channel closed"),
            ))),
        }
    }

    pub(crate) fn recv_timeout(
        &self,
        timeout: Duration,
        stage: &'static str,
    ) -> Result<Result<Value, RpcError>, AcpError> {
        match self.receiver.recv_timeout(timeout) {
            Ok(outcome) => Ok(outcome),
            Err(RecvTimeoutError::Timeout) => Err(AcpError::Timeout(stage)),
            Err(RecvTimeoutError::Disconnected) => Err(AcpError::Closed(ConnectionFailure::new(
                "acp-connection-closed",
                "response channel closed",
            ))),
        }
    }
}

impl AcpConnection {
    /// Spawns the agent and wires its pipes. The child is contained by the platform process
    /// layer (process group / job object), so terminating it reaps whatever it spawned.
    pub(crate) fn spawn(spec: &AcpLaunchSpec) -> Result<Arc<Self>, AcpError> {
        let cwd = spec.cwd.as_deref().map(Path::new);
        let mut child =
            ManagedChild::spawn_in(&spec.executable, &spec.args, &spec.environment, cwd)
                .map_err(|error| AcpError::Spawn(error.to_string()))?;
        let stdin = child
            .take_stdin()
            .map_err(|error| AcpError::Spawn(error.to_string()))?;
        let stdout = child
            .take_stdout()
            .map_err(|error| AcpError::Spawn(error.to_string()))?;
        let stderr = child
            .take_stderr()
            .map_err(|error| AcpError::Spawn(error.to_string()))?;
        Ok(Self::from_streams(
            Box::new(stdout),
            Box::new(stdin),
            Some(Box::new(stderr)),
            Box::new(ManagedChildControl(child)),
        ))
    }

    /// Builds a connection over caller-supplied streams. Production uses `spawn`; tests use an
    /// in-process fake agent on the other end of a pipe pair.
    pub(crate) fn from_streams(
        reader: Box<dyn Read + Send>,
        writer: Box<dyn Write + Send>,
        stderr: Option<Box<dyn Read + Send>>,
        child: Box<dyn AcpChildControl>,
    ) -> Arc<Self> {
        let (inbound_tx, inbound_rx) = mpsc::sync_channel(INBOUND_QUEUE_CAPACITY);
        let connection = Arc::new(Self {
            epoch: NEXT_EPOCH.fetch_add(1, Ordering::Relaxed),
            writer: Mutex::new(Some(writer)),
            pending: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            inbound: Mutex::new(inbound_rx),
            child: Mutex::new(child),
            state: Mutex::new(ConnectionState::Open),
            stderr: Arc::new(Mutex::new(StderrTail::default())),
            late_responses: AtomicU64::new(0),
        });
        if let Some(stderr) = stderr {
            let tail = connection.stderr.clone();
            thread::spawn(move || drain_stderr(stderr, tail));
        }
        let reader_connection = Arc::downgrade(&connection);
        thread::spawn(move || read_loop(reader, inbound_tx, reader_connection));
        connection
    }

    pub(crate) fn epoch(&self) -> u64 {
        self.epoch
    }

    pub(crate) fn process_id(&self) -> Option<u32> {
        lock(&self.child).process_id()
    }

    pub(crate) fn is_open(&self) -> bool {
        matches!(*lock(&self.state), ConnectionState::Open)
    }

    pub(crate) fn failure(&self) -> Option<ConnectionFailure> {
        match &*lock(&self.state) {
            ConnectionState::Open => None,
            ConnectionState::Closed(failure) => Some(failure.clone()),
        }
    }

    /// How many responses arrived for ids nobody was waiting on. Non-zero means the peer answered
    /// late or twice; it is diagnostic, never fatal.
    pub(crate) fn late_responses(&self) -> u64 {
        self.late_responses.load(Ordering::Relaxed)
    }

    pub(crate) fn request(&self, method: &str, params: Value) -> Result<PendingResponse, AcpError> {
        if let Some(failure) = self.failure() {
            return Err(AcpError::Closed(failure));
        }
        let (sender, receiver) = mpsc::channel();
        let id = {
            let mut pending = lock(&self.pending);
            if pending.len() >= MAX_PENDING_OUTBOUND {
                return Err(AcpError::Backpressure {
                    pending: pending.len(),
                });
            }
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
            pending.insert(id, sender);
            id
        };
        if let Err(error) = self.write_document(&request_document(id, method, params)) {
            lock(&self.pending).remove(&id);
            return Err(error);
        }
        Ok(PendingResponse { id, receiver })
    }

    pub(crate) fn notify(&self, method: &str, params: Value) -> Result<(), AcpError> {
        if let Some(failure) = self.failure() {
            return Err(AcpError::Closed(failure));
        }
        self.write_document(&notification_document(method, params))
    }

    pub(crate) fn respond(
        &self,
        id: &RpcId,
        outcome: Result<Value, RpcError>,
    ) -> Result<(), AcpError> {
        if let Some(failure) = self.failure() {
            return Err(AcpError::Closed(failure));
        }
        self.write_document(&response_document(id, outcome))
    }

    /// The next agent-originated message, or a timeout. `Closed` is delivered once, after which
    /// the queue reports disconnection.
    pub(crate) fn next_inbound(&self, timeout: Duration) -> Result<InboundEvent, RecvTimeoutError> {
        lock(&self.inbound).recv_timeout(timeout)
    }

    /// Exit status if the child has already exited, without waiting.
    pub(crate) fn poll_child_exit(&self) -> Option<Option<i32>> {
        lock(&self.child).poll_exit()
    }

    /// Closes stdin and terminates the owned process tree. Idempotent.
    pub(crate) fn terminate(
        &self,
        reason_code: &'static str,
        detail: &str,
    ) -> Result<Option<i32>, AcpError> {
        self.close(ConnectionFailure::new(reason_code, detail));
        lock(&self.writer).take();
        let outcome = lock(&self.child)
            .terminate(Instant::now() + SHUTDOWN_DEADLINE)
            .map_err(AcpError::Io);
        lock(&self.pending).clear();
        outcome
    }

    pub(crate) fn stderr_summary(&self) -> StderrSummary {
        let tail = lock(&self.stderr);
        StderrSummary {
            redacted_tail: crate::platform::logging::redact_text(&String::from_utf8_lossy(
                &tail.retained,
            )),
            truncated: tail.truncated,
            observed_bytes: tail.observed_bytes,
        }
    }

    fn write_document(&self, document: &Value) -> Result<(), AcpError> {
        let frame = encode_frame(document).map_err(AcpError::Framing)?;
        let mut guard = lock(&self.writer);
        let writer = guard.as_mut().ok_or_else(|| {
            AcpError::Closed(ConnectionFailure::new(
                "acp-connection-closed",
                "stdin already closed",
            ))
        })?;
        writer
            .write_all(&frame)
            .and_then(|()| writer.flush())
            .map_err(|error| {
                let failure = ConnectionFailure::new("acp-broken-pipe", error.to_string());
                drop(guard);
                self.close(failure.clone());
                AcpError::Closed(failure)
            })
    }

    fn close(&self, failure: ConnectionFailure) {
        let mut state = lock(&self.state);
        if matches!(*state, ConnectionState::Open) {
            *state = ConnectionState::Closed(failure);
        }
    }

    fn deliver_response(&self, id: &RpcId, outcome: Result<Value, RpcError>) {
        let RpcId::Number(id) = id else {
            self.late_responses.fetch_add(1, Ordering::Relaxed);
            return;
        };
        match lock(&self.pending).remove(id) {
            Some(sender) => {
                let _ = sender.send(outcome);
            }
            None => {
                self.late_responses.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

impl Drop for AcpConnection {
    fn drop(&mut self) {
        // A dropped connection must not leave an owned process behind. The lock cannot be
        // contended here: `Drop` has exclusive access.
        if let Ok(child) = self.child.get_mut() {
            let _ = child.terminate(Instant::now() + Duration::from_millis(500));
        }
    }
}

fn read_loop(
    mut reader: Box<dyn Read + Send>,
    inbound: SyncSender<InboundEvent>,
    connection: std::sync::Weak<AcpConnection>,
) {
    let mut decoder = NdjsonDecoder::default();
    let mut chunk = [0_u8; 16 * 1024];
    let failure = loop {
        let read = match reader.read(&mut chunk) {
            Ok(0) => {
                break ConnectionFailure::new(
                    "acp-connection-eof",
                    if decoder.pending_bytes() > 0 {
                        "agent closed stdout mid-frame"
                    } else {
                        "agent closed stdout"
                    },
                )
            }
            Ok(read) => read,
            Err(error) => break ConnectionFailure::new("acp-read-failed", error.to_string()),
        };
        let frames = match decoder.push(&chunk[..read]) {
            Ok(frames) => frames,
            Err(error) => break ConnectionFailure::new(error.reason_code(), error.to_string()),
        };
        for frame in frames {
            let message = match classify(&frame) {
                Ok(message) => message,
                Err(error) => {
                    // Not a JSON-RPC message even though it parsed as JSON: the peer is not
                    // speaking the protocol. Fail rather than guess.
                    let failure =
                        ConnectionFailure::new("acp-stdout-not-protocol", error.to_string());
                    finish_read_loop(&inbound, &connection, failure);
                    return;
                }
            };
            match message {
                InboundMessage::Response { id, outcome } => match connection.upgrade() {
                    Some(connection) => connection.deliver_response(&id, outcome),
                    None => return,
                },
                InboundMessage::Request { id, method, params } => {
                    if inbound
                        .send(InboundEvent::Request { id, method, params })
                        .is_err()
                    {
                        return;
                    }
                }
                InboundMessage::Notification { method, params } => {
                    if inbound
                        .send(InboundEvent::Notification { method, params })
                        .is_err()
                    {
                        return;
                    }
                }
            }
        }
    };
    finish_read_loop(&inbound, &connection, failure);
}

fn finish_read_loop(
    inbound: &SyncSender<InboundEvent>,
    connection: &std::sync::Weak<AcpConnection>,
    failure: ConnectionFailure,
) {
    if let Some(connection) = connection.upgrade() {
        connection.close(failure.clone());
        // Waiters on outbound requests learn of the closure through their channel closing.
        lock(&connection.pending).clear();
    }
    let _ = inbound.send(InboundEvent::Closed(failure));
}

fn drain_stderr(mut stderr: Box<dyn Read + Send>, tail: Arc<Mutex<StderrTail>>) {
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        match stderr.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => lock(&tail).push(&chunk[..read]),
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
pub(crate) mod test_support {
    //! An in-process fake agent: a thread on the far end of two pipes that speaks ACP.

    use super::*;
    use std::io::{BufRead, BufReader};

    /// A scripted peer. `handler` receives every host message and returns frames to write back.
    pub(crate) struct FakeAgent {
        pub(crate) connection: Arc<AcpConnection>,
        pub(crate) received: Arc<Mutex<Vec<Value>>>,
    }

    pub(crate) fn fake_agent<F>(mut handler: F) -> FakeAgent
    where
        F: FnMut(&Value) -> Vec<Value> + Send + 'static,
    {
        let (agent_reads, host_writes) = std::io::pipe().expect("host->agent pipe");
        let (host_reads, mut agent_writes) = std::io::pipe().expect("agent->host pipe");
        let received = Arc::new(Mutex::new(Vec::new()));
        let recorded = received.clone();
        thread::spawn(move || {
            let reader = BufReader::new(agent_reads);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                let Ok(document) = serde_json::from_str::<Value>(&line) else {
                    break;
                };
                lock(&recorded).push(document.clone());
                for reply in handler(&document) {
                    // A scripted abrupt exit: drop the writer so the host sees EOF mid-turn.
                    if reply.get("__close__").is_some() {
                        return;
                    }
                    let frame = encode_frame(&reply).expect("fake frame");
                    if agent_writes.write_all(&frame).is_err() {
                        return;
                    }
                }
            }
        });
        let connection = AcpConnection::from_streams(
            Box::new(host_reads),
            Box::new(host_writes),
            None,
            Box::new(DetachedChildControl),
        );
        FakeAgent {
            connection,
            received,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::fake_agent;
    use super::*;
    use serde_json::json;

    fn reply(document: &Value, result: Value) -> Value {
        json!({"jsonrpc":"2.0","id":document["id"],"result":result})
    }

    #[test]
    fn requests_correlate_responses_by_id_and_late_responses_are_counted() {
        let agent = fake_agent(|document| {
            if document["method"] == "ping" {
                vec![
                    // A response for an id nobody is waiting on must not be misdelivered.
                    json!({"jsonrpc":"2.0","id":999,"result":{"stale":true}}),
                    reply(document, json!({"pong": document["params"]["n"]})),
                ]
            } else {
                Vec::new()
            }
        });
        let first = agent
            .connection
            .request("ping", json!({"n":1}))
            .expect("send");
        let second = agent
            .connection
            .request("ping", json!({"n":2}))
            .expect("send");
        assert_ne!(first.id(), second.id());
        let one = second
            .recv_timeout(Duration::from_secs(5), "ping")
            .expect("response")
            .expect("ok");
        let zero = first
            .recv_timeout(Duration::from_secs(5), "ping")
            .expect("response")
            .expect("ok");
        assert_eq!(zero["pong"], json!(1));
        assert_eq!(one["pong"], json!(2));
        // Give the reader a moment to count the stale reply; it is delivered before the second
        // real one, so by the time both answered it has been seen.
        assert!(agent.connection.late_responses() >= 1);
        assert!(agent.connection.is_open());
    }

    #[test]
    fn agent_requests_are_queued_while_a_host_request_is_outstanding() {
        // The agent answers the prompt only after the host has answered its permission request:
        // the classic recursive interaction that deadlocks a synchronous client.
        let agent = fake_agent(|document| match document["method"].as_str() {
            Some("session/prompt") => vec![json!({
                "jsonrpc":"2.0","id":41,"method":"session/request_permission",
                "params":{"sessionId":"s","toolCall":{"toolCallId":"c1"},"options":[{"optionId":"o1","name":"Allow","kind":"allow_once"}]}
            })],
            // The host's reply to the permission request is what unblocks the prompt.
            _ if document.get("id") == Some(&json!(41)) => {
                vec![json!({"jsonrpc":"2.0","id":7,"result":{"stopReason":"end_turn"}})]
            }
            _ => Vec::new(),
        });
        // Use a fixed id so the fake can answer the prompt after the permission reply arrives.
        let _ = agent.connection.next_id.swap(7, Ordering::Relaxed);
        let prompt = agent
            .connection
            .request("session/prompt", json!({"sessionId":"s"}))
            .expect("prompt");
        assert_eq!(prompt.id(), 7);
        assert!(prompt.try_recv().is_none());
        let inbound = agent
            .connection
            .next_inbound(Duration::from_secs(5))
            .expect("inbound");
        let InboundEvent::Request { id, method, .. } = inbound else {
            panic!("expected request, got {inbound:?}");
        };
        assert_eq!(method, "session/request_permission");
        agent
            .connection
            .respond(
                &id,
                Ok(json!({"outcome":{"outcome":"selected","optionId":"o1"}})),
            )
            .expect("respond");
        let outcome = prompt
            .recv_timeout(Duration::from_secs(5), "prompt")
            .expect("prompt outcome")
            .expect("ok");
        assert_eq!(outcome["stopReason"], json!("end_turn"));
        let received = lock(&agent.received);
        assert_eq!(received.len(), 2);
        assert_eq!(received[1]["id"], json!(41));
        assert_eq!(received[1]["result"]["outcome"]["optionId"], json!("o1"));
    }

    #[test]
    fn banner_on_stdout_closes_the_connection_with_a_typed_reason() {
        let agent = fake_agent(|_| Vec::new());
        // Write a banner directly by sending a frame the classifier rejects.
        let raw = fake_agent(|document| {
            vec![json!({"type":"banner","text":format!("Welcome {}", document["method"])})]
        });
        drop(agent);
        let pending = raw
            .connection
            .request("initialize", json!({}))
            .expect("send");
        let error = pending
            .recv_timeout(Duration::from_secs(5), "initialize")
            .expect_err("closed");
        assert!(matches!(error, AcpError::Closed(_)));
        let closed = raw
            .connection
            .next_inbound(Duration::from_secs(5))
            .expect("closed event");
        assert!(matches!(
            closed,
            InboundEvent::Closed(failure) if failure.reason_code == "acp-stdout-not-protocol"
        ));
        assert!(!raw.connection.is_open());
        assert!(raw.connection.request("x", json!({})).is_err());
    }

    #[test]
    fn eof_is_reported_once_and_pending_waiters_are_released() {
        let (host_reads, agent_writes) = std::io::pipe().expect("pipe");
        let (_agent_reads, host_writes) = std::io::pipe().expect("pipe");
        let connection = AcpConnection::from_streams(
            Box::new(host_reads),
            Box::new(host_writes),
            None,
            Box::new(DetachedChildControl),
        );
        let pending = connection
            .request("session/prompt", json!({}))
            .expect("send");
        drop(agent_writes);
        let error = pending
            .recv_timeout(Duration::from_secs(5), "prompt")
            .expect_err("released");
        assert!(matches!(error, AcpError::Closed(_)));
        let event = connection
            .next_inbound(Duration::from_secs(5))
            .expect("closed event");
        assert!(matches!(
            event,
            InboundEvent::Closed(failure) if failure.reason_code == "acp-connection-eof"
        ));
        assert!(matches!(
            connection.next_inbound(Duration::from_millis(50)),
            Err(RecvTimeoutError::Disconnected)
        ));
        assert_eq!(
            connection.failure().map(|failure| failure.reason_code),
            Some("acp-connection-eof")
        );
    }

    #[test]
    fn pending_request_budget_applies_backpressure() {
        let agent = fake_agent(|_| Vec::new());
        let mut held = Vec::new();
        for _ in 0..MAX_PENDING_OUTBOUND {
            held.push(
                agent
                    .connection
                    .request("x", json!({}))
                    .expect("within budget"),
            );
        }
        assert!(matches!(
            agent.connection.request("x", json!({})),
            Err(AcpError::Backpressure { pending }) if pending == MAX_PENDING_OUTBOUND
        ));
    }

    #[test]
    fn stderr_tail_is_bounded_and_redacted() {
        let mut tail = StderrTail::default();
        tail.push(b"api_key=sk-abcdefghijklmnopqrstuvwxyz0123456789 ");
        tail.push(&vec![b'z'; STDERR_TAIL_BYTES + 10]);
        assert!(tail.truncated);
        assert_eq!(tail.retained.len(), STDERR_TAIL_BYTES);
        let text = crate::platform::logging::redact_text(&String::from_utf8_lossy(&tail.retained));
        assert!(!text.contains("abcdefghijklmnopqrstuvwxyz0123456789"));
        assert_eq!(AcpError::Timeout("x").reason_code(), "acp-timeout");
        assert_eq!(
            AcpError::Backpressure { pending: 1 }.reason_code(),
            "acp-backpressure"
        );
    }
}
