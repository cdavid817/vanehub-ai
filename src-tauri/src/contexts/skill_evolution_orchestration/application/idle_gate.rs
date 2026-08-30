use crate::contexts::skill_evolution_orchestration::domain::{
    EvolutionIdleSnapshotV1, EvolutionResourcePressure, ORCHESTRATION_SCHEMA_VERSION_V1,
};

pub(crate) const AUTOMATIC_USER_IDLE_MS_V1: i64 = 60_000;
pub(crate) const MAXIMUM_IDLE_WAIT_MS_V1: i64 = 900_000;
pub(crate) const MUTATION_SNAPSHOT_MAX_AGE_MS_V1: i64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionRuntimeLeaseKindV1 {
    AgentStarting,
    AgentGeneration,
    ManagedCliProcess,
    DelegatedUtility,
    PendingApproval,
    Verification,
    SkillWriter,
    OverlayWriter,
    CuratorWriter,
    ApplicationSaga,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionRuntimeLeaseV1 {
    pub(crate) workspace_id: String,
    pub(crate) kind: EvolutionRuntimeLeaseKindV1,
    pub(crate) active_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdleAggregationError {
    CrossWorkspaceLease,
    CounterOverflow,
    InvalidTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdleGatePurposeV1 {
    AutomaticRead,
    ManualRead,
    Writer,
    MutationPreflight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IdleGateDecisionV1 {
    Ready,
    Waiting { safe_reason_codes: Vec<String> },
    Deferred { safe_reason_code: String },
    StaleSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IdleSnapshotInputV1 {
    pub(crate) workspace_id: String,
    pub(crate) captured_at_ms: i64,
    pub(crate) last_user_interaction_at_ms: i64,
    pub(crate) shutting_down: bool,
    pub(crate) resource_pressure: EvolutionResourcePressure,
}

pub(crate) fn aggregate_idle_snapshot(
    input: IdleSnapshotInputV1,
    leases: &[EvolutionRuntimeLeaseV1],
) -> Result<EvolutionIdleSnapshotV1, IdleAggregationError> {
    if input.captured_at_ms < 0
        || input.last_user_interaction_at_ms < 0
        || input.last_user_interaction_at_ms > input.captured_at_ms
    {
        return Err(IdleAggregationError::InvalidTime);
    }
    let mut snapshot = EvolutionIdleSnapshotV1 {
        schema_version: ORCHESTRATION_SCHEMA_VERSION_V1,
        workspace_id: input.workspace_id,
        captured_at_ms: input.captured_at_ms,
        last_user_interaction_at_ms: input.last_user_interaction_at_ms,
        active_agent_generations: 0,
        active_cli_processes: 0,
        active_delegated_utilities: 0,
        pending_approvals: 0,
        active_verifications: 0,
        active_skill_writers: 0,
        active_overlay_writers: 0,
        active_curator_writers: 0,
        active_application_sagas: 0,
        shutting_down: input.shutting_down,
        resource_pressure: input.resource_pressure,
    };
    for lease in leases {
        if lease.workspace_id != snapshot.workspace_id {
            return Err(IdleAggregationError::CrossWorkspaceLease);
        }
        let counter = counter_for(&mut snapshot, lease.kind);
        *counter = counter
            .checked_add(lease.active_count)
            .ok_or(IdleAggregationError::CounterOverflow)?;
    }
    Ok(snapshot)
}

pub(crate) fn evaluate_idle_gate(
    snapshot: &EvolutionIdleSnapshotV1,
    purpose: IdleGatePurposeV1,
    wait_started_at_ms: i64,
    now_ms: i64,
) -> IdleGateDecisionV1 {
    if now_ms < snapshot.captured_at_ms || wait_started_at_ms > now_ms {
        return IdleGateDecisionV1::StaleSnapshot;
    }
    if purpose == IdleGatePurposeV1::MutationPreflight
        && now_ms - snapshot.captured_at_ms > MUTATION_SNAPSHOT_MAX_AGE_MS_V1
    {
        return IdleGateDecisionV1::StaleSnapshot;
    }
    let reasons = blocking_reasons(snapshot, purpose, now_ms);
    if reasons.is_empty() {
        return IdleGateDecisionV1::Ready;
    }
    if purpose == IdleGatePurposeV1::AutomaticRead
        && now_ms - wait_started_at_ms >= MAXIMUM_IDLE_WAIT_MS_V1
    {
        return IdleGateDecisionV1::Deferred {
            safe_reason_code: "idle-wait-timeout".into(),
        };
    }
    IdleGateDecisionV1::Waiting {
        safe_reason_codes: reasons,
    }
}

fn blocking_reasons(
    snapshot: &EvolutionIdleSnapshotV1,
    purpose: IdleGatePurposeV1,
    now_ms: i64,
) -> Vec<String> {
    let mut reasons = Vec::new();
    let requires_runtime_clear = purpose != IdleGatePurposeV1::ManualRead;
    if requires_runtime_clear && snapshot.active_agent_generations > 0 {
        reasons.push("agent-active".into());
    }
    if requires_runtime_clear && snapshot.active_cli_processes > 0 {
        reasons.push("cli-active".into());
    }
    if requires_runtime_clear && snapshot.active_delegated_utilities > 0 {
        reasons.push("utility-active".into());
    }
    if requires_runtime_clear && snapshot.pending_approvals > 0 {
        reasons.push("approval-pending".into());
    }
    if requires_runtime_clear && snapshot.active_verifications > 0 {
        reasons.push("verification-active".into());
    }
    if writer_activity(snapshot) {
        reasons.push("writer-active".into());
    }
    if snapshot.shutting_down {
        reasons.push("shutdown-active".into());
    }
    if snapshot.resource_pressure == EvolutionResourcePressure::Critical {
        reasons.push("resource-pressure-critical".into());
    }
    if purpose == IdleGatePurposeV1::AutomaticRead
        && now_ms - snapshot.last_user_interaction_at_ms < AUTOMATIC_USER_IDLE_MS_V1
    {
        reasons.push("user-active".into());
    }
    reasons
}

fn writer_activity(snapshot: &EvolutionIdleSnapshotV1) -> bool {
    snapshot.active_skill_writers > 0
        || snapshot.active_overlay_writers > 0
        || snapshot.active_curator_writers > 0
        || snapshot.active_application_sagas > 0
}

fn counter_for(
    snapshot: &mut EvolutionIdleSnapshotV1,
    kind: EvolutionRuntimeLeaseKindV1,
) -> &mut u16 {
    match kind {
        EvolutionRuntimeLeaseKindV1::AgentStarting
        | EvolutionRuntimeLeaseKindV1::AgentGeneration => &mut snapshot.active_agent_generations,
        EvolutionRuntimeLeaseKindV1::ManagedCliProcess => &mut snapshot.active_cli_processes,
        EvolutionRuntimeLeaseKindV1::DelegatedUtility => &mut snapshot.active_delegated_utilities,
        EvolutionRuntimeLeaseKindV1::PendingApproval => &mut snapshot.pending_approvals,
        EvolutionRuntimeLeaseKindV1::Verification => &mut snapshot.active_verifications,
        EvolutionRuntimeLeaseKindV1::SkillWriter => &mut snapshot.active_skill_writers,
        EvolutionRuntimeLeaseKindV1::OverlayWriter => &mut snapshot.active_overlay_writers,
        EvolutionRuntimeLeaseKindV1::CuratorWriter => &mut snapshot.active_curator_writers,
        EvolutionRuntimeLeaseKindV1::ApplicationSaga => &mut snapshot.active_application_sagas,
    }
}
