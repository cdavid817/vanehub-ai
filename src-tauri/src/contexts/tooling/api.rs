//! The Tooling context's published cross-context surface.
//!
//! Only immutable contracts and launch-facing facades appear here. Repositories, catalog loaders,
//! command DTOs, and private domain modules stay inside their subdomain.

pub(crate) use super::cli::api::compare_versions;
#[cfg(test)]
pub(crate) use super::extensions::api::PADDLEOCR_INFERENCE_PROTOCOL_VERSION;

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
