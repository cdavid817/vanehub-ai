mod error;
mod evidence;
mod inspection;
mod inspection_cursor;
mod inspection_router;
mod models;
mod ports;
mod query_service;
mod review;
mod service;
mod session_shell;
mod session_shell_registry;
mod session_shell_store;

pub(crate) use error::WorkspaceApplicationError;
pub(crate) use evidence::{
    WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceFileChangeKind,
    WorkspaceShellCloseReason, WorkspaceShellRuntimeKind,
};
pub(crate) use inspection::{
    CapabilityState, GitDiffRequest, ListDirectoryRequest, LocalWorkspaceTarget,
    ReadTextFileRequest, RemoteWorkspaceTarget, WatchMode, WorkspaceInspectionCapabilities,
    WorkspaceInspectionError, WorkspaceInspectionProvider, WorkspaceSearchRequest, WorkspaceTarget,
    WorkspaceTargetResolver,
};
pub(crate) use inspection_cursor::{
    bounded_page_size, kind_rank, DirectoryCursor, DEFAULT_DIRECTORY_PAGE_SIZE,
};
pub(crate) use inspection_router::WorkspaceInspectionRouter;
pub(crate) use models::{
    CreatedWorktree, DirectoryEntry, DirectoryListing, DocumentListing, FileContent,
    FileSearchListing, FileSearchMatch, GitBranchReference, GitDiffFile, GitDiffHunk, GitDiffLine,
    GitDiffResult, GitDiffSource, GitStatusEntry, GitStatusResult, KnownProject,
    KnownRemoteWorkspace, SessionDocument, SessionLogEntry, SessionLogExportResult, SessionLogPage,
    SessionLogQuery, SessionWorkspaceContext, ShellLog, ShellRemoteEndpoint, ShellSshBinding,
    ShellWorkspace, ShellWorkspacePolicy, WorkspaceLogLevel,
};
pub(crate) use ports::{
    ProjectDirectorySelectionPort, WorkspaceClockPort, WorkspaceFilesystemPort, WorkspaceGitPort,
    WorkspaceHistoryRepository, WorkspaceSessionQueryPort, WorkspaceShellContextPort,
    WorkspaceShellLogPort,
};
pub(crate) use query_service::WorkspaceQueryApplicationService;
pub(crate) use review::{
    fingerprint_context, fingerprint_hunk, fingerprint_snapshot, ReviewDiffFile, ReviewDiffHunk,
    ReviewFileSummary, ReviewRevertReceipt, ReviewRevertRequest, ReviewSnapshot,
    WorkspaceReviewPort, MAX_REVIEW_DIFF_BYTES, MAX_REVIEW_FILES, MAX_REVIEW_FILE_BYTES,
};
pub(crate) use service::WorkspaceApplicationService;
pub(crate) use session_shell::{
    AttachSessionShellRequest, CreateSessionShellRequest, ResizeSessionShellRequest,
    SessionShellDescriptor, SessionShellNotice, SessionShellNoticePort, SessionShellRuntimePort,
    SessionShellWorkspace, SessionShellWorkspacePort, ShellAttachSnapshot, ShellAttachmentScope,
    ShellCapacities, ShellClockPort, ShellIdPort, ShellOutputSink, ShellRemoteTarget,
    ShellRuntimeOpen, ShellRuntimeOpened, WriteSessionShellRequest,
};
pub(crate) use session_shell_registry::SessionShellRegistry;
pub(crate) use session_shell_store::ShellStore;

#[cfg(test)]
mod session_shell_retention_tests;
#[cfg(test)]
mod session_shell_tests;
#[cfg(test)]
mod tests;
