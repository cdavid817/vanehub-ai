mod content_search;
mod error;
mod evidence;
mod inspection;
mod inspection_cursor;
mod inspection_router;
mod invalidation;
mod invalidation_dispatcher;
mod models;
mod ports;
mod query_service;
mod review;
mod service;
mod session_shell;
mod session_shell_capacity;
mod session_shell_close;
mod session_shell_reaper;
mod session_shell_registry;
mod session_shell_store;
mod text_metadata;

/// Published to tests only.
///
/// Production has no caller: the snippet bound is applied inside `safe_snippet`, and nothing
/// outside needs to know the number. A test does — asserting the named bound rather than `200`
/// keeps the two from drifting apart the day somebody changes it.
#[cfg(test)]
pub(crate) use content_search::MAX_SNIPPET_CHARS;
pub(crate) use content_search::{
    safe_snippet, WorkspaceContentMatch, WorkspaceContentSearchRequest,
    WorkspaceContentSearchResult, WorkspaceSearchCancellation, MAX_CONTENT_MATCHES,
    MAX_SEARCHED_FILE_BYTES,
};
pub(crate) use error::WorkspaceApplicationError;
pub(crate) use evidence::{
    WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceFileChangeKind,
    WorkspaceShellCloseReason, WorkspaceShellRuntimeKind,
};
pub(crate) use inspection::{
    CapabilityState, DirectoryFingerprint, DirectoryFingerprintState, GitDiffRequest,
    ListDirectoryRequest, LocalWorkspaceTarget, ReadTextFileRequest, RemoteWorkspaceTarget,
    WatchMode, WorkspaceInspectionCapabilities, WorkspaceInspectionError,
    WorkspaceInspectionProvider, WorkspacePathMatch, WorkspacePathSearchRequest,
    WorkspacePathSearchResult, WorkspaceSearchCoverage, WorkspaceSearchRequest, WorkspaceTarget,
    WorkspaceTargetResolver, MAX_FINGERPRINT_PATHS,
};
pub(crate) use inspection_cursor::{
    bounded_page_size, bounded_search_page, kind_rank, DirectoryCursor, PathSearchCursor,
    DEFAULT_DIRECTORY_PAGE_SIZE,
};
pub(crate) use inspection_router::WorkspaceInspectionRouter;
pub(crate) use invalidation::{
    WorkspaceChangeObserverPort, WorkspaceInvalidationChange, WorkspaceInvalidationNotice,
    WorkspaceInvalidationPublisher, WorkspaceInvalidationScope, WorkspaceInvalidationSource,
};
pub(crate) use invalidation_dispatcher::WorkspaceInvalidationDispatcher;
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
    fingerprint_context, fingerprint_hunk, fingerprint_patch, fingerprint_snapshot, ReviewDiffFile,
    ReviewDiffHunk, ReviewFileSummary, ReviewPatch, ReviewPatchRequest, ReviewRevertReceipt,
    ReviewRevertRequest, ReviewSnapshot, WorkspaceReviewPort, MAX_REVIEW_DIFF_BYTES,
    MAX_REVIEW_FILES, MAX_REVIEW_FILE_BYTES, MAX_REVIEW_PATCH_BYTES,
};
pub(crate) use service::WorkspaceApplicationService;
pub(crate) use session_shell::{
    AttachSessionShellRequest, CreateSessionShellRequest, ResizeSessionShellRequest,
    SessionShellDescriptor, SessionShellNotice, SessionShellNoticePort, SessionShellRuntimePort,
    SessionShellWorkspace, SessionShellWorkspacePort, ShellAttachSnapshot, ShellAttachmentScope,
    ShellCapacities, ShellClockPort, ShellIdPort, ShellOutputSink, ShellRemoteTarget,
    ShellRuntimeOpen, ShellRuntimeOpened, WriteSessionShellRequest,
};
pub(crate) use session_shell_close::{
    SessionShellCleanupReport, SessionShellCloseResult, ShellRuntimeCloseOutcome,
};
pub(crate) use session_shell_registry::SessionShellRegistry;
pub(crate) use session_shell_store::ShellStore;
pub(crate) use text_metadata::{detect_encoding, detect_newline};

#[cfg(test)]
mod invalidation_tests;
#[cfg(test)]
mod session_shell_retention_tests;
#[cfg(test)]
mod session_shell_tests;
#[cfg(test)]
mod tests;
