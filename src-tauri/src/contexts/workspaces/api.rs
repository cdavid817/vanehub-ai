pub(crate) use super::application::{
    CreateShellRequest, CreatedWorktree, DirectoryListing, DocumentListing, FileContent,
    GitDiffFile, GitDiffHunk, GitDiffLine, GitDiffResult, GitDiffSource, GitStatusResult,
    KnownProject, KnownRemoteWorkspace, ResizeShellRequest, SessionLogExportResult, SessionLogPage,
    SessionLogQuery, SessionWorkspaceContext, ShellSession,
    WorkspaceApplicationError as WorkspaceError, WorkspaceLogLevel,
};
use super::application::{
    WorkspaceApplicationService, WorkspaceQueryApplicationService, WorkspaceShellApplicationService,
};
pub(crate) use super::domain::{
    ensure_git_worktree_available, ensure_worktree_compatible, ProjectInspection, RemoteWorkspace,
};

#[derive(Clone)]
pub(crate) struct WorkspaceApi {
    service: WorkspaceApplicationService,
    queries: WorkspaceQueryApplicationService,
    shell: WorkspaceShellApplicationService,
}

impl WorkspaceApi {
    pub(crate) fn new(
        service: WorkspaceApplicationService,
        queries: WorkspaceQueryApplicationService,
        shell: WorkspaceShellApplicationService,
    ) -> Self {
        Self {
            service,
            queries,
            shell,
        }
    }

    pub(crate) fn list_known_projects(&self) -> Result<Vec<KnownProject>, WorkspaceError> {
        self.service.list_known_projects()
    }

    pub(crate) fn resolve_session_root(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, WorkspaceError> {
        self.queries.resolve_session_root(session_id)
    }

    pub(crate) fn list_known_remote_workspaces(
        &self,
    ) -> Result<Vec<KnownRemoteWorkspace>, WorkspaceError> {
        self.service.list_known_remote_workspaces()
    }

    pub(crate) fn inspect_project(&self, path: &str) -> Result<ProjectInspection, WorkspaceError> {
        self.service.inspect_project(path)
    }

    pub(crate) fn remember_project(
        &self,
        inspection: &ProjectInspection,
    ) -> Result<(), WorkspaceError> {
        self.service.remember_project(inspection)
    }

    pub(crate) fn remember_remote_workspace(
        &self,
        workspace: &RemoteWorkspace,
    ) -> Result<(), WorkspaceError> {
        self.service.remember_remote_workspace(workspace)
    }

    pub(crate) fn select_project_directory(&self) -> Result<Option<String>, WorkspaceError> {
        self.service.select_project_directory()
    }

    pub(crate) fn create_worktree(
        &self,
        project_path: &str,
        name: &str,
    ) -> Result<CreatedWorktree, WorkspaceError> {
        self.service.create_worktree(project_path, name)
    }

    pub(crate) fn create_guarded_loop_worktree(
        &self,
        project_path: &str,
        name: &str,
        base_branch: &str,
    ) -> Result<CreatedWorktree, WorkspaceError> {
        self.service
            .create_guarded_loop_worktree(project_path, name, base_branch)
    }

    pub(crate) fn list_session_directory(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<DirectoryListing, WorkspaceError> {
        self.queries.list_directory(session_id, path)
    }

    pub(crate) fn list_session_documents(
        &self,
        session_id: &str,
    ) -> Result<DocumentListing, WorkspaceError> {
        self.queries.list_documents(session_id)
    }

    pub(crate) fn read_session_file(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<FileContent, WorkspaceError> {
        self.queries.read_file(session_id, path)
    }

    pub(crate) fn read_session_text_file(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<FileContent, WorkspaceError> {
        self.queries.read_text_file(session_id, path)
    }

    pub(crate) fn get_session_git_status(
        &self,
        session_id: &str,
    ) -> Result<GitStatusResult, WorkspaceError> {
        self.queries.git_status(session_id)
    }

    pub(crate) fn get_session_git_diff(
        &self,
        session_id: &str,
        path: &str,
        source: GitDiffSource,
    ) -> Result<GitDiffResult, WorkspaceError> {
        self.queries.git_diff(session_id, path, source)
    }

    pub(crate) fn list_session_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogPage, WorkspaceError> {
        self.queries.list_logs(query)
    }

    pub(crate) fn export_session_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogExportResult, WorkspaceError> {
        self.queries.export_logs(query)
    }

    pub(crate) fn create_shell(
        &self,
        request: &CreateShellRequest,
    ) -> Result<ShellSession, WorkspaceError> {
        self.shell.create_shell(request)
    }

    pub(crate) fn write_shell_input(
        &self,
        shell_id: &str,
        content: &str,
    ) -> Result<(), WorkspaceError> {
        self.shell.write_input(shell_id, content)
    }

    pub(crate) fn reset_shell_directory(&self, shell_id: &str) -> Result<(), WorkspaceError> {
        self.shell.reset_directory(shell_id)
    }

    pub(crate) fn resize_shell(&self, request: &ResizeShellRequest) -> Result<(), WorkspaceError> {
        self.shell.resize_shell(request)
    }

    pub(crate) fn kill_shell(&self, shell_id: &str) -> Result<(), WorkspaceError> {
        self.shell.kill_shell(shell_id)
    }

    pub(crate) fn kill_shells_for_session(&self, session_id: &str) -> Result<(), WorkspaceError> {
        self.shell.kill_for_session(session_id)
    }
}
