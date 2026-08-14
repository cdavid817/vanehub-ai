use super::apply_backend::NativeApplyBackend;
use crate::contexts::cli_delegation::application::{
    DelegationApplyOncePort, DelegationApplyPathExpectation, DelegationApplyPathWitness,
    DelegationApplyPlan, DelegationApplyStagingPort, DelegationApplyTargetPort,
    DelegationApplyTargetWitness, DelegationRecoveryCapsule,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub(super) struct RecoveryEntry {
    pub(super) path: String,
    pub(super) existed: bool,
}

impl DelegationApplyTargetPort for NativeApplyBackend {
    fn inspect_target(&self, root: &Path) -> Result<DelegationApplyTargetWitness, ()> {
        let canonical_root = root.canonicalize().map_err(|_| ())?;
        if canonical_root != root {
            return Err(());
        }
        let head = String::from_utf8(self.run(root, &["rev-parse", "HEAD"])?).map_err(|_| ())?;
        let head = head.trim().to_ascii_lowercase();
        let status = self.run(root, &["status", "--porcelain=v1", "--untracked-files=all"])?;
        Ok(DelegationApplyTargetWitness {
            canonical_root,
            repository_identity: format!("git:{}:{}", root.to_string_lossy(), head),
            head_commit: head,
            worktree_clean: status.is_empty(),
            index_clean: self.run(root, &["diff", "--cached", "--quiet"]).is_ok(),
            path_compatible: true,
        })
    }
}

impl DelegationApplyOncePort for NativeApplyBackend {
    fn is_available(&self, artifact_id: &str, _: &str) -> Result<bool, ()> {
        self.repository
            .is_change_set_available(artifact_id)
            .map_err(|_| ())
    }
}

impl DelegationApplyStagingPort for NativeApplyBackend {
    fn inspect_paths(
        &self,
        plan: &DelegationApplyPlan,
        expectations: &[DelegationApplyPathExpectation],
    ) -> Result<Vec<DelegationApplyPathWitness>, ()> {
        expectations
            .iter()
            .map(|item| inspect_path(self, &plan.target_root, &item.path))
            .collect()
    }

    fn stage_recovery_capsule(
        &self,
        plan: &DelegationApplyPlan,
        id: &str,
        expectations: &[DelegationApplyPathExpectation],
    ) -> Result<DelegationRecoveryCapsule, ()> {
        let root = self.capsule_root(id);
        fs::create_dir(&root).map_err(|_| ())?;
        let entries = expectations
            .iter()
            .map(|item| RecoveryEntry {
                path: item.path.clone(),
                existed: item.must_exist,
            })
            .collect::<Vec<_>>();
        for entry in entries.iter().filter(|item| item.existed) {
            let destination = root.join("files").join(&entry.path);
            fs::create_dir_all(destination.parent().ok_or(())?).map_err(|_| ())?;
            fs::copy(plan.target_root.join(&entry.path), destination).map_err(|_| ())?;
        }
        let manifest = serde_json::to_vec(&entries).map_err(|_| ())?;
        fs::write(root.join("manifest.json"), &manifest).map_err(|_| ())?;
        Ok(DelegationRecoveryCapsule {
            apply_attempt_id: id.to_owned(),
            reference: root.to_string_lossy().to_string(),
            witness_hash: sha256(&manifest),
        })
    }
}

fn inspect_path(
    backend: &NativeApplyBackend,
    root: &Path,
    path: &str,
) -> Result<DelegationApplyPathWitness, ()> {
    let target = root.join(path);
    let metadata = match fs::symlink_metadata(&target) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(()),
    };
    if metadata
        .as_ref()
        .is_some_and(|value| !value.is_file() || value.file_type().is_symlink())
    {
        return Err(());
    }
    let exists = metadata.is_some();
    let (mode, git_hash) = if exists {
        let stage = String::from_utf8(backend.run(root, &["ls-files", "-s", "--", path])?)
            .map_err(|_| ())?;
        let hash =
            String::from_utf8(backend.run(root, &["hash-object", "--", path])?).map_err(|_| ())?;
        (
            stage.split_whitespace().next().map(str::to_owned),
            Some(hash.trim().to_owned()),
        )
    } else {
        (None, None)
    };
    Ok(DelegationApplyPathWitness {
        path: path.to_owned(),
        exists,
        mode,
        git_hash,
    })
}

fn sha256(bytes: &[u8]) -> String {
    format!(
        "sha256:{}",
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}
