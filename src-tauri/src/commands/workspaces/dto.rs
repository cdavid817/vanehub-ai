use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnownProject {
    pub(crate) path: String,
    pub(crate) display_name: String,
    pub(crate) is_git: bool,
    pub(crate) last_opened_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectInspection {
    pub(crate) path: String,
    pub(crate) display_name: String,
    pub(crate) is_git: bool,
    pub(crate) git_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KnownRemoteWorkspace {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: Option<String>,
    pub(crate) path: String,
    pub(crate) display_name: String,
    pub(crate) uri: String,
    pub(crate) last_opened_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionWorkspaceContext {
    pub(crate) availability: &'static str,
    pub(crate) root_name: Option<String>,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryEntry {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) kind: &'static str,
    pub(crate) size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryListing {
    pub(crate) context: SessionWorkspaceContext,
    pub(crate) path: String,
    pub(crate) items: Vec<DirectoryEntry>,
    /// Whether another page follows. Nothing more than that.
    pub(crate) truncated: bool,
    pub(crate) next_cursor: Option<String>,
    /// How much of the directory the scan actually saw.
    ///
    /// Separate from `truncated`, because one says "ask for the next page" and the other says "some
    /// of this folder was never examined, and paging will not reach it". This is also where a
    /// refused cursor arrives: `invalid_cursor` for a token that does not belong to this listing,
    /// `stale_cursor` for one that did until the directory changed.
    pub(crate) coverage: WorkspaceSearchCoverageDto,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionDocument {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) kind: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentListing {
    pub(crate) context: SessionWorkspaceContext,
    pub(crate) items: Vec<SessionDocument>,
    pub(crate) truncated: bool,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileSearchMatch {
    pub(crate) name: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileSearchListing {
    pub(crate) context: SessionWorkspaceContext,
    pub(crate) items: Vec<FileSearchMatch>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileContent {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) status: &'static str,
    pub(crate) size: u64,
    pub(crate) content: Option<String>,
    /// `utf-8` or `utf-8-bom`. Omitted for anything that is not text, because a binary file has
    /// no encoding this application established and naming one would describe a decode that never
    /// happened.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) encoding: Option<&'static str>,
    /// `lf`, `crlf`, `mixed`, or `none`. Omitted for anything that is not text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) newline: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitStatusEntry {
    pub(crate) path: String,
    pub(crate) previous_path: Option<String>,
    pub(crate) index: String,
    pub(crate) worktree: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitStatusResult {
    pub(crate) context: SessionWorkspaceContext,
    pub(crate) is_git: bool,
    pub(crate) branch: Option<String>,
    pub(crate) items: Vec<GitStatusEntry>,
    pub(crate) truncated: bool,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GitDiffSource {
    Working,
    Staged,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitDiffLine {
    pub(crate) kind: String,
    pub(crate) content: String,
    pub(crate) old_line_number: Option<usize>,
    pub(crate) new_line_number: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitDiffHunk {
    pub(crate) header: String,
    pub(crate) old_start: usize,
    pub(crate) old_lines: usize,
    pub(crate) new_start: usize,
    pub(crate) new_lines: usize,
    pub(crate) lines: Vec<GitDiffLine>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitDiffFile {
    pub(crate) old_path: Option<String>,
    pub(crate) new_path: String,
    pub(crate) binary: bool,
    pub(crate) oversized: bool,
    pub(crate) hunks: Vec<GitDiffHunk>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitDiffResult {
    pub(crate) context: SessionWorkspaceContext,
    pub(crate) source: GitDiffSource,
    pub(crate) files: Vec<GitDiffFile>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum WorkspaceLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogQuery {
    pub(crate) session_id: String,
    pub(crate) levels: Vec<WorkspaceLogLevel>,
    pub(crate) search: String,
    #[serde(default)]
    pub(crate) seat_id: Option<String>,
    /// The correlations a reader can narrow by, all optional and all absent by default.
    ///
    /// The index has filtered on these since it existed; only the wire shape was missing them, so
    /// the Logs tab could not ask. Each one narrows to records that carry that correlation — a
    /// record emitted without one is not attributed to whichever value happens to be selected.
    #[serde(default)]
    pub(crate) run_id: Option<String>,
    #[serde(default)]
    pub(crate) trace_id: Option<String>,
    #[serde(default)]
    pub(crate) span_id: Option<String>,
    #[serde(default)]
    pub(crate) operation_id: Option<String>,
    #[serde(default)]
    pub(crate) agent_id: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) limit: Option<usize>,
    /// Which end of the corpus to read from. Absent means newest first, which is what the Logs tab
    /// has always asked for, so a client that predates the field is unchanged.
    #[serde(default)]
    pub(crate) sort: Option<SessionLogSortDto>,
}

/// The page order a client may ask for.
///
/// Part of the request rather than a constant because it decides which rows are "after" a cursor:
/// a cursor issued in one direction names the opposite boundary in the other, and every row on the
/// wrong side of it would silently vanish. That is why it is in the filter fingerprint too.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SessionLogSortDto {
    NewestFirst,
    OldestFirst,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogEntry {
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) level: WorkspaceLogLevel,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) context: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogPage {
    pub(crate) items: Vec<SessionLogEntry>,
    pub(crate) truncated: bool,
    pub(crate) next_cursor: Option<String>,
    /// What the index can honestly claim about this page.
    ///
    /// Additive: a client that predates the field keeps working, and one that reads it stops
    /// rendering an incomplete answer as a definitive one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) coverage: Option<super::session_log_mapper::SessionLogCoverageDto>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogExportResult {
    pub(crate) status: &'static str,
    pub(crate) path: Option<String>,
}

/// Serialized as an externally tagged discriminated union so the frontend narrows on `kind`
/// rather than on a bare capability string it has to widen by hand.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ShellRuntimeDescriptor {
    #[serde(rename_all = "camelCase")]
    Native {
        supports_resize: bool,
        supports_replay: bool,
        supports_reconnect: bool,
    },
    #[serde(rename_all = "camelCase")]
    Remote {
        connection_id: String,
        profile_revision: i64,
        supports_resize: bool,
        supports_replay: bool,
        supports_reconnect: bool,
    },
    #[serde(rename_all = "camelCase")]
    Simulated {
        supports_resize: bool,
        supports_replay: bool,
        supports_reconnect: bool,
    },
    #[serde(rename_all = "camelCase")]
    Unavailable {
        reason_code: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        remediation: Option<String>,
    },
}

/// What a session's workspace can be asked, and which machine it is on.
///
/// Every capability carries its own state rather than the payload carrying one overall flag: a
/// remote host with Git but no ripgrep is a normal situation, and a single flag would either hide
/// the search gap or disable the four things that work.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceInspectionCapabilitiesDto {
    pub(crate) provider: String,
    /// Absent for a local workspace, which is what a reader assumes when nothing says otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) target_label: Option<String>,
    pub(crate) list_files: CapabilityStateDto,
    pub(crate) read_text_files: CapabilityStateDto,
    pub(crate) search_files: CapabilityStateDto,
    pub(crate) git_status: CapabilityStateDto,
    pub(crate) git_diff: CapabilityStateDto,
    pub(crate) watch_mode: String,
}

/// One page of Quick Open results.
///
/// `coverage` and `next_cursor` answer different questions and both are here. The cursor says more
/// matches follow; coverage says part of the workspace was never examined. A reader who paged to
/// the end still needs to know whether that was the end of the workspace, and one field cannot say
/// both.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspacePathSearchDto {
    pub(crate) coverage: WorkspaceSearchCoverageDto,
    pub(crate) matches: Vec<WorkspacePathMatchDto>,
    /// Absent when this page is the last one, never an empty string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_cursor: Option<String>,
}

/// Positions inside files.
///
/// No cursor here, unlike Quick Open. Content search is bounded by a match count rather than paged:
/// a reader who gets two hundred hits narrows the query, they do not page through them, and a
/// cursor would have to survive a file changing underneath it between pages.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceContentSearchDto {
    /// Which registration under the search id answered.
    ///
    /// Carried so the frontend can drop an answer that is older than the one already on screen. Two
    /// requests can be in flight at once and arrival order is not issue order, so "the last response
    /// wins" is the rule that puts a stale result over a fresh one.
    pub(crate) generation: u64,
    pub(crate) coverage: WorkspaceSearchCoverageDto,
    pub(crate) matches: Vec<WorkspaceContentMatchDto>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceContentMatchDto {
    /// Workspace-relative, with forward slashes.
    pub(crate) path: String,
    /// 1-based, because that is what every editor and every error message uses.
    pub(crate) line: u32,
    /// 1-based and counted in characters, not bytes: a byte column is meaningless on a line with an
    /// accented character in it, and it is not what an editor would jump to.
    pub(crate) column: u32,
    /// A bounded, control-free slice of the matching line, never the whole file.
    pub(crate) snippet: String,
    /// Whether the line was cut to fit. Separate from the search's own bound, because a complete
    /// result made of trimmed lines is still complete.
    pub(crate) snippet_truncated: bool,
}

/// What is retained about one file.
///
/// `observations` counts recorded changes and is what decides whether an action is worth offering
/// at all. Zero is a real answer and the common one: most files in a workspace were never touched
/// by an agent, and a link that led to nothing would be worse than no link.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileEvidenceLinksDto {
    pub(crate) observations: u32,
    pub(crate) run_ids: Vec<String>,
    pub(crate) command_ids: Vec<String>,
    /// Whether more identifiers exist than are listed. Distinct from `observations`, which counts
    /// events rather than distinct runs.
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceSearchCoverageDto {
    /// `complete`, `partial`, or `unavailable`.
    pub(crate) state: String,
    /// Why it is not complete, as a token the frontend translates. Absent when it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason_code: Option<String>,
    /// What the inspection spent, when it was accounted.
    ///
    /// Absent means "not accounted", never "spent nothing". A remote provider counts on the other
    /// machine and a fixture counts nothing at all, and zeroes here would claim a scan that never
    /// happened.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) budget: Option<WorkspaceInspectionBudgetDto>,
}

/// What one inspection actually spent.
///
/// Counters rather than limits. A reader is being told why an answer stopped, and "read 1 MB" is
/// something they can weigh against the workspace they are looking at; the configured ceiling that
/// produced it is this build's business.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceInspectionBudgetDto {
    pub(crate) directories_visited: u64,
    pub(crate) entries_visited: u64,
    pub(crate) files_opened: u64,
    pub(crate) bytes_read: u64,
    pub(crate) metadata_operations: u64,
    pub(crate) candidates_retained: u64,
    pub(crate) results_emitted: u64,
    pub(crate) max_depth_reached: u32,
    /// Entries that were eligible and could not be read.
    pub(crate) unreadable_entries: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspacePathMatchDto {
    pub(crate) name: String,
    /// Workspace-relative, with forward slashes. An absolute path would put this machine's layout
    /// into a list a reader can copy from.
    pub(crate) path: String,
    /// `file` or `directory`. A reader acts on them differently: one opens a preview, the other
    /// reveals a folder in the tree.
    pub(crate) kind: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CapabilityStateDto {
    pub(crate) available: bool,
    /// A stable token. The frontend owns the sentence; a message built in Rust would arrive in
    /// whatever language this build's sources happen to be written in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason_code: Option<String>,
    /// What would fix it, also as a token. "Search is unavailable" and "install ripgrep on the
    /// remote host" are different facts, and only the second is actionable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) remediation: Option<String>,
}

/// A refusal the frontend translates, never a message.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceInspectionErrorDto {
    pub(crate) reason_code: String,
}
