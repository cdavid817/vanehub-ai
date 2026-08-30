use super::{GenerationStageKind, GenerationUsageV1};

pub(crate) const GENERATION_STAGE_ORDER_V1: [GenerationStageKind; 7] = [
    GenerationStageKind::FreezeInput,
    GenerationStageKind::InspectTarget,
    GenerationStageKind::BuildDossier,
    GenerationStageKind::PlanMutation,
    GenerationStageKind::SynthesizeStructuredDraft,
    GenerationStageKind::ValidateAndSimulate,
    GenerationStageKind::PackageForGovernance,
];

pub(crate) fn next_generation_stage(
    completed: Option<GenerationStageKind>,
) -> Option<GenerationStageKind> {
    match completed {
        None => Some(GENERATION_STAGE_ORDER_V1[0]),
        Some(stage) => GENERATION_STAGE_ORDER_V1
            .iter()
            .position(|candidate| *candidate == stage)
            .and_then(|index| GENERATION_STAGE_ORDER_V1.get(index + 1))
            .copied(),
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GenerationCapacitySnapshotV1 {
    pub(crate) other_running_in_workspace: u8,
    pub(crate) other_running_globally: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GenerationDailyUsageV1 {
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GenerationStageDemandV1 {
    pub(crate) model_calls: u16,
    pub(crate) tool_calls: u16,
    pub(crate) input_tokens: u64,
    pub(crate) output_tokens: u64,
    pub(crate) validation_repairs: u8,
}

impl GenerationStageDemandV1 {
    pub(crate) fn projected_usage(&self, current: &GenerationUsageV1) -> GenerationUsageV1 {
        GenerationUsageV1 {
            elapsed_ms: current.elapsed_ms,
            model_calls: current.model_calls.saturating_add(self.model_calls),
            tool_calls: current.tool_calls.saturating_add(self.tool_calls),
            input_tokens: current.input_tokens.saturating_add(self.input_tokens),
            output_tokens: current.output_tokens.saturating_add(self.output_tokens),
            validation_repairs: current
                .validation_repairs
                .saturating_add(self.validation_repairs),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationRuntimeBlock {
    InvalidState,
    WorkspaceCapacity,
    GlobalCapacity,
    WallTimeBudget,
    ModelCallBudget,
    ToolCallBudget,
    InputTokenBudget,
    OutputTokenBudget,
    DailyInputTokenBudget,
    DailyOutputTokenBudget,
    RepairBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationRecoveryAction {
    ResumeAt(GenerationStageKind),
    Complete,
    Cancel,
    Supersede,
    NoAction,
}
