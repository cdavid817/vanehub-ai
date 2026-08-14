use super::{
    ArtifactBlobMetadata, ArtifactBlobStoreError, ArtifactCleanupReport, ArtifactCreateRequest,
    ArtifactDescriptor, ArtifactDownloadChunk, ArtifactPublicationReference, ArtifactTextPreview,
    ArtifactVisibility,
};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use super::service_validation::{validate_artifact_id, validate_request};

pub(crate) trait ArtifactBlobPort: Send + Sync {
    fn seal_bytes(
        &self,
        operation_id: &str,
        display_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<ArtifactBlobMetadata, ArtifactBlobStoreError>;

    fn read_verified(&self, content_hash: &str) -> Result<Vec<u8>, ArtifactBlobStoreError>;

    #[allow(dead_code)]
    fn remove_verified(&self, content_hash: &str) -> Result<(), ArtifactBlobStoreError>;
}

pub(crate) trait ArtifactCatalogPort: Send + Sync {
    fn insert_immutable(&self, artifact: &ArtifactDescriptor) -> Result<(), ArtifactServiceError>;

    fn get(&self, artifact_id: &str) -> Result<Option<ArtifactDescriptor>, ArtifactServiceError>;

    fn list(&self, limit: usize) -> Result<Vec<ArtifactDescriptor>, ArtifactServiceError>;

    fn publish(
        &self,
        publication: &ArtifactPublicationReference,
    ) -> Result<(), ArtifactServiceError>;

    #[allow(dead_code)]
    fn expired_candidates(
        &self,
        now: &str,
        limit: usize,
    ) -> Result<Vec<(ArtifactDescriptor, bool)>, ArtifactServiceError>;

    #[allow(dead_code)]
    fn remove(&self, artifact_id: &str) -> Result<(), ArtifactServiceError>;

    #[allow(dead_code)]
    fn count_by_hash(&self, content_hash: &str) -> Result<u64, ArtifactServiceError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactServiceError {
    InvalidRequest,
    Blob(ArtifactBlobStoreError),
    CatalogFailure,
    NotFound,
    UnsupportedPreview,
    InvalidPage,
    PublicationFailure,
}

pub(crate) struct ArtifactService {
    blobs: Arc<dyn ArtifactBlobPort>,
    catalog: Arc<dyn ArtifactCatalogPort>,
}

impl ArtifactService {
    pub(crate) fn new(
        blobs: Arc<dyn ArtifactBlobPort>,
        catalog: Arc<dyn ArtifactCatalogPort>,
    ) -> Self {
        Self { blobs, catalog }
    }

    pub(crate) fn create_bytes(
        &self,
        request: ArtifactCreateRequest,
        bytes: &[u8],
    ) -> Result<ArtifactDescriptor, ArtifactServiceError> {
        validate_request(&request)?;
        let blob = self
            .blobs
            .seal_bytes(
                &request.operation_id,
                &request.display_name,
                &request.media_type,
                bytes,
            )
            .map_err(ArtifactServiceError::Blob)?;
        let artifact = ArtifactDescriptor {
            contract_version: 1,
            id: format!("artifact-{}", Uuid::new_v4()),
            content_hash: blob.content_hash,
            size_bytes: blob.size_bytes,
            media_type: blob.media_type,
            display_name: blob.display_name,
            creator: request.creator,
            evidence_kind: request.evidence_kind,
            visibility: request.visibility,
            source_operation_id: request.operation_id,
            source_artifact_ids: request.source_artifact_ids,
            created_at: request.created_at,
            expires_at: request.expires_at,
        };
        self.catalog.insert_immutable(&artifact)?;
        Ok(artifact)
    }

    pub(crate) fn create_text(
        &self,
        mut request: ArtifactCreateRequest,
        text: &str,
    ) -> Result<ArtifactDescriptor, ArtifactServiceError> {
        request.media_type = "text/plain".to_owned();
        self.create_bytes(request, text.as_bytes())
    }

    pub(crate) fn create_json(
        &self,
        mut request: ArtifactCreateRequest,
        value: &Value,
    ) -> Result<ArtifactDescriptor, ArtifactServiceError> {
        request.media_type = "application/json".to_owned();
        let bytes = serde_json::to_vec(value).map_err(|_| ArtifactServiceError::InvalidRequest)?;
        self.create_bytes(request, &bytes)
    }

    pub(crate) fn metadata(
        &self,
        artifact_id: &str,
    ) -> Result<ArtifactDescriptor, ArtifactServiceError> {
        validate_artifact_id(artifact_id)?;
        self.catalog
            .get(artifact_id)?
            .ok_or(ArtifactServiceError::NotFound)
    }

    pub(crate) fn list_metadata(
        &self,
        limit: usize,
    ) -> Result<Vec<ArtifactDescriptor>, ArtifactServiceError> {
        if !(1..=100).contains(&limit) {
            return Err(ArtifactServiceError::InvalidPage);
        }
        self.catalog.list(limit)
    }

    pub(crate) fn read_text(
        &self,
        artifact_id: &str,
        offset: u64,
        limit: usize,
    ) -> Result<ArtifactTextPreview, ArtifactServiceError> {
        if !(1..=65_536).contains(&limit) {
            return Err(ArtifactServiceError::InvalidPage);
        }
        let artifact = self.metadata(artifact_id)?;
        if !matches!(
            artifact.media_type.as_str(),
            "text/plain" | "text/markdown" | "text/csv" | "application/json"
        ) {
            return Err(ArtifactServiceError::UnsupportedPreview);
        }
        let bytes = self
            .blobs
            .read_verified(&artifact.content_hash)
            .map_err(ArtifactServiceError::Blob)?;
        let text =
            std::str::from_utf8(&bytes).map_err(|_| ArtifactServiceError::UnsupportedPreview)?;
        let start = usize::try_from(offset).map_err(|_| ArtifactServiceError::InvalidPage)?;
        if start > text.len() || !text.is_char_boundary(start) {
            return Err(ArtifactServiceError::InvalidPage);
        }
        let mut end = start.saturating_add(limit).min(text.len());
        while end > start && !text.is_char_boundary(end) {
            end -= 1;
        }
        let truncated = end < text.len();
        Ok(ArtifactTextPreview {
            contract_version: 1,
            artifact_id: artifact.id,
            content_hash: artifact.content_hash,
            media_type: artifact.media_type,
            offset,
            next_offset: truncated.then_some(end as u64),
            text: text[start..end].to_owned(),
            truncated,
        })
    }

    pub(crate) fn publish(
        &self,
        artifact_id: &str,
        visibility: ArtifactVisibility,
        published_at: &str,
    ) -> Result<ArtifactPublicationReference, ArtifactServiceError> {
        if published_at.trim().is_empty() {
            return Err(ArtifactServiceError::InvalidRequest);
        }
        let artifact = self.metadata(artifact_id)?;
        self.blobs
            .read_verified(&artifact.content_hash)
            .map_err(ArtifactServiceError::Blob)?;
        let publication = ArtifactPublicationReference {
            contract_version: 1,
            reference: format!("artifact-ref-{}", Uuid::new_v4()),
            artifact_id: artifact.id,
            content_hash: artifact.content_hash,
            visibility,
            published_at: published_at.to_owned(),
        };
        self.catalog.publish(&publication)?;
        Ok(publication)
    }

    pub(crate) fn download_chunk(
        &self,
        artifact_id: &str,
        offset: u64,
        limit: usize,
    ) -> Result<ArtifactDownloadChunk, ArtifactServiceError> {
        if !(1..=1_048_576).contains(&limit) {
            return Err(ArtifactServiceError::InvalidPage);
        }
        let artifact = self.metadata(artifact_id)?;
        let bytes = self
            .blobs
            .read_verified(&artifact.content_hash)
            .map_err(ArtifactServiceError::Blob)?;
        let start = usize::try_from(offset).map_err(|_| ArtifactServiceError::InvalidPage)?;
        if start > bytes.len() {
            return Err(ArtifactServiceError::InvalidPage);
        }
        let end = start.saturating_add(limit).min(bytes.len());
        Ok(ArtifactDownloadChunk {
            contract_version: 1,
            artifact_id: artifact.id,
            content_hash: artifact.content_hash,
            offset,
            next_offset: (end < bytes.len()).then_some(end as u64),
            bytes: bytes[start..end].to_vec(),
        })
    }

    #[allow(dead_code)]
    pub(crate) fn cleanup_expired(
        &self,
        now: &str,
        limit: usize,
    ) -> Result<ArtifactCleanupReport, ArtifactServiceError> {
        if now.trim().is_empty() || !(1..=1000).contains(&limit) {
            return Err(ArtifactServiceError::InvalidRequest);
        }
        let mut report = ArtifactCleanupReport {
            contract_version: 1,
            removed_artifact_ids: Vec::new(),
            removed_blob_hashes: Vec::new(),
            retained_referenced: 0,
            integrity_failures: Vec::new(),
        };
        for (artifact, referenced) in self.catalog.expired_candidates(now, limit)? {
            if referenced {
                report.retained_referenced = report.retained_referenced.saturating_add(1);
                continue;
            }
            self.catalog.remove(&artifact.id)?;
            report.removed_artifact_ids.push(artifact.id);
            if self.catalog.count_by_hash(&artifact.content_hash)? == 0 {
                match self.blobs.remove_verified(&artifact.content_hash) {
                    Ok(()) => report.removed_blob_hashes.push(artifact.content_hash),
                    Err(ArtifactBlobStoreError::IntegrityFailure) => {
                        report.integrity_failures.push(artifact.content_hash)
                    }
                    Err(error) => return Err(ArtifactServiceError::Blob(error)),
                }
            }
        }
        Ok(report)
    }
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
