#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KnownProject {
    pub(crate) path: String,
    pub(crate) display_name: String,
    pub(crate) is_git: bool,
    pub(crate) last_opened_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KnownRemoteWorkspace {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: Option<String>,
    pub(crate) path: String,
    pub(crate) display_name: String,
    pub(crate) uri: String,
    pub(crate) last_opened_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatedWorktree {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) branch: String,
    /// The managed record created before Git ran, when this worktree is provenance-tracked.
    /// `None` for Loop worktrees and for assemblies without the cleanup service.
    pub(crate) worktree_id: Option<String>,
}

/// A worktree creation that has been validated but not yet run: every path is settled before
/// any intent is recorded or any Git command executes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedWorktree {
    pub(crate) project: String,
    pub(crate) target: String,
    pub(crate) name: String,
    pub(crate) branch: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitBranchReference {
    pub(crate) name: String,
    pub(crate) kind: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionWorkspaceContext {
    pub(crate) availability: &'static str,
    pub(crate) root_name: Option<String>,
    pub(crate) reason: Option<String>,
}

impl SessionWorkspaceContext {
    pub(crate) fn available(root_name: Option<String>) -> Self {
        Self {
            availability: "available",
            root_name,
            reason: None,
        }
    }

    pub(crate) fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            availability: "unavailable",
            root_name: None,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectoryEntry {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) kind: &'static str,
    pub(crate) size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectoryListing {
    pub(crate) context: SessionWorkspaceContext,
    pub(crate) path: String,
    pub(crate) items: Vec<DirectoryEntry>,
    /// Whether another page follows. Nothing more than that.
    pub(crate) truncated: bool,
    pub(crate) next_cursor: Option<String>,
    /// How much of the directory the scan actually saw.
    ///
    /// Separate from `truncated`, and the separation is the point: one says "ask for the next page",
    /// the other says "some of this folder was never examined, and paging will not reach it". A
    /// reader who was shown only the first reads a stopped scan as the end of the directory.
    pub(crate) coverage: super::inspection::WorkspaceSearchCoverage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionDocument {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) kind: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocumentListing {
    pub(crate) context: SessionWorkspaceContext,
    pub(crate) items: Vec<SessionDocument>,
    /// Whether the document limit was reached, so more documents exist than are listed.
    pub(crate) truncated: bool,
    pub(crate) next_cursor: Option<String>,
    /// How much of the project the walk actually reached.
    ///
    /// Separate from `truncated`, and for the same reason it is separate on a directory listing: one
    /// says the list was cut at its own ceiling, the other says the walk never got to part of the
    /// tree. A reader shown only the first reads a stopped walk as a project with fewer documents.
    pub(crate) coverage: super::inspection::WorkspaceSearchCoverage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSearchMatch {
    pub(crate) name: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSearchListing {
    pub(crate) context: SessionWorkspaceContext,
    pub(crate) items: Vec<FileSearchMatch>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileContent {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) status: &'static str,
    pub(crate) size: u64,
    pub(crate) content: Option<String>,
    /// `utf-8` or `utf-8-bom`, and absent for anything that is not text.
    ///
    /// Absent rather than defaulted: a binary file has no encoding this application established,
    /// and reporting one would be describing a decode that never happened.
    pub(crate) encoding: Option<&'static str>,
    /// `lf`, `crlf`, `mixed`, or `none`. Absent for anything that is not text.
    pub(crate) newline: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitStatusEntry {
    pub(crate) path: String,
    pub(crate) previous_path: Option<String>,
    pub(crate) index: String,
    pub(crate) worktree: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitStatusResult {
    pub(crate) context: SessionWorkspaceContext,
    pub(crate) is_git: bool,
    pub(crate) branch: Option<String>,
    pub(crate) items: Vec<GitStatusEntry>,
    pub(crate) truncated: bool,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GitDiffSource {
    Working,
    Staged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitDiffLine {
    pub(crate) kind: String,
    pub(crate) content: String,
    pub(crate) old_line_number: Option<usize>,
    pub(crate) new_line_number: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitDiffHunk {
    pub(crate) header: String,
    pub(crate) old_start: usize,
    pub(crate) old_lines: usize,
    pub(crate) new_start: usize,
    pub(crate) new_lines: usize,
    pub(crate) lines: Vec<GitDiffLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitDiffFile {
    pub(crate) old_path: Option<String>,
    pub(crate) new_path: String,
    pub(crate) binary: bool,
    pub(crate) oversized: bool,
    pub(crate) hunks: Vec<GitDiffHunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitDiffResult {
    pub(crate) context: SessionWorkspaceContext,
    pub(crate) source: GitDiffSource,
    pub(crate) files: Vec<GitDiffFile>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionLogQuery {
    pub(crate) session_id: String,
    pub(crate) levels: Vec<WorkspaceLogLevel>,
    pub(crate) search: String,
    /// `None` means every seat. A concrete seat matches only records carrying that correlation;
    /// a record written without one is not attributed to whichever seat happens to be selected.
    pub(crate) seat_id: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionLogEntry {
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) level: WorkspaceLogLevel,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) context: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionLogPage {
    pub(crate) items: Vec<SessionLogEntry>,
    pub(crate) truncated: bool,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionLogExportResult {
    pub(crate) status: &'static str,
    pub(crate) path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateShellRequest {
    pub(crate) session_id: String,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
    /// Which participant asked for the shell. `None` in a single-seat session.
    pub(crate) seat_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResizeShellRequest {
    pub(crate) shell_id: String,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellWorkspace {
    pub(crate) agent_id: String,
    pub(crate) root: Option<String>,
    pub(crate) remote: bool,
    pub(crate) remote_endpoint: Option<ShellRemoteEndpoint>,
    pub(crate) ssh_binding: Option<ShellSshBinding>,
    pub(crate) policy: ShellWorkspacePolicy,
    pub(crate) read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellRemoteEndpoint {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) path: String,
    pub(crate) display_name: String,
    pub(crate) uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellSshBinding {
    pub(crate) connection_id: String,
    pub(crate) revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellWorkspacePolicy {
    pub(crate) requires_host_trust: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellLaunch {
    pub(crate) shell_id: String,
    pub(crate) session_id: String,
    pub(crate) root: String,
    pub(crate) dimensions: TerminalDimensions,
    pub(crate) remote_endpoint: Option<ShellRemoteEndpoint>,
    pub(crate) ssh_binding: Option<ShellSshBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellSession {
    pub(crate) shell_id: String,
    pub(crate) session_id: String,
    pub(crate) state: &'static str,
    pub(crate) runtime: ShellRuntimeDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShellEvent {
    Output {
        shell_id: String,
        session_id: String,
        content: String,
    },
    State {
        shell_id: String,
        session_id: String,
        state: &'static str,
        error: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellLog {
    pub(crate) level: WorkspaceLogLevel,
    pub(crate) session_id: String,
    pub(crate) shell_id: String,
    /// Present only where the caller genuinely knows the owning seat. The registry does not track
    /// it yet, so a lifecycle log raised from the runtime leaves it absent rather than guessing.
    pub(crate) seat_id: Option<String>,
    pub(crate) message: String,
}
use crate::contexts::workspaces::domain::{ShellRuntimeDescriptor, TerminalDimensions};
