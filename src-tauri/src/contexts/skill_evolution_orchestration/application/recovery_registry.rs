use std::sync::Arc;

use super::{
    EvolutionRecoveryError, EvolutionRecoveryPort, EvolutionRecoveryReceiptV1,
    EvolutionRecoveryRequestV1, EvolutionRecoverySubsystemV1, EVOLUTION_RECOVERY_ORDER_V1,
};

pub(crate) trait EvolutionSubsystemRecoveryPort: Send + Sync {
    fn subsystem(&self) -> EvolutionRecoverySubsystemV1;

    fn reconcile_authoritative(
        &self,
        request: &EvolutionRecoveryRequestV1,
    ) -> Result<EvolutionRecoveryReceiptV1, EvolutionRecoveryError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionRecoveryRegistryError {
    Incomplete,
    Duplicate,
}

pub(crate) struct EvolutionRecoveryRegistryV1 {
    ports: Vec<Arc<dyn EvolutionSubsystemRecoveryPort>>,
}

impl EvolutionRecoveryRegistryV1 {
    pub(crate) fn new(
        ports: Vec<Arc<dyn EvolutionSubsystemRecoveryPort>>,
    ) -> Result<Self, EvolutionRecoveryRegistryError> {
        if ports.len() != EVOLUTION_RECOVERY_ORDER_V1.len() {
            return Err(EvolutionRecoveryRegistryError::Incomplete);
        }
        for subsystem in EVOLUTION_RECOVERY_ORDER_V1 {
            let count = ports
                .iter()
                .filter(|port| port.subsystem() == subsystem)
                .count();
            if count == 0 {
                return Err(EvolutionRecoveryRegistryError::Incomplete);
            }
            if count > 1 {
                return Err(EvolutionRecoveryRegistryError::Duplicate);
            }
        }
        Ok(Self { ports })
    }
}

impl EvolutionRecoveryPort for EvolutionRecoveryRegistryV1 {
    fn reconcile(
        &self,
        request: &EvolutionRecoveryRequestV1,
    ) -> Result<EvolutionRecoveryReceiptV1, EvolutionRecoveryError> {
        self.ports
            .iter()
            .find(|port| port.subsystem() == request.subsystem)
            .ok_or(EvolutionRecoveryError::SubsystemUnavailable)?
            .reconcile_authoritative(request)
    }
}
