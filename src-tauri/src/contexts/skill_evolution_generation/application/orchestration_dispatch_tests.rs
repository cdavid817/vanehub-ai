use std::{cell::RefCell, collections::BTreeMap};

use super::{
    dispatch_generation_inside_route_governance, fixed_stage_contract_is_unchanged,
    GenerationDispatchBudgetV1, GenerationDispatchStatusV1, RouteGovernanceGenerationPort,
    RouteGovernanceGenerationRequestV1,
};

#[derive(Default)]
struct Dispatcher {
    jobs: RefCell<BTreeMap<String, String>>,
    failure: Option<&'static str>,
}

impl RouteGovernanceGenerationPort for Dispatcher {
    fn dispatch(
        &self,
        request: &RouteGovernanceGenerationRequestV1<'_>,
    ) -> Result<(String, bool), &'static str> {
        if let Some(code) = self.failure {
            return Err(code);
        }
        let mut jobs = self.jobs.borrow_mut();
        if let Some(job_id) = jobs.get(request.idempotency_key) {
            return Ok((job_id.clone(), true));
        }
        let job_id = format!("generation-job-{}", jobs.len() + 1);
        jobs.insert(request.idempotency_key.into(), job_id.clone());
        Ok((job_id, false))
    }
}

#[test]
fn generation_dispatch_is_only_an_optional_route_governance_branch() {
    assert!(fixed_stage_contract_is_unchanged());
    let dispatcher = Dispatcher::default();
    let request = request();
    let first = dispatch_generation_inside_route_governance(&dispatcher, &request);
    let duplicate = dispatch_generation_inside_route_governance(&dispatcher, &request);
    assert_eq!(first.status, GenerationDispatchStatusV1::Dispatched);
    assert_eq!(duplicate.status, GenerationDispatchStatusV1::Duplicate);
    assert_eq!(first.generation_job_id, duplicate.generation_job_id);
}

#[test]
fn disabled_ineligible_and_budget_limited_dispatches_preserve_manual_curation() {
    let dispatcher = Dispatcher::default();
    let mut request = request();
    request.generation_enabled = false;
    let disabled = dispatch_generation_inside_route_governance(&dispatcher, &request);
    assert_eq!(disabled.status, GenerationDispatchStatusV1::SkippedDisabled);
    assert!(disabled.manual_curator_available);

    request.generation_enabled = true;
    request.eligible_for_generation = false;
    let ineligible = dispatch_generation_inside_route_governance(&dispatcher, &request);
    assert_eq!(
        ineligible.status,
        GenerationDispatchStatusV1::SkippedIneligible
    );
    assert!(ineligible.manual_curator_available);

    request.eligible_for_generation = true;
    request.budget.remaining_model_calls = 0;
    let limited = dispatch_generation_inside_route_governance(&dispatcher, &request);
    assert_eq!(limited.status, GenerationDispatchStatusV1::PartialBudget);
    assert!(limited.manual_curator_available);
    assert!(dispatcher.jobs.borrow().is_empty());
}

#[test]
fn generation_failure_is_fail_open_for_the_parent_orchestration_stage() {
    let dispatcher = Dispatcher {
        jobs: RefCell::new(BTreeMap::new()),
        failure: Some("generation_database_unavailable"),
    };
    let result = dispatch_generation_inside_route_governance(&dispatcher, &request());
    assert_eq!(result.status, GenerationDispatchStatusV1::FailedNonBlocking);
    assert_eq!(
        result.safe_reason_code.as_deref(),
        Some("generation_database_unavailable")
    );
    assert!(result.manual_curator_available);
}

#[test]
fn every_generation_failure_class_preserves_the_existing_stage_contract() {
    let failures = [
        "generation_provider_unavailable",
        "generation_database_unavailable",
        "generation_cancelled",
        "generation_stale_input",
        "generation_curator_unavailable",
        "generation_quarantine_failed",
        "generation_rollback_required",
    ];
    for failure in failures {
        let dispatcher = Dispatcher {
            jobs: RefCell::new(BTreeMap::new()),
            failure: Some(failure),
        };
        let result = dispatch_generation_inside_route_governance(&dispatcher, &request());
        assert_eq!(result.status, GenerationDispatchStatusV1::FailedNonBlocking);
        assert_eq!(result.safe_reason_code.as_deref(), Some(failure));
        assert!(result.manual_curator_available);
        assert!(dispatcher.jobs.borrow().is_empty());
        assert!(fixed_stage_contract_is_unchanged());
    }
}

fn request() -> RouteGovernanceGenerationRequestV1<'static> {
    RouteGovernanceGenerationRequestV1 {
        stage: "route_governance",
        idempotency_key: "run:stage:item",
        workspace_id: "workspace-one",
        assessment_attempt_id: "assessment-one",
        generation_enabled: true,
        eligible_for_generation: true,
        budget: GenerationDispatchBudgetV1 {
            remaining_items: 1,
            remaining_model_calls: 3,
            remaining_wall_time_ms: 180_000,
        },
    }
}
