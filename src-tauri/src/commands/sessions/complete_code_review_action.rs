use super::review_dto::{ReviewAnchorInput, ReviewSessionDto};
use super::review_error::map_review_error;
use crate::commands::error::CommandError;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::sessions::application::{ReviewAction, ReviewActionFindingInput};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompleteCodeReviewFindingInput {
    title: String,
    severity: String,
    anchor: Option<ReviewAnchorInput>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompleteCodeReviewActionInput {
    review_id: String,
    operation_id: String,
    action: String,
    findings: Vec<CompleteCodeReviewFindingInput>,
}

impl CompleteCodeReviewActionInput {
    fn into_domain(
        self,
    ) -> Result<(String, String, ReviewAction, Vec<ReviewActionFindingInput>), CommandError> {
        let action = match self.action.as_str() {
            "review-agent" => ReviewAction::ReviewAgent,
            "tests" => ReviewAction::Tests,
            "security" => ReviewAction::Security,
            _ => return Err(CommandError::validation("invalid review action")),
        };
        let findings = self
            .findings
            .into_iter()
            .map(|finding| {
                Ok(ReviewActionFindingInput {
                    title: finding.title,
                    severity: finding.severity,
                    anchor: finding
                        .anchor
                        .map(ReviewAnchorInput::into_domain)
                        .transpose()?,
                })
            })
            .collect::<Result<Vec<_>, String>>()
            .map_err(CommandError::validation)?;
        Ok((self.review_id, self.operation_id, action, findings))
    }
}

#[tauri::command]
pub(crate) fn complete_code_review_action(
    api: State<'_, SessionsApi>,
    input: CompleteCodeReviewActionInput,
) -> Result<ReviewSessionDto, CommandError> {
    let (review_id, operation_id, action, findings) = input.into_domain()?;
    api.complete_review_action(&review_id, action, &operation_id, findings)
        .map(Into::into)
        .map_err(map_review_error)
}
