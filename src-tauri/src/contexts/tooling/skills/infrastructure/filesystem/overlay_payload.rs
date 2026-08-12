#![cfg_attr(not(test), allow(dead_code))]

use super::overlay_layout::{is_overlay_manifest_path, OverlayStorageLayout};
use super::overlay_manifest::{parse_overlay_manifest, OverlayManifestError};
use crate::contexts::tooling::skills::application::{
    OverlayApplicationError, OverlayIntegrityCode, OverlayKey, OverlayPayloadRepository,
    OverlayPayloadWrite, SkillApplicationError,
};
use crate::contexts::tooling::skills::domain::{OverlayDocument, OverlayScope};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const HASH_ALGORITHM: &str = "sha256";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayPayloadRecoveryState {
    Pending,
    Complete,
}

#[derive(Debug)]
pub(crate) struct OverlayPayloadStage {
    payload_ref: String,
    content_hash: String,
    staged_path: Option<PathBuf>,
    final_path: PathBuf,
}

impl OverlayPayloadStage {
    pub(crate) fn payload_ref(&self) -> &str {
        &self.payload_ref
    }

    pub(crate) fn staged_path(&self) -> Option<&Path> {
        self.staged_path.as_deref()
    }

    pub(crate) fn final_path(&self) -> &Path {
        &self.final_path
    }
}

#[derive(Clone)]
pub(crate) struct OverlayPayloadStore {
    home_root: PathBuf,
}

impl OverlayPayloadStore {
    pub(crate) fn new() -> Self {
        Self {
            home_root: super::default_home_root(),
        }
    }

    pub(crate) fn with_home_root(home_root: PathBuf) -> Self {
        Self { home_root }
    }

    pub(crate) fn stage(
        &self,
        key: &OverlayKey,
        write: &OverlayPayloadWrite,
        transaction_id: &str,
    ) -> Result<OverlayPayloadStage, SkillApplicationError> {
        validate_hash(&write.content_hash)?;
        if sha256(&write.content) != write.content_hash {
            return Err(integrity(OverlayIntegrityCode::PayloadHashMismatch));
        }
        validate_transaction_id(transaction_id)?;
        let layout = OverlayStorageLayout::resolve(&self.home_root, key).map_err(layout_error)?;
        let final_root = layout.payload_root.join(HASH_ALGORITHM);
        create_safe_directory(&layout.payload_root, &final_root)?;
        let final_path = final_root.join(&write.content_hash);
        let payload_ref = format!("{HASH_ALGORITHM}/{}", write.content_hash);
        if final_path.is_file() {
            verify_file(&final_path, &write.content_hash)?;
            return Ok(OverlayPayloadStage {
                payload_ref,
                content_hash: write.content_hash.clone(),
                staged_path: None,
                final_path,
            });
        }
        let staging_root = layout.payload_root.join(".staging").join(transaction_id);
        create_safe_directory(&layout.payload_root, &staging_root)?;
        let staged_path = staging_root.join(&write.content_hash);
        if staged_path.is_file() {
            verify_file(&staged_path, &write.content_hash)?;
        } else {
            let mut file = crate::platform::private_relay_fs::create_new_private_file(&staged_path)
                .map_err(filesystem_error)?;
            file.write_all(&write.content).map_err(filesystem_error)?;
            file.sync_all().map_err(filesystem_error)?;
        }
        Ok(OverlayPayloadStage {
            payload_ref,
            content_hash: write.content_hash.clone(),
            staged_path: Some(staged_path),
            final_path,
        })
    }

    pub(crate) fn publish(&self, stage: OverlayPayloadStage) -> Result<(), SkillApplicationError> {
        let Some(staged_path) = stage.staged_path else {
            return verify_file(&stage.final_path, &stage.content_hash);
        };
        if stage.final_path.is_file() {
            verify_file(&stage.final_path, &stage.content_hash)?;
            fs::remove_file(staged_path).map_err(filesystem_error)?;
            return Ok(());
        }
        verify_file(&staged_path, &stage.content_hash)?;
        fs::rename(&staged_path, &stage.final_path).map_err(filesystem_error)?;
        verify_file(&stage.final_path, &stage.content_hash)
    }

    pub(crate) fn discard_stage(
        &self,
        stage: OverlayPayloadStage,
    ) -> Result<(), SkillApplicationError> {
        if let Some(path) = stage.staged_path {
            match fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(filesystem_error(error)),
            }
        } else {
            Ok(())
        }
    }

    pub(crate) fn cleanup_orphans(
        &self,
        key: &OverlayKey,
        retained_revisions: &[OverlayDocument],
        recovery_state: OverlayPayloadRecoveryState,
    ) -> Result<Vec<String>, SkillApplicationError> {
        if recovery_state != OverlayPayloadRecoveryState::Complete {
            return Err(SkillApplicationError::Validation(
                "Overlay payload cleanup requires completed transaction recovery".to_string(),
            ));
        }
        let layout = OverlayStorageLayout::resolve(&self.home_root, key).map_err(layout_error)?;
        let mut reachable = self.boundary_references(key)?;
        for document in retained_revisions {
            validate_retained_boundary(key, document)?;
            reachable.extend(document_references(document)?);
        }
        let payload_directory = layout.payload_root.join(HASH_ALGORITHM);
        if !payload_directory.is_dir() {
            return Ok(Vec::new());
        }
        let mut entries = fs::read_dir(&payload_directory)
            .map_err(filesystem_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(filesystem_error)?;
        entries.sort_by_key(fs::DirEntry::file_name);
        let mut removed = Vec::new();
        for entry in entries {
            let file_type = entry.file_type().map_err(filesystem_error)?;
            if file_type.is_symlink() || !file_type.is_file() {
                continue;
            }
            let Some(content_hash) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if !is_valid_hash(&content_hash) || reachable.contains(&content_hash) {
                continue;
            }
            fs::remove_file(entry.path()).map_err(filesystem_error)?;
            removed.push(content_hash);
        }
        Ok(removed)
    }

    fn boundary_references(
        &self,
        key: &OverlayKey,
    ) -> Result<BTreeSet<String>, SkillApplicationError> {
        let layout = OverlayStorageLayout::resolve(&self.home_root, key).map_err(layout_error)?;
        let manifest_root = layout.manifest_path.parent().ok_or_else(|| {
            SkillApplicationError::Filesystem("Overlay manifest has no parent".into())
        })?;
        let roots = match key.scope {
            OverlayScope::Project => vec![manifest_root.to_path_buf()],
            OverlayScope::System | OverlayScope::User => {
                let home_root = layout.payload_root.parent().ok_or_else(|| {
                    SkillApplicationError::Filesystem("Overlay payload root has no parent".into())
                })?;
                vec![home_root.to_path_buf(), home_root.join("user")]
            }
        };
        let mut references = BTreeSet::new();
        for root in roots {
            for path in manifest_paths(&root)? {
                let document = read_manifest(&path)?;
                references.extend(document_references(&document)?);
            }
        }
        Ok(references)
    }
}

impl OverlayPayloadRepository for OverlayPayloadStore {
    fn read_verified(
        &self,
        key: &OverlayKey,
        content_hash: &str,
    ) -> Result<Vec<u8>, SkillApplicationError> {
        validate_hash(content_hash)?;
        let layout = OverlayStorageLayout::resolve(&self.home_root, key).map_err(layout_error)?;
        let path = layout.payload_root.join(HASH_ALGORITHM).join(content_hash);
        if !path.is_file() {
            return Err(integrity(OverlayIntegrityCode::PayloadMissing));
        }
        let content = fs::read(path).map_err(filesystem_error)?;
        if sha256(&content) != content_hash {
            return Err(integrity(OverlayIntegrityCode::PayloadHashMismatch));
        }
        Ok(content)
    }

    fn referenced_content_hashes(
        &self,
        key: &OverlayKey,
    ) -> Result<Vec<String>, SkillApplicationError> {
        let layout = OverlayStorageLayout::resolve(&self.home_root, key).map_err(layout_error)?;
        if !layout.manifest_path.is_file() {
            return Ok(Vec::new());
        }
        let document = read_manifest(&layout.manifest_path)?;
        validate_document_key(key, &document)?;
        Ok(document_references(&document)?.into_iter().collect())
    }
}

fn manifest_paths(root: &Path) -> Result<Vec<PathBuf>, SkillApplicationError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(root)
        .map_err(filesystem_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(filesystem_error)?;
    let mut paths = Vec::new();
    for entry in entries {
        let file_type = entry.file_type().map_err(filesystem_error)?;
        let path = entry.path();
        if file_type.is_file() && !file_type.is_symlink() && is_overlay_manifest_path(root, &path) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_manifest(path: &Path) -> Result<OverlayDocument, SkillApplicationError> {
    let bytes = fs::read(path).map_err(filesystem_error)?;
    parse_overlay_manifest(&bytes).map_err(manifest_error)
}

fn validate_document_key(
    key: &OverlayKey,
    document: &OverlayDocument,
) -> Result<(), SkillApplicationError> {
    if document.canonical_skill_id != key.canonical_skill_id
        || document.scope() != key.scope
        || document.workspace_identity() != key.workspace_identity.as_deref()
    {
        Err(integrity(OverlayIntegrityCode::DocumentHashMismatch))
    } else {
        Ok(())
    }
}

fn validate_retained_boundary(
    key: &OverlayKey,
    document: &OverlayDocument,
) -> Result<(), SkillApplicationError> {
    let compatible = match key.scope {
        OverlayScope::Project => {
            document.scope() == OverlayScope::Project
                && document.workspace_identity() == key.workspace_identity.as_deref()
        }
        OverlayScope::System | OverlayScope::User => {
            matches!(document.scope(), OverlayScope::System | OverlayScope::User)
                && document.workspace_identity().is_none()
        }
    };
    if compatible {
        Ok(())
    } else {
        Err(integrity(OverlayIntegrityCode::DocumentHashMismatch))
    }
}

fn document_references(
    document: &OverlayDocument,
) -> Result<BTreeSet<String>, SkillApplicationError> {
    let mut references = BTreeSet::new();
    for file in &document.files {
        if !is_valid_hash(&file.content_hash)
            || file.payload_ref != format!("{HASH_ALGORITHM}/{}", file.content_hash)
        {
            return Err(integrity(OverlayIntegrityCode::DocumentHashMismatch));
        }
        references.insert(file.content_hash.clone());
    }
    Ok(references)
}

fn create_safe_directory(root: &Path, directory: &Path) -> Result<(), SkillApplicationError> {
    fs::create_dir_all(root).map_err(filesystem_error)?;
    let root_metadata = fs::symlink_metadata(root).map_err(filesystem_error)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(SkillApplicationError::Filesystem(
            "Overlay payload root is not a safe directory".to_string(),
        ));
    }
    let relative = directory.strip_prefix(root).map_err(|_| {
        SkillApplicationError::Filesystem("Overlay payload path escapes its root".to_string())
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        if let Ok(metadata) = fs::symlink_metadata(&current) {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(SkillApplicationError::Filesystem(
                    "Overlay payload directory contains an unsafe link or file".to_string(),
                ));
            }
        } else {
            fs::create_dir(&current).map_err(filesystem_error)?;
        }
    }
    Ok(())
}

fn verify_file(path: &Path, expected_hash: &str) -> Result<(), SkillApplicationError> {
    let content = fs::read(path).map_err(filesystem_error)?;
    if sha256(&content) == expected_hash {
        Ok(())
    } else {
        Err(integrity(OverlayIntegrityCode::PayloadHashMismatch))
    }
}

fn validate_hash(content_hash: &str) -> Result<(), SkillApplicationError> {
    if is_valid_hash(content_hash) {
        Ok(())
    } else {
        Err(SkillApplicationError::Validation(
            "Overlay payload hash must be 64 lowercase hexadecimal characters".to_string(),
        ))
    }
}

fn is_valid_hash(content_hash: &str) -> bool {
    content_hash.len() == 64
        && content_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_transaction_id(transaction_id: &str) -> Result<(), SkillApplicationError> {
    if !transaction_id.is_empty()
        && transaction_id.len() <= 128
        && transaction_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Ok(())
    } else {
        Err(SkillApplicationError::Validation(
            "Invalid Overlay payload transaction id".to_string(),
        ))
    }
}

fn sha256(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn integrity(code: OverlayIntegrityCode) -> SkillApplicationError {
    OverlayApplicationError::Integrity { code }.into()
}

fn manifest_error(error: OverlayManifestError) -> SkillApplicationError {
    match error {
        OverlayManifestError::UnsupportedSchemaVersion { .. }
        | OverlayManifestError::UnsupportedFutureVersion { .. } => {
            integrity(OverlayIntegrityCode::UnsupportedSchemaVersion)
        }
        OverlayManifestError::InvalidJson(_) | OverlayManifestError::InvalidDomain(_) => {
            integrity(OverlayIntegrityCode::DocumentHashMismatch)
        }
    }
}

fn layout_error(error: impl std::fmt::Display) -> SkillApplicationError {
    SkillApplicationError::Validation(error.to_string())
}

fn filesystem_error(error: std::io::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}
