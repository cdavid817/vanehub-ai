use crate::contexts::skill_evolution_generation::{
    application::canonical_hash,
    domain::{
        GeneratedArtifactKind, GeneratedReviewPackagePayloadV1, GeneratedReviewPackageV1,
        GenerationHandoffStatus, GenerationModelCallRecordV1, GenerationValidationStatus,
        GenerationValidationV1, MutationPlanV1, PreparedGenerationReviewPackageV1,
        QuarantinedSkillProposalV1, RenderedGenerationArtifactV1, StructuredDraftV1,
    },
};

pub(crate) struct BuildReviewPackageRequestV1<'a> {
    pub(crate) package: GeneratedReviewPackageV1,
    pub(crate) dossier_revision: u64,
    pub(crate) dossier_hash: &'a str,
    pub(crate) plan: MutationPlanV1,
    pub(crate) structured_draft: StructuredDraftV1,
    pub(crate) rendered_artifact: RenderedGenerationArtifactV1,
    pub(crate) validation: GenerationValidationV1,
    pub(crate) model_calls: Vec<GenerationModelCallRecordV1>,
    pub(crate) quarantine: Option<QuarantinedSkillProposalV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewPackageError {
    InvalidBinding,
    ValidationFailed,
    InvalidProvenance,
    Failed,
}

pub(crate) fn build_review_package(
    request: BuildReviewPackageRequestV1<'_>,
) -> Result<PreparedGenerationReviewPackageV1, ReviewPackageError> {
    validate_binding(&request)?;
    let payload = GeneratedReviewPackagePayloadV1 {
        package: request.package,
        dossier_revision: request.dossier_revision,
        dossier_hash: request.dossier_hash.into(),
        plan: request.plan,
        structured_draft: request.structured_draft,
        rendered_artifact: request.rendered_artifact,
        validation: request.validation,
        model_calls: request.model_calls,
        quarantine: request.quarantine,
        parent_package_id: None,
        user_edited: false,
        auto_apply_excluded: true,
    };
    let package_hash = canonical_hash(&payload).map_err(|_| ReviewPackageError::Failed)?;
    Ok(PreparedGenerationReviewPackageV1 {
        payload,
        package_hash,
    })
}

pub(crate) fn derive_user_edited_review_package(
    parent: &PreparedGenerationReviewPackageV1,
    package_id: &str,
    structured_draft: StructuredDraftV1,
    rendered_artifact: RenderedGenerationArtifactV1,
    validation: GenerationValidationV1,
    created_at_ms: i64,
) -> Result<PreparedGenerationReviewPackageV1, ReviewPackageError> {
    if package_id.trim().is_empty()
        || package_id == parent.payload.package.package_id
        || created_at_ms < parent.payload.package.created_at_ms
        || rendered_artifact.artifact_kind != parent.payload.plan.artifact_kind
        || rendered_artifact.artifact_id != validation.artifact_id
        || validation.status != GenerationValidationStatus::Passed
        || rendered_artifact.content_hash == parent.payload.rendered_artifact.content_hash
    {
        return Err(ReviewPackageError::InvalidBinding);
    }
    let mut payload = parent.payload.clone();
    payload.package.package_id = package_id.into();
    payload.package.artifact_id = rendered_artifact.artifact_id.clone();
    payload.package.validation_id = validation.validation_id.clone();
    payload.package.preview_witness_hash = validation.preview_witness_hash.clone();
    payload.package.handoff_status = GenerationHandoffStatus::Pending;
    payload.package.curator_candidate_id = None;
    payload.package.created_at_ms = created_at_ms;
    payload.structured_draft = structured_draft;
    payload.rendered_artifact = rendered_artifact;
    payload.validation = validation;
    payload.parent_package_id = Some(parent.payload.package.package_id.clone());
    payload.user_edited = true;
    payload.auto_apply_excluded = true;
    payload.package.permanently_manual = true;
    let package_hash = canonical_hash(&payload).map_err(|_| ReviewPackageError::Failed)?;
    Ok(PreparedGenerationReviewPackageV1 {
        payload,
        package_hash,
    })
}

fn validate_binding(request: &BuildReviewPackageRequestV1<'_>) -> Result<(), ReviewPackageError> {
    let package = &request.package;
    if package.package_id.trim().is_empty()
        || package.job_id.trim().is_empty()
        || package.attempt_id.trim().is_empty()
        || package.dossier_id.trim().is_empty()
        || request.dossier_revision == 0
        || request.dossier_hash.trim().is_empty()
        || package.handoff_status != GenerationHandoffStatus::Pending
        || !package.permanently_manual
        || package.curator_candidate_id.is_some()
        || package.created_at_ms < 0
        || package.artifact_id != request.rendered_artifact.artifact_id
        || package.validation_id != request.validation.validation_id
        || package.preview_witness_hash != request.validation.preview_witness_hash
        || request.validation.artifact_id != request.rendered_artifact.artifact_id
        || request.plan.artifact_kind != request.rendered_artifact.artifact_kind
        || !draft_matches(request.plan.artifact_kind, &request.structured_draft)
    {
        return Err(ReviewPackageError::InvalidBinding);
    }
    if request.validation.status != GenerationValidationStatus::Passed {
        return Err(ReviewPackageError::ValidationFailed);
    }
    let plan_hash = canonical_hash(&request.plan).map_err(|_| ReviewPackageError::Failed)?;
    let provenance_hash =
        canonical_hash(&request.model_calls).map_err(|_| ReviewPackageError::Failed)?;
    if package.plan_hash != plan_hash
        || package.model_provenance_hash != provenance_hash
        || request.model_calls.len() > 3
        || request.model_calls.iter().any(|call| {
            call.purpose != "skill_evolution_generation" || call.stage_attempt_id.trim().is_empty()
        })
    {
        return Err(ReviewPackageError::InvalidProvenance);
    }
    match request.plan.artifact_kind {
        GeneratedArtifactKind::NewSkill => {
            let Some(quarantine) = &request.quarantine else {
                return Err(ReviewPackageError::InvalidBinding);
            };
            if quarantine.job_id != package.job_id
                || quarantine.artifact_hash != request.rendered_artifact.content_hash
            {
                return Err(ReviewPackageError::InvalidBinding);
            }
        }
        _ if request.quarantine.is_some() => return Err(ReviewPackageError::InvalidBinding),
        _ => {}
    }
    Ok(())
}

fn draft_matches(kind: GeneratedArtifactKind, draft: &StructuredDraftV1) -> bool {
    matches!(
        (kind, draft),
        (
            GeneratedArtifactKind::OverlayLearnBlock,
            StructuredDraftV1::OverlayLearnBlock { .. }
        ) | (
            GeneratedArtifactKind::OverlayExactPatch,
            StructuredDraftV1::OverlayExactPatch { .. }
        ) | (
            GeneratedArtifactKind::NewSkill,
            StructuredDraftV1::NewSkill { .. }
        )
    )
}
