use super::{
    SkillToolApplicationError, SkillToolClockPort, SkillToolPackageRef, SkillToolRevisionState,
    SkillToolStateRepository,
};
use crate::contexts::tooling::skill_tools::domain::{
    SkillToolDiagnostic, SkillToolDiagnosticSeverity, SkillToolIntegrity, SkillToolKey,
    SkillToolQuarantine, SkillToolRevision, SkillToolTrustDecision, SkillToolTrustRecord,
    SkillToolValidationState,
};

pub(crate) trait SkillToolRevisionValidationPort: Send + Sync {
    fn validate(&self, state: &SkillToolRevisionState) -> Result<(), SkillToolApplicationError>;
}

#[cfg(test)]
#[path = "governance_tests.rs"]
mod tests;

pub(crate) struct SkillToolGovernanceService<'a> {
    repository: &'a dyn SkillToolStateRepository,
    validator: &'a dyn SkillToolRevisionValidationPort,
    clock: &'a dyn SkillToolClockPort,
}

impl<'a> SkillToolGovernanceService<'a> {
    pub(crate) fn new(
        repository: &'a dyn SkillToolStateRepository,
        validator: &'a dyn SkillToolRevisionValidationPort,
        clock: &'a dyn SkillToolClockPort,
    ) -> Self {
        Self {
            repository,
            validator,
            clock,
        }
    }

    pub(crate) fn list(
        &self,
        owner: &SkillToolPackageRef,
    ) -> Result<Vec<SkillToolRevisionState>, SkillToolApplicationError> {
        self.repository.revision_states(owner)
    }

    pub(crate) fn diagnostics(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        self.state(revision)
    }

    pub(crate) fn validate(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        let mut state = self.state(revision)?;
        let result = self.validator.validate(&state);
        state.lifecycle.validation = if result.is_ok() {
            SkillToolValidationState::Valid
        } else {
            SkillToolValidationState::Invalid
        };
        state.validation_code = Some(
            result
                .as_ref()
                .map(|()| "clean")
                .unwrap_or_else(|error| error.code())
                .to_string(),
        );
        if let Err(error) = &result {
            state.lifecycle.enabled = false;
            state.diagnostics.push(SkillToolDiagnostic::new(
                SkillToolDiagnosticSeverity::Error,
                error.code(),
                "revision validation failed",
            ));
        }
        self.save(&mut state)?;
        result.map(|()| state)
    }

    pub(crate) fn decide_trust(
        &self,
        key: &SkillToolKey,
        integrity: &SkillToolIntegrity,
        actor: &str,
        decision: SkillToolTrustDecision,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        let actor = actor.trim();
        if actor.is_empty() || actor.chars().count() > 128 {
            return Err(SkillToolApplicationError::HostDenied(
                "trust-actor".to_string(),
            ));
        }
        let state = self.state(&key.revision)?;
        if state.key != *key || state.integrity != *integrity {
            return Err(SkillToolApplicationError::StaleRevision);
        }
        if decision == SkillToolTrustDecision::Trusted
            && state.lifecycle.validation != SkillToolValidationState::Valid
        {
            return Err(SkillToolApplicationError::HostDenied(
                "trust-validation".to_string(),
            ));
        }
        self.repository.save_trust(
            &SkillToolTrustRecord {
                revision: key.revision.clone(),
                integrity: integrity.clone(),
                decision,
                actor: actor.to_string(),
                decided_at: self.clock.now(),
            },
            decision,
        )?;
        self.state(&key.revision)
    }

    pub(crate) fn set_enabled(
        &self,
        revision: &SkillToolRevision,
        enabled: bool,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        let mut state = self.state(revision)?;
        if enabled
            && (state.lifecycle.validation != SkillToolValidationState::Valid
                || !state.lifecycle.trusted
                || state.lifecycle.quarantine.is_quarantined())
        {
            return Err(SkillToolApplicationError::HostDenied(
                "enablement-gates".to_string(),
            ));
        }
        state.lifecycle.enabled = enabled;
        self.save(&mut state)?;
        Ok(state)
    }

    pub(crate) fn quarantine(
        &self,
        revision: &SkillToolRevision,
        reason: &str,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        let mut state = self.state(revision)?;
        state.lifecycle.quarantine = SkillToolQuarantine::Quarantined {
            reason: reason.chars().take(256).collect(),
        };
        state.lifecycle.enabled = false;
        self.save(&mut state)?;
        Ok(state)
    }

    pub(crate) fn recover(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        let mut state = self.validate(revision)?;
        if !state.lifecycle.recover(SkillToolValidationState::Valid) {
            return Err(SkillToolApplicationError::HostDenied(
                "recovery-validation".to_string(),
            ));
        }
        self.save(&mut state)?;
        Ok(state)
    }

    fn state(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<SkillToolRevisionState, SkillToolApplicationError> {
        self.repository
            .revision_state(revision)?
            .ok_or_else(|| SkillToolApplicationError::NotFound(revision.as_str().to_string()))
    }

    fn save(&self, state: &mut SkillToolRevisionState) -> Result<(), SkillToolApplicationError> {
        state.updated_at = self.clock.now();
        self.repository.save_lifecycle(
            &state.key.revision,
            &state.lifecycle,
            state.validation_code.as_deref(),
            &state.diagnostics,
            &state.updated_at,
        )
    }
}
