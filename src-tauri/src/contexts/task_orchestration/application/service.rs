use super::{
    build_planner_prompt, parse_planner_response, GeneratePlanDraftRequest, PlanGenerationPort,
    PlanGenerationRequest,
};
use crate::contexts::task_orchestration::domain::{validate_plan_graph, PlanDraft};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum PlanApplicationError {
    #[error("Plan validation failed: {0}")]
    Validation(String),
    #[error("Plan storage failed: {0}")]
    Storage(String),
    #[error("Plan was not found")]
    NotFound,
    #[error("Plan state changed before the operation completed")]
    Conflict,
}

pub(crate) trait PlanRepositoryPort: Send + Sync {
    fn record_generation_failure(
        &self,
        plan_id: Option<&str>,
        requested_version: u32,
        failure_class: &str,
        safe_action: &str,
    ) -> Result<(), PlanApplicationError>;
    fn save_draft(&self, draft: &PlanDraft) -> Result<(), PlanApplicationError>;
    fn list_plan_versions(&self, plan_id: &str) -> Result<Vec<PlanDraft>, PlanApplicationError>;
    fn delete_draft_plan(&self, plan_id: &str) -> Result<(), PlanApplicationError>;
    fn find_latest_draft(&self, plan_id: &str) -> Result<Option<PlanDraft>, PlanApplicationError>;
    fn approve_latest(&self, plan_id: &str, now: &str) -> Result<String, PlanApplicationError>;
}

#[derive(Clone)]
pub(crate) struct PlanApplicationService {
    repository: Arc<dyn PlanRepositoryPort>,
    generator: Arc<dyn PlanGenerationPort>,
}

impl PlanApplicationService {
    pub(crate) fn new(
        repository: Arc<dyn PlanRepositoryPort>,
        generator: Arc<dyn PlanGenerationPort>,
    ) -> Self {
        Self {
            repository,
            generator,
        }
    }

    pub(crate) fn generate_draft(
        &self,
        request: &GeneratePlanDraftRequest,
    ) -> Result<PlanDraft, PlanApplicationError> {
        if request.goal.trim().is_empty()
            || request.project_path.trim().is_empty()
            || request.base_ref.trim().is_empty()
            || request.version == 0
        {
            return Err(PlanApplicationError::Validation(
                "goal, project path, base ref, and a positive version are required".to_string(),
            ));
        }
        let response = self
            .generator
            .generate(&PlanGenerationRequest {
                prompt: build_planner_prompt(request),
            })
            .inspect_err(|_| {
                self.record_generation_failure(
                    request,
                    "planner_generation_failed",
                    "Check the active OnePiece Profile and retry Plan generation.",
                );
            })?;
        let draft = parse_planner_response(request, response).inspect_err(|_| {
            self.record_generation_failure(
                request,
                "planner_output_invalid",
                "Retry generation or edit the Plan draft structure manually.",
            );
        })?;
        self.save_draft(&draft).inspect_err(|_| {
            self.record_generation_failure(
                request,
                "planner_draft_rejected",
                "Review the generated task graph and retry Plan generation.",
            );
        })
    }

    pub(crate) fn save_draft(&self, draft: &PlanDraft) -> Result<PlanDraft, PlanApplicationError> {
        validate_plan_graph(draft)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        self.repository.save_draft(draft)?;
        Ok(draft.clone())
    }

    pub(crate) fn validate_draft(&self, draft: &PlanDraft) -> Result<(), PlanApplicationError> {
        validate_plan_graph(draft)
            .map(|_| ())
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))
    }

    pub(crate) fn list_versions(
        &self,
        plan_id: &str,
    ) -> Result<Vec<PlanDraft>, PlanApplicationError> {
        self.repository.list_plan_versions(plan_id)
    }

    pub(crate) fn delete_draft(&self, plan_id: &str) -> Result<(), PlanApplicationError> {
        self.repository.delete_draft_plan(plan_id)
    }

    pub(crate) fn find_draft(
        &self,
        plan_id: &str,
    ) -> Result<Option<PlanDraft>, PlanApplicationError> {
        self.repository.find_latest_draft(plan_id)
    }

    pub(crate) fn approve(&self, plan_id: &str, now: &str) -> Result<String, PlanApplicationError> {
        self.repository.approve_latest(plan_id, now)
    }

    fn record_generation_failure(
        &self,
        request: &GeneratePlanDraftRequest,
        failure_class: &str,
        safe_action: &str,
    ) {
        let _ = self.repository.record_generation_failure(
            request.plan_id.as_deref(),
            request.version,
            failure_class,
            safe_action,
        );
    }
}
