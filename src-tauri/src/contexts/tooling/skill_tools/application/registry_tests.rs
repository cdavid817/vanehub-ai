use super::*;
use crate::contexts::tooling::skill_tools::application::{
    SkillToolCatalogEntry, SkillToolOwnerKind,
};
use crate::contexts::tooling::skill_tools::domain::{
    SkillToolId, SkillToolLifecycle, SkillToolOwnerId, SkillToolRevision, SkillToolSourceScope,
    SkillToolValidationState,
};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Mutex;

#[derive(Default)]
struct RecordingArtifacts(Mutex<Vec<HashSet<SkillToolRevision>>>);

impl SkillToolCompiledArtifactPort for RecordingArtifacts {
    fn retain_revisions(&self, revisions: &HashSet<SkillToolRevision>) {
        self.0
            .lock()
            .expect("artifact records")
            .push(revisions.clone());
    }
}

fn candidate(owner: &str, revision: char) -> SkillToolCatalogCandidate {
    let key = SkillToolKey::new(
        SkillToolOwnerId::parse(owner).expect("owner"),
        SkillToolSourceScope::global(),
        SkillToolId::parse("check").expect("tool"),
        SkillToolRevision::parse(&revision.to_string().repeat(64)).expect("revision"),
    );
    SkillToolCatalogCandidate {
        entry: SkillToolCatalogEntry {
            canonical_name: key.canonical_name().expect("canonical name"),
            description: "Check".to_string(),
            input_schema: json!({"type": "object"}),
            key,
        },
        owner_kind: SkillToolOwnerKind::Role,
        lifecycle: SkillToolLifecycle {
            validation: SkillToolValidationState::Valid,
            trusted: true,
            enabled: true,
            ..SkillToolLifecycle::default()
        },
        archived: false,
        shadowed: false,
        requires_module_runtime: false,
        allow_plan: true,
    }
}

#[test]
fn every_security_or_skill_transition_atomically_replaces_the_snapshot() {
    let registry = SkillToolRegistry::empty();
    let causes = [
        SkillToolRegistryRefreshCause::Enablement,
        SkillToolRegistryRefreshCause::Archive,
        SkillToolRegistryRefreshCause::Delete,
        SkillToolRegistryRefreshCause::Replacement,
        SkillToolRegistryRefreshCause::Restore,
        SkillToolRegistryRefreshCause::EffectiveScope,
        SkillToolRegistryRefreshCause::Trust,
        SkillToolRegistryRefreshCause::Validation,
        SkillToolRegistryRefreshCause::Quarantine,
        SkillToolRegistryRefreshCause::GlobalKillSwitch,
        SkillToolRegistryRefreshCause::SkillKillSwitch,
    ];
    let revisions = ['a', 'b', 'c', 'd', 'e', 'f', '0', '1', '2', '3', '4'];

    for (index, cause) in causes.into_iter().enumerate() {
        let next = registry
            .refresh(cause, vec![candidate("review", revisions[index])])
            .expect("valid snapshot");
        assert_eq!(next.generation, index as u64 + 1);
        assert_eq!(registry.snapshot().cause, cause);
        assert!(Arc::ptr_eq(&next, &registry.snapshot()));
    }
}

#[test]
fn kill_switches_remove_atomically_and_restore_retained_evidence() {
    let registry = SkillToolRegistry::empty();
    let review = candidate("review", 'a');
    let other = candidate("other", 'b');
    registry
        .refresh(
            SkillToolRegistryRefreshCause::Restore,
            vec![review.clone(), other.clone()],
        )
        .expect("initial");

    registry
        .set_owner_execution_enabled(&review.entry.key.owner, false)
        .expect("disable owner");
    assert_eq!(registry.snapshot().candidates().len(), 1);
    assert_eq!(
        registry.snapshot().candidates()[0].entry.key,
        other.entry.key
    );
    registry
        .set_owner_execution_enabled(&review.entry.key.owner, true)
        .expect("restore owner");
    assert_eq!(registry.snapshot().candidates().len(), 2);

    registry
        .set_global_execution_enabled(false)
        .expect("global disable");
    assert!(registry.snapshot().candidates().is_empty());
    registry
        .set_global_execution_enabled(true)
        .expect("global restore");
    assert_eq!(registry.snapshot().candidates().len(), 2);
}

#[test]
fn invalid_or_colliding_content_never_replaces_the_last_good_snapshot() {
    let registry = SkillToolRegistry::empty();
    let good = registry
        .refresh(
            SkillToolRegistryRefreshCause::Restore,
            vec![candidate("review", 'a')],
        )
        .expect("good snapshot");
    let duplicate = candidate("review", 'a');
    assert!(registry
        .refresh(
            SkillToolRegistryRefreshCause::Replacement,
            vec![duplicate.clone(), duplicate],
        )
        .is_err());
    assert!(Arc::ptr_eq(&good, &registry.snapshot()));

    let mut forged = candidate("review", 'b');
    forged.entry.canonical_name = "skill__forged".to_string();
    assert!(registry
        .refresh(SkillToolRegistryRefreshCause::Validation, vec![forged])
        .is_err());
    assert!(Arc::ptr_eq(&good, &registry.snapshot()));
}

#[test]
fn refresh_pins_old_calls_and_security_quarantine_cancels_only_its_revision() {
    use crate::contexts::tooling::skill_tools::domain::SkillToolQuarantine;
    use std::sync::atomic::Ordering;

    let registry = SkillToolRegistry::empty();
    let first = candidate("review", 'a');
    let unrelated = candidate("other", 'b');
    registry
        .refresh(
            SkillToolRegistryRefreshCause::Restore,
            vec![first.clone(), unrelated.clone()],
        )
        .expect("initial snapshot");
    let pinned = registry
        .pin_invocation(&first.entry.key)
        .expect("pinned invocation");
    let unrelated_pin = registry
        .pin_invocation(&unrelated.entry.key)
        .expect("unrelated invocation");
    let mut quarantined = first;
    quarantined.lifecycle.quarantine = SkillToolQuarantine::Quarantined {
        reason: "deterministic-trap".to_string(),
    };
    let next = registry
        .refresh(
            SkillToolRegistryRefreshCause::Quarantine,
            vec![quarantined, unrelated],
        )
        .expect("quarantine refresh");

    assert_ne!(pinned.snapshot.generation, next.generation);
    assert!(pinned.cancelled.load(Ordering::Acquire));
    assert!(!unrelated_pin.cancelled.load(Ordering::Acquire));
}

#[test]
fn concurrent_refresh_retires_artifacts_only_after_the_last_snapshot_pin_drops() {
    let artifacts = Arc::new(RecordingArtifacts::default());
    let registry = SkillToolRegistry::empty().with_artifacts(artifacts.clone());
    let old = candidate("review", 'a');
    registry
        .refresh(SkillToolRegistryRefreshCause::Restore, vec![old.clone()])
        .expect("old snapshot");
    let pin = registry.pin_invocation(&old.entry.key).expect("pin");
    registry
        .refresh(
            SkillToolRegistryRefreshCause::Replacement,
            vec![candidate("review", 'b')],
        )
        .expect("replacement");
    assert!(artifacts
        .0
        .lock()
        .expect("records")
        .last()
        .expect("retained set")
        .contains(&old.entry.key.revision));

    drop(pin);
    registry
        .refresh(
            SkillToolRegistryRefreshCause::EffectiveScope,
            vec![candidate("review", 'b')],
        )
        .expect("cleanup refresh");
    assert!(!artifacts
        .0
        .lock()
        .expect("records")
        .last()
        .expect("retained set")
        .contains(&old.entry.key.revision));
}
