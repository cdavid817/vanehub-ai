pub(crate) mod agent_runtime;
pub(crate) mod communications;
pub(crate) mod desktop;
pub(crate) mod error;
pub(crate) mod execution_observability;
pub(crate) mod operations;
mod registry;
pub(crate) mod sessions;
pub(crate) mod ssh_connections;
pub(crate) mod tooling;
pub(crate) mod workspaces;

pub(crate) use registry::invoke_handler;
