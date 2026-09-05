use crate::contexts::skill_evolution_assessment::api::{
    DraftLessonShapeApi, DraftQualityCheckApi, DraftQualityReviewApi, DraftQualityReviewRequestApi,
};
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use std::{future::Future, pin::Pin};

#[derive(Clone)]
pub(crate) struct AssessmentApiCuratorDraftReviewer {
    api: DraftQualityReviewApi,
}

impl AssessmentApiCuratorDraftReviewer {
    pub(crate) fn new(api: DraftQualityReviewApi) -> Self {
        Self { api }
    }
}

impl CuratorDraftQualityPort for AssessmentApiCuratorDraftReviewer {
    fn review<'a>(
        &'a self,
        input: &'a CuratorDraftQualityInput,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<CuratorDraftQualityReceipt, CuratorDraftQualityError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let result = self
                .api
                .review(&DraftQualityReviewRequestApi {
                    evidence_ids: input.evidence_ids.clone(),
                    original_checks: input.original_checks.iter().map(api_check).collect(),
                    original_lesson_shape: api_shape(&input.original_lesson_shape),
                    draft_lesson_shape: api_shape(&input.lesson_shape),
                    target_skill_id: input.target_skill_id.clone(),
                    target_revision: input.target_revision.clone(),
                    target_matches: true,
                    target_revision_current: true,
                })
                .await
                .map_err(|error| CuratorDraftQualityError {
                    reason_code: error.code().to_string(),
                })?;
            Ok(CuratorDraftQualityReceipt {
                candidate_witness_hash: input.candidate_witness_hash.clone(),
                target_skill_id: input.target_skill_id.clone(),
                target_revision: input.target_revision.clone(),
                draft_hash: input.draft_hash.clone(),
                checks: result.checks.into_iter().map(curator_check).collect(),
                deterministic_approvable: result.deterministic_approvable,
                model_evaluation_allowed: result.model_evaluation_allowed,
                model_consulted: result.model_consulted,
                model_fallback_reason: result.model_fallback_reason,
            })
        })
    }
}

fn api_check(check: &CuratorQualityCheck) -> DraftQualityCheckApi {
    DraftQualityCheckApi {
        code: check.code.clone(),
        result: check_result(check.result).to_string(),
        reason_code: check.reason_code.clone(),
    }
}

fn curator_check(check: DraftQualityCheckApi) -> CuratorQualityCheck {
    CuratorQualityCheck {
        code: check.code,
        result: match check.result.as_str() {
            "pass" => CuratorCheckResult::Pass,
            "fail" => CuratorCheckResult::Fail,
            "review" => CuratorCheckResult::Review,
            _ => CuratorCheckResult::NotApplicable,
        },
        reason_code: check.reason_code,
    }
}

fn api_shape(shape: &CuratorDraftLessonShape) -> DraftLessonShapeApi {
    DraftLessonShapeApi {
        trigger: shape.trigger.clone(),
        required_behavior: shape.required_behavior.clone(),
        prohibited_behavior: shape.prohibited_behavior.clone(),
        verification: shape.verification.clone(),
        environment: shape.environment.clone(),
        content_kinds: shape.content_kinds.clone(),
    }
}

fn check_result(result: CuratorCheckResult) -> &'static str {
    match result {
        CuratorCheckResult::Pass => "pass",
        CuratorCheckResult::Fail => "fail",
        CuratorCheckResult::Review => "review",
        CuratorCheckResult::NotApplicable => "not_applicable",
    }
}
