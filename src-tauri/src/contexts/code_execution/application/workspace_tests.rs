use super::*;
use crate::contexts::code_execution::application::{
    CodeExecutionRequest, CodeInputArtifact, CodeRuntime, MemorySandboxFilesystem,
    CODE_EXECUTION_CONTRACT_VERSION,
};
use std::collections::BTreeMap;
use std::sync::Mutex;

type ArtifactFixture = (String, String, Vec<u8>);

struct Artifacts(Mutex<BTreeMap<String, ArtifactFixture>>);

impl CodeArtifactInputPort for Artifacts {
    fn read_verified(
        &self,
        artifact_id: &str,
        max_bytes: usize,
    ) -> Result<(String, String, Vec<u8>), SandboxWorkspaceError> {
        let value = self
            .0
            .lock()
            .map_err(|_| SandboxWorkspaceError::ArtifactUnavailable)?
            .get(artifact_id)
            .cloned()
            .ok_or(SandboxWorkspaceError::ArtifactUnavailable)?;
        if value.2.len() > max_bytes {
            return Err(SandboxWorkspaceError::InputLimitExceeded);
        }
        Ok(value)
    }
}

fn request(hash: String) -> CodeExecutionRequest {
    CodeExecutionRequest {
        contract_version: CODE_EXECUTION_CONTRACT_VERSION,
        execution_id: "execution-1".to_owned(),
        runtime: CodeRuntime::Python,
        source: "print('sandbox')".to_owned(),
        arguments: Vec::new(),
        inputs: vec![CodeInputArtifact {
            artifact_id: "artifact-1".to_owned(),
            content_hash: hash,
        }],
        requested_limits: None,
    }
}

#[test]
fn creates_private_layout_materializes_verified_read_only_input_and_cleans_up() {
    let bytes = b"verified input".to_vec();
    let hash = hex_digest(&bytes);
    let artifacts = Arc::new(Artifacts(Mutex::new(BTreeMap::from([(
        "artifact-1".to_owned(),
        (hash.clone(), "text/plain".to_owned(), bytes.clone()),
    )]))));
    let filesystem = Arc::new(MemorySandboxFilesystem::default());
    let service =
        SandboxWorkspaceService::new(PathBuf::from("sandboxes"), artifacts, filesystem.clone())
            .expect("service");
    let workspace = service.create(&request(hash)).expect("workspace");
    assert_eq!(
        filesystem.read(&workspace.source_path).expect("source"),
        b"print('sandbox')"
    );
    let input = workspace.inputs_dir.join("input-001.txt");
    assert_eq!(filesystem.read(&input).expect("input"), bytes);
    assert!(filesystem.is_readonly(&input));
    workspace.cleanup().expect("cleanup");
    assert_eq!(filesystem.run_count(), 0);
}

#[test]
fn hash_mismatch_fails_and_removes_partial_execution_directory() {
    let artifacts = Arc::new(Artifacts(Mutex::new(BTreeMap::from([(
        "artifact-1".to_owned(),
        (
            "0".repeat(64),
            "text/plain".to_owned(),
            b"tampered".to_vec(),
        ),
    )]))));
    let filesystem = Arc::new(MemorySandboxFilesystem::default());
    let service =
        SandboxWorkspaceService::new(PathBuf::from("sandboxes"), artifacts, filesystem.clone())
            .expect("service");
    assert_eq!(
        service.create(&request("0".repeat(64))).err(),
        Some(SandboxWorkspaceError::IntegrityFailure)
    );
    assert_eq!(filesystem.run_count(), 0);
}

#[test]
fn invalid_owned_roots_are_rejected_by_the_filesystem_port() {
    struct InvalidFilesystem;
    impl SandboxFilesystemPort for InvalidFilesystem {
        fn initialize_root(&self, _: &Path) -> Result<PathBuf, SandboxWorkspaceError> {
            Err(SandboxWorkspaceError::InvalidRoot)
        }
        fn create_run_layout(
            &self,
            _: &Path,
            _: &str,
        ) -> Result<SandboxRunLayout, SandboxWorkspaceError> {
            Err(SandboxWorkspaceError::InvalidRoot)
        }
        fn write_new(&self, _: &Path, _: &[u8], _: bool) -> Result<(), SandboxWorkspaceError> {
            Err(SandboxWorkspaceError::InvalidRoot)
        }
        fn scan_outputs(
            &self,
            _: &Path,
            _: CodeExecutionLimits,
        ) -> Result<Vec<SandboxOutputFile>, SandboxOutputError> {
            Err(SandboxOutputError::UnsafeFilesystem)
        }
        fn remove_run(&self, _: &Path, _: &Path) -> Result<(), SandboxWorkspaceError> {
            Err(SandboxWorkspaceError::InvalidRoot)
        }
    }

    let artifacts = Arc::new(Artifacts(Mutex::new(BTreeMap::new())));
    assert_eq!(
        SandboxWorkspaceService::new(
            PathBuf::from("invalid"),
            artifacts,
            Arc::new(InvalidFilesystem),
        )
        .err(),
        Some(SandboxWorkspaceError::InvalidRoot)
    );
}
