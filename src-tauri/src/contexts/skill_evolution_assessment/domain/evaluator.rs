use super::{
    AssessmentRisk, AssessmentRoute, EvaluatorFallbackReason, LessonShape, QualityCheck,
    RankedTarget,
};
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};
use tokio::time::{timeout, Duration};

pub(crate) const EVALUATOR_POLICY_V1: &str = "structured-evaluator-v1";
pub(crate) const DISCLOSURE_VERSION_V1: &str = "assessment-disclosure-v1";
pub(crate) const MAX_EVALUATOR_CALLS_V1: u8 = 2;
pub(crate) const MAX_STAGE_MILLIS_V1: u64 = 15_000;
pub(crate) const MAX_CANDIDATES_V1: usize = 5;
pub(crate) const MAX_RESPONSE_BYTES_V1: usize = 8_192;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ModelEvaluationConsent {
    pub(crate) policy_version: String,
    pub(crate) disclosure_version: String,
    pub(crate) enabled: bool,
    pub(crate) changed_at_ms: i64,
    pub(crate) local_actor: String,
}

impl Default for ModelEvaluationConsent {
    fn default() -> Self {
        Self {
            policy_version: EVALUATOR_POLICY_V1.to_string(),
            disclosure_version: DISCLOSURE_VERSION_V1.to_string(),
            enabled: false,
            changed_at_ms: 0,
            local_actor: "system_default".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaluatorDisclosure {
    pub(crate) version: String,
    pub(crate) outbound_classes: Vec<String>,
    pub(crate) prohibited_classes: Vec<String>,
}

pub(crate) fn evaluator_disclosure_v1() -> EvaluatorDisclosure {
    EvaluatorDisclosure {
        version: DISCLOSURE_VERSION_V1.to_string(),
        outbound_classes: vec![
            "sanitized_seed_summary".to_string(),
            "evidence_categories_and_ids".to_string(),
            "bounded_skill_metadata".to_string(),
            "score_and_check_results".to_string(),
            "bounded_rationales".to_string(),
        ],
        prohibited_classes: vec![
            "raw_prompts".to_string(),
            "tool_arguments".to_string(),
            "terminal_output".to_string(),
            "file_content".to_string(),
            "credentials".to_string(),
            "complete_skill_instructions".to_string(),
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EvaluatorTransportError {
    ProviderUnavailable,
    Timeout,
    RateLimited,
    ProviderFailure,
}

pub(crate) trait StructuredEvaluator: Send + Sync {
    fn evaluate<'a>(
        &'a self,
        stage: EvaluationStage,
        sanitized_json: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<String, EvaluatorTransportError>> + Send + 'a>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvaluationStage {
    TargetConsultation,
    QualityJudge,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AssessmentEvaluationBudget {
    calls: u8,
    target_consulted: bool,
    quality_judged: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EvaluationOutcome<T> {
    pub(crate) value: Option<T>,
    pub(crate) fallback: Option<EvaluatorFallbackReason>,
}

impl<T> EvaluationOutcome<T> {
    fn success(value: T) -> Self {
        Self {
            value: Some(value),
            fallback: None,
        }
    }

    fn fallback(reason: EvaluatorFallbackReason) -> Self {
        Self {
            value: None,
            fallback: Some(reason),
        }
    }
}

impl AssessmentEvaluationBudget {
    pub(crate) fn claim(&mut self, stage: EvaluationStage) -> bool {
        if self.calls >= MAX_EVALUATOR_CALLS_V1 {
            return false;
        }
        let already_called = match stage {
            EvaluationStage::TargetConsultation => &mut self.target_consulted,
            EvaluationStage::QualityJudge => &mut self.quality_judged,
        };
        if *already_called {
            return false;
        }
        *already_called = true;
        self.calls += 1;
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetConsultationRequest {
    pub(crate) policy_version: String,
    pub(crate) candidates: Vec<RankedTarget>,
    pub(crate) evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TargetConsultationResponse {
    pub(crate) selected_target_id: String,
    pub(crate) confidence: f32,
    pub(crate) cited_evidence_ids: Vec<String>,
    pub(crate) rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QualityJudgeRequest {
    pub(crate) policy_version: String,
    pub(crate) checks: Vec<QualityCheck>,
    pub(crate) evidence_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sanitized_draft_projection: Option<LessonShape>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct QualityJudgeResponse {
    pub(crate) support: u8,
    pub(crate) specificity: u8,
    pub(crate) durability: u8,
    pub(crate) actionability: u8,
    pub(crate) contradiction: bool,
    pub(crate) risk: AssessmentRisk,
    pub(crate) recommended_route: AssessmentRoute,
    pub(crate) cited_evidence_ids: Vec<String>,
    pub(crate) rationale: String,
}

pub(crate) fn validate_target_consultation(
    request: &TargetConsultationRequest,
    raw: &str,
) -> Result<TargetConsultationResponse, EvaluatorFallbackReason> {
    if request.candidates.is_empty() || request.candidates.len() > MAX_CANDIDATES_V1 {
        return Err(EvaluatorFallbackReason::InvalidSchema);
    }
    let response: TargetConsultationResponse = parse_bounded(raw)?;
    if !response.confidence.is_finite() || !(0.0..=1.0).contains(&response.confidence) {
        return Err(EvaluatorFallbackReason::InvalidSchema);
    }
    if !request
        .candidates
        .iter()
        .any(|candidate| candidate.witness.skill_id == response.selected_target_id)
    {
        return Err(EvaluatorFallbackReason::InventedTarget);
    }
    validate_citations(&request.evidence_ids, &response.cited_evidence_ids)?;
    validate_rationale(&response.rationale)?;
    Ok(response)
}

pub(crate) fn validate_quality_judge(
    request: &QualityJudgeRequest,
    raw: &str,
) -> Result<QualityJudgeResponse, EvaluatorFallbackReason> {
    let response: QualityJudgeResponse = parse_bounded(raw)?;
    if [
        response.support,
        response.specificity,
        response.durability,
        response.actionability,
    ]
    .iter()
    .any(|rating| *rating > 100)
    {
        return Err(EvaluatorFallbackReason::InvalidSchema);
    }
    validate_citations(&request.evidence_ids, &response.cited_evidence_ids)?;
    validate_rationale(&response.rationale)?;
    Ok(response)
}

pub(crate) async fn consult_target(
    consent: &ModelEvaluationConsent,
    budget: &mut AssessmentEvaluationBudget,
    evaluator: &dyn StructuredEvaluator,
    request: &TargetConsultationRequest,
) -> EvaluationOutcome<TargetConsultationResponse> {
    consult_target_with_deadline(consent, budget, evaluator, request, MAX_STAGE_MILLIS_V1).await
}

pub(crate) async fn judge_quality(
    consent: &ModelEvaluationConsent,
    budget: &mut AssessmentEvaluationBudget,
    evaluator: &dyn StructuredEvaluator,
    request: &QualityJudgeRequest,
) -> EvaluationOutcome<QualityJudgeResponse> {
    if !consent.enabled {
        return EvaluationOutcome::fallback(EvaluatorFallbackReason::DisabledConsent);
    }
    if !budget.claim(EvaluationStage::QualityJudge) {
        return EvaluationOutcome::fallback(EvaluatorFallbackReason::ProviderFailure);
    }
    let serialized = match serde_json::to_string(request) {
        Ok(serialized) => serialized,
        Err(_) => return EvaluationOutcome::fallback(EvaluatorFallbackReason::InvalidSchema),
    };
    match timeout(
        Duration::from_millis(MAX_STAGE_MILLIS_V1),
        evaluator.evaluate(EvaluationStage::QualityJudge, &serialized),
    )
    .await
    {
        Err(_) => EvaluationOutcome::fallback(EvaluatorFallbackReason::Timeout),
        Ok(Err(error)) => EvaluationOutcome::fallback(map_transport_error(error)),
        Ok(Ok(raw)) => match validate_quality_judge(request, &raw) {
            Ok(response) => EvaluationOutcome::success(response),
            Err(reason) => EvaluationOutcome::fallback(reason),
        },
    }
}

pub(crate) async fn consult_target_with_deadline(
    consent: &ModelEvaluationConsent,
    budget: &mut AssessmentEvaluationBudget,
    evaluator: &dyn StructuredEvaluator,
    request: &TargetConsultationRequest,
    deadline_ms: u64,
) -> EvaluationOutcome<TargetConsultationResponse> {
    if !consent.enabled {
        return EvaluationOutcome::fallback(EvaluatorFallbackReason::DisabledConsent);
    }
    if !budget.claim(EvaluationStage::TargetConsultation) {
        return EvaluationOutcome::fallback(EvaluatorFallbackReason::ProviderFailure);
    }
    let serialized = match serde_json::to_string(request) {
        Ok(serialized) => serialized,
        Err(_) => return EvaluationOutcome::fallback(EvaluatorFallbackReason::InvalidSchema),
    };
    match timeout(
        Duration::from_millis(deadline_ms),
        evaluator.evaluate(EvaluationStage::TargetConsultation, &serialized),
    )
    .await
    {
        Err(_) => EvaluationOutcome::fallback(EvaluatorFallbackReason::Timeout),
        Ok(Err(error)) => EvaluationOutcome::fallback(map_transport_error(error)),
        Ok(Ok(raw)) => match validate_target_consultation(request, &raw) {
            Ok(response) => EvaluationOutcome::success(response),
            Err(reason) => EvaluationOutcome::fallback(reason),
        },
    }
}

pub(crate) fn stricter_model_route(
    deterministic: AssessmentRoute,
    advised: AssessmentRoute,
) -> AssessmentRoute {
    if route_strictness(advised) > route_strictness(deterministic) {
        advised
    } else {
        deterministic
    }
}

pub(crate) fn map_transport_error(error: EvaluatorTransportError) -> EvaluatorFallbackReason {
    match error {
        EvaluatorTransportError::ProviderUnavailable => {
            EvaluatorFallbackReason::ProviderUnavailable
        }
        EvaluatorTransportError::Timeout => EvaluatorFallbackReason::Timeout,
        EvaluatorTransportError::RateLimited => EvaluatorFallbackReason::RateLimited,
        EvaluatorTransportError::ProviderFailure => EvaluatorFallbackReason::ProviderFailure,
    }
}

fn parse_bounded<T: for<'de> Deserialize<'de>>(raw: &str) -> Result<T, EvaluatorFallbackReason> {
    if raw.len() > MAX_RESPONSE_BYTES_V1 {
        return Err(EvaluatorFallbackReason::InvalidSchema);
    }
    serde_json::from_str(raw).map_err(|_| EvaluatorFallbackReason::InvalidSchema)
}

fn validate_citations(
    supplied: &[String],
    cited: &[String],
) -> Result<(), EvaluatorFallbackReason> {
    if cited.is_empty() || cited.iter().any(|id| !supplied.contains(id)) {
        Err(EvaluatorFallbackReason::MissingCitation)
    } else {
        Ok(())
    }
}

fn validate_rationale(rationale: &str) -> Result<(), EvaluatorFallbackReason> {
    let normalized = rationale.to_ascii_lowercase();
    let unsafe_fragment = [
        "<system",
        "system instruction",
        "ignore previous",
        "ignore checks",
        "call tool",
        "do not call tools",
        "resource://",
        "return exactly one json",
        "evaluate only the supplied",
    ]
    .iter()
    .any(|fragment| normalized.contains(fragment));
    if rationale.len() <= 512 && !unsafe_fragment {
        Ok(())
    } else {
        Err(EvaluatorFallbackReason::InvalidSchema)
    }
}

fn route_strictness(route: AssessmentRoute) -> u8 {
    match route {
        AssessmentRoute::Advance => 0,
        AssessmentRoute::RecordMemoryOnly => 1,
        AssessmentRoute::MergeDuplicate => 2,
        AssessmentRoute::NeedsHumanReview => 3,
        AssessmentRoute::Drop => 4,
    }
}
