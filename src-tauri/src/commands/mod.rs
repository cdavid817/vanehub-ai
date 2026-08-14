pub(crate) mod agent_runtime;
pub(crate) mod code_intelligence;
pub(crate) mod communications;
pub(crate) mod desktop;
pub(crate) mod error;
pub(crate) mod execution_observability;
pub(crate) mod operations;
pub(crate) mod permissions;
mod registry;
pub(crate) mod retrieval;
pub(crate) mod sessions;
pub(crate) mod skill_evolution_evidence;
pub(crate) mod ssh_connections;
pub(crate) mod task_orchestration;
pub(crate) mod tooling;
pub(crate) mod work_board;
pub(crate) mod workspaces;

pub(crate) use registry::invoke_handler;
