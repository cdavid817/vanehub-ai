use crate::contexts::agent_runtime::api::{
    AgentRuntimeApi, ManualApplyDelegationRequest, ManualStartDelegationRequest,
};
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub(crate) async fn start_delegation(
    api: State<'_, AgentRuntimeApi>,
    input: ManualStartDelegationRequest,
) -> Result<Value, String> {
    api.start_manual_delegation(input).await
}

#[tauri::command]
pub(crate) async fn apply_delegation_changes(
    api: State<'_, AgentRuntimeApi>,
    input: ManualApplyDelegationRequest,
) -> Result<Value, String> {
    api.apply_manual_delegation_changes(input).await
}
