use std::sync::Arc;

use crate::contexts::skill_evolution_orchestration::domain::EvolutionTriggerEnvelopeV1;

use super::{
    AuthoritativeTriggerSourceV1, EvolutionTriggerProjectorV1, RelevantMutationKindV1,
    RuntimeCompletionKindV1, TriggerProjectionError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionTriggerReceiptError {
    InvalidInput,
    Conflict,
    Corrupt,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EvolutionTriggerReceiptOutcomeV1 {
    Duplicate {
        receipt_id: String,
    },
    Queued {
        receipt_id: String,
        request_id: String,
        created_request: bool,
        follow_up: bool,
        not_before_ms: i64,
    },
}

pub(crate) trait EvolutionTriggerReceiptPort: Send + Sync {
    fn record(
        &self,
        trigger: &EvolutionTriggerEnvelopeV1,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerReceiptError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionTriggerIngressError {
    Projection(TriggerProjectionError),
    Receipt(EvolutionTriggerReceiptError),
}

#[derive(Clone)]
pub(crate) struct EvolutionTriggerIngressService {
    receipts: Arc<dyn EvolutionTriggerReceiptPort>,
}

impl EvolutionTriggerIngressService {
    pub(crate) fn new(receipts: Arc<dyn EvolutionTriggerReceiptPort>) -> Self {
        Self { receipts }
    }

    pub(crate) fn startup_recovery(
        &self,
        source: AuthoritativeTriggerSourceV1,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerIngressError> {
        self.submit(
            EvolutionTriggerProjectorV1::startup_recovery(source),
            received_at_ms,
        )
    }

    pub(crate) fn periodic_maintenance(
        &self,
        source: AuthoritativeTriggerSourceV1,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerIngressError> {
        self.submit(
            EvolutionTriggerProjectorV1::periodic_maintenance(source),
            received_at_ms,
        )
    }

    pub(crate) fn application_idle_transition(
        &self,
        source: AuthoritativeTriggerSourceV1,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerIngressError> {
        self.submit(
            EvolutionTriggerProjectorV1::application_idle_transition(source),
            received_at_ms,
        )
    }

    pub(crate) fn runtime_completion(
        &self,
        kind: RuntimeCompletionKindV1,
        source: AuthoritativeTriggerSourceV1,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerIngressError> {
        self.submit(
            EvolutionTriggerProjectorV1::runtime_completion(kind, source),
            received_at_ms,
        )
    }

    pub(crate) fn explicit_feedback_commit(
        &self,
        source: AuthoritativeTriggerSourceV1,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerIngressError> {
        self.submit(
            EvolutionTriggerProjectorV1::explicit_feedback_commit(source),
            received_at_ms,
        )
    }

    pub(crate) fn relevant_mutation(
        &self,
        kind: RelevantMutationKindV1,
        source: AuthoritativeTriggerSourceV1,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerIngressError> {
        self.submit(
            EvolutionTriggerProjectorV1::relevant_mutation(kind, source),
            received_at_ms,
        )
    }

    pub(crate) fn manual_run_request(
        &self,
        source: AuthoritativeTriggerSourceV1,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerIngressError> {
        self.submit(
            EvolutionTriggerProjectorV1::manual_run_request(source),
            received_at_ms,
        )
    }

    fn submit(
        &self,
        projected: Result<EvolutionTriggerEnvelopeV1, TriggerProjectionError>,
        received_at_ms: i64,
    ) -> Result<EvolutionTriggerReceiptOutcomeV1, EvolutionTriggerIngressError> {
        let trigger = projected.map_err(EvolutionTriggerIngressError::Projection)?;
        self.receipts
            .record(&trigger, received_at_ms)
            .map_err(EvolutionTriggerIngressError::Receipt)
    }
}
