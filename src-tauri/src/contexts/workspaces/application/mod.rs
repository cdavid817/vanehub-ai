mod content_search;
mod error;
mod evidence;
mod ignore_policy;
mod inspection;
mod inspection_admission;
mod inspection_budget;
mod inspection_cursor;
mod inspection_execution;
mod inspection_router;
mod invalidation;
mod invalidation_dispatcher;
mod models;
mod ports;
mod query_service;
mod review;
mod search_cancellation;
mod service;
mod session_shell;
mod session_shell_capacity;
mod session_shell_close;
mod session_shell_reaper;
mod session_shell_registry;
mod session_shell_remote;
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
    deliver as deliver_content_search, safe_snippet, WorkspaceContentMatch,
    WorkspaceContentSearchDelivery, WorkspaceContentSearchRequest, WorkspaceContentSearchResult,
    MAX_CONTENT_MATCHES, MAX_SEARCHED_FILE_BYTES,
};
pub(crate) use error::WorkspaceApplicationError;
pub(crate) use evidence::{
    WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceFileChangeKind,
    WorkspaceShellCloseReason, WorkspaceShellRuntimeKind,
};
/// What a recursive walk is looking for, and what it is not. One policy for Quick Open, content
/// search, mention candidates and document discovery, because a file findable by name and not by
/// content reads as the search being broken for that one file.
pub(crate) use ignore_policy::WorkspaceIgnorePolicy;
/// Published to tests only: production reads the state through the coverage it belongs to and never
/// names the enum, while a test asserting which of the three a refusal produced does.
#[cfg(test)]
pub(crate) use inspection::WorkspaceSearchCoverageState;
pub(crate) use inspection::{
    deliver_path_search, CapabilityState, DirectoryFingerprint, DirectoryFingerprintState,
    GitDiffRequest, ListDirectoryRequest, LocalWorkspaceTarget, ReadTextFileRequest,
    RemoteWorkspaceTarget, WatchMode, WorkspaceInspectionCapabilities, WorkspaceInspectionError,
    WorkspaceInspectionProvider, WorkspacePathMatch, WorkspacePathSearchDelivery,
    WorkspacePathSearchRequest, WorkspacePathSearchResult, WorkspaceSearchCoverage,
    WorkspaceSearchRequest, WorkspaceTarget, WorkspaceTargetResolver, MAX_FINGERPRINT_PATHS,
};
/// How many inspections run at once, and the shared work accounting they run under.
pub(crate) use inspection_admission::WorkspaceInspectionAdmission;
/// Published to tests only: a clock that moves when a test moves it, so a deadline can be proved
/// without sleeping through one.
#[cfg(test)]
pub(crate) use inspection_budget::ManualClock;
pub(crate) use inspection_budget::WorkspaceInspectionBudgetSnapshot;
pub(crate) use inspection_budget::{
    MonotonicClockPort, SystemMonotonicClock, WorkspaceInspectionBudget,
    WorkspaceInspectionBudgetLimits, WorkspaceInspectionReason,
};
pub(crate) use inspection_cursor::{
    bounded_page_size, bounded_search_page, kind_rank, workspace_identity, DirectoryCursor,
    DirectoryOrder, DirectoryPageScope, PathSearchCursor, DEFAULT_DIRECTORY_PAGE_SIZE,
};
pub(crate) use inspection_execution::{WorkspaceInspectionExecution, WorkspaceInspectionOperation};
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
    ProjectDirectorySelectionPort, ShellLifecycleDiagnosticsPort, WorkspaceClockPort,
    WorkspaceFilesystemPort, WorkspaceGitPort, WorkspaceHistoryRepository,
    WorkspaceSessionQueryPort, WorkspaceShellContextPort, WorkspaceShellLogPort,
};
pub(crate) use query_service::WorkspaceQueryApplicationService;
pub(crate) use review::{
    fingerprint_context, fingerprint_hunk, fingerprint_patch, fingerprint_snapshot, ReviewDiffFile,
    ReviewDiffHunk, ReviewFileSummary, ReviewPatch, ReviewPatchRequest, ReviewRevertReceipt,
    ReviewRevertRequest, ReviewSnapshot, WorkspaceReviewPort, MAX_REVIEW_DIFF_BYTES,
    MAX_REVIEW_FILES, MAX_REVIEW_FILE_BYTES, MAX_REVIEW_PATCH_BYTES,
};
/// Generation-safe cancellation. The registry is published so the API can own one; the token is
/// published because it is what a provider actually polls.
pub(crate) use search_cancellation::{
    SearchCancellationCause, SearchCancellationToken, WorkspaceSearchCancellation,
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
pub(crate) use session_shell_registry::{SessionShellPorts, SessionShellRegistry};
pub(crate) use session_shell_remote::{
    RemoteShellChannel, RemoteShellChannelError, RemoteShellEvent, RemoteShellOpenFailure,
    RemoteShellTransport,
};
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
