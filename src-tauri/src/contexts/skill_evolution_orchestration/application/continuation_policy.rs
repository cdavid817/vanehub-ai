use crate::contexts::skill_evolution_orchestration::domain::EvolutionRunBudgetV1;

pub(crate) const CONTINUATION_BASE_BACKOFF_MS_V1: i64 = 30_000;
pub(crate) const CONTINUATION_MAX_BACKOFF_MS_V1: i64 = 900_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionBudgetPartitionV1 {
    pub(crate) ordinary: EvolutionRunBudgetV1,
    pub(crate) recovery_reserve: EvolutionRunBudgetV1,
}

pub(crate) fn partition_recovery_reserve(
    budget: &EvolutionRunBudgetV1,
) -> EvolutionBudgetPartitionV1 {
    let reserve = EvolutionRunBudgetV1 {
        wall_time_ms: reserve_u64(budget.wall_time_ms),
        evidence_items: reserve_u32(budget.evidence_items),
        seed_groups: reserve_u16(budget.seed_groups),
        assessments: reserve_u16(budget.assessments),
        model_calls: reserve_u16(budget.model_calls),
        notifications: reserve_u16(budget.notifications),
        automatic_mutations: 0,
    };
    EvolutionBudgetPartitionV1 {
        ordinary: EvolutionRunBudgetV1 {
            wall_time_ms: budget.wall_time_ms - reserve.wall_time_ms,
            evidence_items: budget.evidence_items - reserve.evidence_items,
            seed_groups: budget.seed_groups - reserve.seed_groups,
            assessments: budget.assessments - reserve.assessments,
            model_calls: budget.model_calls - reserve.model_calls,
            notifications: budget.notifications - reserve.notifications,
            automatic_mutations: budget.automatic_mutations,
        },
        recovery_reserve: reserve,
    }
}

pub(crate) fn continuation_not_before_ms(now_ms: i64, continuation_attempt: u16) -> Option<i64> {
    if now_ms < 0 || continuation_attempt == 0 {
        return None;
    }
    let shift = u32::from(continuation_attempt.saturating_sub(1)).min(31);
    let multiplier = 1_i64.checked_shl(shift).unwrap_or(i64::MAX);
    let backoff = CONTINUATION_BASE_BACKOFF_MS_V1
        .saturating_mul(multiplier)
        .min(CONTINUATION_MAX_BACKOFF_MS_V1);
    now_ms.checked_add(backoff)
}

fn reserve_u64(value: u64) -> u64 {
    if value == 0 {
        0
    } else {
        (value / 20).max(1)
    }
}

fn reserve_u32(value: u32) -> u32 {
    if value == 0 {
        0
    } else {
        (value / 20).max(1)
    }
}

fn reserve_u16(value: u16) -> u16 {
    if value == 0 {
        0
    } else {
        (value / 20).max(1)
    }
}
