use super::*;
use std::sync::Mutex;

struct Artifacts {
    bytes: Vec<u8>,
    hash: String,
    sealed: Mutex<Vec<u8>>,
}

impl BrowserArtifactPort for Artifacts {
    fn read_verified(
        &self,
        _artifact_id: &str,
        _max_bytes: usize,
    ) -> Result<(String, String, String, Vec<u8>), BrowserArtifactError> {
        Ok((
            self.hash.clone(),
            "application/pdf".to_string(),
            "input.pdf".to_string(),
            self.bytes.clone(),
        ))
    }

    fn seal_browser_output(
        &self,
        _operation_id: &str,
        _display_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<BrowserArtifactReference, BrowserArtifactError> {
        if media_type != "image/png" {
            return Err(BrowserArtifactError::UnsupportedMedia);
        }
        *self
            .sealed
            .lock()
            .map_err(|_| BrowserArtifactError::StorageFailure)? = bytes.to_vec();
        Ok(BrowserArtifactReference {
            contract_version: 1,
            artifact_id: "artifact-output".to_string(),
            content_hash: hex_digest(bytes),
            size_bytes: bytes.len() as u64,
            media_type: media_type.to_string(),
        })
    }
}

#[test]
fn verified_artifact_bytes_cross_the_boundary_without_a_host_path() {
    let bytes = b"%PDF-1.7 browser upload".to_vec();
    let artifacts = Arc::new(Artifacts {
        hash: hex_digest(&bytes),
        bytes: bytes.clone(),
        sealed: Mutex::new(Vec::new()),
    });
    let bridge = BrowserArtifactBridge::new(artifacts);

    let payload = bridge
        .upload_payload("artifact-input")
        .expect("verified upload payload");

    assert_eq!(payload.artifact_id, "artifact-input");
    assert_eq!(payload.display_name, "input.pdf");
    assert_eq!(
        STANDARD.decode(payload.bytes_base64).unwrap_or_default(),
        bytes
    );
}

#[test]
fn tampered_upload_and_arbitrary_identifier_fail_before_worker_access() {
    let bytes = b"%PDF-1.7 browser upload".to_vec();
    let artifacts = Arc::new(Artifacts {
        hash: "not-the-real-hash".to_string(),
        bytes,
        sealed: Mutex::new(Vec::new()),
    });
    let bridge = BrowserArtifactBridge::new(artifacts);
    assert_eq!(
        bridge.upload_payload("artifact-input"),
        Err(BrowserArtifactError::IntegrityFailure)
    );
    assert_eq!(
        bridge.upload_payload("../host-file"),
        Err(BrowserArtifactError::InvalidRequest)
    );
}

#[test]
fn browser_output_is_decoded_bounded_and_sealed_as_an_artifact() {
    let artifacts = Arc::new(Artifacts {
        hash: hex_digest(b"input"),
        bytes: b"input".to_vec(),
        sealed: Mutex::new(Vec::new()),
    });
    let bridge = BrowserArtifactBridge::new(artifacts.clone());
    let screenshot = b"\x89PNG\r\n\x1a\nfixture";

    let reference = bridge
        .seal_download(
            "operation-1",
            "screenshot.png",
            "image/png",
            &STANDARD.encode(screenshot),
        )
        .expect("bounded browser output");

    assert_eq!(reference.artifact_id, "artifact-output");
    assert_eq!(
        artifacts.sealed.lock().expect("sealed lock").as_slice(),
        screenshot
    );
    assert_eq!(
        bridge.seal_download(
            "operation-1",
            "unsafe.exe",
            "application/octet-stream",
            "AA=="
        ),
        Err(BrowserArtifactError::UnsupportedMedia)
    );
}
