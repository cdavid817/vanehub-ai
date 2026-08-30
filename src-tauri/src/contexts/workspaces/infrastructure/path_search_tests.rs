//! What Quick Open promises about ordering, paging, and honesty.
//!
//! Driven over a real temporary workspace rather than a fixture: the ranking is about names and
//! depths on a filesystem, and a fixture asserting its own layout would prove nothing about the
//! walk that produces it.

use super::path_search::{normalize_query, path_match_score, search_session_paths};
use crate::contexts::workspaces::application::{
    ManualClock, MonotonicClockPort, SearchCancellationCause, SearchCancellationToken,
    WorkspaceApplicationError as AppError, WorkspaceIgnorePolicy, WorkspaceInspectionBudgetLimits,
    WorkspaceInspectionBudgetSnapshot, WorkspaceInspectionExecution, WorkspaceInspectionOperation,
    WorkspacePathSearchRequest, WorkspaceSearchCancellation,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

struct Workspace {
    _directory: TempDirectory,
    database: NativeDatabase,
    root: PathBuf,
}

impl Workspace {
    fn root(&self) -> &Path {
        &self.root
    }

    fn search(&self, query: &str, cursor: Option<String>, limit: Option<usize>) -> SearchAnswer {
        let connection = self.database.connection().expect("connection");
        answer_from(
            search_session_paths(
                &connection,
                "session-1",
                &WorkspacePathSearchRequest {
                    query: query.to_string(),
                    search_id: "quick-open-1".to_string(),
                    cursor,
                    limit,
                },
                &path_search_execution(),
            )
            .expect("search"),
        )
    }

    /// The same search under a token the test already signalled.
    ///
    /// Signalled before the walk starts rather than during it. A test that raced a running traversal
    /// would be asserting on where the walk happened to be, which is the one thing about it that is
    /// not deterministic.
    fn search_stopped_by(&self, query: &str, token: SearchCancellationToken) -> SearchAnswer {
        let connection = self.database.connection().expect("connection");
        answer_from(
            search_session_paths(
                &connection,
                "session-1",
                &WorkspacePathSearchRequest {
                    query: query.to_string(),
                    search_id: "quick-open-1".to_string(),
                    cursor: None,
                    limit: Some(10),
                },
                &execution_stopped_by(token),
            )
            .expect("search"),
        )
    }

    /// The same search under ignore rules the test chose.
    fn search_under(
        &self,
        query: &str,
        limit: Option<usize>,
        ignore: WorkspaceIgnorePolicy,
    ) -> SearchAnswer {
        let connection = self.database.connection().expect("connection");
        answer_from(
            search_session_paths(
                &connection,
                "session-1",
                &WorkspacePathSearchRequest {
                    query: query.to_string(),
                    search_id: "quick-open-1".to_string(),
                    cursor: None,
                    limit,
                },
                &path_search_execution().with_ignore(ignore),
            )
            .expect("search"),
        )
    }

    /// The same search with its budget and its clock chosen by the test.
    fn search_within(
        &self,
        query: &str,
        limit: Option<usize>,
        limits: WorkspaceInspectionBudgetLimits,
        clock: Arc<dyn MonotonicClockPort>,
    ) -> SearchAnswer {
        let connection = self.database.connection().expect("connection");
        answer_from(
            search_session_paths(
                &connection,
                "session-1",
                &WorkspacePathSearchRequest {
                    query: query.to_string(),
                    search_id: "quick-open-1".to_string(),
                    cursor: None,
                    limit,
                },
                &execution_with(clock).with_limits(limits),
            )
            .expect("search"),
        )
    }
}

/// A context for a walk nobody intends to cancel.
///
/// Registered against a real registry rather than assembled from parts, because the generation a
/// context carries is only meaningful if something issued it.
fn path_search_execution() -> WorkspaceInspectionExecution {
    execution_with(Arc::new(ManualClock::default()))
}

fn execution_with(clock: Arc<dyn MonotonicClockPort>) -> WorkspaceInspectionExecution {
    let registry = Arc::new(WorkspaceSearchCancellation::default());
    let registration = registry.begin("quick-open-1");
    let execution = WorkspaceInspectionExecution::path_search(
        registration.generation(),
        registration.token(),
        clock,
    );
    // Completed here rather than held: the walk polls the token, and a guard kept alive only to
    // satisfy a lifetime would be a registration this test never uses.
    registration.complete();
    execution
}

fn execution_stopped_by(token: SearchCancellationToken) -> WorkspaceInspectionExecution {
    let registry = Arc::new(WorkspaceSearchCancellation::default());
    let registration = registry.begin("quick-open-1");
    let execution = WorkspaceInspectionExecution::path_search(
        registration.generation(),
        token,
        Arc::new(ManualClock::default()),
    );
    registration.complete();
    execution
}

/// Makes a directory refuse to be enumerated, until the returned guard is dropped.
///
/// Two mechanisms because there is no one portable way, and the alternative — testing this on Unix
/// only — would leave the branch that turns an unreadable subdirectory into partial coverage
/// unexercised on the platform most of this project's users are on.
///
/// One guard type either way, and the release happens in `Drop` rather than at the call site. The
/// first version had the caller undo it, which meant the Windows path dropped a handle and the Unix
/// path restored a mode — two shapes for one idea, and the arm that does not compile on the machine
/// you are sitting at is the arm you do not find out about until CI.
#[cfg(windows)]
struct EnumerationDenied(#[allow(dead_code)] fs::File);

#[cfg(windows)]
fn deny_enumeration(path: &std::path::Path) -> EnumerationDenied {
    use std::os::windows::fs::OpenOptionsExt;

    // An exclusive handle. `read_dir` opens with the read/write/delete share modes, so a handle that
    // shares nothing makes the next open a sharing violation. `FILE_FLAG_BACKUP_SEMANTICS` is what
    // permits opening a directory at all. Holding it *is* the denial; dropping it lifts it.
    EnumerationDenied(
        fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .custom_flags(0x0200_0000)
            .open(path)
            .expect("an exclusive handle on the directory"),
    )
}

#[cfg(unix)]
struct EnumerationDenied {
    path: PathBuf,
    original: fs::Permissions,
}

#[cfg(unix)]
impl Drop for EnumerationDenied {
    fn drop(&mut self) {
        // Restored even if the test failed: a temporary directory nobody can read is a temporary
        // directory nobody can delete.
        let _ = fs::set_permissions(&self.path, self.original.clone());
    }
}

#[cfg(unix)]
fn deny_enumeration(path: &std::path::Path) -> EnumerationDenied {
    use std::os::unix::fs::PermissionsExt;

    let original = fs::metadata(path).expect("metadata").permissions();
    fs::set_permissions(path, fs::Permissions::from_mode(0o000)).expect("chmod");
    EnumerationDenied {
        path: path.to_path_buf(),
        original,
    }
}

fn answer_from(
    result: crate::contexts::workspaces::application::WorkspacePathSearchResult,
) -> SearchAnswer {
    SearchAnswer {
        paths: result
            .matches
            .iter()
            .map(|entry| entry.path.clone())
            .collect(),
        kinds: result
            .matches
            .iter()
            .map(|entry| entry.kind.to_string())
            .collect(),
        coverage: result.coverage.state.token(),
        reason: result.coverage.reason_code,
        budget: result.coverage.budget,
        next_cursor: result.next_cursor,
    }
}

struct SearchAnswer {
    paths: Vec<String>,
    kinds: Vec<String>,
    coverage: &'static str,
    reason: Option<&'static str>,
    /// What the search spent. The instrumentation the memory bound is read from: a claim about
    /// retained candidates is only checkable if the traversal reports how many it held.
    budget: Option<WorkspaceInspectionBudgetSnapshot>,
    next_cursor: Option<String>,
}

fn spent(answer: &SearchAnswer) -> WorkspaceInspectionBudgetSnapshot {
    answer
        .budget
        .expect("coverage carries what the search spent")
}

/// A budget whose every dimension is generous, so a test can lower exactly one.
fn generous_limits() -> WorkspaceInspectionBudgetLimits {
    WorkspaceInspectionBudgetLimits {
        max_directories_visited: 1_000,
        max_entries_visited: 100_000,
        max_files_opened: 0,
        max_bytes_read: 0,
        max_metadata_operations: 400_000,
        max_retained_candidates: 10_000,
        max_results: 50,
        max_depth: 10,
        deadline: Duration::from_secs(600),
    }
}

fn workspace(files: &[&str], directories: &[&str]) -> Workspace {
    let directory = TempDirectory::new("quick-open");
    let root = directory.path().join("workspace");
    fs::create_dir_all(&root).expect("root");
    for name in directories {
        fs::create_dir_all(root.join(name)).expect("directory");
    }
    for name in files {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(&path, "x").expect("file");
    }

    let database = NativeDatabase::new(directory.path().join("data")).expect("database");
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO sessions \
             (id, title, agent_id, interaction_mode, lifecycle_state, folder, pinned, archived, \
              created_at, updated_at) \
             VALUES ('session-1', 'Quick Open', 'codex-cli', 'cli', 'idle', ?1, 0, 0, \
                     '2026-08-26T10:00:00Z', '2026-08-26T10:00:00Z')",
            params![root.to_string_lossy().as_ref()],
        )
        .expect("insert session");
    drop(connection);

    Workspace {
        _directory: directory,
        database,
        root,
    }
}

#[test]
fn an_exact_name_outranks_a_prefix_and_a_prefix_outranks_a_substring() {
    let workspace = workspace(
        &[
            "main.rs",
            "deep/mainly.rs",
            "deep/domain.rs",
            "deep/other.rs",
        ],
        &["deep"],
    );

    let answer = workspace.search("main", None, None);

    // Four tiers rather than a fuzzy distance: a reader typing `main` expects `main.rs` first, and
    // a scoring function they cannot predict makes the top result feel arbitrary — which is worse
    // than one they disagree with but understand.
    assert_eq!(
        answer.paths,
        vec![
            "main.rs".to_string(),
            "mainly.rs".to_string(),
            "domain.rs".to_string(),
        ]
        .into_iter()
        .map(|name| if name == "main.rs" {
            name
        } else {
            format!("deep/{name}")
        })
        .collect::<Vec<_>>()
    );
}

#[test]
fn a_directory_is_findable_and_says_that_it_is_one() {
    let workspace = workspace(&["components/button.tsx"], &["components"]);

    let answer = workspace.search("compo", None, None);

    // The mention search skips directories, which is right for a prompt and wrong here: a reader
    // opening Quick Open may be trying to reach a folder, and a result list that hid them would
    // make that impossible.
    let index = answer
        .paths
        .iter()
        .position(|path| path == "components")
        .expect("directory present");
    assert_eq!(answer.kinds[index], "directory");
}

#[test]
fn a_lockfile_is_findable_even_though_a_mention_would_not_offer_it() {
    let workspace = workspace(&["package-lock.json"], &[]);

    // The whole reason this is a separate operation. The mention search filters to source
    // extensions, so widening it in place would have changed what an `@` offers.
    assert_eq!(
        workspace.search("lock", None, None).paths,
        vec!["package-lock.json".to_string()]
    );
}

#[test]
fn a_page_resumes_after_the_entry_it_ended_on() {
    let workspace = workspace(&["a-main.rs", "b-main.rs", "c-main.rs", "d-main.rs"], &[]);

    let first = workspace.search("main", None, Some(2));
    assert_eq!(first.paths.len(), 2);
    let cursor = first.next_cursor.clone().expect("cursor");

    let second = workspace.search("main", Some(cursor), Some(2));

    // Nothing repeated and nothing skipped. A repeat reads to a reader as a duplicate file rather
    // than as a paging bug.
    assert!(second.paths.iter().all(|path| !first.paths.contains(path)));
    assert_eq!(
        [first.paths, second.paths].concat(),
        vec![
            "a-main.rs".to_string(),
            "b-main.rs".to_string(),
            "c-main.rs".to_string(),
            "d-main.rs".to_string()
        ]
    );
}

#[test]
fn the_last_page_offers_no_cursor() {
    let workspace = workspace(&["only-main.rs"], &[]);

    // A cursor on the last page would invite one more request that returns nothing, and a reader
    // waiting on it cannot tell that from a slow search.
    assert!(workspace
        .search("main", None, Some(10))
        .next_cursor
        .is_none());
}

#[test]
fn a_cursor_from_another_query_is_refused() {
    let workspace = workspace(&["a-main.rs", "b-main.rs", "c-main.rs"], &[]);
    let cursor = workspace
        .search("main", None, Some(1))
        .next_cursor
        .expect("cursor");

    let connection = workspace.database.connection().expect("connection");
    let refusal = search_session_paths(
        &connection,
        "session-1",
        &WorkspacePathSearchRequest {
            search_id: "quick-open-1".to_string(),
            // A different query ranks the same files differently, so this cursor names a position
            // the new ordering never produced.
            query: "rs".to_string(),
            cursor: Some(cursor),
            limit: Some(1),
        },
        &path_search_execution(),
    );

    let refusal = refusal.expect("a refusal is an answer, not a failure");
    // An empty page carrying the reason. An error would leave the caller unable to tell "start this
    // search again" from "this workspace is unreachable", and only the first is actionable.
    assert!(refusal.matches.is_empty());
    assert_eq!(refusal.next_cursor, None);
    assert_eq!(refusal.coverage.reason_code, Some("invalid_cursor"));
}

#[test]
fn a_complete_walk_says_so() {
    let workspace = workspace(&["main.rs"], &[]);

    // The honest half of the answer. A truncated result that claimed to be complete is how
    // somebody concludes a file does not exist.
    assert_eq!(workspace.search("main", None, None).coverage, "complete");
}

#[test]
fn an_empty_query_browses_shallowest_first() {
    let workspace = workspace(&["top.rs", "one/two/deep.rs"], &["one", "one/two"]);

    let answer = workspace.search("", None, Some(10));

    // Empty is a query, not a missing one. Breadth-first plus the depth tie break puts the entries
    // a reader is most likely to want at the top.
    assert_eq!(answer.paths.first().map(String::as_str), Some("one"));
    assert!(
        answer.paths.iter().position(|path| path == "top.rs")
            < answer
                .paths
                .iter()
                .position(|path| path == "one/two/deep.rs")
    );
}

#[test]
fn an_excluded_tree_is_never_offered() {
    let workspace = workspace(
        &["node_modules/mainlib/index.js", "main.rs"],
        &["node_modules"],
    );

    // A reader is never trying to reach `node_modules` by name, and the result budget spent there
    // is budget not spent on their own files.
    assert_eq!(
        workspace.search("main", None, None).paths,
        vec!["main.rs".to_string()]
    );
}

#[test]
fn the_query_is_normalized_once_so_two_spellings_share_a_cursor() {
    // The cursor carries the query. Normalizing at each comparison instead would let `  Main ` and
    // `main` produce two cursors that refuse each other, for a search the reader considers the same.
    assert_eq!(normalize_query("  Main\\Sub "), "main/sub");
}

#[test]
fn scoring_is_the_same_function_both_providers_use() {
    // Imported by the remote provider rather than reimplemented there. Two implementations of an
    // ordering disagree first about the ties nobody writes tests for, so the tiers are pinned here
    // once and both sides inherit them.
    let exact = path_match_score("main.rs", "main.rs", "src/main.rs");
    let prefix = path_match_score("main", "main.rs", "src/main.rs");
    let substring = path_match_score("ain", "main.rs", "src/main.rs");
    let path_only = path_match_score("src", "main.rs", "src/main.rs");

    assert!(exact > prefix);
    assert!(prefix > substring);
    assert!(substring > path_only);
    assert!(path_only.is_some());
    // Nothing at all is `None` rather than a low score: a zero would sort the whole workspace into
    // every result list, below the matches but present.
    assert_eq!(path_match_score("zzz", "main.rs", "src/main.rs"), None);
    // An empty query browses, so everything qualifies.
    assert_eq!(path_match_score("", "main.rs", "src/main.rs"), Some(0));
}

/// The bound the bounded selection exists for.
///
/// Every one of these files matches, so a full sort would retain all 400 before returning five.
/// What is kept instead is the page plus the one entry that proves another page follows.
#[test]
fn a_workspace_full_of_matches_retains_only_one_page_of_them() {
    let names: Vec<String> = (0..400)
        .map(|index| format!("main_{index:03}.rs"))
        .collect();
    let workspace = workspace(&names.iter().map(String::as_str).collect::<Vec<_>>(), &[]);

    let answer = workspace.search("main", None, Some(5));
    let spent = spent(&answer);

    assert_eq!(answer.paths.len(), 5);
    assert_eq!(spent.entries_visited, 400, "every entry is still visited");
    assert!(
        spent.candidates_retained <= 6,
        "retained {} candidates for a five-entry page",
        spent.candidates_retained
    );
}

/// Visiting is not the same as keeping, and both are counted.
#[test]
fn every_visited_entry_is_charged_even_when_nothing_matches() {
    let names: Vec<String> = (0..50)
        .map(|index| format!("other_{index:02}.rs"))
        .collect();
    let workspace = workspace(&names.iter().map(String::as_str).collect::<Vec<_>>(), &[]);

    let answer = workspace.search("zzzz-no-such-name", None, Some(10));
    let spent = spent(&answer);

    assert!(answer.paths.is_empty());
    // Fifty entries examined for zero results. A result cap never sees this work, which is why the
    // entry budget is the one that matters on a tree that matches nothing.
    assert_eq!(spent.entries_visited, 50);
    assert_eq!(spent.candidates_retained, 0);
    // Nothing was omitted, so an empty answer here really does mean no matches.
    assert_eq!(answer.coverage, "complete");
}

#[test]
fn the_page_is_the_same_one_a_full_sort_would_have_produced() {
    // Interleaved so the best matches are neither first nor last in walk order: a selection that
    // simply kept the first `limit` arrivals would pass a test where they happened to be adjacent.
    let workspace = workspace(
        &[
            "zzz/deep/main.rs",
            "aaa.rs",
            "main.rs",
            "zzz/mainly.rs",
            "bbb/domain.rs",
        ],
        &["zzz", "zzz/deep", "bbb"],
    );

    let answer = workspace.search("main", None, Some(3));

    // All three are name-prefix matches, so the depth tie break decides: shallowest first. The
    // point is that walk order does not — the best entries arrive second, third and fourth, and a
    // selection that kept the first three arrivals would have returned a different page.
    assert_eq!(
        answer.paths,
        vec![
            "main.rs".to_string(),
            "zzz/mainly.rs".to_string(),
            "zzz/deep/main.rs".to_string(),
        ]
    );
}

#[test]
fn equal_sort_keys_break_deterministically_on_the_path() {
    // Same score, same depth. Only the path key separates them, and it has to be the same
    // separation on every run or paging would repeat or skip an entry.
    let workspace = workspace(&["b/main.rs", "a/main.rs", "c/main.rs"], &["a", "b", "c"]);

    let first = workspace.search("main.rs", None, Some(3));
    let second = workspace.search("main.rs", None, Some(3));

    assert_eq!(
        first.paths,
        vec![
            "a/main.rs".to_string(),
            "b/main.rs".to_string(),
            "c/main.rs".to_string(),
        ]
    );
    assert_eq!(first.paths, second.paths);
}

#[test]
fn an_entry_budget_stops_the_walk_and_says_which_one_did() {
    let names: Vec<String> = (0..40).map(|index| format!("main_{index:02}.rs")).collect();
    let workspace = workspace(&names.iter().map(String::as_str).collect::<Vec<_>>(), &[]);

    let mut limits = generous_limits();
    limits.max_entries_visited = 7;
    let answer =
        workspace.search_within("main", Some(50), limits, Arc::new(ManualClock::default()));

    assert_eq!(spent(&answer).entries_visited, 7);
    assert_eq!(answer.coverage, "partial");
    assert_eq!(answer.reason, Some("entry_budget_exhausted"));
}

#[test]
fn a_deadline_is_measured_on_an_injected_monotonic_clock() {
    let names: Vec<String> = (0..40).map(|index| format!("main_{index:02}.rs")).collect();
    let workspace = workspace(&names.iter().map(String::as_str).collect::<Vec<_>>(), &[]);

    let mut limits = generous_limits();
    limits.deadline = Duration::from_secs(1);
    // One tick per checkpoint, so the deadline is reached by counting rather than by waiting.
    let answer = workspace.search_within(
        "main",
        Some(50),
        limits,
        Arc::new(ManualClock::ticking(Duration::from_millis(200))),
    );

    assert_eq!(answer.reason, Some("deadline_exceeded"));
    assert_eq!(answer.coverage, "partial");
}

#[test]
fn a_tree_deeper_than_the_limit_is_partial_rather_than_silently_short() {
    let mut relative = String::new();
    for _ in 0..13 {
        relative.push_str("level/");
    }
    relative.push_str("buried-main.rs");
    let workspace = workspace(&[&relative, "shallow-main.rs"], &[]);

    let answer = workspace.search("main", None, Some(10));

    assert!(answer.paths.contains(&"shallow-main.rs".to_string()));
    assert!(!answer.paths.contains(&relative));
    // The entries beside the refused branch are still searched; only the branch is omitted.
    assert_eq!(answer.coverage, "partial");
    assert_eq!(answer.reason, Some("depth_budget_exhausted"));
}

#[test]
fn a_path_search_never_opens_a_file() {
    let workspace = workspace(&["main.rs", "other.rs"], &[]);

    let spent = spent(&workspace.search("main", None, Some(10)));

    // Zero, not "few". A path search that opened a file has done something it was not asked to,
    // and the budget is where that is a testable statement rather than a convention.
    assert_eq!(spent.files_opened, 0);
    assert_eq!(spent.bytes_read, 0);
}

#[test]
fn quick_open_and_content_search_skip_the_same_trees() {
    let fixture = TempDirectory::new("quick-open-ignores");
    let root = fixture.path().join("workspace");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join(".gitignore"), "generated/\n").expect("rule file");
    for name in [
        "generated/main.rs",
        "node_modules/mainlib/index.js",
        "src/main.rs",
    ] {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(&path, "x").expect("file");
    }
    let database = NativeDatabase::new(fixture.path().join("data")).expect("database");
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO sessions \
             (id, title, agent_id, interaction_mode, lifecycle_state, folder, pinned, archived, \
              created_at, updated_at) \
             VALUES ('session-1', 'Quick Open', 'codex-cli', 'cli', 'idle', ?1, 0, 0, \
                     '2026-08-26T10:00:00Z', '2026-08-26T10:00:00Z')",
            params![root.to_string_lossy().as_ref()],
        )
        .expect("insert session");
    drop(connection);
    let workspace = Workspace {
        _directory: fixture,
        database,
        root,
    };

    // The whole reason for one policy: a workspace should not appear to have a different shape
    // depending on which box a reader types into.
    assert_eq!(
        workspace.search("main", None, Some(10)).paths,
        vec!["src/main.rs".to_string()]
    );
}

/// A context built for another walk is refused rather than obeyed.
///
/// The failure this guards against does not look like a failure: a document-discovery profile
/// applied to Quick Open returns fewer results and calls itself partial, which reads as a workspace
/// that is simply larger than it is. Nothing in the answer would say the wrong budget was used.
#[test]
fn a_context_built_for_another_operation_is_refused() {
    let workspace = workspace(&["main.rs"], &[]);
    let connection = workspace.database.connection().expect("connection");

    let refused = search_session_paths(
        &connection,
        "session-1",
        &WorkspacePathSearchRequest {
            query: "main".to_string(),
            search_id: "quick-open-1".to_string(),
            cursor: None,
            limit: None,
        },
        &path_search_execution().with_operation(WorkspaceInspectionOperation::DocumentDiscovery),
    );

    // An error rather than an empty page, unlike a stale cursor. A cursor a reader carried from an
    // earlier request is something they can recover from by starting again; a mislabelled context is
    // a caller bug, and answering it would hide the bug behind a plausible result.
    assert!(matches!(
        refused,
        Err(AppError::Conflict(
            "workspace_inspection_operation_mismatch"
        ))
    ));
}

/// The walk obeys the rules it was handed, not rules it chose.
///
/// Proved from the other side: under direct navigation the same tree is searched, so the skip in the
/// default mode is the policy acting rather than the fixture missing a file. A traversal that picked
/// its own mode could only ever be observed obeying it.
#[test]
fn an_ignored_tree_is_skipped_under_the_policy_the_caller_supplied() {
    let workspace = workspace(&["node_modules/vendored/main.rs", "src/main.rs"], &[]);

    let default = workspace.search("main.rs", None, Some(10));
    let direct = workspace.search_under(
        "main.rs",
        Some(10),
        WorkspaceIgnorePolicy::direct_navigation(),
    );

    assert_eq!(default.paths, vec!["src/main.rs".to_string()]);
    assert!(direct
        .paths
        .contains(&"node_modules/vendored/main.rs".to_string()));
    // Complete either way. An ignored tree is a discovery rule, not an omission — reporting partial
    // would put a "we did not finish" notice on every search in a project with dependencies, which
    // is how a notice stops being read.
    assert_eq!(default.coverage, "complete");
    assert_eq!(default.reason, None);
}

/// An unreadable subdirectory is an omission, not a failure.
///
/// The entries beside it are still in scope and are still returned. Failing the whole search would
/// make one permission quirk anywhere in a workspace into "Quick Open does not work here", and the
/// reader has no way to find out which folder caused it.
#[test]
fn an_unreadable_subdirectory_is_reported_rather_than_failing_the_search() {
    let workspace = workspace(&["readable/main.rs", "locked/main.rs"], &[]);
    let locked = workspace.root().join("locked");
    let guard = deny_enumeration(&locked);

    let answer = workspace.search("main.rs", None, Some(10));

    // Held across the search and released only now: the guard is the denial on both platforms.
    drop(guard);
    assert_eq!(answer.paths, vec!["readable/main.rs".to_string()]);
    assert_eq!(answer.coverage, "partial");
    assert_eq!(answer.reason, Some("unreadable_entries"));
    assert_eq!(spent(&answer).unreadable_entries, 1);
}

/// A cancelled search says it was cancelled, and says it about an empty list.
///
/// Distinct from "no matches": the reader stopped this one, and telling them the string is not in
/// their workspace is a claim nobody established.
#[test]
fn a_cancelled_search_reports_the_cancellation_rather_than_an_empty_workspace() {
    let names: Vec<String> = (0..40).map(|index| format!("main_{index:02}.rs")).collect();
    let workspace = workspace(&names.iter().map(String::as_str).collect::<Vec<_>>(), &[]);
    let token = SearchCancellationToken::new();
    token.signal(SearchCancellationCause::Cancelled);

    let answer = workspace.search_stopped_by("main", token);

    assert!(answer.paths.is_empty());
    assert_eq!(answer.coverage, "partial");
    assert_eq!(answer.reason, Some("cancelled"));
}

/// A superseded search is told apart from a cancelled one.
///
/// Both stop the walk and both return nothing, and a reader is told different things: they cancelled
/// it, or they typed another character. Collapsing the two would make a keystroke look like a
/// failure the user caused on purpose.
#[test]
fn a_superseded_search_is_not_reported_as_a_cancellation() {
    let names: Vec<String> = (0..40).map(|index| format!("main_{index:02}.rs")).collect();
    let workspace = workspace(&names.iter().map(String::as_str).collect::<Vec<_>>(), &[]);
    let token = SearchCancellationToken::new();
    token.signal(SearchCancellationCause::Superseded);

    let answer = workspace.search_stopped_by("main", token);

    assert!(answer.paths.is_empty());
    assert_eq!(answer.reason, Some("superseded"));
}

/// Every remaining budget dimension stops the walk and names itself.
///
/// One test rather than four, because the assertion is the same one four times and the thing worth
/// reading is the table. A dimension that stopped the walk under another dimension's name would be
/// the failure this catches: the reason code is what a reader is shown, and the wrong one sends them
/// to narrow the wrong thing.
#[test]
fn each_budget_dimension_stops_the_walk_under_its_own_name() {
    let names: Vec<String> = (0..40).map(|index| format!("main_{index:02}.rs")).collect();
    let files: Vec<&str> = names.iter().map(String::as_str).collect();
    let workspace = workspace(&files, &["a", "b", "c", "d"]);

    for (reason, narrow) in [
        (
            "directory_budget_exhausted",
            (|limits: &mut WorkspaceInspectionBudgetLimits| limits.max_directories_visited = 2)
                as fn(&mut WorkspaceInspectionBudgetLimits),
        ),
        ("metadata_budget_exhausted", |limits| {
            limits.max_metadata_operations = 5
        }),
        ("candidate_budget_exhausted", |limits| {
            limits.max_retained_candidates = 3
        }),
        ("result_budget_exhausted", |limits| limits.max_results = 2),
    ] {
        let mut limits = generous_limits();
        narrow(&mut limits);

        let answer =
            workspace.search_within("main", Some(50), limits, Arc::new(ManualClock::default()));

        assert_eq!(answer.coverage, "partial", "{reason}");
        assert_eq!(answer.reason, Some(reason), "{reason}");
    }
}
