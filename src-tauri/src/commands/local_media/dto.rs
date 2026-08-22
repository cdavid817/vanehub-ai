use serde::{Deserialize, Serialize};

use crate::contexts::local_media::domain::LocalMediaProfile;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveLocalMediaProfileRequest {
    pub(crate) profile: LocalMediaProfile,
    pub(crate) expected_revision: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ValidateLocalMediaProfileRequest {
    pub(crate) profile: LocalMediaProfile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProbeEngineRequest {
    pub(crate) engine: String,
}

/// The path the frontend adapter obtained from the native picker.
///
/// It is used once, here, and never returned: what goes back is an opaque staged-input id. That is
/// the whole point of a separate staging step -- a Python worker must never be handed a path the
/// renderer chose.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StageOcrSourceRequest {
    pub(crate) path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartOcrRequest {
    pub(crate) staged_input_id: String,
    pub(crate) composer_scope_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ComposerScopeRequest {
    pub(crate) composer_scope_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecordingRequest {
    pub(crate) recording_id: String,
    pub(crate) composer_scope_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartTtsRequest {
    pub(crate) text: String,
    pub(crate) composer_scope_id: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct StopPlaybackRequest {
    pub(crate) playback_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OperationRequest {
    pub(crate) operation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupStagedRequest {
    pub(crate) staged_input_id: String,
}

/// What a caller gets back the moment an operation is accepted, before any model has loaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalMediaOperationHandleDto {
    pub(crate) operation_id: String,
    pub(crate) kind: String,
    pub(crate) accepted_at: String,
}
