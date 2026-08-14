use super::*;
use crate::contexts::artifacts::application::{
    ArtifactCreator, ArtifactEvidenceKind, ArtifactVisibility,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Default)]
struct Blobs(Mutex<BTreeMap<String, Vec<u8>>>);

impl ArtifactBlobPort for Blobs {
    fn seal_bytes(
        &self,
        _operation_id: &str,
        display_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<ArtifactBlobMetadata, ArtifactBlobStoreError> {
        let hash = Sha256::digest(bytes)
            .iter()
            .fold(String::from("sha256:"), |mut hash, byte| {
                use std::fmt::Write as _;
                write!(hash, "{byte:02x}").expect("string formatting");
                hash
            });
        let mut blobs = self.0.lock().expect("blobs");
        let deduplicated = blobs.contains_key(&hash);
        blobs.entry(hash.clone()).or_insert_with(|| bytes.to_vec());
        Ok(ArtifactBlobMetadata {
            contract_version: 1,
            content_hash: hash.clone(),
            size_bytes: bytes.len() as u64,
            media_type: media_type.to_owned(),
            display_name: display_name.to_owned(),
            storage_key: hash,
            deduplicated,
        })
    }

    fn read_verified(&self, content_hash: &str) -> Result<Vec<u8>, ArtifactBlobStoreError> {
        self.0
            .lock()
            .expect("blobs")
            .get(content_hash)
            .cloned()
            .ok_or(ArtifactBlobStoreError::IntegrityFailure)
    }

    fn remove_verified(&self, content_hash: &str) -> Result<(), ArtifactBlobStoreError> {
        self.0.lock().expect("blobs").remove(content_hash);
        Ok(())
    }
}

#[derive(Default)]
struct Catalog {
    records: Mutex<Vec<ArtifactDescriptor>>,
    publications: Mutex<Vec<ArtifactPublicationReference>>,
    referenced_ids: Mutex<Vec<String>>,
}

impl ArtifactCatalogPort for Catalog {
    fn insert_immutable(&self, artifact: &ArtifactDescriptor) -> Result<(), ArtifactServiceError> {
        let mut records = self.records.lock().expect("records");
        if records.iter().any(|record| record.id == artifact.id) {
            return Err(ArtifactServiceError::CatalogFailure);
        }
        records.push(artifact.clone());
        Ok(())
    }

    fn get(&self, artifact_id: &str) -> Result<Option<ArtifactDescriptor>, ArtifactServiceError> {
        Ok(self
            .records
            .lock()
            .expect("records")
            .iter()
            .find(|record| record.id == artifact_id)
            .cloned())
    }

    fn list(&self, limit: usize) -> Result<Vec<ArtifactDescriptor>, ArtifactServiceError> {
        Ok(self
            .records
            .lock()
            .expect("records")
            .iter()
            .take(limit)
            .cloned()
            .collect())
    }

    fn publish(
        &self,
        publication: &ArtifactPublicationReference,
    ) -> Result<(), ArtifactServiceError> {
        self.publications
            .lock()
            .expect("publications")
            .push(publication.clone());
        Ok(())
    }

    fn expired_candidates(
        &self,
        now: &str,
        limit: usize,
    ) -> Result<Vec<(ArtifactDescriptor, bool)>, ArtifactServiceError> {
        let referenced = self.referenced_ids.lock().expect("references");
        Ok(self
            .records
            .lock()
            .expect("records")
            .iter()
            .filter(|record| {
                record
                    .expires_at
                    .as_deref()
                    .is_some_and(|expiry| expiry <= now)
            })
            .take(limit)
            .map(|record| (record.clone(), referenced.contains(&record.id)))
            .collect())
    }

    fn remove(&self, artifact_id: &str) -> Result<(), ArtifactServiceError> {
        self.records
            .lock()
            .expect("records")
            .retain(|record| record.id != artifact_id);
        Ok(())
    }

    fn count_by_hash(&self, content_hash: &str) -> Result<u64, ArtifactServiceError> {
        Ok(self
            .records
            .lock()
            .expect("records")
            .iter()
            .filter(|record| record.content_hash == content_hash)
            .count() as u64)
    }
}

fn request(operation_id: &str, sources: Vec<String>) -> ArtifactCreateRequest {
    ArtifactCreateRequest {
        operation_id: operation_id.to_owned(),
        display_name: "result.json".to_owned(),
        media_type: "application/json".to_owned(),
        creator: ArtifactCreator {
            kind: "code_execution".to_owned(),
            id: "run-1".to_owned(),
        },
        evidence_kind: ArtifactEvidenceKind::HostVerified,
        visibility: ArtifactVisibility::Private,
        source_artifact_ids: sources,
        created_at: "100".to_owned(),
        expires_at: None,
    }
}

fn service(catalog: Arc<Catalog>) -> ArtifactService {
    ArtifactService::new(Arc::new(Blobs::default()), catalog)
}

#[test]
fn logical_artifacts_keep_distinct_identity_and_provenance_when_blob_is_deduplicated() {
    let catalog = Arc::new(Catalog::default());
    let service = service(catalog.clone());
    let first = service
        .create_json(request("operation-1", Vec::new()), &json!({"ok": true}))
        .expect("first");
    let second = service
        .create_json(
            request("operation-2", vec![first.id.clone()]),
            &json!({"ok": true}),
        )
        .expect("second");

    assert_ne!(first.id, second.id);
    assert_eq!(first.content_hash, second.content_hash);
    assert_eq!(second.source_operation_id, "operation-2");
    assert_eq!(second.source_artifact_ids, vec![first.id]);
    assert_eq!(catalog.records.lock().expect("records").len(), 2);
}

#[test]
fn bounded_text_creation_and_lineage_validation_are_shared() {
    let catalog = Arc::new(Catalog::default());
    let service = service(catalog);
    let text = service
        .create_text(request("operation-1", Vec::new()), "bounded output")
        .expect("text");
    assert_eq!(text.media_type, "text/plain");

    let duplicate_source = request(
        "operation-2",
        vec!["artifact-1".to_owned(), "artifact-1".to_owned()],
    );
    assert_eq!(
        service.create_text(duplicate_source, "invalid"),
        Err(ArtifactServiceError::InvalidRequest)
    );
}

#[test]
fn metadata_and_text_preview_are_bounded_and_never_return_storage_paths() {
    let catalog = Arc::new(Catalog::default());
    let service = service(catalog);
    let artifact = service
        .create_text(request("operation-1", Vec::new()), "alpha-beta-gamma")
        .expect("artifact");
    let first = service.read_text(&artifact.id, 0, 6).expect("first page");
    let second = service
        .read_text(&artifact.id, first.next_offset.expect("next"), 64)
        .expect("second page");

    assert_eq!(first.text, "alpha-");
    assert!(first.truncated);
    assert_eq!(second.text, "beta-gamma");
    assert!(!second.truncated);
    assert_eq!(service.metadata(&artifact.id).expect("metadata"), artifact);
    assert_eq!(service.list_metadata(1).expect("list").len(), 1);
}

#[test]
fn publication_is_application_owned_and_download_is_integrity_checked_and_chunked() {
    let catalog = Arc::new(Catalog::default());
    let service = service(catalog.clone());
    let artifact = service
        .create_text(request("operation-1", Vec::new()), "download-body")
        .expect("artifact");
    let publication = service
        .publish(&artifact.id, ArtifactVisibility::Session, "101")
        .expect("publication");
    let first = service.download_chunk(&artifact.id, 0, 4).expect("chunk");
    let second = service
        .download_chunk(&artifact.id, first.next_offset.expect("next"), 1024)
        .expect("rest");

    assert!(publication.reference.starts_with("artifact-ref-"));
    assert!(!publication.reference.contains("http"));
    assert_eq!(publication.content_hash, artifact.content_hash);
    assert_eq!(first.bytes, b"down");
    assert_eq!(second.bytes, b"load-body");
    assert!(second.next_offset.is_none());
    assert_eq!(catalog.publications.lock().expect("publications").len(), 1);
}

#[test]
fn retention_keeps_referenced_artifacts_and_removes_unreferenced_blobs_once() {
    let catalog = Arc::new(Catalog::default());
    let service = service(catalog.clone());
    let mut first_request = request("operation-1", Vec::new());
    first_request.expires_at = Some("200".to_owned());
    let first = service.create_text(first_request, "same").expect("first");
    let mut second_request = request("operation-2", Vec::new());
    second_request.expires_at = Some("200".to_owned());
    let second = service.create_text(second_request, "same").expect("second");
    catalog
        .referenced_ids
        .lock()
        .expect("references")
        .push(second.id.clone());

    let first_cleanup = service.cleanup_expired("201", 10).expect("cleanup");
    assert_eq!(first_cleanup.removed_artifact_ids, vec![first.id]);
    assert!(first_cleanup.removed_blob_hashes.is_empty());
    assert_eq!(first_cleanup.retained_referenced, 1);
    assert!(service.download_chunk(&second.id, 0, 10).is_ok());

    catalog.referenced_ids.lock().expect("references").clear();
    let second_cleanup = service.cleanup_expired("201", 10).expect("cleanup");
    assert_eq!(second_cleanup.removed_artifact_ids, vec![second.id]);
    assert_eq!(second_cleanup.removed_blob_hashes.len(), 1);
}
