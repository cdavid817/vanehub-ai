use crate::contexts::artifacts::application::{
    ArtifactCreateRequest, ArtifactCreator, ArtifactEvidenceKind, ArtifactService,
    ArtifactServiceError, ArtifactVisibility,
};
use crate::contexts::browser_automation::application::{
    BrowserArtifactError, BrowserArtifactPort, BrowserArtifactReference,
};
use chrono::Utc;
use std::sync::Arc;

pub(crate) struct BrowserArtifactAdapter {
    service: Arc<ArtifactService>,
}

impl BrowserArtifactAdapter {
    pub(crate) fn new(service: Arc<ArtifactService>) -> Self {
        Self { service }
    }
}

impl BrowserArtifactPort for BrowserArtifactAdapter {
    fn read_verified(
        &self,
        artifact_id: &str,
        max_bytes: usize,
    ) -> Result<(String, String, String, Vec<u8>), BrowserArtifactError> {
        let metadata = self
            .service
            .metadata(artifact_id)
            .map_err(map_service_error)?;
        if metadata.size_bytes > max_bytes as u64 {
            return Err(BrowserArtifactError::TooLarge);
        }
        let chunk = self
            .service
            .download_chunk(artifact_id, 0, max_bytes.max(1))
            .map_err(map_service_error)?;
        if chunk.next_offset.is_some() || chunk.content_hash != metadata.content_hash {
            return Err(BrowserArtifactError::IntegrityFailure);
        }
        Ok((
            metadata.content_hash,
            metadata.media_type,
            metadata.display_name,
            chunk.bytes,
        ))
    }

    fn seal_browser_output(
        &self,
        operation_id: &str,
        display_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<BrowserArtifactReference, BrowserArtifactError> {
        let artifact = self
            .service
            .create_bytes(
                ArtifactCreateRequest {
                    operation_id: operation_id.to_owned(),
                    display_name: display_name.to_owned(),
                    media_type: media_type.to_owned(),
                    creator: ArtifactCreator {
                        kind: "browser".to_owned(),
                        id: "onepiece".to_owned(),
                    },
                    evidence_kind: ArtifactEvidenceKind::HostVerified,
                    visibility: ArtifactVisibility::Private,
                    source_artifact_ids: Vec::new(),
                    created_at: Utc::now().to_rfc3339(),
                    expires_at: None,
                },
                bytes,
            )
            .map_err(map_service_error)?;
        Ok(BrowserArtifactReference {
            contract_version: artifact.contract_version,
            artifact_id: artifact.id,
            content_hash: artifact.content_hash,
            size_bytes: artifact.size_bytes,
            media_type: artifact.media_type,
        })
    }
}

fn map_service_error(error: ArtifactServiceError) -> BrowserArtifactError {
    match error {
        ArtifactServiceError::InvalidRequest | ArtifactServiceError::InvalidPage => {
            BrowserArtifactError::InvalidRequest
        }
        ArtifactServiceError::NotFound => BrowserArtifactError::NotFound,
        ArtifactServiceError::UnsupportedPreview => BrowserArtifactError::UnsupportedMedia,
        ArtifactServiceError::Blob(_) => BrowserArtifactError::IntegrityFailure,
        ArtifactServiceError::CatalogFailure | ArtifactServiceError::PublicationFailure => {
            BrowserArtifactError::StorageFailure
        }
    }
}
