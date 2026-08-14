use super::{
    DelegationApplyPlan, DelegationChangeSetCapture, DelegationExactApplyWitness,
    DelegationRecoveryCapsule,
};
use std::sync::Arc;

pub(crate) struct DelegationPostApplyWitness {
    pub(crate) capture: DelegationChangeSetCapture,
    pub(crate) head_commit: String,
    pub(crate) index_clean: bool,
    pub(crate) mutation_lease_held: bool,
}

pub(crate) trait DelegationPostApplyVerificationPort: Send + Sync {
    fn capture_post_apply(
        &self,
        plan: &DelegationApplyPlan,
    ) -> Result<DelegationPostApplyWitness, ()>;

    fn record_success_and_consume(
        &self,
        apply_attempt_id: &str,
        artifact_id: &str,
        approval_input_hash: &str,
        diff_hash: &str,
    ) -> Result<bool, ()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationPostApplyVerificationError {
    InvalidRequest,
    CaptureFailure,
    LeaseLost,
    HeadChanged,
    IndexChanged,
    TreeMismatch,
    StateFailure,
    ApprovalReplay,
}

pub(crate) struct DelegationPostApplyVerificationService {
    port: Arc<dyn DelegationPostApplyVerificationPort>,
}

impl DelegationPostApplyVerificationService {
    pub(crate) fn new(port: Arc<dyn DelegationPostApplyVerificationPort>) -> Self {
        Self { port }
    }

    pub(crate) fn verify_and_complete(
        &self,
        plan: &DelegationApplyPlan,
        capsule: &DelegationRecoveryCapsule,
        application: &DelegationExactApplyWitness,
    ) -> Result<(), DelegationPostApplyVerificationError> {
        if capsule.apply_attempt_id.trim().is_empty()
            || application.applied_diff_hash != plan.artifact.capture.diff_hash
        {
            return Err(DelegationPostApplyVerificationError::InvalidRequest);
        }
        let actual = self
            .port
            .capture_post_apply(plan)
            .map_err(|_| DelegationPostApplyVerificationError::CaptureFailure)?;
        if !actual.mutation_lease_held {
            return Err(DelegationPostApplyVerificationError::LeaseLost);
        }
        if !actual
            .head_commit
            .eq_ignore_ascii_case(&plan.artifact.capture.base_commit)
        {
            return Err(DelegationPostApplyVerificationError::HeadChanged);
        }
        if !actual.index_clean {
            return Err(DelegationPostApplyVerificationError::IndexChanged);
        }
        if actual.capture != plan.artifact.capture {
            return Err(DelegationPostApplyVerificationError::TreeMismatch);
        }
        let consumed = self
            .port
            .record_success_and_consume(
                &capsule.apply_attempt_id,
                &plan.artifact.artifact_id,
                &plan.approval_input_hash,
                &plan.artifact.capture.diff_hash,
            )
            .map_err(|_| DelegationPostApplyVerificationError::StateFailure)?;
        if !consumed {
            return Err(DelegationPostApplyVerificationError::ApprovalReplay);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "apply_verification_tests.rs"]
mod tests;
