//! The Tooling context's published cross-context surface.
//!
//! Only immutable contracts and launch-facing facades appear here. Repositories, catalog loaders,
//! command DTOs, and private domain modules stay inside their subdomain.

pub(crate) use super::cli::api::compare_versions;
// `PADDLEOCR_INFERENCE_PROTOCOL_VERSION` is deliberately absent. The extensions-side inference
// runtime it described is deleted: `local_media` is the single PaddleOCR owner, and re-exporting a
// protocol marker for a runtime that no longer exists would advertise a second one.

#[cfg(test)]
pub(crate) use super::managed_install::api::RetrievedArtifact;
/// Managed retrieval and unpacking of a vendor artifact.
///
/// `code_intelligence` consumes this to install a language server. Its registry declares a
/// distribution in these types rather than in a parallel vocabulary, so the bounds a download is
/// held to are stated once — which means a `domain` module of another context reaches them, and a
/// `domain` module may only reach a published surface.
pub(crate) use super::managed_install::api::{
    extract_tar_gz, extract_zip, ArtifactIntegrity, ArtifactRequest, ExtractionLimits,
    HttpsArtifactRetriever, ManagedArtifactRetriever, ManagedInstallError, RetrievalPolicy,
};

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
