use super::FetchBody;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FetchedBinaryArtifactRequest {
    pub(crate) operation_id: String,
    pub(crate) creator_id: String,
    pub(crate) expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FetchedBinaryReference {
    pub(crate) contract_version: u16,
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) size_bytes: u64,
    pub(crate) media_type: String,
    pub(crate) normalized_url: String,
    pub(crate) final_url: String,
    pub(crate) evidence_kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FetchedBinaryRouteError {
    InvalidRequest,
    UnsupportedMediaType,
    ArtifactRejected,
}

pub(crate) trait FetchedBinaryArtifactPort: Send + Sync {
    fn seal_fetched_binary(
        &self,
        request: &FetchedBinaryArtifactRequest,
        fetched: &FetchBody,
    ) -> Result<FetchedBinaryReference, FetchedBinaryRouteError>;
}

pub(crate) struct FetchedBinaryRouter {
    artifacts: Arc<dyn FetchedBinaryArtifactPort>,
}

impl FetchedBinaryRouter {
    pub(crate) fn new(artifacts: Arc<dyn FetchedBinaryArtifactPort>) -> Self {
        Self { artifacts }
    }

    pub(crate) fn route(
        &self,
        request: &FetchedBinaryArtifactRequest,
        fetched: &FetchBody,
    ) -> Result<FetchedBinaryReference, FetchedBinaryRouteError> {
        if request.operation_id.is_empty()
            || request.operation_id.len() > 128
            || request.creator_id.is_empty()
            || request.creator_id.len() > 128
        {
            return Err(FetchedBinaryRouteError::InvalidRequest);
        }
        if !matches!(
            fetched.media_type.as_str(),
            "application/pdf" | "image/png" | "image/jpeg"
        ) {
            return Err(FetchedBinaryRouteError::UnsupportedMediaType);
        }
        self.artifacts.seal_fetched_binary(request, fetched)
    }
}

#[cfg(test)]
#[path = "binary_artifact_tests.rs"]
mod tests;
