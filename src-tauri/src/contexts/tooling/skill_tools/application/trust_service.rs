use super::{
    SkillToolApplicationError, SkillToolClockPort, SkillToolRevisionState, SkillToolStateRepository,
};
use crate::contexts::tooling::skill_tools::domain::{
    SkillToolIntegrity, SkillToolKey, SkillToolTrustDecision, SkillToolTrustRecord,
    SkillToolValidationState,
};

pub(crate) trait SkillToolTrustRepository: Send + Sync {
    fn state(
        &self,
        key: &SkillToolKey,
    ) -> Result<Option<SkillToolRevisionState>, SkillToolApplicationError>;
    fn record(
        &self,
        key: &SkillToolKey,
    ) -> Result<Option<SkillToolTrustRecord>, SkillToolApplicationError>;
    fn save(
        &self,
        record: &SkillToolTrustRecord,
        decision: SkillToolTrustDecision,
    ) -> Result<(), SkillToolApplicationError>;
}

impl<T: SkillToolStateRepository + ?Sized> SkillToolTrustRepository for T {
    fn state(
        &self,
        key: &SkillToolKey,
    ) -> Result<Option<SkillToolRevisionState>, SkillToolApplicationError> {
        self.revision_state(&key.revision)
    }

    fn record(
        &self,
        key: &SkillToolKey,
    ) -> Result<Option<SkillToolTrustRecord>, SkillToolApplicationError> {
        self.trust_record(&key.revision)
    }

    fn save(
        &self,
        record: &SkillToolTrustRecord,
        decision: SkillToolTrustDecision,
    ) -> Result<(), SkillToolApplicationError> {
        self.save_trust(record, decision)
    }
}

pub(crate) struct SkillToolTrustService<'a> {
    repository: &'a dyn SkillToolTrustRepository,
    clock: &'a dyn SkillToolClockPort,
}

impl<'a> SkillToolTrustService<'a> {
    pub(crate) fn new(
        repository: &'a dyn SkillToolTrustRepository,
        clock: &'a dyn SkillToolClockPort,
    ) -> Self {
        Self { repository, clock }
    }

    pub(crate) fn trust(
        &self,
        key: &SkillToolKey,
        integrity: &SkillToolIntegrity,
        actor: &str,
    ) -> Result<SkillToolTrustRecord, SkillToolApplicationError> {
        self.decide(key, integrity, actor, SkillToolTrustDecision::Trusted)
    }

    pub(crate) fn revoke(
        &self,
        key: &SkillToolKey,
        integrity: &SkillToolIntegrity,
        actor: &str,
    ) -> Result<SkillToolTrustRecord, SkillToolApplicationError> {
        self.decide(key, integrity, actor, SkillToolTrustDecision::Revoked)
    }

    pub(crate) fn authorizes(
        &self,
        key: &SkillToolKey,
        integrity: &SkillToolIntegrity,
    ) -> Result<bool, SkillToolApplicationError> {
        Ok(self
            .repository
            .record(key)?
            .is_some_and(|record| record.authorizes(&key.revision, integrity)))
    }

    fn decide(
        &self,
        key: &SkillToolKey,
        integrity: &SkillToolIntegrity,
        actor: &str,
        decision: SkillToolTrustDecision,
    ) -> Result<SkillToolTrustRecord, SkillToolApplicationError> {
        let actor = actor.trim();
        if actor.is_empty() || actor.chars().count() > 128 {
            return Err(SkillToolApplicationError::HostDenied(
                "trust-actor".to_string(),
            ));
        }
        let state = self.repository.state(key)?.ok_or_else(|| {
            SkillToolApplicationError::NotFound(key.revision.as_str().to_string())
        })?;
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
        let record = SkillToolTrustRecord {
            revision: key.revision.clone(),
            integrity: integrity.clone(),
            decision,
            actor: actor.to_string(),
            decided_at: self.clock.now(),
        };
        self.repository.save(&record, decision)?;
        Ok(record)
    }
}

#[cfg(test)]
#[path = "trust_service_tests.rs"]
mod tests;
