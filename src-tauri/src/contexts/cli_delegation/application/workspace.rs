use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationWorkspace {
    pub(crate) attempt_root: PathBuf,
    pub(crate) workspace: PathBuf,
    pub(crate) inputs: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) control: PathBuf,
    pub(crate) recovery: PathBuf,
    pub(crate) repository_identity: String,
    pub(crate) base_commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationRepositoryBaseline {
    pub(crate) canonical_root: PathBuf,
    pub(crate) repository_identity: String,
    pub(crate) head_commit: String,
    pub(crate) tracked_files: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationWorkspaceError {
    InvalidRequest,
    SourceUnavailable,
    TargetExists,
    GitFailure,
    VerificationFailure,
    CleanupFailure,
}

pub(crate) trait DelegationWorkspacePort: Send + Sync {
    fn inspect_baseline(
        &self,
        source_repository: &Path,
        expected_commit: &str,
    ) -> Result<DelegationRepositoryBaseline, DelegationWorkspaceError>;

    fn create(
        &self,
        source_repository: &Path,
        exact_commit: &str,
    ) -> Result<DelegationWorkspace, DelegationWorkspaceError>;

    fn cleanup(&self, workspace: &DelegationWorkspace) -> Result<(), DelegationWorkspaceError>;
}
