use crate::contexts::skill_evolution_generation::{
    application::canonical_hash,
    domain::{GeneratedArtifactKind, GenerationHandoffStatus, PreparedGenerationReviewPackageV1},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationCuratorHandoffResultV1 {
    pub(crate) package_id: String,
    pub(crate) package_hash: String,
    pub(crate) curator_candidate_id: String,
    pub(crate) status: GenerationHandoffStatus,
    pub(crate) creation_candidate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationCuratorHandoffError {
    InvalidPackage,
    AutoMutationForbidden,
    CuratorUnavailable,
    Conflict,
    Storage,
}

pub(crate) trait GenerationCuratorHandoffPort {
    fn attach_existing_draft(
        &self,
        package: &PreparedGenerationReviewPackageV1,
        idempotency_key: &str,
    ) -> Result<String, GenerationCuratorHandoffError>;

    fn attach_creation_candidate(
        &self,
        package: &PreparedGenerationReviewPackageV1,
        idempotency_key: &str,
    ) -> Result<String, GenerationCuratorHandoffError>;
}

pub(crate) fn handoff_generation_package(
    port: &dyn GenerationCuratorHandoffPort,
    package: &PreparedGenerationReviewPackageV1,
) -> Result<GenerationCuratorHandoffResultV1, GenerationCuratorHandoffError> {
    validate_package(package)?;
    let creation =
        package.payload.rendered_artifact.artifact_kind == GeneratedArtifactKind::NewSkill;
    let candidate_id = if creation {
        port.attach_creation_candidate(package, &package.package_hash)?
    } else {
        port.attach_existing_draft(package, &package.package_hash)?
    };
    if candidate_id.trim().is_empty() {
        return Err(GenerationCuratorHandoffError::Storage);
    }
    Ok(GenerationCuratorHandoffResultV1 {
        package_id: package.payload.package.package_id.clone(),
        package_hash: package.package_hash.clone(),
        curator_candidate_id: candidate_id,
        status: GenerationHandoffStatus::Delivered,
        creation_candidate: creation,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedSkillCreationTransactionV1 {
    pub(crate) package_id: String,
    pub(crate) package_hash: String,
    pub(crate) proposal_id: String,
    pub(crate) expected_proposal_revision: u64,
    pub(crate) expected_workspace_id: Option<String>,
    pub(crate) expected_catalog_witness_hash: String,
    pub(crate) expected_scope: String,
    pub(crate) expected_candidate_id: String,
    pub(crate) expected_preview_witness_hash: String,
    pub(crate) rendered_skill_md: String,
    pub(crate) artifact_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedSkillCommitRequestV1<'a> {
    pub(crate) package: &'a PreparedGenerationReviewPackageV1,
    pub(crate) rendered_skill_md: &'a str,
    pub(crate) interactive_approved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GeneratedSkillCommitResultV1 {
    pub(crate) skill_id: String,
    pub(crate) revision_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GeneratedSkillCommitError {
    NotApproved,
    InvalidPackage,
    StaleWitness,
    Collision,
    Storage,
}

pub(crate) trait NormalSkillCreationTransactionPort {
    fn commit(
        &self,
        transaction: &GeneratedSkillCreationTransactionV1,
    ) -> Result<GeneratedSkillCommitResultV1, GeneratedSkillCommitError>;
}

pub(crate) fn commit_approved_generated_skill(
    port: &dyn NormalSkillCreationTransactionPort,
    request: &GeneratedSkillCommitRequestV1<'_>,
) -> Result<GeneratedSkillCommitResultV1, GeneratedSkillCommitError> {
    if !request.interactive_approved {
        return Err(GeneratedSkillCommitError::NotApproved);
    }
    validate_package(request.package).map_err(|_| GeneratedSkillCommitError::InvalidPackage)?;
    let payload = &request.package.payload;
    if payload.rendered_artifact.artifact_kind != GeneratedArtifactKind::NewSkill
        || request.rendered_skill_md != payload.rendered_artifact.content
    {
        return Err(GeneratedSkillCommitError::InvalidPackage);
    }
    let quarantine = payload
        .quarantine
        .as_ref()
        .ok_or(GeneratedSkillCommitError::InvalidPackage)?;
    let preview_witness = payload
        .package
        .preview_witness_hash
        .clone()
        .ok_or(GeneratedSkillCommitError::InvalidPackage)?;
    let transaction = GeneratedSkillCreationTransactionV1 {
        package_id: payload.package.package_id.clone(),
        package_hash: request.package.package_hash.clone(),
        proposal_id: quarantine.proposal_id.clone(),
        expected_proposal_revision: quarantine.revision,
        expected_workspace_id: quarantine.workspace_id.clone(),
        expected_catalog_witness_hash: quarantine.catalog_witness_hash.clone(),
        expected_scope: quarantine.scope.clone(),
        expected_candidate_id: quarantine.candidate_id.clone(),
        expected_preview_witness_hash: preview_witness,
        rendered_skill_md: request.rendered_skill_md.into(),
        artifact_hash: quarantine.artifact_hash.clone(),
    };
    port.commit(&transaction)
}

fn validate_package(
    package: &PreparedGenerationReviewPackageV1,
) -> Result<(), GenerationCuratorHandoffError> {
    let expected_hash = canonical_hash(&package.payload)
        .map_err(|_| GenerationCuratorHandoffError::InvalidPackage)?;
    if package.package_hash != expected_hash
        || package.payload.package.handoff_status != GenerationHandoffStatus::Pending
        || !package.payload.package.permanently_manual
        || !package.payload.auto_apply_excluded
        || package.payload.package.curator_candidate_id.is_some()
    {
        return Err(GenerationCuratorHandoffError::AutoMutationForbidden);
    }
    Ok(())
}
