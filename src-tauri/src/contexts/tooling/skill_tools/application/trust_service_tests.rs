use super::*;
use crate::contexts::tooling::skill_tools::domain::{
    ContentHash, SkillToolDiagnosticSummary, SkillToolId, SkillToolIntegrity, SkillToolLifecycle,
    SkillToolOwnerId, SkillToolRevision, SkillToolSourceScope,
};
use std::sync::Mutex;

struct Repository {
    state: Mutex<SkillToolRevisionState>,
    record: Mutex<Option<SkillToolTrustRecord>>,
}

impl SkillToolTrustRepository for Repository {
    fn state(
        &self,
        key: &SkillToolKey,
    ) -> Result<Option<SkillToolRevisionState>, SkillToolApplicationError> {
        let state = self.state.lock().expect("state");
        Ok((state.key == *key).then(|| state.clone()))
    }

    fn record(
        &self,
        _key: &SkillToolKey,
    ) -> Result<Option<SkillToolTrustRecord>, SkillToolApplicationError> {
        Ok(self.record.lock().expect("record").clone())
    }

    fn save(
        &self,
        record: &SkillToolTrustRecord,
        _decision: SkillToolTrustDecision,
    ) -> Result<(), SkillToolApplicationError> {
        *self.record.lock().expect("record") = Some(record.clone());
        Ok(())
    }
}

struct Clock;

impl SkillToolClockPort for Clock {
    fn now(&self) -> String {
        "2026-08-17T01:00:00Z".to_string()
    }
}

fn fixture() -> (Repository, SkillToolKey, SkillToolIntegrity) {
    let revision = SkillToolRevision::parse(&"a".repeat(64)).expect("revision");
    let key = SkillToolKey::new(
        SkillToolOwnerId::parse("review").expect("owner"),
        SkillToolSourceScope::global(),
        SkillToolId::parse("check").expect("tool"),
        revision,
    );
    let integrity = SkillToolIntegrity {
        base_revision: "base".to_string(),
        manifest_hash: ContentHash::from_digest(&"b".repeat(64)),
        implementation_hash: ContentHash::from_digest(&"c".repeat(64)),
        capability_digest: format!("sha256:{}", "d".repeat(64)),
    };
    let state = SkillToolRevisionState {
        key: key.clone(),
        integrity: integrity.clone(),
        implementation_kind: "wasm".to_string(),
        lifecycle: SkillToolLifecycle {
            validation: SkillToolValidationState::Valid,
            ..SkillToolLifecycle::default()
        },
        validation_code: Some("clean".to_string()),
        diagnostics: SkillToolDiagnosticSummary::default(),
        created_at: "2026-08-17T00:00:00Z".to_string(),
        updated_at: "2026-08-17T00:00:00Z".to_string(),
    };
    (
        Repository {
            state: Mutex::new(state),
            record: Mutex::new(None),
        },
        key,
        integrity,
    )
}

#[test]
fn trust_and_revoke_bind_actor_time_and_every_integrity_value() {
    let (repository, key, integrity) = fixture();
    let service = SkillToolTrustService::new(&repository, &Clock);
    let trusted = service.trust(&key, &integrity, "operator").expect("trust");
    assert_eq!(trusted.actor, "operator");
    assert_eq!(trusted.decided_at, "2026-08-17T01:00:00Z");
    assert!(service.authorizes(&key, &integrity).expect("authorize"));

    let mut changed = integrity.clone();
    changed.capability_digest = format!("sha256:{}", "e".repeat(64));
    assert!(!service.authorizes(&key, &changed).expect("changed"));
    assert!(matches!(
        service.trust(&key, &changed, "operator"),
        Err(SkillToolApplicationError::StaleRevision)
    ));

    let revoked = service
        .revoke(&key, &integrity, "operator")
        .expect("revoke");
    assert_eq!(revoked.decision, SkillToolTrustDecision::Revoked);
    assert!(!service.authorizes(&key, &integrity).expect("revoked"));
}

#[test]
fn invalid_revision_and_invalid_actor_cannot_be_trusted() {
    let (repository, key, integrity) = fixture();
    repository.state.lock().expect("state").lifecycle.validation =
        SkillToolValidationState::Invalid;
    let service = SkillToolTrustService::new(&repository, &Clock);
    assert!(service.trust(&key, &integrity, "operator").is_err());
    assert!(service.trust(&key, &integrity, " ").is_err());
}
