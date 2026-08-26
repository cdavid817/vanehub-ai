//! What a poll may and may not conclude, and what a notice is allowed to carry.
//!
//! The poller is driven through a provider that answers from a script, because the interesting
//! cases are transitions — readable to gone, gone to readable, readable to unreadable — and a real
//! filesystem can only be pushed through those by racing it.

use super::workspace_invalidation::{
    SystemWorkspaceChangeObserver, TauriInvalidationNotice, WorkspaceInvalidationPoller,
};
use crate::contexts::workspaces::application::{
    CapabilityState, DirectoryFingerprint, DirectoryFingerprintState, DirectoryListing,
    DocumentListing, FileContent, FileSearchListing, GitDiffRequest, GitDiffResult,
    GitStatusResult, ListDirectoryRequest, LocalWorkspaceTarget, ReadTextFileRequest, WatchMode,
    WorkspaceChangeObserverPort, WorkspaceInspectionCapabilities, WorkspaceInspectionError,
    WorkspaceInspectionProvider, WorkspaceInspectionRouter, WorkspaceInvalidationChange,
    WorkspaceInvalidationDispatcher, WorkspaceInvalidationNotice, WorkspaceInvalidationPublisher,
    WorkspaceInvalidationScope, WorkspaceInvalidationSource, WorkspacePathSearchRequest,
    WorkspacePathSearchResult, WorkspaceSearchRequest, WorkspaceTarget, WorkspaceTargetResolver,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const WINDOW: u64 = 250;

#[derive(Default)]
struct RecordingPublisher {
    notices: Mutex<Vec<WorkspaceInvalidationNotice>>,
}

impl RecordingPublisher {
    fn taken(&self) -> Vec<WorkspaceInvalidationNotice> {
        std::mem::take(&mut *self.notices.lock().expect("notices"))
    }
}

impl WorkspaceInvalidationPublisher for RecordingPublisher {
    fn publish(&self, notice: &WorkspaceInvalidationNotice) {
        self.notices.lock().expect("notices").push(notice.clone());
    }
}

struct LocalResolver;

impl WorkspaceTargetResolver for LocalResolver {
    fn resolve(&self, session_id: &str) -> Result<WorkspaceTarget, WorkspaceInspectionError> {
        Ok(WorkspaceTarget::Local(LocalWorkspaceTarget {
            session_id: session_id.to_string(),
            root: PathBuf::from("/workspace"),
        }))
    }
}

/// Answers fingerprints from a queue and refuses everything else.
///
/// Everything else is genuinely unreachable here: a poll asks one question, and a provider that
/// quietly answered the others would let a test pass while the poller called something it must not.
struct ScriptedProvider {
    answers: Mutex<Vec<Result<Vec<DirectoryFingerprint>, WorkspaceInspectionError>>>,
    asked: Mutex<Vec<Vec<String>>>,
}

impl ScriptedProvider {
    fn new(answers: Vec<Result<Vec<DirectoryFingerprint>, WorkspaceInspectionError>>) -> Self {
        Self {
            answers: Mutex::new(answers.into_iter().rev().collect()),
            asked: Mutex::new(Vec::new()),
        }
    }
}

fn known(relative_path: &str, digest: &str) -> DirectoryFingerprint {
    DirectoryFingerprint {
        relative_path: relative_path.to_string(),
        state: DirectoryFingerprintState::Known(digest.to_string()),
    }
}

fn state(relative_path: &str, state: DirectoryFingerprintState) -> DirectoryFingerprint {
    DirectoryFingerprint {
        relative_path: relative_path.to_string(),
        state,
    }
}

#[async_trait::async_trait]
impl WorkspaceInspectionProvider for ScriptedProvider {
    async fn capabilities(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<WorkspaceInspectionCapabilities, WorkspaceInspectionError> {
        Ok(WorkspaceInspectionCapabilities {
            provider: target.provider(),
            list_files: CapabilityState::available(),
            read_text_files: CapabilityState::available(),
            search_files: CapabilityState::available(),
            git_status: CapabilityState::available(),
            git_diff: CapabilityState::available(),
            watch_mode: WatchMode::Polling,
        })
    }

    async fn directory_fingerprints(
        &self,
        _target: &WorkspaceTarget,
        paths: &[String],
    ) -> Result<Vec<DirectoryFingerprint>, WorkspaceInspectionError> {
        self.asked.lock().expect("asked").push(paths.to_vec());
        self.answers
            .lock()
            .expect("answers")
            .pop()
            .unwrap_or_else(|| Ok(Vec::new()))
    }

    async fn search_paths(
        &self,
        _target: &WorkspaceTarget,
        _request: WorkspacePathSearchRequest,
    ) -> Result<WorkspacePathSearchResult, WorkspaceInspectionError> {
        panic!("a poll must not search")
    }

    async fn list_directory(
        &self,
        _target: &WorkspaceTarget,
        _request: ListDirectoryRequest,
    ) -> Result<DirectoryListing, WorkspaceInspectionError> {
        panic!("a poll must not list a directory: the whole point is to avoid enumerating one")
    }

    async fn list_documents(
        &self,
        _target: &WorkspaceTarget,
    ) -> Result<DocumentListing, WorkspaceInspectionError> {
        panic!("a poll must not read documents")
    }

    async fn read_text_file(
        &self,
        _target: &WorkspaceTarget,
        _request: ReadTextFileRequest,
    ) -> Result<FileContent, WorkspaceInspectionError> {
        panic!("a poll must not read a file")
    }

    async fn search(
        &self,
        _target: &WorkspaceTarget,
        _request: WorkspaceSearchRequest,
    ) -> Result<FileSearchListing, WorkspaceInspectionError> {
        panic!("a poll must not search")
    }

    async fn git_status(
        &self,
        _target: &WorkspaceTarget,
    ) -> Result<GitStatusResult, WorkspaceInspectionError> {
        panic!("a poll must not run git")
    }

    async fn git_diff(
        &self,
        _target: &WorkspaceTarget,
        _request: GitDiffRequest,
    ) -> Result<GitDiffResult, WorkspaceInspectionError> {
        panic!("a poll must not run git")
    }
}

struct Harness {
    publisher: Arc<RecordingPublisher>,
    dispatcher: Arc<WorkspaceInvalidationDispatcher>,
    provider: Arc<ScriptedProvider>,
    poller: WorkspaceInvalidationPoller,
}

fn harness(answers: Vec<Result<Vec<DirectoryFingerprint>, WorkspaceInspectionError>>) -> Harness {
    let publisher = Arc::new(RecordingPublisher::default());
    let dispatcher = Arc::new(WorkspaceInvalidationDispatcher::new(publisher.clone()));
    let provider = Arc::new(ScriptedProvider::new(answers));
    let router = Arc::new(WorkspaceInspectionRouter::new(
        Arc::new(LocalResolver),
        provider.clone(),
    ));
    let poller = WorkspaceInvalidationPoller::new(router, dispatcher.clone());
    Harness {
        publisher,
        dispatcher,
        provider,
        poller,
    }
}

#[tokio::test]
async fn the_first_look_at_a_directory_announces_nothing() {
    let harness = harness(vec![Ok(vec![known("src", "1")])]);
    harness.dispatcher.note_directory_read("session", "src", 0);

    assert_eq!(harness.poller.poll_session("session", 1_000).await, 0);
    harness.dispatcher.flush_due(1_000 + WINDOW);

    // The console read this directory to get here, so it already holds the current contents.
    // A notice would send it straight back for what it has.
    assert!(harness.publisher.taken().is_empty());
}

#[tokio::test]
async fn a_changed_fingerprint_invalidates_that_directory_and_no_other() {
    let harness = harness(vec![
        Ok(vec![known("src", "1"), known("docs", "9")]),
        Ok(vec![known("src", "2"), known("docs", "9")]),
    ]);
    harness.dispatcher.note_directory_read("session", "src", 0);
    harness.dispatcher.note_directory_read("session", "docs", 0);

    harness.poller.poll_session("session", 1_000).await;
    assert_eq!(harness.poller.poll_session("session", 2_000).await, 1);
    harness.dispatcher.flush_due(2_000 + WINDOW);

    let notices = harness.publisher.taken();
    assert_eq!(notices.len(), 1);
    assert_eq!(
        notices[0].scope,
        WorkspaceInvalidationScope::Directory {
            relative_path: "src".to_string()
        }
    );
    assert_eq!(notices[0].source, WorkspaceInvalidationSource::Poll);
}

#[tokio::test]
async fn a_directory_that_went_away_is_reported_as_a_removed_path() {
    let harness = harness(vec![
        Ok(vec![known("src/deep", "1")]),
        Ok(vec![state("src/deep", DirectoryFingerprintState::Missing)]),
    ]);
    harness
        .dispatcher
        .note_directory_read("session", "src/deep", 0);

    harness.poller.poll_session("session", 1_000).await;
    harness.poller.poll_session("session", 2_000).await;
    harness.dispatcher.flush_due(2_000 + WINDOW);

    // `Path`, not `Directory`: it is the parent's listing that is now wrong, and telling the
    // console to refetch a directory that is gone would ask it to refresh nothing.
    assert_eq!(
        harness.publisher.taken()[0].scope,
        WorkspaceInvalidationScope::Path {
            relative_path: "src/deep".to_string(),
            change: WorkspaceInvalidationChange::Removed,
        }
    );
}

#[tokio::test]
async fn an_unreadable_directory_is_not_a_deleted_one() {
    let harness = harness(vec![
        Ok(vec![known("src", "1")]),
        Ok(vec![state("src", DirectoryFingerprintState::Unreadable)]),
        Ok(vec![known("src", "1")]),
    ]);
    harness.dispatcher.note_directory_read("session", "src", 0);

    harness.poller.poll_session("session", 1_000).await;
    assert_eq!(harness.poller.poll_session("session", 2_000).await, 0);
    // Recovering to the same value must also be silent. If the unreadable pass had been
    // remembered, the comparison after it would differ from "could not read it" and announce a
    // change that never happened.
    assert_eq!(harness.poller.poll_session("session", 3_000).await, 0);
    harness.dispatcher.flush_due(3_000 + WINDOW);

    assert!(harness.publisher.taken().is_empty());
}

#[tokio::test]
async fn a_provider_that_cannot_answer_reports_no_change_and_keeps_what_it_knew() {
    let harness = harness(vec![
        Ok(vec![known("src", "1")]),
        Err(WorkspaceInspectionError::Timeout),
        Ok(vec![known("src", "2")]),
    ]);
    harness.dispatcher.note_directory_read("session", "src", 0);

    harness.poller.poll_session("session", 1_000).await;
    assert_eq!(harness.poller.poll_session("session", 2_000).await, 0);
    // The change during the outage is still found afterwards, because the failure did not clear
    // what the last successful poll saw.
    assert_eq!(harness.poller.poll_session("session", 3_000).await, 1);
}

#[tokio::test]
async fn a_session_with_nothing_open_never_reaches_the_provider() {
    let harness = harness(vec![Ok(vec![known("src", "1")])]);

    assert_eq!(harness.poller.poll_observed(1_000).await, 0);

    // Not one round trip. On a remote workspace this is the difference between a hidden panel
    // costing nothing and it costing an SSH channel every few seconds, forever.
    assert!(harness.provider.asked.lock().expect("asked").is_empty());
}

#[tokio::test]
async fn a_poll_asks_only_about_directories_somebody_is_looking_at() {
    let harness = harness(vec![Ok(Vec::new())]);
    harness.dispatcher.note_directory_read("session", "src", 0);
    harness.dispatcher.note_directory_read("session", "docs", 0);

    harness.poller.poll_session("session", 1_000).await;

    let asked = harness.provider.asked.lock().expect("asked");
    let mut paths = asked[0].clone();
    paths.sort();
    assert_eq!(paths, vec!["docs".to_string(), "src".to_string()]);
}

#[test]
fn the_change_observer_reaches_the_dispatcher_without_a_clock_of_its_own() {
    let publisher = Arc::new(RecordingPublisher::default());
    let dispatcher = Arc::new(WorkspaceInvalidationDispatcher::new(publisher.clone()));
    let observer = SystemWorkspaceChangeObserver::new(dispatcher.clone());

    observer.observe(
        "session",
        WorkspaceInvalidationSource::ExecutionEvidence,
        WorkspaceInvalidationScope::Path {
            relative_path: "src/main.rs".to_string(),
            change: WorkspaceInvalidationChange::Modified,
        },
    );

    // Far enough past any plausible wall clock that the window has certainly closed. The observer
    // stamps system time, so the assertion has to be about the notice arriving, not about when.
    dispatcher.flush_due(u64::MAX);
    let notices = publisher.taken();
    assert_eq!(notices.len(), 1);
    assert_eq!(
        notices[0].source,
        WorkspaceInvalidationSource::ExecutionEvidence
    );
}

#[test]
fn a_notice_on_the_wire_carries_relative_paths_and_camel_case_keys() {
    let payload = TauriInvalidationNotice::from(&WorkspaceInvalidationNotice {
        session_id: "session-1".to_string(),
        source: WorkspaceInvalidationSource::ExecutionEvidence,
        scope: WorkspaceInvalidationScope::Path {
            relative_path: "src/main.rs".to_string(),
            change: WorkspaceInvalidationChange::Modified,
        },
        observed_at: "2026-08-26T09:00:00Z".to_string(),
        sequence: 4,
        coalesced: Some(2),
    });

    assert_eq!(
        serde_json::to_value(payload).expect("payload"),
        serde_json::json!({
            "sessionId": "session-1",
            "source": "execution-evidence",
            "scope": "path",
            "relativePath": "src/main.rs",
            "change": "modified",
            "sequence": 4,
            "occurredAt": "2026-08-26T09:00:00Z",
            "coalesced": 2,
        })
    );
}

#[test]
fn a_workspace_notice_omits_the_fields_it_has_no_answer_for() {
    let payload = TauriInvalidationNotice::from(&WorkspaceInvalidationNotice {
        session_id: "session-1".to_string(),
        source: WorkspaceInvalidationSource::Poll,
        scope: WorkspaceInvalidationScope::Workspace,
        observed_at: "2026-08-26T09:00:00Z".to_string(),
        sequence: 1,
        coalesced: None,
    });

    // Omitted rather than null: a path of `null` and a change of `null` are two fields a reader
    // has to learn to ignore, and one of them would eventually be read as a path.
    assert_eq!(
        serde_json::to_value(payload).expect("payload"),
        serde_json::json!({
            "sessionId": "session-1",
            "source": "poll",
            "scope": "workspace",
            "sequence": 1,
            "occurredAt": "2026-08-26T09:00:00Z",
        })
    );
}
