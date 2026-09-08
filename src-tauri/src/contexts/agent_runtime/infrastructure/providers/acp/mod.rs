//! ACP (Agent Client Protocol) over stdio: the managed-conversation transport for the six
//! ACP-capable CLI providers.
//!
//! Layering, inside to out: `framing` and `jsonrpc` are the wire; `connection` owns one child
//! process and its reader/writer; `session` negotiates and drives prompt turns; `handlers` decide
//! agent-originated requests against the permission policy with `proxy_fs` and `proxy_terminal`
//! doing the host-side work; `interactions` holds what is waiting on a person; `binding` persists
//! the execution identity; `adapter` plugs the whole thing into the `AgentProcessGateway` port.

pub(crate) mod adapter;
pub(crate) mod binding;
pub(crate) mod blocks;
pub(crate) mod budget;
pub(crate) mod connection;
pub(crate) mod environment;
pub(crate) mod framing;
pub(crate) mod handlers;
pub(crate) mod interactions;
pub(crate) mod jsonrpc;
pub(crate) mod proxy_fs;
pub(crate) mod proxy_terminal;
pub(crate) mod session;

pub(crate) use adapter::{AcpAgentProcessAdapter, AcpAgentProcessDependencies, ACP_PROCESS_PREFIX};
pub(crate) use binding::apply_schema as apply_execution_binding_schema;

use super::definitions;

#[cfg(test)]
mod tests;
