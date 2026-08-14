use super::*;
use crate::contexts::code_execution::application::{
    CodeArtifactInputPort, CodeExecutionLimits, CodeInputArtifact, MemorySandboxFilesystem,
    SandboxBackendCapabilities, SandboxBackendError, SandboxProcess, SandboxWorkspaceError,
    SandboxWorkspaceService,
};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;

struct InputArtifacts;

impl CodeArtifactInputPort for InputArtifacts {
    fn read_verified(
        &self,
        _artifact_id: &str,
        _max_bytes: usize,
    ) -> Result<(String, String, Vec<u8>), SandboxWorkspaceError> {
        let bytes = b"input".to_vec();
        Ok((hex_digest(&bytes), "text/plain".to_owned(), bytes))
    }
}

struct Runtime;

impl CodeRuntimePort for Runtime {
    fn resolve_reviewed(
        &self,
        _runtime: CodeRuntime,
    ) -> Result<(PathBuf, RuntimeVersion), CodeServiceError> {
        Ok((
            std::env::current_exe().map_err(|_| CodeServiceError::RuntimeUnavailable)?,
            RuntimeVersion::new(3, 12, 0),
        ))
    }
}

struct UnavailableRuntime;

impl CodeRuntimePort for UnavailableRuntime {
    fn resolve_reviewed(
        &self,
        _runtime: CodeRuntime,
    ) -> Result<(PathBuf, RuntimeVersion), CodeServiceError> {
        Err(CodeServiceError::RuntimeUnavailable)
    }
}

#[derive(Default)]
struct Outputs(Mutex<Vec<String>>);

impl CodeOutputArtifactPort for Outputs {
    fn seal_output(
        &self,
        _execution_id: &str,
        relative_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<CodeOutputArtifact, CodeServiceError> {
        self.0
            .lock()
            .map_err(|_| CodeServiceError::ArtifactFailure)?
            .push(relative_name.to_owned());
        Ok(CodeOutputArtifact {
            artifact_id: format!("artifact-{relative_name}"),
            content_hash: hex_digest(bytes),
            relative_name: relative_name.to_owned(),
            size_bytes: bytes.len() as u64,
            media_type: media_type.to_owned(),
        })
    }
}

struct Backend {
    observation: SandboxProcessObservation,
    output: Option<(String, Vec<u8>)>,
    terminated: Option<Arc<AtomicBool>>,
    filesystem: Arc<MemorySandboxFilesystem>,
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
        if let Some((name, bytes)) = &self.output {
            let output_dir = request
                .working_directory
                .parent()
                .ok_or(SandboxBackendError::SpawnFailed)?
                .join("outputs");
            self.filesystem.insert_output(&output_dir, name, bytes);
        }
        assert_eq!(request.environment.get("NO_PROXY"), Some(&"*".to_owned()));
        assert!(!request.environment.keys().any(|key| key.contains("TOKEN")));
        Ok(Box::new(Process {
            observation: Some(self.observation.clone()),
            terminated: self.terminated.clone(),
        }))
    }
}

struct Process {
    observation: Option<SandboxProcessObservation>,
    terminated: Option<Arc<AtomicBool>>,
}

impl SandboxProcess for Process {
    fn wait_until(
        &mut self,
        _deadline: Instant,
    ) -> Result<Option<SandboxProcessObservation>, SandboxBackendError> {
        Ok(self.observation.take())
    }

    fn terminate_tree(&mut self, _timeout: Duration) -> Result<(), SandboxBackendError> {
        self.observation = None;
        if let Some(terminated) = &self.terminated {
            terminated.store(true, Ordering::Release);
        }
        Ok(())
    }
}

fn request(limits: Option<CodeExecutionLimits>) -> CodeExecutionRequest {
    let bytes = b"input";
    CodeExecutionRequest {
        contract_version: CODE_EXECUTION_CONTRACT_VERSION,
        execution_id: "execution-1".to_owned(),
        runtime: CodeRuntime::Python,
        source: "print('ok')".to_owned(),
        arguments: Vec::new(),
        inputs: vec![CodeInputArtifact {
            artifact_id: "artifact-1".to_owned(),
            content_hash: hex_digest(bytes),
        }],
        requested_limits: limits,
    }
}

fn service(
    backend: Backend,
    outputs: Arc<Outputs>,
    filesystem: Arc<MemorySandboxFilesystem>,
) -> CodeExecutionService {
    let workspaces = Arc::new(
        SandboxWorkspaceService::new(
            PathBuf::from("sandboxes"),
            Arc::new(InputArtifacts),
            filesystem,
        )
        .expect("workspaces"),
    );
    CodeExecutionService::new(workspaces, Arc::new(backend), Arc::new(Runtime), outputs)
}

#[test]
fn successful_exit_seals_admitted_outputs_and_cleans_private_workspace() {
    let filesystem = Arc::new(MemorySandboxFilesystem::default());
    let outputs = Arc::new(Outputs::default());
    let service = service(
        Backend {
            observation: SandboxProcessObservation {
                exit_code: 0,
                stdout: b"ok\n".to_vec(),
                stderr: Vec::new(),
                cpu_time_ms: Some(1),
                peak_memory_bytes: Some(1024),
            },
            output: Some(("result.json".to_owned(), br#"{"ok":true}"#.to_vec())),
            terminated: None,
            filesystem: filesystem.clone(),
        },
        outputs.clone(),
        filesystem.clone(),
    );
    let result = service
        .execute(request(None), Arc::new(AtomicBool::new(false)))
        .expect("result");
    assert_eq!(result.status, CodeExecutionStatus::Succeeded);
    assert_eq!(result.stdout, "ok\n");
    assert_eq!(result.outputs.len(), 1);
    assert_eq!(
        outputs.0.lock().expect("outputs").as_slice(),
        &["result.json"]
    );
    assert_eq!(filesystem.run_count(), 0);
}

#[test]
fn stdout_flood_is_truncated_failed_and_outputs_are_not_sealed() {
    let filesystem = Arc::new(MemorySandboxFilesystem::default());
    let outputs = Arc::new(Outputs::default());
    let mut limits = CodeExecutionLimits::HARD_CEILING;
    limits.stdout_bytes = 4;
    let service = service(
        Backend {
            observation: SandboxProcessObservation {
                exit_code: 0,
                stdout: b"too much output".to_vec(),
                stderr: Vec::new(),
                cpu_time_ms: None,
                peak_memory_bytes: None,
            },
            output: Some(("result.txt".to_owned(), b"not admitted".to_vec())),
            terminated: None,
            filesystem: filesystem.clone(),
        },
        outputs.clone(),
        filesystem,
    );
    let result = service
        .execute(request(Some(limits)), Arc::new(AtomicBool::new(false)))
        .expect("result");
    assert_eq!(result.status, CodeExecutionStatus::LimitExceeded);
    assert!(result.stdout_truncated);
    assert!(outputs.0.lock().expect("outputs").is_empty());
}

#[test]
fn filesystem_budget_excess_is_a_bounded_terminal_result() {
    let filesystem = Arc::new(MemorySandboxFilesystem::default());
    let outputs = Arc::new(Outputs::default());
    let mut limits = CodeExecutionLimits::HARD_CEILING;
    limits.filesystem_bytes = 4;
    let service = service(
        Backend {
            observation: SandboxProcessObservation {
                exit_code: 0,
                stdout: Vec::new(),
                stderr: Vec::new(),
                cpu_time_ms: None,
                peak_memory_bytes: None,
            },
            output: Some(("result.txt".to_owned(), b"too large".to_vec())),
            terminated: None,
            filesystem: filesystem.clone(),
        },
        outputs.clone(),
        filesystem,
    );
    let result = service
        .execute(request(Some(limits)), Arc::new(AtomicBool::new(false)))
        .expect("bounded result");
    assert_eq!(result.status, CodeExecutionStatus::LimitExceeded);
    assert_eq!(result.limit_reason.as_deref(), Some("filesystem_bytes"));
    assert!(outputs.0.lock().expect("outputs").is_empty());
}

#[test]
fn cancellation_terminates_the_owned_process_tree_and_cleans_workspace() {
    let filesystem = Arc::new(MemorySandboxFilesystem::default());
    let terminated = Arc::new(AtomicBool::new(false));
    let service = service(
        Backend {
            observation: SandboxProcessObservation {
                exit_code: 0,
                stdout: Vec::new(),
                stderr: Vec::new(),
                cpu_time_ms: None,
                peak_memory_bytes: None,
            },
            output: None,
            terminated: Some(terminated.clone()),
            filesystem: filesystem.clone(),
        },
        Arc::new(Outputs::default()),
        filesystem.clone(),
    );
    let result = service
        .execute(request(None), Arc::new(AtomicBool::new(true)))
        .expect("cancelled result");
    assert_eq!(result.status, CodeExecutionStatus::Cancelled);
    assert!(terminated.load(Ordering::Acquire));
    assert_eq!(filesystem.run_count(), 0);
}

#[test]
fn unsupported_output_is_rejected_and_never_sealed() {
    let filesystem = Arc::new(MemorySandboxFilesystem::default());
    let outputs = Arc::new(Outputs::default());
    let service = service(
        Backend {
            observation: SandboxProcessObservation {
                exit_code: 0,
                stdout: Vec::new(),
                stderr: Vec::new(),
                cpu_time_ms: None,
                peak_memory_bytes: None,
            },
            output: Some(("program.exe".to_owned(), b"MZ".to_vec())),
            terminated: None,
            filesystem: filesystem.clone(),
        },
        outputs.clone(),
        filesystem.clone(),
    );
    assert_eq!(
        service.execute(request(None), Arc::new(AtomicBool::new(false))),
        Err(CodeServiceError::OutputRejected)
    );
    assert!(outputs.0.lock().expect("outputs").is_empty());
    assert_eq!(filesystem.run_count(), 0);
}

#[test]
fn runtime_absence_fails_before_workspace_or_process_creation() {
    let sandbox_root = PathBuf::from("sandboxes");
    let filesystem = Arc::new(MemorySandboxFilesystem::default());
    let workspaces = Arc::new(
        SandboxWorkspaceService::new(sandbox_root, Arc::new(InputArtifacts), filesystem.clone())
            .expect("workspaces"),
    );
    let service = CodeExecutionService::new(
        workspaces,
        Arc::new(Backend {
            observation: SandboxProcessObservation {
                exit_code: 0,
                stdout: Vec::new(),
                stderr: Vec::new(),
                cpu_time_ms: None,
                peak_memory_bytes: None,
            },
            output: None,
            terminated: None,
            filesystem: filesystem.clone(),
        }),
        Arc::new(UnavailableRuntime),
        Arc::new(Outputs::default()),
    );

    assert_eq!(
        service.execute(request(None), Arc::new(AtomicBool::new(false))),
        Err(CodeServiceError::RuntimeUnavailable)
    );
    assert_eq!(filesystem.run_count(), 0);
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}
