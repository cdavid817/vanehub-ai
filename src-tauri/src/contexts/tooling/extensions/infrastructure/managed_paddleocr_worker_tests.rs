use super::*;
use crate::contexts::code_execution::application::{
    SandboxBackendCapabilities, SandboxBackendError, SandboxProcess, SandboxProcessObservation,
};
use crate::contexts::tooling::extensions::application::{
    PaddleOcrInferenceInput, PaddleOcrInferenceLimits, PaddleOcrInputKind,
    PADDLEOCR_INFERENCE_PROTOCOL_VERSION,
};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicUsize;

struct Backend {
    launches: Arc<AtomicUsize>,
    cancel_on_wait: Option<Arc<AtomicBool>>,
    terminated: Arc<AtomicBool>,
    result_override: Option<String>,
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
        assert_eq!(request.arguments[1], "--request");
        assert_eq!(request.arguments[3], "--result");
        let result_path = PathBuf::from(&request.arguments[4]);
        if self.cancel_on_wait.is_none() {
            let result = self.result_override.clone().unwrap_or_else(|| {
                format!(
                    r#"{{"protocolVersion":"{PADDLEOCR_INFERENCE_PROTOCOL_VERSION}","engineName":"paddleocr","engineVersion":"3.2.0","blocks":[],"warnings":[],"truncated":false}}"#
                )
            });
            std::fs::write(result_path, result).map_err(|_| SandboxBackendError::SpawnFailed)?;
        }
        Ok(Box::new(Process {
            observation: self
                .cancel_on_wait
                .is_none()
                .then_some(SandboxProcessObservation {
                    exit_code: 0,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                    cpu_time_ms: Some(1),
                    peak_memory_bytes: Some(1024),
                }),
            cancel_on_wait: self.cancel_on_wait.clone(),
            terminated: self.terminated.clone(),
        }))
    }
}

struct Process {
    observation: Option<SandboxProcessObservation>,
    cancel_on_wait: Option<Arc<AtomicBool>>,
    terminated: Arc<AtomicBool>,
}

impl SandboxProcess for Process {
    fn wait_until(
        &mut self,
        _: Instant,
    ) -> Result<Option<SandboxProcessObservation>, SandboxBackendError> {
        if let Some(cancelled) = &self.cancel_on_wait {
            cancelled.store(true, Ordering::Release);
        }
        Ok(self.observation.take())
    }

    fn terminate_tree(&mut self, _: Duration) -> Result<(), SandboxBackendError> {
        self.terminated.store(true, Ordering::Release);
        Ok(())
    }
}

fn fixture(
    cancel_on_wait: Option<Arc<AtomicBool>>,
) -> (
    tempfile::TempDir,
    ManagedPaddleOcrWorker,
    PaddleOcrInferenceRequest,
    Arc<AtomicUsize>,
    Arc<AtomicBool>,
) {
    let root = tempfile::tempdir().expect("root");
    let install = root.path().join("install");
    let execution = root.path().join("execution");
    std::fs::create_dir_all(&install).expect("install");
    std::fs::create_dir_all(execution.join("inputs")).expect("inputs");
    let interpreter = super::super::installation_adapter::venv_python(&install);
    std::fs::create_dir_all(interpreter.parent().expect("interpreter parent")).expect("scripts");
    std::fs::write(&interpreter, b"python").expect("python");
    let worker_path = install.join("paddleocr_worker.py");
    std::fs::write(&worker_path, b"worker").expect("worker");
    let hash = digest_hex(&Sha256::digest(b"worker"));
    std::fs::write(
        install.join(".vanehub-ocr-inference.json"),
        format!(
            r#"{{"protocolVersion":"{PADDLEOCR_INFERENCE_PROTOCOL_VERSION}","engineVersion":"3.2.0","workerSha256":"{hash}"}}"#
        ),
    )
    .expect("manifest");
    let input = execution.join("inputs/source.png");
    std::fs::write(&input, b"image").expect("input");
    let launches = Arc::new(AtomicUsize::new(0));
    let terminated = Arc::new(AtomicBool::new(false));
    let backend = Arc::new(Backend {
        launches: launches.clone(),
        cancel_on_wait,
        terminated: terminated.clone(),
        result_override: None,
    });
    let worker = ManagedPaddleOcrWorker::new(install, execution, backend);
    let request = PaddleOcrInferenceRequest {
        protocol_version: PADDLEOCR_INFERENCE_PROTOCOL_VERSION.to_owned(),
        operation_id: "operation-1".to_owned(),
        inputs: vec![PaddleOcrInferenceInput {
            staged_path: input,
            source_sha256: "a".repeat(64),
            kind: PaddleOcrInputKind::Image,
            page_number: None,
        }],
        languages: vec!["en".to_owned()],
        limits: PaddleOcrInferenceLimits::HARD_CEILING,
    };
    (root, worker, request, launches, terminated)
}

#[test]
fn verified_worker_returns_structured_result_and_cleans_control_files() {
    let (_root, worker, request, launches, _) = fixture(None);
    let result = worker
        .execute(&request, Arc::new(AtomicBool::new(false)))
        .expect("execute");
    assert_eq!(result.engine_version, "3.2.0");
    assert_eq!(launches.load(Ordering::Acquire), 1);
    assert!(!worker.execution_root.join("work/ocr-request.json").exists());
    assert!(!worker
        .execution_root
        .join("outputs/ocr-result.json")
        .exists());
}

#[test]
fn tampered_worker_is_rejected_before_launch() {
    let (_root, worker, request, launches, _) = fixture(None);
    std::fs::write(worker.install_path.join("paddleocr_worker.py"), b"tampered").expect("tamper");
    assert_eq!(
        worker.execute(&request, Arc::new(AtomicBool::new(false))),
        Err(OcrExecutionError::WorkerFailure)
    );
    assert_eq!(launches.load(Ordering::Acquire), 0);
}

#[test]
fn cancellation_terminates_worker_process_tree() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let (_root, worker, request, _, terminated) = fixture(Some(cancelled.clone()));
    assert_eq!(
        worker.execute(&request, cancelled),
        Err(OcrExecutionError::Cancelled)
    );
    assert!(terminated.load(Ordering::Acquire));
}

#[test]
fn malformed_worker_result_fails_closed_and_cleans_control_files() {
    let (_root, mut worker, request, launches, _) = fixture(None);
    worker.backend = Arc::new(Backend {
        launches: launches.clone(),
        cancel_on_wait: None,
        terminated: Arc::new(AtomicBool::new(false)),
        result_override: Some(r#"{"protocolVersion":1,"blocks":"not-an-array"}"#.to_owned()),
    });
    assert_eq!(
        worker.execute(&request, Arc::new(AtomicBool::new(false))),
        Err(OcrExecutionError::ProtocolViolation)
    );
    assert_eq!(launches.load(Ordering::Acquire), 1);
    assert!(!worker.execution_root.join("work/ocr-request.json").exists());
    assert!(!worker
        .execution_root
        .join("outputs/ocr-result.json")
        .exists());
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
