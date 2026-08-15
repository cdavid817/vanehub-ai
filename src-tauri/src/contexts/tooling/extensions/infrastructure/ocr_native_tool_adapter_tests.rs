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
use std::path::PathBuf;
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

/// One successful image OCR call, kept in one place because standing it up costs a checksum-pinned
/// install layout, a sealed source artifact, and a scripted backend.
struct OcrRun {
    response: NativeToolResultEnvelope,
    catalog: Arc<MemoryCatalog>,
    artifacts: Arc<ArtifactService>,
    operations: PathBuf,
    _root: tempfile::TempDir,
}

fn run_image_ocr() -> OcrRun {
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
    let adapter = OcrNativeToolAdapter::new(
        install,
        operations.clone(),
        Arc::new(Backend),
        artifacts.clone(),
    );
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
    OcrRun {
        response,
        catalog,
        artifacts,
        operations,
        _root: root,
    }
}

#[test]
fn image_ocr_cleans_private_bytes_and_publishes_two_linked_artifacts() {
    let run = run_image_ocr();

    assert_eq!(run.response.status, NativeToolResultStatus::Succeeded);
    assert_eq!(
        run.response.output.as_ref().expect("output")["artifacts"]
            .as_array()
            .expect("artifacts")
            .len(),
        2
    );
    assert_eq!(
        run.catalog.publications.lock().expect("publications").len(),
        2
    );
    assert_eq!(
        std::fs::read_dir(run.operations)
            .expect("operations")
            .count(),
        0
    );
}

/// OCR names the page it read, so a model can look at it rather than only at the characters OCR
/// recovered from it. An image source was rasterized by nobody -- it already is the page -- so the
/// declaration is the source itself. A rasterized source declares the page instead; see
/// `a_single_page_pdf_declares_the_rendered_page_rather_than_the_source`.
#[test]
fn a_successful_ocr_result_names_the_page_it_read() {
    let run = run_image_ocr();

    let declared = run.response.metadata[IMAGE_ARTIFACT_METADATA_KEY]
        .as_str()
        .expect("declared image");
    assert_eq!(
        json!(declared),
        run.response.metadata["source_artifact_id"],
        "the declared image is the page OCR read"
    );
    assert!(declared.starts_with("artifact-"), "{declared}");

    // An id, never bytes: this metadata is persisted on the operation record.
    let encoded = serde_json::to_string(&run.response.metadata).expect("metadata");
    assert!(!encoded.contains("base64"), "{encoded}");
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

/// A PDF source drives three sandbox launches -- inspect, render, then the OCR worker -- so the
/// scripted backend dispatches on the result file each one asks for. It writes a real PNG for the
/// rendered page, because everything downstream verifies the bytes rather than trusting the name.
struct PdfBackend;

impl PdfBackend {
    fn page_png() -> Vec<u8> {
        let mut data = Vec::new();
        image::DynamicImage::ImageRgba8(image::RgbaImage::new(24, 32))
            .write_to(
                &mut std::io::Cursor::new(&mut data),
                image::ImageFormat::Png,
            )
            .expect("encode page fixture");
        data
    }
}

impl PdfBackend {
    fn request_path(request: &SandboxLaunchRequest) -> Result<PathBuf, SandboxBackendError> {
        let index = request
            .arguments
            .iter()
            .position(|argument| argument == "--request")
            .ok_or(SandboxBackendError::SpawnFailed)?;
        request
            .arguments
            .get(index + 1)
            .map(PathBuf::from)
            .ok_or(SandboxBackendError::SpawnFailed)
    }
}

impl SandboxProcessBackend for PdfBackend {
    fn capabilities(&self) -> SandboxBackendCapabilities {
        Backend.capabilities()
    }

    fn launch(
        &self,
        request: SandboxLaunchRequest,
    ) -> Result<Box<dyn SandboxProcess>, SandboxBackendError> {
        // Every launcher here puts the result path last, and they disagree on its index.
        let result = PathBuf::from(
            request
                .arguments
                .last()
                .ok_or(SandboxBackendError::SpawnFailed)?,
        );
        let name = result
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(SandboxBackendError::SpawnFailed)?
            .to_owned();
        let protocol = "vanehub.pdfium.render.v1";
        let body = if name == "pdfium-inspect-result.json" {
            format!(
                r#"{{"protocolVersion":"{protocol}","rendererVersion":"1.0.0","action":"inspect","pageCount":8,"pages":[]}}"#
            )
        } else if name == "pdfium-render-result.json" {
            // Render exactly what was asked for -- the request names the pages, and the renderer
            // rejects a result that does not match it page for page.
            let requested: Value = serde_json::from_slice(
                &std::fs::read(Self::request_path(&request)?)
                    .map_err(|_| SandboxBackendError::SpawnFailed)?,
            )
            .map_err(|_| SandboxBackendError::SpawnFailed)?;
            let pages: Vec<u64> = requested["pageNumbers"]
                .as_array()
                .ok_or(SandboxBackendError::SpawnFailed)?
                .iter()
                .filter_map(Value::as_u64)
                .collect();
            let png = Self::page_png();
            let directory = result
                .parent()
                .ok_or(SandboxBackendError::SpawnFailed)?
                .join("rendered");
            std::fs::create_dir_all(&directory).map_err(|_| SandboxBackendError::SpawnFailed)?;
            let entries: Vec<String> = pages
                .iter()
                .map(|page| {
                    std::fs::write(directory.join(format!("page-{page}.png")), &png)
                        .map_err(|_| SandboxBackendError::SpawnFailed)?;
                    Ok(format!(
                        r#"{{"pageNumber":{page},"fileName":"page-{page}.png","width":24,"height":32,"sizeBytes":{}}}"#,
                        png.len()
                    ))
                })
                .collect::<Result<_, SandboxBackendError>>()?;
            format!(
                r#"{{"protocolVersion":"{protocol}","rendererVersion":"1.0.0","action":"render","pageCount":null,"pages":[{}]}}"#,
                entries.join(",")
            )
        } else {
            return Backend.launch(request);
        };
        std::fs::write(result, body).map_err(|_| SandboxBackendError::SpawnFailed)?;
        Ok(Box::new(Process(Some(SandboxProcessObservation {
            exit_code: 0,
            stdout: Vec::new(),
            stderr: Vec::new(),
            cpu_time_ms: Some(1),
            peak_memory_bytes: Some(1024),
        }))))
    }
}

fn run_pdf_ocr(pages: &[u32]) -> OcrRun {
    run_pdf_ocr_with(pages, Arc::new(MemoryBlobs::default()))
}

/// Seals everything except the rendered page, so the one failure this exercises is the one under
/// test rather than a broken store.
#[derive(Default)]
struct PageRefusingBlobs(MemoryBlobs);

impl ArtifactBlobPort for PageRefusingBlobs {
    fn seal_bytes(
        &self,
        operation_id: &str,
        display_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<ArtifactBlobMetadata, ArtifactBlobStoreError> {
        if display_name.starts_with("ocr-page-") {
            return Err(ArtifactBlobStoreError::StorageFailure);
        }
        self.0
            .seal_bytes(operation_id, display_name, media_type, bytes)
    }

    fn read_verified(&self, content_hash: &str) -> Result<Vec<u8>, ArtifactBlobStoreError> {
        self.0.read_verified(content_hash)
    }

    fn remove_verified(&self, content_hash: &str) -> Result<(), ArtifactBlobStoreError> {
        self.0.remove_verified(content_hash)
    }
}

fn run_pdf_ocr_with(pages: &[u32], blobs: Arc<dyn ArtifactBlobPort>) -> OcrRun {
    let root = tempfile::tempdir().expect("root");
    let install = root.path().join("install");
    let operations = root.path().join("operations");
    std::fs::create_dir_all(&install).expect("install");

    // The OCR worker half of the install, exactly as the image test lays it out.
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
    .expect("ocr manifest");

    // The renderer half, checksum-pinned the same way.
    let binary_name = if cfg!(windows) {
        "pdfium-render.exe"
    } else {
        "pdfium-render"
    };
    std::fs::write(install.join(binary_name), b"renderer").expect("renderer binary");
    let binary_hash = digest_hex(&Sha256::digest(b"renderer"));
    std::fs::write(
        install.join(".vanehub-pdfium-render.json"),
        format!(
            r#"{{"protocolVersion":"vanehub.pdfium.render.v1","rendererVersion":"1.0.0","binarySha256":"{binary_hash}"}}"#
        ),
    )
    .expect("renderer manifest");

    let catalog = Arc::new(MemoryCatalog::default());
    let artifacts = Arc::new(ArtifactService::new(blobs, catalog.clone()));
    let source = artifacts
        .create_bytes(
            ArtifactCreateRequest {
                operation_id: "upload-pdf".to_owned(),
                display_name: "source.pdf".to_owned(),
                media_type: "application/pdf".to_owned(),
                creator: ArtifactCreator {
                    kind: "user".to_owned(),
                    id: "user-1".to_owned(),
                },
                evidence_kind: ArtifactEvidenceKind::HostVerified,
                visibility: ArtifactVisibility::Private,
                source_artifact_ids: Vec::new(),
                created_at: "2026-08-15T00:00:00Z".to_owned(),
                expires_at: None,
            },
            b"%PDF-1.7 fixture",
        )
        .expect("source");
    let adapter = OcrNativeToolAdapter::new(
        install,
        operations.clone(),
        Arc::new(PdfBackend),
        artifacts.clone(),
    );
    let response = adapter.execute_ocr(NativeToolPortRequest {
        input: ValidatedNativeToolInput {
            value: json!({
                "artifact_id": source.id,
                "pages": pages,
                "languages": ["en"],
                "publish": true
            }),
            input_hash: "sha256:input".to_owned(),
            operation: NativeToolOperation::ArtifactPublish,
            resource: CanonicalToolResource {
                kind: ToolResourceKind::Artifact,
                canonical_id: "artifact/source/ocr".to_owned(),
                attributes: BTreeMap::new(),
            },
        },
        context: NativeToolExecutionContext {
            call_id: "ocr-pdf-call".to_owned(),
            session_id: "session-1".to_owned(),
            generation_id: "generation-1".to_owned(),
            agent_id: "onepiece".to_owned(),
            canonical_workspace: None,
            deadline: Instant::now() + Duration::from_secs(10),
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(Progress),
        },
    });
    OcrRun {
        response,
        catalog,
        artifacts,
        operations,
        _root: root,
    }
}

/// The point of this change: a rasterized source returns the page pdfium drew, not the PDF it was
/// drawn from. The PDF is not a reviewed image type, so declaring it -- which is what shipped
/// before -- meant the model saw characters and never the page.
#[test]
fn a_single_page_pdf_declares_the_rendered_page_rather_than_the_source() {
    let run = run_pdf_ocr(&[1]);

    assert_eq!(run.response.status, NativeToolResultStatus::Succeeded);
    let declared = run.response.metadata[IMAGE_ARTIFACT_METADATA_KEY]
        .as_str()
        .expect("declared image");
    let source = run.response.metadata["source_artifact_id"]
        .as_str()
        .expect("source");
    assert_ne!(declared, source, "the PDF source is not what OCR read");

    // Retained as evidence linked to the source, and as a type the image path reviews.
    let page = run
        .catalog
        .artifacts
        .lock()
        .expect("artifacts")
        .values()
        .find(|artifact| artifact.id == declared)
        .cloned()
        .expect("the declared page is retained");
    assert_eq!(page.media_type, "image/png");
    assert_eq!(page.display_name, "ocr-page-1.png");
    assert_eq!(page.source_artifact_ids, vec![source.to_owned()]);

    // Readable back as real PNG bytes, which is what the shared resolver reads before preparing an
    // image -- a row labelled image/png over anything else would resolve to no image at all.
    let (bytes, media_type) = run
        .artifacts
        .read_bytes(declared)
        .expect("the declared page is readable");
    assert_eq!(media_type, "image/png");
    assert!(
        bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "{} bytes",
        bytes.len()
    );

    // The sandbox is still torn down; the page survives because it was sealed before cleanup.
    assert_eq!(
        std::fs::read_dir(run.operations)
            .expect("operations")
            .count(),
        0
    );
}

/// Several pages leave nothing to name: the channel carries one identifier, and any choice among
/// them would be arbitrary. The call keeps returning its text.
#[test]
fn a_multi_page_pdf_declares_the_source_and_returns_its_text() {
    let run = run_pdf_ocr(&[1, 2]);

    assert_eq!(run.response.status, NativeToolResultStatus::Succeeded);
    assert_eq!(
        run.response.metadata[IMAGE_ARTIFACT_METADATA_KEY],
        run.response.metadata["source_artifact_id"],
        "no single page could be chosen, so the unreviewed source degrades to text"
    );
}

/// Failing to retain the page is not a failed OCR call. The text was already extracted; losing the
/// image degrades to exactly the result this tool returned before it could return one at all.
#[test]
fn a_page_that_cannot_be_retained_degrades_to_the_text_result() {
    let run = run_pdf_ocr_with(&[1], Arc::new(PageRefusingBlobs::default()));

    assert_eq!(run.response.status, NativeToolResultStatus::Succeeded);
    assert_eq!(
        run.response.metadata[IMAGE_ARTIFACT_METADATA_KEY],
        run.response.metadata["source_artifact_id"],
        "with no retained page the source is declared, and the loop finds no image on it"
    );
    assert!(
        run.response.output.as_ref().expect("output")["result"]["text"]
            .as_str()
            .is_some_and(|text| !text.is_empty())
    );

    // The sandbox is still torn down even though sealing failed.
    assert_eq!(
        std::fs::read_dir(run.operations)
            .expect("operations")
            .count(),
        0
    );
}
