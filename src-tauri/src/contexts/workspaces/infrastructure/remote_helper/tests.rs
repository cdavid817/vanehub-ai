//! The protocol, against a scripted channel.
//!
//! No SSH anywhere. Everything worth asserting here is a decision this side makes — what is sent,
//! in what order, what is refused before anything leaves, and what is done with an answer — and a
//! real host would make each of those observable only when it happened to be configured to trigger
//! it.
//!
//! The one thing not tested here is whether the Python runs. That needs a host, and 11.14's
//! opt-in integration test is where it belongs; a fake that pretended to be Python would prove the
//! fake.

use super::probe::{capabilities_from, probe_capabilities, revalidate};
use super::protocol::{
    HelperOperation, HelperProbe, HelperRequest, RemoteHelperError, MAX_HELPER_REQUEST_BYTES,
    MAX_HELPER_RESPONSE_BYTES,
};
use super::transport::{
    close_failure_context, exchange, exchange_cancellable, exchange_within, RemoteHelperChannel,
    RemoteHelperEvent, RemoteHelperSession, HELPER_BOOTSTRAP_COMMAND, HELPER_PROGRAM,
};
use crate::contexts::workspaces::application::{
    RemoteWorkspaceTarget, SearchCancellationCause, SearchCancellationToken,
};
use async_trait::async_trait;
use std::sync::Mutex;

/// Records what was written and replays a scripted answer.
#[derive(Default)]
struct ScriptedChannel {
    writes: Mutex<Vec<Vec<u8>>>,
    events: Mutex<Vec<RemoteHelperEvent>>,
    eof_sent: Mutex<bool>,
    closed: Mutex<bool>,
}

impl ScriptedChannel {
    fn answering(body: &str) -> Self {
        Self {
            events: Mutex::new(vec![
                RemoteHelperEvent::Stdout(body.as_bytes().to_vec()),
                RemoteHelperEvent::Exited(0),
                RemoteHelperEvent::Ended,
            ]),
            ..Self::default()
        }
    }
}

#[async_trait]
impl RemoteHelperChannel for ScriptedChannel {
    async fn write(&self, bytes: &[u8]) -> Result<(), RemoteHelperError> {
        self.writes.lock().expect("writes").push(bytes.to_vec());
        Ok(())
    }

    async fn send_eof(&self) -> Result<(), RemoteHelperError> {
        *self.eof_sent.lock().expect("eof") = true;
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
        *self.closed.lock().expect("closed") = true;
        Ok(())
    }
}

struct ScriptedSession {
    channel: std::sync::Arc<ScriptedChannel>,
    opens: Mutex<Vec<(String, i64)>>,
}

impl ScriptedSession {
    fn new(channel: std::sync::Arc<ScriptedChannel>) -> Self {
        Self {
            channel,
            opens: Mutex::new(Vec::new()),
        }
    }
}

/// Shares the one channel with the test, so writes are observable after the exchange.
struct SharedChannel(std::sync::Arc<ScriptedChannel>);

#[async_trait]
impl RemoteHelperChannel for SharedChannel {
    async fn write(&self, bytes: &[u8]) -> Result<(), RemoteHelperError> {
        self.0.write(bytes).await
    }
    async fn send_eof(&self) -> Result<(), RemoteHelperError> {
        self.0.send_eof().await
    }
    async fn next_event(&self) -> Result<Option<RemoteHelperEvent>, RemoteHelperError> {
        self.0.next_event().await
    }
    async fn close(&self) -> Result<(), RemoteHelperError> {
        self.0.close().await
    }
}

#[async_trait]
impl RemoteHelperSession for ScriptedSession {
    async fn open(
        &self,
        connection_id: &str,
        revision: i64,
    ) -> Result<Box<dyn RemoteHelperChannel>, RemoteHelperError> {
        self.opens
            .lock()
            .expect("opens")
            .push((connection_id.to_string(), revision));
        Ok(Box::new(SharedChannel(self.channel.clone())))
    }
}

fn target() -> RemoteWorkspaceTarget {
    RemoteWorkspaceTarget {
        session_id: "session-1".to_string(),
        connection_id: "connection-1".to_string(),
        connection_revision: 7,
        root: "/work/app".to_string(),
        display_name: "Remote app".to_string(),
    }
}

fn probe_response() -> String {
    r#"{"version":1,"ok":true,"result":{"probe":{"helperVersion":1,"posix":true,
       "pythonVersion":"3.11.2","git":true,"ripgrep":true,"rootReadable":true}}}"#
        .to_string()
}

fn block<T>(future: impl std::future::Future<Output = T>) -> T {
    tauri::async_runtime::block_on(future)
}

/// A channel that never answers, so the exchange is still waiting when the test decides something.
///
/// `next_event` parks forever rather than returning `None`: returning would end the round trip and
/// the exchange would finish on its own, which is the one thing a cancellation test must not allow —
/// it would pass while proving nothing.
#[derive(Default)]
struct SilentChannel {
    closed: Mutex<bool>,
}

struct SilentSession(std::sync::Arc<SilentChannel>);

#[async_trait]
impl RemoteHelperSession for SilentSession {
    async fn open(
        &self,
        _connection_id: &str,
        _revision: i64,
    ) -> Result<Box<dyn RemoteHelperChannel>, RemoteHelperError> {
        Ok(Box::new(SharedSilentChannel(self.0.clone())))
    }
}

struct SharedSilentChannel(std::sync::Arc<SilentChannel>);

#[async_trait]
impl RemoteHelperChannel for SharedSilentChannel {
    async fn write(&self, _bytes: &[u8]) -> Result<(), RemoteHelperError> {
        Ok(())
    }
    async fn send_eof(&self) -> Result<(), RemoteHelperError> {
        Ok(())
    }
    async fn next_event(&self) -> Result<Option<RemoteHelperEvent>, RemoteHelperError> {
        std::future::pending().await
    }
    async fn close(&self) -> Result<(), RemoteHelperError> {
        *self.0.closed.lock().expect("closed") = true;
        Ok(())
    }
}

/// The defect this path exists for.
///
/// Without it a reader who pressed Escape waited out the full helper timeout — twenty seconds by
/// default — while the remote host kept searching for an answer nobody would read. The cancel does
/// not travel to the remote; closing the channel its stdin and stdout are on is what ends the
/// process there, and that is asserted rather than assumed.
#[tokio::test]
async fn a_cancelled_exchange_stops_waiting_and_closes_the_channel() {
    let channel = std::sync::Arc::new(SilentChannel::default());
    let session = SilentSession(channel.clone());
    let token = SearchCancellationToken::new();
    // Signalled before the exchange starts, so the test does not race the poll interval. What the
    // interval bounds is how long a cancel takes to land, not whether it lands at all.
    token.signal(SearchCancellationCause::Cancelled);

    let outcome = exchange_cancellable(
        &session,
        "connection-1",
        1,
        &HelperRequest::new("/work".to_string(), HelperOperation::Probe),
        Some(&token),
    )
    .await;

    assert_eq!(outcome.err(), Some(RemoteHelperError::Cancelled));
    assert!(
        *channel.closed.lock().expect("closed"),
        "a channel left open holds a remote process for as long as the connection lives"
    );
}

/// An exchange nobody cancelled must not be cut short by the mechanism that cancels one.
#[tokio::test]
async fn an_exchange_without_a_token_answers_normally() {
    let channel = std::sync::Arc::new(ScriptedChannel::answering(
        r#"{"version":1,"ok":true,"result":{}}"#,
    ));
    let session = ScriptedSession::new(channel);

    let response = exchange_cancellable(
        &session,
        "connection-1",
        1,
        &HelperRequest::new("/work".to_string(), HelperOperation::Probe),
        None,
    )
    .await
    .expect("answer");

    assert!(response.ok);
}

// ---------------------------------------------------------------------------------------------
// The command
// ---------------------------------------------------------------------------------------------

/// The command carries nothing from anybody.
///
/// Asserted as a property of the string rather than of one call, because the failure it prevents is
/// a future field being interpolated "just this once" — and a test that only checked one request
/// would keep passing.
#[test]
fn the_bootstrap_command_is_constant_and_free_of_shell_expansion() {
    for dangerous in ['$', '`', '\\', ';', '&', '|', '\n'] {
        assert!(
            !HELPER_BOOTSTRAP_COMMAND.contains(dangerous),
            "{dangerous:?} would be interpreted by the remote login shell"
        );
    }
    // Isolated and without `site`, so an unusual Python installation behaves like a plain one.
    assert!(HELPER_BOOTSTRAP_COMMAND.starts_with("python3 -I -S -c "));
    // No format placeholder, which is what a future interpolation would need.
    assert!(!HELPER_BOOTSTRAP_COMMAND.contains('{'));
}

#[test]
fn the_program_travels_as_data_before_the_request() {
    let channel = std::sync::Arc::new(ScriptedChannel::answering(&probe_response()));
    let session = ScriptedSession::new(channel.clone());

    block(exchange(
        &session,
        "connection-1",
        7,
        &HelperRequest::new("/work/app".to_string(), HelperOperation::Probe),
    ))
    .expect("response");

    let writes = channel.writes.lock().expect("writes");
    assert_eq!(writes.len(), 2, "expected the program then the request");
    // Base64 and a newline: the bootstrap reads exactly one line before executing.
    let program = String::from_utf8(writes[0].clone()).expect("utf-8 program");
    assert!(program.ends_with('\n'));
    assert!(program
        .trim_end()
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "+/=".contains(character)));
    // The request is JSON, and the root is in it rather than in the command.
    let request = String::from_utf8(writes[1].clone()).expect("utf-8 request");
    assert!(request.contains("\"root\":\"/work/app\""));
    assert!(!HELPER_BOOTSTRAP_COMMAND.contains("/work/app"));
}

#[test]
fn the_input_stream_is_ended_rather_than_the_channel_closed_first() {
    let channel = std::sync::Arc::new(ScriptedChannel::answering(&probe_response()));
    let session = ScriptedSession::new(channel.clone());

    block(exchange(
        &session,
        "connection-1",
        7,
        &HelperRequest::new("/work/app".to_string(), HelperOperation::Probe),
    ))
    .expect("response");

    // The helper reads its whole stdin; closing instead of ending the stream would tear the channel
    // down before it could answer.
    assert!(*channel.eof_sent.lock().expect("eof"));
    // And it is closed afterwards: a channel left open holds a remote process and a pool slot.
    assert!(*channel.closed.lock().expect("closed"));
}

#[test]
fn the_open_carries_the_revision_the_target_was_bound_to() {
    let channel = std::sync::Arc::new(ScriptedChannel::answering(&probe_response()));
    let session = ScriptedSession::new(channel);

    block(probe_capabilities(&session, &target())).expect("capabilities");

    assert_eq!(
        *session.opens.lock().expect("opens"),
        vec![("connection-1".to_string(), 7)]
    );
}

// ---------------------------------------------------------------------------------------------
// Bounds
// ---------------------------------------------------------------------------------------------

#[test]
fn an_oversized_request_never_reaches_the_channel() {
    let channel = std::sync::Arc::new(ScriptedChannel::answering(&probe_response()));
    let session = ScriptedSession::new(channel.clone());
    let root = "/".repeat(MAX_HELPER_REQUEST_BYTES + 1);

    let error = block(exchange(
        &session,
        "connection-1",
        7,
        &HelperRequest::new(root, HelperOperation::Probe),
    ))
    .expect_err("refusal");

    assert_eq!(error, RemoteHelperError::RequestTooLarge);
    // The remote reads its whole stdin into memory: an oversized request is an unbounded
    // allocation on a machine this process does not administer.
    assert!(session.opens.lock().expect("opens").is_empty());
    assert!(channel.writes.lock().expect("writes").is_empty());
}

#[test]
fn an_oversized_response_stops_rather_than_being_truncated_and_parsed() {
    let channel = std::sync::Arc::new(ScriptedChannel {
        events: Mutex::new(vec![
            RemoteHelperEvent::Stdout(vec![b'x'; MAX_HELPER_RESPONSE_BYTES]),
            RemoteHelperEvent::Stdout(vec![b'x'; 1024]),
            RemoteHelperEvent::Ended,
        ]),
        ..ScriptedChannel::default()
    });
    let session = ScriptedSession::new(channel);

    let error = block(exchange(
        &session,
        "connection-1",
        7,
        &HelperRequest::new("/work/app".to_string(), HelperOperation::Probe),
    ))
    .expect_err("refusal");

    // A truncated JSON document is not a smaller answer, it is a different one — and reporting it
    // as malformed would name the wrong fault.
    assert_eq!(error, RemoteHelperError::ResponseTooLarge);
}

#[test]
fn stderr_is_discarded_rather_than_parsed() {
    let channel = std::sync::Arc::new(ScriptedChannel {
        events: Mutex::new(vec![
            RemoteHelperEvent::Stderr,
            RemoteHelperEvent::Stdout(probe_response().into_bytes()),
            RemoteHelperEvent::Stderr,
            RemoteHelperEvent::Ended,
        ]),
        ..ScriptedChannel::default()
    });
    let session = ScriptedSession::new(channel);

    // A channel that folded stderr into stdout would feed a remote host's diagnostics — routinely
    // containing paths and usernames — into a JSON parser, and then into whatever logs the failure.
    block(probe_capabilities(&session, &target())).expect("capabilities");
}

// ---------------------------------------------------------------------------------------------
// Answers
// ---------------------------------------------------------------------------------------------

#[test]
fn a_response_from_another_protocol_version_is_refused_rather_than_read() {
    let channel = std::sync::Arc::new(ScriptedChannel::answering(
        r#"{"version":99,"ok":true,"result":{"probe":{"helperVersion":99,"posix":true,
           "pythonVersion":"3.11.2","git":true,"ripgrep":true,"rootReadable":true}}}"#,
    ));
    let session = ScriptedSession::new(channel);

    let error = block(probe_capabilities(&session, &target())).expect_err("refusal");

    assert_eq!(error, RemoteHelperError::VersionMismatch);
}

#[test]
fn a_helper_refusal_keeps_its_own_code() {
    let channel = std::sync::Arc::new(ScriptedChannel::answering(
        r#"{"version":1,"ok":false,"reasonCode":"remote_helper_unsupported_operation"}"#,
    ));
    let session = ScriptedSession::new(channel);

    let error = block(probe_capabilities(&session, &target())).expect_err("refusal");

    // The helper's own vocabulary survives the trip: collapsing it here would lose the one piece of
    // information that says which prerequisite is missing.
    assert_eq!(error.code(), "remote_helper_unsupported_operation");
}

#[test]
fn output_that_is_not_json_is_malformed_rather_than_empty() {
    let channel = std::sync::Arc::new(ScriptedChannel::answering("bash: python3: not found"));
    let session = ScriptedSession::new(channel);

    let error = block(probe_capabilities(&session, &target())).expect_err("refusal");

    // The common real case, and the one an empty-result fallback would render as a workspace with
    // nothing in it.
    assert_eq!(error, RemoteHelperError::MalformedResponse);
}

// ---------------------------------------------------------------------------------------------
// Capability mapping
// ---------------------------------------------------------------------------------------------

fn found() -> HelperProbe {
    HelperProbe {
        helper_version: 1,
        posix: true,
        python_version: "3.11.2".to_string(),
        git: true,
        ripgrep: true,
        root_readable: true,
    }
}

#[test]
fn a_host_with_everything_offers_everything() {
    let capabilities = capabilities_from(&found());

    assert_eq!(capabilities.provider, "ssh");
    assert!(capabilities.list_files.available);
    assert!(capabilities.search_files.available);
    assert!(capabilities.git_status.available);
    // Nothing on the remote host tells this process when a file changed, and saying `native` would
    // leave a reader believing an external change would appear on its own.
    assert_eq!(capabilities.watch_mode.token(), "polling");
}

#[test]
fn a_missing_ripgrep_disables_search_and_nothing_else() {
    let capabilities = capabilities_from(&HelperProbe {
        ripgrep: false,
        ..found()
    });

    // A fallback that walked the tree in Python would be a second search with different bounds,
    // ordering, and ignore rules, reached exactly when nobody could tell which one answered.
    assert!(!capabilities.search_files.available);
    assert_eq!(
        capabilities.search_files.reason_code,
        Some("remote_ripgrep_missing")
    );
    assert_eq!(
        capabilities.search_files.remediation,
        Some("remote_install_ripgrep")
    );
    assert!(capabilities.list_files.available);
    assert!(capabilities.git_status.available);
}

#[test]
fn a_missing_git_disables_only_the_git_capabilities() {
    let capabilities = capabilities_from(&HelperProbe {
        git: false,
        ..found()
    });

    assert!(!capabilities.git_status.available);
    assert!(!capabilities.git_diff.available);
    assert_eq!(
        capabilities.git_status.remediation,
        Some("remote_install_git")
    );
    assert!(capabilities.list_files.available);
    assert!(capabilities.search_files.available);
}

#[test]
fn an_unreadable_root_disables_every_read_including_git() {
    let capabilities = capabilities_from(&HelperProbe {
        root_readable: false,
        ..found()
    });

    // Git under an unreadable root would answer about whatever directory the command happened to
    // start in, which is a real answer about the wrong repository.
    for state in [
        &capabilities.list_files,
        &capabilities.read_text_files,
        &capabilities.search_files,
        &capabilities.git_status,
        &capabilities.git_diff,
    ] {
        assert!(!state.available);
        assert_eq!(state.reason_code, Some("remote_root_unreadable"));
    }
}

#[test]
fn a_non_posix_host_offers_nothing() {
    let capabilities = capabilities_from(&HelperProbe {
        posix: false,
        ..found()
    });

    // The helper's path handling, its realpath confinement, and its argument-array subprocess calls
    // all assume POSIX. A partial answer would offer operations whose safety argument does not hold.
    assert!(!capabilities.list_files.available);
    assert_eq!(
        capabilities.list_files.reason_code,
        Some("remote_host_not_posix")
    );
    assert_eq!(capabilities.watch_mode.token(), "none");
}

// ---------------------------------------------------------------------------------------------
// Revalidation
// ---------------------------------------------------------------------------------------------

#[test]
fn a_binding_that_still_matches_a_trusted_host_passes() {
    assert_eq!(revalidate(7, 7, true), Ok(()));
}

#[test]
fn an_edited_profile_is_stale_rather_than_reconnected_under() {
    // Reconnecting under the new revision would answer about a different machine while the reader
    // believed they were still looking at the first.
    assert_eq!(revalidate(7, 8, true), Err(RemoteHelperError::ProfileStale));
}

#[test]
fn a_host_whose_trust_was_revoked_stops_before_the_network() {
    // Trust can be revoked between two reads, and a probe is the first thing a panel runs.
    assert_eq!(
        revalidate(7, 7, false),
        Err(RemoteHelperError::HostUntrusted)
    );
}

/// A stale revision outranks a trust failure.
#[test]
fn a_stale_revision_is_reported_even_when_the_host_is_also_untrusted() {
    // The profile this session was bound to is gone, so nothing about the profile that replaced
    // it - including whether its host is trusted - describes what the reader asked about.
    assert_eq!(
        revalidate(7, 8, false),
        Err(RemoteHelperError::ProfileStale)
    );
}
// ---------------------------------------------------------------------------------------------
// The program
// ---------------------------------------------------------------------------------------------

/// The helper is the same program on every host.
///
/// Embedded at build time rather than fetched or written to the remote filesystem: a helper that
/// installed itself would need a writable directory, a cleanup story, and a way to notice that the
/// copy on the host is a different version from the client talking to it.
#[test]
fn the_program_is_embedded_and_declares_the_protocol_version() {
    assert!(HELPER_PROGRAM.contains("HELPER_VERSION = 1"));

    // Only the standard library. Checked by reading the import lines rather than by scanning for
    // suspicious substrings: the substring version flagged the *word* `site-packages` the moment
    // the walk started skipping a directory by that name, which is a directory to avoid rather
    // than a dependency to have. Enumerating what is imported cannot make that mistake, and it
    // catches a third-party module the substring list never thought to name.
    // `time` is here for `time.monotonic`, which is what the walk's deadline is measured against. A
    // wall clock would be the wrong instrument on somebody else's machine: NTP moves it, and a
    // deadline that has not arrived would look like one that passed an hour ago.
    const STANDARD_LIBRARY: &[&str] = &[
        "base64",
        "json",
        "os",
        "shutil",
        "subprocess",
        "sys",
        "time",
    ];
    for line in HELPER_PROGRAM.lines() {
        let trimmed = line.trim();
        let Some(module) = trimmed.strip_prefix("import ") else {
            continue;
        };
        assert!(
            STANDARD_LIBRARY.contains(&module.trim()),
            "{module} is not in the standard library set this helper is allowed to use"
        );
    }
    // `from x import y` would slip past the check above, so it is refused outright: every module
    // the helper needs is available as a plain import.
    assert!(!HELPER_PROGRAM.contains("\nfrom "));
    assert!(!HELPER_PROGRAM.contains("pip "));
    // Nothing may reach the interpreter's own module search path. `-S` already disables site
    // packages, and rebuilding the path here would be the one way to get around it.
    assert!(!HELPER_PROGRAM.contains("sys.path"));
    // Subprocess calls are argument arrays. A string command would be a shell, and a shell is where
    // a remote path becomes an injection.
    assert!(!HELPER_PROGRAM.contains("shell=True"));
}

/// A channel that answers and then refuses to close.
///
/// The refusal is what makes the record worth having: the exchange succeeded, so nothing in the
/// answer says a remote process may still be running.
struct UnclosableChannel {
    inner: std::sync::Arc<ScriptedChannel>,
    close_attempts: Mutex<u32>,
}

impl UnclosableChannel {
    fn answering(body: &str) -> Self {
        Self {
            inner: std::sync::Arc::new(ScriptedChannel::answering(body)),
            close_attempts: Mutex::new(0),
        }
    }
}

#[async_trait]
impl RemoteHelperChannel for UnclosableChannel {
    async fn write(&self, bytes: &[u8]) -> Result<(), RemoteHelperError> {
        self.inner.write(bytes).await
    }
    async fn send_eof(&self) -> Result<(), RemoteHelperError> {
        self.inner.send_eof().await
    }
    async fn next_event(&self) -> Result<Option<RemoteHelperEvent>, RemoteHelperError> {
        self.inner.next_event().await
    }
    async fn close(&self) -> Result<(), RemoteHelperError> {
        *self.close_attempts.lock().expect("attempts") += 1;
        Err(RemoteHelperError::ChannelFailed)
    }
}

struct UnclosableSession(std::sync::Arc<UnclosableChannel>);

struct SharedUnclosable(std::sync::Arc<UnclosableChannel>);

#[async_trait]
impl RemoteHelperChannel for SharedUnclosable {
    async fn write(&self, bytes: &[u8]) -> Result<(), RemoteHelperError> {
        self.0.write(bytes).await
    }
    async fn send_eof(&self) -> Result<(), RemoteHelperError> {
        self.0.send_eof().await
    }
    async fn next_event(&self) -> Result<Option<RemoteHelperEvent>, RemoteHelperError> {
        self.0.next_event().await
    }
    async fn close(&self) -> Result<(), RemoteHelperError> {
        self.0.close().await
    }
}

#[async_trait]
impl RemoteHelperSession for UnclosableSession {
    async fn open(
        &self,
        _connection_id: &str,
        _revision: i64,
    ) -> Result<Box<dyn RemoteHelperChannel>, RemoteHelperError> {
        Ok(Box::new(SharedUnclosable(self.0.clone())))
    }
}

/// A close that failed is not allowed to become the caller's answer.
///
/// The reader asked a question and it was answered; a cleanup failure afterwards is an operator's
/// problem, not theirs. What must not happen is the third option — dropping it entirely, which is
/// what `let _ =` did, and which leaves a host accumulating helper processes with nothing anywhere
/// saying so.
#[tokio::test]
async fn a_close_that_fails_is_recorded_without_changing_the_answer() {
    let channel = std::sync::Arc::new(UnclosableChannel::answering(
        r#"{"version":1,"ok":true,"result":{}}"#,
    ));
    let session = UnclosableSession(channel.clone());

    let response = exchange_cancellable(
        &session,
        "connection-1",
        1,
        &HelperRequest::new("/work".to_string(), HelperOperation::Probe),
        None,
    )
    .await
    .expect("the answer survives a failed close");

    assert!(response.ok);
    assert_eq!(*channel.close_attempts.lock().expect("attempts"), 1);
}

/// The record names the connection and nothing else.
///
/// A bounded field set stated by name rather than by count, so a third field has to be added here
/// too — which is where a reviewer sees that somebody added the host.
#[test]
fn a_close_failure_record_carries_two_named_fields_and_no_others() {
    let context = close_failure_context("connection-1", 7);

    assert_eq!(
        context.keys().cloned().collect::<Vec<_>>(),
        vec![
            "connection_id".to_string(),
            "connection_revision".to_string()
        ]
    );
    assert_eq!(context["connection_id"], "connection-1");
    assert_eq!(context["connection_revision"], "7");
}

/// Output that arrives after the cancel is not the answer.
///
/// The channel here has a full, valid response waiting. A select that resolved whichever branch was
/// ready would return it, and the reader who pressed Escape would watch results appear.
#[tokio::test]
async fn output_ready_at_the_moment_of_a_cancel_is_discarded() {
    let channel = std::sync::Arc::new(ScriptedChannel::answering(
        r#"{"version":1,"ok":true,"result":{}}"#,
    ));
    let session = ScriptedSession::new(channel.clone());
    let token = SearchCancellationToken::new();
    token.signal(SearchCancellationCause::Cancelled);

    let outcome = exchange_cancellable(
        &session,
        "connection-1",
        1,
        &HelperRequest::new("/work".to_string(), HelperOperation::Probe),
        Some(&token),
    )
    .await;

    assert_eq!(outcome.err(), Some(RemoteHelperError::Cancelled));
}

/// A supersede is not reported as a cancel, on the remote side either.
///
/// Both stop the exchange and both return nothing. A reader is told different things: they cancelled
/// it, or they typed another character, and only the first is something they did to this search on
/// purpose. The transport reports one error for both because it does not know which; the cause lives
/// on the token, and the provider is what turns it into a reason code.
#[tokio::test]
async fn a_superseded_exchange_stops_the_same_way_a_cancelled_one_does() {
    let channel = std::sync::Arc::new(SilentChannel::default());
    let session = SilentSession(channel.clone());
    let token = SearchCancellationToken::new();
    token.signal(SearchCancellationCause::Superseded);

    let outcome = exchange_cancellable(
        &session,
        "connection-1",
        1,
        &HelperRequest::new("/work".to_string(), HelperOperation::Probe),
        Some(&token),
    )
    .await;

    assert_eq!(outcome.err(), Some(RemoteHelperError::Cancelled));
    assert_eq!(token.cause(), Some(SearchCancellationCause::Superseded));
    assert!(*channel.closed.lock().expect("closed"));
}

/// A host that accepts the connection and never answers is given up on.
///
/// The timeout is supplied rather than waited out: every individual read succeeds here and only the
/// total says anything is wrong, which is exactly the failure the other bounds miss. Zero rather
/// than a small duration, so it trips on every machine rather than on a fast one.
#[tokio::test]
async fn a_host_that_never_answers_times_out_and_closes_the_channel() {
    let channel = std::sync::Arc::new(SilentChannel::default());
    let session = SilentSession(channel.clone());

    let outcome = exchange_within(
        &session,
        "connection-1",
        1,
        &HelperRequest::new("/work".to_string(), HelperOperation::Probe),
        None,
        std::time::Duration::ZERO,
    )
    .await;

    assert_eq!(outcome.err(), Some(RemoteHelperError::Timeout));
    // The same cleanup the cancel path performs. A timeout that left the channel open would hold a
    // remote process for the life of the connection, which is the failure the timeout exists to end.
    assert!(*channel.closed.lock().expect("closed"));
}

/// A successful exchange closes its channel too.
///
/// The cancel and timeout paths have their own tests, and the success path is the one that runs
/// every time: a leak here would be a helper process per read rather than per abandoned read.
#[tokio::test]
async fn a_successful_exchange_closes_the_channel_it_opened() {
    let channel = std::sync::Arc::new(ScriptedChannel::answering(
        r#"{"version":1,"ok":true,"result":{}}"#,
    ));
    let session = ScriptedSession::new(channel.clone());

    exchange_cancellable(
        &session,
        "connection-1",
        1,
        &HelperRequest::new("/work".to_string(), HelperOperation::Probe),
        None,
    )
    .await
    .expect("answer");

    assert!(*channel.closed.lock().expect("closed"));
}
