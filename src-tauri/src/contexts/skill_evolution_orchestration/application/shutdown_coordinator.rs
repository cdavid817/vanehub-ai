#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionShutdownStepV1 {
    StopNonRecoveryTriggers,
    RequestActiveRunCancellation,
    StopNewStageDispatch,
    SettleApplicationSagas,
    PersistCheckpoints,
    ReleaseLeases,
}

pub(crate) const EVOLUTION_SHUTDOWN_ORDER_V1: [EvolutionShutdownStepV1; 6] = [
    EvolutionShutdownStepV1::StopNonRecoveryTriggers,
    EvolutionShutdownStepV1::RequestActiveRunCancellation,
    EvolutionShutdownStepV1::StopNewStageDispatch,
    EvolutionShutdownStepV1::SettleApplicationSagas,
    EvolutionShutdownStepV1::PersistCheckpoints,
    EvolutionShutdownStepV1::ReleaseLeases,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionShutdownStepErrorV1 {
    Unavailable,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionShutdownOutcomeV1 {
    Completed,
    CompletedWithWarnings,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionShutdownWarningV1 {
    pub(crate) step: EvolutionShutdownStepV1,
    pub(crate) error: EvolutionShutdownStepErrorV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionShutdownReportV1 {
    pub(crate) outcome: EvolutionShutdownOutcomeV1,
    pub(crate) completed_steps: Vec<EvolutionShutdownStepV1>,
    pub(crate) warnings: Vec<EvolutionShutdownWarningV1>,
}

pub(crate) trait EvolutionShutdownPort: Send + Sync {
    fn perform(
        &self,
        step: EvolutionShutdownStepV1,
        deadline_at_ms: i64,
    ) -> Result<bool, EvolutionShutdownStepErrorV1>;
}

pub(crate) struct EvolutionShutdownCoordinatorV1;

impl EvolutionShutdownCoordinatorV1 {
    pub(crate) fn shutdown(
        deadline_at_ms: i64,
        port: &dyn EvolutionShutdownPort,
    ) -> EvolutionShutdownReportV1 {
        let mut completed_steps = Vec::with_capacity(EVOLUTION_SHUTDOWN_ORDER_V1.len());
        let mut warnings = Vec::new();
        let mut timed_out = deadline_at_ms < 0;
        for step in EVOLUTION_SHUTDOWN_ORDER_V1 {
            match port.perform(step, deadline_at_ms) {
                Ok(completed_in_time) => {
                    timed_out |= !completed_in_time;
                    completed_steps.push(step);
                }
                Err(error) => warnings.push(EvolutionShutdownWarningV1 { step, error }),
            }
        }
        let outcome = if timed_out {
            EvolutionShutdownOutcomeV1::TimedOut
        } else if warnings.is_empty() {
            EvolutionShutdownOutcomeV1::Completed
        } else {
            EvolutionShutdownOutcomeV1::CompletedWithWarnings
        };
        EvolutionShutdownReportV1 {
            outcome,
            completed_steps,
            warnings,
        }
    }
}
