use crate::contexts::artifacts::application::{
    ArtifactCreateRequest, ArtifactCreator, ArtifactEvidenceKind, ArtifactService,
    ArtifactServiceError, ArtifactVisibility,
};
use crate::contexts::code_execution::application::{
    CodeArtifactInputPort, CodeOutputArtifact, CodeOutputArtifactPort, CodeServiceError,
    SandboxWorkspaceError,
};
use chrono::Utc;
use std::sync::Arc;

pub(crate) struct CodeArtifactAdapter {
    service: Arc<ArtifactService>,
}

impl CodeArtifactAdapter {
    pub(crate) fn new(service: Arc<ArtifactService>) -> Self {
        Self { service }
    }
}

impl CodeArtifactInputPort for CodeArtifactAdapter {
    fn read_verified(
        &self,
        artifact_id: &str,
        max_bytes: usize,
    ) -> Result<(String, String, Vec<u8>), SandboxWorkspaceError> {
        let metadata = self
            .service
            .metadata(artifact_id)
            .map_err(map_input_error)?;
        if metadata.size_bytes > max_bytes as u64 {
            return Err(SandboxWorkspaceError::InputLimitExceeded);
        }
        let chunk = self
            .service
            .download_chunk(artifact_id, 0, max_bytes.max(1))
            .map_err(map_input_error)?;
        if chunk.next_offset.is_some() || chunk.content_hash != metadata.content_hash {
            return Err(SandboxWorkspaceError::IntegrityFailure);
        }
        Ok((metadata.content_hash, metadata.media_type, chunk.bytes))
    }
}

impl CodeOutputArtifactPort for CodeArtifactAdapter {
    fn seal_output(
        &self,
        execution_id: &str,
        relative_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<CodeOutputArtifact, CodeServiceError> {
        let artifact = self
            .service
            .create_bytes(
                ArtifactCreateRequest {
                    operation_id: execution_id.to_owned(),
                    display_name: relative_name.to_owned(),
                    media_type: media_type.to_owned(),
                    creator: ArtifactCreator {
                        kind: "code_execution".to_owned(),
                        id: execution_id.to_owned(),
                    },
                    evidence_kind: ArtifactEvidenceKind::HostVerified,
                    visibility: ArtifactVisibility::Private,
                    source_artifact_ids: Vec::new(),
                    created_at: Utc::now().to_rfc3339(),
                    expires_at: None,
                },
                bytes,
            )
            .map_err(|_| CodeServiceError::ArtifactFailure)?;
        Ok(CodeOutputArtifact {
            artifact_id: artifact.id,
            content_hash: artifact.content_hash,
            relative_name: relative_name.to_owned(),
            size_bytes: artifact.size_bytes,
            media_type: artifact.media_type,
        })
    }
}

fn map_input_error(error: ArtifactServiceError) -> SandboxWorkspaceError {
    match error {
        ArtifactServiceError::NotFound => SandboxWorkspaceError::ArtifactUnavailable,
        ArtifactServiceError::InvalidPage => SandboxWorkspaceError::InputLimitExceeded,
        ArtifactServiceError::Blob(_) => SandboxWorkspaceError::IntegrityFailure,
        ArtifactServiceError::InvalidRequest
        | ArtifactServiceError::UnsupportedPreview
        | ArtifactServiceError::CatalogFailure
        | ArtifactServiceError::PublicationFailure => SandboxWorkspaceError::ArtifactUnavailable,
    }
}
