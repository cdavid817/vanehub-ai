use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::SessionsApi;
use tauri::State;

#[tauri::command]
pub(crate) fn acknowledge_session_recovery(
    api: State<'_, SessionsApi>,
    session_id: String,
    expected_recovery_revision: u64,
) -> Result<dto::SessionRecoveryAcknowledgement, CommandError> {
    api.acknowledge_recovery(&session_id, expected_recovery_revision)
        .and_then(|result| {
            Ok(dto::SessionRecoveryAcknowledgement {
                session: mapper::session_to_dto(result.session)?,
                report: mapper::recovery_report_to_dto(&result.report),
            })
        })
        .map_err(map_command_error)
}
