use super::DelegationWorkspace;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAX_TASK_BYTES: usize = 64 * 1024;
const MAX_CONTEXT_BYTES: usize = 256 * 1024;
const MAX_ARTIFACTS: usize = 16;
const MAX_ARTIFACT_BYTES: usize = 32 * 1024 * 1024;
const MAX_TOTAL_ARTIFACT_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationArtifactInput {
    pub(crate) id: String,
    pub(crate) content_hash: String,
    pub(crate) display_name: String,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) trait DelegationArtifactPort: Send + Sync {
    fn read_verified(&self, artifact_id: &str) -> Result<DelegationArtifactInput, ()>;
}

pub(crate) trait DelegationMaterializationPort: Send + Sync {
    fn write_new(
        &self,
        path: &Path,
        bytes: &[u8],
        readonly: bool,
    ) -> Result<(), DelegationMaterializationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationMaterializationRequest {
    pub(crate) task: String,
    pub(crate) context_summary: Option<String>,
    pub(crate) artifact_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationMaterializationError {
    InvalidRequest,
    LimitExceeded,
    ArtifactUnavailable,
    IntegrityFailure,
    StorageFailure,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FrozenEnvelope<'a> {
    contract_version: u16,
    task: &'a str,
    context_summary: Option<&'a str>,
    artifact_ids: &'a [String],
}

pub(crate) struct DelegationMaterializer {
    artifacts: Arc<dyn DelegationArtifactPort>,
    storage: Arc<dyn DelegationMaterializationPort>,
}

impl DelegationMaterializer {
    pub(crate) fn new(
        artifacts: Arc<dyn DelegationArtifactPort>,
        storage: Arc<dyn DelegationMaterializationPort>,
    ) -> Self {
        Self { artifacts, storage }
    }

    pub(crate) fn materialize(
        &self,
        workspace: &DelegationWorkspace,
        request: &DelegationMaterializationRequest,
    ) -> Result<Vec<PathBuf>, DelegationMaterializationError> {
        validate_request(request)?;
        let envelope = FrozenEnvelope {
            contract_version: 1,
            task: &request.task,
            context_summary: request.context_summary.as_deref(),
            artifact_ids: &request.artifact_ids,
        };
        let envelope_bytes = serde_json::to_vec(&envelope)
            .map_err(|_| DelegationMaterializationError::InvalidRequest)?;
        self.storage.write_new(
            &workspace.control.join("request.json"),
            &envelope_bytes,
            false,
        )?;

        let mut paths = Vec::new();
        let mut total = 0_usize;
        for (index, artifact_id) in request.artifact_ids.iter().enumerate() {
            let artifact = self
                .artifacts
                .read_verified(artifact_id)
                .map_err(|_| DelegationMaterializationError::ArtifactUnavailable)?;
            if artifact.id != *artifact_id || !valid_hash(&artifact.content_hash) {
                return Err(DelegationMaterializationError::IntegrityFailure);
            }
            total = total.saturating_add(artifact.bytes.len());
            if artifact.bytes.len() > MAX_ARTIFACT_BYTES || total > MAX_TOTAL_ARTIFACT_BYTES {
                return Err(DelegationMaterializationError::LimitExceeded);
            }
            let name = format!("{index:02}-{}", safe_name(&artifact.display_name));
            let path = workspace.inputs.join(name);
            self.storage.write_new(&path, &artifact.bytes, true)?;
            paths.push(path);
        }
        Ok(paths)
    }
}

fn validate_request(
    request: &DelegationMaterializationRequest,
) -> Result<(), DelegationMaterializationError> {
    let context_size = request
        .context_summary
        .as_ref()
        .map_or(0, |value| value.len());
    if request.task.trim().is_empty()
        || request.task.len() > MAX_TASK_BYTES
        || context_size > MAX_CONTEXT_BYTES
        || request.artifact_ids.len() > MAX_ARTIFACTS
        || request
            .artifact_ids
            .iter()
            .any(|id| id.trim().is_empty() || id.len() > 128)
    {
        return Err(DelegationMaterializationError::InvalidRequest);
    }
    Ok(())
}

fn valid_hash(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn safe_name(value: &str) -> String {
    let name = Path::new(value)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("artifact.bin");
    let filtered = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .take(96)
        .collect::<String>();
    if filtered.is_empty() {
        "artifact.bin".to_owned()
    } else {
        filtered
    }
}

#[cfg(test)]
#[path = "materialization_tests.rs"]
mod tests;
