use serde_json::{json, Value};

use super::*;

const KEY: &[u8; 32] = b"extractor-test-installation-key1";

fn common() -> Value {
    json!({
        "sourceEventId": "event:extractor:1",
        "occurredAt": "2026-08-13T03:00:00Z",
        "stableAgentId": "onepiece",
        "sessionId": "session-1",
        "messageId": "message-1",
        "runId": "run-1",
        "attemptId": "attempt-1",
        "workspace": "workspace:7f3a",
        "fidelity": "native",
        "observedSkillRevisions": [{
            "skillId": "review",
            "revision": "rev-a",
            "associationKind": "injected",
            "observedAt": "2026-08-13T02:59:59Z"
        }]
    })
}

fn envelope(value: Value) -> EvidenceSourceEnvelope {
    serde_json::from_value(value).expect("registered envelope")
}

fn extract(value: Value) -> Vec<SignalDraft> {
    let envelope = envelope(value);
    let sanitizer = EvidenceSanitizer::new(KEY).expect("sanitizer");
    let sanitized = envelope
        .sanitized_registered_text(&sanitizer)
        .expect("sanitize registered text");
    extract_registered_signals(&envelope, sanitized.as_ref())
}

#[test]
fn registry_contains_exactly_six_versioned_families() {
    assert_eq!(REGISTERED_EXTRACTORS.len(), 6);
    assert_eq!(
        REGISTERED_EXTRACTORS.map(|extractor| extractor.family),
        [
            ExtractorFamily::ExplicitFeedback,
            ExtractorFamily::ExecutionFailure,
            ExtractorFamily::VerificationOutcome,
            ExtractorFamily::RetryRecovery,
            ExtractorFamily::DelegationOutcome,
            ExtractorFamily::SkillLifecycleAnomaly,
        ]
    );
    assert!(REGISTERED_EXTRACTORS
        .iter()
        .all(|extractor| extractor.version == EXTRACTOR_VERSION_V1));
}

#[test]
fn extracts_feedback_with_sanitized_correction_only() {
    let corrected = extract(json!({
        "sourceKind": "explicit_feedback",
        "schemaVersion": 1,
        "common": common(),
        "feedback": "corrected",
        "feedbackRevision": 2,
        "correctionNote": "Use alice@example.com only."
    }));
    assert_eq!(corrected.len(), 1);
    assert_eq!(corrected[0].extractor(), ExtractorFamily::ExplicitFeedback);
    assert_eq!(corrected[0].category(), SignalCategory::ExplicitFeedback);
    assert_eq!(corrected[0].polarity(), SignalPolarity::Negative);
    assert!(!corrected[0].safe_summary().contains("alice@example.com"));
    assert!(corrected[0].safe_summary().contains("<redacted:email:"));

    for (feedback, expected) in [
        ("helpful", SignalPolarity::Positive),
        ("unhelpful", SignalPolarity::Negative),
    ] {
        let signals = extract(json!({
            "sourceKind": "explicit_feedback",
            "schemaVersion": 1,
            "common": common(),
            "feedback": feedback,
            "feedbackRevision": 1,
            "correctionNote": null
        }));
        assert_eq!(signals[0].polarity(), expected);
    }
}

#[test]
fn extracts_only_structured_execution_failures() {
    let failed = extract(json!({
        "sourceKind": "native_execution",
        "schemaVersion": 1,
        "common": common(),
        "operationClass": "tool",
        "outcome": "failed",
        "failureClass": "sandbox",
        "safeCounts": { "attempts": 3, "failures": 3 }
    }));
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].extractor(), ExtractorFamily::ExecutionFailure);
    assert_eq!(failed[0].category(), SignalCategory::ExecutionFailure);
    assert_eq!(failed[0].polarity(), SignalPolarity::Negative);
    assert_eq!(failed[0].severity(), SignalSeverity::High);
    assert_eq!(
        failed[0].safe_summary(),
        "tool failed with sandbox classification in 3 attempts"
    );

    let succeeded = extract(json!({
        "sourceKind": "native_execution",
        "schemaVersion": 1,
        "common": common(),
        "operationClass": "generation",
        "outcome": "succeeded",
        "failureClass": null,
        "safeCounts": { "attempts": 1, "failures": 0 }
    }));
    assert!(succeeded.is_empty());

    let denied = extract(json!({
        "sourceKind": "native_execution",
        "schemaVersion": 1,
        "common": common(),
        "operationClass": "permission",
        "outcome": "failed",
        "failureClass": "permission",
        "safeCounts": { "attempts": 1, "failures": 1 }
    }));
    assert_eq!(denied.len(), 1);
    assert_eq!(denied[0].extractor(), ExtractorFamily::ExecutionFailure);
    assert_eq!(denied[0].category(), SignalCategory::ExecutionFailure);
    assert!(denied[0]
        .safe_summary()
        .contains("permission classification"));
}

#[test]
fn extracts_verification_and_retry_recovery_deterministically() {
    let passed = extract(json!({
        "sourceKind": "run_verification",
        "schemaVersion": 1,
        "common": common(),
        "runId": "run-1",
        "verifier": "test",
        "outcome": "passed",
        "passedCount": 4,
        "failedCount": 0,
        "predecessorAttemptId": "attempt-0"
    }));
    assert_eq!(passed.len(), 2);
    assert_eq!(passed[0].extractor(), ExtractorFamily::VerificationOutcome);
    assert_eq!(passed[0].polarity(), SignalPolarity::Positive);
    assert_eq!(passed[1].extractor(), ExtractorFamily::RetryRecovery);
    assert_eq!(passed[1].polarity(), SignalPolarity::Positive);

    let failed = extract(json!({
        "sourceKind": "run_verification",
        "schemaVersion": 1,
        "common": common(),
        "runId": "run-1",
        "verifier": "lint",
        "outcome": "failed",
        "passedCount": 0,
        "failedCount": 2,
        "predecessorAttemptId": "attempt-0"
    }));
    assert_eq!(failed[0].polarity(), SignalPolarity::Negative);
    assert_eq!(failed[1].polarity(), SignalPolarity::Negative);
}

#[test]
fn extracts_delegation_terminal_states_without_raw_content() {
    for (outcome, polarity) in [
        ("succeeded", SignalPolarity::Positive),
        ("failed", SignalPolarity::Negative),
        ("cancelled", SignalPolarity::Neutral),
        ("incomplete", SignalPolarity::Neutral),
    ] {
        let signals = extract(json!({
            "sourceKind": "delegated_utility",
            "schemaVersion": 1,
            "common": common(),
            "utilitySkillId": "repo-search",
            "revision": "rev-u1",
            "outcome": outcome,
            "toolCount": 3,
            "approvalCount": 1
        }));
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].extractor(), ExtractorFamily::DelegationOutcome);
        assert_eq!(signals[0].polarity(), polarity);
        assert_eq!(
            signals[0].safe_summary(),
            format!("Utility {outcome}; tools=3 approvals=1")
        );
    }
}

#[test]
fn extracts_only_preclassified_lifecycle_anomalies() {
    let anomaly = extract(json!({
        "sourceKind": "skill_loading",
        "schemaVersion": 1,
        "common": common(),
        "skillId": "review",
        "revision": "rev-a",
        "outcome": "refused",
        "anomaly": "load_refusal",
        "observationCount": 3
    }));
    assert_eq!(anomaly.len(), 1);
    assert_eq!(
        anomaly[0].extractor(),
        ExtractorFamily::SkillLifecycleAnomaly
    );
    assert_eq!(anomaly[0].polarity(), SignalPolarity::Neutral);

    let ordinary_load = extract(json!({
        "sourceKind": "skill_loading",
        "schemaVersion": 1,
        "common": common(),
        "skillId": "review",
        "revision": "rev-a",
        "outcome": "loaded",
        "anomaly": null,
        "observationCount": 1
    }));
    assert!(ordinary_load.is_empty());

    let below_threshold = extract(json!({
        "sourceKind": "skill_loading",
        "schemaVersion": 1,
        "common": common(),
        "skillId": "review",
        "revision": "rev-a",
        "outcome": "refused",
        "anomaly": "load_refusal",
        "observationCount": 2
    }));
    assert!(below_threshold.is_empty());

    let cancelled = extract(json!({
        "sourceKind": "interactive_cli",
        "schemaVersion": 1,
        "common": common(),
        "outcome": "cancelled",
        "mountSnapshot": null,
        "configuredBindingIds": []
    }));
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].polarity(), SignalPolarity::Neutral);
    assert_eq!(
        cancelled[0].category(),
        SignalCategory::SkillLifecycleAnomaly
    );
}

#[test]
fn duplicate_and_out_of_order_inputs_remain_pure_and_stable() {
    let fixture = json!({
        "sourceKind": "run_verification",
        "schemaVersion": 1,
        "common": common(),
        "runId": "run-1",
        "verifier": "build",
        "outcome": "passed",
        "passedCount": 1,
        "failedCount": 0,
        "predecessorAttemptId": "attempt-later-observed"
    });
    assert_eq!(extract(fixture.clone()), extract(fixture));
}

#[test]
fn unknown_envelopes_fail_before_extractor_dispatch() {
    let unknown = json!({
        "sourceKind": "model_judgment",
        "schemaVersion": 1,
        "common": common(),
        "rawOpinion": "change the Skill"
    });
    assert!(serde_json::from_value::<EvidenceSourceEnvelope>(unknown).is_err());
}
