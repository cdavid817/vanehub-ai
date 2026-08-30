use async_trait::async_trait;

use crate::contexts::{
    skill_evolution_assessment::api::{
        DraftLessonShapeApi, DraftQualityCheckApi, DraftQualityReviewApi,
        DraftQualityReviewRequestApi,
    },
    skill_evolution_orchestration::{
        application::{
            AutomaticCorrectionDraftRequestV1, AutomaticDraftPipelineError,
            AutomaticDraftQualityPort, CorrectionLessonShapeV1, DraftQualityCheckV1,
            DraftQualityReceiptV1,
        },
        domain::ProducedCorrectionDraftV1,
    },
};

pub(crate) struct AssessmentAutomaticDraftQuality {
    reviewer: DraftQualityReviewApi,
}

impl AssessmentAutomaticDraftQuality {
    pub(crate) fn new(reviewer: DraftQualityReviewApi) -> Self {
        Self { reviewer }
    }
}

#[async_trait]
impl AutomaticDraftQualityPort for AssessmentAutomaticDraftQuality {
    async fn review(
        &self,
        request: &AutomaticCorrectionDraftRequestV1,
        draft: &ProducedCorrectionDraftV1,
    ) -> Result<DraftQualityReceiptV1, AutomaticDraftPipelineError> {
        let result = self
            .reviewer
            .review(&DraftQualityReviewRequestApi {
                evidence_ids: request.evidence_ids.clone(),
                original_checks: request.original_checks.iter().map(api_check).collect(),
                original_lesson_shape: api_shape(&request.original_lesson_shape),
                draft_lesson_shape: DraftLessonShapeApi {
                    trigger: draft.trigger.clone(),
                    required_behavior: draft.guidance.clone(),
                    prohibited_behavior: request.original_lesson_shape.prohibited_behavior.clone(),
                    verification: draft.verification.clone(),
                    environment: request.original_lesson_shape.environment.clone(),
                    content_kinds: vec!["guidance".into()],
                },
                target_skill_id: request.target_skill_id.clone(),
                target_revision: request.target_revision.clone(),
                target_matches: true,
                target_revision_current: true,
            })
            .await
            .map_err(|_| AutomaticDraftPipelineError::QualityRejected)?;
        Ok(DraftQualityReceiptV1 {
            checks: result
                .checks
                .into_iter()
                .map(|check| DraftQualityCheckV1 {
                    code: check.code,
                    result: check.result,
                    reason_code: check.reason_code,
                })
                .collect(),
            deterministic_approvable: result.deterministic_approvable,
        })
    }
}

fn api_shape(value: &CorrectionLessonShapeV1) -> DraftLessonShapeApi {
    DraftLessonShapeApi {
        trigger: value.trigger.clone(),
        required_behavior: value.required_behavior.clone(),
        prohibited_behavior: value.prohibited_behavior.clone(),
        verification: value.verification.clone(),
        environment: value.environment.clone(),
        content_kinds: value.content_kinds.clone(),
    }
}

fn api_check(value: &DraftQualityCheckV1) -> DraftQualityCheckApi {
    DraftQualityCheckApi {
        code: value.code.clone(),
        result: value.result.clone(),
        reason_code: value.reason_code.clone(),
    }
}
