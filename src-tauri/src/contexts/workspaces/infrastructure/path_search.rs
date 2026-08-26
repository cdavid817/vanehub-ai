//! Finding a path by typing part of it.
//!
//! Separate from the mention-candidate search next door, and deliberately so. That one ranks
//! candidates for a prompt, which is why it filters to source extensions and skips directories —
//! exactly the right bounds for composing a message and exactly the wrong ones for a reader trying
//! to reach `package-lock.json`, a lockfile, or a folder. Widening it in place would have changed
//! what an `@` mention offers, which is a different feature nobody asked to change.
//!
//! What the two do share is the ordering rule and the traversal bounds, because a workspace should
//! not appear to have a different shape depending on which box you type into.
//!
//! Coverage and the cursor answer different questions and both are reported. The cursor says more
//! matches follow; coverage says some of the workspace was never examined. Paging fixes the first
//! and can never fix the second, so a reader who reached the end of the list still needs to be told
//! whether that was the end of the workspace.

use super::session_queries::resolve_session_root;
use crate::contexts::workspaces::application::{
    bounded_search_page, PathSearchCursor, WorkspaceApplicationError as AppError,
    WorkspacePathMatch, WorkspacePathSearchRequest, WorkspacePathSearchResult,
    WorkspaceSearchCoverage,
};
use crate::contexts::workspaces::domain::CanonicalPathBoundary;
use rusqlite::Connection;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

/// How deep the walk goes. The same bound the mention search uses: a source tree is deep, and
/// beyond ten levels a path is longer than the box a reader is typing into.
const SEARCH_DEPTH_LIMIT: usize = 10;

/// How many entries the walk will look at before it stops and says so.
///
/// A monorepo has millions of files and a reader is waiting on a keystroke. Stopping is not the
/// interesting part — reporting that it stopped is, because a truncated result that claims to be
/// complete is how somebody concludes a file does not exist.
const SEARCH_SCAN_LIMIT: usize = 20_000;

/// How many matches are collected before ranking, across all pages.
///
/// Bounded independently of the page size: ranking needs the whole candidate set to order it, so
/// this is the memory the search costs regardless of how few rows the caller asked for.
const MATCH_COLLECTION_LIMIT: usize = 2_000;

const SCORE_EXACT_NAME: u32 = 100;
const SCORE_NAME_PREFIX: u32 = 80;
const SCORE_NAME_SUBSTRING: u32 = 60;
const SCORE_PATH_SUBSTRING: u32 = 40;

/// Trees skipped because a reader is never trying to reach them by name.
///
/// The same list the mention search uses. `bin` is deliberately absent: Cargo treats `src/bin` as
/// real source, and a reader looking for a binary's entry point would find nothing.
const EXCLUDED_DIRECTORIES: &[&str] = &[
    "node_modules",
    "bower_components",
    "jspm_packages",
    "vendor",
    "dist",
    "build",
    "out",
    "target",
    "obj",
    "__pycache__",
    "venv",
    "site-packages",
    "coverage",
    "pods",
    "deriveddata",
];

pub(crate) struct Candidate {
    score: u32,
    depth: u32,
    name: String,
    pub(crate) path: String,
    pub(crate) kind: &'static str,
}

pub(crate) fn search_session_paths(
    conn: &Connection,
    session_id: &str,
    request: &WorkspacePathSearchRequest,
) -> Result<WorkspacePathSearchResult, AppError> {
    let normalized = normalize_query(&request.query);
    let cursor = match request.cursor.as_deref() {
        Some(encoded) => Some(
            PathSearchCursor::decode(encoded, &normalized)
                // A cursor for another query resumes at a rank this ordering never produced. Refused
                // as a validation failure so the caller starts the search again rather than
                // receiving a page from the middle of a different result set.
                .map_err(|_| {
                    AppError::Validation("Search cursor is not valid here.".to_string())
                })?,
        ),
        None => None,
    };
    let Some(root) = resolve_session_root(conn, session_id)? else {
        return Ok(WorkspacePathSearchResult {
            coverage: WorkspaceSearchCoverage::unavailable("workspace_search_root_unavailable"),
            matches: Vec::new(),
            next_cursor: None,
        });
    };

    let limit = bounded_search_page(request.limit);
    let (mut candidates, partial_reason) = walk_workspace_paths(&root, &normalized)?;

    candidates.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.depth.cmp(&right.depth))
            .then_with(|| left.path.to_lowercase().cmp(&right.path.to_lowercase()))
    });

    // Resumed after the sort, because the key the cursor holds is the key this ordering produces.
    // Filtering first would compare against an order that does not exist yet.
    let remaining: Vec<Candidate> = match &cursor {
        Some(cursor) => candidates
            .into_iter()
            .filter(|candidate| cursor.precedes(candidate.score, candidate.depth, &candidate.path))
            .collect(),
        None => candidates,
    };

    let has_more = remaining.len() > limit;
    let page: Vec<&Candidate> = remaining.iter().take(limit).collect();
    let next_cursor = has_more.then(|| {
        page.last().map(|last| {
            PathSearchCursor::after(&normalized, last.score, last.depth, &last.path).encode()
        })
    });

    Ok(WorkspacePathSearchResult {
        coverage: match partial_reason {
            Some(reason) => WorkspaceSearchCoverage::partial(reason),
            None => WorkspaceSearchCoverage::complete(),
        },
        matches: page
            .into_iter()
            .map(|candidate| WorkspacePathMatch {
                name: candidate.name.clone(),
                path: candidate.path.clone(),
                kind: candidate.kind,
            })
            .collect(),
        next_cursor: next_cursor.flatten(),
    })
}

/// Trimmed, lowercased, forward-slashed.
///
/// Normalized once here rather than at each comparison, because the cursor carries the query and
/// two spellings of the same search would produce two cursors that refuse each other.
pub(crate) fn normalize_query(query: &str) -> String {
    query.trim().to_ascii_lowercase().replace('\\', "/")
}

/// Every match, and why the walk stopped if it did.
///
/// Shared with the content search rather than duplicated there. Two traversals with their own
/// exclusion lists would let a file be findable by name and not by content, which reads as the
/// search being broken for that one file.
pub(crate) fn walk_workspace_paths(
    root: &Path,
    query: &str,
) -> Result<(Vec<Candidate>, Option<&'static str>), AppError> {
    // Entries are canonicalized before the containment check, so the root must be too: a short
    // (8.3) or symlinked root would otherwise fail every check and return nothing.
    let canonical_root = root
        .canonicalize()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let root = canonical_root.as_path();
    let boundary = CanonicalPathBoundary::new(root);
    let mut queue = VecDeque::from([(root.to_path_buf(), 0usize)]);
    let mut visited: HashSet<PathBuf> = HashSet::from([root.to_path_buf()]);
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut scanned = 0usize;
    let mut partial: Option<&'static str> = None;

    while let Some((directory, depth)) = queue.pop_front() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            // An unreadable root is a real failure; an unreadable subdirectory is a permission
            // quirk that must not fail the whole search — but it does make the answer partial.
            Err(error) if depth == 0 => return Err(AppError::Storage(error.to_string())),
            Err(_) => {
                partial.get_or_insert("workspace_search_directory_unreadable");
                continue;
            }
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let canonical = match entry.path().canonicalize() {
                Ok(value) if value.starts_with(root) => value,
                // A symlink pointing outside the root, or an entry that vanished mid-walk. Skipped
                // rather than reported: offering it would be offering a path the reader cannot open.
                _ => continue,
            };
            let is_directory = canonical.is_dir();
            scanned += 1;
            if scanned > SEARCH_SCAN_LIMIT {
                partial.get_or_insert("workspace_search_scan_limit");
                break;
            }
            let Ok(relative) = boundary.relative(&canonical) else {
                continue;
            };
            if is_directory {
                if is_excluded_directory(&name) {
                    continue;
                }
                if depth + 1 > SEARCH_DEPTH_LIMIT {
                    partial.get_or_insert("workspace_search_depth_limit");
                } else if visited.insert(canonical.clone()) {
                    queue.push_back((canonical.clone(), depth + 1));
                }
            }
            if candidates.len() >= MATCH_COLLECTION_LIMIT {
                partial.get_or_insert("workspace_search_match_limit");
                continue;
            }
            if let Some(score) = path_match_score(query, &name, &relative) {
                candidates.push(Candidate {
                    score,
                    depth: relative.matches('/').count() as u32,
                    name,
                    path: relative,
                    kind: if is_directory { "directory" } else { "file" },
                });
            }
        }
        if scanned > SEARCH_SCAN_LIMIT {
            partial.get_or_insert("workspace_search_scan_limit");
            break;
        }
    }
    Ok((candidates, partial))
}

fn is_excluded_directory(name: &str) -> bool {
    EXCLUDED_DIRECTORIES.contains(&name.to_ascii_lowercase().as_str())
}

/// How well a candidate answers the query. `None` excludes it.
///
/// Four tiers rather than a fuzzy distance. A reader typing `main` expects `main.rs` before
/// `domain/legacy.rs`, and a scoring function they cannot predict makes the top result feel
/// arbitrary — which is worse than one they disagree with but understand.
pub(crate) fn path_match_score(query: &str, name: &str, relative_path: &str) -> Option<u32> {
    if query.is_empty() {
        // An empty query browses. Everything qualifies, and breadth-first order plus the depth tie
        // break already puts the shallowest entries first.
        return Some(0);
    }
    let name = name.to_ascii_lowercase();
    let path = relative_path.to_ascii_lowercase();
    if name == query {
        return Some(SCORE_EXACT_NAME);
    }
    if name.starts_with(query) {
        return Some(SCORE_NAME_PREFIX);
    }
    if name.contains(query) {
        return Some(SCORE_NAME_SUBSTRING);
    }
    if path.contains(query) {
        return Some(SCORE_PATH_SUBSTRING);
    }
    None
}
