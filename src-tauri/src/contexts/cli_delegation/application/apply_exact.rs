use super::{DelegationApplyPlan, DelegationRecoveryCapsule};
use std::sync::Arc;

pub(crate) struct DelegationExactApplyRequest<'a> {
    pub(crate) plan: &'a DelegationApplyPlan,
    pub(crate) capsule: &'a DelegationRecoveryCapsule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationExactApplyWitness {
    pub(crate) applied_diff_hash: String,
    pub(crate) complete_patch_applied: bool,
    pub(crate) index_unchanged: bool,
    pub(crate) network_used: bool,
    pub(crate) history_operation_used: bool,
    pub(crate) partial_success: bool,
}

pub(crate) trait DelegationExactApplyPort: Send + Sync {
    fn apply_exact(
        &self,
        request: DelegationExactApplyRequest<'_>,
    ) -> Result<DelegationExactApplyWitness, ()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationExactApplyError {
    InvalidPlan,
    ApplyFailure,
    IncompleteApplication,
    ForbiddenSideEffect,
    IntegrityFailure,
}

pub(crate) struct DelegationExactApplyService {
    port: Arc<dyn DelegationExactApplyPort>,
}

impl DelegationExactApplyService {
    pub(crate) fn new(port: Arc<dyn DelegationExactApplyPort>) -> Self {
        Self { port }
    }

    pub(crate) fn apply(
        &self,
        plan: &DelegationApplyPlan,
        capsule: &DelegationRecoveryCapsule,
    ) -> Result<DelegationExactApplyWitness, DelegationExactApplyError> {
        if capsule.apply_attempt_id.trim().is_empty()
            || capsule.reference.trim().is_empty()
            || plan.artifact.capture.canonical_patch.is_empty()
        {
            return Err(DelegationExactApplyError::InvalidPlan);
        }
        let witness = self
            .port
            .apply_exact(DelegationExactApplyRequest { plan, capsule })
            .map_err(|_| DelegationExactApplyError::ApplyFailure)?;
        if witness.partial_success || !witness.complete_patch_applied {
            return Err(DelegationExactApplyError::IncompleteApplication);
        }
        if !witness.index_unchanged || witness.network_used || witness.history_operation_used {
            return Err(DelegationExactApplyError::ForbiddenSideEffect);
        }
        if witness.applied_diff_hash != plan.artifact.capture.diff_hash {
            return Err(DelegationExactApplyError::IntegrityFailure);
        }
        Ok(witness)
    }
}

#[cfg(test)]
#[path = "apply_exact_tests.rs"]
mod tests;
