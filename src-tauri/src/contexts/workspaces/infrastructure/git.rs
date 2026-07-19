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
}
