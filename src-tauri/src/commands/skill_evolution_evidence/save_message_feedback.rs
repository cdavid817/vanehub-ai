use serde::{Deserialize, Serialize};
use tauri::State;

use crate::contexts::skill_evolution_evidence::api::SkillEvolutionEvidenceApi;
use crate::contexts::skill_evolution_evidence::domain::FeedbackState;
use crate::contexts::skill_evolution_evidence::infrastructure::{
    FeedbackTransitionError, SaveFeedbackRequest,
};
use crate::contexts::skill_evolution_orchestration::api::SkillEvolutionOrchestrationApi;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveMessageFeedbackInput {
    message_id: String,
    expected_revision: u64,
    state: Option<FeedbackState>,
    correction_note: Option<String>,
    #[serde(default)]
    authorize_reusable_guidance: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReusableGuidanceAuthorizationDto {
    pub(crate) authorization_id: String,
    pub(crate) feedback_revision: u64,
    pub(crate) disclosure_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedMessageFeedbackDto {
    message_id: String,
    revision: u64,
    state: Option<FeedbackState>,
    correction_note: Option<String>,
    reusable_guidance_authorization: Option<ReusableGuidanceAuthorizationDto>,
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
    orchestration: State<'_, SkillEvolutionOrchestrationApi>,
    input: SaveMessageFeedbackInput,
) -> Result<SavedMessageFeedbackDto, FeedbackCommandError> {
    let saved = api
        .save_feedback(&SaveFeedbackRequest {
            message_id: input.message_id,
            expected_revision: input.expected_revision,
            state: input.state,
            correction_note: input.correction_note,
            authorize_reusable_guidance: input.authorize_reusable_guidance,
        })
        .map_err(map_error)?;
    orchestration.publish_feedback_change(
        saved.workspace_id.as_deref(),
        &saved.message_id,
        saved.revision,
        saved.authorization_event_id.as_deref(),
    );
    Ok(SavedMessageFeedbackDto {
        message_id: saved.message_id,
        revision: saved.revision,
        state: saved.state,
        correction_note: saved.sanitized_note,
        reusable_guidance_authorization: saved.reusable_guidance_authorization.map(
            |authorization| ReusableGuidanceAuthorizationDto {
                authorization_id: authorization.authorization_id,
                feedback_revision: authorization.feedback_revision,
                disclosure_version: authorization.disclosure_version,
            },
        ),
    })
}

pub(super) fn map_error(error: FeedbackTransitionError) -> FeedbackCommandError {
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
