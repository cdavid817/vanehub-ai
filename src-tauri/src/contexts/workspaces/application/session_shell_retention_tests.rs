//! What a tab switch and a session switch both reduce to at the registry level.
//!
//! Separate from `session_shell_tests.rs` because these two are the claim the desktop journey
//! checks, stated where it can be checked deterministically: a view that leaves and comes back
//! finds the same Shell, still running, holding everything that happened while it was gone.

use super::evidence::{
    WorkspaceEvidenceSignal, WorkspaceShellCloseReason, WorkspaceShellRuntimeKind,
};
use super::session_shell::{
    AttachSessionShellRequest, CreateSessionShellRequest, ShellAttachmentScope,
};
use super::session_shell_tests::{create, harness, SESSION};
use crate::contexts::workspaces::domain::SessionShellState;

/// If detaching lost the frames that arrived after it, the Shell would still be alive and the user
/// would still have lost the build output they went away to wait for.
#[test]
fn a_detached_shell_keeps_recording_and_replays_what_the_view_missed() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    let attachment = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: 0,
        })
        .expect("attach");
    harness.runtime.emit(&shell_id, "building");

    harness
        .registry
        .detach(&ShellAttachmentScope {
            shell_id: shell_id.clone(),
            attachment_id: attachment.attachment_id,
        })
        .expect("detach");
    harness.runtime.emit(&shell_id, " ... done\n");

    let returned = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: shell_id.clone(),
            after_sequence: attachment.next_sequence - 1,
        })
        .expect("reattach");

    assert_eq!(returned.descriptor.shell_id, shell_id);
    assert_eq!(returned.descriptor.state, SessionShellState::Running);
    assert_eq!(
        returned
            .replay
            .iter()
            .map(|frame| frame.data.as_str())
            .collect::<Vec<_>>(),
        vec!["building", " ... done\n"]
    );
    // Nothing was closed on the way out and nothing was opened on the way back.
    assert_eq!(harness.runtime.opened.lock().expect("opened").len(), 1);
    assert!(harness.runtime.closed.lock().expect("closed").is_empty());
}

/// Switching sessions changes what a view is looking at, not what the registry is holding.
#[test]
fn work_in_one_session_never_disturbs_another_sessions_shells() {
    let harness = harness();
    let kept = create(&harness, None).expect("create");
    harness.runtime.emit(&kept, "long build");

    let other = harness
        .registry
        .create(&CreateSessionShellRequest {
            session_id: "session-other".to_string(),
            seat_id: None,
            rows: 24,
            cols: 80,
            request_id: None,
            title: None,
            working_directory: None,
        })
        .expect("other session shell")
        .shell_id;
    harness.registry.close(&other).expect("close the other one");

    assert_eq!(harness.registry.live_count(SESSION), 1);
    assert_eq!(harness.registry.live_count("session-other"), 0);
    let returned = harness
        .registry
        .attach(&AttachSessionShellRequest {
            shell_id: kept.clone(),
            after_sequence: 0,
        })
        .expect("reattach");
    assert_eq!(returned.replay[0].data, "long build");
    assert_eq!(
        harness.runtime.closed.lock().expect("closed").as_slice(),
        [other]
    );
}

/// The console counts Shells, and the retained registry is now what opens them.
///
/// Without this the Shell figures would go quiet the moment the tab stopped using the older
/// one-view service — and a reader would see "this session opened no shells" rather than a missing
/// wire, which is the exact false claim this capability exists to remove.
#[test]
fn opening_and_closing_a_retained_shell_reaches_the_evidence_journal() {
    let harness = harness();
    let shell_id = create(&harness, None).expect("create");
    harness.registry.close(&shell_id).expect("close");

    let signals = harness.evidence.0.lock().expect("evidence");
    assert!(matches!(
        signals.first(),
        Some(WorkspaceEvidenceSignal::ShellOpened {
            runtime: WorkspaceShellRuntimeKind::Local,
            ..
        })
    ));
    assert!(matches!(
        signals.get(1),
        Some(WorkspaceEvidenceSignal::ShellClosed {
            reason: WorkspaceShellCloseReason::ExplicitClose,
            ..
        })
    ));
    assert_eq!(signals.len(), 2);
}

/// Reclaimed, shut down, and closed by the user are three different facts about a session, and a
/// reader groups by them. Nothing downstream could recover the difference from "the Shell is gone".
#[test]
fn a_reclaimed_shell_is_reported_as_reclaimed_rather_than_as_a_user_close() {
    let harness = harness();
    let idle = create(&harness, None).expect("create");
    harness.advance_past_the_idle_window();

    assert_eq!(harness.registry.sweep_idle(), vec![idle]);

    let signals = harness.evidence.0.lock().expect("evidence");
    assert!(matches!(
        signals.get(1),
        Some(WorkspaceEvidenceSignal::ShellClosed {
            reason: WorkspaceShellCloseReason::IdleCleanup,
            ..
        })
    ));
}
