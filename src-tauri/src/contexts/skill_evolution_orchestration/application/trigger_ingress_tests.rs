use std::sync::{Arc, Mutex};

use super::*;
use crate::contexts::skill_evolution_orchestration::domain::{
    EvolutionTriggerEnvelopeV1, EvolutionTriggerFamily,
};

#[derive(Default)]
struct RecordingReceiptPort {
    triggers: Mutex<Vec<EvolutionTriggerEnvelopeV1>>,
}

impl EvolutionTriggerReceiptPort for RecordingReceiptPort {
    fn record(
        &self,
        trigger: &EvolutionTriggerEnvelopeV1,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerReceiptError> {
        self.triggers
            .lock()
            .expect("trigger lock")
            .push(trigger.clone());
        Ok(EvolutionTriggerReceiptOutcomeV1::Queued {
            receipt_id: trigger.trigger_id.clone(),
            request_id: "request-one".into(),
            created_request: true,
            follow_up: false,
            not_before_ms: received_at_ms,
        })
    }
}

#[test]
fn ingress_exposes_typed_producers_without_accepting_an_open_family_string() {
    let port = Arc::new(RecordingReceiptPort::default());
    let ingress = EvolutionTriggerIngressService::new(port.clone());
    ingress
        .startup_recovery(source("startup"), 100)
        .expect("startup");
    ingress
        .periodic_maintenance(source("periodic"), 100)
        .expect("periodic");
    ingress
        .application_idle_transition(source("idle"), 100)
        .expect("idle");
    for (kind, id) in [
        (RuntimeCompletionKindV1::AgentRun, "run"),
        (RuntimeCompletionKindV1::Conversation, "conversation"),
        (RuntimeCompletionKindV1::Verification, "verification"),
        (RuntimeCompletionKindV1::DelegatedUtility, "utility"),
    ] {
        ingress
            .runtime_completion(kind, source(id), 100)
            .expect("completion");
    }
    ingress
        .explicit_feedback_commit(source("feedback"), 100)
        .expect("feedback");
    ingress
        .relevant_mutation(RelevantMutationKindV1::Overlay, source("overlay"), 100)
        .expect("mutation");
    ingress
        .manual_run_request(source("manual"), 100)
        .expect("manual");
    let triggers = port.triggers.lock().expect("trigger lock");
    assert_eq!(triggers.len(), 10);
    assert_eq!(
        triggers
            .iter()
            .map(|trigger| trigger.family)
            .collect::<Vec<_>>(),
        vec![
            EvolutionTriggerFamily::StartupRecovery,
            EvolutionTriggerFamily::PeriodicMaintenance,
            EvolutionTriggerFamily::ApplicationIdleTransition,
            EvolutionTriggerFamily::AgentRunCompletion,
            EvolutionTriggerFamily::ConversationCompletion,
            EvolutionTriggerFamily::VerificationCompletion,
            EvolutionTriggerFamily::DelegatedUtilityCompletion,
            EvolutionTriggerFamily::ExplicitFeedbackCommit,
            EvolutionTriggerFamily::RelevantPolicyOrSkillChange,
            EvolutionTriggerFamily::ManualRunRequest,
        ]
    );
}

#[test]
fn ingress_stops_before_persistence_when_projection_fails() {
    let port = Arc::new(RecordingReceiptPort::default());
    let ingress = EvolutionTriggerIngressService::new(port.clone());
    let mut invalid = source("manual");
    invalid.workspace_id = "unsafe workspace".into();
    assert_eq!(
        ingress.manual_run_request(invalid, 100),
        Err(EvolutionTriggerIngressError::Projection(
            TriggerProjectionError::InvalidSource
        ))
    );
    assert!(port.triggers.lock().expect("trigger lock").is_empty());
}

fn source(id: &str) -> AuthoritativeTriggerSourceV1 {
    AuthoritativeTriggerSourceV1 {
        workspace_id: "workspace-one".into(),
        source_id: id.into(),
        source_revision: 1,
        occurred_at_ms: 100,
    }
}
