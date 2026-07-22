use crate::contexts::agent_runtime::application::{
    AgentAvailabilityGateway, AgentRegistryRepository, AgentRuntimeApplicationError,
    AgentWorkflowRepository,
};
use crate::contexts::agent_runtime::domain::{
    AgentDefinition, AgentDefinitionInput, AgentLifecycle, AgentWorkflow, InteractionMode,
    LaunchMetadata,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SqliteAgentRuntimeRepository {
    database: NativeDatabase,
    availability: Arc<dyn AgentAvailabilityGateway>,
}

impl SqliteAgentRuntimeRepository {
    pub(crate) fn new(
        database: NativeDatabase,
        availability: Arc<dyn AgentAvailabilityGateway>,
    ) -> Self {
        Self {
            database,
            availability,
        }
    }

    fn connection(&self) -> Result<PooledSqlite, AgentRuntimeApplicationError> {
        self.database
            .connection()
            .map_err(|error| AgentRuntimeApplicationError::Registry(error.to_string()))
    }

    fn find_in(
        &self,
        connection: &Connection,
        agent_id: &str,
    ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
        let row = connection
            .query_row(
                r#"
                SELECT id, display_name, provider, launch_kind, launch_command,
                       launch_url, executable_name, managed_sdk_dependency_id
                FROM agents
                WHERE id = ?1
                "#,
                [agent_id],
                AgentRow::read,
            )
            .optional()
            .map_err(registry_error)?;
        row.map(|row| row.into_domain(connection, self.availability.as_ref()))
            .transpose()
    }
}

impl AgentRegistryRepository for SqliteAgentRuntimeRepository {
    fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT id FROM agents ORDER BY display_name")
            .map_err(registry_error)?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(registry_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(registry_error)?;
        ids.into_iter()
            .map(|agent_id| {
                self.find_in(&connection, &agent_id)?.ok_or_else(|| {
                    AgentRuntimeApplicationError::AgentNotFound(agent_id.to_string())
                })
            })
            .collect()
    }

    fn find(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
        self.find_in(&*self.connection()?, agent_id)
    }
}

impl AgentWorkflowRepository for SqliteAgentRuntimeRepository {
    fn load(&self) -> Result<AgentWorkflow, AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        let row = connection
            .query_row(
                r#"
                SELECT active_agent_id, active_interaction_mode, lifecycle_state, intent
                FROM workflow_state
                WHERE id = 1
                "#,
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(workflow_error)?
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Workflow(
                    "workflow_state singleton row is missing".to_string(),
                )
            })?;
        AgentWorkflow::rehydrate(
            row.0,
            row.1.as_deref().map(InteractionMode::parse).transpose()?,
            AgentLifecycle::from_storage_lossy(&row.2),
            row.3,
        )
        .map_err(AgentRuntimeApplicationError::from)
    }

    fn save(&self, workflow: &AgentWorkflow) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"
                UPDATE workflow_state
                SET active_agent_id = ?1,
                    active_interaction_mode = ?2,
                    lifecycle_state = ?3,
                    intent = ?4
                WHERE id = 1
                "#,
                params![
                    workflow.active_agent_id().map(|id| id.as_str()),
                    workflow.active_interaction_mode().map(|mode| mode.as_str()),
                    workflow.lifecycle().as_str(),
                    workflow.intent(),
                ],
            )
            .map_err(workflow_error)?;
        if changed == 0 {
            return Err(AgentRuntimeApplicationError::Workflow(
                "workflow_state singleton row is missing".to_string(),
            ));
        }
        Ok(())
    }

    fn load_details(
        &self,
    ) -> Result<(String, BTreeMap<String, String>), AgentRuntimeApplicationError> {
        let (adapter, message) = self
            .connection()?
            .query_row(
                "SELECT adapter, message FROM session_details WHERE id = 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(workflow_error)?
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Workflow(
                    "session_details singleton row is missing".to_string(),
                )
            })?;
        Ok((
            adapter,
            BTreeMap::from([
                ("runtime".to_string(), "tauri".to_string()),
                ("message".to_string(), message),
                (
                    "nativeDesktopSupported".to_string(),
                    native_desktop_supported().to_string(),
                ),
            ]),
        ))
    }

    fn save_details(
        &self,
        adapter: &str,
        message: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                "UPDATE session_details SET adapter = ?1, message = ?2 WHERE id = 1",
                params![adapter, message],
            )
            .map_err(workflow_error)?;
        if changed == 0 {
            return Err(AgentRuntimeApplicationError::Workflow(
                "session_details singleton row is missing".to_string(),
            ));
        }
        Ok(())
    }
}

struct AgentRow {
    id: String,
    display_name: String,
    provider: String,
    launch_kind: String,
    launch_command: Option<String>,
    launch_url: Option<String>,
    executable_name: Option<String>,
    managed_sdk_dependency_id: Option<String>,
}

impl AgentRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            display_name: row.get(1)?,
            provider: row.get(2)?,
            launch_kind: row.get(3)?,
            launch_command: row.get(4)?,
            launch_url: row.get(5)?,
            executable_name: row.get(6)?,
            managed_sdk_dependency_id: row.get(7)?,
        })
    }

    fn into_domain(
        self,
        connection: &Connection,
        availability: &dyn AgentAvailabilityGateway,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        let modes = load_modes(connection, &self.id)?;
        let capability_tags = load_tags(connection, &self.id)?;
        let availability = availability.assess(
            self.managed_sdk_dependency_id.as_deref(),
            self.executable_name.as_deref(),
        )?;
        AgentDefinition::new(AgentDefinitionInput {
            id: self.id,
            display_name: self.display_name,
            provider: self.provider,
            managed_sdk_dependency_id: self.managed_sdk_dependency_id,
            launch: LaunchMetadata::new(
                self.launch_kind,
                self.launch_command,
                self.launch_url,
                self.executable_name,
            )?,
            supported_interaction_modes: modes,
            availability,
            capability_tags,
        })
        .map_err(AgentRuntimeApplicationError::from)
    }
}

fn load_modes(
    connection: &Connection,
    agent_id: &str,
) -> Result<Vec<InteractionMode>, AgentRuntimeApplicationError> {
    let mut statement = connection
        .prepare("SELECT mode FROM agent_modes WHERE agent_id = ?1 ORDER BY mode")
        .map_err(registry_error)?;
    let modes = statement
        .query_map([agent_id], |row| row.get::<_, String>(0))
        .map_err(registry_error)?
        .map(|value| {
            value
                .map_err(registry_error)
                .and_then(|value| InteractionMode::parse(&value).map_err(Into::into))
        })
        .collect();
    modes
}

fn load_tags(
    connection: &Connection,
    agent_id: &str,
) -> Result<Vec<String>, AgentRuntimeApplicationError> {
    let mut statement = connection
        .prepare("SELECT tag FROM agent_capability_tags WHERE agent_id = ?1 ORDER BY tag")
        .map_err(registry_error)?;
    let tags = statement
        .query_map([agent_id], |row| row.get::<_, String>(0))
        .map_err(registry_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(registry_error);
    tags
}

fn native_desktop_supported() -> bool {
    cfg!(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux"
    ))
}

fn registry_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Registry(error.to_string())
}

fn workflow_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Workflow(error.to_string())
}
