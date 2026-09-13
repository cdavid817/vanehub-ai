//! Host-managed storage for Loop baselines, phase manifests and sealed acceptance artifacts.
//!
//! Objects are content-addressed copies of actual file contents, so a sealed acceptance names the
//! exact bytes it accepted rather than a hash of a tree that may since have changed. Manifests are
//! verified against their own digest on every load; a mismatch is reported as corruption and is
//! never repaired by reconstructing an invented original.

use super::loop_artifact_scan::{hash_regular_file, hex, WorkspaceManifest};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) const DEFAULT_OBJECT_BUDGET_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EvidenceStoreError {
    BudgetExceeded,
    Corrupted(String),
    Unstable(String),
    Io(String),
}

impl EvidenceStoreError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::BudgetExceeded => "evidence-budget-exceeded",
            Self::Corrupted(_) => "evidence-corrupted",
            Self::Unstable(_) => "evidence-unstable",
            Self::Io(_) => "evidence-io-error",
        }
    }

    pub(crate) fn message(&self) -> String {
        match self {
            Self::BudgetExceeded => "evidence capture exceeded its byte budget".to_string(),
            Self::Corrupted(detail) => format!("stored evidence is corrupted: {detail}"),
            Self::Unstable(path) => format!("{path} changed while it was being captured"),
            Self::Io(detail) => format!("evidence storage failed: {detail}"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LoopEvidenceStore {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureReceipt {
    pub(crate) copied_objects: usize,
    pub(crate) copied_bytes: u64,
}

impl LoopEvidenceStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    #[cfg(test)]
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    fn run_dir(&self, run_id: &str) -> PathBuf {
        let safe: String = run_id
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        self.root.join(safe)
    }

    fn object_path(&self, run_id: &str, digest: &str) -> PathBuf {
        self.run_dir(run_id).join("objects").join(digest)
    }

    fn manifest_path(&self, run_id: &str, manifest_id: &str) -> PathBuf {
        self.run_dir(run_id)
            .join("manifests")
            .join(format!("{manifest_id}.json"))
    }

    pub(crate) fn store_manifest(
        &self,
        run_id: &str,
        manifest_id: &str,
        manifest: &WorkspaceManifest,
    ) -> Result<(), EvidenceStoreError> {
        let path = self.manifest_path(run_id, manifest_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        let encoded =
            serde_json::to_vec(manifest).map_err(|error| io_error(std::io::Error::other(error)))?;
        write_atomically(&path, &encoded)
    }

    pub(crate) fn load_manifest(
        &self,
        run_id: &str,
        manifest_id: &str,
    ) -> Result<WorkspaceManifest, EvidenceStoreError> {
        let path = self.manifest_path(run_id, manifest_id);
        let raw = fs::read(&path).map_err(|error| {
            EvidenceStoreError::Corrupted(format!("manifest {manifest_id} unavailable: {error}"))
        })?;
        let manifest: WorkspaceManifest = serde_json::from_slice(&raw).map_err(|error| {
            EvidenceStoreError::Corrupted(format!("manifest {manifest_id}: {error}"))
        })?;
        if !manifest.verify_integrity() {
            return Err(EvidenceStoreError::Corrupted(format!(
                "manifest {manifest_id} digest mismatch"
            )));
        }
        Ok(manifest)
    }

    /// Copies every regular file named by the manifest into the object store, verifying the
    /// content digest while copying. A file whose bytes no longer match the manifest is reported
    /// as unstable: the snapshot the manifest describes cannot be established.
    pub(crate) fn capture_objects(
        &self,
        run_id: &str,
        worktree_root: &Path,
        manifest: &WorkspaceManifest,
        budget_bytes: u64,
    ) -> Result<CaptureReceipt, EvidenceStoreError> {
        let objects = self.run_dir(run_id).join("objects");
        fs::create_dir_all(&objects).map_err(io_error)?;
        let mut receipt = CaptureReceipt {
            copied_objects: 0,
            copied_bytes: 0,
        };
        for entry in manifest.entries.iter().filter(|entry| entry.kind == "file") {
            let Some(digest) = entry.digest.as_deref() else {
                continue;
            };
            let target = objects.join(digest);
            if target.exists() {
                continue;
            }
            receipt.copied_bytes = receipt.copied_bytes.saturating_add(entry.size);
            if receipt.copied_bytes > budget_bytes {
                return Err(EvidenceStoreError::BudgetExceeded);
            }
            let source = worktree_root.join(&entry.path);
            let (current_digest, _) = hash_regular_file(&source, &entry.path)
                .map_err(|failure| EvidenceStoreError::Unstable(failure.message()))?;
            if current_digest != digest {
                return Err(EvidenceStoreError::Unstable(entry.path.clone()));
            }
            let bytes =
                fs::read(&source).map_err(|_| EvidenceStoreError::Unstable(entry.path.clone()))?;
            if hex(&Sha256::digest(&bytes)) != digest {
                return Err(EvidenceStoreError::Unstable(entry.path.clone()));
            }
            write_atomically(&target, &bytes)?;
            receipt.copied_objects += 1;
        }
        Ok(receipt)
    }

    pub(crate) fn read_object(
        &self,
        run_id: &str,
        digest: &str,
    ) -> Result<Vec<u8>, EvidenceStoreError> {
        let path = self.object_path(run_id, digest);
        let bytes = fs::read(&path)
            .map_err(|error| EvidenceStoreError::Corrupted(format!("object {digest}: {error}")))?;
        if hex(&Sha256::digest(&bytes)) != digest {
            return Err(EvidenceStoreError::Corrupted(format!(
                "object {digest} content mismatch"
            )));
        }
        Ok(bytes)
    }

    /// Every file the manifest names must exist as an object whose bytes still hash to the
    /// recorded digest. Used before acceptance commits and when evidence is inspected later.
    pub(crate) fn verify_objects(
        &self,
        run_id: &str,
        manifest: &WorkspaceManifest,
    ) -> Result<(), EvidenceStoreError> {
        for entry in manifest.entries.iter().filter(|entry| entry.kind == "file") {
            if let Some(digest) = entry.digest.as_deref() {
                self.read_object(run_id, digest)?;
            }
        }
        Ok(())
    }
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), EvidenceStoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| io_error(std::io::Error::other("no parent")))?;
    fs::create_dir_all(parent).map_err(io_error)?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        uuid::Uuid::new_v4()
    ));
    let outcome = (|| -> std::io::Result<()> {
        let mut file = crate::platform::filesystem::create_new_file(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if outcome.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    outcome.map_err(io_error)
}

fn io_error(error: std::io::Error) -> EvidenceStoreError {
    EvidenceStoreError::Io(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::infrastructure::loop_artifact_scan::{
        scan_workspace, ScanBudget,
    };
    use crate::test_support::TempDirectory;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn objects_and_manifests_round_trip_and_detect_corruption() {
        let workspace = TempDirectory::new("evidence-store-ws");
        let storage = TempDirectory::new("evidence-store");
        workspace.write("src/a.txt", "alpha");
        workspace.write("src/b.txt", "beta");
        let manifest = scan_workspace(
            workspace.path(),
            &ScanBudget::default(),
            &AtomicBool::new(false),
        )
        .expect("scan");
        let store = LoopEvidenceStore::new(storage.path().to_path_buf());
        store
            .store_manifest("run-1", "baseline", &manifest)
            .expect("store manifest");
        let receipt = store
            .capture_objects(
                "run-1",
                workspace.path(),
                &manifest,
                DEFAULT_OBJECT_BUDGET_BYTES,
            )
            .expect("capture");
        assert_eq!(receipt.copied_objects, 2);
        assert_eq!(
            store.load_manifest("run-1", "baseline").expect("load"),
            manifest
        );
        store
            .verify_objects("run-1", &manifest)
            .expect("objects intact");
        let digest = manifest
            .find("src/a.txt")
            .and_then(|entry| entry.digest.clone())
            .expect("digest");
        assert_eq!(
            store.read_object("run-1", &digest).expect("object"),
            b"alpha"
        );

        // A second capture copies nothing new.
        let again = store
            .capture_objects(
                "run-1",
                workspace.path(),
                &manifest,
                DEFAULT_OBJECT_BUDGET_BYTES,
            )
            .expect("capture again");
        assert_eq!(again.copied_objects, 0);

        // Tampering with a stored object or manifest is detected, never repaired.
        fs::write(
            store.root().join("run-1/objects").join(&digest),
            b"tampered",
        )
        .expect("tamper");
        assert_eq!(
            store
                .read_object("run-1", &digest)
                .expect_err("corrupt")
                .code(),
            "evidence-corrupted"
        );
        assert_eq!(
            store
                .verify_objects("run-1", &manifest)
                .expect_err("corrupt")
                .code(),
            "evidence-corrupted"
        );
        let manifest_path = store.root().join("run-1/manifests/baseline.json");
        let mut raw = fs::read_to_string(&manifest_path).expect("raw");
        raw = raw.replacen("\"size\":5", "\"size\":6", 1);
        fs::write(&manifest_path, raw).expect("tamper manifest");
        assert_eq!(
            store
                .load_manifest("run-1", "baseline")
                .expect_err("corrupt")
                .code(),
            "evidence-corrupted"
        );
    }

    #[test]
    fn capture_reports_budget_overflow_and_content_drift() {
        let workspace = TempDirectory::new("evidence-store-drift");
        let storage = TempDirectory::new("evidence-store-drift-store");
        workspace.write("a.txt", "0123456789");
        let manifest = scan_workspace(
            workspace.path(),
            &ScanBudget::default(),
            &AtomicBool::new(false),
        )
        .expect("scan");
        let store = LoopEvidenceStore::new(storage.path().to_path_buf());
        assert_eq!(
            store
                .capture_objects("run-2", workspace.path(), &manifest, 4)
                .expect_err("budget")
                .code(),
            "evidence-budget-exceeded"
        );
        workspace.write("a.txt", "changed-after-scan");
        assert_eq!(
            store
                .capture_objects(
                    "run-2",
                    workspace.path(),
                    &manifest,
                    DEFAULT_OBJECT_BUDGET_BYTES
                )
                .expect_err("drift")
                .code(),
            "evidence-unstable"
        );
    }
}
