//! One round trip to the remote helper.
//!
//! The command is a constant. It contains no path, no session id, and no shell metacharacter that
//! matters — the helper program itself arrives base64-encoded on stdin, and the request arrives
//! after it. That arrangement is the point: with nothing interpolated, "this cannot be a shell
//! injection" is a property of the shape rather than a rule somebody has to keep applying to every
//! new field.
//!
//! Everything is bounded. The request is refused before it is sent if it is too large, the response
//! stops accumulating at its ceiling, and the whole exchange is under a wall-clock timeout — which
//! is the one that catches the failure the others miss: a remote that accepted the connection and
//! then never answered, where every individual read succeeds and only the total says so.
//!
//! Nothing from stderr is kept. It is the remote host's own diagnostics, written by programs this
//! process did not choose, and it routinely contains paths and usernames. A client cannot redact
//! what it does not understand, so it does not collect it.

use super::protocol::{
    HelperRequest, HelperResponse, RemoteHelperError, HELPER_TIMEOUT_SECONDS, HELPER_VERSION,
    MAX_HELPER_REQUEST_BYTES, MAX_HELPER_RESPONSE_BYTES,
};
use crate::contexts::workspaces::application::SearchCancellationToken;
use async_trait::async_trait;
use base64::Engine;
use std::time::Duration;

/// The whole remote command.
///
/// Reads one line — the base64 program — and executes it; the program then reads the rest of stdin
/// as its request. `-I` isolates the interpreter (no environment, no user site directory) and `-S`
/// skips `site`, so the helper runs the same way on a host with an unusual Python installation as
/// on a plain one.
///
/// Double quotes with no `$`, no backtick, and no backslash inside, so every shell that could be
/// the remote login shell parses this identically.
pub(crate) const HELPER_BOOTSTRAP_COMMAND: &str =
    "python3 -I -S -c \"exec(__import__('base64').b64decode(__import__('sys').stdin.readline()))\"";

/// The helper program, embedded at build time.
pub(crate) const HELPER_PROGRAM: &str = include_str!("helper.py");

/// What a helper exchange needs from a channel.
///
/// A port rather than the SSH channel directly, so the protocol — write the program, write the
/// request, end the input, read a bounded answer — can be proved against a scripted double. The
/// alternative is proving it only against a real host, which means proving it nowhere that runs in
/// CI.
#[async_trait]
pub(crate) trait RemoteHelperChannel: Send + Sync {
    async fn write(&self, bytes: &[u8]) -> Result<(), RemoteHelperError>;
    async fn send_eof(&self) -> Result<(), RemoteHelperError>;
    /// `None` ends the stream. Anything that is not standard output is reported as such so the
    /// caller can discard it without having to know which stream number means what.
    async fn next_event(&self) -> Result<Option<RemoteHelperEvent>, RemoteHelperError>;
    async fn close(&self) -> Result<(), RemoteHelperError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RemoteHelperEvent {
    Stdout(Vec<u8>),
    /// Kept as a variant rather than dropped at the source so the transport can be explicit about
    /// discarding it; a channel that silently folded stderr into stdout would feed a remote host's
    /// diagnostics into a JSON parser.
    Stderr,
    Exited(u32),
    Ended,
}

/// Where a channel comes from.
#[async_trait]
pub(crate) trait RemoteHelperSession: Send + Sync {
    /// Opens one exec channel running the bootstrap command.
    ///
    /// The connection id and revision are re-checked by whoever implements this: a stale profile
    /// must fail here, before anything is written, rather than after a request has been sent to a
    /// host the user has since reconfigured.
    async fn open(
        &self,
        connection_id: &str,
        revision: i64,
    ) -> Result<Box<dyn RemoteHelperChannel>, RemoteHelperError>;
}

/// Sends one request and reads one response.
pub(crate) async fn exchange(
    session: &dyn RemoteHelperSession,
    connection_id: &str,
    revision: i64,
    request: &HelperRequest,
) -> Result<HelperResponse, RemoteHelperError> {
    exchange_cancellable(session, connection_id, revision, request, None).await
}

/// How often a waiting exchange asks whether it is still wanted.
///
/// The token is a polled flag rather than a future, because the thing that sets it is a Tauri
/// command and the things that read it are blocking walks. Polling here bounds how long a cancelled
/// exchange keeps a remote process alive; it does not need to be immediate, it needs to be finite.
/// Twenty-five milliseconds is far below anything a reader perceives and far above anything that
/// costs measurable CPU.
const CANCELLATION_POLL: Duration = Duration::from_millis(25);

/// One exchange that can be given up on.
///
/// The cancel does not travel to the remote host — there is no second channel to send it on, and
/// opening one would mean a connection per cancel. What ends the remote process is closing the
/// channel its stdin and stdout are on, which happens on this path exactly as it does on the timeout
/// path. Without this, a reader who pressed Escape waited out the full helper timeout while the
/// remote host kept searching for an answer nobody would read.
pub(crate) async fn exchange_cancellable(
    session: &dyn RemoteHelperSession,
    connection_id: &str,
    revision: i64,
    request: &HelperRequest,
    cancellation: Option<&SearchCancellationToken>,
) -> Result<HelperResponse, RemoteHelperError> {
    let body = serde_json::to_vec(request).map_err(|_| RemoteHelperError::MalformedResponse)?;
    if body.len() > MAX_HELPER_REQUEST_BYTES {
        // Refused here, not sent. The remote reads its whole stdin into memory, so an oversized
        // request is an unbounded allocation on a machine this process does not administer.
        return Err(RemoteHelperError::RequestTooLarge);
    }

    let channel = session.open(connection_id, revision).await?;
    let outcome = tokio::select! {
        biased;
        // First, so a token already signalled when the exchange starts wins over a round trip that
        // happens to be ready in the same poll.
        () = wait_for_cancellation(cancellation) => Err(RemoteHelperError::Cancelled),
        result = tokio::time::timeout(
            Duration::from_secs(HELPER_TIMEOUT_SECONDS),
            round_trip(channel.as_ref(), &body),
        ) => result.map_err(|_| RemoteHelperError::Timeout).and_then(|raw| raw),
    };
    // Closed on every path including the timeout and the cancel: a channel left open holds a remote
    // process and a pool slot for as long as the connection lives.
    let _ = channel.close().await;

    parse(&outcome?)
}

/// Resolves when the token is signalled, and never when there is no token.
async fn wait_for_cancellation(cancellation: Option<&SearchCancellationToken>) {
    let Some(token) = cancellation else {
        std::future::pending::<()>().await;
        return;
    };
    loop {
        if token.is_cancelled() {
            return;
        }
        tokio::time::sleep(CANCELLATION_POLL).await;
    }
}

async fn round_trip(
    channel: &dyn RemoteHelperChannel,
    request: &[u8],
) -> Result<Vec<u8>, RemoteHelperError> {
    let mut program = base64::engine::general_purpose::STANDARD.encode(HELPER_PROGRAM);
    program.push('\n');
    channel.write(program.as_bytes()).await?;
    channel.write(request).await?;
    // EOF rather than close: the helper reads its whole stdin, and closing would tear the channel
    // down before it could answer.
    channel.send_eof().await?;

    let mut output = Vec::new();
    loop {
        match channel.next_event().await? {
            Some(RemoteHelperEvent::Stdout(bytes)) => {
                if output.len() + bytes.len() > MAX_HELPER_RESPONSE_BYTES {
                    // Stop reading rather than truncate and parse: a truncated JSON document is not
                    // a smaller answer, it is a different one, and the parser would report it as
                    // malformed while the real fault was a bound.
                    return Err(RemoteHelperError::ResponseTooLarge);
                }
                output.extend_from_slice(&bytes);
            }
            // Discarded on purpose. See the module comment.
            Some(RemoteHelperEvent::Stderr) => {}
            Some(RemoteHelperEvent::Exited(_)) => {}
            Some(RemoteHelperEvent::Ended) | None => break,
        }
    }
    Ok(output)
}

fn parse(raw: &[u8]) -> Result<HelperResponse, RemoteHelperError> {
    let response: HelperResponse =
        serde_json::from_slice(raw).map_err(|_| RemoteHelperError::MalformedResponse)?;
    if response.version != HELPER_VERSION {
        // Refused rather than interpreted. The fields might happen to line up, and acting on a
        // payload whose meaning is not the meaning that was intended is what a version prevents.
        return Err(RemoteHelperError::VersionMismatch);
    }
    if !response.ok {
        return Err(RemoteHelperError::Refused(
            response
                .reason_code
                .unwrap_or_else(|| "remote_helper_refused".to_string()),
        ));
    }
    Ok(response)
}

/// A session that answers with prepared bodies instead of connecting.
///
/// Lives beside the transport rather than in a test file because two suites need it, and a second
/// copy would be a second idea of what the wire looks like — which is exactly the thing a shared
/// contract suite exists to prevent.
#[cfg(test)]
pub(crate) fn scripted_session(bodies: Vec<String>) -> ScriptedHelperSession {
    ScriptedHelperSession {
        bodies: std::sync::Mutex::new(bodies),
    }
}

/// A session whose channel never answers.
///
/// For the cases where what matters is what this side does *while* the remote is still thinking: a
/// cancel that lands mid-flight, and the channel close that ends the remote process. A scripted
/// answer would complete the round trip before any of that could be observed, and the test would
/// pass while proving nothing.
#[cfg(test)]
pub(crate) fn silent_session() -> SilentHelperSession {
    SilentHelperSession {
        closes: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    }
}

#[cfg(test)]
pub(crate) struct SilentHelperSession {
    closes: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
impl SilentHelperSession {
    /// How many channels were closed. A channel left open holds a remote process.
    pub(crate) fn closes(&self) -> usize {
        self.closes.load(std::sync::atomic::Ordering::Acquire)
    }
}

#[cfg(test)]
#[async_trait]
impl RemoteHelperSession for SilentHelperSession {
    async fn open(
        &self,
        _connection_id: &str,
        _revision: i64,
    ) -> Result<Box<dyn RemoteHelperChannel>, RemoteHelperError> {
        Ok(Box::new(SilentHelperChannel {
            closes: std::sync::Arc::clone(&self.closes),
        }))
    }
}

#[cfg(test)]
struct SilentHelperChannel {
    closes: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
#[async_trait]
impl RemoteHelperChannel for SilentHelperChannel {
    async fn write(&self, _bytes: &[u8]) -> Result<(), RemoteHelperError> {
        Ok(())
    }
    async fn send_eof(&self) -> Result<(), RemoteHelperError> {
        Ok(())
    }
    async fn next_event(&self) -> Result<Option<RemoteHelperEvent>, RemoteHelperError> {
        // Parks rather than returning `None`: returning would end the round trip and the exchange
        // would finish on its own.
        std::future::pending().await
    }
    async fn close(&self) -> Result<(), RemoteHelperError> {
        self.closes
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        Ok(())
    }
}

#[cfg(test)]
pub(crate) struct ScriptedHelperSession {
    bodies: std::sync::Mutex<Vec<String>>,
}

#[cfg(test)]
#[async_trait]
impl RemoteHelperSession for ScriptedHelperSession {
    async fn open(
        &self,
        _connection_id: &str,
        _revision: i64,
    ) -> Result<Box<dyn RemoteHelperChannel>, RemoteHelperError> {
        let mut bodies = self.bodies.lock().expect("bodies");
        // The last body repeats, so a case that retries or asks twice does not have to script the
        // same answer twice and accidentally assert the scripting rather than the behaviour.
        let body = if bodies.len() > 1 {
            bodies.remove(0)
        } else {
            bodies.first().cloned().unwrap_or_default()
        };
        Ok(Box::new(ScriptedHelperChannel {
            events: std::sync::Mutex::new(vec![
                RemoteHelperEvent::Stdout(body.into_bytes()),
                RemoteHelperEvent::Ended,
            ]),
        }))
    }
}

#[cfg(test)]
struct ScriptedHelperChannel {
    events: std::sync::Mutex<Vec<RemoteHelperEvent>>,
}

#[cfg(test)]
#[async_trait]
impl RemoteHelperChannel for ScriptedHelperChannel {
    async fn write(&self, _bytes: &[u8]) -> Result<(), RemoteHelperError> {
        Ok(())
    }
    async fn send_eof(&self) -> Result<(), RemoteHelperError> {
        Ok(())
    }
    async fn next_event(&self) -> Result<Option<RemoteHelperEvent>, RemoteHelperError> {
        let mut events = self.events.lock().expect("events");
        if events.is_empty() {
            return Ok(None);
        }
        Ok(Some(events.remove(0)))
    }
    async fn close(&self) -> Result<(), RemoteHelperError> {
        Ok(())
    }
}
