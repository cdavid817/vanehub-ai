use super::{
    DirectoryListing, DocumentListing, FileContent, GitDiffResult, GitDiffSource, GitStatusResult,
    KnownProject, KnownRemoteWorkspace, SessionLogExportResult, SessionLogPage, SessionLogQuery,
    ShellEvent, ShellLaunch, ShellLog, ShellWorkspace, WorkspaceApplicationError,
};
use crate::contexts::workspaces::domain::{
    ProjectInspection, ProjectPath, RemoteWorkspace, TerminalDimensions, WorktreeName,
};

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

    fn create_worktree(
        &self,
        project_path: &str,
        target_path: &str,
        branch: &str,
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

    fn list_documents(
        &self,
        session_id: &str,
    ) -> Result<DocumentListing, WorkspaceApplicationError>;

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

pub(crate) trait WorkspaceShellRuntimePort: Send + Sync {
    fn open_shell(&self, launch: &ShellLaunch) -> Result<(), WorkspaceApplicationError>;

    fn write_input(&self, shell_id: &str, content: &str) -> Result<(), WorkspaceApplicationError>;

    fn reset_directory(&self, shell_id: &str) -> Result<(), WorkspaceApplicationError>;

    fn resize(
        &self,
        shell_id: &str,
        dimensions: TerminalDimensions,
    ) -> Result<(), WorkspaceApplicationError>;

    fn stop(&self, shell_id: &str) -> Result<Option<String>, WorkspaceApplicationError>;

    fn stop_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<(String, String)>, WorkspaceApplicationError>;
}

pub(crate) trait WorkspaceShellIdPort: Send + Sync {
    fn next_shell_id(&self) -> String;
}

pub(crate) trait WorkspaceShellEventPort: Send + Sync {
    fn publish(&self, event: ShellEvent);
}

pub(crate) trait WorkspaceShellLogPort: Send + Sync {
    fn write(&self, log: ShellLog);
}
