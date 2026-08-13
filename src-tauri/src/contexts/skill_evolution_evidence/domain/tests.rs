use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

use super::*;

#[test]
fn workspace_scope_is_keyed_normalized_and_idempotent() {
    let key = [7_u8; 32];
    let first =
        canonical_workspace_scope(&key, r"D:\Users\Alice\Project").expect("workspace scope");
    let equivalent = canonical_workspace_scope(&key, "d:/users/alice/project")
        .expect("normalized workspace scope");
    let other_key = canonical_workspace_scope(&[8_u8; 32], "d:/users/alice/project")
        .expect("other installation scope");

    assert_eq!(first, equivalent);
    assert_ne!(first, other_key);
    assert_eq!(first.len(), 34);
    assert_eq!(canonical_workspace_scope(&key, &first), Ok(first.clone()));
    assert!(!first.contains("alice"));
}

const OCCURRED_AT: &str = "2026-08-12T10:00:00Z";

fn common(fidelity: &str) -> Value {
    json!({
        "sourceEventId": "event:native:1",
        "occurredAt": OCCURRED_AT,
        "stableAgentId": "onepiece",
        "sessionId": "session-1",
        "messageId": "message-1",
        "runId": "run-1",
        "attemptId": "attempt-1",
        "workspace": "workspace:7f3a",
        "fidelity": fidelity,
        "observedSkillRevisions": [{
            "skillId": "code-review",
            "revision": "revision-1",
            "associationKind": "injected",
            "observedAt": OCCURRED_AT
        }]
    })
}

fn envelope_fixtures() -> Vec<Value> {
    vec![
        json!({
            "sourceKind": "native_execution",
            "schemaVersion": 1,
            "common": common("native"),
            "operationClass": "generation",
            "outcome": "failed",
            "failureClass": "provider",
            "safeCounts": { "attempts": 1, "failures": 1 }
        }),
        json!({
            "sourceKind": "skill_loading",
            "schemaVersion": 1,
            "common": common("native"),
            "skillId": "code-review",
            "revision": "revision-1",
            "outcome": "loaded",
            "anomaly": null,
            "observationCount": 1
        }),
        json!({
            "sourceKind": "delegated_utility",
            "schemaVersion": 1,
            "common": common("native"),
            "utilitySkillId": "repo-search",
            "revision": "revision-2",
            "outcome": "succeeded",
            "toolCount": 3,
            "approvalCount": 1
        }),
        json!({
            "sourceKind": "plan_verification",
            "schemaVersion": 1,
            "common": common("native"),
            "planRunId": "plan-run-1",
            "verifier": "test",
            "outcome": "passed",
            "passedCount": 4,
            "failedCount": 0,
            "predecessorAttemptId": "attempt-0"
        }),
        json!({
            "sourceKind": "managed_cli",
            "schemaVersion": 1,
            "common": common("proxied"),
            "outcome": "failed",
            "failureClass": "process",
            "mountSnapshot": {
                "manifestHash": "manifest-1",
                "skills": [{ "skillId": "code-review", "revision": "revision-1" }]
            },
            "configuredBindingIds": ["binding-1"]
        }),
        json!({
            "sourceKind": "interactive_cli",
            "schemaVersion": 1,
            "common": common("opaque"),
            "outcome": "cancelled",
            "mountSnapshot": null,
            "configuredBindingIds": ["binding-1"]
        }),
        json!({
            "sourceKind": "explicit_feedback",
            "schemaVersion": 1,
            "common": common("native"),
            "feedback": "corrected",
            "feedbackRevision": 2,
            "correctionNote": "Prefer the bounded repository query."
        }),
    ]
}

fn assert_round_trip<T>(value: T, expected: &str)
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug + Copy,
{
    assert_eq!(
        serde_json::to_value(value).expect("serialize enum"),
        json!(expected)
    );
    assert_eq!(
        serde_json::from_value::<T>(json!(expected)).expect("deserialize enum"),
        value
    );
}

#[test]
fn keeps_v1_domain_enum_wire_values_stable() {
    for (value, expected) in [
        (SourceFamily::NativeExecution, "native_execution"),
        (SourceFamily::SkillLoading, "skill_loading"),
        (SourceFamily::DelegatedUtility, "delegated_utility"),
        (SourceFamily::PlanVerification, "plan_verification"),
        (SourceFamily::ManagedCli, "managed_cli"),
        (SourceFamily::InteractiveCli, "interactive_cli"),
        (SourceFamily::ExplicitFeedback, "explicit_feedback"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (SignalCategory::ExplicitFeedback, "explicit_feedback"),
        (SignalCategory::ExecutionFailure, "execution_failure"),
        (SignalCategory::VerificationOutcome, "verification_outcome"),
        (SignalCategory::RetryRecovery, "retry_recovery"),
        (SignalCategory::DelegationOutcome, "delegation_outcome"),
        (
            SignalCategory::SkillLifecycleAnomaly,
            "skill_lifecycle_anomaly",
        ),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (AttributionStrength::Verified, "verified"),
        (AttributionStrength::Correlated, "correlated"),
        (AttributionStrength::Weak, "weak"),
        (AttributionStrength::Unattributed, "unattributed"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (FeedbackState::Helpful, "helpful"),
        (FeedbackState::Unhelpful, "unhelpful"),
        (FeedbackState::Corrected, "corrected"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (SeedReadiness::Collecting, "collecting"),
        (SeedReadiness::Ready, "ready"),
        (SeedReadiness::HumanReviewOnly, "human_review_only"),
        (SeedReadiness::Ineligible, "ineligible"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (SourceFidelity::Native, "native"),
        (SourceFidelity::Proxied, "proxied"),
        (SourceFidelity::Inferred, "inferred"),
        (SourceFidelity::Opaque, "opaque"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (SkillAssociationKind::Injected, "injected"),
        (SkillAssociationKind::Loaded, "loaded"),
        (SkillAssociationKind::Delegated, "delegated"),
        (SkillAssociationKind::Mounted, "mounted"),
        (SkillAssociationKind::Configured, "configured"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (OperationClass::Generation, "generation"),
        (OperationClass::Tool, "tool"),
        (OperationClass::Permission, "permission"),
        (OperationClass::Provider, "provider"),
        (OperationClass::Process, "process"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (TerminalOutcome::Succeeded, "succeeded"),
        (TerminalOutcome::Failed, "failed"),
        (TerminalOutcome::Cancelled, "cancelled"),
        (TerminalOutcome::Incomplete, "incomplete"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (FailureClass::Agent, "agent"),
        (FailureClass::Provider, "provider"),
        (FailureClass::Process, "process"),
        (FailureClass::Tool, "tool"),
        (FailureClass::Permission, "permission"),
        (FailureClass::Timeout, "timeout"),
        (FailureClass::Limit, "limit"),
        (FailureClass::Sandbox, "sandbox"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (SkillLoadOutcome::Loaded, "loaded"),
        (SkillLoadOutcome::Refused, "refused"),
        (SkillLoadOutcome::Unavailable, "unavailable"),
        (SkillLoadOutcome::Omitted, "omitted"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (SkillLifecycleAnomaly::LoadRefusal, "load_refusal"),
        (
            SkillLifecycleAnomaly::DependencyUnavailable,
            "dependency_unavailable",
        ),
        (SkillLifecycleAnomaly::OverlayConflict, "overlay_conflict"),
        (
            SkillLifecycleAnomaly::PromptBudgetOmission,
            "prompt_budget_omission",
        ),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (UtilityOutcome::Succeeded, "succeeded"),
        (UtilityOutcome::Failed, "failed"),
        (UtilityOutcome::Cancelled, "cancelled"),
        (UtilityOutcome::TimedOut, "timed_out"),
        (UtilityOutcome::Limited, "limited"),
        (UtilityOutcome::Refused, "refused"),
        (UtilityOutcome::Incomplete, "incomplete"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (VerificationClass::Test, "test"),
        (VerificationClass::Build, "build"),
        (VerificationClass::Lint, "lint"),
        (VerificationClass::Review, "review"),
        (VerificationClass::Type, "type"),
        (VerificationClass::Security, "security"),
        (VerificationClass::Specification, "specification"),
        (VerificationClass::Acceptance, "acceptance"),
        (VerificationClass::Plan, "plan"),
    ] {
        assert_round_trip(value, expected);
    }

    for (value, expected) in [
        (VerificationOutcome::Passed, "passed"),
        (VerificationOutcome::Failed, "failed"),
        (VerificationOutcome::Inconclusive, "inconclusive"),
        (VerificationOutcome::Skipped, "skipped"),
    ] {
        assert_round_trip(value, expected);
    }
}

#[test]
fn round_trips_every_registered_v1_envelope() {
    for fixture in envelope_fixtures() {
        let envelope: EvidenceSourceEnvelope =
            serde_json::from_value(fixture.clone()).expect("deserialize envelope");
        envelope.validate().expect("valid envelope");
        assert_eq!(envelope.schema_version(), EVIDENCE_ENVELOPE_SCHEMA_V1);
        let expected_family: SourceFamily = serde_json::from_value(fixture["sourceKind"].clone())
            .expect("registered source family");
        assert_eq!(envelope.source_family(), expected_family);
        assert_eq!(
            serde_json::to_value(envelope).expect("serialize envelope"),
            fixture
        );
    }
}

#[test]
fn rejects_unknown_fields_and_enum_values() {
    let mut fixture = envelope_fixtures().remove(0);
    fixture
        .as_object_mut()
        .expect("object")
        .insert("rawPrompt".to_string(), json!("must never enter evidence"));
    assert!(serde_json::from_value::<EvidenceSourceEnvelope>(fixture).is_err());

    let mut nested = envelope_fixtures().remove(0);
    nested["common"]["rawToolResult"] = json!("must never enter evidence");
    assert!(serde_json::from_value::<EvidenceSourceEnvelope>(nested).is_err());

    assert!(serde_json::from_value::<SignalCategory>(json!("model_opinion")).is_err());
    assert!(serde_json::from_value::<EvidenceSourceEnvelope>(json!({
        "sourceKind": "future_runtime_source",
        "schemaVersion": 1,
        "common": common("native")
    }))
    .is_err());
}

#[test]
fn rejects_unsupported_versions_invalid_timestamps_and_malformed_ids() {
    let mut unsupported = envelope_fixtures().remove(0);
    unsupported["schemaVersion"] = json!(2);
    let unsupported: EvidenceSourceEnvelope =
        serde_json::from_value(unsupported).expect("known envelope shape");
    assert_eq!(
        unsupported.validate(),
        Err(EvidenceValidationError::UnsupportedSchemaVersion(2))
    );

    let mut bad_timestamp = envelope_fixtures().remove(0);
    bad_timestamp["common"]["occurredAt"] = json!("today");
    let bad_timestamp: EvidenceSourceEnvelope =
        serde_json::from_value(bad_timestamp).expect("known envelope shape");
    assert_eq!(
        bad_timestamp.validate(),
        Err(EvidenceValidationError::InvalidTimestamp("occurred_at"))
    );

    let mut bad_id = envelope_fixtures().remove(0);
    bad_id["common"]["sourceEventId"] = json!("event with spaces");
    let bad_id: EvidenceSourceEnvelope =
        serde_json::from_value(bad_id).expect("known envelope shape");
    assert_eq!(
        bad_id.validate(),
        Err(EvidenceValidationError::MalformedIdentifier(
            "source_event_id"
        ))
    );
}

#[test]
fn rejects_oversized_correction_notes() {
    let mut fixture = envelope_fixtures().remove(6);
    fixture["correctionNote"] = json!("x".repeat(MAX_CORRECTION_NOTE_CHARS + 1));
    let envelope: EvidenceSourceEnvelope =
        serde_json::from_value(fixture).expect("known envelope shape");

    assert_eq!(
        envelope.validate(),
        Err(EvidenceValidationError::CorrectionNoteTooLong {
            max: MAX_CORRECTION_NOTE_CHARS,
        })
    );
}
