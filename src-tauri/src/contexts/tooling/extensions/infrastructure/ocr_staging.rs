use crate::contexts::tooling::extensions::application::{OcrAdmissionError, OcrStagingPort};
use crate::platform::filesystem::create_new_file;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) struct OwnedOcrStagingAdapter;

impl OcrStagingPort for OwnedOcrStagingAdapter {
    fn stage_read_only(
        &self,
        staging_root: &Path,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, OcrAdmissionError> {
        let metadata = std::fs::symlink_metadata(staging_root)
            .map_err(|_| OcrAdmissionError::UnsafeWorkspace)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(OcrAdmissionError::UnsafeWorkspace);
        }
        let path = staging_root.join(file_name);
        let mut file = create_new_file(&path).map_err(|_| OcrAdmissionError::UnsafeWorkspace)?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| OcrAdmissionError::UnsafeWorkspace)?;
        let mut permissions = file
            .metadata()
            .map_err(|_| OcrAdmissionError::UnsafeWorkspace)?
            .permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&path, permissions)
            .map_err(|_| OcrAdmissionError::UnsafeWorkspace)?;
        Ok(path)
    }
}
