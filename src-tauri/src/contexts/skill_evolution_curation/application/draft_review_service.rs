use super::*;
use crate::contexts::skill_evolution_curation::domain::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub(crate) struct CuratorDraftReviewService<'a, S, Q> {
    store: &'a mut S,
    reviewer: &'a Q,
}

impl<'a, S, Q> CuratorDraftReviewService<'a, S, Q>
where
    S: CuratorDraftReviewStore,
    Q: CuratorDraftQualityPort,
{
    pub(crate) fn new(store: &'a mut S, reviewer: &'a Q) -> Self {
        Self { store, reviewer }
    }

    pub(crate) async fn review_current(
        &mut self,
        request: CuratorDraftReviewRequest<'_>,
        occurred_at_ms: i64,
    ) -> Result<CuratorDraftAssessment, CuratorDraftReviewError> {
        let binding = self.store.review_binding(request.candidate_id)?;
        validate_request(request, &binding)?;
        let input = quality_input(&binding);
        let receipt = self.reviewer.review(&input).await?;
        let approvable = validate_quality_receipt(&input, &receipt)
            .map_err(CuratorDraftReviewError::InvalidReceipt)?;
        let assessment = build_assessment(&input, receipt, approvable)?;
        self.store
            .persist_draft_assessment(&assessment, occurred_at_ms)?;
        Ok(assessment)
    }
}

fn validate_request(
    request: CuratorDraftReviewRequest<'_>,
    binding: &CuratorDraftReviewBinding,
) -> Result<(), CuratorDraftReviewError> {
    if binding.state != CuratorCandidateState::AwaitingDraft
        || request.expected_candidate_revision != binding.candidate_revision
        || request.expected_draft_revision != binding.draft_revision
    {
        return Err(CuratorDraftReviewError::Conflict);
    }
    Ok(())
}

fn quality_input(binding: &CuratorDraftReviewBinding) -> CuratorDraftQualityInput {
    CuratorDraftQualityInput {
        candidate_id: binding.candidate_id.clone(),
        candidate_revision: binding.candidate_revision,
        candidate_witness_hash: binding.candidate_witness_hash.clone(),
        assessment_attempt_id: binding.assessment_attempt_id.clone(),
        assessment_revision: binding.assessment_revision.clone(),
        target_skill_id: binding.target_skill_id.clone(),
        target_revision: binding.target_revision.clone(),
        draft_id: binding.draft_id.clone(),
        draft_revision: binding.draft_revision,
        draft_hash: binding.draft_hash.clone(),
        evidence_ids: binding.evidence_ids.clone(),
        original_checks: binding.original_checks.clone(),
        original_lesson_shape: binding.original_lesson_shape.clone(),
        lesson_shape: project_draft_lesson(binding),
    }
}

fn build_assessment(
    input: &CuratorDraftQualityInput,
    receipt: CuratorDraftQualityReceipt,
    approvable: bool,
) -> Result<CuratorDraftAssessment, CuratorDraftReviewError> {
    let witness_hash = hash(&AssessmentWitness {
        candidate_witness_hash: &input.candidate_witness_hash,
        candidate_revision: input.candidate_revision,
        target_skill_id: &input.target_skill_id,
        target_revision: &input.target_revision,
        draft_hash: &input.draft_hash,
        draft_revision: input.draft_revision,
        checks: &receipt.checks,
        approvable,
    })?;
    Ok(CuratorDraftAssessment {
        assessment_id: format!("draft-assessment-{witness_hash}"),
        candidate_id: input.candidate_id.clone(),
        candidate_revision: input.candidate_revision,
        draft_id: input.draft_id.clone(),
        draft_revision: input.draft_revision,
        draft_hash: input.draft_hash.clone(),
        candidate_witness_hash: input.candidate_witness_hash.clone(),
        target_skill_id: input.target_skill_id.clone(),
        target_revision: input.target_revision.clone(),
        checks: receipt.checks,
        approvable,
        model_evaluation_allowed: receipt.model_evaluation_allowed,
        model_consulted: receipt.model_consulted,
        model_fallback_reason: receipt.model_fallback_reason,
        witness_hash,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AssessmentWitness<'a> {
    candidate_witness_hash: &'a str,
    candidate_revision: u64,
    target_skill_id: &'a str,
    target_revision: &'a str,
    draft_hash: &'a str,
    draft_revision: u64,
    checks: &'a [CuratorQualityCheck],
    approvable: bool,
}

fn hash(value: &impl Serialize) -> Result<String, CuratorDraftReviewError> {
    let bytes = serde_json::to_vec(value).map_err(|_| CuratorDraftReviewError::InvalidInput)?;
    let digest = Sha256::digest(bytes);
    Ok(format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum CuratorDraftReviewError {
    #[error("curator draft review changed concurrently")]
    Conflict,
    #[error("curator draft review input is invalid")]
    InvalidInput,
    #[error("curator draft reviewer returned an invalid receipt: {0}")]
    InvalidReceipt(&'static str),
    #[error(transparent)]
    Store(#[from] CuratorDraftReviewStoreError),
    #[error(transparent)]
    Quality(#[from] CuratorDraftQualityError),
}
