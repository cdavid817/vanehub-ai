//! Worktree isolation and ChangeSet capture for a mutating subagent child
//! (`add-onepiece-subagents` D8).
//!
//! A mutating child never writes into the parent's workspace. It gets a detached git worktree of
//! the parent's exact base commit, edits there, and its result is captured as a ChangeSet before
//! the worktree is reaped. Capturing without sealing would leave the child's edits in a directory
//! about to be deleted and have the model report work that no longer exists, so the capture and
//! the reap are ordered, not concurrent.

use crate::platform::git::GitAdapter;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

const GIT_TIMEOUT: Duration = Duration::from_secs(120);

/// Files a single child ChangeSet may carry. A child that touched more than this has stopped
/// being a bounded edit and become a refactor nobody reviewed.
pub(crate) const MAX_CHANGE_SET_FILES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorktreeError {
    /// The parent workspace is not a git repository, so there is nothing to branch from and no
    /// base commit to bind a ChangeSet to.
    NotARepository,
    /// The parent workspace has uncommitted changes. A ChangeSet captured against a dirty tree
    /// cannot state what it applies to.
    WorkspaceNotClean,
    /// Git refused to provision or reap the worktree.
    GitFailure,
    /// The child changed more files than a reviewable ChangeSet may carry.
    TooManyFiles,
}

impl WorktreeError {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::NotARepository => {
                "A subagent can only make changes inside a git repository, and this workspace is not one."
            }
            Self::WorkspaceNotClean => {
                "Commit or stash the current changes first: a subagent's changes are captured against an exact base commit."
            }
            Self::GitFailure => "An isolated workspace could not be prepared for the subagent.",
            Self::TooManyFiles => {
                "The subagent changed more files than one reviewable change set may carry."
            }
        }
    }
}

/// One changed file, in the shape the shared ChangeSet record wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapturedFile {
    pub(crate) path: String,
    pub(crate) status: char,
    pub(crate) new_hash: Option<String>,
    pub(crate) binary: bool,
}

/// A provisioned worktree. Reaped on drop so no exit path -- success, limit, cancellation, or an
/// early return added later -- can leave one behind.
#[derive(Debug)]
pub(crate) struct ChildWorktree {
    repository_root: PathBuf,
    path: PathBuf,
    base_commit: String,
    repository_identity: String,
    git: GitAdapter,
}

impl ChildWorktree {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn base_commit(&self) -> &str {
        &self.base_commit
    }

    pub(crate) fn repository_identity(&self) -> &str {
        &self.repository_identity
    }

    /// Provisions a detached worktree at the workspace's current commit.
    ///
    /// Both preflights are refusals rather than warnings: without a repository there is no base
    /// commit to bind to, and against a dirty tree a captured ChangeSet cannot say what it applies
    /// to.
    pub(crate) fn provision(
        workspace: &Path,
        operations_root: &Path,
    ) -> Result<Self, WorktreeError> {
        let git = GitAdapter::default();
        let run = |root: &Path, args: &[&str]| -> Result<String, WorktreeError> {
            let args = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
            let output = git
                .execute(root, &args, GIT_TIMEOUT)
                .map_err(|_| WorktreeError::GitFailure)?;
            if !output.status.success() {
                return Err(WorktreeError::GitFailure);
            }
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        };

        let repository_root = run(workspace, &["rev-parse", "--show-toplevel"])
            .map_err(|_| WorktreeError::NotARepository)?;
        if repository_root.is_empty() {
            return Err(WorktreeError::NotARepository);
        }
        let repository_root = PathBuf::from(repository_root);
        let base_commit = run(&repository_root, &["rev-parse", "HEAD"])
            .map_err(|_| WorktreeError::NotARepository)?;
        if !run(&repository_root, &["status", "--porcelain"])?.is_empty() {
            return Err(WorktreeError::WorkspaceNotClean);
        }

        let path = operations_root.join(format!("subagent-{}", Uuid::new_v4()));
        let path_text = path.to_string_lossy().into_owned();
        run(
            &repository_root,
            &["worktree", "add", "--detach", &path_text, &base_commit],
        )?;

        Ok(Self {
            repository_identity: identity(&repository_root),
            repository_root,
            path,
            base_commit,
            git,
        })
    }

    /// Captures what the child changed, as paths plus content hashes. Must run before the worktree
    /// is reaped, which `Drop` does.
    pub(crate) fn capture(&self) -> Result<Vec<CapturedFile>, WorktreeError> {
        let args = ["status", "--porcelain", "--untracked-files=all"]
            .iter()
            .map(|arg| (*arg).to_owned())
            .collect::<Vec<_>>();
        let output = self
            .git
            .execute(&self.path, &args, GIT_TIMEOUT)
            .map_err(|_| WorktreeError::GitFailure)?;
        if !output.status.success() {
            return Err(WorktreeError::GitFailure);
        }

        let mut captured = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let Some(entry) = parse_status_line(line) else {
                continue;
            };
            if captured.len() >= MAX_CHANGE_SET_FILES {
                return Err(WorktreeError::TooManyFiles);
            }
            captured.push(entry);
        }
        Ok(captured
            .into_iter()
            .map(|(status, path)| {
                let absolute = self.path.join(&path);
                let bytes = std::fs::read(&absolute).ok();
                CapturedFile {
                    new_hash: bytes.as_ref().map(|content| digest(content)),
                    binary: bytes.as_deref().is_some_and(is_binary),
                    path,
                    status,
                }
            })
            .collect())
    }

    /// The unified diff the ChangeSet artifact carries. Includes untracked files so an added file
    /// is reviewable rather than a bare path.
    pub(crate) fn diff(&self) -> Result<Vec<u8>, WorktreeError> {
        let args = ["add", "--intent-to-add", "--all"]
            .iter()
            .map(|arg| (*arg).to_owned())
            .collect::<Vec<_>>();
        let staged = self
            .git
            .execute(&self.path, &args, GIT_TIMEOUT)
            .map_err(|_| WorktreeError::GitFailure)?;
        if !staged.status.success() {
            return Err(WorktreeError::GitFailure);
        }
        let args = ["diff", "--binary", "HEAD"]
            .iter()
            .map(|arg| (*arg).to_owned())
            .collect::<Vec<_>>();
        let output = self
            .git
            .execute(&self.path, &args, GIT_TIMEOUT)
            .map_err(|_| WorktreeError::GitFailure)?;
        if !output.status.success() {
            return Err(WorktreeError::GitFailure);
        }
        Ok(output.stdout)
    }
}

impl Drop for ChildWorktree {
    fn drop(&mut self) {
        let path = self.path.to_string_lossy().into_owned();
        let args = ["worktree", "remove", "--force", &path]
            .iter()
            .map(|arg| (*arg).to_owned())
            .collect::<Vec<_>>();
        let _ = self.git.execute(&self.repository_root, &args, GIT_TIMEOUT);
        // `worktree remove` leaves the directory behind if git considers it dirty in a way it
        // cannot resolve; removing it directly is the backstop so a cancelled child cannot
        // accumulate directories.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Parses one `git status --porcelain` line into its status letter and path. Renames report
/// `old -> new`; the new path is what the ChangeSet carries.
fn parse_status_line(line: &str) -> Option<(char, String)> {
    if line.len() < 4 {
        return None;
    }
    let status = line
        .chars()
        .take(2)
        .find(|character| !character.is_whitespace())
        .unwrap_or('M');
    let path = line[3..].trim();
    let path = path.rsplit(" -> ").next().unwrap_or(path);
    let path = path.trim_matches('"');
    (!path.is_empty()).then(|| (status, path.to_owned()))
}

fn digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8000).any(|byte| *byte == 0)
}

fn identity(repository_root: &Path) -> String {
    format!(
        "repo-{}",
        digest(repository_root.to_string_lossy().as_bytes())
            .chars()
            .take(32)
            .collect::<String>()
    )
}

#[cfg(test)]
#[path = "subagent_worktree_tests.rs"]
mod tests;
