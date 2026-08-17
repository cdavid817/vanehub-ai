use super::{
    SkillToolApplicationError, SkillToolClockPort, SkillToolModuleOutcome, SkillToolRevisionState,
    SkillToolStateRepository,
};
use crate::contexts::tooling::skill_tools::domain::{
    SkillToolDiagnostic, SkillToolDiagnosticSeverity, SkillToolRevision,
};

pub(crate) trait SkillToolLifecycleRepository: Send + Sync {
    fn load(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<Option<SkillToolRevisionState>, SkillToolApplicationError>;

    fn save(&self, state: &SkillToolRevisionState) -> Result<(), SkillToolApplicationError>;
}

impl<T: SkillToolStateRepository> SkillToolLifecycleRepository for T {
    fn load(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<Option<SkillToolRevisionState>, SkillToolApplicationError> {
        self.revision_state(revision)
    }

    fn save(&self, state: &SkillToolRevisionState) -> Result<(), SkillToolApplicationError> {
        self.save_lifecycle(
            &state.key.revision,
            &state.lifecycle,
            state.validation_code.as_deref(),
            &state.diagnostics,
            &state.updated_at,
        )
    }
}

pub(crate) struct SkillToolFailurePolicy<'a> {
    repository: &'a dyn SkillToolLifecycleRepository,
    clock: &'a dyn SkillToolClockPort,
}

impl<'a> SkillToolFailurePolicy<'a> {
    pub(crate) fn new(
        repository: &'a dyn SkillToolLifecycleRepository,
        clock: &'a dyn SkillToolClockPort,
    ) -> Self {
        Self { repository, clock }
    }

    pub(crate) fn record(
        &self,
        revision: &SkillToolRevision,
        outcome: &SkillToolModuleOutcome,
    ) -> Result<bool, SkillToolApplicationError> {
        let Some(mut state) = self.repository.load(revision)? else {
            return Err(SkillToolApplicationError::NotFound(
                revision.as_str().to_string(),
            ));
        };
        let newly_quarantined = match outcome {
            SkillToolModuleOutcome::Completed(_) => {
                state.lifecycle.record_success();
                false
            }
            SkillToolModuleOutcome::LimitBreached { limit } => {
                let code = bounded_code("limit", limit);
                state.diagnostics.push(SkillToolDiagnostic::new(
                    SkillToolDiagnosticSeverity::Error,
                    code.clone(),
                    "module resource limit breached",
                ));
                state.lifecycle.record_failure(&code)
            }
            SkillToolModuleOutcome::Trapped { .. } => {
                let code = "module-trapped";
                state.diagnostics.push(SkillToolDiagnostic::new(
                    SkillToolDiagnosticSeverity::Error,
                    code,
                    "module trapped",
                ));
                state.lifecycle.record_failure(code)
            }
            SkillToolModuleOutcome::Cancelled => return Ok(false),
        };
        state.updated_at = self.clock.now();
        self.repository.save(&state)?;
        Ok(newly_quarantined)
    }
}

fn bounded_code(prefix: &str, value: &str) -> String {
    format!("{prefix}-{value}").chars().take(64).collect()
}

#[cfg(test)]
#[path = "failure_policy_tests.rs"]
mod tests;
