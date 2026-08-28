//! What the dispatcher promises, stated as the cases that would break a console.
//!
//! Every case here drives time as a parameter rather than sleeping. A coalescing window verified by
//! waiting for it is a test that is slow when it passes and flaky when the machine is loaded, and
//! neither tells you anything the explicit clock does not.

use super::invalidation::{
    WorkspaceInvalidationChange, WorkspaceInvalidationNotice, WorkspaceInvalidationPublisher,
    WorkspaceInvalidationScope, WorkspaceInvalidationSource,
};
use super::invalidation_dispatcher::{
    WorkspaceInvalidationDispatcher, COALESCE_WINDOW_MS, MAX_OBSERVED_DIRECTORIES,
    MAX_PENDING_SCOPES, OBSERVATION_TTL_MS,
};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct RecordingPublisher {
    notices: Mutex<Vec<WorkspaceInvalidationNotice>>,
}

impl RecordingPublisher {
    fn taken(&self) -> Vec<WorkspaceInvalidationNotice> {
        std::mem::take(&mut *self.notices.lock().expect("publisher lock"))
    }
}

impl WorkspaceInvalidationPublisher for RecordingPublisher {
    fn publish(&self, notice: &WorkspaceInvalidationNotice) {
        self.notices
            .lock()
            .expect("publisher lock")
            .push(notice.clone());
    }
}

fn dispatcher() -> (Arc<RecordingPublisher>, WorkspaceInvalidationDispatcher) {
    let publisher = Arc::new(RecordingPublisher::default());
    let dispatcher = WorkspaceInvalidationDispatcher::new(publisher.clone());
    (publisher, dispatcher)
}

fn path(relative_path: &str, change: WorkspaceInvalidationChange) -> WorkspaceInvalidationScope {
    WorkspaceInvalidationScope::Path {
        relative_path: relative_path.to_string(),
        change,
    }
}

#[test]
fn nothing_is_published_before_the_window_closes() {
    let (publisher, dispatcher) = dispatcher();

    dispatcher.observe(
        "session",
        WorkspaceInvalidationSource::Watch,
        path("src/main.rs", WorkspaceInvalidationChange::Modified),
        1_000,
    );

    // The whole point of buffering: a write that is still being followed by more writes has not
    // finished being a change yet.
    assert_eq!(dispatcher.flush_due(1_000 + COALESCE_WINDOW_MS - 1), 0);
    assert!(publisher.taken().is_empty());

    assert_eq!(dispatcher.flush_due(1_000 + COALESCE_WINDOW_MS), 1);
    let notices = publisher.taken();
    assert_eq!(notices.len(), 1);
    assert_eq!(
        notices[0].scope,
        path("src/main.rs", WorkspaceInvalidationChange::Modified)
    );
    assert_eq!(notices[0].sequence, 1);
    // Absent, not zero: this notice stands for exactly the one observation it describes.
    assert_eq!(notices[0].coalesced, None);
}

#[test]
fn separate_paths_stay_separate() {
    let (publisher, dispatcher) = dispatcher();

    for name in ["src/a.rs", "src/b.rs", "docs/c.md"] {
        dispatcher.observe(
            "session",
            WorkspaceInvalidationSource::Watch,
            path(name, WorkspaceInvalidationChange::Modified),
            1_000,
        );
    }
    dispatcher.flush_due(1_000 + COALESCE_WINDOW_MS);

    // Coalescing must not become "one notice per burst". Targeted invalidation is the feature; a
    // burst that collapsed by default would spend it.
    let mut named: Vec<String> = publisher
        .taken()
        .into_iter()
        .filter_map(|notice| notice.scope.relative_path().map(str::to_string))
        .collect();
    named.sort();
    assert_eq!(named, vec!["docs/c.md", "src/a.rs", "src/b.rs"]);
}

#[test]
fn repeated_writes_to_one_file_become_one_notice_that_counts_them() {
    let (publisher, dispatcher) = dispatcher();

    for offset in 0..5 {
        dispatcher.observe(
            "session",
            WorkspaceInvalidationSource::Watch,
            path("src/main.rs", WorkspaceInvalidationChange::Modified),
            1_000 + offset,
        );
    }
    dispatcher.flush_due(1_000 + COALESCE_WINDOW_MS);

    let notices = publisher.taken();
    assert_eq!(notices.len(), 1);
    // Four beyond the one the notice is. A reader who sees `4` knows the file was rewritten
    // repeatedly rather than touched once.
    assert_eq!(notices[0].coalesced, Some(4));
    // The earliest observation, because the question is how long the view has been wrong.
    assert!(notices[0].observed_at.contains("1970-01-01T00:00:01"));
}

#[test]
fn two_different_changes_to_one_path_report_neither() {
    let (publisher, dispatcher) = dispatcher();

    dispatcher.observe(
        "session",
        WorkspaceInvalidationSource::Watch,
        path("src/main.rs", WorkspaceInvalidationChange::Created),
        1_000,
    );
    dispatcher.observe(
        "session",
        WorkspaceInvalidationSource::Watch,
        path("src/main.rs", WorkspaceInvalidationChange::Removed),
        1_010,
    );
    dispatcher.flush_due(1_000 + COALESCE_WINDOW_MS);

    // Naming either would assert an order nobody observed. `Unknown` is also what the consumer
    // needs: it refreshes the entry and its parent, which is right whichever happened.
    assert_eq!(
        publisher.taken()[0].scope,
        path("src/main.rs", WorkspaceInvalidationChange::Unknown)
    );
}

#[test]
fn a_burst_past_the_bound_collapses_instead_of_dropping() {
    let (publisher, dispatcher) = dispatcher();

    let total = MAX_PENDING_SCOPES + 20;
    for index in 0..total {
        dispatcher.observe(
            "session",
            WorkspaceInvalidationSource::Watch,
            path(
                &format!("src/file-{index}.rs"),
                WorkspaceInvalidationChange::Modified,
            ),
            1_000,
        );
    }
    dispatcher.flush_due(1_000 + COALESCE_WINDOW_MS);

    let notices = publisher.taken();
    // One notice, not eighty-four, and not sixty-four with twenty silently gone. A dropped
    // observation leaves the console showing content it cannot know is stale, which looks exactly
    // like a workspace where nothing happened.
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].scope, WorkspaceInvalidationScope::Workspace);
    assert_eq!(notices[0].coalesced, Some(total as u32 - 1));
}

#[test]
fn a_workspace_notice_swallows_the_specific_ones_it_already_covers() {
    let (publisher, dispatcher) = dispatcher();

    dispatcher.observe(
        "session",
        WorkspaceInvalidationSource::Watch,
        path("src/main.rs", WorkspaceInvalidationChange::Modified),
        1_000,
    );
    dispatcher.observe(
        "session",
        WorkspaceInvalidationSource::Poll,
        WorkspaceInvalidationScope::Workspace,
        1_010,
    );
    dispatcher.flush_due(1_000 + COALESCE_WINDOW_MS);

    let notices = publisher.taken();
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].scope, WorkspaceInvalidationScope::Workspace);
}

#[test]
fn a_coalesced_notice_claims_only_what_its_weakest_source_saw() {
    let (publisher, dispatcher) = dispatcher();

    dispatcher.observe(
        "session",
        WorkspaceInvalidationSource::ExecutionEvidence,
        path("src/main.rs", WorkspaceInvalidationChange::Modified),
        1_000,
    );
    dispatcher.observe(
        "session",
        WorkspaceInvalidationSource::Poll,
        path("src/main.rs", WorkspaceInvalidationChange::Modified),
        1_010,
    );
    dispatcher.flush_due(1_000 + COALESCE_WINDOW_MS);

    // A poll's answer is true as of the poll. Folding it in with an exact observation and calling
    // the result exact would overstate the half that matters.
    assert_eq!(
        publisher.taken()[0].source,
        WorkspaceInvalidationSource::Poll
    );
}

#[test]
fn sequences_are_per_session_and_never_go_backwards() {
    let (publisher, dispatcher) = dispatcher();

    for round in 0..3u64 {
        for session in ["alpha", "beta"] {
            dispatcher.observe(
                session,
                WorkspaceInvalidationSource::Watch,
                path("src/main.rs", WorkspaceInvalidationChange::Modified),
                1_000 + round * 1_000,
            );
        }
        dispatcher.flush_due(1_000 + round * 1_000 + COALESCE_WINDOW_MS);
    }

    let notices = publisher.taken();
    let alpha: Vec<u64> = notices
        .iter()
        .filter(|notice| notice.session_id == "alpha")
        .map(|notice| notice.sequence)
        .collect();
    let beta: Vec<u64> = notices
        .iter()
        .filter(|notice| notice.session_id == "beta")
        .map(|notice| notice.sequence)
        .collect();

    // Per session, so one busy workspace cannot make another look like it lost notices.
    assert_eq!(alpha, vec![1, 2, 3]);
    assert_eq!(beta, vec![1, 2, 3]);
}

#[test]
fn a_sequence_survives_a_session_going_quiet() {
    let (publisher, dispatcher) = dispatcher();

    dispatcher.observe(
        "session",
        WorkspaceInvalidationSource::Watch,
        path("src/main.rs", WorkspaceInvalidationChange::Modified),
        1_000,
    );
    dispatcher.flush_due(1_000 + COALESCE_WINDOW_MS);
    publisher.taken();

    // Everything expires; the counter must not.
    dispatcher.expire(1_000 + OBSERVATION_TTL_MS * 10);
    dispatcher.observe(
        "session",
        WorkspaceInvalidationSource::Watch,
        path("src/main.rs", WorkspaceInvalidationChange::Modified),
        999_000,
    );
    dispatcher.flush_due(999_000 + COALESCE_WINDOW_MS);

    // A restarted counter is indistinguishable from a replay to anyone watching for gaps.
    assert_eq!(publisher.taken()[0].sequence, 2);
}

#[test]
fn a_directory_stops_being_observed_when_nobody_reads_it() {
    let (_publisher, dispatcher) = dispatcher();

    dispatcher.note_directory_read("session", "src", 1_000);
    dispatcher.note_directory_read("session", "docs", 1_500);

    assert_eq!(
        dispatcher.observed_directories("session", 2_000),
        vec!["docs".to_string(), "src".to_string()],
        "freshest read first"
    );

    // No unregister call. A console that crashed or was hidden never sends one, and those are
    // exactly the clients whose watches would otherwise never be released.
    assert_eq!(
        dispatcher.observed_directories("session", 1_000 + OBSERVATION_TTL_MS),
        vec!["docs".to_string()],
        "the older read has aged out, the newer one has not"
    );
    assert!(dispatcher
        .observed_directories("session", 1_500 + OBSERVATION_TTL_MS)
        .is_empty());
}

#[test]
fn observing_more_directories_than_the_bound_drops_the_coldest() {
    let (_publisher, dispatcher) = dispatcher();

    for index in 0..MAX_OBSERVED_DIRECTORIES {
        dispatcher.note_directory_read("session", &format!("dir-{index}"), 1_000 + index as u64);
    }
    dispatcher.note_directory_read("session", "the-one-on-screen", 9_000);

    let observed = dispatcher.observed_directories("session", 9_100);
    assert_eq!(observed.len(), MAX_OBSERVED_DIRECTORIES);
    // The newest read is the directory the user is looking at. Refusing it to protect the bound
    // would stop watching precisely there.
    assert_eq!(observed[0], "the-one-on-screen");
    assert!(!observed.contains(&"dir-0".to_string()));
}

#[test]
fn a_session_with_nothing_open_is_not_worth_polling() {
    let (_publisher, dispatcher) = dispatcher();

    dispatcher.note_directory_read("session", "src", 1_000);
    assert_eq!(
        dispatcher.observed_sessions(1_100),
        vec!["session".to_string()]
    );

    // Expiry is what stops the driver on its own: a hidden tab stops refreshing its reads, the
    // reads age out, and polling ends without anyone having to send a visibility signal.
    assert!(dispatcher
        .observed_sessions(1_000 + OBSERVATION_TTL_MS)
        .is_empty());
    assert_eq!(dispatcher.expire(1_000 + OBSERVATION_TTL_MS), 0);
}
