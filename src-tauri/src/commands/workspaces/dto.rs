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
    pub(crate) truncated: bool,
    pub(crate) next_cursor: Option<String>,
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
