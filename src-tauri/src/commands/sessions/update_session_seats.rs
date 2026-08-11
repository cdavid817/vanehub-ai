use super::{dto, mapper};
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::sessions::api::{SessionsApi, UpdateSessionSeatsRequest};
use tauri::State;

#[tauri::command]
pub(crate) fn update_session_seats(
    api: State<'_, SessionsApi>,
    input: dto::UpdateSessionSeatsInput,
) -> Result<dto::Session, CommandError> {
    api.update_seats(UpdateSessionSeatsRequest {
        session_id: input.session_id,
        expected_updated_at: input.expected_updated_at,
        seats: mapper::seats_from_dto(input.seats),
    })
    .and_then(mapper::session_to_dto)
    .map_err(map_command_error)
}
