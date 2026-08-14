use super::*;
use std::sync::Mutex;

#[derive(Debug, Default)]
struct RecordingArtifacts {
    recorded_bytes: Mutex<Vec<u8>>,
}

impl FetchedBinaryArtifactPort for RecordingArtifacts {
    fn seal_fetched_binary(
        &self,
        _request: &FetchedBinaryArtifactRequest,
        fetched: &FetchBody,
    ) -> Result<FetchedBinaryReference, FetchedBinaryRouteError> {
        *self
            .recorded_bytes
            .lock()
            .map_err(|_| FetchedBinaryRouteError::ArtifactRejected)? = fetched.bytes.clone();
        Ok(FetchedBinaryReference {
            contract_version: 1,
            artifact_id: "artifact-fixture".to_string(),
            content_hash: "abc123".to_string(),
            size_bytes: fetched.bytes.len() as u64,
            media_type: fetched.media_type.clone(),
            normalized_url: fetched.normalized_url.clone(),
            final_url: fetched.final_url.clone(),
            evidence_kind: "untrusted_external_fetch".to_string(),
        })
    }
}

fn fetched(media_type: &str, bytes: &[u8]) -> FetchBody {
    FetchBody {
        normalized_url: "https://example.com/file".to_string(),
        final_url: "https://cdn.example.com/file".to_string(),
        media_type: media_type.to_string(),
        bytes: bytes.to_vec(),
        redirect_count: 1,
    }
}

fn request() -> FetchedBinaryArtifactRequest {
    FetchedBinaryArtifactRequest {
        operation_id: "operation-1".to_string(),
        creator_id: "onepiece".to_string(),
        expires_at: None,
    }
}

#[test]
fn admitted_binary_is_returned_only_as_an_artifact_reference() {
    let artifacts = Arc::new(RecordingArtifacts::default());
    let router = FetchedBinaryRouter::new(artifacts.clone());
    let bytes = b"%PDF-1.7 fixture";

    let reference = router
        .route(&request(), &fetched("application/pdf", bytes))
        .expect("admitted fixture should route");

    assert_eq!(reference.artifact_id, "artifact-fixture");
    assert_eq!(reference.size_bytes, bytes.len() as u64);
    assert_eq!(reference.evidence_kind, "untrusted_external_fetch");
    assert_eq!(
        artifacts
            .recorded_bytes
            .lock()
            .expect("fixture lock should be available")
            .as_slice(),
        bytes
    );
}

#[test]
fn active_or_text_content_cannot_enter_the_binary_artifact_route() {
    let router = FetchedBinaryRouter::new(Arc::new(RecordingArtifacts::default()));
    assert_eq!(
        router.route(&request(), &fetched("application/javascript", b"alert(1)")),
        Err(FetchedBinaryRouteError::UnsupportedMediaType)
    );
    assert_eq!(
        router.route(&request(), &fetched("text/html", b"<p>text</p>")),
        Err(FetchedBinaryRouteError::UnsupportedMediaType)
    );
}
