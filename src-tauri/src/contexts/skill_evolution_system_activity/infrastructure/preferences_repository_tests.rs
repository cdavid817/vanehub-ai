use rusqlite::Connection;

use super::*;
use crate::contexts::skill_evolution_system_activity::domain::*;

#[test]
fn preferences_are_versioned_scoped_and_return_current_state_on_conflict() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let initial = preferences(0);
    let ActivityPreferenceUpdateOutcome::Updated(created) = repository
        .update_preferences(&initial, 1)
        .expect("create preferences")
    else {
        panic!("expected created preferences");
    };
    assert_eq!(created.revision, 1);

    let mut update = created.clone();
    update.visible = false;
    update.minimum_timeline_severity = ActivitySeverity::Critical;
    update.digest_cadence = ActivityDigestCadence::Daily;
    update.detail_retention_days = 30;
    let ActivityPreferenceUpdateOutcome::Updated(updated) = repository
        .update_preferences(&update, 2)
        .expect("update preferences")
    else {
        panic!("expected updated preferences");
    };
    assert_eq!(updated.revision, 2);
    assert!(!updated.visible);
    assert_eq!(updated.detail_retention_days, 30);

    let ActivityPreferenceUpdateOutcome::Conflict(current) = repository
        .update_preferences(&update, 3)
        .expect("typed conflict")
    else {
        panic!("expected conflict");
    };
    assert_eq!(current, updated);
}

#[test]
fn preference_limits_and_mandatory_warning_retention_fail_closed() {
    let connection = Connection::open_in_memory().expect("database");
    apply_schema(&connection).expect("schema");
    let repository = SqliteActivityProjectionRepository::new(&connection);
    let mut invalid = preferences(0);
    invalid.detail_retention_days = 29;
    assert_eq!(
        repository.update_preferences(&invalid, 1),
        Err(ActivityProjectionRepositoryError::InvalidInput)
    );

    let routine = envelope(ActivityAttentionKind::None, ActivitySeverity::Warning);
    let protected = envelope(ActivityAttentionKind::Security, ActivitySeverity::Warning);
    assert!(!timeline_policy_allows(
        &routine,
        ActivitySeverity::Critical
    ));
    assert!(timeline_policy_allows(
        &protected,
        ActivitySeverity::Critical
    ));
}

fn preferences(revision: u64) -> EvolutionActivityPreferences {
    EvolutionActivityPreferences {
        scope_kind: ActivityScopeKind::Workspace,
        canonical_scope_id: "workspace-1".into(),
        visible: true,
        minimum_timeline_severity: ActivitySeverity::Info,
        notification_threshold: ActivitySeverity::Warning,
        digest_cadence: ActivityDigestCadence::Off,
        read_retention_days: 180,
        detail_retention_days: 180,
        export_item_limit: 1_000,
        export_size_limit_bytes: 10 * 1024 * 1024,
        revision,
    }
}

fn envelope(
    attention_kind: ActivityAttentionKind,
    severity: ActivitySeverity,
) -> EvolutionActivityEnvelopeV1 {
    use std::collections::BTreeMap;
    EvolutionActivityEnvelopeV1 {
        schema_version: 1,
        event_id: "event-1".into(),
        event_code: ActivityEventCode::RunFailed,
        source_domain: "orchestration".into(),
        source_id: "run-1".into(),
        source_revision: "revision-1".into(),
        source_sequence: 1,
        scope_kind: ActivityScopeKind::Workspace,
        canonical_scope_id: "workspace-1".into(),
        occurred_at_ms: 1,
        committed_at_ms: 1,
        severity,
        status: ActivityStatus::Failed,
        attention_kind,
        safe_actor_kind: ActivityActorKind::System,
        safe_identities: Vec::new(),
        metrics: BTreeMap::new(),
        reason_codes: Vec::new(),
        navigation: None,
        supersedes_event_id: None,
        payload: None,
        projection_policy_version: 1,
        content_hash: String::new(),
    }
    .seal()
    .expect("envelope")
}
