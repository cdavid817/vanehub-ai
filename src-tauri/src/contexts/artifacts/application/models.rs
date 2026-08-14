#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactBlobStorePolicy {
    pub(crate) max_blob_bytes: u64,
    pub(crate) max_operation_items: u32,
    pub(crate) max_operation_bytes: u64,
    pub(crate) max_total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactBlobMetadata {
    pub(crate) contract_version: u16,
    pub(crate) content_hash: String,
    pub(crate) size_bytes: u64,
    pub(crate) media_type: String,
    pub(crate) display_name: String,
    pub(crate) storage_key: String,
    pub(crate) deduplicated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactBlobStoreError {
    InvalidOperationId,
    UnsafeDisplayName,
    UnsupportedMediaType,
    InvalidMediaContent,
    ItemQuotaExceeded,
    OperationByteQuotaExceeded,
    BlobByteQuotaExceeded,
    StoreByteQuotaExceeded,
    InvalidHash,
    IntegrityFailure,
    StorageFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactCreator {
    pub(crate) kind: String,
    pub(crate) id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactEvidenceKind {
    HostVerified,
    ProviderReported,
    UntrustedExternal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactVisibility {
    Private,
    Session,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactCreateRequest {
    pub(crate) operation_id: String,
    pub(crate) display_name: String,
    pub(crate) media_type: String,
    pub(crate) creator: ArtifactCreator,
    pub(crate) evidence_kind: ArtifactEvidenceKind,
    pub(crate) visibility: ArtifactVisibility,
    pub(crate) source_artifact_ids: Vec<String>,
    pub(crate) created_at: String,
    pub(crate) expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactDescriptor {
    pub(crate) contract_version: u16,
    pub(crate) id: String,
    pub(crate) content_hash: String,
    pub(crate) size_bytes: u64,
    pub(crate) media_type: String,
    pub(crate) display_name: String,
    pub(crate) creator: ArtifactCreator,
    pub(crate) evidence_kind: ArtifactEvidenceKind,
    pub(crate) visibility: ArtifactVisibility,
    pub(crate) source_operation_id: String,
    pub(crate) source_artifact_ids: Vec<String>,
    pub(crate) created_at: String,
    pub(crate) expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactTextPreview {
    pub(crate) contract_version: u16,
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) media_type: String,
    pub(crate) offset: u64,
    pub(crate) next_offset: Option<u64>,
    pub(crate) text: String,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactPublicationReference {
    pub(crate) contract_version: u16,
    pub(crate) reference: String,
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) visibility: ArtifactVisibility,
    pub(crate) published_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactDownloadChunk {
    pub(crate) contract_version: u16,
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) offset: u64,
    pub(crate) next_offset: Option<u64>,
    pub(crate) bytes: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactCleanupReport {
    pub(crate) contract_version: u16,
    pub(crate) removed_artifact_ids: Vec<String>,
    pub(crate) removed_blob_hashes: Vec<String>,
    pub(crate) retained_referenced: u32,
    pub(crate) integrity_failures: Vec<String>,
}
