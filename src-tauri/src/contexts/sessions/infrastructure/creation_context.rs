use crate::contexts::agent_runtime::application::AgentRegistryRepository;
use crate::contexts::agent_runtime::domain::InteractionMode;
use crate::contexts::sessions::application::{
    CreatedSessionWorktree, NewRemoteWorkspace, SessionAgentEligibilityPort,
    SessionCreationContextPort, SessionProject, SessionRemoteWorkspace, SessionSshProfile,
    SessionsApplicationError,
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

#[derive(Clone)]
pub(crate) struct SessionAgentEligibilityAdapter {
    registry: std::sync::Arc<dyn AgentRegistryRepository>,
}

impl SessionAgentEligibilityAdapter {
    pub(crate) fn new(registry: std::sync::Arc<dyn AgentRegistryRepository>) -> Self {
        Self { registry }
    }
}

impl SessionAgentEligibilityPort for SessionAgentEligibilityAdapter {
    fn ensure_agent_supports(
        &self,
        agent_id: &str,
        interaction_mode: &str,
    ) -> Result<(), SessionsApplicationError> {
        let mode = InteractionMode::parse(interaction_mode).map_err(|_| {
            SessionsApplicationError::UnsupportedInteractionMode(interaction_mode.to_string())
        })?;
        let agent = self
            .registry
            .find(agent_id)
            .map_err(|error| SessionsApplicationError::Repository(error.to_string()))?;
        let Some(agent) = agent else {
            return Err(SessionsApplicationError::AgentNotFound(
                agent_id.to_string(),
            ));
        };
        agent.ensure_session_selectable(mode).map_err(|error| match error {
            crate::contexts::agent_runtime::domain::AgentRuntimeDomainError::InteractionModeNotSupported { mode, .. } => {
                SessionsApplicationError::UnsupportedInteractionMode(mode)
            }
            other => SessionsApplicationError::Validation(other.to_string()),
        })
    }
}

impl SessionCreationContextPort for SessionCreationContextAdapter {
    fn remote_workspace_uri(&self, workspace: &NewRemoteWorkspace) -> Option<String> {
        remote_workspace(workspace)
            .ok()
            .map(|workspace| workspace.uri().to_string())
    }

    fn find_ssh_profile(
        &self,
        connection_id: &str,
    ) -> Result<Option<SessionSshProfile>, SessionsApplicationError> {
        self.connection()?
            .query_row(
                "SELECT id, revision, host, port, user FROM ssh_connections WHERE id = ?1",
                [connection_id],
                |row| {
                    Ok(SessionSshProfile {
                        connection_id: row.get(0)?,
                        revision: row.get(1)?,
                        host: row.get(2)?,
                        port: row.get(3)?,
                        user: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(repository_error)
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
                worktree_id: worktree.worktree_id,
            })
            .map_err(workspace_error)
    }

    fn bind_worktree_session(
        &self,
        worktree_id: &str,
        session_id: &str,
    ) -> Result<(), SessionsApplicationError> {
        self.workspaces
            .confirm_worktree_created(worktree_id, session_id)
            .map(|_| ())
            .map_err(workspace_error)
    }

    fn ensure_workspace_admits_binding(&self, path: &str) -> Result<(), SessionsApplicationError> {
        if self
            .workspaces
            .is_path_gated(path)
            .map_err(workspace_error)?
        {
            return Err(SessionsApplicationError::Validation(
                crate::contexts::sessions::application::deletion_error_code::GATE_HELD.to_string(),
            ));
        }
        Ok(())
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
        // The code, unwrapped. Sessions has no richer conflict of its own to map onto, and losing
        // it would turn a matchable refusal into an unexplained validation failure.
        WorkspaceError::Conflict(code) => SessionsApplicationError::Validation(code.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{
        AgentRegistryRepository, AgentRuntimeApplicationError,
    };
    use crate::contexts::agent_runtime::domain::{
        AgentAvailability, AgentDefinition, AgentDefinitionInput, AgentOrigin,
        AvailabilityAssessment, LaunchMetadata,
    };
    use std::sync::Arc;

    struct Registry(Vec<AgentDefinition>);

    impl AgentRegistryRepository for Registry {
        fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
            Ok(self.0.clone())
        }

        fn find(
            &self,
            agent_id: &str,
        ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
            Ok(self
                .0
                .iter()
                .find(|agent| agent.id().as_str() == agent_id)
                .cloned())
        }
    }

    fn onepiece(availability: AgentAvailability) -> AgentDefinition {
        AgentDefinition::new_with_origin(
            AgentDefinitionInput {
                id: "onepiece".to_string(),
                display_name: "OnePiece".to_string(),
                provider: "VaneHub".to_string(),
                managed_sdk_dependency_id: None,
                launch: LaunchMetadata::new("api".to_string(), None, None, None).expect("launch"),
                supported_interaction_modes: vec![InteractionMode::Api],
                availability: AvailabilityAssessment::new(availability, None),
                capability_tags: vec!["api".to_string()],
            },
            AgentOrigin::Builtin,
        )
        .expect("OnePiece")
    }

    fn cli_with_missing_sdk() -> AgentDefinition {
        AgentDefinition::new_with_origin(
            AgentDefinitionInput {
                id: "claude-code".to_string(),
                display_name: "Claude Code".to_string(),
                provider: "Anthropic".to_string(),
                managed_sdk_dependency_id: Some("claude-sdk".to_string()),
                launch: LaunchMetadata::new(
                    "cli".to_string(),
                    Some("claude".to_string()),
                    None,
                    Some("claude".to_string()),
                )
                .expect("launch"),
                supported_interaction_modes: vec![
                    InteractionMode::Cli,
                    InteractionMode::NativeDesktop,
                ],
                availability: AvailabilityAssessment::new(
                    AgentAvailability::Unavailable,
                    Some("Managed SDK dependency 'claude-sdk' is not installed.".to_string()),
                ),
                capability_tags: vec!["cli".to_string()],
            },
            AgentOrigin::Builtin,
        )
        .expect("Claude Code")
    }

    #[test]
    fn eligibility_accepts_ready_onepiece_and_rejects_non_ready_mode_and_unknown() {
        let ready = SessionAgentEligibilityAdapter::new(Arc::new(Registry(vec![onepiece(
            AgentAvailability::Available,
        )])));
        ready
            .ensure_agent_supports("onepiece", "api")
            .expect("ready OnePiece");
        assert!(matches!(
            ready.ensure_agent_supports("onepiece", "cli"),
            Err(SessionsApplicationError::UnsupportedInteractionMode(_))
        ));
        assert!(matches!(
            ready.ensure_agent_supports("missing", "api"),
            Err(SessionsApplicationError::AgentNotFound(_))
        ));

        let needs_auth = SessionAgentEligibilityAdapter::new(Arc::new(Registry(vec![onepiece(
            AgentAvailability::NeedsAuthentication,
        )])));
        assert!(matches!(
            needs_auth.ensure_agent_supports("onepiece", "api"),
            Err(SessionsApplicationError::Validation(_))
        ));
    }

    #[test]
    fn eligibility_allows_cli_mode_when_only_the_optional_sdk_is_missing() {
        let eligibility =
            SessionAgentEligibilityAdapter::new(Arc::new(Registry(vec![cli_with_missing_sdk()])));

        eligibility
            .ensure_agent_supports("claude-code", "cli")
            .expect("CLI session remains selectable");
        assert!(matches!(
            eligibility.ensure_agent_supports("claude-code", "native-desktop"),
            Err(SessionsApplicationError::Validation(_))
        ));
    }
}
