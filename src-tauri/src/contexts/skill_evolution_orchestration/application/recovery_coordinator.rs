use crate::contexts::skill_evolution_orchestration::domain::{
    is_safe_identifier, orchestration_idempotency_key,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionRecoverySubsystemV1 {
    Evidence,
    Assessment,
    Overlay,
    Curator,
    Notification,
    RateReservation,
    Run,
}

pub(crate) const EVOLUTION_RECOVERY_ORDER_V1: [EvolutionRecoverySubsystemV1; 7] = [
    EvolutionRecoverySubsystemV1::Evidence,
    EvolutionRecoverySubsystemV1::Assessment,
    EvolutionRecoverySubsystemV1::Overlay,
    EvolutionRecoverySubsystemV1::Curator,
    EvolutionRecoverySubsystemV1::Notification,
    EvolutionRecoverySubsystemV1::RateReservation,
    EvolutionRecoverySubsystemV1::Run,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionRecoveryReceiptStatusV1 {
    NoWork,
    Reconciled,
    Retryable,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionRecoveryRequestV1 {
    pub(crate) run_id: String,
    pub(crate) subsystem: EvolutionRecoverySubsystemV1,
    pub(crate) idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionRecoveryReceiptV1 {
    pub(crate) subsystem: EvolutionRecoverySubsystemV1,
    pub(crate) receipt_id: String,
    pub(crate) status: EvolutionRecoveryReceiptStatusV1,
    pub(crate) safe_reason_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionRecoveryOutcomeV1 {
    Completed,
    Retryable,
    Terminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionRecoveryReportV1 {
    pub(crate) outcome: EvolutionRecoveryOutcomeV1,
    pub(crate) receipts: Vec<EvolutionRecoveryReceiptV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionRecoveryError {
    InvalidRun,
    InvalidReceipt,
    SubsystemUnavailable,
}

pub(crate) trait EvolutionRecoveryPort: Send + Sync {
    fn reconcile(
        &self,
        request: &EvolutionRecoveryRequestV1,
    ) -> Result<EvolutionRecoveryReceiptV1, EvolutionRecoveryError>;
}

pub(crate) struct EvolutionRecoveryCoordinatorV1;

impl EvolutionRecoveryCoordinatorV1 {
    pub(crate) fn reconcile(
        run_id: &str,
        port: &dyn EvolutionRecoveryPort,
    ) -> Result<EvolutionRecoveryReportV1, EvolutionRecoveryError> {
        if !is_safe_identifier(run_id, 128) {
            return Err(EvolutionRecoveryError::InvalidRun);
        }
        let mut receipts = Vec::with_capacity(EVOLUTION_RECOVERY_ORDER_V1.len());
        for subsystem in EVOLUTION_RECOVERY_ORDER_V1 {
            let request = EvolutionRecoveryRequestV1 {
                run_id: run_id.into(),
                subsystem,
                idempotency_key: recovery_key(run_id, subsystem)?,
            };
            let receipt = port.reconcile(&request)?;
            validate_receipt(subsystem, &receipt)?;
            let outcome = match receipt.status {
                EvolutionRecoveryReceiptStatusV1::Retryable => {
                    Some(EvolutionRecoveryOutcomeV1::Retryable)
                }
                EvolutionRecoveryReceiptStatusV1::Terminal => {
                    Some(EvolutionRecoveryOutcomeV1::Terminal)
                }
                EvolutionRecoveryReceiptStatusV1::NoWork
                | EvolutionRecoveryReceiptStatusV1::Reconciled => None,
            };
            receipts.push(receipt);
            if let Some(outcome) = outcome {
                return Ok(EvolutionRecoveryReportV1 { outcome, receipts });
            }
        }
        Ok(EvolutionRecoveryReportV1 {
            outcome: EvolutionRecoveryOutcomeV1::Completed,
            receipts,
        })
    }
}

fn recovery_key(
    run_id: &str,
    subsystem: EvolutionRecoverySubsystemV1,
) -> Result<String, EvolutionRecoveryError> {
    orchestration_idempotency_key("recovery", subsystem_name(subsystem), &run_id)
        .map_err(|_| EvolutionRecoveryError::InvalidRun)
}

fn validate_receipt(
    expected: EvolutionRecoverySubsystemV1,
    receipt: &EvolutionRecoveryReceiptV1,
) -> Result<(), EvolutionRecoveryError> {
    let safe_reason = receipt
        .safe_reason_code
        .as_ref()
        .is_none_or(|reason| is_safe_identifier(reason, 64));
    if receipt.subsystem != expected
        || !is_safe_identifier(&receipt.receipt_id, 128)
        || !safe_reason
    {
        return Err(EvolutionRecoveryError::InvalidReceipt);
    }
    Ok(())
}

fn subsystem_name(subsystem: EvolutionRecoverySubsystemV1) -> &'static str {
    match subsystem {
        EvolutionRecoverySubsystemV1::Evidence => "evidence",
        EvolutionRecoverySubsystemV1::Assessment => "assessment",
        EvolutionRecoverySubsystemV1::Overlay => "overlay",
        EvolutionRecoverySubsystemV1::Curator => "curator",
        EvolutionRecoverySubsystemV1::Notification => "notification",
        EvolutionRecoverySubsystemV1::RateReservation => "rate-reservation",
        EvolutionRecoverySubsystemV1::Run => "run",
    }
}
