use super::{ArtifactCreateRequest, ArtifactServiceError};
use std::collections::BTreeSet;

pub(super) fn validate_request(
    request: &ArtifactCreateRequest,
) -> Result<(), ArtifactServiceError> {
    let sources = request.source_artifact_ids.iter().collect::<BTreeSet<_>>();
    if request.creator.kind.trim().is_empty()
        || request.creator.id.trim().is_empty()
        || request.created_at.trim().is_empty()
        || sources.len() != request.source_artifact_ids.len()
        || request
            .source_artifact_ids
            .iter()
            .any(|source| source.trim().is_empty())
    {
        return Err(ArtifactServiceError::InvalidRequest);
    }
    Ok(())
}

pub(super) fn validate_artifact_id(artifact_id: &str) -> Result<(), ArtifactServiceError> {
    if !artifact_id.starts_with("artifact-")
        || artifact_id.len() > 64
        || !artifact_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(ArtifactServiceError::InvalidRequest);
    }
    Ok(())
}
