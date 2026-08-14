use super::*;
use crate::contexts::agent_runtime::application::{
    CanonicalToolResource, NativeToolExecutionContext, NativeToolOperation, NativeToolProgressSink,
    ToolResourceKind, ValidatedNativeToolInput,
};
use crate::contexts::artifacts::application::{
    ArtifactBlobMetadata, ArtifactBlobPort, ArtifactBlobStoreError, ArtifactCatalogPort,
    ArtifactPublicationReference, ArtifactServiceError,
};
use crate::contexts::code_execution::application::{
    SandboxBackendCapabilities, SandboxBackendError, SandboxLaunchRequest, SandboxProcess,
    SandboxProcessObservation,
};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Default)]
struct MemoryBlobs(Mutex<BTreeMap<String, Vec<u8>>>);

impl ArtifactBlobPort for MemoryBlobs {
    fn seal_bytes(
        &self,
        _: &str,
        display_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<ArtifactBlobMetadata, ArtifactBlobStoreError> {
        let hash = format!("sha256:{}", digest_hex(&Sha256::digest(bytes)));
        self.0
            .lock()
            .map_err(|_| ArtifactBlobStoreError::StorageFailure)?
            .insert(hash.clone(), bytes.to_vec());
        Ok(ArtifactBlobMetadata {
            contract_version: 1,
            content_hash: hash,
            size_bytes: bytes.len() as u64,
            media_type: media_type.to_owned(),
            display_name: display_name.to_owned(),
            storage_key: "memory".to_owned(),
            deduplicated: false,
        })
    }

    fn read_verified(&self, hash: &str) -> Result<Vec<u8>, ArtifactBlobStoreError> {
        self.0
            .lock()
            .map_err(|_| ArtifactBlobStoreError::StorageFailure)?
            .get(hash)
            .cloned()
            .ok_or(ArtifactBlobStoreError::IntegrityFailure)
    }

    fn remove_verified(&self, _: &str) -> Result<(), ArtifactBlobStoreError> {
        Ok(())
    }
}

#[derive(Default)]
struct MemoryCatalog {
    artifacts: Mutex<BTreeMap<String, ArtifactDescriptor>>,
    publications: Mutex<Vec<ArtifactPublicationReference>>,
}

impl ArtifactCatalogPort for MemoryCatalog {
    fn insert_immutable(&self, artifact: &ArtifactDescriptor) -> Result<(), ArtifactServiceError> {
        self.artifacts
            .lock()
            .map_err(|_| ArtifactServiceError::CatalogFailure)?
            .insert(artifact.id.clone(), artifact.clone());
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Option<ArtifactDescriptor>, ArtifactServiceError> {
        Ok(self
            .artifacts
            .lock()
            .map_err(|_| ArtifactServiceError::CatalogFailure)?
            .get(id)
            .cloned())
    }

    fn list(&self, _: usize) -> Result<Vec<ArtifactDescriptor>, ArtifactServiceError> {
        Ok(self
            .artifacts
            .lock()
            .map_err(|_| ArtifactServiceError::CatalogFailure)?
            .values()
            .cloned()
            .collect())
    }

    fn publish(
        &self,
        publication: &ArtifactPublicationReference,
    ) -> Result<(), ArtifactServiceError> {
        self.publications
            .lock()
            .map_err(|_| ArtifactServiceError::CatalogFailure)?
            .push(publication.clone());
        Ok(())
    }

    fn expired_candidates(
        &self,
        _: &str,
        _: usize,
    ) -> Result<Vec<(ArtifactDescriptor, bool)>, ArtifactServiceError> {
        Ok(Vec::new())
    }

    fn remove(&self, _: &str) -> Result<(), ArtifactServiceError> {
        Ok(())
    }

    fn count_by_hash(&self, _: &str) -> Result<u64, ArtifactServiceError> {
        Ok(1)
    }
}

struct Backend;

impl SandboxProcessBackend for Backend {
    fn capabilities(&self) -> SandboxBackendCapabilities {
        SandboxBackendCapabilities {
            restricted_identity: true,
            job_cpu_limit: true,
            job_memory_limit: true,
            job_process_limit: true,
            kill_process_tree: true,
            acl_confinement: true,
            network_denied: true,
        }
    }

    fn launch(
        &self,
        request: SandboxLaunchRequest,
    ) -> Result<Box<dyn SandboxProcess>, SandboxBackendError> {
        let result = PathBuf::from(
            request
                .arguments
                .get(4)
                .ok_or(SandboxBackendError::SpawnFailed)?,
        );
        std::fs::write(
            result,
            format!(
                r#"{{"protocolVersion":"{}","engineName":"paddleocr","engineVersion":"3.2.0","blocks":[{{"pageNumber":1,"order":1,"text":"hello","polygon":null,"confidence":null}}],"warnings":[],"truncated":false}}"#,
                crate::contexts::tooling::extensions::application::PADDLEOCR_INFERENCE_PROTOCOL_VERSION
            ),
        )
        .map_err(|_| SandboxBackendError::SpawnFailed)?;
        Ok(Box::new(Process(Some(SandboxProcessObservation {
            exit_code: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
            cpu_time_ms: Some(1),
            peak_memory_bytes: Some(1024),
        }))))
    }
}

struct Process(Option<SandboxProcessObservation>);

impl SandboxProcess for Process {
    fn wait_until(
        &mut self,
        _: Instant,
    ) -> Result<Option<SandboxProcessObservation>, SandboxBackendError> {
        Ok(self.0.take())
    }

    fn terminate_tree(&mut self, _: Duration) -> Result<(), SandboxBackendError> {
        Ok(())
    }
}

#[derive(Debug)]
struct Progress;

impl NativeToolProgressSink for Progress {
    fn publish(&self, _: NativeToolProgress) {}
}

#[test]
fn image_ocr_cleans_private_bytes_and_publishes_two_linked_artifacts() {
    let root = tempfile::tempdir().expect("root");
    let install = root.path().join("install");
    let operations = root.path().join("operations");
    std::fs::create_dir_all(&install).expect("install");
    let interpreter = super::super::installation_adapter::venv_python(&install);
    std::fs::create_dir_all(interpreter.parent().expect("parent")).expect("scripts");
    std::fs::write(&interpreter, b"python").expect("python");
    std::fs::write(install.join("paddleocr_worker.py"), b"worker").expect("worker");
    let worker_hash = digest_hex(&Sha256::digest(b"worker"));
    std::fs::write(
        install.join(".vanehub-ocr-inference.json"),
        format!(
            r#"{{"protocolVersion":"vanehub.paddleocr.inference.v1","engineVersion":"3.2.0","workerSha256":"{worker_hash}"}}"#
        ),
    )
    .expect("manifest");
    let blobs = Arc::new(MemoryBlobs::default());
    let catalog = Arc::new(MemoryCatalog::default());
    let artifacts = Arc::new(ArtifactService::new(blobs, catalog.clone()));
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&16_u32.to_be_bytes());
    png.extend_from_slice(&8_u32.to_be_bytes());
    let source = artifacts
        .create_bytes(
            ArtifactCreateRequest {
                operation_id: "upload-1".to_owned(),
                display_name: "source.png".to_owned(),
                media_type: "image/png".to_owned(),
                creator: ArtifactCreator {
                    kind: "user".to_owned(),
                    id: "user-1".to_owned(),
                },
                evidence_kind: ArtifactEvidenceKind::HostVerified,
                visibility: ArtifactVisibility::Private,
                source_artifact_ids: Vec::new(),
                created_at: "2026-08-13T00:00:00Z".to_owned(),
                expires_at: None,
            },
            &png,
        )
        .expect("source");
    let adapter =
        OcrNativeToolAdapter::new(install, operations.clone(), Arc::new(Backend), artifacts);
    let input = json!({
        "artifact_id": source.id,
        "languages": ["en"],
        "publish": true
    });
    let response = adapter.execute_ocr(NativeToolPortRequest {
        input: ValidatedNativeToolInput {
            value: input,
            input_hash: "sha256:input".to_owned(),
            operation: NativeToolOperation::ArtifactPublish,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::Artifact,
                canonical_id: "artifact/source/ocr".to_owned(),
                attributes: BTreeMap::new(),
            },
        },
        context: NativeToolExecutionContext {
            call_id: "ocr-call-1".to_owned(),
            session_id: "session-1".to_owned(),
            generation_id: "generation-1".to_owned(),
            agent_id: "onepiece".to_owned(),
            canonical_workspace: None,
            deadline: Instant::now() + Duration::from_secs(5),
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Progress),
        },
    });
    assert_eq!(response.status, NativeToolResultStatus::Succeeded);
    assert_eq!(
        response.output.as_ref().expect("output")["artifacts"]
            .as_array()
            .expect("artifacts")
            .len(),
        2
    );
    assert_eq!(catalog.publications.lock().expect("publications").len(), 2);
    assert_eq!(
        std::fs::read_dir(operations).expect("operations").count(),
        0
    );
}

fn digest_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}
