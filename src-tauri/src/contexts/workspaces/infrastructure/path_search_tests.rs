//! What Quick Open promises about ordering, paging, and honesty.
//!
//! Driven over a real temporary workspace rather than a fixture: the ranking is about names and
//! depths on a filesystem, and a fixture asserting its own layout would prove nothing about the
//! walk that produces it.

use super::path_search::{normalize_query, path_match_score, search_session_paths};
use crate::contexts::workspaces::application::WorkspacePathSearchRequest;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::fs;

struct Workspace {
    _directory: TempDirectory,
    database: NativeDatabase,
}

impl Workspace {
    fn search(&self, query: &str, cursor: Option<String>, limit: Option<usize>) -> SearchAnswer {
        let connection = self.database.connection().expect("connection");
        let result = search_session_paths(
            &connection,
            "session-1",
            &WorkspacePathSearchRequest {
                query: query.to_string(),
                cursor,
                limit,
            },
        )
        .expect("search");
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
            next_cursor: result.next_cursor,
        }
    }
}

struct SearchAnswer {
    paths: Vec<String>,
    kinds: Vec<String>,
    coverage: &'static str,
    next_cursor: Option<String>,
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
            // A different query ranks the same files differently, so this cursor names a position
            // the new ordering never produced.
            query: "rs".to_string(),
            cursor: Some(cursor),
            limit: Some(1),
        },
    );

    assert!(refusal.is_err());
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
