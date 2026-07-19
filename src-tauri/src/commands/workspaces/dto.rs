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
    pub(crate) cursor: Option<String>,
    pub(crate) limit: Option<usize>,
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
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogExportResult {
    pub(crate) status: &'static str,
    pub(crate) path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateShellInput {
    pub(crate) session_id: String,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResizeShellInput {
    pub(crate) shell_id: String,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShellSession {
    pub(crate) shell_id: String,
    pub(crate) session_id: String,
    pub(crate) state: &'static str,
    pub(crate) capability: &'static str,
}
