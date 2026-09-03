use async_trait::async_trait;

use crate::contexts::skill_evolution_orchestration::domain::{
    produce_authorized_correction_draft, AuthorizedCorrectionDraftInputV1, CorrectionDraftError,
    DeterministicCorrectionDraftV1, ProducedCorrectionDraftV1, AUTO_DRAFT_QUALITY_CHECK_ORDER_V1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthorizedCorrectionSourceV1 {
    pub(crate) authorization_id: String,
    pub(crate) sanitized_guidance: String,
    pub(crate) sanitizer_version: u16,
    pub(crate) authorization_witness_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CorrectionLessonShapeV1 {
    pub(crate) trigger: String,
    pub(crate) required_behavior: String,
    pub(crate) prohibited_behavior: String,
    pub(crate) verification: String,
    pub(crate) environment: String,
    pub(crate) content_kinds: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DraftQualityCheckV1 {
    pub(crate) code: String,
    pub(crate) result: String,
    pub(crate) reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutomaticCorrectionDraftRequestV1 {
    pub(crate) workspace_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) authorization_id: String,
    pub(crate) assessment_id: String,
    pub(crate) trigger: String,
    pub(crate) verification: String,
    pub(crate) original_lesson_shape: CorrectionLessonShapeV1,
    pub(crate) original_checks: Vec<DraftQualityCheckV1>,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) overlay_scope: String,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DraftSafetyReceiptV1 {
    pub(crate) scanner_version: String,
    pub(crate) overlay_preview_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DraftQualityReceiptV1 {
    pub(crate) checks: Vec<DraftQualityCheckV1>,
    pub(crate) deterministic_approvable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidatedAutomaticCorrectionDraftV1 {
    pub(crate) draft: ProducedCorrectionDraftV1,
    pub(crate) safety: DraftSafetyReceiptV1,
    pub(crate) quality: DraftQualityReceiptV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutomaticDraftPipelineError {
    SourceUnavailable,
    InvalidDraft(CorrectionDraftError),
    UnsafeContent,
    OverlayRejected,
    QualityRejected,
    Storage,
}

pub(crate) trait AuthorizedCorrectionSourcePort: Send + Sync {
    fn resolve(
        &self,
        authorization_id: &str,
    ) -> Result<Option<AuthorizedCorrectionSourceV1>, AutomaticDraftPipelineError>;
}

pub(crate) trait AutomaticDraftSafetyPort: Send + Sync {
    fn validate(
        &self,
        request: &AutomaticCorrectionDraftRequestV1,
        draft: &ProducedCorrectionDraftV1,
    ) -> Result<DraftSafetyReceiptV1, AutomaticDraftPipelineError>;
}

#[async_trait]
pub(crate) trait AutomaticDraftQualityPort: Send + Sync {
    async fn review(
        &self,
        request: &AutomaticCorrectionDraftRequestV1,
        draft: &ProducedCorrectionDraftV1,
    ) -> Result<DraftQualityReceiptV1, AutomaticDraftPipelineError>;
}

pub(crate) trait AutomaticDraftStore: Send + Sync {
    fn persist(
        &self,
        request: &AutomaticCorrectionDraftRequestV1,
        record: &DeterministicCorrectionDraftV1,
    ) -> Result<(), AutomaticDraftPipelineError>;
}

pub(crate) struct AutomaticCorrectionDraftPipeline<'a, S, V, Q, R> {
    source: &'a S,
    safety: &'a V,
    quality: &'a Q,
    store: &'a R,
}

impl<'a, S, V, Q, R> AutomaticCorrectionDraftPipeline<'a, S, V, Q, R>
where
    S: AuthorizedCorrectionSourcePort,
    V: AutomaticDraftSafetyPort,
    Q: AutomaticDraftQualityPort,
    R: AutomaticDraftStore,
{
    pub(crate) fn new(source: &'a S, safety: &'a V, quality: &'a Q, store: &'a R) -> Self {
        Self {
            source,
            safety,
            quality,
            store,
        }
    }

    pub(crate) async fn produce(
        &self,
        request: &AutomaticCorrectionDraftRequestV1,
    ) -> Result<ValidatedAutomaticCorrectionDraftV1, AutomaticDraftPipelineError> {
        let source = self
            .source
            .resolve(&request.authorization_id)?
            .ok_or(AutomaticDraftPipelineError::SourceUnavailable)?;
        if source.authorization_id != request.authorization_id {
            return Err(AutomaticDraftPipelineError::SourceUnavailable);
        }
        let draft = produce_authorized_correction_draft(&AuthorizedCorrectionDraftInputV1 {
            workspace_id: request.workspace_id.clone(),
            target_skill_id: request.target_skill_id.clone(),
            target_revision: request.target_revision.clone(),
            authorization_id: source.authorization_id,
            authorization_witness_hash: source.authorization_witness_hash,
            assessment_id: request.assessment_id.clone(),
            sanitizer_version: source.sanitizer_version,
            authorization_current: true,
            trigger: request.trigger.clone(),
            guidance: source.sanitized_guidance,
            verification: request.verification.clone(),
            created_at_ms: request.created_at_ms,
        })
        .map_err(AutomaticDraftPipelineError::InvalidDraft)?;
        let safety = self.safety.validate(request, &draft)?;
        let quality = self.quality.review(request, &draft).await?;
        if !quality.deterministic_approvable || !all_quality_checks_pass(&quality.checks) {
            return Err(AutomaticDraftPipelineError::QualityRejected);
        }
        self.store.persist(request, &draft.record)?;
        Ok(ValidatedAutomaticCorrectionDraftV1 {
            draft,
            safety,
            quality,
        })
    }
}

fn all_quality_checks_pass(checks: &[DraftQualityCheckV1]) -> bool {
    checks.len() == AUTO_DRAFT_QUALITY_CHECK_ORDER_V1.len()
        && checks
            .iter()
            .zip(AUTO_DRAFT_QUALITY_CHECK_ORDER_V1)
            .all(|(check, expected)| check.code == expected && check.result == "pass")
}
