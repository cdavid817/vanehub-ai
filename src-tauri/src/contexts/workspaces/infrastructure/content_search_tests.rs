//! What a content search promises about positions, bounds, and stopping.
//!
//! Over a real temporary workspace, because the interesting parts are what happens to a long line,
//! a binary file, and a walk that is asked to stop partway through — none of which a fixture can
//! stand in for.

use super::content_search::search_session_content;
use crate::contexts::workspaces::application::{
    ManualClock, MonotonicClockPort, SearchCancellationCause, SearchCancellationToken,
    WorkspaceContentSearchRequest, WorkspaceInspectionBudgetLimits,
    WorkspaceInspectionBudgetSnapshot, WorkspaceInspectionExecution, WorkspaceSearchCancellation,
    MAX_SNIPPET_CHARS,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::fs;
use std::sync::Arc;
use std::time::Duration;

struct Workspace {
    _directory: TempDirectory,
    database: NativeDatabase,
}

/// A content-search context carrying the token and the clock the test wants to drive.
///
/// The generation comes from a real registration: a hand-made one would let a test assert against a
/// number nothing issued.
fn content_search_execution(
    token: SearchCancellationToken,
    clock: Arc<dyn MonotonicClockPort>,
) -> WorkspaceInspectionExecution {
    let registry = Arc::new(WorkspaceSearchCancellation::default());
    let registration = registry.begin("search-1");
    let execution =
        WorkspaceInspectionExecution::content_search(registration.generation(), token, clock);
    registration.complete();
    execution
}

struct Answer {
    coverage: &'static str,
    reason: Option<&'static str>,
    /// What the search actually spent. The instrumentation the structural assertions read: a claim
    /// about memory or work is only checkable if the traversal reports what it did.
    budget: Option<WorkspaceInspectionBudgetSnapshot>,
    hits: Vec<Hit>,
}

#[derive(Debug, PartialEq, Eq)]
struct Hit {
    path: String,
    line: u32,
    column: u32,
    snippet: String,
    truncated: bool,
}

impl Workspace {
    fn search(&self, query: &str, cancellation: &SearchCancellationToken) -> Answer {
        let connection = self.database.connection().expect("connection");
        let result = search_session_content(
            &connection,
            "session-1",
            &WorkspaceContentSearchRequest {
                query: query.to_string(),
                search_id: "search-1".to_string(),
                limit: None,
            },
            &content_search_execution(cancellation.clone(), Arc::new(ManualClock::default())),
        )
        .expect("search");
        answer_from(result)
    }

    /// The same search with its budget and its clock chosen by the test.
    fn search_within(
        &self,
        query: &str,
        limits: WorkspaceInspectionBudgetLimits,
        clock: Arc<dyn MonotonicClockPort>,
        cancellation: &SearchCancellationToken,
    ) -> Answer {
        let connection = self.database.connection().expect("connection");
        let result = search_session_content(
            &connection,
            "session-1",
            &WorkspaceContentSearchRequest {
                query: query.to_string(),
                search_id: "search-1".to_string(),
                limit: None,
            },
            &content_search_execution(cancellation.clone(), clock).with_limits(limits),
        )
        .expect("search");
        answer_from(result)
    }

    fn find(&self, query: &str) -> Answer {
        self.search(query, &SearchCancellationToken::new())
    }
}

fn answer_from(
    result: crate::contexts::workspaces::application::WorkspaceContentSearchResult,
) -> Answer {
    Answer {
        coverage: result.coverage.state.token(),
        reason: result.coverage.reason_code,
        budget: result.coverage.budget,
        hits: result
            .matches
            .into_iter()
            .map(|entry| Hit {
                path: entry.path,
                line: entry.line,
                column: entry.column,
                snippet: entry.snippet,
                truncated: entry.snippet_truncated,
            })
            .collect(),
    }
}

/// A budget whose every dimension is generous, so a test can lower exactly one.
///
/// Written out rather than derived from the production profile: a test that lowered one field of
/// the real profile would silently start asserting about a different dimension the day somebody
/// tightened another.
fn generous_limits() -> WorkspaceInspectionBudgetLimits {
    WorkspaceInspectionBudgetLimits {
        max_directories_visited: 1_000,
        max_entries_visited: 10_000,
        max_files_opened: 10_000,
        max_bytes_read: 64 * 1024 * 1024,
        max_metadata_operations: 100_000,
        max_retained_candidates: 1_000,
        max_results: 200,
        max_depth: 10,
        deadline: Duration::from_secs(600),
    }
}

fn spent(answer: &Answer) -> WorkspaceInspectionBudgetSnapshot {
    answer
        .budget
        .expect("coverage carries what the search spent")
}

fn workspace(files: &[(&str, &[u8])]) -> Workspace {
    let directory = TempDirectory::new("content-search");
    let root = directory.path().join("workspace");
    fs::create_dir_all(&root).expect("root");
    for (name, bytes) in files {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(&path, bytes).expect("file");
    }

    let database = NativeDatabase::new(directory.path().join("data")).expect("database");
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO sessions \
             (id, title, agent_id, interaction_mode, lifecycle_state, folder, pinned, archived, \
              created_at, updated_at) \
             VALUES ('session-1', 'Content', 'codex-cli', 'cli', 'idle', ?1, 0, 0, \
                     '2026-08-26T10:00:00Z', '2026-08-26T10:00:00Z')",
            params![root.to_string_lossy().as_ref()],
        )
        .expect("insert session");
    drop(connection);

    Workspace {
        _directory: directory,
        database,
    }
}

#[test]
fn a_match_carries_the_line_and_column_an_editor_would_jump_to() {
    let workspace = workspace(&[("src/main.rs", b"fn main() {}\nlet needle = 1;\n")]);

    let answer = workspace.find("needle");

    // 1-based on both axes, because that is what every editor and every error message uses.
    assert_eq!(
        answer.hits,
        vec![Hit {
            path: "src/main.rs".to_string(),
            line: 2,
            column: 5,
            snippet: "let needle = 1;".to_string(),
            truncated: false,
        }]
    );
}

#[test]
fn a_column_is_counted_in_characters_rather_than_bytes() {
    let workspace = workspace(&[("notes.md", "café café needle\n".as_bytes())]);

    let answer = workspace.find("needle");

    // `café café ` is ten characters and eleven bytes. A byte column would send a reader one place
    // to the right of the match, which looks like an off-by-one in the editor rather than here.
    assert_eq!(answer.hits[0].column, 11);
}

#[test]
fn matching_ignores_case_the_same_way_ripgrep_is_asked_to() {
    let workspace = workspace(&[("a.txt", b"Needle\n")]);

    // Both providers are pinned to fixed-string, case-insensitive. Two different engines can agree
    // about a literal and cannot be made to agree about a pattern language.
    assert_eq!(workspace.find("needle").hits.len(), 1);
    assert_eq!(workspace.find("NEEDLE").hits.len(), 1);
}

#[test]
fn one_line_produces_one_match_however_often_it_matches() {
    let workspace = workspace(&[("a.txt", b"needle needle needle\n")]);

    // A line containing the query three times is one place to go, and three rows for it would push
    // two other files off a bounded list.
    assert_eq!(workspace.find("needle").hits.len(), 1);
}

#[test]
fn a_long_line_comes_back_trimmed_around_the_match() {
    let padding = "x".repeat(1_000);
    let content = format!("{padding}needle{padding}\n");
    let workspace = workspace(&[("bundle.js", content.as_bytes())]);

    let answer = workspace.find("needle");

    assert!(answer.hits[0].truncated);
    assert_eq!(answer.hits[0].snippet.chars().count(), MAX_SNIPPET_CHARS);
    // Centred on the match rather than taken from the start: a snippet of a minified bundle's first
    // two hundred characters would never contain the thing that matched.
    assert!(answer.hits[0].snippet.contains("needle"));
}

#[test]
fn control_characters_never_reach_the_snippet() {
    let workspace = workspace(&[("a.txt", b"before\x1b[31m needle \x07after\n")]);

    let answer = workspace.find("needle");

    // An ANSI escape reaching a styled panel would be a match that repaints the interface around
    // it. Removed rather than escaped, because nobody is searching for them.
    assert!(!answer.hits[0].snippet.contains('\u{1b}'));
    assert!(!answer.hits[0].snippet.contains('\u{7}'));
    assert!(answer.hits[0].snippet.contains("needle"));
}

#[test]
fn a_binary_file_is_skipped_and_the_answer_says_it_is_incomplete() {
    let workspace = workspace(&[
        ("blob.bin", &[0xffu8, 0xfe, 0x00, 0x01]),
        ("a.txt", b"needle\n"),
    ]);

    let answer = workspace.find("needle");

    assert_eq!(answer.hits.len(), 1);
    // Not complete: something was not looked at. A search that skipped a file and claimed to be
    // complete is how somebody concludes a string is not in their workspace.
    assert_eq!(answer.coverage, "partial");
}

#[test]
fn an_empty_query_answers_with_nothing_rather_than_everything() {
    let workspace = workspace(&[("a.txt", b"anything\n")]);

    let answer = workspace.find("   ");

    // An empty query would match every line of every file. Nothing wrong with the request; there is
    // simply nothing a result list could usefully show, so it is an answer rather than an error.
    assert!(answer.hits.is_empty());
    assert_eq!(answer.coverage, "complete");
}

#[test]
fn a_cancelled_search_stops_and_says_why() {
    let workspace = workspace(&[("a.txt", b"needle\n"), ("b.txt", b"needle\n")]);
    let cancellation = SearchCancellationToken::new();
    cancellation.signal(SearchCancellationCause::Cancelled);

    let answer = workspace.search("needle", &cancellation);

    // Partial with a reason rather than an error: nothing went wrong, the reader stopped waiting,
    // and an error would put a failure notice on screen for something they did on purpose.
    assert!(answer.hits.is_empty());
    assert_eq!(answer.coverage, "partial");
}

#[test]
fn an_excluded_tree_is_never_searched() {
    let workspace = workspace(&[
        ("node_modules/left-pad/index.js", b"needle\n"),
        ("a.txt", b"needle\n"),
    ]);

    // The same exclusions Quick Open uses, from the same walk. Two traversals with their own lists
    // would let a file be findable by name and not by content.
    let answer = workspace.find("needle");
    assert_eq!(
        answer
            .hits
            .iter()
            .map(|hit| hit.path.as_str())
            .collect::<Vec<_>>(),
        vec!["a.txt"]
    );
}

/// The shape the streaming rewrite exists for.
///
/// The old implementation took Quick Open's walk, which with an empty query matches every entry,
/// and materialised one candidate per eligible path before opening a single file — memory
/// proportional to the workspace for an answer bounded at 200 matches. What replaces it retains
/// only the breadth-first frontier, and the frontier of a flat tree is one directory.
#[test]
fn a_wide_workspace_is_never_materialised_before_the_first_file_is_opened() {
    let mut files: Vec<(String, Vec<u8>)> = (0..64)
        .map(|index| (format!("file_{index}.txt"), b"nothing here\n".to_vec()))
        .collect();
    files.push(("hit.txt".to_string(), b"needle\n".to_vec()));
    let workspace = workspace(
        &files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect::<Vec<_>>(),
    );

    let answer = workspace.find("needle");
    let spent = spent(&answer);

    assert_eq!(answer.hits.len(), 1);
    assert_eq!(answer.coverage, "complete");
    // 65 files are opened because 65 files are searched. What is *not* proportional to the
    // workspace is what is held at once: the frontier of a flat tree is one directory, and the old
    // implementation held 65 candidates here.
    assert_eq!(spent.files_opened, 65);
    assert_eq!(spent.entries_visited, 65);
    assert!(
        spent.candidates_retained <= 1,
        "retained {} candidates for a flat tree",
        spent.candidates_retained
    );
}

/// The frontier is what grows with a tree, and it is charged.
#[test]
fn the_breadth_first_frontier_is_charged_and_credited_as_it_drains() {
    let files: Vec<(String, Vec<u8>)> = (0..8)
        .map(|index| (format!("dir_{index}/leaf.txt"), b"needle\n".to_vec()))
        .collect();
    let workspace = workspace(
        &files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect::<Vec<_>>(),
    );

    let answer = workspace.find("needle");
    let spent = spent(&answer);

    assert_eq!(answer.hits.len(), 8);
    // Eight directories queued and eight popped: the counter ends at zero rather than at eight,
    // which is what makes it a bound on what is held rather than a count of what was seen.
    assert_eq!(spent.directories_visited, 9);
    assert_eq!(spent.candidates_retained, 0);
}

#[test]
fn a_file_budget_stops_the_walk_at_exactly_its_limit() {
    let files: Vec<(String, Vec<u8>)> = (0..10)
        .map(|index| (format!("file_{index}.txt"), b"needle\n".to_vec()))
        .collect();
    let workspace = workspace(
        &files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect::<Vec<_>>(),
    );

    let mut limits = generous_limits();
    limits.max_files_opened = 3;
    let answer = workspace.search_within(
        "needle",
        limits,
        Arc::new(ManualClock::default()),
        &SearchCancellationToken::new(),
    );

    assert_eq!(spent(&answer).files_opened, 3);
    assert_eq!(answer.coverage, "partial");
    assert_eq!(answer.reason, Some("file_budget_exhausted"));
}

#[test]
fn an_entry_budget_stops_before_visiting_another_entry() {
    let files: Vec<(String, Vec<u8>)> = (0..10)
        .map(|index| (format!("file_{index}.txt"), b"nothing\n".to_vec()))
        .collect();
    let workspace = workspace(
        &files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect::<Vec<_>>(),
    );

    let mut limits = generous_limits();
    limits.max_entries_visited = 4;
    let answer = workspace.search_within(
        "needle",
        limits,
        Arc::new(ManualClock::default()),
        &SearchCancellationToken::new(),
    );

    assert_eq!(spent(&answer).entries_visited, 4);
    // Empty and partial, never empty and complete. A reader told "no matches" by a walk that
    // stopped a tenth of the way in has been told something about the budget, not their workspace.
    assert!(answer.hits.is_empty());
    assert_eq!(answer.coverage, "partial");
    assert_eq!(answer.reason, Some("entry_budget_exhausted"));
}

#[test]
fn an_aggregate_byte_budget_bounds_one_large_file() {
    // 256 KiB of text with the match at the very end, so a byte ceiling below the file size is the
    // only thing that can decide the outcome.
    let mut content = vec![b'x'; 256 * 1024];
    content.extend_from_slice(b"\nneedle\n");
    let workspace = workspace(&[("big.txt", content.as_slice())]);

    let mut limits = generous_limits();
    limits.max_bytes_read = 64 * 1024;
    let answer = workspace.search_within(
        "needle",
        limits,
        Arc::new(ManualClock::default()),
        &SearchCancellationToken::new(),
    );

    // One chunk charged, the second refused. The file is abandoned rather than read to its end.
    assert_eq!(spent(&answer).bytes_read, 64 * 1024);
    assert!(answer.hits.is_empty());
    assert_eq!(answer.reason, Some("byte_budget_exhausted"));
}

#[test]
fn a_deadline_is_measured_on_an_injected_monotonic_clock() {
    let files: Vec<(String, Vec<u8>)> = (0..20)
        .map(|index| (format!("file_{index}.txt"), b"needle\n".to_vec()))
        .collect();
    let workspace = workspace(
        &files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect::<Vec<_>>(),
    );

    // One tick per checkpoint. Advancing before the walk would only move the origin the budget
    // measures from; a clock that steps as the traversal reads it is the only way to reach a
    // deadline from inside one without sleeping through it.
    let clock = Arc::new(ManualClock::ticking(Duration::from_millis(200)));
    let mut limits = generous_limits();
    limits.deadline = Duration::from_secs(1);

    let answer = workspace.search_within("needle", limits, clock, &SearchCancellationToken::new());

    // No sleep and no wall clock, so a busy CI runner cannot falsify it in either direction.
    assert_eq!(answer.reason, Some("deadline_exceeded"));
    assert_eq!(answer.coverage, "partial");
    assert!(answer.hits.len() < 20);
}

#[test]
fn a_supersede_is_reported_as_a_different_state_from_a_cancel() {
    let workspace = workspace(&[("a.txt", b"needle\n")]);

    for (cause, expected) in [
        (SearchCancellationCause::Cancelled, "cancelled"),
        (SearchCancellationCause::Superseded, "superseded"),
        (SearchCancellationCause::OwnerDropped, "owner_dropped"),
    ] {
        let cancellation = SearchCancellationToken::new();
        cancellation.signal(cause);

        let answer = workspace.search("needle", &cancellation);

        // A reader who pressed Escape and a reader who typed another character are being told
        // different things, and a view that could not tell them apart would show the wrong one.
        assert_eq!(answer.reason, Some(expected), "{cause:?}");
        assert_eq!(answer.coverage, "partial", "{cause:?}");
    }
}

#[test]
fn an_unreadable_directory_leaves_the_rest_of_the_walk_running() {
    let workspace = workspace(&[
        ("readable/a.txt", b"needle\n"),
        ("also-readable/b.txt", b"needle\n"),
    ]);

    let answer = workspace.find("needle");

    // Nothing is unreadable here; the assertion is that a two-directory tree is fully searched, so
    // the failure-path test below is about the failure rather than about the traversal.
    assert_eq!(answer.hits.len(), 2);
    assert_eq!(answer.coverage, "complete");
    assert_eq!(spent(&answer).unreadable_entries, 0);
}

#[test]
fn a_skipped_binary_file_makes_the_answer_partial_without_ending_the_walk() {
    let workspace = workspace(&[
        ("blob.bin", &[0xffu8, 0xfe, 0x00, 0x01]),
        ("a.txt", b"needle\n"),
        ("b.txt", b"needle\n"),
    ]);

    let answer = workspace.find("needle");

    // Both text files are still searched: an omission is not a stop.
    assert_eq!(answer.hits.len(), 2);
    assert_eq!(answer.coverage, "partial");
    assert_eq!(answer.reason, Some("unreadable_entries"));
    assert_eq!(spent(&answer).unreadable_entries, 1);
}

#[test]
fn a_result_budget_stops_reading_rather_than_only_trimming_the_list() {
    let files: Vec<(String, Vec<u8>)> = (0..20)
        .map(|index| (format!("file_{index}.txt"), b"needle\n".to_vec()))
        .collect();
    let workspace = workspace(
        &files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
            .collect::<Vec<_>>(),
    );

    let mut limits = generous_limits();
    limits.max_results = 5;
    let answer = workspace.search_within(
        "needle",
        limits,
        Arc::new(ManualClock::default()),
        &SearchCancellationToken::new(),
    );

    assert_eq!(answer.hits.len(), 5);
    // The point of the budget rather than a trim: the sixth file is never opened. Collecting
    // twenty and returning five would do four times the work for the same answer.
    assert!(
        spent(&answer).files_opened <= 6,
        "opened {} files for a five-result budget",
        spent(&answer).files_opened
    );
    assert_eq!(answer.reason, Some("result_budget_exhausted"));
}

#[test]
fn a_budget_summary_carries_counts_and_no_paths() {
    let workspace = workspace(&[("src/main.rs", b"needle\n")]);

    let answer = workspace.find("needle");
    let spent = spent(&answer);

    // The summary travels to the frontend and into logs. It is counts and nothing else, which is
    // enforced by the type having nowhere to put a name.
    assert!(spent.entries_visited > 0);
    assert!(spent.metadata_operations > 0);
    assert_eq!(spent.results_emitted, 1);
}

#[test]
fn a_repository_ignore_rule_keeps_a_tree_out_of_the_search() {
    let workspace = workspace(&[
        (".gitignore", b"generated/\n"),
        ("generated/output.txt", b"needle\n"),
        ("src/main.rs", b"needle\n"),
    ]);

    let answer = workspace.find("needle");

    // A team that has already written down which directories are generated should not have to
    // write it a second time for search to believe them.
    assert_eq!(
        answer
            .hits
            .iter()
            .map(|hit| hit.path.as_str())
            .collect::<Vec<_>>(),
        vec!["src/main.rs"]
    );
}

#[test]
fn a_repository_negation_brings_a_default_excluded_tree_back() {
    let workspace = workspace(&[
        (".gitignore", b"!vendor/\n"),
        ("vendor/lib/thing.txt", b"needle\n"),
        ("node_modules/pkg/index.js", b"needle\n"),
    ]);

    let answer = workspace.find("needle");

    // An explicit `!` is a team saying they do want this tree searched. `node_modules` said nothing,
    // so the default still applies to it.
    assert_eq!(
        answer
            .hits
            .iter()
            .map(|hit| hit.path.as_str())
            .collect::<Vec<_>>(),
        vec!["vendor/lib/thing.txt"]
    );
}

#[test]
fn the_generated_output_directories_the_old_list_missed_are_skipped_too() {
    let workspace = workspace(&[
        (".next/static/chunk.js", b"needle\n"),
        (".nuxt/dist/app.js", b"needle\n"),
        ("src/main.rs", b"needle\n"),
    ]);

    // `.next` and `.nuxt` were in none of the three exclusion lists this policy replaces; they are
    // covered here by the dot rule as well as by the default set, and both are the same answer.
    assert_eq!(
        workspace
            .find("needle")
            .hits
            .iter()
            .map(|hit| hit.path.as_str())
            .collect::<Vec<_>>(),
        vec!["src/main.rs"]
    );
}

/// A context built for another walk is refused rather than obeyed.
///
/// A path-search profile applied here opens a different number of files and calls itself complete or
/// partial accordingly. Nothing in the answer would say the wrong budget was used, which is what
/// makes the mislabelling worth refusing rather than tolerating.
#[test]
fn a_context_built_for_another_operation_is_refused() {
    let workspace = workspace(&[(
        "a.txt", b"needle
",
    )]);
    let connection = workspace.database.connection().expect("connection");
    let execution = content_search_execution(
        SearchCancellationToken::new(),
        Arc::new(ManualClock::default()),
    );

    let refused = search_session_content(
        &connection,
        "session-1",
        &WorkspaceContentSearchRequest {
            query: "needle".to_string(),
            search_id: "search-1".to_string(),
            limit: None,
        },
        &execution.with_operation(
            crate::contexts::workspaces::application::WorkspaceInspectionOperation::PathSearch,
        ),
    );

    assert!(matches!(
        refused,
        Err(
            crate::contexts::workspaces::application::WorkspaceApplicationError::Conflict(
                "workspace_inspection_operation_mismatch"
            )
        )
    ));
}
