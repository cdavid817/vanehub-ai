use std::sync::Mutex;

use super::*;

struct RecordingRecoveryPort {
    requests: Mutex<Vec<EvolutionRecoveryRequestV1>>,
    stop_at: Option<(
        EvolutionRecoverySubsystemV1,
        EvolutionRecoveryReceiptStatusV1,
    )>,
}

impl EvolutionRecoveryPort for RecordingRecoveryPort {
    fn reconcile(
        &self,
        request: &EvolutionRecoveryRequestV1,
    ) -> Result<EvolutionRecoveryReceiptV1, EvolutionRecoveryError> {
        self.requests
            .lock()
            .expect("request lock")
            .push(request.clone());
        let status = self
            .stop_at
            .filter(|(subsystem, _)| *subsystem == request.subsystem)
            .map_or(
                EvolutionRecoveryReceiptStatusV1::Reconciled,
                |(_, status)| status,
            );
        Ok(EvolutionRecoveryReceiptV1 {
            subsystem: request.subsystem,
            receipt_id: format!("receipt-{}", request.idempotency_key),
            status,
            safe_reason_code: None,
        })
    }
}

#[test]
fn recovery_queries_every_authoritative_subsystem_in_fixed_order() {
    let port = RecordingRecoveryPort {
        requests: Mutex::new(Vec::new()),
        stop_at: None,
    };
    let report = EvolutionRecoveryCoordinatorV1::reconcile("run-one", &port).expect("recovery");
    assert_eq!(report.outcome, EvolutionRecoveryOutcomeV1::Completed);
    assert_eq!(report.receipts.len(), 7);
    assert_eq!(
        port.requests
            .lock()
            .expect("request lock")
            .iter()
            .map(|request| request.subsystem)
            .collect::<Vec<_>>(),
        EVOLUTION_RECOVERY_ORDER_V1
    );
}

#[test]
fn retryable_receipt_stops_before_later_subsystems_and_keeps_stable_keys() {
    let port = RecordingRecoveryPort {
        requests: Mutex::new(Vec::new()),
        stop_at: Some((
            EvolutionRecoverySubsystemV1::Curator,
            EvolutionRecoveryReceiptStatusV1::Retryable,
        )),
    };
    let first = EvolutionRecoveryCoordinatorV1::reconcile("run-one", &port).expect("first");
    assert_eq!(first.outcome, EvolutionRecoveryOutcomeV1::Retryable);
    assert_eq!(first.receipts.len(), 4);
    let first_keys: Vec<_> = port
        .requests
        .lock()
        .expect("request lock")
        .iter()
        .map(|request| request.idempotency_key.clone())
        .collect();
    let second_port = RecordingRecoveryPort {
        requests: Mutex::new(Vec::new()),
        stop_at: port.stop_at,
    };
    EvolutionRecoveryCoordinatorV1::reconcile("run-one", &second_port).expect("second");
    assert_eq!(
        first_keys,
        second_port
            .requests
            .lock()
            .expect("request lock")
            .iter()
            .map(|request| request.idempotency_key.clone())
            .collect::<Vec<_>>()
    );
}
