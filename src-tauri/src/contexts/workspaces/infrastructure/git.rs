use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::workspaces::application::{WorkspaceApplicationError, WorkspaceGitPort};
use crate::platform::git::{GitAdapter, GitOutput};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

const GIT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub(crate) struct WorkspaceGitAdapter {
    git: GitAdapter,
    logging: Arc<dyn DiagnosticLogPort>,
}

impl WorkspaceGitAdapter {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self {
            git: GitAdapter::default(),
            logging,
        }
    }

    fn record(&self, severity: LogSeverity, category: &str, operation: &str, message: String) {
        let mut context = BTreeMap::new();
        context.insert("operation".to_string(), operation.to_string());
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity,
            category: category.to_string(),
            message,
            context,
        });
    }

    fn diagnostic(operation: &str, root: &Path, output: &GitOutput) -> String {
        GitAdapter::redacted_diagnostic(operation, root, output)
    }
}

impl WorkspaceGitPort for WorkspaceGitAdapter {
    fn repository_root(
        &self,
        project_path: &str,
    ) -> Result<Option<String>, WorkspaceApplicationError> {
        let root = Path::new(project_path);
        let args = vec!["rev-parse".to_string(), "--show-toplevel".to_string()];
        let output = match self.git.execute(root, &args, GIT_TIMEOUT) {
            Ok(output) => output,
            Err(error) => {
                self.record(
                    LogSeverity::Error,
                    "git.project",
                    "inspect",
                    crate::platform::logging::redact_text(&format!(
                        "Git command unavailable: {error}"
                    )),
                );
                return Ok(None);
            }
        };
        if !output.status.success() {
            self.record(
                LogSeverity::Info,
                "git.project",
                "inspect",
                Self::diagnostic("inspect", root, &output),
            );
            return Ok(None);
        }
        let repository_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok((!repository_root.is_empty()).then_some(repository_root))
    }

    fn create_worktree(
        &self,
        project_path: &str,
        target_path: &str,
        branch: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        let root = Path::new(project_path);
        let args = vec![
            "worktree".to_string(),
            "add".to_string(),
            target_path.to_string(),
            "-b".to_string(),
            branch.to_string(),
        ];
        let output = self
            .git
            .execute(root, &args, GIT_TIMEOUT)
            .map_err(|error| {
                self.record(
                    LogSeverity::Error,
                    "git.worktree",
                    "create",
                    crate::platform::logging::redact_text(&format!(
                        "Git command unavailable: {error}"
                    )),
                );
                WorkspaceApplicationError::Validation("Git unavailable".to_string())
            })?;
        if !output.status.success() {
            self.record(
                LogSeverity::Error,
                "git.worktree",
                "create",
                Self::diagnostic("worktree-add", root, &output),
            );
            return Err(WorkspaceApplicationError::Validation(
                "Git worktree failed".to_string(),
            ));
        }
        self.record(
            LogSeverity::Info,
            "git.worktree",
            "create",
            "Git worktree created".to_string(),
        );
        Ok(())
    }

    fn validate_loop_worktree(
        &self,
        project_path: &str,
        target_path: &str,
        branch: &str,
        base_branch: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        let root = Path::new(project_path);
        let base = self
            .git
            .execute(
                root,
                &[
                    "rev-parse".to_string(),
                    "--verify".to_string(),
                    "--quiet".to_string(),
                    format!("{base_branch}^{{commit}}"),
                ],
                GIT_TIMEOUT,
            )
            .map_err(|error| WorkspaceApplicationError::Validation(error.to_string()))?;
        if !base.status.success() {
            return Err(WorkspaceApplicationError::Validation(
                "Loop base branch does not resolve to a commit.".to_string(),
            ));
        }

        let branch_probe = self
            .git
            .execute(
                root,
                &[
                    "show-ref".to_string(),
                    "--verify".to_string(),
                    "--quiet".to_string(),
                    format!("refs/heads/{branch}"),
                ],
                GIT_TIMEOUT,
            )
            .map_err(|error| WorkspaceApplicationError::Validation(error.to_string()))?;
        if branch_probe.status.success() {
            return Err(WorkspaceApplicationError::Validation(
                "Loop worktree branch already exists.".to_string(),
            ));
        }

        let worktrees = self
            .git
            .execute(
                root,
                &[
                    "worktree".to_string(),
                    "list".to_string(),
                    "--porcelain".to_string(),
                ],
                GIT_TIMEOUT,
            )
            .map_err(|error| WorkspaceApplicationError::Validation(error.to_string()))?;
        if !worktrees.status.success() {
            return Err(WorkspaceApplicationError::Validation(
                "Unable to inspect Git worktrees.".to_string(),
            ));
        }
        let target = normalized_path(target_path);
        let collision = String::from_utf8_lossy(&worktrees.stdout)
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .any(|path| normalized_path(path) == target);
        if collision {
            return Err(WorkspaceApplicationError::Validation(
                "Loop worktree target is already registered.".to_string(),
            ));
        }
        Ok(())
    }

    fn create_loop_worktree(
        &self,
        project_path: &str,
        target_path: &str,
        branch: &str,
        base_branch: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        let root = Path::new(project_path);
        let output = self
            .git
            .execute(
                root,
                &[
                    "worktree".to_string(),
                    "add".to_string(),
                    "-b".to_string(),
                    branch.to_string(),
                    target_path.to_string(),
                    base_branch.to_string(),
                ],
                GIT_TIMEOUT,
            )
            .map_err(|error| WorkspaceApplicationError::Validation(error.to_string()))?;
        if !output.status.success() {
            self.record(
                LogSeverity::Error,
                "git.worktree",
                "loop-create",
                Self::diagnostic("loop-worktree-add", root, &output),
            );
            return Err(WorkspaceApplicationError::Validation(
                "Loop Git worktree creation failed.".to_string(),
            ));
        }
        self.record(
            LogSeverity::Info,
            "git.worktree",
            "loop-create",
            "Loop Git worktree created".to_string(),
        );
        Ok(())
    }
}

fn normalized_path(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}
