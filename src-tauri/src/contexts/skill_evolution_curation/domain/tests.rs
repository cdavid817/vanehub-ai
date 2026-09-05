use super::*;
use serde_json::json;

fn witness() -> CuratorWitnessBundleV1 {
    CuratorWitnessBundleV1 {
        schema_version: CURATOR_SCHEMA_VERSION_V1,
        candidate_id: "candidate-1".into(),
        candidate_revision: 3,
        draft_id: Some("draft-1".into()),
        draft_revision: Some(2),
        draft_hash: Some("sha256:draft".into()),
        assessment_hash: "sha256:assessment".into(),
        target_revision: "target-revision-1".into(),
        base_hash: "sha256:base".into(),
        effective_hash: "sha256:effective".into(),
        overlay_revision: "overlay-revision-1".into(),
        pin_witness: "unpinned".into(),
        trust_witness: "trusted".into(),
        scanner_version: "scanner-v1".into(),
        policy_revision: 4,
        preview_hash: Some("sha256:preview".into()),
    }
}

#[test]
fn enum_wire_values_and_policy_schema_are_stable() {
    assert_eq!(
        serde_json::to_value(CuratorCandidateState::ReadyForReview).expect("state"),
        json!("ready_for_review")
    );
    assert_eq!(
        serde_json::from_value::<CuratorActorClass>(json!("local_interactive_user"))
            .expect("actor"),
        CuratorActorClass::LocalInteractiveUser
    );
    let policy = CuratorPolicyV1::manual_default("workspace:one".into());
    assert_eq!(policy.schema_version, CURATOR_SCHEMA_VERSION_V1);
    assert_eq!(policy.enqueue_routes.len(), 2);

    let mut value = serde_json::to_value(policy).expect("policy json");
    value
        .as_object_mut()
        .expect("object")
        .insert("automaticApply".into(), json!(true));
    assert!(serde_json::from_value::<CuratorPolicyV1>(value).is_err());
}

#[test]
fn candidate_state_machine_enforces_terminal_and_review_invariants() {
    let awaiting = transition_candidate(
        CuratorCandidateState::Pending,
        CuratorTransition::IntakeValidatedWithoutDraft,
    )
    .expect("awaiting draft");
    let ready = transition_candidate(awaiting, CuratorTransition::DraftBecameReady)
        .expect("ready for review");
    let applying = transition_candidate(ready, CuratorTransition::Approve).expect("applying");
    let applied =
        transition_candidate(applying, CuratorTransition::ApplySucceeded).expect("applied");

    assert!(is_terminal(applied));
    assert_eq!(
        transition_candidate(applied, CuratorTransition::RetryPrepared),
        Err(CuratorTransitionError::TerminalState)
    );
    assert_eq!(
        transition_candidate(awaiting, CuratorTransition::Approve),
        Err(CuratorTransitionError::InvalidTransition)
    );
}

#[test]
fn defer_resume_and_failed_application_require_explicit_paths() {
    let deferred = transition_candidate(
        CuratorCandidateState::ReadyForReview,
        CuratorTransition::Defer,
    )
    .expect("deferred");
    assert_eq!(
        transition_candidate(deferred, CuratorTransition::ResumeWithReadyDraft),
        Ok(CuratorCandidateState::ReadyForReview)
    );
    let failed = transition_candidate(
        CuratorCandidateState::Applying,
        CuratorTransition::ApplyFailed,
    )
    .expect("apply failed");
    assert_eq!(
        transition_candidate(failed, CuratorTransition::Approve),
        Err(CuratorTransitionError::InvalidTransition)
    );
    assert_eq!(
        transition_candidate(failed, CuratorTransition::RetryPrepared),
        Ok(CuratorCandidateState::ReadyForReview)
    );
}

#[test]
fn witness_hash_is_canonical_and_revision_checks_detect_conflicts() {
    let first = witness().canonical_hash().expect("hash");
    let second = witness().canonical_hash().expect("stable hash");
    let mut changed = witness();
    changed.overlay_revision = "overlay-revision-2".into();

    assert_eq!(first, second);
    assert_ne!(first, changed.canonical_hash().expect("changed hash"));
    assert_eq!(require_revision(3, 3), Ok(()));
    assert_eq!(
        require_revision(2, 3),
        Err(OptimisticRevisionConflict {
            expected: 2,
            actual: 3
        })
    );
}

#[test]
fn snapshot_compatibility_rejects_unknown_fields() {
    let value = json!({
        "schemaVersion": 1,
        "candidateId": "candidate-1",
        "workspaceId": "workspace:one",
        "seedId": "seed-1",
        "seedRevision": "seed-revision-1",
        "assessmentAttemptId": "assessment-1",
        "assessmentRevision": "assessment-revision-1",
        "targetSkillId": "code-review",
        "targetRevision": "target-revision-1",
        "overlayScope": "project",
        "route": "advance",
        "risk": "low",
        "confidence": "high",
        "evidenceIds": ["evidence-1"],
        "qualityChecks": [],
        "witnessHash": "sha256:witness",
        "state": "awaiting_draft",
        "staleness": [],
        "revision": 1,
        "createdAtMs": 1,
        "updatedAtMs": 1,
        "providerPayload": "forbidden"
    });
    assert!(serde_json::from_value::<CuratorCandidateSnapshot>(value).is_err());
}
