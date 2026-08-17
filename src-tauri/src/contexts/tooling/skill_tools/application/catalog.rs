use super::{
    SkillToolBinding, SkillToolCatalogContext, SkillToolCatalogEntry, SkillToolCatalogMode,
};
use crate::contexts::tooling::skill_tools::domain::SkillToolLifecycle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolOwnerKind {
    Role,
    Utility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolCatalogCandidate {
    pub(crate) entry: SkillToolCatalogEntry,
    pub(crate) owner_kind: SkillToolOwnerKind,
    pub(crate) lifecycle: SkillToolLifecycle,
    pub(crate) archived: bool,
    pub(crate) shadowed: bool,
    pub(crate) requires_module_runtime: bool,
    pub(crate) allow_plan: bool,
}

/// Applies context eligibility to an already validated registry snapshot. Installation alone is
/// never enough: identity and exact revision must be present in the active context.
pub(crate) fn project_contextual_catalog(
    candidates: &[SkillToolCatalogCandidate],
    context: &SkillToolCatalogContext,
) -> Vec<SkillToolCatalogEntry> {
    candidates
        .iter()
        .filter(|candidate| eligible(candidate, context))
        .map(|candidate| candidate.entry.clone())
        .collect()
}

fn eligible(candidate: &SkillToolCatalogCandidate, context: &SkillToolCatalogContext) -> bool {
    if candidate.archived
        || !candidate
            .lifecycle
            .availability(
                candidate.shadowed,
                crate::contexts::tooling::skill_tools::MODULE_RUNTIME_ENABLED,
                candidate.requires_module_runtime,
            )
            .is_available()
        || matches!(context, SkillToolCatalogContext::ExternalCli { .. })
        || matches!(context_mode(context), Some(SkillToolCatalogMode::Plan))
            && !candidate.allow_plan
    {
        return false;
    }
    let workspace_matches = candidate.entry.key.source.workspace_path.as_deref()
        == context.workspace_path()
        || candidate.entry.key.source.workspace_path.is_none();
    if !workspace_matches {
        return false;
    }
    match (candidate.owner_kind, context) {
        (
            SkillToolOwnerKind::Role,
            SkillToolCatalogContext::RoleGeneration { loaded_roles, .. },
        ) => loaded_roles
            .iter()
            .any(|binding| binding_matches(binding, &candidate.entry)),
        (
            SkillToolOwnerKind::Utility,
            SkillToolCatalogContext::UtilityDelegation { utility, .. },
        ) => binding_matches(utility, &candidate.entry),
        _ => false,
    }
}

fn context_mode(context: &SkillToolCatalogContext) -> Option<SkillToolCatalogMode> {
    match context {
        SkillToolCatalogContext::RoleGeneration { mode, .. }
        | SkillToolCatalogContext::UtilityDelegation { mode, .. } => Some(*mode),
        SkillToolCatalogContext::ExternalCli { .. } => None,
    }
}

fn binding_matches(binding: &SkillToolBinding, entry: &SkillToolCatalogEntry) -> bool {
    binding.skill_id == entry.key.owner.as_str() && binding.revision == entry.key.revision.as_str()
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
