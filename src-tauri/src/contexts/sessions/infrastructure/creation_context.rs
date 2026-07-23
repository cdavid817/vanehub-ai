use crate::contexts::sessions::application::{
    CreatedSessionWorktree, NewRemoteWorkspace, SessionCreationContextPort, SessionProject,
    SessionRemoteWorkspace, SessionsApplicationError,
};
use crate::contexts::workspaces::api::{
    ensure_git_worktree_available, ensure_worktree_compatible, RemoteWorkspace, WorkspaceApi,
    WorkspaceError,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::OptionalExtension;

#[derive(Clone)]
pub(crate) struct SessionCreationContextAdapter {
    database: NativeDatabase,
    workspaces: WorkspaceApi,
}

impl SessionCreationContextAdapter {
    pub(crate) fn new(database: NativeDatabase, workspaces: WorkspaceApi) -> Self {
        Self {
            database,
            workspaces,
        }
    }
}

impl SessionCreationContextPort for SessionCreationContextAdapter {
    fn remote_workspace_uri(&self, workspace: &NewRemoteWorkspace) -> Option<String> {
        remote_workspace(workspace)
            .ok()
            .map(|workspace| workspace.uri().to_string())
    }

    fn ensure_agent_supports(
        &self,
        agent_id: &str,
        interaction_mode: &str,
    ) -> Result<(), SessionsApplicationError> {
        let connection = self.connection()?;
        let exists = connection
            .query_row("SELECT 1 FROM agents WHERE id = ?1", [agent_id], |_| Ok(()))
            .optional()
            .map_err(repository_error)?
            .is_some();
        if !exists {
            return Err(SessionsApplicationError::AgentNotFound(
                agent_id.to_string(),
            ));
        }

        let mut statement = connection
            .prepare("SELECT mode FROM agent_modes WHERE agent_id = ?1 ORDER BY mode")
            .map_err(repository_error)?;
        let modes = statement
            .query_map([agent_id], |row| row.get::<_, String>(0))
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        for mode in &modes {
            if !matches!(mode.as_str(), "browser" | "native-desktop" | "cli") {
                return Err(SessionsApplicationError::UnsupportedInteractionMode(
                    mode.clone(),
                ));
            }
        }
        if !modes.iter().any(|mode| mode == interaction_mode) {
            return Err(SessionsApplicationError::UnsupportedInteractionMode(
                interaction_mode.to_string(),
            ));
        }
        Ok(())
    }

    fn ensure_worktree_compatible(
        &self,
        remote_workspace_selected: bool,
        worktree_enabled: bool,
    ) -> Result<(), SessionsApplicationError> {
        ensure_worktree_compatible(remote_workspace_selected, worktree_enabled)
            .map_err(|error| SessionsApplicationError::Validation(error.to_string()))
    }

    fn prepare_project(&self, path: &str) -> Result<SessionProject, SessionsApplicationError> {
        let inspection = self
            .workspaces
            .inspect_project(path)
            .map_err(workspace_error)?;
        self.workspaces
            .remember_project(&inspection)
            .map_err(workspace_error)?;
        Ok(SessionProject {
            path: inspection.path().to_string(),
            is_git: inspection.is_git(),
        })
    }

    fn normalize_remote_workspace(
        &self,
        workspace: &NewRemoteWorkspace,
    ) -> Result<SessionRemoteWorkspace, SessionsApplicationError> {
        remote_workspace(workspace).map(|workspace| SessionRemoteWorkspace {
            host: workspace.host().to_string(),
            port: Some(workspace.port()),
            user: workspace.user().map(str::to_string),
            path: workspace.path().to_string(),
            display_name: workspace.display_name().to_string(),
            uri: workspace.uri().to_string(),
        })
    }

    fn remember_remote_workspace(
        &self,
        workspace: &SessionRemoteWorkspace,
    ) -> Result<(), SessionsApplicationError> {
        let workspace = RemoteWorkspace::new(
            &workspace.host,
            workspace.port,
            workspace.user.as_deref(),
            &workspace.path,
            Some(&workspace.display_name),
        )
        .map_err(|error| SessionsApplicationError::Validation(error.to_string()))?;
        self.workspaces
            .remember_remote_workspace(&workspace)
            .map_err(workspace_error)
    }

    fn ensure_git_worktree_available(
        &self,
        project: &SessionProject,
    ) -> Result<(), SessionsApplicationError> {
        ensure_git_worktree_available(project.is_git)
            .map_err(|error| SessionsApplicationError::Validation(error.to_string()))
    }

    fn create_worktree(
        &self,
        project_path: &str,
        name: &str,
    ) -> Result<CreatedSessionWorktree, SessionsApplicationError> {
        self.workspaces
            .create_worktree(project_path, name)
            .map(|worktree| CreatedSessionWorktree {
                path: worktree.path,
                name: worktree.name,
                branch: worktree.branch,
            })
            .map_err(workspace_error)
    }
}

impl SessionCreationContextAdapter {
    fn connection(&self) -> Result<PooledSqlite, SessionsApplicationError> {
        self.database
            .connection()
            .map_err(|error| SessionsApplicationError::Repository(error.to_string()))
    }
}

fn remote_workspace(
    workspace: &NewRemoteWorkspace,
) -> Result<RemoteWorkspace, SessionsApplicationError> {
    RemoteWorkspace::new(
        &workspace.host,
        workspace.port,
        workspace.user.as_deref(),
        &workspace.path,
        workspace.display_name.as_deref(),
    )
    .map_err(|error| SessionsApplicationError::Validation(error.to_string()))
}

fn repository_error(error: rusqlite::Error) -> SessionsApplicationError {
    SessionsApplicationError::Repository(error.to_string())
}

fn workspace_error(error: WorkspaceError) -> SessionsApplicationError {
    match error {
        WorkspaceError::Domain(error) => SessionsApplicationError::Validation(error.to_string()),
        WorkspaceError::Validation(message) => SessionsApplicationError::Validation(message),
        WorkspaceError::LaunchFailed(message) => SessionsApplicationError::WorkspaceLaunch(message),
        WorkspaceError::SessionNotFound(session_id) => {
            SessionsApplicationError::SessionNotFound(session_id)
        }
        WorkspaceError::PolicyDenied { session_id, action } => {
            SessionsApplicationError::Validation(format!(
                "Verifier session {session_id} cannot perform workspace action: {action}"
            ))
        }
        WorkspaceError::Repository(message)
        | WorkspaceError::Selection(message)
        | WorkspaceError::Filesystem(message)
        | WorkspaceError::Storage(message) => SessionsApplicationError::Workspace(message),
    }
}
