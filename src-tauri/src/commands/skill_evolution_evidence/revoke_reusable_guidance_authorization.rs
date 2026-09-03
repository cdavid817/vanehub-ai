use serde::Deserialize;
use tauri::State;

use crate::contexts::skill_evolution_evidence::{
    api::SkillEvolutionEvidenceApi, infrastructure::RevokeReusableGuidanceAuthorizationRequest,
};
use crate::contexts::skill_evolution_orchestration::api::SkillEvolutionOrchestrationApi;

use super::save_message_feedback::{map_error, FeedbackCommandError};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RevokeReusableGuidanceAuthorizationInput {
    message_id: String,
    expected_feedback_revision: u64,
}

#[tauri::command]
pub(crate) fn revoke_reusable_guidance_authorization(
    api: State<'_, SkillEvolutionEvidenceApi>,
    orchestration: State<'_, SkillEvolutionOrchestrationApi>,
    input: RevokeReusableGuidanceAuthorizationInput,
) -> Result<(), FeedbackCommandError> {
    let revoked = api
        .revoke_reusable_guidance_authorization(&RevokeReusableGuidanceAuthorizationRequest {
            message_id: input.message_id,
            expected_feedback_revision: input.expected_feedback_revision,
        })
        .map_err(map_error)?;
    orchestration.publish_feedback_change(
        revoked.workspace_id.as_deref(),
        &revoked.message_id,
        revoked.feedback_revision,
        Some(&revoked.authorization_event_id),
    );
    Ok(())
}
