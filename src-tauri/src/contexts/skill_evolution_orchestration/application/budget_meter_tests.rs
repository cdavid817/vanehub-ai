use super::*;
use crate::contexts::skill_evolution_orchestration::domain::{
    EvolutionRunBudgetV1, EvolutionRunUsageV1,
};

#[test]
fn automatic_budget_accepts_every_exact_version_one_boundary() {
    let budget = EvolutionRunBudgetV1::automatic_v1();
    let delta = EvolutionRunUsageV1 {
        elapsed_ms: budget.wall_time_ms,
        evidence_items: budget.evidence_items,
        seed_groups: budget.seed_groups,
        assessments: budget.assessments,
        model_calls: budget.model_calls,
        notifications: budget.notifications,
        automatic_mutations: budget.automatic_mutations,
    };
    assert_eq!(
        apply_committed_usage(&budget, &EvolutionRunUsageV1::default(), &delta),
        EvolutionBudgetDecisionV1::Accepted { usage: delta }
    );
}

#[test]
fn each_resource_rejects_the_first_unit_above_its_limit() {
    let budget = EvolutionRunBudgetV1::automatic_v1();
    let cases = [
        (
            EvolutionRunUsageV1 {
                elapsed_ms: budget.wall_time_ms + 1,
                ..EvolutionRunUsageV1::default()
            },
            EvolutionBudgetResourceV1::WallTime,
        ),
        (
            EvolutionRunUsageV1 {
                evidence_items: budget.evidence_items + 1,
                ..EvolutionRunUsageV1::default()
            },
            EvolutionBudgetResourceV1::EvidenceItems,
        ),
        (
            EvolutionRunUsageV1 {
                seed_groups: budget.seed_groups + 1,
                ..EvolutionRunUsageV1::default()
            },
            EvolutionBudgetResourceV1::SeedGroups,
        ),
        (
            EvolutionRunUsageV1 {
                assessments: budget.assessments + 1,
                ..EvolutionRunUsageV1::default()
            },
            EvolutionBudgetResourceV1::Assessments,
        ),
        (
            EvolutionRunUsageV1 {
                model_calls: budget.model_calls + 1,
                ..EvolutionRunUsageV1::default()
            },
            EvolutionBudgetResourceV1::ModelCalls,
        ),
        (
            EvolutionRunUsageV1 {
                notifications: budget.notifications + 1,
                ..EvolutionRunUsageV1::default()
            },
            EvolutionBudgetResourceV1::Notifications,
        ),
        (
            EvolutionRunUsageV1 {
                automatic_mutations: budget.automatic_mutations + 1,
                ..EvolutionRunUsageV1::default()
            },
            EvolutionBudgetResourceV1::AutomaticMutations,
        ),
    ];
    for (delta, resource) in cases {
        assert_eq!(
            apply_committed_usage(&budget, &EvolutionRunUsageV1::default(), &delta),
            EvolutionBudgetDecisionV1::Exhausted { resource }
        );
    }
}

#[test]
fn committed_usage_accumulates_and_overflow_fails_closed() {
    let budget = EvolutionRunBudgetV1::manual_v1();
    let current = EvolutionRunUsageV1 {
        evidence_items: 4_999,
        ..EvolutionRunUsageV1::default()
    };
    let one = EvolutionRunUsageV1 {
        evidence_items: 1,
        ..EvolutionRunUsageV1::default()
    };
    assert!(matches!(
        apply_committed_usage(&budget, &current, &one),
        EvolutionBudgetDecisionV1::Accepted { .. }
    ));
    let overflow_current = EvolutionRunUsageV1 {
        elapsed_ms: u64::MAX,
        ..EvolutionRunUsageV1::default()
    };
    let elapsed = EvolutionRunUsageV1 {
        elapsed_ms: 1,
        ..EvolutionRunUsageV1::default()
    };
    assert_eq!(
        apply_committed_usage(&budget, &overflow_current, &elapsed),
        EvolutionBudgetDecisionV1::CounterOverflow {
            resource: EvolutionBudgetResourceV1::WallTime
        }
    );
}
