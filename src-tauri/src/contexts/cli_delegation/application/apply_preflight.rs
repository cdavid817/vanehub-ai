use super::{DelegationChangeSetCapture, DelegationChangeSetLimits, DelegationChangeSetPolicy};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) struct DelegationApplyArtifactEvidence {
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) repository_identity: String,
    pub(crate) capture: DelegationChangeSetCapture,
    pub(crate) applyable: bool,
    pub(crate) integrity_verified: bool,
}

pub(crate) trait DelegationApplyArtifactPort: Send + Sync {
    fn load_apply_evidence(&self, artifact_id: &str)
        -> Result<DelegationApplyArtifactEvidence, ()>;
}

pub(crate) struct DelegationApplyTargetWitness {
    pub(crate) canonical_root: PathBuf,
    pub(crate) repository_identity: String,
    pub(crate) head_commit: String,
    pub(crate) worktree_clean: bool,
    pub(crate) index_clean: bool,
    pub(crate) path_compatible: bool,
}

pub(crate) trait DelegationApplyTargetPort: Send + Sync {
    fn inspect_target(&self, root: &Path) -> Result<DelegationApplyTargetWitness, ()>;
}

pub(crate) trait DelegationApplyOncePort: Send + Sync {
    fn is_available(&self, artifact_id: &str, approval_input_hash: &str) -> Result<bool, ()>;
}

pub(crate) struct DelegationApplyPreflightRequest {
    pub(crate) target_root: PathBuf,
    pub(crate) artifact_id: String,
    pub(crate) expected_content_hash: String,
    pub(crate) expected_diff_hash: String,
    pub(crate) expected_repository_identity: String,
    pub(crate) expected_base_commit: String,
    pub(crate) approval_input_hash: String,
}

pub(crate) struct DelegationApplyPlan {
    pub(crate) target_root: PathBuf,
    pub(crate) artifact: DelegationApplyArtifactEvidence,
    pub(crate) approval_input_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationApplyPreflightError {
    InvalidRequest,
    ArtifactUnavailable,
    IntegrityFailure,
    TargetUnavailable,
    RepositoryMismatch,
    StaleBase,
    DirtyTarget,
    PlatformIncompatible,
    ApprovalConsumed,
    StateFailure,
}

pub(crate) struct DelegationApplyPreflightService {
    artifacts: Arc<dyn DelegationApplyArtifactPort>,
    targets: Arc<dyn DelegationApplyTargetPort>,
    once: Arc<dyn DelegationApplyOncePort>,
}

impl DelegationApplyPreflightService {
    pub(crate) fn new(
        artifacts: Arc<dyn DelegationApplyArtifactPort>,
        targets: Arc<dyn DelegationApplyTargetPort>,
        once: Arc<dyn DelegationApplyOncePort>,
    ) -> Self {
        Self {
            artifacts,
            targets,
            once,
        }
    }

    pub(crate) fn preflight(
        &self,
        request: DelegationApplyPreflightRequest,
    ) -> Result<DelegationApplyPlan, DelegationApplyPreflightError> {
        validate_request(&request)?;
        let available = self
            .once
            .is_available(&request.artifact_id, &request.approval_input_hash)
            .map_err(|_| DelegationApplyPreflightError::StateFailure)?;
        if !available {
            return Err(DelegationApplyPreflightError::ApprovalConsumed);
        }
        let artifact = self
            .artifacts
            .load_apply_evidence(&request.artifact_id)
            .map_err(|_| DelegationApplyPreflightError::ArtifactUnavailable)?;
        verify_artifact(&request, &artifact)?;
        let target = self
            .targets
            .inspect_target(&request.target_root)
            .map_err(|_| DelegationApplyPreflightError::TargetUnavailable)?;
        verify_target(&request, &target)?;
        Ok(DelegationApplyPlan {
            target_root: target.canonical_root,
            artifact,
            approval_input_hash: request.approval_input_hash,
        })
    }
}

fn validate_request(
    request: &DelegationApplyPreflightRequest,
) -> Result<(), DelegationApplyPreflightError> {
    if !request.target_root.is_absolute()
        || [
            request.artifact_id.as_str(),
            request.expected_content_hash.as_str(),
            request.expected_diff_hash.as_str(),
            request.expected_repository_identity.as_str(),
            request.expected_base_commit.as_str(),
            request.approval_input_hash.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(DelegationApplyPreflightError::InvalidRequest);
    }
    Ok(())
}

fn verify_artifact(
    request: &DelegationApplyPreflightRequest,
    artifact: &DelegationApplyArtifactEvidence,
) -> Result<(), DelegationApplyPreflightError> {
    if !artifact.integrity_verified
        || artifact.artifact_id != request.artifact_id
        || artifact.content_hash != request.expected_content_hash
        || artifact.capture.diff_hash != request.expected_diff_hash
    {
        return Err(DelegationApplyPreflightError::IntegrityFailure);
    }
    if !artifact.applyable
        || artifact.repository_identity != request.expected_repository_identity
        || artifact.capture.base_commit != request.expected_base_commit
    {
        return Err(DelegationApplyPreflightError::RepositoryMismatch);
    }
    DelegationChangeSetPolicy::validate(&artifact.capture, DelegationChangeSetLimits::HARD_CEILING)
        .map_err(|_| DelegationApplyPreflightError::PlatformIncompatible)
}

fn verify_target(
    request: &DelegationApplyPreflightRequest,
    target: &DelegationApplyTargetWitness,
) -> Result<(), DelegationApplyPreflightError> {
    if target.canonical_root != request.target_root
        || target.repository_identity != request.expected_repository_identity
    {
        return Err(DelegationApplyPreflightError::RepositoryMismatch);
    }
    if !target
        .head_commit
        .eq_ignore_ascii_case(&request.expected_base_commit)
    {
        return Err(DelegationApplyPreflightError::StaleBase);
    }
    if !target.worktree_clean || !target.index_clean {
        return Err(DelegationApplyPreflightError::DirtyTarget);
    }
    if !target.path_compatible {
        return Err(DelegationApplyPreflightError::PlatformIncompatible);
    }
    Ok(())
}

#[cfg(test)]
#[path = "apply_preflight_tests.rs"]
mod tests;
