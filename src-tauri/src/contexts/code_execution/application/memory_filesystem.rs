use super::{
    CodeExecutionLimits, SandboxFilesystemPort, SandboxOutputError, SandboxOutputFile,
    SandboxRunLayout, SandboxWorkspaceError,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Default)]
pub(crate) struct MemorySandboxFilesystem {
    files: Mutex<BTreeMap<PathBuf, (Vec<u8>, bool)>>,
    runs: Mutex<BTreeSet<PathBuf>>,
}

impl MemorySandboxFilesystem {
    pub(crate) fn insert_output(&self, output_dir: &Path, name: &str, bytes: &[u8]) {
        self.files
            .lock()
            .expect("files")
            .insert(output_dir.join(name), (bytes.to_vec(), false));
    }

    pub(crate) fn read(&self, path: &Path) -> Option<Vec<u8>> {
        self.files
            .lock()
            .expect("files")
            .get(path)
            .map(|value| value.0.clone())
    }

    pub(crate) fn is_readonly(&self, path: &Path) -> bool {
        self.files
            .lock()
            .expect("files")
            .get(path)
            .is_some_and(|value| value.1)
    }

    pub(crate) fn run_count(&self) -> usize {
        self.runs.lock().expect("runs").len()
    }
}

impl SandboxFilesystemPort for MemorySandboxFilesystem {
    fn initialize_root(&self, root: &Path) -> Result<PathBuf, SandboxWorkspaceError> {
        Ok(root.to_path_buf())
    }

    fn create_run_layout(
        &self,
        owned_root: &Path,
        run_name: &str,
    ) -> Result<SandboxRunLayout, SandboxWorkspaceError> {
        let root = owned_root.join(run_name);
        let mut runs = self.runs.lock().expect("runs");
        if !runs.insert(root.clone()) {
            return Err(SandboxWorkspaceError::AlreadyExists);
        }
        Ok(SandboxRunLayout {
            inputs_dir: root.join("inputs"),
            work_dir: root.join("work"),
            outputs_dir: root.join("outputs"),
            root,
        })
    }

    fn write_new(
        &self,
        path: &Path,
        bytes: &[u8],
        readonly: bool,
    ) -> Result<(), SandboxWorkspaceError> {
        let mut files = self.files.lock().expect("files");
        if files.contains_key(path) {
            return Err(SandboxWorkspaceError::AlreadyExists);
        }
        files.insert(path.to_path_buf(), (bytes.to_vec(), readonly));
        Ok(())
    }

    fn scan_outputs(
        &self,
        outputs_dir: &Path,
        limits: CodeExecutionLimits,
    ) -> Result<Vec<SandboxOutputFile>, SandboxOutputError> {
        let files = self.files.lock().expect("files");
        let mut outputs = files
            .iter()
            .filter(|(path, _)| path.parent() == Some(outputs_dir))
            .map(|(path, value)| SandboxOutputFile {
                name: path
                    .file_name()
                    .expect("output name")
                    .to_string_lossy()
                    .into_owned(),
                bytes: value.0.clone(),
            })
            .collect::<Vec<_>>();
        outputs.sort_by(|left, right| left.name.cmp(&right.name));
        if outputs.len() > limits.file_count as usize {
            return Err(SandboxOutputError::FileCountLimit);
        }
        let bytes = outputs.iter().fold(0_u64, |total, output| {
            total.saturating_add(output.bytes.len() as u64)
        });
        if bytes > limits.filesystem_bytes {
            return Err(SandboxOutputError::ByteLimit);
        }
        Ok(outputs)
    }

    fn remove_run(&self, owned_root: &Path, run_root: &Path) -> Result<(), SandboxWorkspaceError> {
        if run_root.parent() != Some(owned_root) {
            return Err(SandboxWorkspaceError::UnsafeFilesystem);
        }
        self.runs.lock().expect("runs").remove(run_root);
        self.files
            .lock()
            .expect("files")
            .retain(|path, _| !path.starts_with(run_root));
        Ok(())
    }
}
