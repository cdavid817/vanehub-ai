use crate::contexts::skill_evolution_orchestration::domain::{
    EvolutionRunBudgetV1, EvolutionRunUsageV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionBudgetResourceV1 {
    WallTime,
    EvidenceItems,
    SeedGroups,
    Assessments,
    ModelCalls,
    Notifications,
    AutomaticMutations,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EvolutionBudgetDecisionV1 {
    Accepted { usage: EvolutionRunUsageV1 },
    Exhausted { resource: EvolutionBudgetResourceV1 },
    CounterOverflow { resource: EvolutionBudgetResourceV1 },
}

pub(crate) fn apply_committed_usage(
    budget: &EvolutionRunBudgetV1,
    current: &EvolutionRunUsageV1,
    committed_delta: &EvolutionRunUsageV1,
) -> EvolutionBudgetDecisionV1 {
    let elapsed_ms = match current.elapsed_ms.checked_add(committed_delta.elapsed_ms) {
        Some(value) => value,
        None => return overflow(EvolutionBudgetResourceV1::WallTime),
    };
    if elapsed_ms > budget.wall_time_ms {
        return exhausted(EvolutionBudgetResourceV1::WallTime);
    }
    let evidence_items = match current
        .evidence_items
        .checked_add(committed_delta.evidence_items)
    {
        Some(value) => value,
        None => return overflow(EvolutionBudgetResourceV1::EvidenceItems),
    };
    if evidence_items > budget.evidence_items {
        return exhausted(EvolutionBudgetResourceV1::EvidenceItems);
    }
    let seed_groups = match current.seed_groups.checked_add(committed_delta.seed_groups) {
        Some(value) => value,
        None => return overflow(EvolutionBudgetResourceV1::SeedGroups),
    };
    if seed_groups > budget.seed_groups {
        return exhausted(EvolutionBudgetResourceV1::SeedGroups);
    }
    let assessments = match current.assessments.checked_add(committed_delta.assessments) {
        Some(value) => value,
        None => return overflow(EvolutionBudgetResourceV1::Assessments),
    };
    if assessments > budget.assessments {
        return exhausted(EvolutionBudgetResourceV1::Assessments);
    }
    let model_calls = match current.model_calls.checked_add(committed_delta.model_calls) {
        Some(value) => value,
        None => return overflow(EvolutionBudgetResourceV1::ModelCalls),
    };
    if model_calls > budget.model_calls {
        return exhausted(EvolutionBudgetResourceV1::ModelCalls);
    }
    let notifications = match current
        .notifications
        .checked_add(committed_delta.notifications)
    {
        Some(value) => value,
        None => return overflow(EvolutionBudgetResourceV1::Notifications),
    };
    if notifications > budget.notifications {
        return exhausted(EvolutionBudgetResourceV1::Notifications);
    }
    let automatic_mutations = match current
        .automatic_mutations
        .checked_add(committed_delta.automatic_mutations)
    {
        Some(value) => value,
        None => return overflow(EvolutionBudgetResourceV1::AutomaticMutations),
    };
    if automatic_mutations > budget.automatic_mutations {
        return exhausted(EvolutionBudgetResourceV1::AutomaticMutations);
    }
    EvolutionBudgetDecisionV1::Accepted {
        usage: EvolutionRunUsageV1 {
            elapsed_ms,
            evidence_items,
            seed_groups,
            assessments,
            model_calls,
            notifications,
            automatic_mutations,
        },
    }
}

fn exhausted(resource: EvolutionBudgetResourceV1) -> EvolutionBudgetDecisionV1 {
    EvolutionBudgetDecisionV1::Exhausted { resource }
}

fn overflow(resource: EvolutionBudgetResourceV1) -> EvolutionBudgetDecisionV1 {
    EvolutionBudgetDecisionV1::CounterOverflow { resource }
}
