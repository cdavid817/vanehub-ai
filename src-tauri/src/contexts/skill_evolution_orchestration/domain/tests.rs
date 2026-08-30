use super::*;

#[test]
fn trigger_registry_contains_exactly_ten_stable_families() {
    assert_eq!(EVOLUTION_TRIGGER_FAMILIES_V1.len(), 10);
    assert_eq!(
        serde_json::to_value(EVOLUTION_TRIGGER_FAMILIES_V1).expect("trigger registry"),
        serde_json::json!([
            "startup_recovery",
            "periodic_maintenance",
            "application_idle_transition",
            "agent_run_completion",
            "conversation_completion",
            "explicit_feedback_commit",
            "verification_completion",
            "delegated_utility_completion",
            "relevant_policy_or_skill_change",
            "manual_run_request"
        ])
    );
    assert!(serde_json::from_str::<EvolutionTriggerFamily>("\"unknown\"").is_err());
    for family in EVOLUTION_TRIGGER_FAMILIES_V1 {
        assert_eq!(
            EvolutionTriggerFamily::from_versioned_name(1, family.as_str()),
            Ok(family)
        );
    }
    assert_eq!(
        EvolutionTriggerFamily::from_versioned_name(2, "startup_recovery"),
        Err(EvolutionTriggerRegistryError::UnsupportedVersion)
    );
    assert_eq!(
        EvolutionTriggerFamily::from_versioned_name(1, "future_trigger"),
        Err(EvolutionTriggerRegistryError::UnknownFamily)
    );
}

#[test]
fn run_engine_keeps_the_fixed_eight_stage_contract() {
    assert_eq!(EVOLUTION_STAGE_ORDER_V1.len(), 8);
    assert_eq!(EVOLUTION_STAGE_ORDER_V1[0], EvolutionStageKind::Recover);
    assert_eq!(
        EVOLUTION_STAGE_ORDER_V1[4],
        EvolutionStageKind::RouteGovernance
    );
    assert_eq!(EVOLUTION_STAGE_ORDER_V1[7], EvolutionStageKind::Notify);
}

#[test]
fn version_one_budgets_match_the_reviewed_automatic_and_manual_limits() {
    assert_eq!(
        EvolutionRunBudgetV1::automatic_v1(),
        EvolutionRunBudgetV1 {
            wall_time_ms: 120_000,
            evidence_items: 1_000,
            seed_groups: 100,
            assessments: 25,
            model_calls: 10,
            notifications: 20,
            automatic_mutations: 1,
        }
    );
    assert_eq!(EvolutionRunBudgetV1::manual_v1().wall_time_ms, 300_000);
    assert_eq!(EvolutionRunBudgetV1::manual_v1().evidence_items, 5_000);
    assert_eq!(EvolutionRunBudgetV1::manual_v1().automatic_mutations, 1);
}

#[test]
fn orchestration_policy_is_default_off_without_consent_or_wildcards() {
    let policy = EvolutionOrchestrationPolicyV1::default_off("workspace-one".into(), 7);
    assert_eq!(policy.mode, EvolutionPolicyMode::Off);
    assert!(policy.allowed_skill_ids.is_empty());
    assert!(policy.consent.is_none());
    assert_eq!(policy.user_idle_ms, 60_000);
    assert_eq!(policy.maximum_idle_wait_ms, 900_000);
    assert!(!policy.notify_routine_completion);
}

#[test]
fn model_deserialization_rejects_unknown_fields_and_enum_values() {
    let policy = EvolutionOrchestrationPolicyV1::default_off("workspace-one".into(), 7);
    let mut value = serde_json::to_value(policy).expect("policy");
    value["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<EvolutionOrchestrationPolicyV1>(value).is_err());
    assert!(serde_json::from_str::<EvolutionPolicyMode>("\"automatic\"").is_err());
    assert!(serde_json::from_str::<EvolutionRunStatus>("\"paused\"").is_err());
}

#[test]
fn idle_snapshot_reports_every_runtime_and_writer_blocker() {
    let mut snapshot = empty_idle_snapshot("workspace-one".into(), 10);
    assert!(!snapshot.has_runtime_or_writer_activity());
    snapshot.pending_approvals = 1;
    assert!(snapshot.has_runtime_or_writer_activity());
    snapshot.pending_approvals = 0;
    snapshot.resource_pressure = EvolutionResourcePressure::Critical;
    assert!(snapshot.has_runtime_or_writer_activity());
}

#[test]
fn run_status_transition_table_is_stable_and_terminal_states_never_reopen() {
    assert!(EvolutionRunStatus::Requested.can_transition_to(EvolutionRunStatus::WaitingIdle));
    assert!(EvolutionRunStatus::Running.can_transition_to(EvolutionRunStatus::Completed));
    assert!(EvolutionRunStatus::CancelRequested.can_transition_to(EvolutionRunStatus::Cancelled));
    assert!(!EvolutionRunStatus::Running.can_transition_to(EvolutionRunStatus::WaitingIdle));
    for terminal in [
        EvolutionRunStatus::Completed,
        EvolutionRunStatus::Failed,
        EvolutionRunStatus::Cancelled,
    ] {
        assert!(terminal.is_terminal());
        for next in [
            EvolutionRunStatus::Requested,
            EvolutionRunStatus::Running,
            EvolutionRunStatus::Recovered,
        ] {
            assert!(!terminal.can_transition_to(next));
        }
    }
    assert_eq!(
        EvolutionRunStatus::from_persisted("future_status"),
        Err(EvolutionRunStatusParseError::UnknownStatus)
    );
}
