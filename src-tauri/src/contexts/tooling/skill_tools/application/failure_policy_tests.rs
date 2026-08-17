use super::*;
use crate::contexts::tooling::skill_tools::domain::{
    ContentHash, SkillToolId, SkillToolIntegrity, SkillToolKey, SkillToolLifecycle,
    SkillToolOwnerId, SkillToolRevision, SkillToolSourceScope,
};
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Default)]
struct Repository(Mutex<BTreeMap<SkillToolRevision, SkillToolRevisionState>>);

impl SkillToolLifecycleRepository for Repository {
    fn load(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<Option<SkillToolRevisionState>, SkillToolApplicationError> {
        Ok(self.0.lock().expect("states").get(revision).cloned())
    }

    fn save(&self, state: &SkillToolRevisionState) -> Result<(), SkillToolApplicationError> {
        self.0
            .lock()
            .expect("states")
            .insert(state.key.revision.clone(), state.clone());
        Ok(())
    }
}

struct Clock;

impl SkillToolClockPort for Clock {
    fn now(&self) -> String {
        "2026-08-17T00:00:00Z".to_string()
    }
}

fn state(fill: char) -> SkillToolRevisionState {
    let revision = SkillToolRevision::parse(&fill.to_string().repeat(64)).expect("revision");
    SkillToolRevisionState {
        key: SkillToolKey::new(
            SkillToolOwnerId::parse("review").expect("owner"),
            SkillToolSourceScope::global(),
            SkillToolId::parse("check").expect("tool"),
            revision,
        ),
        integrity: SkillToolIntegrity {
            base_revision: "base".to_string(),
            manifest_hash: ContentHash::from_digest(&"a".repeat(64)),
            implementation_hash: ContentHash::from_digest(&"b".repeat(64)),
            capability_digest: format!("sha256:{}", "c".repeat(64)),
        },
        implementation_kind: "wasm".to_string(),
        lifecycle: SkillToolLifecycle::default(),
        validation_code: Some("clean".to_string()),
        diagnostics: Default::default(),
        created_at: "2026-08-16T00:00:00Z".to_string(),
        updated_at: "2026-08-16T00:00:00Z".to_string(),
    }
}

#[test]
fn deterministic_failures_quarantine_only_the_exact_revision() {
    let repository = Repository::default();
    let affected = state('a');
    let unrelated = state('b');
    repository
        .save(&affected)
        .and_then(|()| repository.save(&unrelated))
        .expect("seed");
    let policy = SkillToolFailurePolicy::new(&repository, &Clock);
    let trapped = SkillToolModuleOutcome::Trapped {
        detail: "untrusted raw trap detail".to_string(),
    };
    assert!(!policy
        .record(&affected.key.revision, &trapped)
        .expect("first"));
    assert!(!policy
        .record(&affected.key.revision, &trapped)
        .expect("second"));
    assert!(policy
        .record(&affected.key.revision, &trapped)
        .expect("third"));

    let states = repository.0.lock().expect("states");
    let affected = states.get(&affected.key.revision).expect("affected");
    let unrelated = states.get(&unrelated.key.revision).expect("unrelated");
    assert!(affected.lifecycle.quarantine.is_quarantined());
    assert_eq!(
        affected.lifecycle.quarantine.reason(),
        Some("module-trapped")
    );
    assert_eq!(affected.lifecycle.consecutive_failures, 3);
    assert!(!unrelated.lifecycle.quarantine.is_quarantined());
    assert_eq!(unrelated.lifecycle.consecutive_failures, 0);
    assert!(!affected.diagnostics.entries()[0]
        .detail
        .contains("untrusted raw trap detail"));
}

#[test]
fn cancellation_is_not_a_failure_and_success_resets_only_its_own_streak() {
    let repository = Repository::default();
    let revision = state('c');
    repository.save(&revision).expect("seed");
    let policy = SkillToolFailurePolicy::new(&repository, &Clock);
    policy
        .record(&revision.key.revision, &SkillToolModuleOutcome::Cancelled)
        .expect("cancel");
    policy
        .record(
            &revision.key.revision,
            &SkillToolModuleOutcome::LimitBreached {
                limit: "fuel".to_string(),
            },
        )
        .expect("failure");
    policy
        .record(
            &revision.key.revision,
            &SkillToolModuleOutcome::Completed(serde_json::Value::Null),
        )
        .expect("success");
    let states = repository.0.lock().expect("states");
    assert_eq!(
        states
            .get(&revision.key.revision)
            .expect("state")
            .lifecycle
            .consecutive_failures,
        0
    );
}
