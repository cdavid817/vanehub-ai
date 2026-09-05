use std::collections::BTreeSet;

use crate::contexts::skill_evolution_generation::domain::{
    GeneratedArtifactKind, GenerationCitationV1, MutationPlanV1, MutationTargetV1,
};

const MAX_RATIONALE_CHARACTERS_V1: usize = 1_000;
const MAX_CLAIM_CHARACTERS_V1: usize = 2_000;
const MAX_CITATIONS_V1: usize = 64;
const MAX_VERIFICATION_STEPS_V1: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExpectedGenerationTargetV1 {
    ExistingSkill {
        skill_id: String,
        effective_revision: String,
        overlay_scope: String,
    },
    NewSkill {
        candidate_id: String,
        scope: String,
        workspace_id: Option<String>,
    },
}

pub(crate) struct MutationPlanValidationContextV1 {
    pub(crate) expected_target: ExpectedGenerationTargetV1,
    pub(crate) registered_citations: BTreeSet<(String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MutationPlanValidationError {
    InvalidShape,
    TargetDrift,
    InventedCitation,
    UncitedClaim,
}

pub(crate) fn validate_mutation_plan_structure(
    plan: &MutationPlanV1,
) -> Result<(), MutationPlanValidationError> {
    if plan.schema_version != 1
        || plan.plan_id.trim().is_empty()
        || plan.content_hash.trim().is_empty()
        || !bounded(&plan.rationale, MAX_RATIONALE_CHARACTERS_V1)
        || !bounded(&plan.lesson.trigger, MAX_CLAIM_CHARACTERS_V1)
        || !bounded(&plan.lesson.action, MAX_CLAIM_CHARACTERS_V1)
        || !bounded(&plan.lesson.verification, MAX_CLAIM_CHARACTERS_V1)
        || !bounded(&plan.expected_behavior, MAX_CLAIM_CHARACTERS_V1)
        || plan.evidence_citations.is_empty()
        || plan.evidence_citations.len() > MAX_CITATIONS_V1
        || plan.verification_steps.is_empty()
        || plan.verification_steps.len() > MAX_VERIFICATION_STEPS_V1
    {
        return Err(MutationPlanValidationError::InvalidShape);
    }
    validate_target_kind(plan)?;
    let mut step_ids = BTreeSet::new();
    for step in &plan.verification_steps {
        if !step_ids.insert(step.step_id.as_str())
            || !bounded(&step.step_id, 128)
            || !bounded(&step.action_code, 256)
            || !bounded(&step.expected_code, 256)
            || step.citation_ids.is_empty()
        {
            return Err(MutationPlanValidationError::InvalidShape);
        }
    }
    let mut citations = BTreeSet::new();
    for citation in &plan.evidence_citations {
        if !valid_citation(citation)
            || !citations.insert((
                citation.claim_id.as_str(),
                citation.dossier_section.as_str(),
                citation.source_id.as_str(),
            ))
        {
            return Err(MutationPlanValidationError::InvalidShape);
        }
    }
    Ok(())
}

pub(crate) fn validate_mutation_plan_against_frozen(
    plan: &MutationPlanV1,
    context: &MutationPlanValidationContextV1,
) -> Result<(), MutationPlanValidationError> {
    validate_mutation_plan_structure(plan)?;
    if !target_matches(&plan.target, &context.expected_target) {
        return Err(MutationPlanValidationError::TargetDrift);
    }
    if plan.evidence_citations.iter().any(|citation| {
        !context
            .registered_citations
            .contains(&(citation.dossier_section.clone(), citation.source_id.clone()))
    }) {
        return Err(MutationPlanValidationError::InventedCitation);
    }
    let claims: BTreeSet<&str> = plan
        .evidence_citations
        .iter()
        .map(|citation| citation.claim_id.as_str())
        .collect();
    let required = [
        "lesson.trigger",
        "lesson.action",
        "lesson.verification",
        "expected_behavior",
    ];
    if required.iter().any(|claim| !claims.contains(claim))
        || plan
            .verification_steps
            .iter()
            .any(|step| !claims.contains(step.step_id.as_str()))
    {
        return Err(MutationPlanValidationError::UncitedClaim);
    }
    for step in &plan.verification_steps {
        if step.citation_ids.iter().any(|source_id| {
            !plan
                .evidence_citations
                .iter()
                .any(|item| &item.source_id == source_id)
        }) {
            return Err(MutationPlanValidationError::InventedCitation);
        }
    }
    Ok(())
}

fn validate_target_kind(plan: &MutationPlanV1) -> Result<(), MutationPlanValidationError> {
    let compatible = matches!(
        (&plan.artifact_kind, &plan.target),
        (
            GeneratedArtifactKind::OverlayLearnBlock | GeneratedArtifactKind::OverlayExactPatch,
            MutationTargetV1::ExistingSkill { .. }
        ) | (
            GeneratedArtifactKind::NewSkill,
            MutationTargetV1::NewSkill { .. }
        )
    );
    compatible
        .then_some(())
        .ok_or(MutationPlanValidationError::InvalidShape)
}

fn target_matches(target: &MutationTargetV1, expected: &ExpectedGenerationTargetV1) -> bool {
    match (target, expected) {
        (
            MutationTargetV1::ExistingSkill {
                skill_id,
                effective_revision,
                overlay_scope,
            },
            ExpectedGenerationTargetV1::ExistingSkill {
                skill_id: expected_id,
                effective_revision: expected_revision,
                overlay_scope: expected_scope,
            },
        ) => {
            skill_id == expected_id
                && effective_revision == expected_revision
                && overlay_scope == expected_scope
        }
        (
            MutationTargetV1::NewSkill {
                candidate_id,
                scope,
                workspace_id,
            },
            ExpectedGenerationTargetV1::NewSkill {
                candidate_id: expected_id,
                scope: expected_scope,
                workspace_id: expected_workspace,
            },
        ) => {
            candidate_id == expected_id
                && scope == expected_scope
                && workspace_id == expected_workspace
        }
        _ => false,
    }
}

fn valid_citation(citation: &GenerationCitationV1) -> bool {
    bounded(&citation.claim_id, 128)
        && bounded(&citation.dossier_section, 128)
        && bounded(&citation.source_id, 256)
}

fn bounded(value: &str, max_characters: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max_characters
}
