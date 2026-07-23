use super::{dto, mapper};
use crate::contexts::execution_observability::api::{ExecutionObservabilityApi, PageRequest};
use tauri::State;

#[tauri::command]
pub(crate) fn list_execution_runs(
    api: State<'_, ExecutionObservabilityApi>,
    request: dto::PageRequestDto,
    session_id: Option<String>,
) -> Result<dto::ExecutionRunPageDto, dto::ObservabilityCommandErrorDto> {
    let request = PageRequest::new(request.limit, request.page_token)
        .map_err(|error| mapper::invalid_page(error.to_string()))?;
    api.list_runs(&request, session_id.as_deref())
        .map(mapper::runs_to_dto)
        .map_err(mapper::adapter_error)
}
