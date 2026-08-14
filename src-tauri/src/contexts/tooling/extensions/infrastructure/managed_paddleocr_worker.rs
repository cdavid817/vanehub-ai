use super::paddleocr_readiness::verified_inference_runtime;
use crate::contexts::code_execution::application::{
    CodeExecutionLimits, SandboxLaunchRequest, SandboxProcessBackend,
};
use crate::contexts::tooling::extensions::application::{
    OcrExecutionError, PaddleOcrEngineResult, PaddleOcrInferenceRequest, PaddleOcrWorkerPort,
};
use crate::platform::filesystem::create_new_file;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const REQUEST_FILE: &str = "ocr-request.json";
const RESULT_FILE: &str = "ocr-result.json";
const MAX_RESULT_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) struct ManagedPaddleOcrWorker {
    install_path: PathBuf,
    execution_root: PathBuf,
    backend: Arc<dyn SandboxProcessBackend>,
}

impl ManagedPaddleOcrWorker {
    pub(crate) fn new(
        install_path: PathBuf,
        execution_root: PathBuf,
        backend: Arc<dyn SandboxProcessBackend>,
    ) -> Self {
        Self {
            install_path,
            execution_root,
            backend,
        }
    }
}

impl PaddleOcrWorkerPort for ManagedPaddleOcrWorker {
    fn execute(
        &self,
        request: &PaddleOcrInferenceRequest,
        cancelled: Arc<AtomicBool>,
    ) -> Result<PaddleOcrEngineResult, OcrExecutionError> {
        request
            .validate()
            .map_err(|_| OcrExecutionError::InvalidRequest)?;
        let work = self.execution_root.join("work");
        let inputs = self.execution_root.join("inputs");
        let outputs = self.execution_root.join("outputs");
        ensure_owned_directory(&inputs)?;
        ensure_owned_directory(&work)?;
        ensure_owned_directory(&outputs)?;
        let request_path = work.join(REQUEST_FILE);
        let result_path = outputs.join(RESULT_FILE);
        let controls = WorkerControlFiles::new(request_path.clone(), result_path.clone());
        let request_bytes =
            serde_json::to_vec(request).map_err(|_| OcrExecutionError::InvalidRequest)?;
        use std::io::Write;
        let mut request_file =
            create_new_file(&request_path).map_err(|_| OcrExecutionError::WorkerFailure)?;
        request_file
            .write_all(&request_bytes)
            .map_err(|_| OcrExecutionError::WorkerFailure)?;
        let runtime = verified_inference_runtime(&self.install_path)
            .map_err(|_| OcrExecutionError::WorkerFailure)?;
        if runtime.protocol_version != request.protocol_version {
            return Err(OcrExecutionError::ProtocolViolation);
        }
        let limits = CodeExecutionLimits {
            wall_time_ms: request.limits.duration_ms,
            cpu_time_ms: request.limits.duration_ms,
            memory_bytes: 1024 * 1024 * 1024,
            process_count: 2,
            stdout_bytes: 64 * 1024,
            stderr_bytes: 64 * 1024,
            filesystem_bytes: MAX_RESULT_BYTES,
            file_count: 1,
            event_count: 64,
        };
        let launch = SandboxLaunchRequest {
            executable: runtime.interpreter,
            arguments: vec![
                runtime.worker.to_string_lossy().into_owned(),
                "--request".to_owned(),
                request_path.to_string_lossy().into_owned(),
                "--result".to_owned(),
                result_path.to_string_lossy().into_owned(),
            ],
            working_directory: work,
            environment: minimal_windows_environment(),
            limits,
        };
        let mut process = self
            .backend
            .launch(launch)
            .map_err(|_| OcrExecutionError::WorkerFailure)?;
        let deadline = Instant::now() + Duration::from_millis(request.limits.duration_ms);
        let observation = loop {
            if cancelled.load(Ordering::Acquire) {
                process
                    .terminate_tree(Duration::from_secs(2))
                    .map_err(|_| OcrExecutionError::WorkerFailure)?;
                return Err(OcrExecutionError::Cancelled);
            }
            if Instant::now() >= deadline {
                process
                    .terminate_tree(Duration::from_secs(2))
                    .map_err(|_| OcrExecutionError::WorkerFailure)?;
                return Err(OcrExecutionError::LimitExceeded);
            }
            if let Some(observation) = process
                .wait_until((Instant::now() + Duration::from_millis(50)).min(deadline))
                .map_err(|_| OcrExecutionError::WorkerFailure)?
            {
                break observation;
            }
        };
        if observation.exit_code != 0 {
            return Err(OcrExecutionError::WorkerFailure);
        }
        let metadata = std::fs::symlink_metadata(&result_path)
            .map_err(|_| OcrExecutionError::WorkerFailure)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > MAX_RESULT_BYTES
        {
            return Err(OcrExecutionError::ProtocolViolation);
        }
        let bytes = std::fs::read(result_path).map_err(|_| OcrExecutionError::WorkerFailure)?;
        let result =
            serde_json::from_slice(&bytes).map_err(|_| OcrExecutionError::ProtocolViolation);
        drop(controls);
        result
    }
}

struct WorkerControlFiles {
    request: PathBuf,
    result: PathBuf,
}

impl WorkerControlFiles {
    fn new(request: PathBuf, result: PathBuf) -> Self {
        Self { request, result }
    }
}

impl Drop for WorkerControlFiles {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.request);
        let _ = std::fs::remove_file(&self.result);
    }
}

fn ensure_owned_directory(path: &std::path::Path) -> Result<(), OcrExecutionError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(OcrExecutionError::WorkerFailure),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(path).map_err(|_| OcrExecutionError::WorkerFailure)
        }
        Err(_) => Err(OcrExecutionError::WorkerFailure),
    }
}

fn minimal_windows_environment() -> BTreeMap<String, String> {
    let environment = BTreeMap::from([("NO_PROXY".to_owned(), "*".to_owned())]);
    #[cfg(windows)]
    {
        let mut environment = environment;
        let root = std::env::var("SystemRoot")
            .or_else(|_| std::env::var("WINDIR"))
            .unwrap_or_else(|_| "C:\\Windows".to_owned());
        environment.insert("SYSTEMROOT".to_owned(), root.clone());
        environment.insert("WINDIR".to_owned(), root.clone());
        environment.insert("PATH".to_owned(), format!("{root}\\System32"));
        environment.insert("PYTHONNOUSERSITE".to_owned(), "1".to_owned());
        environment.insert("PYTHONDONTWRITEBYTECODE".to_owned(), "1".to_owned());
        environment
    }
    #[cfg(not(windows))]
    {
        environment
    }
}

#[cfg(test)]
#[path = "managed_paddleocr_worker_tests.rs"]
mod tests;
