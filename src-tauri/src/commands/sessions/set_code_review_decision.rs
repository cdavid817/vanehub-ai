use super::review_dto::ReviewSessionDto;
use super::review_error::map_review_error;
use crate::commands::error::CommandError;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::sessions::domain::ReviewDecision;
use tauri::State;

fn parse_review_decision(decision: &str) -> Result<ReviewDecision, CommandError> {
    match decision {
        "pending" => Ok(ReviewDecision::Pending),
        "accepted" => Ok(ReviewDecision::Accepted),
        "changes-requested" => Ok(ReviewDecision::ChangesRequested),
        _ => Err(CommandError::validation("invalid review decision")),
    }
}

#[tauri::command]
pub(crate) fn set_code_review_decision(
    api: State<'_, SessionsApi>,
    review_id: String,
    decision: String,
) -> Result<ReviewSessionDto, CommandError> {
    let decision = parse_review_decision(&decision)?;
    api.set_review_decision(&review_id, decision)
        .map(Into::into)
        .map_err(map_review_error)
}
