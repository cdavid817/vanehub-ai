use std::{collections::BTreeMap, sync::Mutex};

use super::*;
use crate::contexts::skill_evolution_system_activity::domain::*;

struct FakeTargets {
    envelope: EvolutionActivityEnvelopeV1,
    receipts: Mutex<BTreeMap<(ActivityTargetKind, String), ActivityTargetReceipt>>,
    deliveries: Mutex<BTreeMap<ActivityTargetKind, usize>>,
    failing: Mutex<Option<ActivityTargetKind>>,
}

impl FakeTargets {
    fn new(attention: ActivityAttentionKind) -> Self {
        Self {
            envelope: envelope(attention),
            receipts: Mutex::new(BTreeMap::new()),
            deliveries: Mutex::new(BTreeMap::new()),
            failing: Mutex::new(None),
        }
    }

    fn delivery_count(&self, kind: ActivityTargetKind) -> usize {
        self.deliveries
            .lock()
            .expect("delivery lock")
            .get(&kind)
            .copied()
            .unwrap_or(0)
    }
}

impl ActivityTargetDeliveryPort for FakeTargets {
    fn load_envelope(
        &self,
        event_id: &str,
    ) -> Result<EvolutionActivityEnvelopeV1, ActivityTargetProjectionError> {
        if event_id == self.envelope.event_id {
            Ok(self.envelope.clone())
        } else {
            Err(ActivityTargetProjectionError::InvalidInput)
        }
    }

    fn receipt(
        &self,
        _event_id: &str,
        target_kind: ActivityTargetKind,
        target_scope: &str,
    ) -> Result<Option<ActivityTargetReceipt>, ActivityTargetProjectionError> {
        Ok(self
            .receipts
            .lock()
            .expect("receipt lock")
            .get(&(target_kind, target_scope.to_owned()))
            .cloned())
    }

    fn deliver(
        &self,
        target_kind: ActivityTargetKind,
        _envelope: &EvolutionActivityEnvelopeV1,
        _target_scope: &str,
        _projected_at_ms: i64,
    ) -> Result<(), ActivityTargetProjectionError> {
        *self
            .deliveries
            .lock()
            .expect("delivery lock")
            .entry(target_kind)
            .or_default() += 1;
        if *self.failing.lock().expect("failure lock") == Some(target_kind) {
            Err(ActivityTargetProjectionError::Unavailable)
        } else {
            Ok(())
        }
    }

    fn record_receipt(
        &self,
        receipt: &ActivityTargetReceipt,
    ) -> Result<(), ActivityTargetProjectionError> {
        self.receipts.lock().expect("receipt lock").insert(
            (receipt.target_kind, receipt.target_scope.clone()),
            receipt.clone(),
        );
        Ok(())
    }
}

#[test]
fn four_targets_apply_independent_policy_and_replay_from_receipts() {
    let targets = FakeTargets::new(ActivityAttentionKind::None);
    let projector = ActivityTargetProjector::new(&targets);

    let first = projector.project("event-1", 20).expect("first projection");
    assert_eq!(first.len(), 4);
    assert_eq!(
        first
            .iter()
            .find(|outcome| outcome.target_kind == ActivityTargetKind::Notification)
            .expect("notification")
            .status,
        Ok(ActivityDeliveryStatus::Suppressed)
    );
    assert_eq!(targets.delivery_count(ActivityTargetKind::Notification), 0);
    assert_eq!(targets.receipts.lock().expect("receipt lock").len(), 4);

    let replay = projector.project("event-1", 30).expect("replay");
    assert!(replay.iter().all(|outcome| outcome.replayed));
    assert_eq!(
        targets.delivery_count(ActivityTargetKind::SystemTimeline),
        1
    );
    assert_eq!(
        targets.delivery_count(ActivityTargetKind::SkillDashboard),
        1
    );
    assert_eq!(targets.delivery_count(ActivityTargetKind::UnreadState), 1);
}

#[test]
fn one_target_failure_is_receipted_without_rolling_back_other_targets() {
    let targets = FakeTargets::new(ActivityAttentionKind::Security);
    *targets.failing.lock().expect("failure lock") = Some(ActivityTargetKind::SkillDashboard);
    let projector = ActivityTargetProjector::new(&targets);

    let first = projector
        .project("event-1", 20)
        .expect("partial projection");
    let dashboard = first
        .iter()
        .find(|outcome| outcome.target_kind == ActivityTargetKind::SkillDashboard)
        .expect("dashboard");
    assert_eq!(
        dashboard.status,
        Err(ActivityTargetProjectionError::Unavailable)
    );
    assert!(first.iter().any(|outcome| {
        outcome.target_kind == ActivityTargetKind::Notification
            && outcome.status == Ok(ActivityDeliveryStatus::Delivered)
    }));

    *targets.failing.lock().expect("failure lock") = None;
    let retry = projector.project("event-1", 30).expect("target retry");
    assert_eq!(
        targets.delivery_count(ActivityTargetKind::SkillDashboard),
        2
    );
    assert_eq!(
        targets.delivery_count(ActivityTargetKind::SystemTimeline),
        1
    );
    assert_eq!(targets.delivery_count(ActivityTargetKind::UnreadState), 1);
    assert_eq!(targets.delivery_count(ActivityTargetKind::Notification), 1);
    assert!(retry.iter().all(|outcome| {
        outcome.target_kind == ActivityTargetKind::SkillDashboard || outcome.replayed
    }));
}

fn envelope(attention_kind: ActivityAttentionKind) -> EvolutionActivityEnvelopeV1 {
    EvolutionActivityEnvelopeV1 {
        schema_version: ACTIVITY_SCHEMA_VERSION_V1,
        event_id: "event-1".into(),
        event_code: ActivityEventCode::RunCompleted,
        source_domain: "orchestration".into(),
        source_id: "run-1".into(),
        source_revision: "revision-1".into(),
        source_sequence: 1,
        scope_kind: ActivityScopeKind::Workspace,
        canonical_scope_id: "workspace-1".into(),
        occurred_at_ms: 1,
        committed_at_ms: 2,
        severity: ActivitySeverity::Warning,
        status: ActivityStatus::Succeeded,
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
