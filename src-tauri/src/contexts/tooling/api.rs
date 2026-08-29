//! The Tooling context's published cross-context surface.
//!
//! Only immutable contracts and launch-facing facades appear here. Repositories, catalog loaders,
//! command DTOs, and private domain modules stay inside their subdomain.

pub(crate) use super::cli::api::compare_versions;
// `PADDLEOCR_INFERENCE_PROTOCOL_VERSION` is deliberately absent. The extensions-side inference
// runtime it described is deleted: `local_media` is the single PaddleOCR owner, and re-exporting a
// protocol marker for a runtime that no longer exists would advertise a second one.

#[cfg(test)]
pub(crate) use super::cli_parameters::api::CliParameterDiagnostic;
/// CLI launch-parameter resolution. `agent_runtime`, Agent Terminal, and `sessions` consume this
/// and nothing else from the CLI-parameter subdomain: no repository, no catalog loader, no save,
/// no reset, no private domain module.
///
/// The surface is deliberately minimal — a symbol is added when a consumer needs it, so an unused
/// re-export can never quietly widen the published contract.
pub(crate) use super::cli_parameters::api::{
    CliLaunchExecutionContext, CliLaunchScope, CliParameterRuntimeApi, CliParameterSelection,
    CliParameterSelectionMap, CliParameterValue, ResolveCliLaunchParametersInput,
};
#[cfg(test)]
pub(crate) use super::skills::api::EffectiveSkillCatalogShadow;
pub(crate) use super::skills::api::{
    project_effective_skill_catalog, EffectiveSkill, EffectiveSkillCatalogEntry, SkillAvailability,
    SkillLayer, SkillTrust, SkillType,
};
