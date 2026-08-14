use crate::contexts::artifacts::application::{
    ArtifactCreateRequest, ArtifactCreator, ArtifactEvidenceKind, ArtifactService,
    ArtifactVisibility,
};
use crate::contexts::web_research::application::{
    FetchBody, FetchedBinaryArtifactPort, FetchedBinaryArtifactRequest, FetchedBinaryReference,
    FetchedBinaryRouteError,
};
use chrono::Utc;
use std::sync::Arc;

pub(crate) struct ArtifactFetchedBinaryAdapter {
    artifacts: Arc<ArtifactService>,
}

impl ArtifactFetchedBinaryAdapter {
    pub(crate) fn new(artifacts: Arc<ArtifactService>) -> Self {
        Self { artifacts }
    }
}

impl FetchedBinaryArtifactPort for ArtifactFetchedBinaryAdapter {
    fn seal_fetched_binary(
        &self,
        request: &FetchedBinaryArtifactRequest,
        fetched: &FetchBody,
    ) -> Result<FetchedBinaryReference, FetchedBinaryRouteError> {
        let descriptor = self
            .artifacts
            .create_bytes(
                ArtifactCreateRequest {
                    operation_id: request.operation_id.clone(),
                    display_name: display_name(&fetched.media_type)?.to_string(),
                    media_type: fetched.media_type.clone(),
                    creator: ArtifactCreator {
                        kind: "agent".to_string(),
                        id: request.creator_id.clone(),
                    },
                    evidence_kind: ArtifactEvidenceKind::UntrustedExternal,
                    visibility: ArtifactVisibility::Session,
                    source_artifact_ids: vec![],
                    created_at: Utc::now().to_rfc3339(),
                    expires_at: request.expires_at.clone(),
                },
                &fetched.bytes,
            )
            .map_err(|_| FetchedBinaryRouteError::ArtifactRejected)?;
        Ok(FetchedBinaryReference {
            contract_version: 1,
            artifact_id: descriptor.id,
            content_hash: descriptor.content_hash,
            size_bytes: descriptor.size_bytes,
            media_type: descriptor.media_type,
            normalized_url: fetched.normalized_url.clone(),
            final_url: fetched.final_url.clone(),
            evidence_kind: "untrusted_external_fetch".to_string(),
        })
    }
}

fn display_name(media_type: &str) -> Result<&'static str, FetchedBinaryRouteError> {
    match media_type {
        "application/pdf" => Ok("fetched-document.pdf"),
        "image/png" => Ok("fetched-image.png"),
        "image/jpeg" => Ok("fetched-image.jpg"),
        _ => Err(FetchedBinaryRouteError::UnsupportedMediaType),
    }
}
