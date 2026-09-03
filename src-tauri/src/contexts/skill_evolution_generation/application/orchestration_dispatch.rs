use std::collections::BTreeSet;

pub(crate) const EVOLUTION_ORCHESTRATION_STAGE_ORDER_V1: [&str; 8] = [
    "recover",
    "maintain_evidence",
    "build_seeds",
    "assess",
    "route_governance",
    "evaluate_auto_apply",
    "project_results",
    "notify",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GenerationDispatchBudgetV1 {
    pub(crate) remaining_items: u16,
    pub(crate) remaining_model_calls: u16,
    pub(crate) remaining_wall_time_ms: u64,
}

pub(crate) struct RouteGovernanceGenerationRequestV1<'a> {
    pub(crate) stage: &'a str,
    pub(crate) idempotency_key: &'a str,
    pub(crate) workspace_id: &'a str,
    pub(crate) assessment_attempt_id: &'a str,
    pub(crate) generation_enabled: bool,
    pub(crate) eligible_for_generation: bool,
    pub(crate) budget: GenerationDispatchBudgetV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationDispatchStatusV1 {
    Dispatched,
    Duplicate,
    SkippedDisabled,
    SkippedIneligible,
    PartialBudget,
    FailedNonBlocking,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteGovernanceGenerationResultV1 {
    pub(crate) status: GenerationDispatchStatusV1,
    pub(crate) generation_job_id: Option<String>,
    pub(crate) safe_reason_code: Option<String>,
    pub(crate) manual_curator_available: bool,
}

pub(crate) trait RouteGovernanceGenerationPort {
    fn dispatch(
        &self,
        request: &RouteGovernanceGenerationRequestV1<'_>,
    ) -> Result<(String, bool), &'static str>;
}

pub(crate) fn dispatch_generation_inside_route_governance(
    port: &dyn RouteGovernanceGenerationPort,
    request: &RouteGovernanceGenerationRequestV1<'_>,
) -> RouteGovernanceGenerationResultV1 {
    let result = |status, job_id, reason| RouteGovernanceGenerationResultV1 {
        status,
        generation_job_id: job_id,
        safe_reason_code: reason,
        manual_curator_available: true,
    };
    if request.stage != "route_governance"
        || request.idempotency_key.trim().is_empty()
        || request.workspace_id.trim().is_empty()
        || request.assessment_attempt_id.trim().is_empty()
    {
        return result(
            GenerationDispatchStatusV1::FailedNonBlocking,
            None,
            Some("generation_dispatch_invalid".into()),
        );
    }
    if !request.generation_enabled {
        return result(GenerationDispatchStatusV1::SkippedDisabled, None, None);
    }
    if !request.eligible_for_generation {
        return result(GenerationDispatchStatusV1::SkippedIneligible, None, None);
    }
    if request.budget.remaining_items == 0
        || request.budget.remaining_model_calls == 0
        || request.budget.remaining_wall_time_ms == 0
    {
        return result(
            GenerationDispatchStatusV1::PartialBudget,
            None,
            Some("generation_dispatch_budget_exhausted".into()),
        );
    }
    match port.dispatch(request) {
        Ok((job_id, duplicate)) if !job_id.trim().is_empty() => result(
            if duplicate {
                GenerationDispatchStatusV1::Duplicate
            } else {
                GenerationDispatchStatusV1::Dispatched
            },
            Some(job_id),
            None,
        ),
        Ok(_) => result(
            GenerationDispatchStatusV1::FailedNonBlocking,
            None,
            Some("generation_dispatch_invalid_receipt".into()),
        ),
        Err(code) => result(
            GenerationDispatchStatusV1::FailedNonBlocking,
            None,
            Some(code.into()),
        ),
    }
}

pub(crate) fn fixed_stage_contract_is_unchanged() -> bool {
    EVOLUTION_ORCHESTRATION_STAGE_ORDER_V1
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .len()
        == 8
        && EVOLUTION_ORCHESTRATION_STAGE_ORDER_V1[4] == "route_governance"
}
