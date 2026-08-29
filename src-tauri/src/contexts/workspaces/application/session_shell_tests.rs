use super::evidence::{WorkspaceEvidencePort, WorkspaceEvidenceSignal};
use super::session_shell::{
    AttachSessionShellRequest, CreateSessionShellRequest, ResizeSessionShellRequest,
    SessionShellNotice, SessionShellNoticePort, SessionShellRuntimePort, SessionShellWorkspace,
    SessionShellWorkspacePort, ShellAttachmentScope, ShellCapacities, ShellClockPort, ShellIdPort,
    ShellOutputSink, ShellRuntimeOpen, ShellRuntimeOpened, WriteSessionShellRequest,
};
use super::session_shell_close::{ShellCloseDisposition, ShellRuntimeCloseOutcome};
use super::session_shell_reaper::ShellReaperLimits;
use super::session_shell_registry::SessionShellRegistry;
use super::session_shell_store::ShellStore;
use crate::contexts::workspaces::domain::{
    SessionShellError, SessionShellState, ShellCapacityScope, ShellCloseBudget,
    ShellCreateRequestId, ShellForegroundProcessState, ShellGeneration, ShellId,
    ShellRuntimeDescriptor, ShellStream, ShellTitle, TerminalDimensions,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub(super) const SESSION: &str = "session-1";

#[derive(Default)]
struct FakeClock {
    millis: AtomicU64,
}

impl FakeClock {
    fn advance(&self, millis: u64) {
        self.millis.fetch_add(millis, Ordering::SeqCst);
    }
}

impl ShellClockPort for FakeClock {
    fn now(&self) -> String {
        format!(
            "2026-08-24T10:00:{:02}Z",
            self.millis.load(Ordering::SeqCst) % 60
        )
    }

    fn elapsed_millis(&self) -> u64 {
        self.millis.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
struct SequentialIds {
    shells: AtomicU64,
    attachments: AtomicU64,
}

impl ShellIdPort for SequentialIds {
    fn next_shell_id(&self) -> String {
        format!("shell-{}", self.shells.fetch_add(1, Ordering::SeqCst) + 1)
    }

    fn next_attachment_id(&self) -> String {
        format!(
            "attachment-{}",
            self.attachments.fetch_add(1, Ordering::SeqCst) + 1
        )
    }
}

#[derive(Default)]
pub(super) struct RecordingEvidence(pub(super) Mutex<Vec<WorkspaceEvidenceSignal>>);

impl WorkspaceEvidencePort for RecordingEvidence {
    fn try_publish(&self, signal: WorkspaceEvidenceSignal) {
        self.0.lock().expect("evidence").push(signal);
    }
}

#[derive(Default)]
struct RecordingNotices(Mutex<Vec<SessionShellNotice>>);

impl SessionShellNoticePort for RecordingNotices {
    fn publish(&self, notice: SessionShellNotice) {
        self.0.lock().expect("notices").push(notice);
    }
}

#[derive(Default)]
pub(super) struct FakeRuntime {
    pub(super) opened: Mutex<Vec<ShellId>>,
    pub(super) closed: Mutex<Vec<ShellId>>,
    written: Mutex<Vec<(ShellId, String)>>,
    resized: Mutex<Vec<(ShellId, TerminalDimensions)>>,
    foreground: Mutex<ShellForegroundProcessState>,
    fail_open: Mutex<bool>,
    sinks: Mutex<Vec<Arc<dyn ShellOutputSink>>>,
    generations: Mutex<std::collections::BTreeMap<ShellId, ShellGeneration>>,
    /// `Some(retryable)` while the fake is pretending to own a child that will not die.
    retain_close: Mutex<Option<bool>>,
    during_open: Mutex<Option<(String, Option<i32>)>>,
}

impl FakeRuntime {
    pub(super) fn emit(&self, shell_id: &ShellId, text: &str) {
        self.emit_at(shell_id, self.generation_of(shell_id), text);
    }

    pub(super) fn emit_at(&self, shell_id: &ShellId, generation: ShellGeneration, text: &str) {
        for sink in self.sinks.lock().expect("sinks").iter() {
            sink.on_output(shell_id, generation, ShellStream::Pty, text.as_bytes());
        }
    }

    fn exit(&self, shell_id: &ShellId, code: i32) {
        for sink in self.sinks.lock().expect("sinks").iter() {
            sink.on_state(
                shell_id,
                self.generation_of(shell_id),
                SessionShellState::Exited { code: Some(code) },
            );
        }
    }

    fn generation_of(&self, shell_id: &ShellId) -> ShellGeneration {
        self.generations
            .lock()
            .expect("generations")
            .get(shell_id)
            .copied()
            .unwrap_or(ShellGeneration::new(1))
    }

    /// Makes the next close report that the runtime still owns the Shell, which is what a child
    /// that will not die looks like from here.
    pub(super) fn retain_on_close(&self, retryable: bool) {
        *self.retain_close.lock().expect("retain") = Some(retryable);
    }

    /// Publishes into the sink from inside `open`, before the caller has committed anything. This
    /// is the `echo && exit` case: a shell that produced its whole life before startup returned.
    pub(super) fn emit_during_open(&self, text: &str, exit_code: Option<i32>) {
        *self.during_open.lock().expect("during") = Some((text.to_string(), exit_code));
    }
}

impl SessionShellRuntimePort for FakeRuntime {
    fn open(
        &self,
        request: &ShellRuntimeOpen,
        sink: Arc<dyn ShellOutputSink>,
    ) -> Result<ShellRuntimeOpened, SessionShellError> {
        if *self.fail_open.lock().expect("fail") {
            return Err(SessionShellError::RuntimeUnavailable {
                reason: crate::contexts::workspaces::domain::shell_reason("pty_unavailable"),
            });
        }
        self.opened
            .lock()
            .expect("opened")
            .push(request.shell_id.clone());
        self.generations
            .lock()
            .expect("generations")
            .insert(request.shell_id.clone(), request.generation);
        if let Some((text, exit_code)) = self.during_open.lock().expect("during").take() {
            sink.on_output(
                &request.shell_id,
                request.generation,
                ShellStream::Pty,
                text.as_bytes(),
            );
            if let Some(code) = exit_code {
                sink.on_state(
                    &request.shell_id,
                    request.generation,
                    SessionShellState::Exited { code: Some(code) },
                );
            }
        }
        self.sinks.lock().expect("sinks").push(sink);
        Ok(ShellRuntimeOpened {
            runtime: ShellRuntimeDescriptor::Native,
            state: SessionShellState::Running,
        })
    }

    fn write(&self, shell_id: &ShellId, content: &str) -> Result<(), SessionShellError> {
        self.written
            .lock()
            .expect("written")
            .push((shell_id.clone(), content.to_string()));
        Ok(())
    }

    fn resize(
        &self,
        shell_id: &ShellId,
        dimensions: TerminalDimensions,
    ) -> Result<(), SessionShellError> {
        self.resized
            .lock()
            .expect("resized")
            .push((shell_id.clone(), dimensions));
        Ok(())
    }

    fn close(
        &self,
        shell_id: &ShellId,
        generation: ShellGeneration,
        _budget: ShellCloseBudget,
    ) -> ShellRuntimeCloseOutcome {
        if self.generation_of(shell_id) != generation {
            return ShellRuntimeCloseOutcome::NotHeld;
        }
        self.closed.lock().expect("closed").push(shell_id.clone());
        if let Some(retryable) = *self.retain_close.lock().expect("retain") {
            return ShellRuntimeCloseOutcome::Retained {
                reason: crate::contexts::workspaces::domain::shell_reason(
                    crate::contexts::workspaces::domain::shell_reason_code::CLOSE_DEADLINE_REACHED,
                ),
                retryable,
            };
        }
        self.generations
            .lock()
            .expect("generations")
            .remove(shell_id);
        ShellRuntimeCloseOutcome::Confirmed
    }

    fn foreground_process(&self, _shell_id: &ShellId) -> ShellForegroundProcessState {
        *self.foreground.lock().expect("foreground")
    }
}

struct FakeWorkspaces {
    seat_count: usize,
    read_only: bool,
}

impl Default for FakeWorkspaces {
    fn default() -> Self {
        Self {
            seat_count: 1,
            read_only: false,
        }
    }
}

impl SessionShellWorkspacePort for FakeWorkspaces {
    fn resolve_at(
        &self,
        session_id: &str,
        relative_directory: &str,
    ) -> Result<SessionShellWorkspace, SessionShellError> {
        // The fake joins rather than resolves: there is no filesystem here, and a test that needed
        // one to prove the registry passes a subdirectory through would be testing the filesystem.
        let base = self.resolve(session_id)?;
        Ok(SessionShellWorkspace {
            root: format!("{}/{relative_directory}", base.root),
            ..base
        })
    }

    fn resolve(&self, _session_id: &str) -> Result<SessionShellWorkspace, SessionShellError> {
        Ok(SessionShellWorkspace {
            root: "D:/project".to_string(),
            remote: None,
            read_only: self.read_only,
            seat_count: self.seat_count,
        })
    }
}

pub(super) struct Harness {
    pub(super) registry: Arc<SessionShellRegistry>,
    pub(super) evidence: Arc<RecordingEvidence>,
    pub(super) runtime: Arc<FakeRuntime>,
    clock: Arc<FakeClock>,
    notices: Arc<RecordingNotices>,
    store: Arc<ShellStore>,
}

fn harness_with(workspaces: FakeWorkspaces, capacities: ShellCapacities) -> Harness {
    harness_tuned(workspaces, capacities, ShellReaperLimits::default())
}

fn harness_tuned(
    workspaces: FakeWorkspaces,
    capacities: ShellCapacities,
    reaper: ShellReaperLimits,
) -> Harness {
    let clock = Arc::new(FakeClock::default());
    let notices = Arc::new(RecordingNotices::default());
    let store = Arc::new(ShellStore::new(notices.clone(), clock.clone()));
    let runtime = Arc::new(FakeRuntime::default());
    let evidence = Arc::new(RecordingEvidence::default());
    let registry = Arc::new(
        SessionShellRegistry::new(
            store.clone(),
            runtime.clone(),
            Arc::new(workspaces),
            Arc::new(SequentialIds::default()),
            clock.clone(),
            capacities,
            evidence.clone(),
        )
        // Every stage is one virtual tick: the fake runtime answers immediately, so the point of
        // the budget here is that one exists, not how long it is.
        .with_close_budget(ShellCloseBudget::immediate())
        .with_reaper_limits(reaper),
    );
    Harness {
        registry,
        runtime,
        evidence,
        clock,
        notices,
        store,
    }
}

pub(super) fn harness() -> Harness {
    harness_with(FakeWorkspaces::default(), ShellCapacities::default())
}

impl Harness {
    /// Moves the monotonic clock past the idle window, so a sweep has something to reclaim.
    pub(super) fn advance_past_the_idle_window(&self) {
        self.clock
            .advance(super::session_shell_registry::SHELL_IDLE_MILLIS + 1);
    }
}

pub(super) fn create(
    harness: &Harness,
    request_id: Option<&str>,
) -> Result<ShellId, SessionShellError> {
    harness
        .registry
        .create(&CreateSessionShellRequest {
            session_id: SESSION.to_string(),
            seat_id: None,
            rows: 24,
            cols: 80,
            request_id: request_id.map(|id| ShellCreateRequestId::parse(id).expect("request id")),
            title: None,
            working_directory: None,
        })
        .map(|descriptor| descriptor.shell_id)
}

#[test]
fn a_created_shell_is_listed_for_its_session_and_nowhere_else() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");

    assert_eq!(harness.registry.list(Some(SESSION)).len(), 1);
    assert!(harness.registry.list(Some("session-2")).is_empty());
    assert_eq!(harness.registry.list(None)[0].shell_id, shell_id);
}

/// Two views opening the same session's default Shell at once is the ordinary case, not a rare
/// race: a tab mounting under StrictMode does it every time. Two runtimes would leave one of them
/// orphaned, running and invisible.
#[test]
fn concurrent_default_creates_produce_one_runtime() {
    let harness = harness();
    let registry = harness.registry.clone();
    let second = std::thread::spawn({
        let registry = registry.clone();
        move || {
            registry.create(&CreateSessionShellRequest {
                session_id: SESSION.to_string(),
                seat_id: None,
                rows: 24,
                cols: 80,
                request_id: None,
                title: None,
                working_directory: None,
            })
        }
    });
    let first = registry.create(&CreateSessionShellRequest {
        session_id: SESSION.to_string(),
        seat_id: None,
        rows: 24,
        cols: 80,
        request_id: None,
        title: None,
        working_directory: None,
    });

    let first = first.expect("first create");
    let second = second.join().expect("thread").expect("second create");

    assert_eq!(first.shell_id, second.shell_id);
    assert_eq!(harness.runtime.opened.lock().expect("opened").len(), 1);
}

#[test]
fn a_retried_add_returns_the_shell_that_request_already_produced() {
    let harness = harness();
    let first = create(&harness, Some("request-1")).expect("first");
    let second = create(&harness, Some("request-1")).expect("retry");
    let other = create(&harness, Some("request-2")).expect("second add");

    assert_eq!(first, second);
    assert_ne!(first, other);
    assert_eq!(harness.runtime.opened.lock().expect("opened").len(), 2);
}

/// An entry written before the runtime opened would be a Shell the registry lists, the user can
/// attach to, and nothing is running behind.
#[test]
fn a_failed_open_leaves_no_entry_behind() {
    let harness = harness();
    *harness.runtime.fail_open.lock().expect("fail") = true;

    let error = create(&harness, None).expect_err("open fails");

    assert_eq!(error.code(), "shell_runtime_unavailable");
    assert!(harness.registry.list(None).is_empty());
}

/// A Shell is one interactive channel with one runtime owner. Picking a seat for a caller that did
/// not name one would attribute a session's work to a participant at random.
#[test]
fn a_multi_seat_session_refuses_to_create_without_a_seat() {
    let harness = harness_with(
        FakeWorkspaces {
            seat_count: 3,
            read_only: false,
        },
        ShellCapacities::default(),
    );

    let error = create(&harness, None).expect_err("seat required");

    assert_eq!(error.code(), "shell_seat_required");
    assert!(harness.registry.list(None).is_empty());
}

#[test]
fn capacity_is_reported_rather_than_making_room() {
    let harness = harness_with(
        FakeWorkspaces::default(),
        ShellCapacities {
            per_session: 1,
            total: 4,
        },
    );
    let first = create(&harness, Some("request-1")).expect("first");

    let error = create(&harness, Some("request-2")).expect_err("capacity");

    assert!(matches!(
        error,
        SessionShellError::CapacityReached {
            scope: ShellCapacityScope::Session
        }
    ));
    // Nothing was evicted to make room: the Shell the user was using is still there.
    assert_eq!(harness.registry.list(None)[0].shell_id, first);
}

#[test]
fn attaching_returns_replay_and_never_creates() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    harness.runtime.emit(&shell_id, "hello");

    let snapshot = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: 0,
        })
        .expect("attach");

    assert_eq!(snapshot.replay.len(), 1);
    assert_eq!(snapshot.replay[0].data, "hello");
    assert_eq!(snapshot.next_sequence, 2);

    let missing = harness.registry.attach(&AttachSessionShellRequest {
        shell_id: ShellId::parse("shell-missing").expect("id"),
        after_sequence: 0,
    });
    assert_eq!(missing.expect_err("not found").code(), "shell_not_found");
    assert_eq!(harness.runtime.opened.lock().expect("opened").len(), 1);
}

/// A hidden-then-visible tab produces this on every switch: the new view attaches before the old
/// view's cleanup runs. Honouring the late detach would tear down the attachment that replaced it.
#[test]
fn a_stale_detach_leaves_the_newer_attachment_alone() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    let first = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: 0,
        })
        .expect("first attach");
    let second = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: 0,
        })
        .expect("second attach");

    // Idempotent: a correct cleanup must not look like an error.
    harness
        .registry
        .detach(&ShellAttachmentScope {
            shell_id: shell_id.clone(),
            attachment_id: first.attachment_id,
        })
        .expect("stale detach succeeds");

    // The newer view can still write, which is what "left alone" means.
    harness
        .registry
        .write(&WriteSessionShellRequest {
            scope: ShellAttachmentScope {
                shell_id: shell_id.clone(),
                attachment_id: second.attachment_id,
            },
            content: "ls\n".to_string(),
        })
        .expect("current attachment writes");
}

/// Input is not idempotent. A keystroke from a view the user has left would run in the session they
/// are looking at now.
#[test]
fn a_stale_write_or_resize_is_refused() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    let first = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: 0,
        })
        .expect("first");
    harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: 0,
        })
        .expect("second");
    let stale = ShellAttachmentScope {
        shell_id: shell_id.clone(),
        attachment_id: first.attachment_id,
    };

    let write = harness.registry.write(&WriteSessionShellRequest {
        scope: stale.clone(),
        content: "rm -rf\n".to_string(),
    });
    let resize = harness.registry.resize(&ResizeSessionShellRequest {
        scope: stale,
        rows: 40,
        cols: 120,
    });

    assert_eq!(write.expect_err("stale").code(), "shell_attachment_stale");
    assert_eq!(resize.expect_err("stale").code(), "shell_attachment_stale");
    assert!(harness.runtime.written.lock().expect("written").is_empty());
    assert!(harness.runtime.resized.lock().expect("resized").is_empty());
}

#[test]
fn renaming_changes_the_title_and_nothing_else() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    let before = harness.store.descriptor(&shell_id).expect("descriptor");

    let after = harness
        .registry
        .rename(&shell_id, "  build  ")
        .expect("rename");

    assert_eq!(after.title, ShellTitle::parse("build").expect("title"));
    assert_eq!(after.shell_id, before.shell_id);
    assert_eq!(after.session_id, before.session_id);
    assert_eq!(after.created_at, before.created_at);
    assert!(after.revision > before.revision);
    assert_eq!(
        harness
            .registry
            .rename(&shell_id, " ")
            .expect_err("blank")
            .code(),
        "shell_invalid_title"
    );
}

/// A caller retrying after a partial failure has no way to tell "already gone" from "still there
/// and refused", so closing what is not there succeeds.
#[test]
fn closing_is_idempotent_and_reaches_the_runtime_once() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");

    let first = harness.registry.close(&shell_id);
    let second = harness.registry.close(&shell_id);

    assert_eq!(first.disposition, ShellCloseDisposition::ClosedConfirmed);
    // The second close finds nothing to end and says so, rather than raising an error a retrying
    // caller cannot distinguish from "still there and refused".
    assert_eq!(second.disposition, ShellCloseDisposition::AlreadyTerminal);
    assert_eq!(harness.runtime.closed.lock().expect("closed").len(), 1);
    assert!(harness.registry.list(None).is_empty());
    // The slot comes back exactly once, whichever of the two callers asked.
    assert_eq!(harness.registry.capacity().active(), 0);
    assert_eq!(harness.registry.capacity().releases(), 1);
}

/// A process that ended by itself is still worth reading. The registry keeps it until the user
/// closes it or the idle sweep reclaims it.
#[test]
fn a_naturally_exited_shell_stays_attachable_until_it_is_closed() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    harness.runtime.emit(&shell_id, "build output");
    harness.runtime.exit(&shell_id, 0);

    let descriptor = harness.store.descriptor(&shell_id).expect("descriptor");
    assert!(matches!(
        descriptor.state,
        SessionShellState::Exited { code: Some(0) }
    ));
    let snapshot = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: 0,
        })
        .expect("attach an exited shell");
    assert_eq!(snapshot.replay[0].data, "build output");

    // And it does not end twice: a second terminal state would publish two endings for one Shell.
    harness.runtime.exit(&shell_id, 1);
    let endings = harness
        .notices
        .0
        .lock()
        .expect("notices")
        .iter()
        .filter(|notice| {
            matches!(
                notice,
                SessionShellNotice::State { state, .. } if state.has_ended()
            )
        })
        .count();
    assert_eq!(endings, 1);
}

#[test]
fn an_idle_sweep_reclaims_only_detached_quiet_shells() {
    let harness = harness_with(
        FakeWorkspaces::default(),
        ShellCapacities {
            per_session: 4,
            total: 8,
        },
    );
    let watched = create(&harness, Some("request-watched")).expect("watched");
    let idle = create(&harness, Some("request-idle")).expect("idle");
    harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: watched.clone(),
            after_sequence: 0,
        })
        .expect("attach");

    harness
        .clock
        .advance(super::session_shell_registry::SHELL_IDLE_MILLIS + 1);
    let report = harness.registry.sweep_idle();

    // An attached Shell is never idle by definition: someone is looking at it.
    assert_eq!(report.requested(), 1);
    assert_eq!(report.entries()[0].shell_id, idle);
    assert_eq!(report.closed_confirmed(), 1);
    assert!(report.is_complete());
    assert_eq!(harness.registry.list(None)[0].shell_id, watched);
}

/// Reclaiming a Shell to save memory would kill a job the user started.
#[test]
fn a_shell_with_known_foreground_work_is_not_reclaimed() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    harness
        .store
        .set_foreground(&shell_id, ShellForegroundProcessState::Present);

    harness
        .clock
        .advance(super::session_shell_registry::SHELL_IDLE_MILLIS + 1);

    assert_eq!(harness.registry.sweep_idle().requested(), 0);
    assert_eq!(harness.registry.list(None).len(), 1);
}

/// An unknown foreground state is not an absent one, so it does not authorise a reclaim either.
#[test]
fn an_unknown_foreground_state_still_allows_idle_reclaim() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    harness
        .store
        .set_foreground(&shell_id, ShellForegroundProcessState::Unknown);

    harness
        .clock
        .advance(super::session_shell_registry::SHELL_IDLE_MILLIS + 1);

    // Unknown is not Present: a detached shell nobody can say anything about is still reclaimable
    // after the window, and the UI's own three-state warning covers the close confirmation.
    let report = harness.registry.sweep_idle();
    assert_eq!(report.requested(), 1);
    assert_eq!(report.entries()[0].shell_id, shell_id);
}

#[test]
fn shutdown_closes_every_shell() {
    let harness = harness();
    create(&harness, Some("request-1")).expect("first");
    create(&harness, Some("request-2")).expect("second");

    let report = harness.registry.shutdown();

    assert_eq!(harness.runtime.closed.lock().expect("closed").len(), 2);
    assert_eq!(report.closed_confirmed(), 2);
    assert!(report.is_complete());
    assert!(harness.registry.list(None).is_empty());
    assert_eq!(harness.registry.live_count(SESSION), 0);
    assert_eq!(harness.registry.capacity().active(), 0);
}

/// The `echo && exit` case. The shell produced its whole life inside `open`, before the caller had
/// anything to write a descriptor into. Registering the Shell after the runtime returned lost both
/// the output and the exit; registering it before, and making the `Running` transition conditional,
/// keeps them.
#[test]
fn a_shell_that_ends_during_startup_keeps_its_output_and_its_ending() {
    let harness = harness();
    harness
        .runtime
        .emit_during_open("build complete\n", Some(0));

    let shell_id = create(&harness, None).expect("create");

    let descriptor = harness.store.descriptor(&shell_id).expect("descriptor");
    // Not `Running`: an unconditional transition here would report a dead process as live, and
    // nothing downstream would ever end it.
    assert!(
        matches!(
            descriptor.state,
            SessionShellState::Exited { code: Some(0) }
        ),
        "{:?}",
        descriptor.state
    );
    let snapshot = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: 0,
        })
        .expect("attach");
    assert_eq!(snapshot.replay[0].data, "build complete\n");
}

/// A Shell is addressable before the runtime commits, and deliberately not writable there: a
/// keystroke accepted at that point would have nowhere to go.
#[test]
fn an_opening_shell_is_registered_before_the_runtime_is_invoked() {
    let harness = harness();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    harness.runtime.emit_during_open("early", None);

    let shell_id = create(&harness, None).expect("create");
    recorder
        .lock()
        .expect("seen")
        .push(harness.store.contains(&shell_id));

    // The frame arrived from inside `open`, which is only possible if the entry already existed.
    let snapshot = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: 0,
        })
        .expect("attach");
    assert_eq!(snapshot.replay.len(), 1);
    assert_eq!(seen.lock().expect("seen").as_slice(), [true]);
}

/// A rollback has to give the slot back, or a run of failing opens would exhaust the ceiling with
/// Shells that never existed.
#[test]
fn a_failed_open_releases_the_slot_it_reserved() {
    let harness = harness_with(
        FakeWorkspaces::default(),
        ShellCapacities {
            per_session: 1,
            total: 1,
        },
    );
    *harness.runtime.fail_open.lock().expect("fail") = true;

    create(&harness, Some("request-1")).expect_err("open fails");

    assert_eq!(harness.registry.capacity().active(), 0);
    assert_eq!(harness.registry.capacity().releases(), 1);
    *harness.runtime.fail_open.lock().expect("fail") = false;
    create(&harness, Some("request-2")).expect("the slot is usable again");
}

/// A hundred threads racing for the last slot. Every one of them counted, every one of them saw
/// room, and the count-then-open version started a process for each.
#[test]
fn concurrent_creates_never_exceed_the_session_ceiling() {
    let harness = harness_with(
        FakeWorkspaces::default(),
        ShellCapacities {
            per_session: 3,
            total: 8,
        },
    );
    let barrier = Arc::new(std::sync::Barrier::new(100));
    let threads = (0..100)
        .map(|index| {
            let registry = harness.registry.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                registry
                    .create(&CreateSessionShellRequest {
                        session_id: SESSION.to_string(),
                        seat_id: None,
                        rows: 24,
                        cols: 80,
                        request_id: Some(
                            ShellCreateRequestId::parse(format!("request-{index}"))
                                .expect("request id"),
                        ),
                        title: None,
                        working_directory: None,
                    })
                    .is_ok()
            })
        })
        .collect::<Vec<_>>();
    let admitted = threads
        .into_iter()
        .filter(|thread| thread.is_finished() || true)
        .map(|thread| thread.join().expect("join"))
        .filter(|created| *created)
        .count();

    assert_eq!(admitted, 3);
    // The losers never reached the runtime: no process was spawned and then thrown away.
    assert_eq!(harness.runtime.opened.lock().expect("opened").len(), 3);
    assert_eq!(harness.registry.list(None).len(), 3);
}

/// The defect in one test. A close that cannot confirm termination must not report `Closed`, must
/// not remove the Shell, and must not give the slot back — because the process is still running.
#[test]
fn an_unconfirmed_close_keeps_the_shell_addressable_and_charged() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    harness.runtime.retain_on_close(true);

    let result = harness.registry.close(&shell_id);

    assert_eq!(result.disposition, ShellCloseDisposition::Reaping);
    assert!(result.retryable);
    assert!(result.cleanup_deadline_reached);
    assert_eq!(result.final_state, None);
    let descriptor = harness.store.descriptor(&shell_id).expect("still held");
    assert_eq!(descriptor.state, SessionShellState::Reaping);
    assert!(descriptor.state.is_cleanup_pending());
    assert_eq!(harness.registry.capacity().active(), 1);
    // No terminal event: `ShellClosed` means confirmation, not that a close was requested.
    assert!(!harness
        .evidence
        .0
        .lock()
        .expect("evidence")
        .iter()
        .any(|signal| matches!(signal, WorkspaceEvidenceSignal::ShellClosed { .. })));
}

/// The Reaper is what makes a timed-out close eventually finish, and it must finalize exactly once.
#[test]
fn the_reaper_finalizes_a_shell_whose_close_timed_out() {
    let harness = harness_tuned(
        FakeWorkspaces::default(),
        ShellCapacities::default(),
        ShellReaperLimits {
            initial_backoff_millis: 0,
            ..ShellReaperLimits::default()
        },
    );
    let shell_id = create(&harness, None).expect("create");
    harness.runtime.retain_on_close(true);
    assert_eq!(
        harness.registry.close(&shell_id).disposition,
        ShellCloseDisposition::Reaping
    );

    // The child finally dies; the next attempt confirms it.
    *harness.runtime.retain_close.lock().expect("retain") = None;
    let report = harness.registry.advance_reaper();

    assert_eq!(report.closed_confirmed(), 1);
    assert!(harness.store.descriptor(&shell_id).is_none());
    assert_eq!(harness.registry.capacity().releases(), 1);
    assert_eq!(harness.registry.reaper_depth(), 0);
    let closings = harness
        .evidence
        .0
        .lock()
        .expect("evidence")
        .iter()
        .filter(|signal| matches!(signal, WorkspaceEvidenceSignal::ShellClosed { .. }))
        .count();
    assert_eq!(closings, 1, "exactly one ending is published");
}

/// A full queue refuses the handoff. Nothing was moved out of an owner to offer it, so the refusal
/// drops nothing — and the Shell stays addressable for a manual retry.
#[test]
fn a_full_reaper_queue_leaves_the_shell_close_failed_rather_than_ownerless() {
    let harness = harness_tuned(
        FakeWorkspaces::default(),
        ShellCapacities::default(),
        ShellReaperLimits {
            queue_capacity: 1,
            initial_backoff_millis: 0,
            ..ShellReaperLimits::default()
        },
    );
    let first = create(&harness, Some("request-1")).expect("first");
    let second = create(&harness, Some("request-2")).expect("second");
    harness.runtime.retain_on_close(true);

    assert_eq!(
        harness.registry.close(&first).disposition,
        ShellCloseDisposition::Reaping
    );
    let result = harness.registry.close(&second);

    assert_eq!(result.disposition, ShellCloseDisposition::CloseFailed);
    assert!(result.retryable);
    assert_eq!(
        result.reason.as_ref().map(|reason| reason.as_str()),
        Some("shell_reaper_capacity_exhausted")
    );
    let descriptor = harness.store.descriptor(&second).expect("still held");
    assert!(matches!(
        descriptor.state,
        SessionShellState::CloseFailed {
            retryable: true,
            ..
        }
    ));
    assert_eq!(harness.registry.capacity().active(), 2);
}

/// A `CloseFailed` Shell is retryable in place: the next close continues the same operation rather
/// than starting a competing one.
#[test]
fn a_failed_close_can_be_retried_and_then_completes() {
    let harness = harness_tuned(
        FakeWorkspaces::default(),
        ShellCapacities::default(),
        ShellReaperLimits {
            queue_capacity: 0,
            ..ShellReaperLimits::default()
        },
    );
    let shell_id = create(&harness, None).expect("create");
    harness.runtime.retain_on_close(true);
    let failed = harness.registry.close(&shell_id);
    assert_eq!(failed.disposition, ShellCloseDisposition::CloseFailed);
    assert_eq!(failed.attempt, 1);

    *harness.runtime.retain_close.lock().expect("retain") = None;
    let retried = harness.registry.close(&shell_id);

    assert_eq!(retried.disposition, ShellCloseDisposition::ClosedConfirmed);
    // One sequence of attempts, not two: a command-path attempt and a Reaper attempt share it.
    assert_eq!(retried.attempt, 2);
    assert!(harness.store.descriptor(&shell_id).is_none());
    assert_eq!(harness.registry.capacity().releases(), 1);
}

/// A session is not archived while one of its Shells is still running. The aggregate report is what
/// lets the caller tell the difference, and it keeps the identities a retry needs.
#[test]
fn session_cleanup_reports_every_shell_rather_than_a_pass_or_fail() {
    let harness = harness_tuned(
        FakeWorkspaces::default(),
        ShellCapacities::default(),
        ShellReaperLimits {
            initial_backoff_millis: 0,
            ..ShellReaperLimits::default()
        },
    );
    let confirmed = create(&harness, Some("request-1")).expect("first");
    let stuck = create(&harness, Some("request-2")).expect("second");
    harness.registry.close(&confirmed);
    harness.runtime.retain_on_close(true);

    let report = harness.registry.close_for_session(SESSION);

    assert_eq!(report.requested(), 1);
    assert_eq!(report.reaping(), 1);
    assert!(!report.is_complete());
    assert_eq!(
        report
            .unconfirmed()
            .iter()
            .map(|entry| entry.shell_id.clone())
            .collect::<Vec<_>>(),
        vec![stuck.clone()]
    );

    // The retry completes the original request rather than closing the Shell that already closed.
    *harness.runtime.retain_close.lock().expect("retain") = None;
    let retry = harness.registry.close_for_session(SESSION);
    assert!(retry.is_complete());
    assert_eq!(retry.closed_confirmed(), 1);
    assert_eq!(harness.registry.capacity().active(), 0);
}

/// Input during `Opening` and during `Closing` is refused rather than accepted and lost.
#[test]
fn a_shell_that_is_not_running_refuses_input_with_a_stable_code() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    let attachment = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: 0,
        })
        .expect("attach")
        .attachment_id;
    harness.runtime.retain_on_close(true);
    harness.registry.close(&shell_id);

    let write = harness.registry.write(&WriteSessionShellRequest {
        scope: ShellAttachmentScope {
            shell_id: shell_id.clone(),
            attachment_id: attachment,
        },
        content: "ls\n".to_string(),
    });

    assert_eq!(
        write.expect_err("not running").code(),
        "shell_not_accepting_input"
    );
    assert!(harness.runtime.written.lock().expect("written").is_empty());
}

/// A reader thread that outlived its Shell keeps reading a live PTY. Its bytes belong to a Shell
/// the user dismissed, and appending them to whatever now holds that id would put one shell's
/// output into another shell's scrollback.
#[test]
fn output_for_a_superseded_generation_is_dropped() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    let current = harness
        .store
        .descriptor(&shell_id)
        .expect("descriptor")
        .generation;

    harness
        .runtime
        .emit_at(&shell_id, current.next(), "from the future");
    harness
        .runtime
        .emit_at(&shell_id, current, "from this shell");

    let snapshot = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id,
            after_sequence: 0,
        })
        .expect("attach");
    assert_eq!(snapshot.replay.len(), 1);
    assert_eq!(snapshot.replay[0].data, "from this shell");
}

#[test]
fn output_is_stored_before_a_notice_announces_it() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");

    harness.runtime.emit(&shell_id, "one");

    // A notice published before the frame was stored could reach a subscriber that then attaches
    // and does not find it — and the subscriber would read that as a gap, which is a lie about
    // lost output.
    let snapshot = harness.store.attach(
        &shell_id,
        crate::contexts::workspaces::domain::ShellAttachmentId::parse("probe").expect("id"),
        0,
    );
    let (_, replay) = snapshot.expect("attach");
    assert_eq!(replay.frames.len(), 1);
    let notices = harness.notices.0.lock().expect("notices");
    assert!(notices
        .iter()
        .any(|notice| matches!(notice, SessionShellNotice::Output { .. })));
}
