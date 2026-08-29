//! The remote startup and close paths, with the channel under the test's control.
//!
//! Every branch here needs a channel that fails in one particular way at one particular moment —
//! the connection refusing, the channel refusing on a healthy connection, the close hanging past
//! its budget, the reader never finishing. A real SSH server cannot be asked for any of that, which
//! is why this file did not exist until `RemoteShellTransport` did.
//!
//! Two of the doubles are shared rather than per-test: `SharedTransport` hands out channels from
//! one pool so a test can prove that closing one Shell leaves the other alone, which is the whole
//! point of one-channel-per-Shell and is invisible from a single-Shell test.

use super::retained_remote_shell::{RetainedRemoteShellRuntime, RoutedShellRuntime};
use crate::contexts::workspaces::application::{
    RemoteShellChannel, RemoteShellChannelError, RemoteShellEvent, RemoteShellOpenFailure,
    RemoteShellTransport, SessionShellRuntimePort, ShellOutputSink, ShellRemoteTarget,
    ShellRuntimeCloseOutcome, ShellRuntimeOpen,
};
use crate::contexts::workspaces::domain::{
    shell_reason_code, SessionShellError, SessionShellState, ShellCloseBudget, ShellGeneration,
    ShellId, ShellStream, TerminalDimensions,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How a fake channel behaves when the runtime closes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloseBehaviour {
    Succeeds,
    /// Returns an error — the remote refused.
    Fails,
    /// Never returns. Stands in for a wedged transport, which is the case an unbounded
    /// `block_on(close())` turns into an application that will not shut down.
    Hangs,
}

/// What the reader sees, and what closing the channel does.
struct FakeChannel {
    label: &'static str,
    events: Mutex<Vec<Result<Option<RemoteShellEvent>, RemoteShellChannelError>>>,
    close_behaviour: CloseBehaviour,
    closed: AtomicBool,
    writes: AtomicUsize,
    /// Signalled once the reader has drained the scripted events, so a test can wait for the
    /// worker to be parked in `next_event` rather than sleeping and hoping.
    drained: Arc<AtomicBool>,
}

impl FakeChannel {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            events: Mutex::new(Vec::new()),
            close_behaviour: CloseBehaviour::Succeeds,
            closed: AtomicBool::new(false),
            writes: AtomicUsize::new(0),
            drained: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Scripted in the order the reader will see them. The last one is normally an end-of-stream,
    /// because a channel that never ends parks the worker forever — which some tests want and most
    /// do not.
    fn emitting(
        label: &'static str,
        events: Vec<Result<Option<RemoteShellEvent>, RemoteShellChannelError>>,
    ) -> Self {
        let channel = Self::new(label);
        *channel.events.lock().unwrap() = events;
        channel
    }

    fn closing(mut self, behaviour: CloseBehaviour) -> Self {
        self.close_behaviour = behaviour;
        self
    }

    fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

#[async_trait(?Send)]
impl RemoteShellChannel for FakeChannel {
    async fn write(&self, _content: &[u8]) -> Result<(), RemoteShellChannelError> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn resize(&self, _columns: u16, _rows: u16) -> Result<(), RemoteShellChannelError> {
        Ok(())
    }

    async fn next_event(&self) -> Result<Option<RemoteShellEvent>, RemoteShellChannelError> {
        let next = {
            let mut events = self.events.lock().unwrap();
            if events.is_empty() {
                None
            } else {
                Some(events.remove(0))
            }
        };
        match next {
            Some(event) => event,
            None => {
                self.drained.store(true, Ordering::SeqCst);
                // Parked. A closed channel is what normally ends this in production; a test that
                // wants the reader to finish scripts an end-of-stream instead.
                loop {
                    if self.closed.load(Ordering::SeqCst) {
                        return Ok(None);
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    async fn close(&self) -> Result<(), RemoteShellChannelError> {
        match self.close_behaviour {
            CloseBehaviour::Succeeds => {
                self.closed.store(true, Ordering::SeqCst);
                Ok(())
            }
            CloseBehaviour::Fails => Err(RemoteShellChannelError),
            CloseBehaviour::Hangs => {
                // Longer than any budget a test uses, so the deadline is what ends the wait.
                tokio::time::sleep(Duration::from_secs(30)).await;
                Ok(())
            }
        }
    }
}

/// Hands out one prepared channel per connection id, or refuses.
struct FakeTransport {
    channels: Mutex<HashMap<String, Arc<FakeChannel>>>,
    failure: Option<RemoteShellOpenFailure>,
    opens: AtomicUsize,
}

impl FakeTransport {
    fn serving(channels: Vec<(&str, Arc<FakeChannel>)>) -> Self {
        Self {
            channels: Mutex::new(
                channels
                    .into_iter()
                    .map(|(id, channel)| (id.to_string(), channel))
                    .collect(),
            ),
            failure: None,
            opens: AtomicUsize::new(0),
        }
    }

    fn refusing(failure: RemoteShellOpenFailure) -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
            failure: Some(failure),
            opens: AtomicUsize::new(0),
        }
    }
}

#[async_trait(?Send)]
impl RemoteShellTransport for FakeTransport {
    async fn open_channel(
        &self,
        connection_id: &str,
        _profile_revision: i64,
        _columns: u16,
        _rows: u16,
    ) -> Result<Arc<dyn RemoteShellChannel>, RemoteShellOpenFailure> {
        self.opens.fetch_add(1, Ordering::SeqCst);
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        let channel = self
            .channels
            .lock()
            .unwrap()
            .get(connection_id)
            .cloned()
            .ok_or(RemoteShellOpenFailure::ChannelUnavailable)?;
        Ok(channel)
    }
}

#[derive(Default)]
struct RecordingSink {
    output: Mutex<Vec<(String, String)>>,
    states: Mutex<Vec<(String, SessionShellState)>>,
}

impl ShellOutputSink for RecordingSink {
    fn on_output(
        &self,
        shell_id: &ShellId,
        _generation: ShellGeneration,
        _stream: ShellStream,
        data: &[u8],
    ) {
        self.output.lock().unwrap().push((
            shell_id.as_str().to_string(),
            String::from_utf8_lossy(data).to_string(),
        ));
    }

    fn on_state(&self, shell_id: &ShellId, _generation: ShellGeneration, state: SessionShellState) {
        self.states
            .lock()
            .unwrap()
            .push((shell_id.as_str().to_string(), state));
    }
}

/// The generation every fixture opens under. Named rather than repeated so the stale-generation
/// test reads as "a different one" instead of as an arithmetic detail.
const FIRST: ShellGeneration = ShellGeneration::new(1);

fn shell_id(value: &str) -> ShellId {
    ShellId::parse(value).expect("shell id")
}

fn open_request(id: &str, connection_id: &str) -> ShellRuntimeOpen {
    ShellRuntimeOpen {
        shell_id: shell_id(id),
        generation: FIRST,
        session_id: "session-1".to_string(),
        root: "/workspace".to_string(),
        dimensions: TerminalDimensions::bounded(24, 80),
        remote: Some(ShellRemoteTarget {
            connection_id: connection_id.to_string(),
            profile_revision: 1,
            path: "/workspace".to_string(),
        }),
    }
}

/// Short enough that a hanging close is over in well under a second, long enough that a scheduler
/// hiccup does not decide the outcome.
fn test_budget() -> ShellCloseBudget {
    ShellCloseBudget {
        graceful: Duration::from_millis(20),
        terminate: Duration::from_millis(120),
        force: Duration::from_millis(20),
        worker: Duration::from_millis(400),
        total: Duration::from_millis(800),
        poll: Duration::from_millis(2),
    }
}

/// Waits until the reader has consumed its scripted events, so an assertion about what the sink
/// saw is not racing the worker that produces it.
fn await_drained(channel: &FakeChannel) {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !channel.drained.load(Ordering::SeqCst) {
        assert!(
            std::time::Instant::now() < deadline,
            "the reader never drained its scripted events"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

// --- Group 4: startup ------------------------------------------------------------------------

#[test]
fn a_refused_connection_and_a_refused_channel_are_different_answers() {
    for (failure, expected) in [
        (
            RemoteShellOpenFailure::ConnectionUnavailable,
            "shell_remote_connection_unavailable",
        ),
        (
            RemoteShellOpenFailure::ChannelUnavailable,
            "shell_remote_channel_unavailable",
        ),
    ] {
        let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::refusing(failure)));
        let error = runtime
            .open(
                &open_request("shell-1", "connection-1"),
                Arc::new(RecordingSink::default()),
            )
            .expect_err("the transport refused");

        // A reader acts differently on the two: the host or profile is wrong, versus this one
        // Shell could not get a channel on a connection that is working.
        match error {
            SessionShellError::RuntimeUnavailable { reason } => {
                assert_eq!(reason.as_str(), expected)
            }
            other => panic!("expected a runtime-unavailable reason, got {other:?}"),
        }
    }
}

#[test]
fn a_startup_that_fails_after_the_channel_opens_closes_that_channel() {
    // The worker cannot be made to fail from here, so this covers the guard's own contract: a
    // channel handed to the guard and never committed is closed when the guard drops.
    let channel = Arc::new(FakeChannel::new("only"));
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![(
        "connection-1",
        channel.clone(),
    )])));
    let sink = Arc::new(RecordingSink::default());
    runtime
        .open(&open_request("shell-1", "connection-1"), sink)
        .expect("open succeeds");

    // Committed, so it is emphatically not closed — the inverse of the rollback case, and the one
    // that would silently break if `commit` stopped disarming the guard.
    assert!(!channel.is_closed());
    assert_eq!(
        runtime.close(&shell_id("shell-1"), FIRST, test_budget()),
        ShellRuntimeCloseOutcome::Confirmed
    );
}

#[test]
fn a_shell_opened_for_one_generation_is_not_closed_by_another() {
    let channel = Arc::new(FakeChannel::new("only"));
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![(
        "connection-1",
        channel.clone(),
    )])));
    runtime
        .open(
            &open_request("shell-1", "connection-1"),
            Arc::new(RecordingSink::default()),
        )
        .expect("open succeeds");

    let stale = FIRST.next();
    assert_eq!(
        runtime.close(&shell_id("shell-1"), stale, test_budget()),
        ShellRuntimeCloseOutcome::NotHeld,
        "a stale generation must not be able to close the current Shell"
    );
    assert!(
        !channel.is_closed(),
        "the current generation's channel was closed by a stale request"
    );
}

#[test]
fn early_output_and_an_early_exit_both_reach_the_sink() {
    // The `echo-and-exit` shape: everything the remote sends arrives before anyone could have
    // asked about the Shell, and none of it may be lost to the registration race.
    let channel = Arc::new(FakeChannel::emitting(
        "fast",
        vec![
            Ok(Some(RemoteShellEvent::Output(b"hello\n".to_vec()))),
            Ok(Some(RemoteShellEvent::Exited { code: Some(0) })),
        ],
    ));
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![(
        "connection-1",
        channel.clone(),
    )])));
    let sink = Arc::new(RecordingSink::default());

    runtime
        .open(&open_request("shell-1", "connection-1"), sink.clone())
        .expect("open succeeds");

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while sink.states.lock().unwrap().is_empty() {
        assert!(
            std::time::Instant::now() < deadline,
            "no terminal state arrived"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(
        *sink.output.lock().unwrap(),
        vec![("shell-1".to_string(), "hello\n".to_string())]
    );
    assert_eq!(
        *sink.states.lock().unwrap(),
        vec![(
            "shell-1".to_string(),
            SessionShellState::Exited { code: Some(0) }
        )]
    );
}

#[test]
fn a_transport_failure_is_reported_as_a_disconnect_rather_than_a_silence() {
    let channel = Arc::new(FakeChannel::emitting(
        "lost",
        vec![Err(RemoteShellChannelError)],
    ));
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![(
        "connection-1",
        channel.clone(),
    )])));
    let sink = Arc::new(RecordingSink::default());

    runtime
        .open(&open_request("shell-1", "connection-1"), sink.clone())
        .expect("open succeeds");

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while sink.states.lock().unwrap().is_empty() {
        assert!(std::time::Instant::now() < deadline, "no state arrived");
        std::thread::sleep(Duration::from_millis(2));
    }
    // Disconnected, not Exited: the UI keeps the replay it holds and says why nothing more is
    // arriving, instead of showing a live terminal that stopped answering.
    let states = sink.states.lock().unwrap();
    assert!(
        matches!(states[0].1, SessionShellState::Disconnected { .. }),
        "a transport failure was reported as {:?}",
        states[0].1
    );
}

#[test]
fn an_end_of_stream_without_a_code_never_reports_a_clean_exit() {
    let channel = Arc::new(FakeChannel::emitting("ended", vec![Ok(None)]));
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![(
        "connection-1",
        channel.clone(),
    )])));
    let sink = Arc::new(RecordingSink::default());

    runtime
        .open(&open_request("shell-1", "connection-1"), sink.clone())
        .expect("open succeeds");

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while sink.states.lock().unwrap().is_empty() {
        assert!(std::time::Instant::now() < deadline, "no state arrived");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(
        sink.states.lock().unwrap()[0].1,
        SessionShellState::Exited { code: None },
        "an ending with no reported code became a zero, which reads as success"
    );
}

// --- Group 6: bounded close and shared transport ---------------------------------------------

#[test]
fn a_close_that_hangs_returns_by_its_deadline_and_keeps_the_shell() {
    let channel = Arc::new(FakeChannel::new("wedged").closing(CloseBehaviour::Hangs));
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![(
        "connection-1",
        channel.clone(),
    )])));
    runtime
        .open(
            &open_request("shell-1", "connection-1"),
            Arc::new(RecordingSink::default()),
        )
        .expect("open succeeds");
    await_drained(&channel);

    let started = std::time::Instant::now();
    let outcome = runtime.close(&shell_id("shell-1"), FIRST, test_budget());

    match outcome {
        ShellRuntimeCloseOutcome::Retained { reason, retryable } => {
            assert_eq!(reason.as_str(), shell_reason_code::CLOSE_DEADLINE_REACHED);
            assert!(
                retryable,
                "a wedged transport may recover; the Shell is retryable"
            );
        }
        other => panic!("a hanging close returned {other:?} instead of a bounded outcome"),
    }
    // The bound is what this test is about. An unbounded `block_on` here is the difference between
    // a terminal that will not close and an application that will not shut down.
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the close ran past its budget"
    );
}

#[test]
fn a_refused_close_keeps_the_shell_and_can_be_retried_with_the_same_id() {
    let channel = Arc::new(FakeChannel::new("refuses").closing(CloseBehaviour::Fails));
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![(
        "connection-1",
        channel.clone(),
    )])));
    runtime
        .open(
            &open_request("shell-1", "connection-1"),
            Arc::new(RecordingSink::default()),
        )
        .expect("open succeeds");
    await_drained(&channel);

    let outcome = runtime.close(&shell_id("shell-1"), FIRST, test_budget());

    match outcome {
        ShellRuntimeCloseOutcome::Retained { reason, retryable } => {
            assert_eq!(reason.as_str(), shell_reason_code::TERMINATE_FAILED);
            assert!(retryable);
        }
        other => panic!("a refused close returned {other:?}"),
    }
    // Retained means still held: removing the entry and then failing is what makes a retry have
    // nothing to retry, and sends the routed runtime to the local adapter for a Shell that never
    // was local.
    assert_eq!(
        runtime.close(&shell_id("shell-1"), FIRST, test_budget()),
        ShellRuntimeCloseOutcome::Retained {
            reason: crate::contexts::workspaces::domain::shell_reason(
                shell_reason_code::TERMINATE_FAILED
            ),
            retryable: true,
        },
        "the retry found nothing to retry"
    );
}

#[test]
fn closing_one_shell_leaves_the_other_channel_on_the_same_transport_alone() {
    // The point of one channel per Shell, and invisible from any single-Shell test: both Shells
    // come from one transport, and only the closed one may end.
    let first = Arc::new(FakeChannel::new("first"));
    let second = Arc::new(FakeChannel::new("second"));
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![
        ("connection-1", first.clone()),
        ("connection-2", second.clone()),
    ])));
    for (shell, connection) in [("shell-1", "connection-1"), ("shell-2", "connection-2")] {
        runtime
            .open(
                &open_request(shell, connection),
                Arc::new(RecordingSink::default()),
            )
            .expect("open succeeds");
    }
    await_drained(&first);
    await_drained(&second);

    assert_eq!(
        runtime.close(&shell_id("shell-1"), FIRST, test_budget()),
        ShellRuntimeCloseOutcome::Confirmed
    );

    assert!(first.is_closed());
    assert!(
        !second.is_closed(),
        "closing one Shell closed another Shell's channel on the same transport"
    );
    // And the survivor is still usable, not merely un-closed.
    assert!(runtime.write(&shell_id("shell-2"), "echo\n").is_ok());
    assert_eq!(second.writes.load(Ordering::SeqCst), 1);
}

#[test]
fn one_failing_close_does_not_prevent_the_other_shell_from_closing() {
    let stuck = Arc::new(FakeChannel::new("stuck").closing(CloseBehaviour::Fails));
    let healthy = Arc::new(FakeChannel::new("healthy"));
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![
        ("connection-1", stuck.clone()),
        ("connection-2", healthy.clone()),
    ])));
    for (shell, connection) in [("shell-1", "connection-1"), ("shell-2", "connection-2")] {
        runtime
            .open(
                &open_request(shell, connection),
                Arc::new(RecordingSink::default()),
            )
            .expect("open succeeds");
    }
    await_drained(&stuck);
    await_drained(&healthy);

    assert!(matches!(
        runtime.close(&shell_id("shell-1"), FIRST, test_budget()),
        ShellRuntimeCloseOutcome::Retained { .. }
    ));
    // Independent outcomes: one Shell's failure is not the transport's failure, so the other
    // still closes normally.
    assert_eq!(
        runtime.close(&shell_id("shell-2"), FIRST, test_budget()),
        ShellRuntimeCloseOutcome::Confirmed
    );
    assert!(healthy.is_closed());
}

#[test]
fn closing_a_shell_the_runtime_never_held_is_not_held_rather_than_confirmed() {
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![])));

    // Not `Confirmed`: the routed runtime uses this answer to decide whether the other adapter
    // owns the Shell, and a false confirmation there would strand a live local terminal.
    assert_eq!(
        runtime.close(&shell_id("never-opened"), FIRST, test_budget()),
        ShellRuntimeCloseOutcome::NotHeld
    );
}

#[test]
fn a_closed_shell_no_longer_accepts_input() {
    let channel = Arc::new(FakeChannel::new("only"));
    let runtime = RetainedRemoteShellRuntime::new(Arc::new(FakeTransport::serving(vec![(
        "connection-1",
        channel.clone(),
    )])));
    runtime
        .open(
            &open_request("shell-1", "connection-1"),
            Arc::new(RecordingSink::default()),
        )
        .expect("open succeeds");
    await_drained(&channel);
    runtime.close(&shell_id("shell-1"), FIRST, test_budget());

    assert!(matches!(
        runtime.write(&shell_id("shell-1"), "echo\n"),
        Err(SessionShellError::NotFound)
    ));
}

/// A runtime that asks the router where a Shell goes *while* it is opening it.
///
/// This is the window the pre-registration exists for: a real remote worker starts publishing the
/// moment the channel is up, which is before `open` has returned. Recording the route afterwards
/// left everything addressed by shell id falling through to the *local* runtime in that window —
/// for a remote Shell that is "not found" for a terminal the user is looking at, and a close that
/// reports success for a channel that is still open.
struct RoutingProbe {
    router: Mutex<Option<Arc<RoutedShellRuntime>>>,
    routed_during_open: Mutex<Option<bool>>,
    fail: bool,
}

impl RoutingProbe {
    fn new(fail: bool) -> Arc<Self> {
        Arc::new(Self {
            router: Mutex::new(None),
            routed_during_open: Mutex::new(None),
            fail,
        })
    }
}

impl SessionShellRuntimePort for RoutingProbe {
    fn open(
        &self,
        request: &ShellRuntimeOpen,
        _sink: Arc<dyn ShellOutputSink>,
    ) -> Result<crate::contexts::workspaces::application::ShellRuntimeOpened, SessionShellError>
    {
        // Asked from inside the open, which is exactly what a publishing worker does.
        let router = self.router.lock().unwrap().clone();
        let reached_here = router
            .map(|router| router.write(&request.shell_id, "").is_ok())
            .unwrap_or(false);
        *self.routed_during_open.lock().unwrap() = Some(reached_here);
        if self.fail {
            return Err(SessionShellError::RuntimeUnavailable {
                reason: crate::contexts::workspaces::domain::shell_reason(
                    shell_reason_code::OPEN_SETUP_FAILED,
                ),
            });
        }
        Ok(
            crate::contexts::workspaces::application::ShellRuntimeOpened {
                state: SessionShellState::Running,
                runtime: crate::contexts::workspaces::domain::ShellRuntimeDescriptor::Native,
            },
        )
    }

    fn write(&self, _shell_id: &ShellId, _content: &str) -> Result<(), SessionShellError> {
        Ok(())
    }

    fn resize(
        &self,
        _shell_id: &ShellId,
        _dimensions: TerminalDimensions,
    ) -> Result<(), SessionShellError> {
        Ok(())
    }

    fn close(
        &self,
        _shell_id: &ShellId,
        _generation: ShellGeneration,
        _budget: ShellCloseBudget,
    ) -> ShellRuntimeCloseOutcome {
        ShellRuntimeCloseOutcome::NotHeld
    }

    fn foreground_process(
        &self,
        _shell_id: &ShellId,
    ) -> crate::contexts::workspaces::domain::ShellForegroundProcessState {
        crate::contexts::workspaces::domain::ShellForegroundProcessState::Unknown
    }
}

/// A local runtime that refuses everything, so a misrouted call is loud rather than plausible.
struct RefusingLocal;

impl SessionShellRuntimePort for RefusingLocal {
    fn open(
        &self,
        _request: &ShellRuntimeOpen,
        _sink: Arc<dyn ShellOutputSink>,
    ) -> Result<crate::contexts::workspaces::application::ShellRuntimeOpened, SessionShellError>
    {
        Err(SessionShellError::NotFound)
    }

    fn write(&self, _shell_id: &ShellId, _content: &str) -> Result<(), SessionShellError> {
        Err(SessionShellError::NotFound)
    }

    fn resize(
        &self,
        _shell_id: &ShellId,
        _dimensions: TerminalDimensions,
    ) -> Result<(), SessionShellError> {
        Err(SessionShellError::NotFound)
    }

    fn close(
        &self,
        _shell_id: &ShellId,
        _generation: ShellGeneration,
        _budget: ShellCloseBudget,
    ) -> ShellRuntimeCloseOutcome {
        ShellRuntimeCloseOutcome::NotHeld
    }

    fn foreground_process(
        &self,
        _shell_id: &ShellId,
    ) -> crate::contexts::workspaces::domain::ShellForegroundProcessState {
        crate::contexts::workspaces::domain::ShellForegroundProcessState::Unknown
    }
}

#[test]
fn a_remote_route_exists_before_the_runtime_can_publish() {
    let probe = RoutingProbe::new(false);
    let router = Arc::new(RoutedShellRuntime::new(
        Arc::new(RefusingLocal),
        probe.clone(),
    ));
    *probe.router.lock().unwrap() = Some(router.clone());

    router
        .open(
            &open_request("shell-1", "connection-1"),
            Arc::new(RecordingSink::default()),
        )
        .expect("open succeeds");

    assert_eq!(
        *probe.routed_during_open.lock().unwrap(),
        Some(true),
        "a call made during the open reached the remote runtime rather than the local one"
    );
}

#[test]
fn a_failed_open_leaves_no_route_behind() {
    let probe = RoutingProbe::new(true);
    let router = Arc::new(RoutedShellRuntime::new(
        Arc::new(RefusingLocal),
        probe.clone(),
    ));
    *probe.router.lock().unwrap() = Some(router.clone());

    router
        .open(
            &open_request("shell-1", "connection-1"),
            Arc::new(RecordingSink::default()),
        )
        .expect_err("open fails");

    // Released, because a route to a Shell that does not exist would send the next call for that id
    // to a runtime holding nothing.
    assert!(matches!(
        router.write(
            &shell_id("shell-1"),
            "echo
"
        ),
        Err(SessionShellError::NotFound)
    ));
}

#[test]
fn a_newer_generation_keeps_the_id_against_a_late_open() {
    let probe = RoutingProbe::new(false);
    let router = Arc::new(RoutedShellRuntime::new(
        Arc::new(RefusingLocal),
        probe.clone(),
    ));
    let mut newer = open_request("shell-1", "connection-1");
    newer.generation = FIRST.next();
    router
        .open(&newer, Arc::new(RecordingSink::default()))
        .expect("newer open succeeds");

    // The older generation's open arrives afterwards. Accepting it would leave a runtime handle
    // nothing routes to, and would send the newer Shell's writes to whichever runtime won last.
    let late = router.open(
        &open_request("shell-1", "connection-1"),
        Arc::new(RecordingSink::default()),
    );

    assert!(matches!(late, Err(SessionShellError::Runtime { .. })));
    assert!(router
        .write(
            &shell_id("shell-1"),
            "echo
"
        )
        .is_ok());
}
