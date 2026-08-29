//! What a content search promises about positions, bounds, and stopping.
//!
//! Over a real temporary workspace, because the interesting parts are what happens to a long line,
//! a binary file, and a walk that is asked to stop partway through — none of which a fixture can
//! stand in for.

use super::content_search::search_session_content;
use crate::contexts::workspaces::application::{WorkspaceContentSearchRequest, MAX_SNIPPET_CHARS};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
use std::fs;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

struct Workspace {
    _directory: TempDirectory,
    database: NativeDatabase,
}

struct Answer {
    coverage: &'static str,
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
    fn search(&self, query: &str, cancelled: &Arc<AtomicBool>) -> Answer {
        let connection = self.database.connection().expect("connection");
        let result = search_session_content(
            &connection,
            "session-1",
            &WorkspaceContentSearchRequest {
                query: query.to_string(),
                search_id: "search-1".to_string(),
                limit: None,
            },
            cancelled,
        )
        .expect("search");
        Answer {
            coverage: result.coverage.state.token(),
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

    fn find(&self, query: &str) -> Answer {
        self.search(query, &Arc::new(AtomicBool::new(false)))
    }
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
    let cancelled = Arc::new(AtomicBool::new(true));

    let answer = workspace.search("needle", &cancelled);

    // Partial with a reason rather than an error: nothing went wrong, the reader stopped waiting,
    // and an error would put a failure notice on screen for something they did on purpose.
    assert!(answer.hits.is_empty());
    assert_eq!(answer.coverage, "partial");
}

/// What the search retains before it opens anything.
///
/// The walk that feeds a content search is the Quick Open walk with an empty query, and an empty
/// query matches every entry. Every eligible path in the workspace is therefore materialized into
/// one vector before the first file is read — memory proportional to the workspace, for an answer
/// bounded at 200 matches. Recorded as it is today so the streaming rewrite has something to
/// compare against rather than a claim.
#[test]
fn characterizes_a_candidate_vector_proportional_to_the_workspace() {
    let directory = TempDirectory::new("content-search-candidates");
    let root = directory.path().join("workspace");
    fs::create_dir_all(&root).expect("root");
    for index in 0..64 {
        fs::write(root.join(format!("file_{index}.txt")), b"nothing here\n").expect("file");
    }
    fs::write(root.join("hit.txt"), b"needle\n").expect("file");

    let (candidates, partial) = super::path_search::walk_workspace_paths(&root, "").expect("walk");

    // 65 files, one match, and the walk reports nothing was skipped. The vector is the workspace.
    assert_eq!(candidates.len(), 65);
    assert_eq!(partial, None);
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
