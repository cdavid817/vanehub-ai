use super::{CodeExecutionLimits, CodeExecutionRequest};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_TOTAL_INPUT_BYTES: usize = 32 * 1024 * 1024;

pub(crate) trait CodeArtifactInputPort: Send + Sync {
    fn read_verified(
        &self,
        artifact_id: &str,
        max_bytes: usize,
    ) -> Result<(String, String, Vec<u8>), SandboxWorkspaceError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SandboxOutputFile {
    pub(crate) name: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SandboxOutputError {
    FileCountLimit,
    ByteLimit,
    UnsafeFilesystem,
}

pub(crate) trait SandboxFilesystemPort: Send + Sync {
    fn initialize_root(&self, root: &Path) -> Result<PathBuf, SandboxWorkspaceError>;
    fn create_run_layout(
        &self,
        owned_root: &Path,
        run_name: &str,
    ) -> Result<SandboxRunLayout, SandboxWorkspaceError>;
    fn write_new(
        &self,
        path: &Path,
        bytes: &[u8],
        readonly: bool,
    ) -> Result<(), SandboxWorkspaceError>;
    fn scan_outputs(
        &self,
        outputs_dir: &Path,
        limits: CodeExecutionLimits,
    ) -> Result<Vec<SandboxOutputFile>, SandboxOutputError>;
    fn remove_run(&self, owned_root: &Path, run_root: &Path) -> Result<(), SandboxWorkspaceError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SandboxRunLayout {
    pub(crate) root: PathBuf,
    pub(crate) inputs_dir: PathBuf,
    pub(crate) work_dir: PathBuf,
    pub(crate) outputs_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SandboxWorkspaceError {
    InvalidRoot,
    AlreadyExists,
    ArtifactUnavailable,
    IntegrityFailure,
    InputLimitExceeded,
    UnsafeFilesystem,
    IoFailure,
}

pub(crate) struct SandboxWorkspace {
    filesystem: Arc<dyn SandboxFilesystemPort>,
    owned_root: PathBuf,
    root: PathBuf,
    pub(crate) source_path: PathBuf,
    #[allow(dead_code)]
    pub(crate) inputs_dir: PathBuf,
    pub(crate) work_dir: PathBuf,
    pub(crate) outputs_dir: PathBuf,
    cleaned: bool,
}

impl SandboxWorkspace {
    pub(crate) fn scan_outputs(
        &self,
        limits: CodeExecutionLimits,
    ) -> Result<Vec<SandboxOutputFile>, SandboxOutputError> {
        self.filesystem.scan_outputs(&self.outputs_dir, limits)
    }

    pub(crate) fn cleanup(mut self) -> Result<(), SandboxWorkspaceError> {
        self.remove_owned_tree()?;
        self.cleaned = true;
        Ok(())
    }

    fn remove_owned_tree(&self) -> Result<(), SandboxWorkspaceError> {
        self.filesystem.remove_run(&self.owned_root, &self.root)
    }
}

impl Drop for SandboxWorkspace {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.remove_owned_tree();
        }
    }
}

pub(crate) struct SandboxWorkspaceService {
    filesystem: Arc<dyn SandboxFilesystemPort>,
    owned_root: PathBuf,
    artifacts: Arc<dyn CodeArtifactInputPort>,
    next_id: AtomicU64,
}

impl SandboxWorkspaceService {
    pub(crate) fn new(
        owned_root: PathBuf,
        artifacts: Arc<dyn CodeArtifactInputPort>,
        filesystem: Arc<dyn SandboxFilesystemPort>,
    ) -> Result<Self, SandboxWorkspaceError> {
        let owned_root = filesystem.initialize_root(&owned_root)?;
        Ok(Self {
            filesystem,
            owned_root,
            artifacts,
            next_id: AtomicU64::new(1),
        })
    }

    pub(crate) fn create(
        &self,
        request: &CodeExecutionRequest,
    ) -> Result<SandboxWorkspace, SandboxWorkspaceError> {
        request
            .validate()
            .map_err(|_| SandboxWorkspaceError::UnsafeFilesystem)?;
        let run_name = format!(
            "run-{}-{}",
            request.execution_id,
            self.next_id.fetch_add(1, Ordering::Relaxed)
        );
        let layout = self
            .filesystem
            .create_run_layout(&self.owned_root, &run_name)?;
        let result = self.populate(layout, request);
        if result.is_err() {
            let _ = self
                .filesystem
                .remove_run(&self.owned_root, &self.owned_root.join(run_name));
        }
        result
    }

    fn populate(
        &self,
        layout: SandboxRunLayout,
        request: &CodeExecutionRequest,
    ) -> Result<SandboxWorkspace, SandboxWorkspaceError> {
        let source_path = layout
            .work_dir
            .join(format!("main.{}", request.runtime.source_extension()));
        self.filesystem
            .write_new(&source_path, request.source.as_bytes(), false)?;
        let mut total_bytes = 0_usize;
        for (index, input) in request.inputs.iter().enumerate() {
            let (actual_hash, media_type, bytes) = self
                .artifacts
                .read_verified(&input.artifact_id, MAX_INPUT_BYTES)?;
            total_bytes = total_bytes
                .checked_add(bytes.len())
                .ok_or(SandboxWorkspaceError::InputLimitExceeded)?;
            if total_bytes > MAX_TOTAL_INPUT_BYTES {
                return Err(SandboxWorkspaceError::InputLimitExceeded);
            }
            if actual_hash != input.content_hash || hex_digest(&bytes) != input.content_hash {
                return Err(SandboxWorkspaceError::IntegrityFailure);
            }
            let path = layout.inputs_dir.join(format!(
                "input-{:03}.{}",
                index + 1,
                admitted_extension(&media_type)?
            ));
            self.filesystem.write_new(&path, &bytes, true)?;
        }
        Ok(SandboxWorkspace {
            filesystem: self.filesystem.clone(),
            owned_root: self.owned_root.clone(),
            root: layout.root,
            source_path,
            inputs_dir: layout.inputs_dir,
            work_dir: layout.work_dir,
            outputs_dir: layout.outputs_dir,
            cleaned: false,
        })
    }
}

fn admitted_extension(media_type: &str) -> Result<&'static str, SandboxWorkspaceError> {
    match media_type {
        "text/plain" => Ok("txt"),
        "text/csv" => Ok("csv"),
        "application/json" => Ok("json"),
        "application/pdf" => Ok("pdf"),
        "image/png" => Ok("png"),
        "image/jpeg" => Ok("jpg"),
        _ => Err(SandboxWorkspaceError::ArtifactUnavailable),
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
