use crate::contexts::skill_evolution_orchestration::{
    application::{
        EvolutionTriggerReceiptError, EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerReceiptPort,
    },
    domain::EvolutionTriggerEnvelopeV1,
};

use super::{OrchestrationPersistenceError, OrchestrationRepository, ReceiveTriggerOutcome};

impl EvolutionTriggerReceiptPort for OrchestrationRepository {
    fn record(
        &self,
        trigger: &EvolutionTriggerEnvelopeV1,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerReceiptError> {
        self.receive_trigger(trigger, received_at_ms)
            .map(map_receive_outcome)
            .map_err(map_persistence_error)
    }
}

fn map_receive_outcome(outcome: ReceiveTriggerOutcome) -> EvolutionTriggerReceiptOutcomeV1 {
    match outcome {
        ReceiveTriggerOutcome::Duplicate { receipt_id } => {
            EvolutionTriggerReceiptOutcomeV1::Duplicate { receipt_id }
        }
        ReceiveTriggerOutcome::Queued {
            receipt_id,
            request_id,
            created_request,
            follow_up,
            not_before_ms,
        } => EvolutionTriggerReceiptOutcomeV1::Queued {
            receipt_id,
            request_id,
            created_request,
            follow_up,
            not_before_ms,
        },
    }
}

fn map_persistence_error(error: OrchestrationPersistenceError) -> EvolutionTriggerReceiptError {
    match error {
        OrchestrationPersistenceError::InvalidInput => EvolutionTriggerReceiptError::InvalidInput,
        OrchestrationPersistenceError::Conflict => EvolutionTriggerReceiptError::Conflict,
        OrchestrationPersistenceError::Corrupt => EvolutionTriggerReceiptError::Corrupt,
        OrchestrationPersistenceError::NotFound | OrchestrationPersistenceError::Storage => {
            EvolutionTriggerReceiptError::Unavailable
        }
    }
}
