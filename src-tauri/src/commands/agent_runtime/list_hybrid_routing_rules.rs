use super::{dto, mapper};
use crate::commands::error::CommandError;
use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use tauri::State;

#[tauri::command]
pub(crate) fn list_hybrid_routing_rules(
    api: State<'_, AgentRuntimeApi>,
) -> Result<Vec<dto::HybridRoutingRule>, CommandError> {
    api.hybrid_routing_rules()
        .map(|rules| rules.into_iter().map(mapper::hybrid_rule_to_dto).collect())
        .map_err(CommandError::from)
}
