pub(crate) use super::application::{
    AttachSessionShellRequest, CreateSessionShellRequest, ResizeSessionShellRequest,
    SessionShellDescriptor, SessionShellRegistry, ShellAttachSnapshot, ShellAttachmentScope,
    WriteSessionShellRequest,
};
/// Provider-neutral workspace inspection.
///
/// Published so bootstrap can assemble the router and a command can ask it questions. The
/// `WorkspaceTarget` itself is published too, because a caller has to be able to tell a reader
/// which machine an answer came from — but nothing outside this context can construct one.
pub(crate) use super::application::{
    CapabilityState, WorkspaceInspectionCapabilities, WorkspaceInspectionError,
    WorkspaceInspectionRouter, WorkspaceTarget,
};
pub(crate) use super::application::{
    CreatedWorktree, DirectoryListing, DocumentListing, FileContent, FileSearchListing,
    GitBranchReference, GitDiffFile, GitDiffHunk, GitDiffLine, GitDiffResult, GitDiffSource,
    GitStatusResult, KnownProject, KnownRemoteWorkspace, ReviewDiffFile, ReviewRevertReceipt,
    ReviewRevertRequest, ReviewSnapshot, SessionLogExportResult, SessionLogQuery,
    SessionWorkspaceContext, WorkspaceApplicationError as WorkspaceError, WorkspaceLogLevel,
    WorkspaceReviewPort,
};
use super::application::{WorkspaceApplicationService, WorkspaceQueryApplicationService};
/// Normalized workspace change notices.
///
/// The scope and source vocabularies are published because producers outside this context observe
/// changes — the runtime knows it wrote a file long before any watcher could see it. The dispatcher
/// is published so bootstrap can assemble it once and hand the same one to every producer.
pub(crate) use super::application::{
    WorkspaceChangeObserverPort, WorkspaceInvalidationChange, WorkspaceInvalidationDispatcher,
    WorkspaceInvalidationScope, WorkspaceInvalidationSource,
};
pub(crate) use super::application::{
    WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceFileChangeKind,
    WorkspaceShellCloseReason, WorkspaceShellRuntimeKind,
};
pub(crate) use super::domain::{
    ensure_git_worktree_available, ensure_worktree_compatible, ProjectInspection, RemoteWorkspace,
    ShellRuntimeDescriptor,
};
pub(crate) use super::domain::{SessionShellError, ShellId};
pub(crate) use super::infrastructure::PreparedEvaluationFixture;
use super::infrastructure::SystemWorkspaceChangeObserver;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Wall-clock milliseconds, for the coalescing window and the observation lifetime.
///
/// A clock before the epoch is treated as zero rather than refused: the consequence is one poll
/// cycle behaving oddly on a machine whose clock is badly wrong, which is not worth failing a file
/// listing over.
fn unix_milliseconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone)]
pub(crate) struct WorkspaceApi {
    service: WorkspaceApplicationService,
    queries: WorkspaceQueryApplicationService,
    review: Arc<dyn WorkspaceReviewPort>,
    shells: Arc<SessionShellRegistry>,
    /// Provider-neutral inspection. Shared rather than owned per call because selection is a
    /// property of the session, not of the caller, and two routers could disagree about it.
    inspection: Arc<WorkspaceInspectionRouter>,
    /// Where change notices are buffered, and what remembers which directories are open.
    ///
    /// Held here because the reads that populate it come through this API. A separate "subscribe to
    /// this directory" call would be a second statement of what a console is looking at, and the
    /// two would disagree the first time one of them was forgotten.
    invalidation: Arc<WorkspaceInvalidationDispatcher>,
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
        review: Arc<dyn WorkspaceReviewPort>,
        shells: Arc<SessionShellRegistry>,
        inspection: Arc<WorkspaceInspectionRouter>,
        invalidation: Arc<WorkspaceInvalidationDispatcher>,
    ) -> Self {
        Self {
            service,
            queries,
            review,
            shells,
            inspection,
            invalidation,
        }
    }

    /// How a producer elsewhere in the process reports a change it saw.
    ///
    /// Handed out as the narrow port rather than as this API, so the runtime's mutation fanout takes
    /// a dependency on "somewhere to report a change" instead of on workspaces as a whole.
    pub(crate) fn change_observer(&self) -> Arc<dyn WorkspaceChangeObserverPort> {
        Arc::new(SystemWorkspaceChangeObserver::new(
            self.invalidation.clone(),
        ))
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
        let listing = self.queries.list_directory(session_id, path)?;
        // Recorded on the way out, and only when the read worked. A directory that could not be
        // listed is not one a console is showing, and polling it would be spending a stat every
        // tick to rediscover that.
        self.invalidation
            .note_directory_read(session_id, path, unix_milliseconds());
        Ok(listing)
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

    /// Which machine this session's workspace is on, and what can be read there.
    ///
    /// Resolved from the registered binding on every call rather than cached: a session can be
    /// rebound between two reads, and a cached target would keep answering about the host it was
    /// bound to when the panel opened.
    pub(crate) fn inspection_target(
        &self,
        session_id: &str,
    ) -> Result<WorkspaceTarget, WorkspaceInspectionError> {
        self.inspection.target(session_id)
    }

    pub(crate) async fn inspection_capabilities(
        &self,
        session_id: &str,
    ) -> Result<WorkspaceInspectionCapabilities, WorkspaceInspectionError> {
        self.inspection.capabilities(session_id).await
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

    // The interactive log query moved to the operations-owned index. Nothing here scans log
    // files for a query any more: a fallback would be a second implementation with different
    // bounds and different coverage semantics, reached exactly when a reader is least able to
    // tell which one answered. Export still reads the redacted files, which is what an export is.
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

    /// Ends every Shell a session owns.
    ///
    /// Called on the "this session is done" edge — archive and delete — and on no other. A retained
    /// Shell outlives its view by design, so nothing else would ever close it: the session it
    /// belonged to would be gone from the list while its process kept running with no way left to
    /// reach it.
    pub(crate) fn kill_shells_for_session(&self, session_id: &str) -> Result<(), WorkspaceError> {
        for descriptor in self.shells.list(Some(session_id)) {
            let _ = self.shells.close(&descriptor.shell_id);
        }
        Ok(())
    }

    pub(crate) fn list_session_shells(&self, session_id: &str) -> Vec<SessionShellDescriptor> {
        self.shells.list(Some(session_id))
    }

    pub(crate) fn create_session_shell(
        &self,
        request: &CreateSessionShellRequest,
    ) -> Result<SessionShellDescriptor, SessionShellError> {
        self.shells.create(request)
    }

    /// Opens a Shell off the main thread.
    ///
    /// A PTY spawn is quick and an SSH handshake is not, and a synchronous Tauri command runs where
    /// the webview runs: the window would stop repainting until the far end answered. The four
    /// Shell operations that reach the runtime all go through the blocking pool for that reason —
    /// closing is the worst of them, because it kills a process, waits for it, and joins its reader.
    pub(crate) async fn create_session_shell_blocking(
        &self,
        request: CreateSessionShellRequest,
    ) -> Result<SessionShellDescriptor, SessionShellError> {
        self.on_blocking_pool(move |api| api.create_session_shell(&request))
            .await
    }

    pub(crate) async fn write_session_shell_blocking(
        &self,
        request: WriteSessionShellRequest,
    ) -> Result<(), SessionShellError> {
        self.on_blocking_pool(move |api| api.write_session_shell(&request))
            .await
    }

    pub(crate) async fn resize_session_shell_blocking(
        &self,
        request: ResizeSessionShellRequest,
    ) -> Result<(), SessionShellError> {
        self.on_blocking_pool(move |api| api.resize_session_shell(&request))
            .await
    }

    pub(crate) async fn close_session_shell_blocking(
        &self,
        shell_id: ShellId,
    ) -> Result<(), SessionShellError> {
        self.on_blocking_pool(move |api| api.close_session_shell(&shell_id))
            .await
    }

    /// A task that could not be scheduled is reported as a runtime failure rather than swallowed:
    /// the caller has to know its Shell operation did not happen.
    async fn on_blocking_pool<T, F>(&self, work: F) -> Result<T, SessionShellError>
    where
        T: Send + 'static,
        F: FnOnce(WorkspaceApi) -> Result<T, SessionShellError> + Send + 'static,
    {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || work(api))
            .await
            .map_err(|_| SessionShellError::Runtime {
                reason: crate::contexts::workspaces::domain::shell_reason("shell_task_failed"),
            })?
    }

    pub(crate) fn attach_session_shell(
        &self,
        request: &AttachSessionShellRequest,
    ) -> Result<ShellAttachSnapshot, SessionShellError> {
        self.shells.attach(request)
    }

    pub(crate) fn detach_session_shell(
        &self,
        scope: &ShellAttachmentScope,
    ) -> Result<(), SessionShellError> {
        self.shells.detach(scope)
    }

    pub(crate) fn write_session_shell(
        &self,
        request: &WriteSessionShellRequest,
    ) -> Result<(), SessionShellError> {
        self.shells.write(request)
    }

    pub(crate) fn resize_session_shell(
        &self,
        request: &ResizeSessionShellRequest,
    ) -> Result<(), SessionShellError> {
        self.shells.resize(request)
    }

    pub(crate) fn rename_session_shell(
        &self,
        shell_id: &ShellId,
        title: &str,
    ) -> Result<SessionShellDescriptor, SessionShellError> {
        self.shells.rename(shell_id, title)
    }

    pub(crate) fn close_session_shell(&self, shell_id: &ShellId) -> Result<(), SessionShellError> {
        self.shells.close(shell_id)
    }

    /// How many Shells a session is holding, for the workspace summary.
    ///
    /// Owned by the registry rather than counted by the panel: a badge produced by mounting a list
    /// is a badge that opens what it is describing.
    pub(crate) fn live_session_shell_count(&self, session_id: &str) -> usize {
        self.shells.live_count(session_id)
    }

    /// Reclaims detached, quiet Shells. Bounded per sweep and never a Shell someone is watching.
    pub(crate) fn sweep_idle_session_shells(&self) -> usize {
        self.shells.sweep_idle().len()
    }

    /// Closes every Shell and joins its runtime workers.
    ///
    /// Called on the way out rather than left to the process teardown, because a joined worker is
    /// the difference between a clean exit and a window that has closed while the process is still
    /// waiting on a thread reading a dead PTY.
    pub(crate) fn shutdown_session_shells(&self) {
        self.shells.shutdown();
    }
}
