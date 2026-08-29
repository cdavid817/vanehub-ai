use super::retained_remote_shell::RoutedShellRuntime;
use super::retained_shell_process::{
    ShellDeadlineClock, ShellProcessError, ShellProcessHandle, ShellPtyHandle, ShellWorker,
};
use super::retained_shell_runtime::{LocalPtyFactory, LocalPtySession, RetainedLocalShellRuntime};
use crate::contexts::workspaces::application::{
    SessionShellRuntimePort, ShellLifecycleDiagnosticsPort, ShellOutputSink, ShellRemoteTarget,
    ShellRuntimeCloseOutcome, ShellRuntimeOpen, ShellRuntimeOpened,
};
use crate::contexts::workspaces::domain::{
    shell_reason, SessionShellError, SessionShellState, ShellCloseBudget,
    ShellForegroundProcessState, ShellGeneration, ShellId, ShellRuntimeDescriptor, ShellStream,
    TerminalDimensions,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct RecordingRuntime {
    label: &'static str,
    calls: Mutex<Vec<String>>,
    fail_open: bool,
    /// When set, `close` reports that this runtime still owns the Shell.
    retain_close: bool,
}

impl RecordingRuntime {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            calls: Mutex::new(Vec::new()),
            fail_open: false,
            retain_close: false,
        }
    }

    fn failing(label: &'static str) -> Self {
        Self {
            label,
            calls: Mutex::new(Vec::new()),
            fail_open: true,
            retain_close: false,
        }
    }

    fn retaining(label: &'static str) -> Self {
        Self {
            label,
            calls: Mutex::new(Vec::new()),
            fail_open: false,
            retain_close: true,
        }
    }

    fn record(&self, call: &str) {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("{}:{call}", self.label));
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().expect("calls").clone()
    }
}

impl SessionShellRuntimePort for RecordingRuntime {
    fn open(
        &self,
        request: &ShellRuntimeOpen,
        _sink: Arc<dyn ShellOutputSink>,
    ) -> Result<ShellRuntimeOpened, SessionShellError> {
        if self.fail_open {
            return Err(SessionShellError::RuntimeUnavailable {
                reason: crate::contexts::workspaces::domain::shell_reason("unavailable"),
            });
        }
        self.record(&format!("open:{}", request.shell_id.as_str()));
        Ok(ShellRuntimeOpened {
            runtime: ShellRuntimeDescriptor::Native,
            state: SessionShellState::Running,
        })
    }

    fn write(&self, shell_id: &ShellId, _content: &str) -> Result<(), SessionShellError> {
        self.record(&format!("write:{}", shell_id.as_str()));
        Ok(())
    }

    fn resize(
        &self,
        shell_id: &ShellId,
        _dimensions: TerminalDimensions,
    ) -> Result<(), SessionShellError> {
        self.record(&format!("resize:{}", shell_id.as_str()));
        Ok(())
    }

    fn close(
        &self,
        shell_id: &ShellId,
        _generation: ShellGeneration,
        _budget: ShellCloseBudget,
    ) -> ShellRuntimeCloseOutcome {
        self.record(&format!("close:{}", shell_id.as_str()));
        if self.retain_close {
            return ShellRuntimeCloseOutcome::Retained {
                reason: crate::contexts::workspaces::domain::shell_reason(
                    crate::contexts::workspaces::domain::shell_reason_code::CLOSE_DEADLINE_REACHED,
                ),
                retryable: true,
            };
        }
        ShellRuntimeCloseOutcome::Confirmed
    }

    fn foreground_process(&self, _shell_id: &ShellId) -> ShellForegroundProcessState {
        ShellForegroundProcessState::Unknown
    }
}

#[derive(Default)]
struct SilentSink;

impl ShellOutputSink for SilentSink {
    fn on_output(
        &self,
        _shell_id: &ShellId,
        _generation: ShellGeneration,
        _stream: ShellStream,
        _bytes: &[u8],
    ) {
    }

    fn on_state(
        &self,
        _shell_id: &ShellId,
        _generation: ShellGeneration,
        _state: SessionShellState,
    ) {
    }
}

fn shell(id: &str) -> ShellId {
    ShellId::parse(id).expect("shell id")
}

fn open_request(id: &str, remote: bool) -> ShellRuntimeOpen {
    ShellRuntimeOpen {
        shell_id: shell(id),
        generation: ShellGeneration::new(1),
        session_id: "session-1".to_string(),
        root: "D:/project".to_string(),
        dimensions: TerminalDimensions::bounded(24, 80),
        remote: remote.then(|| ShellRemoteTarget {
            connection_id: "connection-1".to_string(),
            profile_revision: 3,
            path: "/srv/project".to_string(),
        }),
    }
}

/// Virtual time. Every deadline in the close sequence is asserted by advancing a number, so no test
/// here waits out a production budget or depends on how loaded the machine is.
#[derive(Default)]
struct VirtualClock {
    elapsed: Mutex<Duration>,
}

impl ShellDeadlineClock for VirtualClock {
    fn elapsed(&self) -> Duration {
        *self.elapsed.lock().expect("elapsed")
    }

    fn park(&self, duration: Duration) {
        let mut elapsed = self.elapsed.lock().expect("elapsed");
        *elapsed = elapsed.saturating_add(duration);
    }
}

/// A child that behaves exactly as told: already gone, gone after N observations, never gone, or
/// unable to answer at all.
struct FakeProcess {
    /// How many `try_reap` calls happen before the child reports itself gone. `None` never does.
    reaps_after: Option<u32>,
    observed: AtomicU32,
    terminations: AtomicU32,
    fail_terminate: bool,
    fail_observe: bool,
}

impl FakeProcess {
    fn exits_after(observations: u32) -> Self {
        Self {
            reaps_after: Some(observations),
            observed: AtomicU32::new(0),
            terminations: AtomicU32::new(0),
            fail_terminate: false,
            fail_observe: false,
        }
    }

    fn never_exits() -> Self {
        Self {
            reaps_after: None,
            observed: AtomicU32::new(0),
            terminations: AtomicU32::new(0),
            fail_terminate: false,
            fail_observe: false,
        }
    }

    fn unkillable() -> Self {
        Self {
            fail_terminate: true,
            ..Self::never_exits()
        }
    }

    fn unobservable() -> Self {
        Self {
            fail_observe: true,
            ..Self::never_exits()
        }
    }
}

impl ShellProcessHandle for FakeProcess {
    fn try_reap(&self) -> Result<Option<i32>, ShellProcessError> {
        if self.fail_observe {
            return Err(ShellProcessError::Observe);
        }
        let seen = self.observed.fetch_add(1, Ordering::SeqCst) + 1;
        match self.reaps_after {
            Some(threshold) if seen >= threshold => Ok(Some(0)),
            _ => Ok(None),
        }
    }

    fn terminate(&self) -> Result<(), ShellProcessError> {
        self.terminations.fetch_add(1, Ordering::SeqCst);
        if self.fail_terminate {
            return Err(ShellProcessError::Terminate);
        }
        Ok(())
    }
}

struct FakeMaster;

impl ShellPtyHandle for FakeMaster {
    fn resize(&self, _dimensions: TerminalDimensions) -> Result<(), ()> {
        Ok(())
    }
}

fn budget() -> ShellCloseBudget {
    ShellCloseBudget {
        graceful: Duration::from_millis(10),
        terminate: Duration::from_millis(10),
        force: Duration::from_millis(10),
        worker: Duration::from_millis(10),
        total: Duration::from_millis(50),
        poll: Duration::from_millis(5),
    }
}

fn install(
    runtime: &RetainedLocalShellRuntime,
    id: &str,
    process: Arc<dyn ShellProcessHandle>,
    workers: Vec<Arc<ShellWorker>>,
) -> ShellId {
    let shell_id = shell(id);
    runtime.install_for_tests(
        &shell_id,
        ShellGeneration::new(1),
        process,
        Arc::new(FakeMaster),
        workers,
    );
    shell_id
}

fn finished_worker() -> Arc<ShellWorker> {
    Arc::new(ShellWorker::detached(Arc::new(AtomicBool::new(true))))
}

fn stuck_worker() -> Arc<ShellWorker> {
    Arc::new(ShellWorker::detached(Arc::new(AtomicBool::new(false))))
}

#[test]
fn a_shell_stays_with_the_runtime_that_opened_it() {
    let local = Arc::new(RecordingRuntime::new("local"));
    let remote = Arc::new(RecordingRuntime::new("remote"));
    let routed = RoutedShellRuntime::new(local.clone(), remote.clone());

    routed
        .open(&open_request("shell-local", false), Arc::new(SilentSink))
        .expect("local open");
    routed
        .open(&open_request("shell-remote", true), Arc::new(SilentSink))
        .expect("remote open");
    routed.write(&shell("shell-local"), "ls\n").expect("write");
    routed.write(&shell("shell-remote"), "ls\n").expect("write");
    routed.close(
        &shell("shell-remote"),
        ShellGeneration::new(1),
        ShellCloseBudget::immediate(),
    );

    // A Shell that could change which runtime it belongs to midway would be two shells sharing an
    // id, so the route is decided once at open and read back for every later call.
    assert_eq!(
        local.calls(),
        vec!["local:open:shell-local", "local:write:shell-local"]
    );
    assert_eq!(
        remote.calls(),
        vec![
            "remote:open:shell-remote",
            "remote:write:shell-remote",
            "remote:close:shell-remote"
        ]
    );
}

#[test]
fn a_failed_open_records_no_route() {
    let local = Arc::new(RecordingRuntime::failing("local"));
    let remote = Arc::new(RecordingRuntime::new("remote"));
    let routed = RoutedShellRuntime::new(local.clone(), remote.clone());

    routed
        .open(&open_request("shell-1", false), Arc::new(SilentSink))
        .expect_err("open fails");
    // A route to a Shell that does not exist would send a later write into a runtime that never
    // opened it.
    routed.close(
        &shell("shell-1"),
        ShellGeneration::new(1),
        ShellCloseBudget::immediate(),
    );
    assert!(remote.calls().is_empty());
}

/// The defect this change exists to remove on the routing side. A remote close that could not
/// confirm used to delete the route on its way out, so the retry the user pressed next went to the
/// *local* runtime, found nothing, and reported success for a channel that was still open.
#[test]
fn an_unconfirmed_remote_close_keeps_its_route() {
    let local = Arc::new(RecordingRuntime::new("local"));
    let remote = Arc::new(RecordingRuntime::retaining("remote"));
    let routed = RoutedShellRuntime::new(local.clone(), remote.clone());
    routed
        .open(&open_request("shell-1", true), Arc::new(SilentSink))
        .expect("remote open");

    let first = routed.close(
        &shell("shell-1"),
        ShellGeneration::new(1),
        ShellCloseBudget::immediate(),
    );
    let second = routed.close(
        &shell("shell-1"),
        ShellGeneration::new(1),
        ShellCloseBudget::immediate(),
    );

    assert!(matches!(
        first,
        ShellRuntimeCloseOutcome::Retained {
            retryable: true,
            ..
        }
    ));
    assert!(matches!(
        second,
        ShellRuntimeCloseOutcome::Retained {
            retryable: true,
            ..
        }
    ));
    // Both attempts reached the remote runtime. The local runtime never saw the Shell at all.
    assert_eq!(
        remote.calls(),
        vec![
            "remote:open:shell-1",
            "remote:close:shell-1",
            "remote:close:shell-1"
        ]
    );
    assert_eq!(local.calls(), Vec::<String>::new());
}

/// Closing a Shell the runtime does not hold is `NotHeld`, not an error: a registry entry can
/// outlive its process, and failing on that would make cleanup unreliable exactly where it matters.
#[test]
fn closing_an_unknown_local_shell_reports_nothing_held() {
    let runtime = RetainedLocalShellRuntime::for_test();

    let outcome = runtime.close(
        &shell("shell-missing"),
        ShellGeneration::new(1),
        ShellCloseBudget::immediate(),
    );

    assert_eq!(outcome, ShellRuntimeCloseOutcome::NotHeld);
    assert!(outcome.is_released());
    assert_eq!(
        runtime.foreground_process(&shell("shell-missing")),
        ShellForegroundProcessState::Absent
    );
}

/// A local PTY exposes no reliable foreground marker, and guessing one from terminal text would be
/// parsing output to invent a fact.
#[test]
fn a_local_runtime_refuses_a_remote_open_rather_than_opening_here() {
    let runtime = RetainedLocalShellRuntime::for_test();

    let error = runtime
        .open(&open_request("shell-1", true), Arc::new(SilentSink))
        .expect_err("remote refused");

    // Opening a local PTY at a remote path would open a shell on this machine and label it remote.
    assert_eq!(error.code(), "shell_runtime_unavailable");
}

/// A shell that has already finished needs no signal at all. Killing one that was on its way out
/// would replace an orderly exit with a killed one for the same result.
#[test]
fn a_child_that_has_already_exited_is_confirmed_without_being_signalled() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    let process = Arc::new(FakeProcess::exits_after(1));
    let shell_id = install(
        &runtime,
        "shell-1",
        process.clone(),
        vec![finished_worker()],
    );

    let outcome = runtime.close(&shell_id, ShellGeneration::new(1), budget());

    assert_eq!(outcome, ShellRuntimeCloseOutcome::Confirmed);
    assert_eq!(process.terminations.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.observations().terminate_requests(), 0);
    assert!(
        !runtime.holds(&shell_id),
        "a confirmed close gives up the entry"
    );
}

/// A shell that notices its stdin closed and finishes on its own, inside the graceful window. The
/// stage exists for exactly this: signalling here would replace an orderly exit with a killed one
/// for the same result.
#[test]
fn a_child_that_finishes_after_input_closes_is_never_signalled() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    // Three observations fit inside the graceful window; this one ends on the third.
    let process = Arc::new(FakeProcess::exits_after(3));
    let shell_id = install(
        &runtime,
        "shell-1",
        process.clone(),
        vec![finished_worker()],
    );

    let outcome = runtime.close(&shell_id, ShellGeneration::new(1), budget());

    assert_eq!(outcome, ShellRuntimeCloseOutcome::Confirmed);
    assert_eq!(process.terminations.load(Ordering::SeqCst), 0);
}

/// A shell that ignores the first termination request and goes on the second. Two stages rather
/// than one, because a platform whose only primitive is forceful still has to be asked twice before
/// the close gives up on it.
#[test]
fn a_child_that_survives_the_first_request_is_confirmed_after_the_force_stage() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    let process = Arc::new(FakeProcess::exits_after(8));
    let shell_id = install(
        &runtime,
        "shell-1",
        process.clone(),
        vec![finished_worker()],
    );

    let outcome = runtime.close(&shell_id, ShellGeneration::new(1), budget());

    assert_eq!(outcome, ShellRuntimeCloseOutcome::Confirmed);
    assert_eq!(process.terminations.load(Ordering::SeqCst), 2);
    assert!(!runtime.holds(&shell_id));
}

/// The ordinary case: the shell ignores the closed input, is terminated, and is then reaped.
#[test]
fn a_child_that_needs_terminating_is_terminated_and_then_confirmed() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    // Three observations fit in the graceful window before the terminate stage begins.
    let process = Arc::new(FakeProcess::exits_after(5));
    let shell_id = install(
        &runtime,
        "shell-1",
        process.clone(),
        vec![finished_worker()],
    );

    let outcome = runtime.close(&shell_id, ShellGeneration::new(1), budget());

    assert_eq!(outcome, ShellRuntimeCloseOutcome::Confirmed);
    assert!(process.terminations.load(Ordering::SeqCst) >= 1);
    assert!(!runtime.holds(&shell_id));
}

/// The whole point. A child that will not die must not produce a `Closed` Shell, and the runtime
/// must still be holding it afterwards so a retry has something to retry.
#[test]
fn a_child_that_never_exits_is_retained_rather_than_reported_closed() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    let process = Arc::new(FakeProcess::never_exits());
    let shell_id = install(
        &runtime,
        "shell-1",
        process.clone(),
        vec![finished_worker()],
    );

    let outcome = runtime.close(&shell_id, ShellGeneration::new(1), budget());

    assert!(matches!(
        outcome,
        ShellRuntimeCloseOutcome::Retained {
            retryable: true,
            ..
        }
    ));
    assert!(!outcome.is_released());
    assert!(runtime.holds(&shell_id), "ownership stays here");
    // Bounded: every stage gave up, and the observations are countable rather than unbounded.
    let checks = runtime.observations().reap_checks();
    assert!(checks > 0 && checks < 40, "{checks} observations");
    assert_eq!(runtime.observations().terminate_requests(), 2);
}

/// A kill that fails outright is reported as such rather than folded into "the deadline passed".
/// The two need different operator responses.
#[test]
fn a_kill_that_fails_is_reported_as_a_termination_failure() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    let shell_id = install(
        &runtime,
        "shell-1",
        Arc::new(FakeProcess::unkillable()),
        vec![finished_worker()],
    );

    let outcome = runtime.close(&shell_id, ShellGeneration::new(1), budget());

    let ShellRuntimeCloseOutcome::Retained { reason, .. } = outcome else {
        panic!("expected the runtime to retain the shell");
    };
    assert_eq!(reason.as_str(), "shell_terminate_failed");
    assert!(runtime.holds(&shell_id));
}

/// A platform that cannot say whether the child is alive ends the stage rather than spinning: it is
/// not going to answer if asked faster.
#[test]
fn an_observation_error_ends_the_stage_instead_of_spinning() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    let shell_id = install(
        &runtime,
        "shell-1",
        Arc::new(FakeProcess::unobservable()),
        vec![finished_worker()],
    );

    let outcome = runtime.close(&shell_id, ShellGeneration::new(1), budget());

    let ShellRuntimeCloseOutcome::Retained { reason, .. } = outcome else {
        panic!("expected the runtime to retain the shell");
    };
    assert_eq!(reason.as_str(), "shell_reap_deadline_reached");
    // Three stages, one failed observation each: no stage kept asking a question with no answer.
    assert_eq!(runtime.observations().reap_checks(), 3);
}

/// A reader blocked inside a driver read is the thread an unconditional `join()` would hang on, and
/// it is also the thread most likely to be blocked. The close returns by its deadline instead.
#[test]
fn a_worker_that_never_completes_does_not_block_the_close() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    let shell_id = install(
        &runtime,
        "shell-1",
        Arc::new(FakeProcess::exits_after(1)),
        vec![stuck_worker()],
    );

    let outcome = runtime.close(&shell_id, ShellGeneration::new(1), budget());

    let ShellRuntimeCloseOutcome::Retained { reason, retryable } = outcome else {
        panic!("expected the runtime to retain the shell");
    };
    assert_eq!(reason.as_str(), "shell_worker_completion_pending");
    assert!(retryable);
    assert!(runtime.holds(&shell_id), "the worker is still owned here");
}

/// A close for a generation the runtime no longer holds must not evict whatever now answers to that
/// id. Getting this wrong closes a Shell the user just opened.
#[test]
fn a_close_for_a_stale_generation_leaves_the_current_one_alone() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    let shell_id = install(
        &runtime,
        "shell-1",
        Arc::new(FakeProcess::exits_after(1)),
        vec![finished_worker()],
    );

    let outcome = runtime.close(&shell_id, ShellGeneration::new(2), budget());

    assert_eq!(outcome, ShellRuntimeCloseOutcome::NotHeld);
    assert!(runtime.holds(&shell_id));
}

/// Closing twice is one termination and one confirmation, not two competing attempts.
#[test]
fn a_duplicate_close_finds_nothing_left_to_close() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    let shell_id = install(
        &runtime,
        "shell-1",
        Arc::new(FakeProcess::exits_after(1)),
        vec![finished_worker()],
    );

    assert_eq!(
        runtime.close(&shell_id, ShellGeneration::new(1), budget()),
        ShellRuntimeCloseOutcome::Confirmed
    );
    assert_eq!(
        runtime.close(&shell_id, ShellGeneration::new(1), budget()),
        ShellRuntimeCloseOutcome::NotHeld
    );
}

/// A retry after an unconfirmed close continues at the same id and generation, and succeeds once
/// the child finally goes.
#[test]
fn a_retry_confirms_a_child_that_dies_between_attempts() {
    let runtime = RetainedLocalShellRuntime::with_clock(Arc::new(VirtualClock::default()));
    // Never dies during the first attempt's observations; dies before the second finishes.
    let process = Arc::new(FakeProcess::exits_after(12));
    let shell_id = install(&runtime, "shell-1", process, vec![finished_worker()]);

    let first = runtime.close(&shell_id, ShellGeneration::new(1), budget());
    let second = runtime.close(&shell_id, ShellGeneration::new(1), budget());

    assert!(matches!(first, ShellRuntimeCloseOutcome::Retained { .. }));
    assert_eq!(second, ShellRuntimeCloseOutcome::Confirmed);
    assert!(!runtime.holds(&shell_id));
}

/// A worker reports itself finished however its body leaves, including by panicking. A flag set on
/// the last line of the body would stay false for a panicking worker, and a close would then wait
/// out its whole worker window for a thread that finished before it started.
#[test]
fn a_worker_that_panics_still_reports_itself_complete() {
    let worker = ShellWorker::spawn("vanehub-test-panicking-worker".to_string(), || {
        panic!("the reader's handle went away");
    })
    .expect("spawn");

    // Bounded by construction: the body is one statement, so this loop is not a disguised join.
    for _ in 0..500 {
        if worker.try_join() {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("the worker never reported completion");
}

// ---------------------------------------------------------------------------------------------
// Startup acquisition
// ---------------------------------------------------------------------------------------------

/// Where a staged startup gives up.
///
/// Named steps rather than an index, because the interesting property is *which* resources are
/// already owned when it fails — and an index says nothing about that to a reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupStep {
    OpenPty,
    Spawn,
    Reader,
    Writer,
    Master,
    Nothing,
}

/// A terminal this side never really opened.
///
/// The child is a fake that reports itself already gone, so a rollback confirms immediately and the
/// test is about ownership rather than about how long a kill takes.
struct StagedPtyFactory {
    fail_at: StartupStep,
    spawned: Arc<AtomicUsize>,
}

impl LocalPtyFactory for StagedPtyFactory {
    fn open(
        &self,
        _dimensions: TerminalDimensions,
    ) -> Result<Box<dyn LocalPtySession>, SessionShellError> {
        if self.fail_at == StartupStep::OpenPty {
            return Err(SessionShellError::RuntimeUnavailable {
                reason: shell_reason("shell_pty_unavailable"),
            });
        }
        Ok(Box::new(StagedPtySession {
            fail_at: self.fail_at,
            spawned: self.spawned.clone(),
        }))
    }
}

struct StagedPtySession {
    fail_at: StartupStep,
    spawned: Arc<AtomicUsize>,
}

impl LocalPtySession for StagedPtySession {
    fn spawn(&mut self, _root: &Path) -> Result<Arc<dyn ShellProcessHandle>, SessionShellError> {
        if self.fail_at == StartupStep::Spawn {
            return Err(SessionShellError::RuntimeUnavailable {
                reason: shell_reason("shell_process_launch_failed"),
            });
        }
        self.spawned.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::new(FakeProcess::exits_after(0)))
    }

    fn reader(&mut self) -> Result<Box<dyn std::io::Read + Send>, SessionShellError> {
        if self.fail_at == StartupStep::Reader {
            return Err(SessionShellError::RuntimeUnavailable {
                reason: shell_reason("shell_reader_unavailable"),
            });
        }
        Ok(Box::new(std::io::empty()))
    }

    fn writer(&mut self) -> Result<Box<dyn std::io::Write + Send>, SessionShellError> {
        if self.fail_at == StartupStep::Writer {
            return Err(SessionShellError::RuntimeUnavailable {
                reason: shell_reason("shell_writer_unavailable"),
            });
        }
        Ok(Box::new(std::io::sink()))
    }

    fn master(&mut self) -> Result<Arc<dyn ShellPtyHandle>, SessionShellError> {
        if self.fail_at == StartupStep::Master {
            return Err(SessionShellError::RuntimeUnavailable {
                reason: shell_reason("shell_pty_unavailable"),
            });
        }
        Ok(Arc::new(FakeMaster))
    }
}

fn staged_runtime(fail_at: StartupStep) -> (RetainedLocalShellRuntime, Arc<AtomicUsize>) {
    let spawned = Arc::new(AtomicUsize::new(0));
    let runtime = RetainedLocalShellRuntime::with_pty(
        Arc::new(StagedPtyFactory {
            fail_at,
            spawned: spawned.clone(),
        }),
        Arc::new(RecordingStartupDiagnostics::default()),
    );
    (runtime, spawned)
}

fn local_open(id: &str) -> ShellRuntimeOpen {
    ShellRuntimeOpen {
        shell_id: shell(id),
        generation: ShellGeneration::new(1),
        session_id: "session-1".to_string(),
        root: ".".to_string(),
        dimensions: TerminalDimensions::bounded(24, 80),
        remote: None,
    }
}

/// Each acquisition failure is its own code, and none of them leaves a Shell behind.
///
/// One code per step because a reader acting on "startup failed" can do nothing, while "the PTY is
/// unavailable" and "the shell could not be started" point at different problems on their machine.
#[test]
fn every_startup_step_reports_its_own_failure_and_keeps_nothing() {
    let expected = [
        (StartupStep::OpenPty, "shell_pty_unavailable"),
        (StartupStep::Spawn, "shell_process_launch_failed"),
        (StartupStep::Reader, "shell_reader_unavailable"),
        (StartupStep::Writer, "shell_writer_unavailable"),
        (StartupStep::Master, "shell_pty_unavailable"),
    ];

    for (step, reason) in expected {
        let (runtime, _) = staged_runtime(step);

        let error = runtime
            .open(&local_open("shell-1"), Arc::new(SilentSink))
            .expect_err("startup fails");

        assert_eq!(error.code(), "shell_runtime_unavailable", "{step:?}");
        assert!(
            format!("{error:?}").contains(reason),
            "{step:?} did not carry {reason}"
        );
        // Nothing is registered, so a later close finds nothing to close rather than a handle to a
        // terminal that was never finished.
        assert_eq!(
            runtime.close(&shell("shell-1"), ShellGeneration::new(1), budget()),
            ShellRuntimeCloseOutcome::NotHeld,
            "{step:?}"
        );
    }
}

/// A failure after the child exists still ends the child.
///
/// This is the case the stepwise seam exists for. Failing at `OpenPty` proves nothing — nothing was
/// running — while failing at the reader means a live process exists that only the guard knows
/// about.
#[test]
fn a_failure_after_the_child_exists_still_rolls_the_child_back() {
    for step in [
        StartupStep::Reader,
        StartupStep::Writer,
        StartupStep::Master,
    ] {
        let (runtime, spawned) = staged_runtime(step);

        runtime
            .open(&local_open("shell-1"), Arc::new(SilentSink))
            .expect_err("startup fails");

        assert_eq!(spawned.load(Ordering::SeqCst), 1, "{step:?}");
    }
}

/// A startup that never spawned anything reports no rollback, because there was nothing to roll.
#[test]
fn a_startup_that_never_spawned_reports_no_unconfirmed_cleanup() {
    let diagnostics = Arc::new(RecordingStartupDiagnostics::default());
    let runtime = RetainedLocalShellRuntime::with_pty(
        Arc::new(StagedPtyFactory {
            fail_at: StartupStep::OpenPty,
            spawned: Arc::new(AtomicUsize::new(0)),
        }),
        diagnostics.clone(),
    );

    runtime
        .open(&local_open("shell-1"), Arc::new(SilentSink))
        .expect_err("startup fails");

    // An unconfirmed-cleanup record here would tell an operator a child outlived its startup when
    // no child was ever created.
    assert_eq!(diagnostics.rollbacks.load(Ordering::SeqCst), 0);
}

/// Records only that a rollback could not confirm, which is all these cases need to distinguish.
#[derive(Default)]
struct RecordingStartupDiagnostics {
    rollbacks: AtomicUsize,
}

impl ShellLifecycleDiagnosticsPort for RecordingStartupDiagnostics {
    fn stale_reaper_completion(&self, _shell_id: &str, _attempted: u64, _current: u64) {}
    fn orphaned_reaper_completion(&self, _shell_id: &str, _attempted: u64) {}
    fn startup_rollback_unconfirmed(&self, _shell_id: &str, _generation: u64, _reason: &str) {
        self.rollbacks.fetch_add(1, Ordering::SeqCst);
    }
}
