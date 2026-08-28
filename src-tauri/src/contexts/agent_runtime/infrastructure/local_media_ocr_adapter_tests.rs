use super::*;
use crate::contexts::artifacts::application::{
    ArtifactBlobMetadata, ArtifactBlobPort, ArtifactBlobStoreError, ArtifactCatalogPort,
    ArtifactPublicationReference, ArtifactServiceError,
};
use crate::contexts::local_media::api::OcrLine;

/// A blob store whose bytes are whatever the test says, independent of what the catalog declares.
///
/// That divergence is the whole point: a truncated blob, an artifact rewritten between the metadata
/// read and the last chunk, and a storage bug all present as a length the descriptor disagrees
/// with, and the adapter has to refuse before OCR sees anything.
struct StubBlobs {
    bytes: Vec<u8>,
}

impl ArtifactBlobPort for StubBlobs {
    fn seal_bytes(
        &self,
        _operation_id: &str,
        _display_name: &str,
        _media_type: &str,
        _bytes: &[u8],
    ) -> Result<ArtifactBlobMetadata, ArtifactBlobStoreError> {
        Err(ArtifactBlobStoreError::StorageFailure)
    }

    fn read_verified(&self, _content_hash: &str) -> Result<Vec<u8>, ArtifactBlobStoreError> {
        Ok(self.bytes.clone())
    }

    fn remove_verified(&self, _content_hash: &str) -> Result<(), ArtifactBlobStoreError> {
        Ok(())
    }
}

struct StubCatalog {
    descriptor: ArtifactDescriptor,
}

impl ArtifactCatalogPort for StubCatalog {
    fn insert_immutable(&self, _artifact: &ArtifactDescriptor) -> Result<(), ArtifactServiceError> {
        Err(ArtifactServiceError::CatalogFailure)
    }

    fn get(&self, artifact_id: &str) -> Result<Option<ArtifactDescriptor>, ArtifactServiceError> {
        Ok((artifact_id == self.descriptor.id).then(|| self.descriptor.clone()))
    }

    fn list(&self, _limit: usize) -> Result<Vec<ArtifactDescriptor>, ArtifactServiceError> {
        Ok(vec![self.descriptor.clone()])
    }

    fn publish(
        &self,
        _publication: &ArtifactPublicationReference,
    ) -> Result<(), ArtifactServiceError> {
        Err(ArtifactServiceError::PublicationFailure)
    }

    fn expired_candidates(
        &self,
        _now: &str,
        _limit: usize,
    ) -> Result<Vec<(ArtifactDescriptor, bool)>, ArtifactServiceError> {
        Ok(Vec::new())
    }

    fn remove(&self, _artifact_id: &str) -> Result<(), ArtifactServiceError> {
        Ok(())
    }

    fn count_by_hash(&self, _content_hash: &str) -> Result<u64, ArtifactServiceError> {
        Ok(1)
    }
}

fn descriptor(size_bytes: u64) -> ArtifactDescriptor {
    ArtifactDescriptor {
        contract_version: 1,
        id: "artifact-1".to_owned(),
        content_hash: "sha256:".to_owned() + &"a".repeat(64),
        size_bytes,
        media_type: "image/png".to_owned(),
        display_name: "page.png".to_owned(),
        creator: ArtifactCreator {
            kind: "native_tool".to_owned(),
            id: "agent-1".to_owned(),
        },
        evidence_kind: ArtifactEvidenceKind::HostVerified,
        visibility: ArtifactVisibility::Session,
        source_operation_id: "call-1".to_owned(),
        source_artifact_ids: Vec::new(),
        created_at: "2026-08-23T00:00:00Z".to_owned(),
        expires_at: None,
    }
}

/// An `ArtifactService` whose catalog declares `declared` bytes and whose blob store holds `stored`.
fn artifacts(declared: u64, stored: usize) -> ArtifactService {
    ArtifactService::new(
        Arc::new(StubBlobs {
            bytes: vec![0x41; stored],
        }),
        Arc::new(StubCatalog {
            descriptor: descriptor(declared),
        }),
    )
}

#[test]
fn an_artifact_whose_bytes_match_its_declared_length_is_admitted() {
    let service = artifacts(2_048, 2_048);

    let (found, bytes) = read_admitted_artifact(&service, "artifact-1").expect("admitted");

    assert_eq!(found.size_bytes, 2_048);
    assert_eq!(bytes.len(), 2_048);
}

#[test]
fn a_multi_chunk_artifact_that_matches_is_admitted() {
    // Past the 1 MiB chunk limit, so the loop really iterates.
    let service = artifacts(2_500_000, 2_500_000);

    let (_, bytes) = read_admitted_artifact(&service, "artifact-1").expect("admitted");

    assert_eq!(bytes.len(), 2_500_000);
}

#[test]
fn an_artifact_that_ends_short_of_its_declared_length_is_refused() {
    // A truncated PNG still sniffs as a PNG and its header dimensions still parse, so nothing
    // downstream would notice.
    let service = artifacts(2_048, 1_024);

    assert_eq!(
        read_admitted_artifact(&service, "artifact-1").map(|_| ()),
        Err(OcrToolError::Admission)
    );
}

#[test]
fn an_artifact_that_ends_short_after_several_chunks_is_refused() {
    let service = artifacts(3_000_000, 2_500_000);

    assert_eq!(
        read_admitted_artifact(&service, "artifact-1").map(|_| ()),
        Err(OcrToolError::Admission)
    );
}

#[test]
fn an_artifact_carrying_more_bytes_than_it_declared_is_refused() {
    let service = artifacts(1_024, 2_048);

    assert_eq!(
        read_admitted_artifact(&service, "artifact-1").map(|_| ()),
        Err(OcrToolError::Admission)
    );
}

#[test]
fn an_empty_blob_behind_a_non_empty_descriptor_is_refused() {
    let service = artifacts(2_048, 0);

    assert_eq!(
        read_admitted_artifact(&service, "artifact-1").map(|_| ()),
        Err(OcrToolError::Admission)
    );
}

#[test]
fn a_refused_length_reports_integrity_rather_than_an_engine_failure() {
    let service = artifacts(2_048, 1_024);
    let error = read_admitted_artifact(&service, "artifact-1")
        .map(|_| ())
        .expect_err("refused");

    // The stable contract the Agent's retry policy branches on. A length mismatch is the source
    // artifact failing verification, not the OCR engine failing.
    let envelope = error.envelope();
    assert_eq!(envelope.status, NativeToolResultStatus::Failed);
    assert_eq!(
        envelope.error_code,
        Some(NativeToolErrorCode::IntegrityFailure)
    );
    // No result, so nothing can carry provenance for a document that was never read.
    assert!(envelope.output.is_none());
}

fn page(number: u32, lines: Vec<OcrLine>) -> OcrPage {
    OcrPage {
        page_number: number,
        text: lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        line_count: lines.len() as u32,
        lines,
    }
}

fn line(text: &str, confidence: Option<f32>) -> OcrLine {
    OcrLine {
        text: text.to_owned(),
        confidence,
        polygon: None,
    }
}

#[test]
fn pages_flatten_into_blocks_numbered_from_one_per_page() {
    let blocks = blocks_from_pages(&[
        page(1, vec![line("alpha", Some(0.9)), line("beta", None)]),
        page(2, vec![line("gamma", Some(0.4))]),
    ]);

    assert_eq!(blocks.len(), 3);
    assert_eq!((blocks[0].page_number, blocks[0].order), (1, 1));
    assert_eq!((blocks[1].page_number, blocks[1].order), (1, 2));
    assert_eq!((blocks[2].page_number, blocks[2].order), (2, 1));
    assert_eq!(blocks[0].confidence, Some(0.9));
    assert_eq!(blocks[1].confidence, None);
}

#[test]
fn geometry_is_carried_across_the_shared_boundary() {
    let positioned = OcrLine {
        text: "alpha".to_owned(),
        confidence: None,
        polygon: Some(vec![(0.0, 0.0), (8.0, 0.0), (8.0, 3.0), (0.0, 3.0)]),
    };
    let blocks = blocks_from_pages(&[page(1, vec![positioned])]);
    let polygon = blocks[0].polygon.as_ref().expect("polygon");
    assert_eq!(polygon.len(), 4);
    assert_eq!((polygon[1].x, polygon[1].y), (8.0, 0.0));
}

#[test]
fn a_page_with_no_lines_contributes_no_blocks() {
    assert!(blocks_from_pages(&[page(1, Vec::new())]).is_empty());
    assert!(blocks_from_pages(&[]).is_empty());
}

#[test]
fn the_tool_input_requires_an_artifact_id_and_languages() {
    let valid = serde_json::json!({"artifact_id": "artifact-1", "languages": ["en"]});
    let parsed = parse_input(&valid).expect("valid input");
    assert_eq!(parsed.artifact_id, "artifact-1");
    assert_eq!(parsed.languages, vec!["en".to_owned()]);
    assert!(!parsed.publish);

    for invalid in [
        serde_json::json!({"languages": ["en"]}),
        serde_json::json!({"artifact_id": "artifact-1"}),
        serde_json::json!({"artifact_id": 7, "languages": ["en"]}),
        serde_json::json!({"artifact_id": "artifact-1", "languages": [7]}),
        serde_json::json!("not an object"),
    ] {
        assert_eq!(
            parse_input(&invalid).err(),
            Some(OcrToolError::InvalidInput)
        );
    }
}

#[test]
fn a_host_path_cannot_be_supplied_through_the_tool_schema() {
    // Sharing the composer's runtime must not widen this input. A `path` field is simply not read,
    // so a caller that supplies one gets OCR of the artifact they named -- or nothing.
    let hostile = serde_json::json!({
        "path": "/etc/passwd",
        "source_path": "C:\\Users\\someone\\secrets.png",
        "languages": ["en"],
    });
    assert_eq!(
        parse_input(&hostile).err(),
        Some(OcrToolError::InvalidInput)
    );

    let with_extra = serde_json::json!({
        "artifact_id": "artifact-1",
        "languages": ["en"],
        "path": "/etc/passwd",
    });
    let parsed = parse_input(&with_extra).expect("artifact input");
    assert_eq!(parsed.artifact_id, "artifact-1");
}

#[test]
fn publish_defaults_to_false() {
    let parsed = parse_input(&serde_json::json!({
        "artifact_id": "artifact-1",
        "languages": ["en"]
    }))
    .expect("input");
    assert!(!parsed.publish);

    let explicit = parse_input(&serde_json::json!({
        "artifact_id": "artifact-1",
        "languages": ["en"],
        "publish": true
    }))
    .expect("input");
    assert!(explicit.publish);
}

#[test]
fn cancellation_and_limits_keep_their_own_envelope_status() {
    // A cancelled call must not read as a failure, and a deadline must not read as a crash: the
    // Agent's retry policy branches on these.
    assert_eq!(
        OcrToolError::Cancelled.envelope().status,
        NativeToolResultStatus::Cancelled
    );
    assert_eq!(
        OcrToolError::Limit.envelope().status,
        NativeToolResultStatus::LimitExceeded
    );
    assert_eq!(
        OcrToolError::Admission.envelope().error_code,
        Some(NativeToolErrorCode::IntegrityFailure)
    );
    assert_eq!(
        OcrToolError::InvalidInput.envelope().error_code,
        Some(NativeToolErrorCode::InvalidInput)
    );
    assert_eq!(
        OcrToolError::Execution.envelope().error_code,
        Some(NativeToolErrorCode::ExternalFailure)
    );
}

#[test]
fn an_error_envelope_carries_no_engine_detail() {
    for error in [
        OcrToolError::Execution,
        OcrToolError::Artifact,
        OcrToolError::Protocol,
        OcrToolError::Admission,
    ] {
        let envelope = error.envelope();
        let message = envelope.safe_error.unwrap_or_default();
        assert!(!message.contains('/'), "{message} leaks a path fragment");
        assert!(!message.contains('\\'), "{message} leaks a path fragment");
        assert!(envelope.output.is_none());
        assert!(envelope.metadata.is_empty());
    }
}
