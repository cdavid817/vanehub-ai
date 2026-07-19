use super::{
    CreatedWorktree, KnownProject, KnownRemoteWorkspace, ProjectDirectorySelectionPort,
    WorkspaceApplicationError, WorkspaceClockPort, WorkspaceFilesystemPort, WorkspaceGitPort,
    WorkspaceHistoryRepository,
};
use crate::contexts::workspaces::domain::{
    ProjectInspection, ProjectPath, RemoteWorkspace, WorktreeName,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct WorkspaceApplicationService {
    history: Arc<dyn WorkspaceHistoryRepository>,
    filesystem: Arc<dyn WorkspaceFilesystemPort>,
    git: Arc<dyn WorkspaceGitPort>,
    selection: Arc<dyn ProjectDirectorySelectionPort>,
    clock: Arc<dyn WorkspaceClockPort>,
}

impl WorkspaceApplicationService {
    pub(crate) fn new(
        history: Arc<dyn WorkspaceHistoryRepository>,
        filesystem: Arc<dyn WorkspaceFilesystemPort>,
        git: Arc<dyn WorkspaceGitPort>,
        selection: Arc<dyn ProjectDirectorySelectionPort>,
        clock: Arc<dyn WorkspaceClockPort>,
    ) -> Self {
        Self {
            history,
            filesystem,
            git,
            selection,
            clock,
        }
    }

    pub(crate) fn list_known_projects(
        &self,
    ) -> Result<Vec<KnownProject>, WorkspaceApplicationError> {
        self.history.list_projects()
    }

    pub(crate) fn list_known_remote_workspaces(
        &self,
    ) -> Result<Vec<KnownRemoteWorkspace>, WorkspaceApplicationError> {
        self.history.list_remote_workspaces()
    }

    pub(crate) fn inspect_project(
        &self,
        path: &str,
    ) -> Result<ProjectInspection, WorkspaceApplicationError> {
        let requested = ProjectPath::parse(path.to_string())?;
        let canonical = self.filesystem.canonicalize_project(&requested)?;
        let git_root = self.git.repository_root(&canonical)?;
        ProjectInspection::from_probe(canonical, git_root).map_err(Into::into)
    }

    pub(crate) fn remember_project(
        &self,
        inspection: &ProjectInspection,
    ) -> Result<(), WorkspaceApplicationError> {
        self.history.remember_project(inspection, &self.clock.now())
    }

    pub(crate) fn remember_remote_workspace(
        &self,
        workspace: &RemoteWorkspace,
    ) -> Result<(), WorkspaceApplicationError> {
        self.history
            .remember_remote_workspace(workspace, &self.clock.now())
    }

    pub(crate) fn select_project_directory(
        &self,
    ) -> Result<Option<String>, WorkspaceApplicationError> {
        self.selection.select_directory()
    }

    pub(crate) fn create_worktree(
        &self,
        project_path: &str,
        name: &str,
    ) -> Result<CreatedWorktree, WorkspaceApplicationError> {
        let project = ProjectPath::parse(project_path.to_string())?;
        let name = WorktreeName::parse(name.to_string())?;
        let target = self
            .filesystem
            .sibling_worktree_target(project.as_str(), &name)?;
        let branch = name.branch_name();
        self.git
            .create_worktree(project.as_str(), &target, &branch)?;
        Ok(CreatedWorktree {
            path: target,
            name: name.as_str().to_string(),
            branch,
        })
    }
}
