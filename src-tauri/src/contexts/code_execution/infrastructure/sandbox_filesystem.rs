use crate::contexts::code_execution::application::{
    CodeExecutionLimits, SandboxFilesystemPort, SandboxOutputError, SandboxOutputFile,
    SandboxRunLayout, SandboxWorkspaceError,
};
use crate::platform::filesystem::create_new_file;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) struct SystemSandboxFilesystem;

impl SandboxFilesystemPort for SystemSandboxFilesystem {
    fn initialize_root(&self, root: &Path) -> Result<PathBuf, SandboxWorkspaceError> {
        std::fs::create_dir_all(root).map_err(|_| SandboxWorkspaceError::IoFailure)?;
        let metadata =
            std::fs::symlink_metadata(root).map_err(|_| SandboxWorkspaceError::InvalidRoot)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(SandboxWorkspaceError::InvalidRoot);
        }
        std::fs::canonicalize(root).map_err(|_| SandboxWorkspaceError::InvalidRoot)
    }

    fn create_run_layout(
        &self,
        owned_root: &Path,
        run_name: &str,
    ) -> Result<SandboxRunLayout, SandboxWorkspaceError> {
        let root = owned_root.join(run_name);
        std::fs::create_dir(&root).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                SandboxWorkspaceError::AlreadyExists
            } else {
                SandboxWorkspaceError::IoFailure
            }
        })?;
        let inputs_dir = create_directory(&root, "inputs")?;
        let work_dir = create_directory(&root, "work")?;
        let outputs_dir = create_directory(&root, "outputs")?;
        Ok(SandboxRunLayout {
            root,
            inputs_dir,
            work_dir,
            outputs_dir,
        })
    }

    fn write_new(
        &self,
        path: &Path,
        bytes: &[u8],
        readonly: bool,
    ) -> Result<(), SandboxWorkspaceError> {
        let mut file = create_new_file(path).map_err(|_| SandboxWorkspaceError::IoFailure)?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| SandboxWorkspaceError::IoFailure)?;
        if readonly {
            let mut permissions = file
                .metadata()
                .map_err(|_| SandboxWorkspaceError::IoFailure)?
                .permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(path, permissions)
                .map_err(|_| SandboxWorkspaceError::IoFailure)?;
        }
        Ok(())
    }

    fn scan_outputs(
        &self,
        outputs_dir: &Path,
        limits: CodeExecutionLimits,
    ) -> Result<Vec<SandboxOutputFile>, SandboxOutputError> {
        let mut entries = std::fs::read_dir(outputs_dir)
            .map_err(|_| SandboxOutputError::UnsafeFilesystem)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| SandboxOutputError::UnsafeFilesystem)?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        if entries.len() > limits.file_count as usize {
            return Err(SandboxOutputError::FileCountLimit);
        }
        let mut total = 0_u64;
        let mut files = Vec::with_capacity(entries.len());
        for entry in entries {
            let metadata = std::fs::symlink_metadata(entry.path())
                .map_err(|_| SandboxOutputError::UnsafeFilesystem)?;
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(SandboxOutputError::UnsafeFilesystem);
            }
            total = total
                .checked_add(metadata.len())
                .ok_or(SandboxOutputError::ByteLimit)?;
            if total > limits.filesystem_bytes {
                return Err(SandboxOutputError::ByteLimit);
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| SandboxOutputError::UnsafeFilesystem)?;
            let bytes =
                std::fs::read(entry.path()).map_err(|_| SandboxOutputError::UnsafeFilesystem)?;
            files.push(SandboxOutputFile { name, bytes });
        }
        Ok(files)
    }

    fn remove_run(&self, owned_root: &Path, run_root: &Path) -> Result<(), SandboxWorkspaceError> {
        if run_root.parent() != Some(owned_root) {
            return Err(SandboxWorkspaceError::UnsafeFilesystem);
        }
        let metadata =
            std::fs::symlink_metadata(run_root).map_err(|_| SandboxWorkspaceError::IoFailure)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(SandboxWorkspaceError::UnsafeFilesystem);
        }
        std::fs::remove_dir_all(run_root).map_err(|_| SandboxWorkspaceError::IoFailure)
    }
}

fn create_directory(root: &Path, name: &str) -> Result<PathBuf, SandboxWorkspaceError> {
    let path = root.join(name);
    std::fs::create_dir(&path).map_err(|_| SandboxWorkspaceError::IoFailure)?;
    Ok(path)
}
