use super::AssessmentApiError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::skill_evolution_assessment::domain::*;
use crate::contexts::skill_evolution_assessment::infrastructure::{
    ConfiguredStructuredEvaluator, SqliteAssessmentPolicyRepository,
};
use crate::platform::database::NativeDatabase;
use serde::{Deserialize, Serialize};

mod deterministic;
use deterministic::{deterministic_review, validate_request};

#[cfg(test)]
mod deterministic_tests;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DraftQualityCheckApi {
    pub(crate) code: String,
    pub(crate) result: String,
    pub(crate) reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DraftLessonShapeApi {
    pub(crate) trigger: String,
    pub(crate) required_behavior: String,
    pub(crate) prohibited_behavior: String,
    pub(crate) verification: String,
    pub(crate) environment: String,
    pub(crate) content_kinds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DraftQualityReviewRequestApi {
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) original_checks: Vec<DraftQualityCheckApi>,
    pub(crate) original_lesson_shape: DraftLessonShapeApi,
    pub(crate) draft_lesson_shape: DraftLessonShapeApi,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) target_matches: bool,
    pub(crate) target_revision_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DraftQualityReviewResultApi {
    pub(crate) checks: Vec<DraftQualityCheckApi>,
    pub(crate) deterministic_approvable: bool,
    pub(crate) model_evaluation_allowed: bool,
    pub(crate) model_consulted: bool,
    pub(crate) model_fallback_reason: Option<String>,
}

#[derive(Clone)]
pub(crate) struct DraftQualityReviewApi {
    policy: SqliteAssessmentPolicyRepository,
    evaluator: Option<ConfiguredStructuredEvaluator>,
}

impl DraftQualityReviewApi {
    pub(crate) fn deterministic(database: NativeDatabase) -> Self {
        Self {
            policy: SqliteAssessmentPolicyRepository::new(database),
            evaluator: None,
        }
    }

    pub(crate) fn with_model(database: NativeDatabase, runtime: AgentRuntimeApi) -> Self {
        Self {
            policy: SqliteAssessmentPolicyRepository::new(database),
            evaluator: Some(ConfiguredStructuredEvaluator::new(runtime)),
        }
    }

    pub(crate) async fn review(
        &self,
        request: &DraftQualityReviewRequestApi,
    ) -> Result<DraftQualityReviewResultApi, AssessmentApiError> {
        validate_request(request)?;
        let mut assessment = deterministic_review(request)?;
        let consent = self
            .policy
            .load()
            .map_err(|_| AssessmentApiError::Storage)?;
        let model_allowed = assessment.model_evaluation_allowed && consent.enabled;
        let (consulted, fallback, model_approvable) = if model_allowed {
            self.optional_model_review(&consent, request, &assessment.checks)
                .await
        } else {
            (false, Some("disabled_consent".to_string()), true)
        };
        assessment.model_evaluation_allowed = model_allowed;
        assessment.model_consulted = consulted;
        assessment.model_fallback_reason = fallback;
        assessment.deterministic_approvable &= model_approvable;
        Ok(assessment)
    }

    async fn optional_model_review(
        &self,
        consent: &ModelEvaluationConsent,
        request: &DraftQualityReviewRequestApi,
        checks: &[DraftQualityCheckApi],
    ) -> (bool, Option<String>, bool) {
        let Some(evaluator) = &self.evaluator else {
            return (false, Some("provider_unavailable".to_string()), true);
        };
        let quality_checks = checks.iter().map(internal_check).collect::<Vec<_>>();
        let judge_request = QualityJudgeRequest {
            policy_version: EVALUATOR_POLICY_V1.to_string(),
            checks: quality_checks,
            evidence_ids: request.evidence_ids.clone(),
            sanitized_draft_projection: Some(internal_shape(&request.draft_lesson_shape)),
        };
        let outcome = judge_quality(
            consent,
            &mut AssessmentEvaluationBudget::default(),
            evaluator,
            &judge_request,
        )
        .await;
        match outcome.value {
            Some(value) => (
                true,
                None,
                matches!(
                    value.recommended_route,
                    AssessmentRoute::Advance | AssessmentRoute::NeedsHumanReview
                ) && !value.contradiction,
            ),
            None => (
                false,
                outcome
                    .fallback
                    .map(|reason| fallback_name(reason).to_string()),
                true,
            ),
        }
    }
}

pub(super) fn internal_shape(shape: &DraftLessonShapeApi) -> LessonShape {
    LessonShape {
        trigger: Some(shape.trigger.clone()),
        required_behavior: Some(shape.required_behavior.clone()),
        prohibited_behavior: Some(shape.prohibited_behavior.clone()),
        verification: Some(shape.verification.clone()),
        environment: Some(shape.environment.clone()),
        content_kinds: shape.content_kinds.clone(),
    }
}

pub(super) fn api_check(check: &QualityCheck) -> DraftQualityCheckApi {
    DraftQualityCheckApi {
        code: check_kind(check.kind).to_string(),
        result: check_result(check.result).to_string(),
        reason_code: check.reason_code.clone(),
    }
}

fn internal_check(check: &DraftQualityCheckApi) -> QualityCheck {
    QualityCheck {
        kind: parse_kind(&check.code),
        result: parse_result(&check.result),
        severity: AssessmentRisk::Medium,
        reason_code: check.reason_code.clone(),
        evidence_ids: Vec::new(),
        route_constraints: Vec::new(),
    }
}

fn check_kind(kind: QualityCheckKind) -> &'static str {
    match kind {
        QualityCheckKind::PrivacyResidue => "privacy_residue",
        QualityCheckKind::EvidenceSufficiency => "evidence_sufficiency",
        QualityCheckKind::DuplicateKnowledge => "duplicate_knowledge",
        QualityCheckKind::TransientIncident => "transient_incident",
        QualityCheckKind::GuidanceSpecificity => "guidance_specificity",
        QualityCheckKind::EvidenceConsistency => "evidence_consistency",
        QualityCheckKind::TargetCompatibility => "target_compatibility",
        QualityCheckKind::ExecutableContentRisk => "executable_content_risk",
        QualityCheckKind::TargetLifecycleMutability => "target_lifecycle_mutability",
    }
}

fn check_result(result: QualityCheckResult) -> &'static str {
    match result {
        QualityCheckResult::Pass => "pass",
        QualityCheckResult::Fail => "fail",
        QualityCheckResult::Review => "review",
        QualityCheckResult::NotApplicable => "not_applicable",
    }
}

fn parse_kind(code: &str) -> QualityCheckKind {
    match code {
        "evidence_sufficiency" => QualityCheckKind::EvidenceSufficiency,
        "duplicate_knowledge" => QualityCheckKind::DuplicateKnowledge,
        "transient_incident" => QualityCheckKind::TransientIncident,
        "guidance_specificity" => QualityCheckKind::GuidanceSpecificity,
        "evidence_consistency" => QualityCheckKind::EvidenceConsistency,
        "target_compatibility" => QualityCheckKind::TargetCompatibility,
        "executable_content_risk" => QualityCheckKind::ExecutableContentRisk,
        "target_lifecycle_mutability" => QualityCheckKind::TargetLifecycleMutability,
        _ => QualityCheckKind::PrivacyResidue,
    }
}

fn parse_result(result: &str) -> QualityCheckResult {
    match result {
        "pass" => QualityCheckResult::Pass,
        "fail" => QualityCheckResult::Fail,
        "review" => QualityCheckResult::Review,
        _ => QualityCheckResult::NotApplicable,
    }
}

fn fallback_name(reason: EvaluatorFallbackReason) -> &'static str {
    match reason {
        EvaluatorFallbackReason::DisabledConsent => "disabled_consent",
        EvaluatorFallbackReason::ProviderUnavailable => "provider_unavailable",
        EvaluatorFallbackReason::Timeout => "timeout",
        EvaluatorFallbackReason::RateLimited => "rate_limited",
        EvaluatorFallbackReason::InvalidSchema => "invalid_schema",
        EvaluatorFallbackReason::InventedTarget => "invented_target",
        EvaluatorFallbackReason::MissingCitation => "missing_citation",
        EvaluatorFallbackReason::ProviderFailure => "provider_failure",
    }
}
