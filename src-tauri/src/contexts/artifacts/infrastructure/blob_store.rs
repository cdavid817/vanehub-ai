use crate::contexts::artifacts::application::{
    ArtifactBlobMetadata, ArtifactBlobStoreError, ArtifactBlobStorePolicy,
};
use crate::platform::filesystem::create_new_file;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const BLOB_CONTRACT_VERSION: u16 = 1;
use super::blob_validation::{
    hex_digest, validate_display_name, validate_identifier, validate_media,
};

#[derive(Debug, Default)]
struct QuotaState {
    operations: HashMap<String, OperationUsage>,
}

#[derive(Debug, Default)]
struct OperationUsage {
    items: u32,
    bytes: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ArtifactBlobStore {
    root: PathBuf,
    policy: ArtifactBlobStorePolicy,
    quota: Arc<Mutex<QuotaState>>,
}

impl ArtifactBlobStore {
    pub(crate) fn new(
        app_data_dir: &Path,
        policy: ArtifactBlobStorePolicy,
    ) -> Result<Self, ArtifactBlobStoreError> {
        let root = app_data_dir.join("artifacts");
        create_owned_dir(&root)?;
        create_owned_dir(&root.join("blobs").join("sha256"))?;
        create_owned_dir(&root.join("staging"))?;
        create_owned_dir(&root.join("recovery"))?;
        Ok(Self {
            root,
            policy,
            quota: Arc::new(Mutex::new(QuotaState::default())),
        })
    }

    pub(crate) fn seal_bytes(
        &self,
        operation_id: &str,
        display_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<ArtifactBlobMetadata, ArtifactBlobStoreError> {
        validate_identifier(operation_id)?;
        validate_display_name(display_name)?;
        validate_media(media_type, bytes)?;
        let size_bytes = u64::try_from(bytes.len())
            .map_err(|_| ArtifactBlobStoreError::BlobByteQuotaExceeded)?;
        if size_bytes > self.policy.max_blob_bytes {
            return Err(ArtifactBlobStoreError::BlobByteQuotaExceeded);
        }
        let mut quota = self
            .quota
            .lock()
            .map_err(|_| ArtifactBlobStoreError::StorageFailure)?;
        check_operation_quota(&self.policy, &quota, operation_id, size_bytes)?;

        let digest = hex_digest(bytes);
        let target = self.blob_path(&digest)?;
        let existed = target.exists();
        if existed {
            verify_file(&target, &digest, self.policy.max_blob_bytes)?;
        } else {
            let total = store_size(&self.root.join("blobs").join("sha256"))?;
            if total.saturating_add(size_bytes) > self.policy.max_total_bytes {
                return Err(ArtifactBlobStoreError::StoreByteQuotaExceeded);
            }
            self.atomic_write(operation_id, &target, bytes, &digest)?;
        }
        record_operation_usage(&mut quota, operation_id, size_bytes);
        Ok(ArtifactBlobMetadata {
            contract_version: BLOB_CONTRACT_VERSION,
            content_hash: format!("sha256:{digest}"),
            size_bytes,
            media_type: media_type.to_owned(),
            display_name: display_name.to_owned(),
            storage_key: format!("sha256/{}/{}", &digest[..2], &digest[2..]),
            deduplicated: existed,
        })
    }

    pub(crate) fn read_verified(
        &self,
        content_hash: &str,
    ) -> Result<Vec<u8>, ArtifactBlobStoreError> {
        let digest = content_hash
            .strip_prefix("sha256:")
            .ok_or(ArtifactBlobStoreError::InvalidHash)?;
        let path = self.blob_path(digest)?;
        verify_file(&path, digest, self.policy.max_blob_bytes)
    }

    fn blob_path(&self, digest: &str) -> Result<PathBuf, ArtifactBlobStoreError> {
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ArtifactBlobStoreError::InvalidHash);
        }
        Ok(self
            .root
            .join("blobs")
            .join("sha256")
            .join(&digest[..2])
            .join(&digest[2..]))
    }

    fn atomic_write(
        &self,
        operation_id: &str,
        target: &Path,
        bytes: &[u8],
        digest: &str,
    ) -> Result<(), ArtifactBlobStoreError> {
        let staging = self.root.join("staging").join(operation_id);
        create_owned_dir(&staging)?;
        let temporary = staging.join(format!("{}.staged", Uuid::new_v4()));
        let write_result = (|| {
            let mut file = create_new_file(&temporary).map_err(storage_error)?;
            file.write_all(bytes).map_err(storage_error)?;
            file.sync_all().map_err(storage_error)?;
            let parent = target
                .parent()
                .ok_or(ArtifactBlobStoreError::StorageFailure)?;
            create_owned_dir(parent)?;
            match fs::rename(&temporary, target) {
                Ok(()) => Ok(()),
                Err(_) if target.exists() => {
                    verify_file(target, digest, self.policy.max_blob_bytes)?;
                    fs::remove_file(&temporary).map_err(storage_error)
                }
                Err(error) => Err(storage_error(error)),
            }
        })();
        if write_result.is_err() && temporary.exists() {
            let _ = fs::remove_file(&temporary);
        }
        write_result
    }
}

impl crate::contexts::artifacts::application::ArtifactBlobPort for ArtifactBlobStore {
    fn seal_bytes(
        &self,
        operation_id: &str,
        display_name: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<ArtifactBlobMetadata, ArtifactBlobStoreError> {
        Self::seal_bytes(self, operation_id, display_name, media_type, bytes)
    }

    fn read_verified(&self, content_hash: &str) -> Result<Vec<u8>, ArtifactBlobStoreError> {
        Self::read_verified(self, content_hash)
    }

    fn remove_verified(&self, content_hash: &str) -> Result<(), ArtifactBlobStoreError> {
        let digest = content_hash
            .strip_prefix("sha256:")
            .ok_or(ArtifactBlobStoreError::InvalidHash)?;
        let path = self.blob_path(digest)?;
        verify_file(&path, digest, self.policy.max_blob_bytes)?;
        fs::remove_file(path).map_err(storage_error)
    }
}

fn check_operation_quota(
    policy: &ArtifactBlobStorePolicy,
    state: &QuotaState,
    operation_id: &str,
    bytes: u64,
) -> Result<(), ArtifactBlobStoreError> {
    let usage = state.operations.get(operation_id);
    if usage.map_or(0, |value| value.items) >= policy.max_operation_items {
        return Err(ArtifactBlobStoreError::ItemQuotaExceeded);
    }
    if usage.map_or(0, |value| value.bytes).saturating_add(bytes) > policy.max_operation_bytes {
        return Err(ArtifactBlobStoreError::OperationByteQuotaExceeded);
    }
    Ok(())
}

fn record_operation_usage(state: &mut QuotaState, operation_id: &str, bytes: u64) {
    let usage = state.operations.entry(operation_id.to_owned()).or_default();
    usage.items = usage.items.saturating_add(1);
    usage.bytes = usage.bytes.saturating_add(bytes);
}

fn create_owned_dir(path: &Path) -> Result<(), ArtifactBlobStoreError> {
    if path.exists() {
        let metadata = fs::symlink_metadata(path).map_err(storage_error)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ArtifactBlobStoreError::IntegrityFailure);
        }
        return Ok(());
    }
    fs::create_dir_all(path).map_err(storage_error)
}

fn verify_file(
    path: &Path,
    expected_digest: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, ArtifactBlobStoreError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| ArtifactBlobStoreError::IntegrityFailure)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > max_bytes {
        return Err(ArtifactBlobStoreError::IntegrityFailure);
    }
    let bytes = fs::read(path).map_err(storage_error)?;
    let actual = hex_digest(&bytes);
    if actual != expected_digest {
        return Err(ArtifactBlobStoreError::IntegrityFailure);
    }
    Ok(bytes)
}

fn store_size(root: &Path) -> Result<u64, ArtifactBlobStoreError> {
    let mut total = 0_u64;
    for prefix in fs::read_dir(root).map_err(storage_error)? {
        let prefix = prefix.map_err(storage_error)?;
        let metadata = fs::symlink_metadata(prefix.path()).map_err(storage_error)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(ArtifactBlobStoreError::IntegrityFailure);
        }
        for blob in fs::read_dir(prefix.path()).map_err(storage_error)? {
            let blob = blob.map_err(storage_error)?;
            let metadata = fs::symlink_metadata(blob.path()).map_err(storage_error)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(ArtifactBlobStoreError::IntegrityFailure);
            }
            total = total
                .checked_add(metadata.len())
                .ok_or(ArtifactBlobStoreError::StoreByteQuotaExceeded)?;
        }
    }
    Ok(total)
}

fn storage_error(_error: std::io::Error) -> ArtifactBlobStoreError {
    ArtifactBlobStoreError::StorageFailure
}

#[cfg(test)]
#[path = "blob_store_tests.rs"]
mod tests;
