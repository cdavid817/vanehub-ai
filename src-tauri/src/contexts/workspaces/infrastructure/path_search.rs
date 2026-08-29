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
//!
//! Ranking is a bounded selection rather than a sort of everything. Without an index the walk still
//! has to *visit* every eligible entry to know which ones rank highest — that is a real cost, and
//! the entry budget is what bounds it — but it does not have to *keep* them. A heap of the best
//! `limit + 1` candidates answers the same question in memory proportional to the page rather than
//! to the workspace.

use super::bounded_selection::BoundedSelection;
use super::ignore_matcher::WorkspaceIgnoreMatcher;
use super::session_queries::resolve_session_root;
use crate::contexts::workspaces::application::{
    bounded_search_page, MonotonicClockPort, PathSearchCursor, SearchCancellationToken,
    SystemMonotonicClock, WorkspaceApplicationError as AppError, WorkspaceIgnorePolicy,
    WorkspaceInspectionBudget, WorkspaceInspectionBudgetLimits, WorkspaceInspectionReason,
    WorkspacePathMatch, WorkspacePathSearchRequest, WorkspacePathSearchResult,
    WorkspaceSearchCoverage,
};
use crate::contexts::workspaces::domain::CanonicalPathBoundary;
use rusqlite::Connection;
use std::cmp::Ordering;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// How deep the walk goes. The same bound the mention search uses: a source tree is deep, and
/// beyond ten levels a path is longer than the box a reader is typing into.
const SEARCH_DEPTH_LIMIT: u32 = 10;

const SCORE_EXACT_NAME: u32 = 100;
const SCORE_NAME_PREFIX: u32 = 80;
const SCORE_NAME_SUBSTRING: u32 = 60;
const SCORE_PATH_SUBSTRING: u32 = 40;

/// One entry the walk found, with the key the ordering compares.
///
/// The key is stored rather than recomputed. Lowercasing on every comparison would be the dominant
/// cost of a heap that sees every eligible entry in the workspace, and the two spellings would have
/// to agree exactly for a cursor to resume where the page ended.
struct RankedCandidate {
    score: u32,
    depth: u32,
    path_key: String,
    name: String,
    path: String,
    kind: &'static str,
}

impl RankedCandidate {
    /// The ordering, as one comparable value, smallest first.
    ///
    /// Score is negated rather than the comparison being reversed, so "better" is a plain `<`
    /// everywhere. A ranking expressed as a mix of ascending and descending comparisons is one
    /// somebody eventually gets backwards in exactly one of the places it appears.
    fn rank_key(&self) -> (i64, u32, &str) {
        (-(i64::from(self.score)), self.depth, self.path_key.as_str())
    }
}

impl PartialEq for RankedCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.rank_key() == other.rank_key()
    }
}

impl Eq for RankedCandidate {}

impl PartialOrd for RankedCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RankedCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank_key().cmp(&other.rank_key())
    }
}

pub(crate) fn search_session_paths(
    conn: &Connection,
    session_id: &str,
    request: &WorkspacePathSearchRequest,
    cancellation: &SearchCancellationToken,
) -> Result<WorkspacePathSearchResult, AppError> {
    search_session_paths_with(
        conn,
        session_id,
        request,
        WorkspaceInspectionBudgetLimits::path_search(),
        Arc::new(SystemMonotonicClock::default()),
        cancellation.clone(),
    )
}

/// The same search with its limits and its clock supplied.
///
/// The seam exists for tests and for nothing else. A budget dimension is only worth having if a
/// test can drive the traversal into it.
pub(super) fn search_session_paths_with(
    conn: &Connection,
    session_id: &str,
    request: &WorkspacePathSearchRequest,
    limits: WorkspaceInspectionBudgetLimits,
    clock: Arc<dyn MonotonicClockPort>,
    cancellation: SearchCancellationToken,
) -> Result<WorkspacePathSearchResult, AppError> {
    let normalized = normalize_query(&request.query);
    let cursor = match request.cursor.as_deref() {
        Some(encoded) => match PathSearchCursor::decode(encoded, &normalized) {
            Ok(cursor) => Some(cursor),
            // A cursor for another query resumes at a rank this ordering never produced. Answered
            // rather than raised: an error leaves the caller unable to tell "start this search
            // again" from "this workspace is unreachable", and only the first is something it can
            // act on. The empty page carries the reason so it can act on it.
            Err(_) => {
                return Ok(WorkspacePathSearchResult {
                    coverage: WorkspaceSearchCoverage::stopped(
                        WorkspaceInspectionReason::InvalidCursor,
                    ),
                    matches: Vec::new(),
                    next_cursor: None,
                })
            }
        },
        None => None,
    };
    let Some(root) = resolve_session_root(conn, session_id)? else {
        return Ok(WorkspacePathSearchResult {
            coverage: WorkspaceSearchCoverage::stopped(
                WorkspaceInspectionReason::ProviderUnavailable,
            ),
            matches: Vec::new(),
            next_cursor: None,
        });
    };

    let limit = bounded_search_page(request.limit);
    let capacity = limit + 1;
    let mut limits = limits;
    // The page, plus the one entry that proves another page exists. Stated as a budget rather than
    // only as the heap's capacity, so the bound is something a test reads off the answer instead of
    // something it has to take the implementation's word for.
    limits.max_retained_candidates = limits.max_retained_candidates.min(capacity as u64);
    limits.max_results = limits.max_results.min(limit as u64);
    let mut budget = WorkspaceInspectionBudget::new(limits, clock, cancellation);

    let selection =
        walk_ranked_candidates(&root, &normalized, cursor.as_ref(), capacity, &mut budget)?;
    let ranked = selection.into_sorted();

    let has_more = ranked.len() > limit;
    let mut matches: Vec<WorkspacePathMatch> = Vec::new();
    let mut last: Option<(u32, u32, String)> = None;
    for candidate in ranked.into_iter().take(limit) {
        if !budget.try_emit_result() {
            break;
        }
        last = Some((candidate.score, candidate.depth, candidate.path.clone()));
        matches.push(WorkspacePathMatch {
            name: candidate.name,
            path: candidate.path,
            kind: candidate.kind,
        });
    }
    // A cursor only when there is more. Issuing one for an exhausted result set would invite a
    // caller to fetch a page that is always empty.
    let next_cursor = match (has_more, last) {
        (true, Some((score, depth, path))) => {
            Some(PathSearchCursor::after(&normalized, score, depth, &path).encode())
        }
        _ => None,
    };

    let coverage = match budget.incomplete_reason() {
        Some(reason) => WorkspaceSearchCoverage::stopped(reason),
        None => WorkspaceSearchCoverage::complete(),
    };
    Ok(WorkspacePathSearchResult {
        coverage: coverage.with_budget(budget.snapshot()),
        matches,
        next_cursor,
    })
}

/// Trimmed, lowercased, forward-slashed.
///
/// Normalized once here rather than at each comparison, because the cursor carries the query and
/// two spellings of the same search would produce two cursors that refuse each other.
pub(crate) fn normalize_query(query: &str) -> String {
    query.trim().to_ascii_lowercase().replace('\\', "/")
}

/// One breadth-first pass that keeps only the best `capacity` matches it has seen.
///
/// Every visited entry is charged whether or not it matches. Those are the entries a result cap
/// never sees, and on the trees where any of this matters they are most of the work.
fn walk_ranked_candidates(
    root: &Path,
    query: &str,
    cursor: Option<&PathSearchCursor>,
    capacity: usize,
    budget: &mut WorkspaceInspectionBudget,
) -> Result<BoundedSelection<RankedCandidate>, AppError> {
    // Entries are canonicalized before the containment check, so the root must be too: a short
    // (8.3) or symlinked root would otherwise fail every check and return nothing.
    let canonical_root = root
        .canonicalize()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let root = canonical_root.as_path();
    let boundary = CanonicalPathBoundary::new(root);
    let ignores =
        WorkspaceIgnoreMatcher::for_root(root, WorkspaceIgnorePolicy::recursive_discovery());
    let mut queue: VecDeque<(PathBuf, u32)> = VecDeque::from([(root.to_path_buf(), 0u32)]);
    let mut visited: HashSet<PathBuf> = HashSet::from([root.to_path_buf()]);
    let mut selection = BoundedSelection::new(capacity);

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
            let is_directory = canonical.is_dir();
            let Ok(relative) = boundary.relative(&canonical) else {
                continue;
            };
            if ignores.skips(&relative, &name, is_directory) {
                continue;
            }
            if is_directory {
                if depth + 1 > SEARCH_DEPTH_LIMIT {
                    // A per-branch refusal, not a reason to abandon the walk: the entries beside
                    // this one are still in scope, and the coverage records the omission.
                    budget.note_omission(WorkspaceInspectionReason::DepthBudgetExhausted);
                } else if visited.insert(canonical.clone()) {
                    if !budget.try_descend(depth + 1) {
                        break;
                    }
                    queue.push_back((canonical, depth + 1));
                }
            }
            let Some(score) = path_match_score(query, &name, &relative) else {
                continue;
            };
            let candidate = RankedCandidate {
                score,
                depth: relative.matches('/').count() as u32,
                path_key: relative.to_lowercase(),
                name,
                path: relative,
                kind: if is_directory { "directory" } else { "file" },
            };
            // Resumed here rather than after ranking. A cursor names a position in this ordering,
            // and one candidate's place in it does not depend on the rest of the set — so the
            // comparison is as valid now as it would be after a full sort, and it keeps the
            // selection to the page actually being asked for.
            if cursor
                .is_some_and(|cursor| !cursor.precedes(score, candidate.depth, &candidate.path))
            {
                continue;
            }
            if !selection.offer(candidate, budget) {
                break;
            }
        }
        if budget.is_stopped() {
            break;
        }
    }
    Ok(selection)
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
