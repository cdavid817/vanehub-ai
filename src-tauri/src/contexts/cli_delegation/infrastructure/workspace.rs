use crate::contexts::cli_delegation::application::{
    DelegationRepositoryBaseline, DelegationWorkspace, DelegationWorkspaceError,
    DelegationWorkspacePort,
};
use crate::platform::git::GitAdapter;
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

const GIT_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) struct IndependentGitWorkspaceAdapter {
    operations_root: PathBuf,
    git: GitAdapter,
}

impl IndependentGitWorkspaceAdapter {
    pub(crate) fn new(operations_root: PathBuf) -> Self {
        Self {
            operations_root,
            git: GitAdapter::default(),
        }
    }

    fn run(&self, root: &Path, arguments: &[&str]) -> Result<String, DelegationWorkspaceError> {
        let arguments = arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect::<Vec<_>>();
        let output = self
            .git
            .execute(root, &arguments, GIT_TIMEOUT)
            .map_err(|_| DelegationWorkspaceError::GitFailure)?;
        if !output.status.success() {
            #[cfg(test)]
            eprintln!(
                "git {:?} failed in {}: {}",
                arguments,
                root.display(),
                String::from_utf8_lossy(&output.stderr)
            );
            return Err(DelegationWorkspaceError::GitFailure);
        }
        String::from_utf8(output.stdout)
            .map(|value| value.trim().to_owned())
            .map_err(|_| DelegationWorkspaceError::VerificationFailure)
    }

    fn create_layout(&self, attempt_root: &Path) -> Result<(), DelegationWorkspaceError> {
        if attempt_root.exists() {
            return Err(DelegationWorkspaceError::TargetExists);
        }
        std::fs::create_dir_all(attempt_root.join("inputs"))
            .and_then(|_| std::fs::create_dir(attempt_root.join("output")))
            .and_then(|_| std::fs::create_dir(attempt_root.join("control")))
            .and_then(|_| std::fs::create_dir(attempt_root.join("recovery")))
            .map_err(|_| DelegationWorkspaceError::TargetExists)
    }

    fn verify_independent_clone(
        &self,
        workspace: &Path,
        expected_commit: &str,
    ) -> Result<(), DelegationWorkspaceError> {
        let actual = self.run(workspace, &["rev-parse", "HEAD"])?;
        let symbolic = self.run(workspace, &["symbolic-ref", "-q", "HEAD"]);
        let remotes = self.run(workspace, &["remote"])?;
        let git_dir = workspace.join(".git");
        let alternates = git_dir.join("objects/info/alternates");
        if !actual.eq_ignore_ascii_case(expected_commit)
            || symbolic.is_ok()
            || !remotes.is_empty()
            || !git_dir.is_dir()
            || alternates.exists()
        {
            #[cfg(test)]
            eprintln!(
                "clone verification actual={actual} expected={expected_commit} symbolic={} remotes={remotes:?} git_dir={} alternates={}",
                symbolic.is_ok(),
                git_dir.is_dir(),
                alternates.exists()
            );
            return Err(DelegationWorkspaceError::VerificationFailure);
        }
        Ok(())
    }
}

impl DelegationWorkspacePort for IndependentGitWorkspaceAdapter {
    fn inspect_baseline(
        &self,
        source_repository: &Path,
        expected_commit: &str,
    ) -> Result<DelegationRepositoryBaseline, DelegationWorkspaceError> {
        if !valid_commit(expected_commit) {
            return Err(DelegationWorkspaceError::InvalidRequest);
        }
        let source = source_repository
            .canonicalize()
            .map_err(|_| DelegationWorkspaceError::SourceUnavailable)?;
        super::repository_preflight::inspect(&self.git, &source, expected_commit, GIT_TIMEOUT)
    }

    fn create(
        &self,
        source_repository: &Path,
        exact_commit: &str,
    ) -> Result<DelegationWorkspace, DelegationWorkspaceError> {
        if !valid_commit(exact_commit) {
            return Err(DelegationWorkspaceError::InvalidRequest);
        }
        let source = source_repository
            .canonicalize()
            .map_err(|_| DelegationWorkspaceError::SourceUnavailable)?;
        if !source.is_dir() {
            return Err(DelegationWorkspaceError::SourceUnavailable);
        }
        let baseline =
            super::repository_preflight::inspect(&self.git, &source, exact_commit, GIT_TIMEOUT)?;
        std::fs::create_dir_all(&self.operations_root)
            .map_err(|_| DelegationWorkspaceError::TargetExists)?;
        let operations_root = self
            .operations_root
            .canonicalize()
            .map_err(|_| DelegationWorkspaceError::TargetExists)?;
        let attempt_root = operations_root.join(format!("delegation-{}", Uuid::new_v4()));
        self.create_layout(&attempt_root)?;
        let workspace = attempt_root.join("workspace");
        let hooks = attempt_root.join("control/hooks");
        if std::fs::create_dir(&hooks).is_err() {
            let _ = std::fs::remove_dir_all(&attempt_root);
            return Err(DelegationWorkspaceError::TargetExists);
        }
        let source_value = git_path(&source);
        let workspace_value = git_path(&workspace);
        let clone = self.run(
            &operations_root,
            &[
                "clone",
                "--no-hardlinks",
                "--no-checkout",
                "--no-tags",
                "--no-recurse-submodules",
                "--",
                &source_value,
                &workspace_value,
            ],
        );
        if clone.is_err()
            || self
                .run(
                    &workspace,
                    &["checkout", "--detach", "--force", exact_commit],
                )
                .is_err()
            || self
                .run(&workspace, &["remote", "remove", "origin"])
                .is_err()
            || self
                .run(
                    &workspace,
                    &["config", "--local", "core.hooksPath", &git_path(&hooks)],
                )
                .is_err()
            || self
                .verify_independent_clone(&workspace, exact_commit)
                .is_err()
        {
            let _ = std::fs::remove_dir_all(&attempt_root);
            return Err(DelegationWorkspaceError::VerificationFailure);
        }
        Ok(DelegationWorkspace {
            workspace,
            inputs: attempt_root.join("inputs"),
            output: attempt_root.join("output"),
            control: attempt_root.join("control"),
            recovery: attempt_root.join("recovery"),
            attempt_root,
            repository_identity: baseline.repository_identity,
            base_commit: baseline.head_commit,
        })
    }

    fn cleanup(&self, workspace: &DelegationWorkspace) -> Result<(), DelegationWorkspaceError> {
        let root = self
            .operations_root
            .canonicalize()
            .map_err(|_| DelegationWorkspaceError::CleanupFailure)?;
        let target = workspace
            .attempt_root
            .canonicalize()
            .map_err(|_| DelegationWorkspaceError::CleanupFailure)?;
        if target.parent() != Some(root.as_path()) {
            return Err(DelegationWorkspaceError::CleanupFailure);
        }
        std::fs::remove_dir_all(target).map_err(|_| DelegationWorkspaceError::CleanupFailure)
    }
}

fn valid_commit(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn git_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    if let Some(stripped) = value.strip_prefix(r"\\?\") {
        return stripped.to_owned();
    }
    value.into_owned()
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
