//! Language-server protocol, process, and persistence adapters.

// Bootstrap assembly is introduced after the persistence foundation in the task sequence.
#[cfg_attr(not(test), allow(dead_code))]
mod configuration_repository;
#[cfg_attr(not(test), allow(dead_code))]
mod diagnostics_cache;
mod document_invalidation;
#[cfg_attr(not(test), allow(dead_code))]
mod document_lease;
#[cfg_attr(not(test), allow(dead_code))]
mod document_snapshot;
// The actor introduced in task 2.4 becomes the production framing consumer.
#[cfg_attr(not(test), allow(dead_code))]
mod lsp_diagnostics;
#[cfg_attr(not(test), allow(dead_code))]
mod lsp_framing;
// The process coordinator introduced in task 3.6 becomes the production actor owner.
#[cfg_attr(not(test), allow(dead_code))]
mod json_rpc_actor;
// The initialize flow introduced in task 3.8 installs this production handler.
#[cfg_attr(not(test), allow(dead_code))]
mod lsp_server_requests;
// The lifecycle registry introduced in task 3.6 becomes the production owner.
#[cfg_attr(not(test), allow(dead_code))]
mod initialize_negotiation;
#[cfg_attr(not(test), allow(dead_code))]
mod lsp_stdio_child;
#[cfg_attr(not(test), allow(dead_code))]
mod position_conversion;
#[cfg_attr(not(test), allow(dead_code))]
mod process_registry;
#[cfg_attr(not(test), allow(dead_code))]
mod project_root;
mod runtime_notifications;
mod runtime_process_coordinator;
mod schema;
mod semantic_query_coordinator;
#[cfg_attr(not(test), allow(dead_code))]
mod semantic_results;
#[cfg_attr(not(test), allow(dead_code))]
mod server_discovery;
#[cfg_attr(not(test), allow(dead_code))]
mod server_test;
#[cfg_attr(not(test), allow(dead_code))]
mod shutdown_coordinator;

#[cfg(test)]
mod configuration_repository_tests;
#[cfg(test)]
mod diagnostics_cache_tests;
#[cfg(test)]
mod document_lease_tests;
#[cfg(test)]
mod document_snapshot_tests;
#[cfg(test)]
mod initialize_negotiation_tests;
#[cfg(test)]
mod json_rpc_actor_tests;
#[cfg(test)]
mod lsp_diagnostics_tests;
#[cfg(test)]
mod lsp_framing_tests;
#[cfg(test)]
mod lsp_server_requests_tests;
#[cfg(test)]
mod lsp_stdio_child_tests;
#[cfg(test)]
mod position_conversion_tests;
#[cfg(test)]
mod process_registry_tests;
#[cfg(test)]
mod project_root_tests;
#[cfg(test)]
mod runtime_process_coordinator_tests;
#[cfg(test)]
mod schema_tests;
#[cfg(test)]
mod semantic_query_coordinator_tests;
#[cfg(test)]
mod semantic_results_tests;
#[cfg(test)]
mod server_discovery_tests;
#[cfg(test)]
mod server_test_tests;
#[cfg(test)]
mod shutdown_coordinator_tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use configuration_repository::SqliteCodeIntelligenceRepository;
pub(crate) use document_invalidation::LspDocumentInvalidationQueue;
pub(crate) use lsp_diagnostics::LspDiagnosticLogger;
pub(crate) use position_conversion::AgentPosition;
pub(crate) use process_registry::{ActivationReason, LifecyclePolicy};
pub(crate) use project_root::{ProcessKey, ProjectRootError, ProjectRootResolver};
pub(crate) use runtime_process_coordinator::{LspProcessLaunch, RuntimeProcessCoordinator};
pub(crate) use schema::{apply_language_registry_schema, apply_schema};
pub(crate) use semantic_query_coordinator::SemanticQueryCoordinator;
pub(crate) use server_discovery::{
    DiscoveryAvailability, DiscoveryReason, ServerDiscovery, ServerDiscoveryResult,
    SystemNativeExecutableLocator,
};
pub(crate) use server_test::{
    IsolatedServerTestResult, IsolatedServerTester, ServerTestCommand, ServerTestPhase,
    ServerTestPhaseStatus, ServerTestReason,
};
pub(crate) use shutdown_coordinator::LspShutdownCoordinator;
