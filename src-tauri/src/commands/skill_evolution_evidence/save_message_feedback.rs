use serde::{Deserialize, Serialize};
use tauri::State;

use crate::contexts::skill_evolution_evidence::api::SkillEvolutionEvidenceApi;
use crate::contexts::skill_evolution_evidence::domain::FeedbackState;
use crate::contexts::skill_evolution_evidence::infrastructure::{
    FeedbackTransitionError, SaveFeedbackRequest,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveMessageFeedbackInput {
    message_id: String,
    expected_revision: u64,
    state: Option<FeedbackState>,
    correction_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedMessageFeedbackDto {
    message_id: String,
    revision: u64,
    state: Option<FeedbackState>,
    correction_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeedbackCommandError {
    code: &'static str,
    current_revision: Option<u64>,
}

#[tauri::command]
pub(crate) fn save_message_feedback(
    api: State<'_, SkillEvolutionEvidenceApi>,
    input: SaveMessageFeedbackInput,
) -> Result<SavedMessageFeedbackDto, FeedbackCommandError> {
    api.save_feedback(&SaveFeedbackRequest {
        message_id: input.message_id,
        expected_revision: input.expected_revision,
        state: input.state,
        correction_note: input.correction_note,
    })
    .map(|saved| SavedMessageFeedbackDto {
        message_id: saved.message_id,
        revision: saved.revision,
        state: saved.state,
        correction_note: saved.sanitized_note,
    })
    .map_err(map_error)
}

fn map_error(error: FeedbackTransitionError) -> FeedbackCommandError {
    match error {
        FeedbackTransitionError::Conflict { current_revision } => FeedbackCommandError {
            code: "feedback-conflict",
            current_revision: Some(current_revision),
        },
        FeedbackTransitionError::MessageNotEligible => FeedbackCommandError {
            code: "message-not-eligible",
            current_revision: None,
        },
        FeedbackTransitionError::InvalidInput => FeedbackCommandError {
            code: "invalid-feedback",
            current_revision: None,
        },
        FeedbackTransitionError::Storage => FeedbackCommandError {
            code: "feedback-save-failed",
            current_revision: None,
        },
    }
}
