use super::content_search::{WorkspaceContentSearchRequest, WorkspaceContentSearchResult};
use super::inspection::{
    DirectoryFingerprint, WorkspacePathSearchRequest, WorkspacePathSearchResult,
};
use super::{
    DirectoryListing, DocumentListing, FileContent, FileSearchListing, GitBranchReference,
    GitDiffResult, GitDiffSource, GitStatusResult, KnownProject, KnownRemoteWorkspace,
    SessionLogExportResult, SessionLogPage, SessionLogQuery, ShellLog, ShellWorkspace,
    WorkspaceApplicationError,
};
use crate::contexts::workspaces::domain::{
    ProjectInspection, ProjectPath, RemoteWorkspace, WorktreeName,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) trait WorkspaceHistoryRepository: Send + Sync {
    fn list_projects(&self) -> Result<Vec<KnownProject>, WorkspaceApplicationError>;

    fn list_remote_workspaces(
        &self,
    ) -> Result<Vec<KnownRemoteWorkspace>, WorkspaceApplicationError>;

    fn remember_project(
        &self,
        inspection: &ProjectInspection,
        opened_at: &str,
    ) -> Result<(), WorkspaceApplicationError>;

    fn remember_remote_workspace(
        &self,
        workspace: &RemoteWorkspace,
        opened_at: &str,
    ) -> Result<(), WorkspaceApplicationError>;
}

pub(crate) trait WorkspaceFilesystemPort: Send + Sync {
    fn canonicalize_project(&self, path: &ProjectPath)
        -> Result<String, WorkspaceApplicationError>;

    fn sibling_worktree_target(
        &self,
        project_path: &str,
        name: &WorktreeName,
    ) -> Result<String, WorkspaceApplicationError>;
}

pub(crate) trait WorkspaceGitPort: Send + Sync {
    fn repository_root(
        &self,
        project_path: &str,
    ) -> Result<Option<String>, WorkspaceApplicationError>;

    fn resolve_commit_oid(
        &self,
        project_path: &str,
        reference: &str,
    ) -> Result<String, WorkspaceApplicationError> {
        let _ = (project_path, reference);
        Err(WorkspaceApplicationError::Validation(
            "Git commit resolution is unavailable.".to_string(),
        ))
    }

    fn list_branches(
        &self,
        project_path: &str,
        limit: usize,
    ) -> Result<Vec<GitBranchReference>, WorkspaceApplicationError> {
        let _ = (project_path, limit);
        Err(WorkspaceApplicationError::Validation(
            "Git branch discovery is unavailable.".to_string(),
        ))
    }

    fn create_worktree(
        &self,
        project_path: &str,
        target_path: &str,
        branch: &str,
    ) -> Result<(), WorkspaceApplicationError>;

    fn validate_loop_worktree(
        &self,
        project_path: &str,
        target_path: &str,
        branch: &str,
        base_branch: &str,
    ) -> Result<(), WorkspaceApplicationError>;

    fn create_loop_worktree(
        &self,
        project_path: &str,
        target_path: &str,
        branch: &str,
        base_branch: &str,
    ) -> Result<(), WorkspaceApplicationError>;
}

pub(crate) trait ProjectDirectorySelectionPort: Send + Sync {
    fn select_directory(&self) -> Result<Option<String>, WorkspaceApplicationError>;
}

pub(crate) trait WorkspaceClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait WorkspaceSessionQueryPort: Send + Sync {
    fn resolve_session_root(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, WorkspaceApplicationError>;

    fn list_directory(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<DirectoryListing, WorkspaceApplicationError>;

    /// A directory inside the workspace, as an absolute path, or a refusal.
    ///
    /// Resolved against the canonical root rather than joined onto it: a `..` that lands on a real
    /// directory is exactly what a textual check lets through. `None` means the session has no
    /// local workspace, which is different from a path that is not inside one.
    fn resolve_session_directory(
        &self,
        session_id: &str,
        relative: &str,
    ) -> Result<Option<String>, WorkspaceApplicationError>;

    /// One page of a directory, resuming after a cursor.
    ///
    /// `list_directory` is the first page of this with the default bound. Keeping them separate
    /// in the trait and identical underneath is what lets the existing command stay unchanged
    /// while the provider pages, without a second ordering that drifts from the first.
    fn list_directory_page(
        &self,
        session_id: &str,
        path: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<DirectoryListing, WorkspaceApplicationError>;

    /// A cheap answer to "do these directories still look the same".
    ///
    /// Deliberately not built from `list_directory_page`: enumerating and sorting a directory to
    /// decide whether it changed does the expensive half of the work to avoid the cheap half. Every
    /// requested path is answered, including the ones that are gone.
    fn directory_fingerprints(
        &self,
        session_id: &str,
        paths: &[String],
    ) -> Result<Vec<DirectoryFingerprint>, WorkspaceApplicationError>;

    /// Quick Open, over the confined walk.
    ///
    /// Separate from `search_files`, which ranks prompt-mention candidates and therefore filters
    /// to source extensions and skips directories.
    fn search_paths(
        &self,
        session_id: &str,
        request: &WorkspacePathSearchRequest,
    ) -> Result<WorkspacePathSearchResult, WorkspaceApplicationError>;

    /// Content search over the confined walk, polling the flag it is given.
    fn search_content(
        &self,
        session_id: &str,
        request: &WorkspaceContentSearchRequest,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<WorkspaceContentSearchResult, WorkspaceApplicationError>;

    fn list_documents(
        &self,
        session_id: &str,
    ) -> Result<DocumentListing, WorkspaceApplicationError>;

    fn search_files(
        &self,
        session_id: &str,
        query: &str,
        max_results: usize,
    ) -> Result<FileSearchListing, WorkspaceApplicationError>;

    fn read_file(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<FileContent, WorkspaceApplicationError>;

    fn read_text_file(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<FileContent, WorkspaceApplicationError>;

    fn git_status(&self, session_id: &str) -> Result<GitStatusResult, WorkspaceApplicationError>;

    fn git_diff(
        &self,
        session_id: &str,
        path: &str,
        source: GitDiffSource,
    ) -> Result<GitDiffResult, WorkspaceApplicationError>;

    fn list_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogPage, WorkspaceApplicationError>;

    fn export_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogExportResult, WorkspaceApplicationError>;
}

pub(crate) trait WorkspaceShellContextPort: Send + Sync {
    fn load_shell_workspace(
        &self,
        session_id: &str,
    ) -> Result<ShellWorkspace, WorkspaceApplicationError>;
}

/// Where a remote terminal's own diagnostics go.
///
/// The only shell-shaped port left here. Its Session Shell counterparts went with the one-view
/// service they served; this one belongs to the remote terminal capability, which has its own
/// lifecycle and its own logging.
pub(crate) trait WorkspaceShellLogPort: Send + Sync {
    fn write(&self, log: ShellLog);
}
