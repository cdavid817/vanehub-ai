use super::*;
use crate::contexts::tooling::skill_tools::domain::{
    ContentHash, SkillToolDiagnosticSummary, SkillToolId, SkillToolLifecycle, SkillToolOwnerId,
    SkillToolRevision, SkillToolSourceScope, SkillToolTrustRecord,
};
use std::sync::Mutex;

struct Repository(Mutex<SkillToolRevisionState>);

impl SkillToolStateRepository for Repository {
    fn revision_states(
        &self,
        _owner: &SkillToolPackageRef,
    ) -> Result<Vec<SkillToolRevisionState>, SkillToolApplicationError> {
        Ok(vec![self.0.lock().expect("state").clone()])
    }

    fn revision_state(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<Option<SkillToolRevisionState>, SkillToolApplicationError> {
        let state = self.0.lock().expect("state").clone();
        Ok((state.key.revision == *revision).then_some(state))
    }

    fn record_discovered(
        &self,
        _state: &SkillToolRevisionState,
    ) -> Result<(), SkillToolApplicationError> {
        Ok(())
    }

    fn save_lifecycle(
        &self,
        _revision: &SkillToolRevision,
        lifecycle: &SkillToolLifecycle,
        validation_code: Option<&str>,
        diagnostics: &SkillToolDiagnosticSummary,
        updated_at: &str,
    ) -> Result<(), SkillToolApplicationError> {
        let mut state = self.0.lock().expect("state");
        state.lifecycle = lifecycle.clone();
        state.validation_code = validation_code.map(str::to_string);
        state.diagnostics = diagnostics.clone();
        state.updated_at = updated_at.to_string();
        Ok(())
    }

    fn trust_record(
        &self,
        _revision: &SkillToolRevision,
    ) -> Result<Option<SkillToolTrustRecord>, SkillToolApplicationError> {
        Ok(None)
    }

    fn save_trust(
        &self,
        _record: &SkillToolTrustRecord,
        decision: SkillToolTrustDecision,
    ) -> Result<(), SkillToolApplicationError> {
        let mut state = self.0.lock().expect("state");
        state.lifecycle.trusted = decision == SkillToolTrustDecision::Trusted;
        if decision == SkillToolTrustDecision::Revoked {
            state.lifecycle.enabled = false;
        }
        Ok(())
    }
}

struct Validator(bool);

impl SkillToolRevisionValidationPort for Validator {
    fn validate(&self, _state: &SkillToolRevisionState) -> Result<(), SkillToolApplicationError> {
        self.0
            .then_some(())
            .ok_or_else(|| SkillToolApplicationError::IntegrityMismatch {
                path: "redacted".to_string(),
            })
    }
}

struct Clock;

impl SkillToolClockPort for Clock {
    fn now(&self) -> String {
        "2026-08-17T00:00:00Z".to_string()
    }
}

fn state() -> SkillToolRevisionState {
    let revision = SkillToolRevision::parse(&"a".repeat(64)).expect("revision");
    SkillToolRevisionState {
        key: SkillToolKey::new(
            SkillToolOwnerId::parse("review").expect("owner"),
            SkillToolSourceScope::global(),
            SkillToolId::parse("check").expect("tool"),
            revision,
        ),
        integrity: SkillToolIntegrity {
            base_revision: "base".to_string(),
            manifest_hash: ContentHash::parse(&format!("sha256:{}", "b".repeat(64)))
                .expect("manifest"),
            implementation_hash: ContentHash::parse(&format!("sha256:{}", "c".repeat(64)))
                .expect("implementation"),
            capability_digest: "digest".to_string(),
        },
        implementation_kind: "declarative".to_string(),
        lifecycle: SkillToolLifecycle::default(),
        validation_code: None,
        diagnostics: SkillToolDiagnosticSummary::default(),
        created_at: "created".to_string(),
        updated_at: "created".to_string(),
    }
}

#[test]
fn governance_requires_validation_then_separate_trust_and_enablement() {
    let repository = Repository(Mutex::new(state()));
    let service = SkillToolGovernanceService::new(&repository, &Validator(true), &Clock);
    let revision = repository.0.lock().expect("state").key.revision.clone();
    assert!(service.set_enabled(&revision, true).is_err());
    let validated = service.validate(&revision).expect("validate");
    assert_eq!(
        validated.lifecycle.validation,
        SkillToolValidationState::Valid
    );
    let trusted = service
        .decide_trust(
            &validated.key,
            &validated.integrity,
            "operator",
            SkillToolTrustDecision::Trusted,
        )
        .expect("trust");
    assert!(trusted.lifecycle.trusted);
    assert!(
        service
            .set_enabled(&revision, true)
            .expect("enable")
            .lifecycle
            .enabled
    );
}

#[test]
fn quarantine_disables_and_recovery_requires_clean_revalidation() {
    let repository = Repository(Mutex::new(state()));
    let service = SkillToolGovernanceService::new(&repository, &Validator(true), &Clock);
    let revision = repository.0.lock().expect("state").key.revision.clone();
    let quarantined = service
        .quarantine(&revision, "deterministic failure")
        .expect("quarantine");
    assert!(!quarantined.lifecycle.enabled);
    assert!(quarantined.lifecycle.quarantine.is_quarantined());
    let recovered = service.recover(&revision).expect("recover");
    assert!(!recovered.lifecycle.quarantine.is_quarantined());

    let failing = SkillToolGovernanceService::new(&repository, &Validator(false), &Clock);
    service.quarantine(&revision, "again").expect("quarantine");
    assert!(failing.recover(&revision).is_err());
    assert_eq!(
        repository.0.lock().expect("state").lifecycle.validation,
        SkillToolValidationState::Invalid
    );
}
