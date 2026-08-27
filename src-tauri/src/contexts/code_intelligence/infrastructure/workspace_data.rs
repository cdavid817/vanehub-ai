//! Writable per-workspace state a language server owns.
//!
//! `jdtls` writes an index into a `-data` directory and expects it back on the next start. Two
//! workspaces sharing one would let each read the other's; a directory named after the workspace
//! path would put that path in a filename. So it is derived from a hash of the canonical root,
//! under a directory VaneHub owns.
//!
//! Derived rather than recorded on purpose: there is no table to keep in step with trust, and the
//! only way to reach a workspace's directory is to already have its canonical root.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Where a language's per-workspace directories live, under the caller-supplied application data
/// directory.
fn language_root(app_data: &Path, language_id: &str) -> PathBuf {
    app_data.join("lsp").join(language_id).join("workspaces")
}

/// This workspace's directory for this language.
///
/// The name is a hash, so a workspace path never lands in a directory name — the parent is
/// VaneHub's, but the name would otherwise be the user's absolute path, visible to anything that
/// can list the parent.
pub(crate) fn workspace_data_directory(
    app_data: &Path,
    language_id: &str,
    canonical_root: &Path,
) -> PathBuf {
    let mut digest = Sha256::new();
    digest.update(canonical_root.to_string_lossy().as_bytes());
    let name = digest
        .finalize()
        .iter()
        .take(16)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    language_root(app_data, language_id).join(name)
}

/// Removes every language's data directory for one workspace.
///
/// Called when trust is revoked. Best effort per language: one language's directory being locked
/// must not stop the others from going, because the point is that a revoked workspace stops having
/// a server-built index of its source on disk.
pub(crate) fn remove_workspace_data(
    app_data: &Path,
    language_ids: impl IntoIterator<Item = &'static str>,
    canonical_root: &Path,
) {
    for language_id in language_ids {
        let directory = workspace_data_directory(app_data, language_id, canonical_root);
        if directory.is_dir() {
            let _ = std::fs::remove_dir_all(&directory);
        }
    }
}

#[cfg(test)]
#[path = "workspace_data_tests.rs"]
mod tests;
