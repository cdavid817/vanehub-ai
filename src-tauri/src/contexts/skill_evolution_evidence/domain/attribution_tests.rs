use serde_json::{json, Value};

use super::*;

fn common(observations: Value, fidelity: &str) -> Value {
    json!({
        "sourceEventId": "event:1",
        "occurredAt": "2026-08-13T02:00:00Z",
        "stableAgentId": "onepiece",
        "sessionId": "session-1",
        "messageId": "message-1",
        "runId": "run-1",
        "attemptId": "attempt-1",
        "workspace": "workspace:7f3a",
        "fidelity": fidelity,
        "observedSkillRevisions": observations
    })
}

fn native(observations: Value) -> EvidenceSourceEnvelope {
    serde_json::from_value(json!({
        "sourceKind": "native_execution",
        "schemaVersion": 1,
        "common": common(observations, "native"),
        "operationClass": "generation",
        "outcome": "failed",
        "failureClass": "tool",
        "safeCounts": { "attempts": 1, "failures": 1 }
    }))
    .expect("native envelope")
}

#[test]
fn preserves_all_exact_native_participants_as_verified() {
    let envelope = native(json!([
        {
            "skillId": "review",
            "revision": "rev-a",
            "associationKind": "injected",
            "observedAt": "2026-08-13T01:59:58Z"
        },
        {
            "skillId": "testing",
            "revision": "rev-b",
            "associationKind": "loaded",
            "observedAt": "2026-08-13T01:59:59Z"
        }
    ]));

    let result = attribute_evidence(&envelope);
    assert_eq!(result.strength(), AttributionStrength::Verified);
    assert_eq!(
        result.targeting_eligibility(),
        TargetingEligibility::AutomatedConsideration
    );
    assert_eq!(result.associations().len(), 2);
    assert_eq!(result.associations()[0].skill_id(), "review");
    assert_eq!(result.associations()[1].skill_id(), "testing");
    assert!(
        result
            .associations()
            .iter()
            .all(|association| association.rationale()
                == AttributionRationale::ExactNativeObservation)
    );
}

#[test]
fn keeps_revision_changes_as_distinct_participation_facts() {
    let envelope = native(json!([
        {
            "skillId": "review",
            "revision": "rev-a",
            "associationKind": "injected",
            "observedAt": "2026-08-13T01:59:57Z"
        },
        {
            "skillId": "review",
            "revision": "rev-b",
            "associationKind": "loaded",
            "observedAt": "2026-08-13T01:59:59Z"
        }
    ]));

    let result = attribute_evidence(&envelope);
    assert_eq!(result.associations().len(), 2);
    assert_eq!(result.associations()[0].revision(), "rev-a");
    assert_eq!(result.associations()[1].revision(), "rev-b");
}

#[test]
fn cli_mount_snapshot_is_correlated_and_human_review_only() {
    let envelope: EvidenceSourceEnvelope = serde_json::from_value(json!({
        "sourceKind": "managed_cli",
        "schemaVersion": 1,
        "common": common(json!([]), "proxied"),
        "outcome": "failed",
        "failureClass": "process",
        "mountSnapshot": {
            "manifestHash": "manifest-a",
            "skills": [
                { "skillId": "review", "revision": "rev-a" },
                { "skillId": "testing", "revision": "rev-b" }
            ]
        },
        "configuredBindingIds": ["binding-stale"]
    }))
    .expect("managed CLI envelope");

    let result = attribute_evidence(&envelope);
    assert_eq!(result.strength(), AttributionStrength::Correlated);
    assert_eq!(
        result.targeting_eligibility(),
        TargetingEligibility::HumanReviewOnly
    );
    assert_eq!(result.associations().len(), 2);
    assert!(
        result
            .associations()
            .iter()
            .all(|association| association.rationale()
                == AttributionRationale::ActiveCliMountSnapshot)
    );
}

#[test]
fn stale_cli_binding_is_weak_without_inventing_a_skill_identity() {
    let envelope: EvidenceSourceEnvelope = serde_json::from_value(json!({
        "sourceKind": "interactive_cli",
        "schemaVersion": 1,
        "common": common(json!([]), "opaque"),
        "outcome": "cancelled",
        "mountSnapshot": null,
        "configuredBindingIds": ["binding-stale"]
    }))
    .expect("interactive CLI envelope");

    let result = attribute_evidence(&envelope);
    assert_eq!(result.strength(), AttributionStrength::Weak);
    assert_eq!(
        result.targeting_eligibility(),
        TargetingEligibility::Ineligible
    );
    assert!(result.associations().is_empty());
    assert_eq!(
        result.rationale(),
        AttributionRationale::ConfiguredBindingOnly
    );
}

#[test]
fn missing_observations_remain_unattributed() {
    let result = attribute_evidence(&native(json!([])));

    assert_eq!(result.strength(), AttributionStrength::Unattributed);
    assert_eq!(
        result.targeting_eligibility(),
        TargetingEligibility::Ineligible
    );
    assert!(result.associations().is_empty());
    assert_eq!(
        result.rationale(),
        AttributionRationale::NoObservedSkillParticipation
    );
}

#[test]
fn opaque_cli_never_upgrades_mount_participation_to_verified() {
    let envelope: EvidenceSourceEnvelope = serde_json::from_value(json!({
        "sourceKind": "interactive_cli",
        "schemaVersion": 1,
        "common": common(json!([{
            "skillId": "untrusted-hook-claim",
            "revision": "rev-hook",
            "associationKind": "loaded",
            "observedAt": "2026-08-13T01:59:59Z"
        }]), "opaque"),
        "outcome": "succeeded",
        "mountSnapshot": {
            "manifestHash": "manifest-a",
            "skills": [{ "skillId": "review", "revision": "rev-a" }]
        },
        "configuredBindingIds": []
    }))
    .expect("opaque CLI envelope");

    let result = attribute_evidence(&envelope);
    assert_eq!(result.strength(), AttributionStrength::Correlated);
    assert_eq!(result.associations().len(), 1);
    assert_eq!(result.associations()[0].skill_id(), "review");
}
