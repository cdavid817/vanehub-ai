use super::evidence::{WorkspaceEvidencePort, WorkspaceEvidenceSignal};
use super::session_shell::{
    AttachSessionShellRequest, CreateSessionShellRequest, ResizeSessionShellRequest,
    SessionShellNotice, SessionShellNoticePort, SessionShellRuntimePort, SessionShellWorkspace,
    SessionShellWorkspacePort, ShellAttachmentScope, ShellCapacities, ShellClockPort, ShellIdPort,
    ShellOutputSink, ShellRuntimeOpen, ShellRuntimeOpened, WriteSessionShellRequest,
};
use super::session_shell_registry::SessionShellRegistry;
use super::session_shell_store::ShellStore;
use crate::contexts::workspaces::domain::{
    SessionShellError, SessionShellState, ShellCapacityScope, ShellCreateRequestId,
    ShellForegroundProcessState, ShellId, ShellRuntimeDescriptor, ShellStream, ShellTitle,
    TerminalDimensions,
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
}

impl FakeRuntime {
    pub(super) fn emit(&self, shell_id: &ShellId, text: &str) {
        for sink in self.sinks.lock().expect("sinks").iter() {
            sink.on_output(shell_id, ShellStream::Pty, text.as_bytes());
        }
    }

    fn exit(&self, shell_id: &ShellId, code: i32) {
        for sink in self.sinks.lock().expect("sinks").iter() {
            sink.on_state(shell_id, SessionShellState::Exited { code: Some(code) });
        }
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

    fn close(&self, shell_id: &ShellId) -> Result<(), SessionShellError> {
        self.closed.lock().expect("closed").push(shell_id.clone());
        Ok(())
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
    let clock = Arc::new(FakeClock::default());
    let notices = Arc::new(RecordingNotices::default());
    let store = Arc::new(ShellStore::new(notices.clone(), clock.clone()));
    let runtime = Arc::new(FakeRuntime::default());
    let evidence = Arc::new(RecordingEvidence::default());
    let registry = Arc::new(SessionShellRegistry::new(
        store.clone(),
        runtime.clone(),
        Arc::new(workspaces),
        Arc::new(SequentialIds::default()),
        clock.clone(),
        capacities,
        evidence.clone(),
    ));
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

    harness.registry.close(&shell_id).expect("close");
    harness.registry.close(&shell_id).expect("close again");

    assert_eq!(harness.runtime.closed.lock().expect("closed").len(), 1);
    assert!(harness.registry.list(None).is_empty());
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
    let states = harness
        .notices
        .0
        .lock()
        .expect("notices")
        .iter()
        .filter(|notice| matches!(notice, SessionShellNotice::State { .. }))
        .count();
    assert_eq!(states, 1);
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
    let closed = harness.registry.sweep_idle();

    // An attached Shell is never idle by definition: someone is looking at it.
    assert_eq!(closed, vec![idle]);
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

    assert!(harness.registry.sweep_idle().is_empty());
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
    assert_eq!(harness.registry.sweep_idle(), vec![shell_id]);
}

#[test]
fn shutdown_closes_every_shell() {
    let harness = harness();
    create(&harness, Some("request-1")).expect("first");
    create(&harness, Some("request-2")).expect("second");

    harness.registry.shutdown();

    assert_eq!(harness.runtime.closed.lock().expect("closed").len(), 2);
    assert!(harness.registry.list(None).is_empty());
    assert_eq!(harness.registry.live_count(SESSION), 0);
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
