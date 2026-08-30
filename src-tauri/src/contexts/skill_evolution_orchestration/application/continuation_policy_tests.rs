use super::*;
use crate::contexts::skill_evolution_orchestration::domain::EvolutionRunBudgetV1;

#[test]
fn automatic_budget_reserves_five_percent_for_recovery_before_ordinary_work() {
    let budget = EvolutionRunBudgetV1::automatic_v1();
    let partition = partition_recovery_reserve(&budget);
    assert_eq!(partition.recovery_reserve.wall_time_ms, 6_000);
    assert_eq!(partition.recovery_reserve.evidence_items, 50);
    assert_eq!(partition.recovery_reserve.seed_groups, 5);
    assert_eq!(partition.recovery_reserve.assessments, 1);
    assert_eq!(partition.recovery_reserve.model_calls, 1);
    assert_eq!(partition.recovery_reserve.notifications, 1);
    assert_eq!(partition.recovery_reserve.automatic_mutations, 0);
    assert_eq!(
        partition.ordinary.wall_time_ms + partition.recovery_reserve.wall_time_ms,
        budget.wall_time_ms
    );
    assert_eq!(
        partition.ordinary.evidence_items + partition.recovery_reserve.evidence_items,
        budget.evidence_items
    );
    assert_eq!(
        partition.ordinary.automatic_mutations,
        budget.automatic_mutations
    );
}

#[test]
fn continuation_backoff_doubles_and_caps_at_fifteen_minutes() {
    assert_eq!(continuation_not_before_ms(1_000, 1), Some(31_000));
    assert_eq!(continuation_not_before_ms(1_000, 2), Some(61_000));
    assert_eq!(continuation_not_before_ms(1_000, 3), Some(121_000));
    assert_eq!(continuation_not_before_ms(1_000, 20), Some(901_000));
    assert_eq!(continuation_not_before_ms(-1, 1), None);
    assert_eq!(continuation_not_before_ms(1_000, 0), None);
    assert_eq!(continuation_not_before_ms(i64::MAX, 1), None);
}
