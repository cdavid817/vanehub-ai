use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DelegationChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    TypeChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DelegationChangeFile {
    pub(crate) path: String,
    pub(crate) previous_path: Option<String>,
    pub(crate) kind: DelegationChangeKind,
    pub(crate) before_mode: Option<String>,
    pub(crate) after_mode: Option<String>,
    pub(crate) before_git_hash: Option<String>,
    pub(crate) after_git_hash: Option<String>,
    pub(crate) binary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationChangeSetCapture {
    pub(crate) base_commit: String,
    pub(crate) files: Vec<DelegationChangeFile>,
    pub(crate) canonical_patch: Vec<u8>,
    pub(crate) diff_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationChangeSetCaptureError {
    InvalidWorkspace,
    BaseMismatch,
    GitFailure,
    InvalidGitOutput,
    StorageFailure,
}

pub(crate) trait DelegationChangeSetCapturePort: Send + Sync {
    fn capture(
        &self,
        workspace: &Path,
        control: &Path,
        expected_base: &str,
    ) -> Result<DelegationChangeSetCapture, DelegationChangeSetCaptureError>;
}
