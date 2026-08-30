use std::sync::Arc;

use super::*;

struct SubsystemPort(EvolutionRecoverySubsystemV1);

impl EvolutionSubsystemRecoveryPort for SubsystemPort {
    fn subsystem(&self) -> EvolutionRecoverySubsystemV1 {
        self.0
    }

    fn reconcile_authoritative(
        &self,
        request: &EvolutionRecoveryRequestV1,
    ) -> Result<EvolutionRecoveryReceiptV1, EvolutionRecoveryError> {
        Ok(EvolutionRecoveryReceiptV1 {
            subsystem: request.subsystem,
            receipt_id: format!("receipt-{}", request.idempotency_key),
            status: EvolutionRecoveryReceiptStatusV1::Reconciled,
            safe_reason_code: None,
        })
    }
}

#[test]
fn registry_requires_exactly_one_authoritative_port_for_every_subsystem() {
    let complete: Vec<Arc<dyn EvolutionSubsystemRecoveryPort>> = EVOLUTION_RECOVERY_ORDER_V1
        .into_iter()
        .map(|subsystem| {
            Arc::new(SubsystemPort(subsystem)) as Arc<dyn EvolutionSubsystemRecoveryPort>
        })
        .collect();
    let registry = EvolutionRecoveryRegistryV1::new(complete).expect("complete registry");
    assert_eq!(
        EvolutionRecoveryCoordinatorV1::reconcile("run-one", &registry)
            .expect("reconciliation")
            .receipts
            .len(),
        7
    );
    let incomplete: Vec<Arc<dyn EvolutionSubsystemRecoveryPort>> =
        vec![Arc::new(SubsystemPort(EvolutionRecoverySubsystemV1::Run))];
    assert!(matches!(
        EvolutionRecoveryRegistryV1::new(incomplete),
        Err(EvolutionRecoveryRegistryError::Incomplete)
    ));
}

#[test]
fn duplicate_port_cannot_silently_replace_an_authoritative_subsystem() {
    let mut ports: Vec<Arc<dyn EvolutionSubsystemRecoveryPort>> = EVOLUTION_RECOVERY_ORDER_V1
        .into_iter()
        .map(|subsystem| {
            Arc::new(SubsystemPort(subsystem)) as Arc<dyn EvolutionSubsystemRecoveryPort>
        })
        .collect();
    ports.pop();
    ports.push(Arc::new(SubsystemPort(
        EvolutionRecoverySubsystemV1::Evidence,
    )));
    assert!(matches!(
        EvolutionRecoveryRegistryV1::new(ports),
        Err(EvolutionRecoveryRegistryError::Duplicate)
    ));
}
