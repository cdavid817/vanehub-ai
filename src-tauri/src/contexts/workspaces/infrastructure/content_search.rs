//! Reading files to find a string in them.
//!
//! In-process rather than through ripgrep, unlike the remote side. Requiring ripgrep locally would
//! make content search unavailable on most machines, and "install a tool to search your own files"
//! is not a remediation a desktop application gets to offer. The remote side uses ripgrep because
//! the alternative there is transferring the workspace.
//!
//! Two different engines is exactly why the matching rule is deliberately small: fixed-string,
//! case-insensitive. A pattern language is the one thing two engines cannot be made to agree about,
//! and a reader whose query means something different depending on which machine holds the files
//! has been handed a puzzle.
//!
//! The walk polls its cancellation flag between files rather than between directories. A single
//! large file is the case where a reader gives up waiting, and a check that only ran at directory
//! boundaries would keep reading it long after they had moved on.

use super::path_search::walk_workspace_paths;
use super::session_queries::resolve_session_root;
use crate::contexts::workspaces::application::{
    safe_snippet, WorkspaceApplicationError as AppError, WorkspaceContentMatch,
    WorkspaceContentSearchRequest, WorkspaceContentSearchResult, WorkspaceSearchCoverage,
    MAX_CONTENT_MATCHES, MAX_SEARCHED_FILE_BYTES,
};
use rusqlite::Connection;
use std::fs;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub(crate) fn search_session_content(
    conn: &Connection,
    session_id: &str,
    request: &WorkspaceContentSearchRequest,
    cancelled: &Arc<AtomicBool>,
) -> Result<WorkspaceContentSearchResult, AppError> {
    let needle = request.query.trim().to_lowercase();
    if needle.is_empty() {
        // An empty content query would match every line of every file. Refused as an answer rather
        // than as an error: there is nothing wrong with the request, there is simply nothing a
        // result list could usefully show.
        return Ok(WorkspaceContentSearchResult {
            coverage: WorkspaceSearchCoverage::complete(),
            matches: Vec::new(),
        });
    }
    let Some(root) = resolve_session_root(conn, session_id)? else {
        return Ok(WorkspaceContentSearchResult {
            coverage: WorkspaceSearchCoverage::unavailable("workspace_search_root_unavailable"),
            matches: Vec::new(),
        });
    };

    let limit = request
        .limit
        .unwrap_or(MAX_CONTENT_MATCHES)
        .clamp(1, MAX_CONTENT_MATCHES);
    // The same walk Quick Open uses, so the two agree about which trees exist. A second traversal
    // with its own exclusions would let a file be findable by name and not by content.
    let (candidates, walk_partial) = walk_workspace_paths(&root, "")?;

    let mut matches: Vec<WorkspaceContentMatch> = Vec::new();
    let mut partial = walk_partial;
    for candidate in candidates {
        if cancelled.load(Ordering::Relaxed) {
            return Ok(WorkspaceContentSearchResult {
                coverage: WorkspaceSearchCoverage::partial("workspace_search_cancelled"),
                matches,
            });
        }
        if candidate.kind != "file" {
            continue;
        }
        if matches.len() >= limit {
            partial.get_or_insert("workspace_search_match_limit");
            break;
        }
        let absolute = root.join(candidate.path.replace('/', std::path::MAIN_SEPARATOR_STR));
        match read_bounded(&absolute) {
            Ok(Some(content)) => {
                scan(&content, &needle, &candidate.path, limit, &mut matches);
            }
            // Binary, oversized, or unreadable. Skipped rather than reported per file: a result
            // list is not the place to enumerate what could not be looked at, and the coverage
            // flag already says the answer is not complete.
            Ok(None) => {
                partial.get_or_insert("workspace_search_files_skipped");
            }
            Err(_) => {
                partial.get_or_insert("workspace_search_files_skipped");
            }
        }
    }

    Ok(WorkspaceContentSearchResult {
        coverage: match partial {
            Some(reason) => WorkspaceSearchCoverage::partial(reason),
            None => WorkspaceSearchCoverage::complete(),
        },
        matches,
    })
}

/// The file's text, or nothing when it is not text this can search.
///
/// Decoded strictly. A lossy decode would produce replacement characters that match nothing and
/// offsets that no longer line up with the file, so a reader clicking the result would land on the
/// wrong column.
fn read_bounded(path: &std::path::Path) -> Result<Option<String>, AppError> {
    let metadata = fs::metadata(path).map_err(|error| AppError::Storage(error.to_string()))?;
    if metadata.len() > MAX_SEARCHED_FILE_BYTES {
        return Ok(None);
    }
    let file = fs::File::open(path).map_err(|error| AppError::Storage(error.to_string()))?;
    let mut raw = Vec::new();
    file.take(MAX_SEARCHED_FILE_BYTES)
        .read_to_end(&mut raw)
        .map_err(|error| AppError::Storage(error.to_string()))?;
    Ok(String::from_utf8(raw).ok())
}

/// Every match in one file, up to what is left of the budget.
///
/// One match per line. A line containing the query six times is one place to go, and six rows for
/// it would push five other files off a bounded list.
fn scan(
    content: &str,
    needle: &str,
    path: &str,
    limit: usize,
    matches: &mut Vec<WorkspaceContentMatch>,
) {
    for (index, line) in content.lines().enumerate() {
        if matches.len() >= limit {
            return;
        }
        let lowered = line.to_lowercase();
        let Some(byte_offset) = lowered.find(needle) else {
            continue;
        };
        // Counted in characters, not bytes. A byte column is meaningless to a reader looking at a
        // line with an accented character in it, and it is not what an editor would jump to.
        let column_chars = lowered[..byte_offset].chars().count();
        let (snippet, snippet_truncated, column) = safe_snippet(line, column_chars);
        matches.push(WorkspaceContentMatch {
            path: path.to_string(),
            line: index as u32 + 1,
            column,
            snippet,
            snippet_truncated,
        });
    }
}
