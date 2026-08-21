pub(crate) use super::application::{
    CreateShellRequest, CreatedWorktree, DirectoryListing, DocumentListing, FileContent,
    FileSearchListing, GitBranchReference, GitDiffFile, GitDiffHunk, GitDiffLine, GitDiffResult,
    GitDiffSource, GitStatusResult, KnownProject, KnownRemoteWorkspace, ResizeShellRequest,
    ReviewDiffFile, ReviewRevertReceipt, ReviewRevertRequest, ReviewSnapshot,
    SessionLogExportResult, SessionLogPage, SessionLogQuery, SessionWorkspaceContext, ShellSession,
    WorkspaceApplicationError as WorkspaceError, WorkspaceLogLevel, WorkspaceReviewPort,
};
use super::application::{
    WorkspaceApplicationService, WorkspaceQueryApplicationService, WorkspaceShellApplicationService,
};
pub(crate) use super::domain::{
    ensure_git_worktree_available, ensure_worktree_compatible, ProjectInspection, RemoteWorkspace,
};
pub(crate) use super::infrastructure::PreparedEvaluationFixture;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct WorkspaceApi {
    service: WorkspaceApplicationService,
    queries: WorkspaceQueryApplicationService,
    shell: WorkspaceShellApplicationService,
    review: Arc<dyn WorkspaceReviewPort>,
}

impl WorkspaceApi {
    pub(crate) fn prepare_evaluation_fixture(
        &self,
        source: &Path,
        root: &Path,
        attempt_id: &str,
    ) -> Result<PreparedEvaluationFixture, String> {
        super::infrastructure::prepare_evaluation_fixture(source, root, attempt_id)
    }

    pub(crate) fn cleanup_evaluation_fixture(
        &self,
        root: &Path,
        attempt_id: &str,
    ) -> Result<(), String> {
        super::infrastructure::cleanup_evaluation_fixture(root, attempt_id)
    }
    pub(crate) fn changed_evaluation_paths(
        &self,
        source: &Path,
        workspace: &Path,
    ) -> Result<Vec<String>, String> {
        super::infrastructure::changed_evaluation_paths(source, workspace)
    }
    pub(crate) fn new(
        service: WorkspaceApplicationService,
        queries: WorkspaceQueryApplicationService,
        shell: WorkspaceShellApplicationService,
        review: Arc<dyn WorkspaceReviewPort>,
    ) -> Self {
        Self {
            service,
            queries,
            shell,
            review,
        }
    }

    pub(crate) fn create_review_snapshot(
        &self,
        session_id: &str,
    ) -> Result<ReviewSnapshot, WorkspaceError> {
        self.review.create_review_snapshot(session_id)
    }

    pub(crate) fn load_review_file(
        &self,
        session_id: &str,
        path: &str,
        expected_snapshot: &str,
    ) -> Result<ReviewDiffFile, WorkspaceError> {
        self.review
            .load_review_file(session_id, path, expected_snapshot)
    }

    pub(crate) fn revert_review_change(
        &self,
        request: &ReviewRevertRequest,
    ) -> Result<ReviewRevertReceipt, WorkspaceError> {
        self.review.revert_review_change(request)
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

    pub(crate) fn list_git_branches(
        &self,
        project_path: &str,
    ) -> Result<Vec<GitBranchReference>, WorkspaceError> {
        self.service.list_git_branches(project_path)
    }

    pub(crate) async fn list_git_branches_blocking(
        &self,
        project_path: String,
    ) -> Result<Vec<GitBranchReference>, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || api.list_git_branches(&project_path))
            .await
            .map_err(|_| WorkspaceError::Storage("Git branch discovery task failed".to_string()))?
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

    pub(crate) fn search_session_files(
        &self,
        session_id: &str,
        query: &str,
        max_results: usize,
    ) -> Result<FileSearchListing, WorkspaceError> {
        self.queries.search_files(session_id, query, max_results)
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

    /// Async wrapper that runs `git status` on the blocking pool, since it can hit the
    /// process timeout on slow repositories and must not freeze the async executor.
    pub(crate) async fn get_session_git_status_blocking(
        &self,
        session_id: String,
    ) -> Result<GitStatusResult, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || api.get_session_git_status(&session_id))
            .await
            .map_err(|_| WorkspaceError::Storage("git status task failed".to_string()))?
    }

    pub(crate) fn get_session_git_diff(
        &self,
        session_id: &str,
        path: &str,
        source: GitDiffSource,
    ) -> Result<GitDiffResult, WorkspaceError> {
        self.queries.git_diff(session_id, path, source)
    }

    /// Async wrapper for `git diff`, which can spawn git twice on slow repositories.
    pub(crate) async fn get_session_git_diff_blocking(
        &self,
        session_id: String,
        path: String,
        source: GitDiffSource,
    ) -> Result<GitDiffResult, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            api.get_session_git_diff(&session_id, &path, source)
        })
        .await
        .map_err(|_| WorkspaceError::Storage("git diff task failed".to_string()))?
    }

    pub(crate) fn list_session_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogPage, WorkspaceError> {
        self.queries.list_logs(query)
    }

    /// Async wrapper for log listing, which reads and filters whole log files.
    pub(crate) async fn list_session_logs_blocking(
        &self,
        query: SessionLogQuery,
    ) -> Result<SessionLogPage, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || api.list_session_logs(&query))
            .await
            .map_err(|_| WorkspaceError::Storage("session logs task failed".to_string()))?
    }

    pub(crate) fn export_session_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogExportResult, WorkspaceError> {
        self.queries.export_logs(query)
    }

    /// Async wrapper for log export, which writes a file and may surface a save dialog.
    pub(crate) async fn export_session_logs_blocking(
        &self,
        query: SessionLogQuery,
    ) -> Result<SessionLogExportResult, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || api.export_session_logs(&query))
            .await
            .map_err(|_| WorkspaceError::Storage("session log export task failed".to_string()))?
    }

    /// Async wrapper for directory listing, which walks the filesystem synchronously.
    pub(crate) async fn list_session_directory_blocking(
        &self,
        session_id: String,
        path: String,
    ) -> Result<DirectoryListing, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || api.list_session_directory(&session_id, &path))
            .await
            .map_err(|_| WorkspaceError::Storage("session directory task failed".to_string()))?
    }

    /// Async wrapper for mention candidate search, which walks the filesystem synchronously.
    pub(crate) async fn search_session_files_blocking(
        &self,
        session_id: String,
        query: String,
        max_results: usize,
    ) -> Result<FileSearchListing, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            api.search_session_files(&session_id, &query, max_results)
        })
        .await
        .map_err(|_| WorkspaceError::Storage("session file search task failed".to_string()))?
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
