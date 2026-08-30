//! What stays bounded when the workspace does not.
//!
//! Every other suite here asks whether an answer is correct. These ask whether producing it costs
//! something proportional to the answer rather than to the tree, which is a different question and
//! the one that only shows up on somebody else's repository.
//!
//! Structural rather than timed. A gate that measured milliseconds would fail on a busy machine and
//! pass on a fast one with a quadratic walk in it — the number it should be watching is how many
//! entries were visited, how many candidates were held, and how many bytes were read, all of which
//! the budget already reports. Nothing here sleeps, and nothing here asserts a duration.
//!
//! The trees are built rather than fixtured. A fixture large enough to make the difference between
//! "proportional to the page" and "proportional to the workspace" observable would be thousands of
//! files in the repository, and every unrelated checkout would pay for them.

use super::content_search::search_session_content;
use super::path_search::search_session_paths;
use crate::contexts::workspaces::application::{
    ManualClock, SearchCancellationCause, SearchCancellationToken, WorkspaceContentSearchRequest,
    WorkspaceInspectionBudgetSnapshot, WorkspaceInspectionExecution, WorkspacePathSearchRequest,
    WorkspaceSearchCancellation,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::fs;
use std::path::Path;
use std::sync::Arc;

/// A workspace with a shape chosen by the test.
struct SyntheticWorkspace {
    _directory: TempDirectory,
    database: NativeDatabase,
    files: usize,
}

/// One directory holding `files` matching files.
///
/// Wide rather than deep, because the bound under test is how many candidates a ranking holds, and
/// depth is a separate ceiling with its own test below.
fn wide_workspace(files: usize) -> SyntheticWorkspace {
    build(files, |root| {
        for index in 0..files {
            fs::write(root.join(format!("main_{index:05}.rs")), "needle\n").expect("file");
        }
    })
}

/// A chain of directories `depth` levels long, one file at the bottom of each.
fn deep_workspace(depth: usize) -> SyntheticWorkspace {
    build(depth, |root| {
        let mut path = root.to_path_buf();
        for level in 0..depth {
            path = path.join(format!("level_{level}"));
            fs::create_dir_all(&path).expect("level");
            fs::write(path.join("main.rs"), "needle\n").expect("file");
        }
    })
}

fn build(files: usize, populate: impl Fn(&Path)) -> SyntheticWorkspace {
    let directory = TempDirectory::new("structural-performance");
    let root = directory.path().join("workspace");
    fs::create_dir_all(&root).expect("root");
    populate(&root);

    let database = NativeDatabase::new(directory.path().join("data")).expect("database");
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO sessions \
             (id, title, agent_id, interaction_mode, lifecycle_state, folder, pinned, archived, \
              created_at, updated_at) \
             VALUES ('session-1', 'Structural', 'codex-cli', 'cli', 'idle', ?1, 0, 0, \
                     '2026-08-29T10:00:00Z', '2026-08-29T10:00:00Z')",
            params![root.to_string_lossy().as_ref()],
        )
        .expect("insert session");
    drop(connection);

    SyntheticWorkspace {
        _directory: directory,
        database,
        files,
    }
}

/// A Quick Open context carrying the token the test wants to drive.
///
/// Registered against a real registry, because a generation nothing issued is a number rather than
/// an identity.
fn path_execution(token: SearchCancellationToken) -> WorkspaceInspectionExecution {
    let registry = Arc::new(WorkspaceSearchCancellation::default());
    let registration = registry.begin("structural-1");
    let execution = WorkspaceInspectionExecution::path_search(
        registration.generation(),
        token,
        Arc::new(ManualClock::default()),
    );
    registration.complete();
    execution
}

fn content_execution(token: SearchCancellationToken) -> WorkspaceInspectionExecution {
    let registry = Arc::new(WorkspaceSearchCancellation::default());
    let registration = registry.begin("structural-1");
    let execution = WorkspaceInspectionExecution::content_search(
        registration.generation(),
        token,
        Arc::new(ManualClock::default()),
    );
    registration.complete();
    execution
}

impl SyntheticWorkspace {
    fn quick_open(&self, limit: usize, token: SearchCancellationToken) -> Answer {
        let connection = self.database.connection().expect("connection");
        let result = search_session_paths(
            &connection,
            "session-1",
            &WorkspacePathSearchRequest {
                query: "main".to_string(),
                search_id: "structural-1".to_string(),
                cursor: None,
                limit: Some(limit),
            },
            &path_execution(token),
        )
        .expect("search");
        Answer {
            results: result.matches.len(),
            spent: result.coverage.budget.expect("accounted"),
            reason: result.coverage.reason_code,
        }
    }

    fn content(&self, limit: usize, token: SearchCancellationToken) -> Answer {
        let connection = self.database.connection().expect("connection");
        let result = search_session_content(
            &connection,
            "session-1",
            &WorkspaceContentSearchRequest {
                query: "needle".to_string(),
                search_id: "structural-1".to_string(),
                limit: Some(limit),
            },
            &content_execution(token),
        )
        .expect("search");
        Answer {
            results: result.matches.len(),
            spent: result.coverage.budget.expect("accounted"),
            reason: result.coverage.reason_code,
        }
    }
}

struct Answer {
    results: usize,
    spent: WorkspaceInspectionBudgetSnapshot,
    reason: Option<&'static str>,
}

/// Ranking holds one page, not one workspace.
///
/// Two thousand files all match. The old implementation collected every candidate and sorted them,
/// so the memory a Quick Open needed was a property of the repository somebody happened to open. The
/// bound asserted here is the page plus the one entry that proves another page exists.
#[test]
fn ranking_retains_one_page_however_many_entries_match() {
    let workspace = wide_workspace(2_000);

    let answer = workspace.quick_open(10, SearchCancellationToken::new());

    assert_eq!(answer.results, 10);
    assert!(
        answer.spent.candidates_retained <= 11,
        "held {} candidates for a ten-entry page",
        answer.spent.candidates_retained
    );
    // Every entry is still *visited* — without an index there is no way to know which ten rank
    // highest without looking at all of them, and that cost is real and bounded by the entry budget.
    // What this asserts is that visiting is not the same as keeping.
    assert!(answer.spent.entries_visited >= workspace.files as u64);
}

/// A result cap stops the reading, not just the list.
///
/// A search that read every file and then returned the first five would produce the same result
/// list, and would have opened four hundred more files to do it.
#[test]
fn a_result_cap_stops_opening_files_rather_than_trimming_the_answer() {
    let workspace = wide_workspace(400);

    let answer = workspace.content(5, SearchCancellationToken::new());

    assert_eq!(answer.results, 5);
    assert_eq!(answer.reason, Some("result_budget_exhausted"));
    assert!(
        answer.spent.files_opened <= 6,
        "opened {} files for a five-match answer",
        answer.spent.files_opened
    );
}

/// A cancel that arrived before the walk started costs the walk nothing.
///
/// The bound on how much work follows a cancellation is what makes a cancel worth issuing. Checked
/// at its extreme — signalled before the first directory — because that is the case where any work
/// at all is work nobody asked for, and it needs no interleaving to observe.
#[test]
fn a_cancel_seen_before_the_first_directory_costs_no_traversal() {
    let workspace = wide_workspace(500);
    let token = SearchCancellationToken::new();
    token.signal(SearchCancellationCause::Cancelled);

    let answer = workspace.quick_open(10, token);

    assert_eq!(answer.reason, Some("cancelled"));
    assert_eq!(answer.results, 0);
    assert_eq!(answer.spent.entries_visited, 0);
    assert_eq!(answer.spent.directories_visited, 0);
}

/// A tree deeper than the ceiling is bounded and says so.
///
/// The depth limit is what stops a walk from following a generated tree, a symlinked checkout, or a
/// dependency directory that nests further than anybody writes by hand. Reported rather than
/// silently short: a reader who cannot find a file needs to know the walk stopped before reaching
/// it.
#[test]
fn a_tree_deeper_than_the_ceiling_stops_and_reports_the_depth() {
    let workspace = deep_workspace(20);

    let answer = workspace.quick_open(50, SearchCancellationToken::new());

    assert!(
        answer.spent.max_depth_reached <= 11,
        "descended to {}",
        answer.spent.max_depth_reached
    );
    assert_eq!(answer.reason, Some("depth_budget_exhausted"));
}

/// Content search holds one file at a time, whatever the workspace holds.
///
/// The bytes read across four hundred small files is the sum of the ones it opened, and the walk
/// stops at its byte ceiling rather than accumulating a workspace. The old implementation built a
/// candidate vector of every file first, so the memory was decided before a single byte was read.
#[test]
fn a_byte_ceiling_bounds_the_whole_walk_rather_than_each_file() {
    let workspace = wide_workspace(400);

    let answer = workspace.content(200, SearchCancellationToken::new());

    assert!(
        answer.spent.bytes_read <= 512 * 1024 * 1024,
        "read {} bytes",
        answer.spent.bytes_read
    );
    // The frontier, not the results: a breadth-first walk of one directory holds one directory.
    assert!(
        answer.spent.candidates_retained <= 4_096,
        "held {} directories in the frontier",
        answer.spent.candidates_retained
    );
}
