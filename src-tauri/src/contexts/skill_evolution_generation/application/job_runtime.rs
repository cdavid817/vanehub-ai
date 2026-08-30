use crate::contexts::skill_evolution_generation::domain::{
    next_generation_stage, GenerationCapacitySnapshotV1, GenerationDailyBudgetV1,
    GenerationDailyUsageV1, GenerationJobStatus, GenerationJobV1, GenerationRecoveryAction,
    GenerationRuntimeBlock, GenerationStageDemandV1, GenerationStageKind,
};

pub(crate) fn authorize_stage_start(
    job: &GenerationJobV1,
    demand: &GenerationStageDemandV1,
    capacity: &GenerationCapacitySnapshotV1,
    daily_usage: &GenerationDailyUsageV1,
    daily_budget: &GenerationDailyBudgetV1,
) -> Result<(), GenerationRuntimeBlock> {
    if !matches!(
        job.status,
        GenerationJobStatus::Requested | GenerationJobStatus::Queued | GenerationJobStatus::Running
    ) {
        return Err(GenerationRuntimeBlock::InvalidState);
    }
    if capacity.other_running_in_workspace >= daily_budget.concurrent_workspace_jobs {
        return Err(GenerationRuntimeBlock::WorkspaceCapacity);
    }
    if capacity.other_running_globally >= daily_budget.concurrent_global_jobs {
        return Err(GenerationRuntimeBlock::GlobalCapacity);
    }
    let projected = demand.projected_usage(&job.usage);
    if projected.elapsed_ms > job.budget.wall_time_ms {
        return Err(GenerationRuntimeBlock::WallTimeBudget);
    }
    if projected.model_calls > job.budget.model_calls {
        return Err(GenerationRuntimeBlock::ModelCallBudget);
    }
    if projected.tool_calls > job.budget.tool_calls {
        return Err(GenerationRuntimeBlock::ToolCallBudget);
    }
    if projected.input_tokens > job.budget.input_tokens {
        return Err(GenerationRuntimeBlock::InputTokenBudget);
    }
    if projected.output_tokens > job.budget.output_tokens {
        return Err(GenerationRuntimeBlock::OutputTokenBudget);
    }
    if projected.validation_repairs > job.budget.validation_repairs {
        return Err(GenerationRuntimeBlock::RepairBudget);
    }
    if daily_usage.input_tokens.saturating_add(demand.input_tokens) > daily_budget.input_tokens {
        return Err(GenerationRuntimeBlock::DailyInputTokenBudget);
    }
    if daily_usage
        .output_tokens
        .saturating_add(demand.output_tokens)
        > daily_budget.output_tokens
    {
        return Err(GenerationRuntimeBlock::DailyOutputTokenBudget);
    }
    Ok(())
}

pub(crate) fn reconcile_generation_job(
    job: &GenerationJobV1,
    last_completed_stage: Option<GenerationStageKind>,
    current_input_witness_hash: &str,
) -> GenerationRecoveryAction {
    if job.status == GenerationJobStatus::CancelRequested {
        return GenerationRecoveryAction::Cancel;
    }
    if job.input_witness_hash != current_input_witness_hash {
        return GenerationRecoveryAction::Supersede;
    }
    if matches!(
        job.status,
        GenerationJobStatus::Completed
            | GenerationJobStatus::Cancelled
            | GenerationJobStatus::Failed
            | GenerationJobStatus::Superseded
            | GenerationJobStatus::BlockedConsent
    ) {
        return GenerationRecoveryAction::NoAction;
    }
    match next_generation_stage(last_completed_stage) {
        Some(stage) => GenerationRecoveryAction::ResumeAt(stage),
        None => GenerationRecoveryAction::Complete,
    }
}

pub(crate) fn stage_follows(
    prior: Option<GenerationStageKind>,
    requested: GenerationStageKind,
) -> bool {
    next_generation_stage(prior) == Some(requested)
}
