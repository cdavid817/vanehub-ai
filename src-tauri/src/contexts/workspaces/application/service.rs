use super::{
    CreatedWorktree, GitBranchReference, KnownProject, KnownRemoteWorkspace, PlannedWorktree,
    ProjectDirectorySelectionPort, WorkspaceApplicationError, WorkspaceClockPort,
    WorkspaceFilesystemPort, WorkspaceGitPort, WorkspaceHistoryRepository,
};
use crate::contexts::workspaces::domain::{
    GitReference, ProjectInspection, ProjectPath, RemoteWorkspace, WorktreeName,
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

    pub(crate) fn list_git_branches(
        &self,
        project_path: &str,
    ) -> Result<Vec<GitBranchReference>, WorkspaceApplicationError> {
        let requested = ProjectPath::parse(project_path.to_string())?;
        let canonical = self.filesystem.canonicalize_project(&requested)?;
        let root = self.git.repository_root(&canonical)?.ok_or_else(|| {
            WorkspaceApplicationError::Validation(
                "Loop branch discovery requires a local Git repository.".to_string(),
            )
        })?;
        self.git.list_branches(&root, 200)
    }

    pub(crate) fn create_worktree(
        &self,
        project_path: &str,
        name: &str,
    ) -> Result<CreatedWorktree, WorkspaceApplicationError> {
        let plan = self.plan_worktree(project_path, name)?;
        self.create_planned_worktree(&plan)
    }

    /// Validates the request and settles the target path without running Git, so a caller can
    /// record its intent against the exact directory before the directory exists.
    pub(crate) fn plan_worktree(
        &self,
        project_path: &str,
        name: &str,
    ) -> Result<PlannedWorktree, WorkspaceApplicationError> {
        let project = ProjectPath::parse(project_path.to_string())?;
        let name = WorktreeName::parse(name.to_string())?;
        let target = self
            .filesystem
            .sibling_worktree_target(project.as_str(), &name)?;
        Ok(PlannedWorktree {
            project: project.as_str().to_string(),
            target,
            branch: name.branch_name(),
            name: name.as_str().to_string(),
        })
    }

    pub(crate) fn create_planned_worktree(
        &self,
        plan: &PlannedWorktree,
    ) -> Result<CreatedWorktree, WorkspaceApplicationError> {
        self.git
            .create_worktree(&plan.project, &plan.target, &plan.branch)?;
        Ok(CreatedWorktree {
            path: plan.target.clone(),
            name: plan.name.clone(),
            branch: plan.branch.clone(),
            worktree_id: None,
        })
    }

    pub(crate) fn create_guarded_loop_worktree(
        &self,
        project_path: &str,
        name: &str,
        base_branch: &str,
    ) -> Result<CreatedWorktree, WorkspaceApplicationError> {
        let requested = ProjectPath::parse(project_path.to_string())?;
        let name = WorktreeName::parse(name.to_string())?;
        let base_branch = GitReference::parse(base_branch.to_string())?;
        let canonical = self.filesystem.canonicalize_project(&requested)?;
        let root = self.git.repository_root(&canonical)?.ok_or_else(|| {
            WorkspaceApplicationError::Validation(
                "Loop worktrees require a local Git repository.".to_string(),
            )
        })?;
        let target = self.filesystem.sibling_worktree_target(&root, &name)?;
        let branch = name.branch_name();
        self.git
            .validate_loop_worktree(&root, &target, &branch, base_branch.as_str())?;
        self.git
            .create_loop_worktree(&root, &target, &branch, base_branch.as_str())?;
        Ok(CreatedWorktree {
            path: target,
            name: name.as_str().to_string(),
            branch,
            worktree_id: None,
        })
    }
}
