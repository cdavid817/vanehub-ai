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
//! The traversal is a stream, not two passes. It used to borrow Quick Open's walk, take the whole
//! candidate vector it returned, and only then start opening files — memory proportional to the
//! workspace for an answer bounded at two hundred matches, and no cancellation check between the
//! first directory and the last. Now each file is opened as it is reached, and the only things held
//! at once are the breadth-first frontier, one file, and the bounded result list.
//!
//! Everything is charged to a budget before it happens: the directory before it is enumerated, the
//! entry before it is examined, the stat before it is taken, the file before it is opened, and each
//! chunk before it is read. That is what makes a search over a generated tree stop and say so,
//! rather than run until the reader closes the window.

use crate::contexts::workspaces::application::{
    safe_snippet, MonotonicClockPort, SearchCancellationToken, SystemMonotonicClock,
    WorkspaceApplicationError as AppError, WorkspaceContentMatch, WorkspaceContentSearchRequest,
    WorkspaceContentSearchResult, WorkspaceInspectionBudget, WorkspaceInspectionBudgetLimits,
    WorkspaceInspectionReason, WorkspaceSearchCoverage, MAX_CONTENT_MATCHES,
    MAX_SEARCHED_FILE_BYTES,
};
use crate::contexts::workspaces::domain::CanonicalPathBoundary;
use rusqlite::Connection;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// How much of a file is read per checkpoint.
///
/// The unit in which cancellation is observed *inside* one file. A single large file is the case
/// where a reader gives up waiting, and a check that only ran between files would keep reading it
/// long after they had moved on. 64 KiB is sixteen checkpoints across the per-file ceiling, which
/// is frequent enough to feel immediate and coarse enough not to matter to throughput.
const READ_CHUNK_BYTES: usize = 64 * 1024;

/// Trees skipped because a reader searching their own code is not asking about them.
///
/// The same list Quick Open uses, and identical to it deliberately: two traversals with their own
/// exclusion lists would let a file be findable by name and not by content, which reads as the
/// search being broken for that one file.
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

pub(crate) fn search_session_content(
    conn: &Connection,
    session_id: &str,
    request: &WorkspaceContentSearchRequest,
    cancellation: &SearchCancellationToken,
) -> Result<WorkspaceContentSearchResult, AppError> {
    search_session_content_with(
        conn,
        session_id,
        request,
        cancellation,
        WorkspaceInspectionBudgetLimits::content_search(),
        Arc::new(SystemMonotonicClock::default()),
    )
}

/// The same search with its limits and its clock supplied.
///
/// The seam exists for tests and for nothing else. A budget dimension is only worth having if a
/// test can drive the traversal into it, and a deadline is only worth having if one can be reached
/// without waiting twenty seconds for it.
pub(super) fn search_session_content_with(
    conn: &Connection,
    session_id: &str,
    request: &WorkspaceContentSearchRequest,
    cancellation: &SearchCancellationToken,
    limits: WorkspaceInspectionBudgetLimits,
    clock: Arc<dyn MonotonicClockPort>,
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
    let Some(root) = super::session_queries::resolve_session_root(conn, session_id)? else {
        return Ok(WorkspaceContentSearchResult {
            coverage: WorkspaceSearchCoverage::stopped(
                WorkspaceInspectionReason::ProviderUnavailable,
            ),
            matches: Vec::new(),
        });
    };

    let limit = request
        .limit
        .unwrap_or(MAX_CONTENT_MATCHES)
        .clamp(1, MAX_CONTENT_MATCHES);
    let mut limits = limits;
    // A caller asking for fewer matches is asking for less work, not merely a shorter list. Leaving
    // the result budget at its ceiling would keep opening files after the answer was complete.
    limits.max_results = limits.max_results.min(limit as u64);
    let mut budget = WorkspaceInspectionBudget::new(limits, clock, cancellation.clone());

    let matches = stream_search(&root, &needle, &mut budget)?;

    // An omission counts as much as a stop: something the walk skipped and continued past is
    // still something a reader was not shown, and reporting complete is how somebody concludes a
    // string is not in their workspace.
    let coverage = match budget.incomplete_reason() {
        Some(reason) => WorkspaceSearchCoverage::stopped(reason),
        None => WorkspaceSearchCoverage::complete(),
    };
    Ok(WorkspaceContentSearchResult {
        coverage: coverage.with_budget(budget.snapshot()),
        matches,
    })
}

/// One breadth-first pass that opens files as it reaches them.
///
/// The frontier is charged as retained candidates and credited when a directory is popped, so the
/// queue is bounded rather than merely finite: a tree wide enough to hold more unvisited
/// directories at once than the budget allows stops and says so.
fn stream_search(
    root: &Path,
    needle: &str,
    budget: &mut WorkspaceInspectionBudget,
) -> Result<Vec<WorkspaceContentMatch>, AppError> {
    // Entries are canonicalized before the containment check, so the root must be too: a short
    // (8.3) or symlinked root would otherwise fail every check and return nothing.
    let canonical_root = root
        .canonicalize()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let root = canonical_root.as_path();
    let boundary = CanonicalPathBoundary::new(root);
    let mut matches: Vec<WorkspaceContentMatch> = Vec::new();
    let mut queue: VecDeque<(PathBuf, u32)> = VecDeque::from([(root.to_path_buf(), 0u32)]);
    let mut visited: HashSet<PathBuf> = HashSet::from([root.to_path_buf()]);

    while let Some((directory, depth)) = queue.pop_front() {
        if !budget.try_visit_directory() {
            break;
        }
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            // An unreadable root is a real failure; an unreadable subdirectory is a permission
            // quirk that must not fail the whole search — but it does make the answer partial.
            Err(error) if depth == 0 => return Err(AppError::Storage(error.to_string())),
            Err(_) => {
                budget.note_omission(WorkspaceInspectionReason::UnreadableEntries);
                continue;
            }
        };
        for entry in entries.flatten() {
            if !budget.try_visit_entry() {
                break;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if !budget.try_metadata() {
                break;
            }
            let canonical = match entry.path().canonicalize() {
                Ok(value) if value.starts_with(root) => value,
                // A symlink pointing outside the root, or an entry that vanished mid-walk. Skipped
                // rather than reported: offering it would be offering a path the reader cannot open.
                _ => continue,
            };
            if !budget.try_metadata() {
                break;
            }
            if canonical.is_dir() {
                if is_excluded_directory(&name) || !visited.insert(canonical.clone()) {
                    continue;
                }
                if depth + 1 > DEPTH_LIMIT {
                    // A per-branch refusal, not a reason to abandon the walk: the entries beside
                    // this one are still in scope, and the coverage records the omission.
                    budget.note_omission(WorkspaceInspectionReason::DepthBudgetExhausted);
                    continue;
                }
                if !budget.try_descend(depth + 1) || !budget.try_retain_candidate() {
                    break;
                }
                queue.push_back((canonical, depth + 1));
                continue;
            }
            let Ok(relative) = boundary.relative(&canonical) else {
                continue;
            };
            match read_bounded(&canonical, budget) {
                Ok(Some(content)) => scan(&content, needle, &relative, budget, &mut matches),
                // Binary, oversized, or unreadable. Skipped rather than reported per file: a result
                // list is not the place to enumerate what could not be looked at, and the coverage
                // already says the answer is not complete.
                Ok(None) => {
                    // A budget stop already explains itself; counting it as a skipped file too
                    // would report a reader's own cancellation as an unreadable workspace.
                    if !budget.is_stopped() {
                        budget.note_omission(WorkspaceInspectionReason::UnreadableEntries);
                    }
                }
                Err(_) => budget.note_omission(WorkspaceInspectionReason::UnreadableEntries),
            }
            if budget.is_stopped() {
                break;
            }
        }
        budget.release_candidate();
        if budget.is_stopped() {
            break;
        }
    }

    Ok(matches)
}

/// How deep the walk goes.
///
/// Mirrors the budget's depth ceiling so the branch above can refuse one subtree without consuming
/// the shared depth accounting for the entries beside it.
const DEPTH_LIMIT: u32 = 10;

fn is_excluded_directory(name: &str) -> bool {
    EXCLUDED_DIRECTORIES.contains(&name.to_ascii_lowercase().as_str())
}

/// The file's text, or nothing when it is not text this can search.
///
/// Decoded strictly. A lossy decode would produce replacement characters that match nothing and
/// offsets that no longer line up with the file, so a reader clicking the result would land on the
/// wrong column.
///
/// Read in chunks so cancellation is observed inside a large file rather than only between files,
/// and so each chunk is charged against the aggregate byte budget before it is requested. Charging
/// what came back instead would let a file growing under the reader run past the ceiling one short
/// read at a time.
fn read_bounded(
    path: &Path,
    budget: &mut WorkspaceInspectionBudget,
) -> Result<Option<String>, AppError> {
    if !budget.try_metadata() {
        return Ok(None);
    }
    let metadata = fs::metadata(path).map_err(|error| AppError::Storage(error.to_string()))?;
    if metadata.len() > MAX_SEARCHED_FILE_BYTES {
        return Ok(None);
    }
    if !budget.try_open_file() {
        return Ok(None);
    }
    let file = fs::File::open(path).map_err(|error| AppError::Storage(error.to_string()))?;
    let mut reader = file.take(MAX_SEARCHED_FILE_BYTES);
    let mut raw: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; READ_CHUNK_BYTES];
    loop {
        if !budget.try_read_bytes(READ_CHUNK_BYTES as u64) {
            return Ok(None);
        }
        let read = reader
            .read(&mut chunk)
            .map_err(|error| AppError::Storage(error.to_string()))?;
        if read == 0 {
            break;
        }
        raw.extend_from_slice(&chunk[..read]);
    }
    Ok(String::from_utf8(raw).ok())
}

/// Every match in one file, up to what is left of the result budget.
///
/// One match per line. A line containing the query six times is one place to go, and six rows for
/// it would push five other files off a bounded list.
fn scan(
    content: &str,
    needle: &str,
    path: &str,
    budget: &mut WorkspaceInspectionBudget,
    matches: &mut Vec<WorkspaceContentMatch>,
) {
    for (index, line) in content.lines().enumerate() {
        let lowered = line.to_lowercase();
        let Some(byte_offset) = lowered.find(needle) else {
            continue;
        };
        if !budget.try_emit_result() {
            return;
        }
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
