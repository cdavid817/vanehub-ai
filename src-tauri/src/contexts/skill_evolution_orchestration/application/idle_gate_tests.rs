use super::*;
use crate::contexts::skill_evolution_orchestration::domain::EvolutionResourcePressure;

#[test]
fn aggregator_projects_every_authoritative_runtime_and_writer_lease() {
    let kinds = [
        EvolutionRuntimeLeaseKindV1::AgentStarting,
        EvolutionRuntimeLeaseKindV1::AgentGeneration,
        EvolutionRuntimeLeaseKindV1::ManagedCliProcess,
        EvolutionRuntimeLeaseKindV1::DelegatedUtility,
        EvolutionRuntimeLeaseKindV1::PendingApproval,
        EvolutionRuntimeLeaseKindV1::Verification,
        EvolutionRuntimeLeaseKindV1::SkillWriter,
        EvolutionRuntimeLeaseKindV1::OverlayWriter,
        EvolutionRuntimeLeaseKindV1::CuratorWriter,
        EvolutionRuntimeLeaseKindV1::ApplicationSaga,
    ];
    let leases: Vec<_> = kinds
        .into_iter()
        .map(|kind| EvolutionRuntimeLeaseV1 {
            workspace_id: "workspace-one".into(),
            kind,
            active_count: 1,
        })
        .collect();
    let snapshot = aggregate_idle_snapshot(input(100_000, 0), &leases).expect("snapshot");
    assert_eq!(snapshot.active_agent_generations, 2);
    assert_eq!(snapshot.active_cli_processes, 1);
    assert_eq!(snapshot.active_delegated_utilities, 1);
    assert_eq!(snapshot.pending_approvals, 1);
    assert_eq!(snapshot.active_verifications, 1);
    assert_eq!(snapshot.active_skill_writers, 1);
    assert_eq!(snapshot.active_overlay_writers, 1);
    assert_eq!(snapshot.active_curator_writers, 1);
    assert_eq!(snapshot.active_application_sagas, 1);
}

#[test]
fn automatic_gate_waits_for_quiescence_and_defers_at_the_bound() {
    let snapshot = aggregate_idle_snapshot(
        input(10_000, 9_999),
        &[lease(EvolutionRuntimeLeaseKindV1::AgentGeneration)],
    )
    .expect("snapshot");
    assert!(matches!(
        evaluate_idle_gate(&snapshot, IdleGatePurposeV1::AutomaticRead, 0, 10_000),
        IdleGateDecisionV1::Waiting { .. }
    ));
    assert_eq!(
        evaluate_idle_gate(&snapshot, IdleGatePurposeV1::AutomaticRead, 10_000, 910_000),
        IdleGateDecisionV1::Deferred {
            safe_reason_code: "idle-wait-timeout".into()
        }
    );
}

#[test]
fn manual_read_bypasses_user_and_agent_idle_but_never_writer_or_shutdown_gates() {
    let active_agent = aggregate_idle_snapshot(
        input(10_000, 10_000),
        &[lease(EvolutionRuntimeLeaseKindV1::AgentGeneration)],
    )
    .expect("agent snapshot");
    assert_eq!(
        evaluate_idle_gate(&active_agent, IdleGatePurposeV1::ManualRead, 10_000, 10_000),
        IdleGateDecisionV1::Ready
    );
    let writer = aggregate_idle_snapshot(
        input(10_000, 10_000),
        &[lease(EvolutionRuntimeLeaseKindV1::OverlayWriter)],
    )
    .expect("writer snapshot");
    assert!(matches!(
        evaluate_idle_gate(&writer, IdleGatePurposeV1::ManualRead, 10_000, 10_000),
        IdleGateDecisionV1::Waiting { .. }
    ));
}

#[test]
fn mutation_preflight_rejects_snapshots_older_than_five_seconds() {
    let snapshot = aggregate_idle_snapshot(input(10_000, 0), &[]).expect("snapshot");
    assert_eq!(
        evaluate_idle_gate(&snapshot, IdleGatePurposeV1::MutationPreflight, 0, 15_001),
        IdleGateDecisionV1::StaleSnapshot
    );
    assert_eq!(
        evaluate_idle_gate(&snapshot, IdleGatePurposeV1::MutationPreflight, 0, 15_000),
        IdleGateDecisionV1::Ready
    );
}

#[test]
fn mutation_preflight_races_fail_closed_for_every_authoritative_transition() {
    let ready = aggregate_idle_snapshot(input(100_000, 0), &[]).expect("ready snapshot");
    assert_eq!(
        evaluate_idle_gate(&ready, IdleGatePurposeV1::MutationPreflight, 0, 100_000),
        IdleGateDecisionV1::Ready
    );
    for kind in [
        EvolutionRuntimeLeaseKindV1::AgentGeneration,
        EvolutionRuntimeLeaseKindV1::PendingApproval,
        EvolutionRuntimeLeaseKindV1::Verification,
        EvolutionRuntimeLeaseKindV1::OverlayWriter,
        EvolutionRuntimeLeaseKindV1::CuratorWriter,
        EvolutionRuntimeLeaseKindV1::ApplicationSaga,
    ] {
        let raced =
            aggregate_idle_snapshot(input(100_001, 0), &[lease(kind)]).expect("raced snapshot");
        assert!(matches!(
            evaluate_idle_gate(&raced, IdleGatePurposeV1::MutationPreflight, 0, 100_001),
            IdleGateDecisionV1::Waiting { .. }
        ));
    }
    let mut shutdown = input(100_001, 0);
    shutdown.shutting_down = true;
    let shutdown = aggregate_idle_snapshot(shutdown, &[]).expect("shutdown snapshot");
    assert!(matches!(
        evaluate_idle_gate(&shutdown, IdleGatePurposeV1::MutationPreflight, 0, 100_001),
        IdleGateDecisionV1::Waiting { .. }
    ));
}

#[test]
fn aggregation_rejects_cross_workspace_leases_and_counter_overflow() {
    let mut other = lease(EvolutionRuntimeLeaseKindV1::Verification);
    other.workspace_id = "workspace-two".into();
    assert_eq!(
        aggregate_idle_snapshot(input(10, 0), &[other]),
        Err(IdleAggregationError::CrossWorkspaceLease)
    );
    let leases = [
        EvolutionRuntimeLeaseV1 {
            active_count: u16::MAX,
            ..lease(EvolutionRuntimeLeaseKindV1::AgentStarting)
        },
        lease(EvolutionRuntimeLeaseKindV1::AgentGeneration),
    ];
    assert_eq!(
        aggregate_idle_snapshot(input(10, 0), &leases),
        Err(IdleAggregationError::CounterOverflow)
    );
}

fn input(captured_at_ms: i64, last_user_interaction_at_ms: i64) -> IdleSnapshotInputV1 {
    IdleSnapshotInputV1 {
        workspace_id: "workspace-one".into(),
        captured_at_ms,
        last_user_interaction_at_ms,
        shutting_down: false,
        resource_pressure: EvolutionResourcePressure::Normal,
    }
}

fn lease(kind: EvolutionRuntimeLeaseKindV1) -> EvolutionRuntimeLeaseV1 {
    EvolutionRuntimeLeaseV1 {
        workspace_id: "workspace-one".into(),
        kind,
        active_count: 1,
    }
}
