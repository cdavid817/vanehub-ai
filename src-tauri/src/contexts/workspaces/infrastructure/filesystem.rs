use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::workspaces::application::{
    WorkspaceApplicationError, WorkspaceFilesystemPort,
};
use crate::contexts::workspaces::domain::{ProjectPath, WorktreeName};
use crate::platform::filesystem::{self, BoundaryError};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct WorkspaceFilesystemAdapter {
    logging: Arc<dyn DiagnosticLogPort>,
}

impl WorkspaceFilesystemAdapter {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { logging }
    }

    fn record_failure(&self, operation: &str, error: &str) {
        let mut context = BTreeMap::new();
        context.insert("operation".to_string(), operation.to_string());
        context.insert("error".to_string(), error.to_string());
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Error,
            category: "project.inspect".to_string(),
            message: "Workspace filesystem operation failed".to_string(),
            context,
        });
    }
}

impl WorkspaceFilesystemPort for WorkspaceFilesystemAdapter {
    fn canonicalize_project(
        &self,
        path: &ProjectPath,
    ) -> Result<String, WorkspaceApplicationError> {
        std::fs::canonicalize(path.as_str())
            .map(|path| path.to_string_lossy().to_string())
            .map_err(|error| {
                self.record_failure("canonicalize-project", &error.to_string());
                WorkspaceApplicationError::Validation("Project unavailable".to_string())
            })
    }

    fn sibling_worktree_target(
        &self,
        project_path: &str,
        name: &WorktreeName,
    ) -> Result<String, WorkspaceApplicationError> {
        filesystem::sibling_worktree_target(Path::new(project_path), name.as_str())
            .map(|path| path.to_string_lossy().to_string())
            .map_err(map_worktree_target_error)
    }
}

fn map_worktree_target_error(error: BoundaryError) -> WorkspaceApplicationError {
    match error {
        BoundaryError::MissingParent => {
            WorkspaceApplicationError::Validation("Project parent unavailable".to_string())
        }
        BoundaryError::MissingFileName => {
            WorkspaceApplicationError::Validation("Project name unavailable".to_string())
        }
        BoundaryError::OutsideRoot => {
            WorkspaceApplicationError::Validation("Invalid worktree target".to_string())
        }
        BoundaryError::Io(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            WorkspaceApplicationError::Validation("Git worktree target exists".to_string())
        }
        error => WorkspaceApplicationError::Filesystem(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::application::ApplicationError;
    use crate::test_support::TempDirectory;

    #[derive(Clone)]
    struct NoopLogging;

    impl DiagnosticLogPort for NoopLogging {
        fn write_diagnostic(&self, _entry: DiagnosticLog) -> Result<(), ApplicationError> {
            Ok(())
        }
    }

    #[test]
    fn canonicalization_and_sibling_target_keep_filesystem_effects_outside_the_domain() {
        let directory = TempDirectory::new("workspace-filesystem-adapter");
        let project = directory.path().join("app");
        std::fs::create_dir_all(&project).expect("project");
        let adapter = WorkspaceFilesystemAdapter::new(Arc::new(NoopLogging));
        let project_path =
            ProjectPath::parse(project.to_string_lossy().to_string()).expect("project path");
        let name = WorktreeName::parse("feature-a").expect("worktree name");

        let canonical = adapter
            .canonicalize_project(&project_path)
            .expect("canonical project");
        let target = adapter
            .sibling_worktree_target(&canonical, &name)
            .expect("target");
        let expected = std::path::PathBuf::from(&canonical)
            .parent()
            .expect("project parent")
            .join("app-feature-a");

        assert_eq!(std::path::PathBuf::from(target), expected);
    }

    #[test]
    fn existing_worktree_target_keeps_the_legacy_validation_message() {
        let directory = TempDirectory::new("workspace-existing-target");
        let project = directory.path().join("app");
        std::fs::create_dir_all(&project).expect("project");
        std::fs::create_dir_all(directory.path().join("app-feature-a")).expect("target");
        let adapter = WorkspaceFilesystemAdapter::new(Arc::new(NoopLogging));
        let name = WorktreeName::parse("feature-a").expect("worktree name");

        assert_eq!(
            adapter.sibling_worktree_target(&project.to_string_lossy(), &name),
            Err(WorkspaceApplicationError::Validation(
                "Git worktree target exists".to_string()
            ))
        );
    }
}
