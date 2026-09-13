use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::agent_runtime::api::{AgentRuntimeApi, RequestLoopAcceptanceRequest};
use tauri::State;

#[tauri::command]
pub(crate) fn request_loop_acceptance(
    api: State<'_, AgentRuntimeApi>,
    input: dto::RequestLoopAcceptanceInput,
) -> Result<dto::LoopAcceptanceResult, CommandError> {
    let accepted = api
        .request_loop_acceptance(RequestLoopAcceptanceRequest {
            run_id: input.run_id,
            expected_revision: input.expected_revision,
            expected_scope_digest: input.expected_scope_digest,
            expected_evidence_id: input.expected_evidence_id,
            idempotency_key: input.idempotency_key,
        })
        .map_err(map_command_error)?;
    let run = api
        .get_loop_run(&accepted.run_id)
        .map_err(map_command_error)?;
    Ok(dto::LoopAcceptanceResult {
        run: mapper::run(run),
        operation_id: accepted.operation_id,
    })
}
