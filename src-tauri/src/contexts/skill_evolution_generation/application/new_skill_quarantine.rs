use std::collections::BTreeSet;

use crate::contexts::skill_evolution_generation::domain::{
    GeneratedArtifactKind, GenerationQuarantineStatus, MutationPlanV1, MutationTargetV1,
    QuarantinedSkillProposalV1, RenderedGenerationArtifactV1, StructuredDraftV1,
};

use super::GenerationSafetyReceiptV1;
use super::{
    validate_mutation_plan_against_frozen, ExpectedGenerationTargetV1,
    MutationPlanValidationContextV1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewSkillEligibilityV1 {
    pub(crate) no_target: bool,
    pub(crate) uncovered_capability_confidence_basis_points: u16,
    pub(crate) independent_run_ids: BTreeSet<String>,
    pub(crate) non_target_checks_passed: bool,
    pub(crate) focused_capability: bool,
    pub(crate) explicitly_requested_by_user_or_curator: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillCatalogInventoryV1 {
    pub(crate) effective_ids: BTreeSet<String>,
    pub(crate) shadowed_ids: BTreeSet<String>,
    pub(crate) reserved_ids: BTreeSet<String>,
    pub(crate) quarantined_ids: BTreeSet<String>,
    pub(crate) archived_ids: BTreeSet<String>,
    pub(crate) recently_rejected_ids: BTreeSet<String>,
    pub(crate) catalog_witness_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewSkillCreationPreviewV1 {
    pub(crate) candidate_id: String,
    pub(crate) scope: String,
    pub(crate) workspace_id: Option<String>,
    pub(crate) skill_type: String,
    pub(crate) frontmatter: String,
    pub(crate) instructions: String,
    pub(crate) estimated_tokens: u32,
    pub(crate) built_in_tools: Vec<String>,
    pub(crate) collision_free: bool,
    pub(crate) catalog_witness_hash: String,
    pub(crate) artifact_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedQuarantinedSkillV1 {
    pub(crate) proposal: QuarantinedSkillProposalV1,
    pub(crate) rendered_skill_md: String,
    pub(crate) preview: NewSkillCreationPreviewV1,
    pub(crate) created_at_ms: i64,
}

pub(crate) struct PrepareNewSkillQuarantineRequestV1<'a> {
    pub(crate) proposal_id: &'a str,
    pub(crate) job_id: &'a str,
    pub(crate) plan: &'a MutationPlanV1,
    pub(crate) draft: &'a StructuredDraftV1,
    pub(crate) artifact: &'a RenderedGenerationArtifactV1,
    pub(crate) eligibility: &'a NewSkillEligibilityV1,
    pub(crate) inventory: &'a SkillCatalogInventoryV1,
    pub(crate) expected_catalog_witness_hash: &'a str,
    pub(crate) requested_scope: &'a str,
    pub(crate) requested_workspace_id: Option<&'a str>,
    pub(crate) registered_citations: &'a BTreeSet<(String, String)>,
    pub(crate) estimated_tokens: u32,
    pub(crate) maximum_tokens: u32,
    pub(crate) created_at_ms: i64,
}

pub(crate) trait NewSkillQuarantineValidationPort {
    fn scan(
        &self,
        artifact: &RenderedGenerationArtifactV1,
    ) -> Result<GenerationSafetyReceiptV1, &'static str>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NewSkillQuarantineError {
    Ineligible,
    InvalidTarget,
    Collision,
    InvalidArtifact,
    UnsafeContent,
}

pub(crate) fn prepare_new_skill_quarantine(
    port: &dyn NewSkillQuarantineValidationPort,
    request: &PrepareNewSkillQuarantineRequestV1<'_>,
) -> Result<PreparedQuarantinedSkillV1, NewSkillQuarantineError> {
    validate_eligibility(request.eligibility)?;
    let (candidate_id, scope, workspace_id) = target(request.plan)?;
    let fields = draft_fields(request.draft)?;
    let plan_context = MutationPlanValidationContextV1 {
        expected_target: ExpectedGenerationTargetV1::NewSkill {
            candidate_id: candidate_id.into(),
            scope: request.requested_scope.into(),
            workspace_id: request.requested_workspace_id.map(str::to_owned),
        },
        registered_citations: request.registered_citations.clone(),
    };
    if candidate_id != fields.candidate_id
        || validate_mutation_plan_against_frozen(request.plan, &plan_context).is_err()
        || request.plan.artifact_kind != GeneratedArtifactKind::NewSkill
        || request.artifact.artifact_kind != GeneratedArtifactKind::NewSkill
        || request.artifact.media_type != "text/markdown"
        || request.proposal_id.trim().is_empty()
        || request.job_id.trim().is_empty()
        || request.created_at_ms < 0
        || request.maximum_tokens == 0
        || request.estimated_tokens > request.maximum_tokens
    {
        return Err(NewSkillQuarantineError::InvalidArtifact);
    }
    validate_scope(scope, workspace_id)?;
    if request.expected_catalog_witness_hash.trim().is_empty()
        || request.expected_catalog_witness_hash != request.inventory.catalog_witness_hash
    {
        return Err(NewSkillQuarantineError::InvalidTarget);
    }
    validate_inventory(candidate_id, request.inventory)?;
    let safety = port
        .scan(request.artifact)
        .map_err(|_| NewSkillQuarantineError::UnsafeContent)?;
    if safety.sanitizer_version.trim().is_empty()
        || safety.content_hash != request.artifact.content_hash
        || !safety.privacy_passed
        || !safety.injection_passed
        || !safety.prohibited_content_passed
    {
        return Err(NewSkillQuarantineError::UnsafeContent);
    }
    let (frontmatter, instructions) = split_skill_document(&request.artifact.content)?;
    Ok(PreparedQuarantinedSkillV1 {
        proposal: QuarantinedSkillProposalV1 {
            proposal_id: request.proposal_id.into(),
            job_id: request.job_id.into(),
            status: GenerationQuarantineStatus::Quarantined,
            candidate_id: candidate_id.into(),
            scope: scope.into(),
            workspace_id: workspace_id.map(str::to_owned),
            artifact_hash: request.artifact.content_hash.clone(),
            catalog_witness_hash: request.inventory.catalog_witness_hash.clone(),
            revision: 1,
        },
        rendered_skill_md: request.artifact.content.clone(),
        preview: NewSkillCreationPreviewV1 {
            candidate_id: candidate_id.into(),
            scope: scope.into(),
            workspace_id: workspace_id.map(str::to_owned),
            skill_type: fields.skill_type.into(),
            frontmatter,
            instructions,
            estimated_tokens: request.estimated_tokens,
            built_in_tools: fields.built_in_tools.to_vec(),
            collision_free: true,
            catalog_witness_hash: request.inventory.catalog_witness_hash.clone(),
            artifact_hash: request.artifact.content_hash.clone(),
        },
        created_at_ms: request.created_at_ms,
    })
}

fn validate_eligibility(value: &NewSkillEligibilityV1) -> Result<(), NewSkillQuarantineError> {
    if !value.no_target
        || value.uncovered_capability_confidence_basis_points < 9_000
        || value.independent_run_ids.len() < 3
        || value
            .independent_run_ids
            .iter()
            .any(|id| id.trim().is_empty())
        || !value.non_target_checks_passed
        || !value.focused_capability
        || !value.explicitly_requested_by_user_or_curator
    {
        return Err(NewSkillQuarantineError::Ineligible);
    }
    Ok(())
}

fn target(plan: &MutationPlanV1) -> Result<(&str, &str, Option<&str>), NewSkillQuarantineError> {
    match &plan.target {
        MutationTargetV1::NewSkill {
            candidate_id,
            scope,
            workspace_id,
        } => Ok((candidate_id, scope, workspace_id.as_deref())),
        MutationTargetV1::ExistingSkill { .. } => Err(NewSkillQuarantineError::InvalidTarget),
    }
}

struct NewSkillDraftFields<'a> {
    candidate_id: &'a str,
    skill_type: &'a str,
    built_in_tools: &'a [String],
}

fn draft_fields(
    draft: &StructuredDraftV1,
) -> Result<NewSkillDraftFields<'_>, NewSkillQuarantineError> {
    match draft {
        StructuredDraftV1::NewSkill {
            candidate_id,
            skill_type,
            built_in_tools,
            ..
        } => Ok(NewSkillDraftFields {
            candidate_id,
            skill_type,
            built_in_tools,
        }),
        _ => Err(NewSkillQuarantineError::InvalidArtifact),
    }
}

fn validate_scope(scope: &str, workspace_id: Option<&str>) -> Result<(), NewSkillQuarantineError> {
    let valid = match scope {
        "user" => workspace_id.is_none(),
        "project" => workspace_id.is_some_and(|id| !id.trim().is_empty()),
        _ => false,
    };
    valid
        .then_some(())
        .ok_or(NewSkillQuarantineError::InvalidTarget)
}

fn validate_inventory(
    candidate_id: &str,
    inventory: &SkillCatalogInventoryV1,
) -> Result<(), NewSkillQuarantineError> {
    if inventory.catalog_witness_hash.trim().is_empty() {
        return Err(NewSkillQuarantineError::InvalidTarget);
    }
    let collision = [
        &inventory.effective_ids,
        &inventory.shadowed_ids,
        &inventory.reserved_ids,
        &inventory.quarantined_ids,
        &inventory.archived_ids,
        &inventory.recently_rejected_ids,
    ]
    .into_iter()
    .any(|ids| ids.contains(candidate_id));
    (!collision)
        .then_some(())
        .ok_or(NewSkillQuarantineError::Collision)
}

fn split_skill_document(content: &str) -> Result<(String, String), NewSkillQuarantineError> {
    let rest = content
        .strip_prefix("---\n")
        .ok_or(NewSkillQuarantineError::InvalidArtifact)?;
    let (frontmatter, instructions) = rest
        .split_once("\n---\n\n")
        .ok_or(NewSkillQuarantineError::InvalidArtifact)?;
    if frontmatter.contains("config_schema:")
        || frontmatter.contains("delegation:")
        || instructions.trim().is_empty()
    {
        return Err(NewSkillQuarantineError::InvalidArtifact);
    }
    Ok((frontmatter.into(), instructions.trim().into()))
}
