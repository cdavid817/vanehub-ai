use crate::contexts::skill_evolution_generation::domain::{
    next_generation_stage, GenerationBudgetV1, GenerationCapacitySnapshotV1,
    GenerationDailyBudgetV1, GenerationDailyUsageV1, GenerationJobStatus, GenerationJobV1,
    GenerationRecoveryAction, GenerationRuntimeBlock, GenerationStageDemandV1, GenerationStageKind,
    GenerationUsageV1, GENERATION_SCHEMA_VERSION_V1, GENERATION_STAGE_ORDER_V1,
};

use super::{authorize_stage_start, reconcile_generation_job, stage_follows};

#[test]
fn seven_stages_have_one_fixed_successor_order() {
    let mut prior = None;
    for stage in GENERATION_STAGE_ORDER_V1 {
        assert_eq!(next_generation_stage(prior), Some(stage));
        assert!(stage_follows(prior, stage));
        prior = Some(stage);
    }
    assert_eq!(next_generation_stage(prior), None);
}

#[test]
fn every_job_and_daily_budget_fails_at_its_boundary() {
    let daily_budget = daily_budget();
    let capacity = GenerationCapacitySnapshotV1::default();
    let daily_usage = GenerationDailyUsageV1::default();
    let mut value = job();
    assert_eq!(
        authorize_stage_start(
            &value,
            &GenerationStageDemandV1::default(),
            &capacity,
            &daily_usage,
            &daily_budget
        ),
        Ok(())
    );
    value.usage.elapsed_ms = 180_001;
    assert_block(
        &value,
        GenerationStageDemandV1::default(),
        &capacity,
        &daily_usage,
        &daily_budget,
        GenerationRuntimeBlock::WallTimeBudget,
    );
    let mut value = job();
    value.usage.model_calls = 3;
    assert_block(
        &value,
        GenerationStageDemandV1 {
            model_calls: 1,
            ..Default::default()
        },
        &capacity,
        &daily_usage,
        &daily_budget,
        GenerationRuntimeBlock::ModelCallBudget,
    );
    let mut value = job();
    value.usage.tool_calls = 8;
    assert_block(
        &value,
        GenerationStageDemandV1 {
            tool_calls: 1,
            ..Default::default()
        },
        &capacity,
        &daily_usage,
        &daily_budget,
        GenerationRuntimeBlock::ToolCallBudget,
    );
    assert_block(
        &job(),
        GenerationStageDemandV1 {
            input_tokens: 48_001,
            ..Default::default()
        },
        &capacity,
        &daily_usage,
        &daily_budget,
        GenerationRuntimeBlock::InputTokenBudget,
    );
    assert_block(
        &job(),
        GenerationStageDemandV1 {
            output_tokens: 8_001,
            ..Default::default()
        },
        &capacity,
        &daily_usage,
        &daily_budget,
        GenerationRuntimeBlock::OutputTokenBudget,
    );
    let mut value = job();
    value.usage.validation_repairs = 1;
    assert_block(
        &value,
        GenerationStageDemandV1 {
            validation_repairs: 1,
            ..Default::default()
        },
        &capacity,
        &daily_usage,
        &daily_budget,
        GenerationRuntimeBlock::RepairBudget,
    );
    let workspace_full = GenerationCapacitySnapshotV1 {
        other_running_in_workspace: 1,
        other_running_globally: 1,
    };
    assert_block(
        &job(),
        GenerationStageDemandV1::default(),
        &workspace_full,
        &daily_usage,
        &daily_budget,
        GenerationRuntimeBlock::WorkspaceCapacity,
    );
    let global_full = GenerationCapacitySnapshotV1 {
        other_running_in_workspace: 0,
        other_running_globally: 2,
    };
    assert_block(
        &job(),
        GenerationStageDemandV1::default(),
        &global_full,
        &daily_usage,
        &daily_budget,
        GenerationRuntimeBlock::GlobalCapacity,
    );
    let daily_input_full = GenerationDailyUsageV1 {
        input_tokens: 250_000,
        output_tokens: 0,
    };
    assert_block(
        &job(),
        GenerationStageDemandV1 {
            input_tokens: 1,
            ..Default::default()
        },
        &capacity,
        &daily_input_full,
        &daily_budget,
        GenerationRuntimeBlock::DailyInputTokenBudget,
    );
}

#[test]
fn cancellation_supersession_and_restart_recovery_are_deterministic() {
    let mut value = job();
    value.status = GenerationJobStatus::CancelRequested;
    assert_eq!(
        reconcile_generation_job(
            &value,
            Some(GenerationStageKind::BuildDossier),
            "sha256:input"
        ),
        GenerationRecoveryAction::Cancel
    );
    let mut value = job();
    value.status = GenerationJobStatus::Running;
    assert_eq!(
        reconcile_generation_job(
            &value,
            Some(GenerationStageKind::BuildDossier),
            "sha256:changed"
        ),
        GenerationRecoveryAction::Supersede
    );
    assert_eq!(
        reconcile_generation_job(
            &value,
            Some(GenerationStageKind::BuildDossier),
            "sha256:input"
        ),
        GenerationRecoveryAction::ResumeAt(GenerationStageKind::PlanMutation)
    );
    assert_eq!(
        reconcile_generation_job(
            &value,
            Some(GenerationStageKind::PackageForGovernance),
            "sha256:input"
        ),
        GenerationRecoveryAction::Complete
    );
}

fn assert_block(
    job: &GenerationJobV1,
    demand: GenerationStageDemandV1,
    capacity: &GenerationCapacitySnapshotV1,
    daily_usage: &GenerationDailyUsageV1,
    daily_budget: &GenerationDailyBudgetV1,
    expected: GenerationRuntimeBlock,
) {
    assert_eq!(
        authorize_stage_start(job, &demand, capacity, daily_usage, daily_budget),
        Err(expected)
    );
}

fn daily_budget() -> GenerationDailyBudgetV1 {
    GenerationDailyBudgetV1 {
        input_tokens: 250_000,
        output_tokens: 50_000,
        concurrent_workspace_jobs: 1,
        concurrent_global_jobs: 2,
    }
}

fn job() -> GenerationJobV1 {
    GenerationJobV1 {
        schema_version: GENERATION_SCHEMA_VERSION_V1,
        job_id: "job".into(),
        request_id: "request".into(),
        workspace_id: Some("workspace".into()),
        status: GenerationJobStatus::Queued,
        current_stage: None,
        input_witness_hash: "sha256:input".into(),
        current_attempt: 1,
        budget: GenerationBudgetV1 {
            wall_time_ms: 180_000,
            model_calls: 3,
            tool_calls: 8,
            input_tokens: 48_000,
            output_tokens: 8_000,
            validation_repairs: 1,
        },
        usage: GenerationUsageV1::default(),
        safe_failure_code: None,
        supersedes_job_id: None,
        created_at_ms: 1,
        updated_at_ms: 1,
    }
}
