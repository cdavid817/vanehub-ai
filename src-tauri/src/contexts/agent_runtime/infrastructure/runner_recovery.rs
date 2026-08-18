use crate::contexts::agent_runtime::application::{
    AgentRunner, RunnerInspection, RunnerKind, RunnerRecoveryMode, RunnerReference,
};
use crate::contexts::operations::api::{
    AgentRun, OperationsError, RunOwnerRecoveryPort, RunRecoveryDecision, RunRunnerKind,
    RunRunnerRecovery,
};
use std::sync::Arc;

pub(crate) struct RunnerRunRecoveryAdapter {
    runners: Arc<dyn AgentRunner>,
}

impl RunnerRunRecoveryAdapter {
    pub(crate) fn new(runners: Arc<dyn AgentRunner>) -> Self {
        Self { runners }
    }
}

impl RunOwnerRecoveryPort for RunnerRunRecoveryAdapter {
    fn reconcile(&self, run: &AgentRun) -> Result<RunRecoveryDecision, OperationsError> {
        let Some(runner) = &run.runner else {
            return Ok(RunRecoveryDecision::Interrupted);
        };
        let reference = RunnerReference {
            kind: match runner.kind {
                RunRunnerKind::Local => RunnerKind::Local,
                RunRunnerKind::Ssh => RunnerKind::Ssh,
            },
            target_id: runner.target_id.clone(),
            target_revision: runner.target_revision,
            recovery: match runner.recovery {
                RunRunnerRecovery::None => RunnerRecoveryMode::None,
                RunRunnerRecovery::InspectOnly => RunnerRecoveryMode::InspectOnly,
                RunRunnerRecovery::Reattach => RunnerRecoveryMode::Reattach,
            },
            authority_witness: runner.authority_witness.clone(),
        };
        if reference.recovery == RunnerRecoveryMode::None {
            return Ok(RunRecoveryDecision::Interrupted);
        }
        Ok(
            match self
                .runners
                .recover(&reference, runner.recovery_reference.as_deref())
            {
                // Inspect-only can prove liveness but cannot prove output continuity, so it requires
                // attention instead of pretending the original stream was reattached.
                Ok(RunnerInspection::Running | RunnerInspection::Disconnected) => {
                    RunRecoveryDecision::Blocked
                }
                Ok(RunnerInspection::Exited(_) | RunnerInspection::Unknown) | Err(_) => {
                    RunRecoveryDecision::Interrupted
                }
            },
        )
    }
}

#[cfg(test)]
#[path = "runner_recovery_tests.rs"]
mod tests;
