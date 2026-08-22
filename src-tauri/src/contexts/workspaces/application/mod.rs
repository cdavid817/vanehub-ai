mod error;
mod evidence;
mod models;
mod ports;
mod query_service;
mod review;
mod service;
mod shell_service;

pub(crate) use error::WorkspaceApplicationError;
pub(crate) use evidence::{
    NoWorkspaceEvidence, WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceShellRuntimeKind,
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
pub(crate) use shell_service::WorkspaceShellApplicationService;

#[cfg(test)]
mod shell_tests;
#[cfg(test)]
mod tests;
