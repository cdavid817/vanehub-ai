use super::{DelegationApplyPlan, DelegationRecoveryCapsule};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationRecoveryOutcome {
    RolledBack,
    ManualRecoveryRequired,
}

pub(crate) trait DelegationApplyRecoveryPort: Send + Sync {
    fn restore_from_capsule(
        &self,
        plan: &DelegationApplyPlan,
        capsule: &DelegationRecoveryCapsule,
    ) -> Result<(), ()>;

    fn verify_pre_apply_witness(
        &self,
        plan: &DelegationApplyPlan,
        capsule: &DelegationRecoveryCapsule,
    ) -> Result<bool, ()>;

    fn persist_recovery(
        &self,
        apply_attempt_id: &str,
        outcome: DelegationRecoveryOutcome,
        capsule_reference: Option<&str>,
    ) -> Result<(), ()>;

    fn remove_capsule(&self, capsule: &DelegationRecoveryCapsule) -> Result<(), ()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationApplyRecoveryError {
    InvalidRequest,
    PersistenceFailure,
}

pub(crate) struct DelegationApplyRecoveryService {
    port: Arc<dyn DelegationApplyRecoveryPort>,
}

impl DelegationApplyRecoveryService {
    pub(crate) fn new(port: Arc<dyn DelegationApplyRecoveryPort>) -> Self {
        Self { port }
    }

    pub(crate) fn recover(
        &self,
        plan: &DelegationApplyPlan,
        capsule: &DelegationRecoveryCapsule,
    ) -> Result<DelegationRecoveryOutcome, DelegationApplyRecoveryError> {
        if capsule.apply_attempt_id.trim().is_empty() || capsule.reference.trim().is_empty() {
            return Err(DelegationApplyRecoveryError::InvalidRequest);
        }
        let restored = self.port.restore_from_capsule(plan, capsule).is_ok();
        let verified = restored
            && self
                .port
                .verify_pre_apply_witness(plan, capsule)
                .unwrap_or(false);
        let outcome = if verified {
            DelegationRecoveryOutcome::RolledBack
        } else {
            DelegationRecoveryOutcome::ManualRecoveryRequired
        };
        let retained_reference = (outcome == DelegationRecoveryOutcome::ManualRecoveryRequired)
            .then_some(capsule.reference.as_str());
        self.port
            .persist_recovery(&capsule.apply_attempt_id, outcome, retained_reference)
            .map_err(|_| DelegationApplyRecoveryError::PersistenceFailure)?;
        if outcome == DelegationRecoveryOutcome::RolledBack {
            self.port
                .remove_capsule(capsule)
                .map_err(|_| DelegationApplyRecoveryError::PersistenceFailure)?;
        }
        Ok(outcome)
    }
}

#[cfg(test)]
#[path = "apply_recovery_tests.rs"]
mod tests;
