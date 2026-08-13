use std::sync::Mutex;

use super::*;
use crate::contexts::skill_evolution_evidence::domain::{
    MountedSkillRevision, ObservedSkillRevision, SkillAssociationKind, SourceFidelity,
};

#[derive(Default)]
struct Capture(Mutex<Vec<EvidenceSourceEnvelope>>);

impl EvidenceProjectionSink for Capture {
    fn submit(&self, envelope: EvidenceSourceEnvelope) -> ProjectionDisposition {
        self.0.lock().expect("capture").push(envelope);
        ProjectionDisposition::Accepted
    }
}

fn common(event: &str) -> EnvelopeCommon {
    EnvelopeCommon {
        source_event_id: event.to_string(),
        occurred_at: "2026-08-13T10:00:00Z".to_string(),
        stable_agent_id: Some("onepiece".to_string()),
        session_id: Some("session-1".to_string()),
        message_id: Some("message-1".to_string()),
        run_id: Some("run-1".to_string()),
        attempt_id: Some("attempt-1".to_string()),
        workspace: Some("workspace-a".to_string()),
        fidelity: SourceFidelity::Native,
        observed_skill_revisions: vec![ObservedSkillRevision {
            skill_id: "review".to_string(),
            revision: "rev-a".to_string(),
            association_kind: SkillAssociationKind::Injected,
            observed_at: "2026-08-13T09:59:59Z".to_string(),
        }],
    }
}

#[test]
fn disabled_projection_is_a_strict_noop() {
    let projector = RuntimeEvidenceProjector::disabled();
    assert_eq!(
        projector.native(NativeExecutionFact {
            common: common("disabled"),
            operation_class: OperationClass::Generation,
            outcome: TerminalOutcome::Succeeded,
            failure_class: None,
            safe_counts: SafeCounts {
                attempts: 1,
                failures: 0,
            },
        }),
        ProjectionDisposition::Disabled
    );
}

#[test]
fn all_runtime_families_project_only_registered_metadata() {
    let capture = Arc::new(Capture::default());
    let projector = RuntimeEvidenceProjector::enabled(capture.clone(), &[7_u8; 32]);
    projector.native(NativeExecutionFact {
        common: common("native"),
        operation_class: OperationClass::Tool,
        outcome: TerminalOutcome::Failed,
        failure_class: Some(FailureClass::Sandbox),
        safe_counts: SafeCounts {
            attempts: 2,
            failures: 1,
        },
    });
    projector.skill_lifecycle(SkillLifecycleFact {
        common: common("skill"),
        skill_id: "review".to_string(),
        revision: "rev-a".to_string(),
        outcome: SkillLoadOutcome::Loaded,
        anomaly: None,
        observation_count: 1,
    });
    projector.delegation(DelegatedUtilityFact {
        common: common("utility"),
        utility_skill_id: "test-runner".to_string(),
        revision: "rev-u".to_string(),
        outcome: UtilityOutcome::Succeeded,
        duration_ms: 42,
        tool_count: 3,
        approval_count: 1,
    });
    projector.verification(PlanVerificationFact {
        common: common("verification"),
        plan_run_id: "plan-1".to_string(),
        verifier: VerificationClass::Test,
        outcome: VerificationOutcome::Passed,
        passed_count: 8,
        failed_count: 0,
        predecessor_attempt_id: Some("attempt-0".to_string()),
    });
    projector.cli(CliLifecycleFact {
        kind: CliRuntimeKind::Managed,
        common: common("managed-cli"),
        outcome: TerminalOutcome::Succeeded,
        failure_class: None,
        mount_snapshot: Some(CliMountSnapshot {
            manifest_hash: "manifest-a".to_string(),
            skills: vec![MountedSkillRevision {
                skill_id: "review".to_string(),
                revision: "rev-a".to_string(),
            }],
        }),
        configured_binding_ids: vec!["binding-a".to_string()],
    });
    projector.cli(CliLifecycleFact {
        kind: CliRuntimeKind::Interactive,
        common: common("interactive-cli"),
        outcome: TerminalOutcome::Cancelled,
        failure_class: None,
        mount_snapshot: None,
        configured_binding_ids: Vec::new(),
    });

    let envelopes = capture.0.lock().expect("capture");
    assert_eq!(envelopes.len(), 6);
    assert!(envelopes.iter().all(|envelope| envelope.validate().is_ok()));
    let wire = serde_json::to_string(&*envelopes).expect("wire");
    for prohibited in [
        "prompt",
        "toolArguments",
        "toolOutput",
        "terminalOutput",
        "content",
    ] {
        assert!(!wire.contains(prohibited), "prohibited field {prohibited}");
    }
}
