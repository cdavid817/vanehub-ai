use super::*;
use std::collections::BTreeMap;
use std::sync::Mutex;

struct Artifacts(BTreeMap<String, DelegationArtifactInput>);

impl DelegationArtifactPort for Artifacts {
    fn read_verified(&self, artifact_id: &str) -> Result<DelegationArtifactInput, ()> {
        self.0.get(artifact_id).cloned().ok_or(())
    }
}

#[derive(Default)]
struct Storage(Mutex<BTreeMap<PathBuf, (Vec<u8>, bool)>>);

impl DelegationMaterializationPort for Storage {
    fn write_new(
        &self,
        path: &Path,
        bytes: &[u8],
        readonly: bool,
    ) -> Result<(), DelegationMaterializationError> {
        let mut files = self.0.lock().expect("files");
        if files.contains_key(path) {
            return Err(DelegationMaterializationError::StorageFailure);
        }
        files.insert(path.to_path_buf(), (bytes.to_vec(), readonly));
        Ok(())
    }
}

fn workspace(root: &Path) -> DelegationWorkspace {
    DelegationWorkspace {
        attempt_root: root.to_path_buf(),
        workspace: root.join("workspace"),
        inputs: root.join("inputs"),
        output: root.join("output"),
        control: root.join("control"),
        recovery: root.join("recovery"),
        repository_identity: "repo".to_owned(),
        base_commit: "a".repeat(40),
    }
}

#[test]
fn only_frozen_envelope_and_selected_verified_artifacts_are_materialized() {
    let root = Path::new("owned-attempt");
    let delegation_workspace = workspace(root);
    let artifact = DelegationArtifactInput {
        id: "artifact-selected".to_owned(),
        content_hash: format!("sha256:{}", "a".repeat(64)),
        display_name: "../private context.txt".to_owned(),
        bytes: b"untrusted artifact instructions".to_vec(),
    };
    let storage = Arc::new(Storage::default());
    let materializer = DelegationMaterializer::new(
        Arc::new(Artifacts(BTreeMap::from([(artifact.id.clone(), artifact)]))),
        storage.clone(),
    );
    let paths = materializer
        .materialize(
            &delegation_workspace,
            &DelegationMaterializationRequest {
                task: "bounded task".to_owned(),
                context_summary: Some("explicit summary".to_owned()),
                artifact_ids: vec!["artifact-selected".to_owned()],
            },
        )
        .expect("materialize");
    assert_eq!(paths.len(), 1);
    assert!(paths[0].starts_with(&delegation_workspace.inputs));
    let files = storage.0.lock().expect("files");
    assert!(files.get(&paths[0]).expect("artifact").1);
    assert_eq!(
        files.get(&paths[0]).expect("artifact").0,
        b"untrusted artifact instructions".to_vec()
    );
    let envelope = String::from_utf8(
        files
            .get(&delegation_workspace.control.join("request.json"))
            .expect("envelope")
            .0
            .clone(),
    )
    .expect("utf8");
    assert!(envelope.contains("bounded task"));
    assert!(envelope.contains("explicit summary"));
    assert!(!envelope.contains("parent transcript"));
    assert!(!files
        .keys()
        .any(|path| path.starts_with(&delegation_workspace.workspace)));
}

#[test]
fn unknown_artifact_and_oversized_explicit_context_fail_closed() {
    let delegation_workspace = workspace(Path::new("owned-attempt"));
    let materializer = DelegationMaterializer::new(
        Arc::new(Artifacts(BTreeMap::new())),
        Arc::new(Storage::default()),
    );
    assert_eq!(
        materializer.materialize(
            &delegation_workspace,
            &DelegationMaterializationRequest {
                task: "task".to_owned(),
                context_summary: None,
                artifact_ids: vec!["missing".to_owned()],
            }
        ),
        Err(DelegationMaterializationError::ArtifactUnavailable)
    );

    assert_eq!(
        materializer.materialize(
            &workspace(Path::new("second-attempt")),
            &DelegationMaterializationRequest {
                task: "task".to_owned(),
                context_summary: Some("x".repeat(MAX_CONTEXT_BYTES + 1)),
                artifact_ids: Vec::new(),
            }
        ),
        Err(DelegationMaterializationError::InvalidRequest)
    );
}
