mod error;
mod evidence;
mod models;
mod ports;
mod query_service;
mod review;
mod service;
mod session_shell;
mod session_shell_registry;
mod session_shell_store;
mod shell_service;

pub(crate) use error::WorkspaceApplicationError;
#[cfg(test)]
pub(crate) use evidence::NoWorkspaceEvidence;
pub(crate) use evidence::{
    WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceFileChangeKind,
    WorkspaceShellCloseReason, WorkspaceShellRuntimeKind,
};
pub(crate) use models::{
    CreateShellRequest, CreatedWorktree, DirectoryEntry, DirectoryListing, DocumentListing,
    FileContent, FileSearchListing, FileSearchMatch, GitBranchReference, GitDiffFile, GitDiffHunk,
    GitDiffLine, GitDiffResult, GitDiffSource, GitStatusEntry, GitStatusResult, KnownProject,
    KnownRemoteWorkspace, ResizeShellRequest, SessionDocument, SessionLogEntry,
    SessionLogExportResult, SessionLogPage, SessionLogQuery, SessionWorkspaceContext, ShellEvent,
    ShellLaunch, ShellLog, ShellRemoteEndpoint, ShellSession, ShellSshBinding, ShellWorkspace,
    ShellWorkspacePolicy, WorkspaceLogLevel,
};
pub(crate) use ports::{
    ProjectDirectorySelectionPort, WorkspaceClockPort, WorkspaceFilesystemPort, WorkspaceGitPort,
    WorkspaceHistoryRepository, WorkspaceSessionQueryPort, WorkspaceShellContextPort,
    WorkspaceShellEventPort, WorkspaceShellIdPort, WorkspaceShellLogPort,
    WorkspaceShellRuntimePort,
};
pub(crate) use query_service::WorkspaceQueryApplicationService;
pub(crate) use review::{
    fingerprint_context, fingerprint_hunk, fingerprint_snapshot, ReviewDiffFile, ReviewDiffHunk,
    ReviewFileSummary, ReviewRevertReceipt, ReviewRevertRequest, ReviewSnapshot,
    WorkspaceReviewPort, MAX_REVIEW_DIFF_BYTES, MAX_REVIEW_FILES, MAX_REVIEW_FILE_BYTES,
};
pub(crate) use service::WorkspaceApplicationService;
// The retained Shell modules publish through this context's api once a consumer exists; the
// registry lands with its own tests first so the contract can be reviewed before it is wired.
pub(crate) use shell_service::WorkspaceShellApplicationService;

#[cfg(test)]
mod session_shell_tests;
#[cfg(test)]
mod shell_tests;
#[cfg(test)]
mod tests;
