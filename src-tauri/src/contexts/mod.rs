//! Bounded contexts for native domain and application behavior.

pub(crate) mod agent_runtime;
pub(crate) mod artifacts;
pub(crate) mod browser_automation;
pub(crate) mod cli_delegation;
pub(crate) mod code_execution;
pub(crate) mod code_intelligence;
pub(crate) mod communications;
pub(crate) mod desktop;
pub(crate) mod execution_observability;
pub(crate) mod goals;
pub(crate) mod operations;
pub(crate) mod permissions;
pub(crate) mod retrieval;
pub(crate) mod sessions;
// The evidence context is intentionally dormant until its bounded ingestion worker is enabled.
#[allow(dead_code, unused_imports)]
pub(crate) mod skill_evolution_evidence;
pub(crate) mod ssh_connections;
pub(crate) mod task_orchestration;
pub(crate) mod tooling;
pub(crate) mod web_research;
pub(crate) mod work_board;
pub(crate) mod workspaces;
