use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationInterruptedApply {
    pub(crate) apply_attempt_id: String,
    pub(crate) capsule_reference: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelegationRestartWitness {
    pub(crate) post_apply_matches: bool,
    pub(crate) pre_apply_matches: bool,
    pub(crate) capsule_integral: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationRestartResolution {
    SafelyCompleted,
    RolledBack,
    ManualRecoveryRequired,
}

pub(crate) trait DelegationRestartRecoveryPort: Send + Sync {
    fn list_interrupted(&self) -> Result<Vec<DelegationInterruptedApply>, ()>;
    fn inspect(&self, apply: &DelegationInterruptedApply) -> Result<DelegationRestartWitness, ()>;
    fn persist_resolution(
        &self,
        apply: &DelegationInterruptedApply,
        resolution: DelegationRestartResolution,
    ) -> Result<(), ()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationRestartRecoveryError {
    StateFailure,
    InvalidRecord,
}

pub(crate) struct DelegationRestartRecoveryService {
    port: Arc<dyn DelegationRestartRecoveryPort>,
}

impl DelegationRestartRecoveryService {
    pub(crate) fn new(port: Arc<dyn DelegationRestartRecoveryPort>) -> Self {
        Self { port }
    }

    pub(crate) fn reconcile(
        &self,
    ) -> Result<Vec<(String, DelegationRestartResolution)>, DelegationRestartRecoveryError> {
        let interrupted = self
            .port
            .list_interrupted()
            .map_err(|_| DelegationRestartRecoveryError::StateFailure)?;
        let mut outcomes = Vec::with_capacity(interrupted.len());
        for apply in interrupted {
            if apply.apply_attempt_id.trim().is_empty() || apply.capsule_reference.trim().is_empty()
            {
                return Err(DelegationRestartRecoveryError::InvalidRecord);
            }
            let witness = self
                .port
                .inspect(&apply)
                .unwrap_or(DelegationRestartWitness {
                    post_apply_matches: false,
                    pre_apply_matches: false,
                    capsule_integral: false,
                });
            let resolution = classify(witness);
            self.port
                .persist_resolution(&apply, resolution)
                .map_err(|_| DelegationRestartRecoveryError::StateFailure)?;
            outcomes.push((apply.apply_attempt_id, resolution));
        }
        Ok(outcomes)
    }
}

fn classify(witness: DelegationRestartWitness) -> DelegationRestartResolution {
    if witness.post_apply_matches && !witness.pre_apply_matches {
        DelegationRestartResolution::SafelyCompleted
    } else if witness.pre_apply_matches && witness.capsule_integral && !witness.post_apply_matches {
        DelegationRestartResolution::RolledBack
    } else {
        DelegationRestartResolution::ManualRecoveryRequired
    }
}

#[cfg(test)]
#[path = "restart_recovery_tests.rs"]
mod tests;
