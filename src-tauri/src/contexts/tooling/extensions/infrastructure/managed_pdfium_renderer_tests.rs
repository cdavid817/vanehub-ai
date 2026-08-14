use super::*;
use crate::contexts::code_execution::application::{
    SandboxBackendCapabilities, SandboxBackendError, SandboxLaunchRequest, SandboxProcess,
    SandboxProcessObservation,
};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicUsize;

#[derive(Clone, Copy)]
enum ResponseMode {
    Valid,
    WrongProtocol,
    Hanging,
}

struct Backend {
    mode: ResponseMode,
    launches: Arc<AtomicUsize>,
    cancellation: Option<Arc<AtomicBool>>,
    terminated: Arc<AtomicBool>,
}

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
        self.launches.fetch_add(1, Ordering::AcqRel);
        assert_eq!(
            request.environment.get("NO_PROXY").map(String::as_str),
            Some("*")
        );
        assert_eq!(request.limits.process_count, 1);
        assert_eq!(request.arguments[0], "--protocol");
        let request_path = PathBuf::from(&request.arguments[3]);
        let result_path = PathBuf::from(&request.arguments[5]);
        let value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(&request_path).map_err(|_| SandboxBackendError::SpawnFailed)?,
        )
        .map_err(|_| SandboxBackendError::SpawnFailed)?;
        if !matches!(self.mode, ResponseMode::Hanging) {
            write_response(self.mode, &value, &result_path)?;
        }
        Ok(Box::new(Process {
            observation: (!matches!(self.mode, ResponseMode::Hanging)).then_some(
                SandboxProcessObservation {
                    exit_code: 0,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    cpu_time_ms: Some(1),
                    peak_memory_bytes: Some(1024),
                },
            ),
            cancellation: self.cancellation.clone(),
            terminated: self.terminated.clone(),
        }))
    }
}

struct Process {
    observation: Option<SandboxProcessObservation>,
    cancellation: Option<Arc<AtomicBool>>,
    terminated: Arc<AtomicBool>,
}

impl SandboxProcess for Process {
    fn wait_until(
        &mut self,
        _: Instant,
    ) -> Result<Option<SandboxProcessObservation>, SandboxBackendError> {
        if let Some(cancellation) = &self.cancellation {
            cancellation.store(true, Ordering::Release);
        }
        Ok(self.observation.take())
    }

    fn terminate_tree(&mut self, _: Duration) -> Result<(), SandboxBackendError> {
        self.terminated.store(true, Ordering::Release);
        Ok(())
    }
}

fn write_response(
    mode: ResponseMode,
    request: &serde_json::Value,
    result_path: &Path,
) -> Result<(), SandboxBackendError> {
    let protocol = if matches!(mode, ResponseMode::WrongProtocol) {
        "vanehub.pdfium.render.v2"
    } else {
        PROTOCOL_VERSION
    };
    let action = request["action"]
        .as_str()
        .ok_or(SandboxBackendError::SpawnFailed)?;
    let value = if action == "inspect" {
        serde_json::json!({
            "protocolVersion": protocol,
            "rendererVersion": "1.0.0",
            "action": "inspect",
            "pageCount": 3,
            "pages": []
        })
    } else {
        let output = request["outputDirectory"]
            .as_str()
            .map(PathBuf::from)
            .ok_or(SandboxBackendError::SpawnFailed)?;
        let page = request["pageNumbers"][0]
            .as_u64()
            .ok_or(SandboxBackendError::SpawnFailed)? as u32;
        let png = test_png(16, 8);
        std::fs::write(output.join(format!("page-{page}.png")), &png)
            .map_err(|_| SandboxBackendError::SpawnFailed)?;
        serde_json::json!({
            "protocolVersion": protocol,
            "rendererVersion": "1.0.0",
            "action": "render",
            "pageCount": null,
            "pages": [{
                "pageNumber": page,
                "fileName": format!("page-{page}.png"),
                "width": 16,
                "height": 8,
                "sizeBytes": png.len()
            }]
        })
    };
    std::fs::write(
        result_path,
        serde_json::to_vec(&value).map_err(|_| SandboxBackendError::SpawnFailed)?,
    )
    .map_err(|_| SandboxBackendError::SpawnFailed)
}

fn fixture(
    mode: ResponseMode,
    cancellation: Option<Arc<AtomicBool>>,
) -> (
    tempfile::TempDir,
    ManagedPdfiumRenderer,
    PathBuf,
    Arc<AtomicUsize>,
    Arc<AtomicBool>,
) {
    let root = tempfile::tempdir().expect("root");
    let install = root.path().join("install");
    let execution = root.path().join("execution");
    std::fs::create_dir(&install).expect("install");
    std::fs::create_dir(&execution).expect("execution");
    std::fs::create_dir(execution.join("inputs")).expect("inputs");
    let source = execution.join("inputs/source.pdf");
    std::fs::write(&source, b"%PDF-1.7\n").expect("pdf");
    let binary_name = if cfg!(windows) {
        "pdfium-render.exe"
    } else {
        "pdfium-render"
    };
    let binary = install.join(binary_name);
    std::fs::write(&binary, b"reviewed-pdfium-binary").expect("binary");
    let hash = digest_hex(&Sha256::digest(b"reviewed-pdfium-binary"));
    std::fs::write(
        install.join(".vanehub-pdfium-render.json"),
        format!(
            r#"{{"protocolVersion":"{PROTOCOL_VERSION}","rendererVersion":"1.0.0","binarySha256":"{hash}"}}"#
        ),
    )
    .expect("manifest");
    let launches = Arc::new(AtomicUsize::new(0));
    let terminated = Arc::new(AtomicBool::new(false));
    let backend = Arc::new(Backend {
        mode,
        launches: launches.clone(),
        cancellation,
        terminated: terminated.clone(),
    });
    let renderer = ManagedPdfiumRenderer::new(install, execution, backend);
    (root, renderer, source, launches, terminated)
}

#[test]
fn fixed_checksum_verified_renderer_inspects_and_renders_bounded_pages() {
    let (_root, renderer, source, launches, _) = fixture(ResponseMode::Valid, None);
    let cancelled = Arc::new(AtomicBool::new(false));
    assert_eq!(
        renderer
            .page_count(&source, cancelled.clone())
            .expect("pages"),
        3
    );
    let output = renderer.execution_root.join("outputs/rendered");
    let pages = renderer
        .render_pages(
            &source,
            &[2],
            &output,
            OcrAdmissionLimits::HARD_CEILING,
            cancelled,
        )
        .expect("render");
    assert_eq!(pages[0].page_number, 2);
    assert_eq!(pages[0].width, 16);
    assert_eq!(launches.load(Ordering::Acquire), 2);
    assert!(!renderer
        .execution_root
        .join("work/pdfium-render-request.json")
        .exists());
}

#[test]
fn protocol_mismatch_fails_closed() {
    let (_root, renderer, source, _, _) = fixture(ResponseMode::WrongProtocol, None);
    assert_eq!(
        renderer.page_count(&source, Arc::new(AtomicBool::new(false))),
        Err(OcrExecutionError::ProtocolViolation)
    );
}

#[test]
fn cancellation_after_launch_terminates_the_owned_tree() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let (_root, renderer, source, _, terminated) =
        fixture(ResponseMode::Hanging, Some(cancelled.clone()));
    assert_eq!(
        renderer.page_count(&source, cancelled),
        Err(OcrExecutionError::Cancelled)
    );
    assert!(terminated.load(Ordering::Acquire));
}

#[test]
fn binary_checksum_mismatch_prevents_process_launch() {
    let (_root, renderer, source, launches, _) = fixture(ResponseMode::Valid, None);
    std::fs::write(
        renderer.install_path.join(if cfg!(windows) {
            "pdfium-render.exe"
        } else {
            "pdfium-render"
        }),
        b"tampered",
    )
    .expect("tamper");
    assert_eq!(
        renderer.page_count(&source, Arc::new(AtomicBool::new(false))),
        Err(OcrExecutionError::ProtocolViolation)
    );
    assert_eq!(launches.load(Ordering::Acquire), 0);
}

fn test_png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes
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
