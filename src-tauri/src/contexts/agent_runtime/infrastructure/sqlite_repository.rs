use crate::contexts::agent_runtime::application::{
    AgentAvailabilityGateway, AgentRegistryRepository, AgentRuntimeApplicationError,
    AgentWorkflowRepository, ApiAgentGateway, ApiProviderConfig, RegisterApiAgentInput,
    StoredOnePieceProviderConfig, StoredOnePieceProviderProfile, UpdateApiAgentInput,
    INTERFACE_FORMAT_ANTHROPIC, INTERFACE_FORMAT_OPENAI_COMPATIBLE,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentDefinitionInput, AgentLifecycle, AgentOrigin,
    AgentWorkflow, AvailabilityAssessment, InteractionMode, LaunchMetadata,
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
                       launch_url, executable_name, managed_sdk_dependency_id, model_id,
                       interface_format, base_url, agent_origin
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

impl ApiAgentGateway for SqliteAgentRuntimeRepository {
    fn register(
        &self,
        agent_id: &str,
        input: &RegisterApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        connection
            .execute(
                r#"
                INSERT INTO agents (id, display_name, provider, launch_kind, model_id, interface_format, base_url, agent_origin)
                VALUES (?1, ?2, ?3, 'api', ?4, ?5, ?6, 'user')
                "#,
                params![
                    agent_id,
                    input.display_name,
                    input.provider,
                    input.model_id,
                    input.interface_format,
                    input.base_url,
                ],
            )
            .map_err(registry_error)?;
        connection
            .execute(
                "INSERT OR IGNORE INTO agent_modes (agent_id, mode) VALUES (?1, 'api')",
                params![agent_id],
            )
            .map_err(registry_error)?;
        connection
            .execute(
                "INSERT OR IGNORE INTO agent_capability_tags (agent_id, tag) VALUES (?1, 'api')",
                params![agent_id],
            )
            .map_err(registry_error)?;
        self.find_in(&connection, agent_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound(agent_id.to_string()))
    }

    fn provider_config(
        &self,
        agent_id: &str,
    ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
        self.connection()?
            .query_row(
                "SELECT model_id, interface_format, base_url, auto_approve_tools, (SELECT source_provider_id FROM onepiece_provider_profiles WHERE agent_id = agents.id AND active = 1 LIMIT 1) FROM agents WHERE id = ?1",
                [agent_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, bool>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(registry_error)
            .map(|row| {
                row.and_then(|(model_id, interface_format, base_url, auto_approve_tools, source_provider_id)| {
                    Some(ApiProviderConfig {
                        source_provider_id,
                        model_id: model_id?,
                        interface_format: interface_format
                            .unwrap_or_else(|| INTERFACE_FORMAT_ANTHROPIC.to_string()),
                        base_url,
                        auto_approve_tools,
                    })
                })
            })
    }

    fn update(
        &self,
        agent_id: &str,
        input: &UpdateApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        let changed = connection
            .execute(
                r#"
                UPDATE agents
                SET display_name = ?2, model_id = ?3, base_url = ?4
                WHERE id = ?1 AND launch_kind = 'api'
                "#,
                params![agent_id, input.display_name, input.model_id, input.base_url],
            )
            .map_err(registry_error)?;
        if changed == 0 {
            return Err(AgentRuntimeApplicationError::AgentNotFound(
                agent_id.to_string(),
            ));
        }
        self.find_in(&connection, agent_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound(agent_id.to_string()))
    }

    fn delete(&self, agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(registry_error)?;
        let existing_origin: Option<String> = transaction
            .query_row(
                "SELECT agent_origin FROM agents WHERE id = ?1 AND launch_kind = 'api'",
                [agent_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(registry_error)?;
        let Some(existing_origin) = existing_origin else {
            return Err(AgentRuntimeApplicationError::AgentNotFound(
                agent_id.to_string(),
            ));
        };
        if existing_origin == "builtin" {
            return Err(AgentRuntimeApplicationError::Validation(
                "Built-in agents cannot be deleted; reset their provider configuration instead."
                    .to_string(),
            ));
        }
        // Every table is checked before anything is deleted, and the whole operation rolls
        // back (transaction dropped without a commit) if any of them still reference this
        // agent — a delete is never partially applied (design.md Decision 2).
        let mut blocking = Vec::new();
        for (label, sql) in [
            (
                "sessions",
                "SELECT COUNT(*) FROM sessions WHERE agent_id = ?1",
            ),
            (
                "memories",
                "SELECT COUNT(*) FROM agent_memories WHERE agent_id = ?1",
            ),
            (
                "model invocations",
                "SELECT COUNT(*) FROM model_invocations WHERE agent_id = ?1",
            ),
            (
                "Loop definitions as worker",
                "SELECT COUNT(*) FROM loop_definitions WHERE worker_agent_id = ?1",
            ),
            (
                "Loop definitions as verifier",
                "SELECT COUNT(*) FROM loop_definitions WHERE verifier_agent_id = ?1",
            ),
        ] {
            let count: i64 = transaction
                .query_row(sql, [agent_id], |row| row.get(0))
                .map_err(registry_error)?;
            if count > 0 {
                blocking.push(format!("{count} {label}"));
            }
        }
        if !blocking.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(format!(
                "Cannot delete this agent: it is still referenced by {}.",
                blocking.join(", ")
            )));
        }
        transaction
            .execute(
                "DELETE FROM skill_api_agent_bindings WHERE agent_id = ?1",
                [agent_id],
            )
            .map_err(registry_error)?;
        transaction
            .execute(
                "DELETE FROM skill_agent_bindings WHERE agent_id = ?1",
                [agent_id],
            )
            .map_err(registry_error)?;
        transaction
            .execute(
                "DELETE FROM skill_agent_mount_paths WHERE agent_id = ?1",
                [agent_id],
            )
            .map_err(registry_error)?;
        transaction
            .execute("DELETE FROM agents WHERE id = ?1", [agent_id])
            .map_err(registry_error)?;
        transaction.commit().map_err(registry_error)
    }

    fn onepiece_provider_config(
        &self,
    ) -> Result<StoredOnePieceProviderConfig, AgentRuntimeApplicationError> {
        self.connection()?
            .query_row(
                "SELECT provider, model_id, interface_format, base_url, auto_approve_tools FROM agents WHERE id = 'onepiece' AND launch_kind = 'api' AND agent_origin = 'builtin'",
                [],
                |row| {
                    Ok(StoredOnePieceProviderConfig {
                        provider: row.get(0)?,
                        model_id: row.get(1)?,
                        interface_format: row.get(2)?,
                        base_url: row.get(3)?,
                        auto_approve_tools: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(registry_error)?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound("onepiece".to_string()))
    }

    fn save_onepiece_provider_config(
        &self,
        input: &StoredOnePieceProviderConfig,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "UPDATE agents SET provider = ?1, model_id = ?2, interface_format = ?3, base_url = ?4 WHERE id = 'onepiece' AND launch_kind = 'api' AND agent_origin = 'builtin'",
                params![input.provider, input.model_id, input.interface_format, input.base_url],
            )
            .map_err(registry_error)?;
        if changed == 0 {
            return Err(AgentRuntimeApplicationError::AgentNotFound(
                "onepiece".to_string(),
            ));
        }
        connection
            .execute(
                "UPDATE onepiece_provider_profiles SET active = 0 WHERE agent_id = 'onepiece'",
                [],
            )
            .map_err(registry_error)?;
        connection
            .execute(
                "INSERT INTO onepiece_provider_profiles (id, agent_id, name, provider, model_id, interface_format, base_url, active) VALUES ('legacy-default', 'onepiece', ?1, ?1, ?2, ?3, ?4, 1) ON CONFLICT(id) DO UPDATE SET name = excluded.name, provider = excluded.provider, model_id = excluded.model_id, interface_format = excluded.interface_format, base_url = excluded.base_url, active = 1, updated_at = strftime('%s', 'now')",
                params![input.provider, input.model_id, input.interface_format, input.base_url],
            )
            .map_err(registry_error)?;
        self.find_in(&connection, "onepiece")?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound("onepiece".to_string()))
    }

    fn reset_onepiece_provider_config(
        &self,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "UPDATE agents SET provider = 'VaneHub', model_id = NULL, interface_format = NULL, base_url = NULL, auto_approve_tools = 0 WHERE id = 'onepiece' AND launch_kind = 'api' AND agent_origin = 'builtin'",
                [],
            )
            .map_err(registry_error)?;
        if changed == 0 {
            return Err(AgentRuntimeApplicationError::AgentNotFound(
                "onepiece".to_string(),
            ));
        }
        connection
            .execute(
                "DELETE FROM onepiece_provider_profiles WHERE agent_id = 'onepiece'",
                [],
            )
            .map_err(registry_error)?;
        self.find_in(&connection, "onepiece")?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound("onepiece".to_string()))
    }

    fn list_onepiece_provider_profiles(
        &self,
    ) -> Result<Vec<StoredOnePieceProviderProfile>, AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT id, name, source_preset_id, source_provider_id, source_endpoint_type, source_preset_version, provider, model_id, interface_format, base_url, active FROM onepiece_provider_profiles WHERE agent_id = 'onepiece' ORDER BY active DESC, created_at, id")
            .map_err(registry_error)?;
        let profiles = statement
            .query_map([], |row| {
                Ok(StoredOnePieceProviderProfile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    source_preset_id: row.get(2)?,
                    source_provider_id: row.get(3)?,
                    source_endpoint_type: row.get(4)?,
                    source_preset_version: row.get(5)?,
                    provider: row.get(6)?,
                    model_id: row.get(7)?,
                    interface_format: row.get(8)?,
                    base_url: row.get(9)?,
                    active: row.get(10)?,
                })
            })
            .map_err(registry_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(registry_error)?;
        Ok(profiles)
    }

    fn save_onepiece_provider_profile(
        &self,
        profile: &StoredOnePieceProviderProfile,
    ) -> Result<StoredOnePieceProviderProfile, AgentRuntimeApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(registry_error)?;
        let valid_agent: i64 = transaction
            .query_row("SELECT COUNT(*) FROM agents WHERE id = 'onepiece' AND launch_kind = 'api' AND agent_origin = 'builtin'", [], |row| row.get(0))
            .map_err(registry_error)?;
        if valid_agent == 0 {
            return Err(AgentRuntimeApplicationError::AgentNotFound(
                "onepiece".to_string(),
            ));
        }
        if profile.active {
            transaction
                .execute(
                    "UPDATE onepiece_provider_profiles SET active = 0 WHERE agent_id = 'onepiece'",
                    [],
                )
                .map_err(registry_error)?;
        }
        transaction
            .execute(
                "INSERT INTO onepiece_provider_profiles (id, agent_id, name, source_preset_id, source_provider_id, source_endpoint_type, source_preset_version, provider, model_id, interface_format, base_url, active) VALUES (?1, 'onepiece', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) ON CONFLICT(id) DO UPDATE SET name = excluded.name, source_preset_id = excluded.source_preset_id, source_provider_id = excluded.source_provider_id, source_endpoint_type = excluded.source_endpoint_type, source_preset_version = excluded.source_preset_version, provider = excluded.provider, model_id = excluded.model_id, interface_format = excluded.interface_format, base_url = excluded.base_url, updated_at = strftime('%s', 'now') WHERE onepiece_provider_profiles.agent_id = 'onepiece'",
                params![profile.id, profile.name, profile.source_preset_id, profile.source_provider_id, profile.source_endpoint_type, profile.source_preset_version, profile.provider, profile.model_id, profile.interface_format, profile.base_url, profile.active],
            )
            .map_err(registry_error)?;
        if profile.active {
            transaction
                .execute(
                    "UPDATE onepiece_provider_profiles SET active = 1 WHERE agent_id = 'onepiece' AND id = ?1",
                    [&profile.id],
                )
                .map_err(registry_error)?;
            transaction
                .execute("UPDATE agents SET provider = ?1, model_id = ?2, interface_format = ?3, base_url = ?4 WHERE id = 'onepiece'", params![profile.provider, profile.model_id, profile.interface_format, profile.base_url])
                .map_err(registry_error)?;
        }
        transaction.commit().map_err(registry_error)?;
        Ok(profile.clone())
    }

    fn activate_onepiece_provider_profile(
        &self,
        profile_id: &str,
    ) -> Result<StoredOnePieceProviderProfile, AgentRuntimeApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(registry_error)?;
        let mut profile = transaction
            .query_row(
                "SELECT id, name, source_preset_id, source_provider_id, source_endpoint_type, source_preset_version, provider, model_id, interface_format, base_url, active FROM onepiece_provider_profiles WHERE agent_id = 'onepiece' AND id = ?1",
                [profile_id],
                |row| Ok(StoredOnePieceProviderProfile { id: row.get(0)?, name: row.get(1)?, source_preset_id: row.get(2)?, source_provider_id: row.get(3)?, source_endpoint_type: row.get(4)?, source_preset_version: row.get(5)?, provider: row.get(6)?, model_id: row.get(7)?, interface_format: row.get(8)?, base_url: row.get(9)?, active: row.get(10)? }),
            )
            .optional()
            .map_err(registry_error)?
            .ok_or_else(|| AgentRuntimeApplicationError::Validation("OnePiece provider profile was not found.".to_string()))?;
        transaction
            .execute(
                "UPDATE onepiece_provider_profiles SET active = 0 WHERE agent_id = 'onepiece'",
                [],
            )
            .map_err(registry_error)?;
        transaction
            .execute(
                "UPDATE onepiece_provider_profiles SET active = 1 WHERE agent_id = 'onepiece' AND id = ?1",
                [profile_id],
            )
            .map_err(registry_error)?;
        transaction
            .execute("UPDATE agents SET provider = ?1, model_id = ?2, interface_format = ?3, base_url = ?4 WHERE id = 'onepiece' AND launch_kind = 'api' AND agent_origin = 'builtin'", params![profile.provider, profile.model_id, profile.interface_format, profile.base_url])
            .map_err(registry_error)?;
        transaction.commit().map_err(registry_error)?;
        profile.active = true;
        Ok(profile)
    }

    fn delete_onepiece_provider_profile(
        &self,
        profile_id: &str,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(registry_error)?;
        let active = transaction
            .query_row("SELECT active FROM onepiece_provider_profiles WHERE agent_id = 'onepiece' AND id = ?1", [profile_id], |row| row.get::<_, bool>(0))
            .optional()
            .map_err(registry_error)?
            .ok_or_else(|| AgentRuntimeApplicationError::Validation("OnePiece provider profile was not found.".to_string()))?;
        transaction
            .execute(
                "DELETE FROM onepiece_provider_profiles WHERE agent_id = 'onepiece' AND id = ?1",
                [profile_id],
            )
            .map_err(registry_error)?;
        if active {
            transaction
                .execute("UPDATE agents SET provider = 'VaneHub', model_id = NULL, interface_format = NULL, base_url = NULL WHERE id = 'onepiece'", [])
                .map_err(registry_error)?;
        }
        transaction.commit().map_err(registry_error)?;
        Ok(active)
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
    model_id: Option<String>,
    interface_format: Option<String>,
    base_url: Option<String>,
    agent_origin: String,
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
            model_id: row.get(8)?,
            interface_format: row.get(9)?,
            base_url: row.get(10)?,
            agent_origin: row.get(11)?,
        })
    }

    fn into_domain(
        self,
        connection: &Connection,
        availability: &dyn AgentAvailabilityGateway,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        let modes = load_modes(connection, &self.id)?;
        let capability_tags = load_tags(connection, &self.id)?;
        let availability = if self.launch_kind == "api" {
            let has_model = self
                .model_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty());
            let has_base_url_if_required = self.interface_format.as_deref()
                != Some(INTERFACE_FORMAT_OPENAI_COMPATIBLE)
                || self
                    .base_url
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty());
            if has_model && has_base_url_if_required {
                AvailabilityAssessment::new(AgentAvailability::Available, None)
            } else if !has_model {
                AvailabilityAssessment::new(
                    AgentAvailability::Unavailable,
                    Some("API agent is missing a configured model.".to_string()),
                )
            } else {
                AvailabilityAssessment::new(
                    AgentAvailability::Unavailable,
                    Some("API agent is missing a configured base URL.".to_string()),
                )
            }
        } else {
            availability.assess(
                self.managed_sdk_dependency_id.as_deref(),
                self.executable_name.as_deref(),
            )?
        };
        let origin = AgentOrigin::parse(&self.agent_origin)?;
        AgentDefinition::new_with_origin(
            AgentDefinitionInput {
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
            },
            origin,
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::AgentAvailability;
    use crate::test_support::TempDirectory;

    struct FakeAvailability;

    impl AgentAvailabilityGateway for FakeAvailability {
        fn assess(
            &self,
            _managed_sdk_dependency_id: Option<&str>,
            _executable_name: Option<&str>,
        ) -> Result<AvailabilityAssessment, AgentRuntimeApplicationError> {
            Ok(AvailabilityAssessment::new(
                AgentAvailability::Available,
                None,
            ))
        }
    }

    fn fixture(label: &str) -> (TempDirectory, SqliteAgentRuntimeRepository) {
        let directory = TempDirectory::new(label);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("test database");
        let repository = SqliteAgentRuntimeRepository::new(database, Arc::new(FakeAvailability));
        (directory, repository)
    }

    fn register_test_agent(repository: &SqliteAgentRuntimeRepository, id: &str) -> AgentDefinition {
        repository
            .register(
                id,
                &RegisterApiAgentInput {
                    display_name: "Test Agent".to_string(),
                    provider: "Test".to_string(),
                    api_key: "unused-by-register".to_string(),
                    model_id: "gpt-test".to_string(),
                    interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
                    base_url: None,
                },
            )
            .expect("register")
    }

    fn seed_session(repository: &SqliteAgentRuntimeRepository, agent_id: &str) {
        repository
            .connection()
            .expect("connection")
            .execute(
                r#"
                INSERT INTO sessions
                    (id, title, agent_id, interaction_mode, lifecycle_state, created_at, updated_at)
                VALUES ('fixture-session', 'Fixture', ?1, 'api', 'idle', '2026-01-01', '2026-01-01')
                "#,
                params![agent_id],
            )
            .expect("seed session");
    }

    fn seed_memory(repository: &SqliteAgentRuntimeRepository, agent_id: &str) {
        repository
            .connection()
            .expect("connection")
            .execute(
                r#"
                INSERT INTO agent_memories (id, agent_id, folder, content, source, created_at, updated_at)
                VALUES ('fixture-memory', ?1, '', 'Uses pnpm.', 'explicit', '2026-01-01', '2026-01-01')
                "#,
                params![agent_id],
            )
            .expect("seed memory");
    }

    fn seed_usage_record(repository: &SqliteAgentRuntimeRepository, agent_id: &str) {
        let connection = repository.connection().expect("connection");
        connection
            .execute(
                r#"
                INSERT INTO sessions
                    (id, title, agent_id, interaction_mode, lifecycle_state, created_at, updated_at)
                VALUES ('usage-session', 'Fixture', ?1, 'api', 'idle', '2026-01-01', '2026-01-01')
                "#,
                params![agent_id],
            )
            .expect("seed usage session");
        connection
            .execute(
                r#"
                INSERT INTO model_invocations
                    (id, session_id, agent_id, interaction_kind, purpose, request_sequence,
                     attempt, status, started_at, completed_at)
                VALUES ('fixture-invocation', 'usage-session', ?1, 'native-api',
                        'assistant-initial', 0, 0, 'succeeded', '2026-01-01', '2026-01-01')
                "#,
                params![agent_id],
            )
            .expect("seed model invocation");
    }

    fn seed_loop_definition(
        repository: &SqliteAgentRuntimeRepository,
        agent_id: &str,
        as_worker: bool,
    ) {
        let (worker, verifier) = if as_worker {
            (agent_id, "other-agent")
        } else {
            ("other-agent", agent_id)
        };
        let connection = repository.connection().expect("connection");
        connection
            .execute(
                "INSERT INTO agents (id, display_name, provider, launch_kind) VALUES ('other-agent', 'Other', 'Test', 'api')",
                [],
            )
            .expect("seed other agent");
        connection
            .execute(
                r#"
                INSERT INTO loop_definitions
                    (id, name, project_path, base_branch, goal, acceptance_criteria, allowed_paths,
                     protected_paths, worker_agent_id, verifier_agent_id, verification_commands,
                     limits, version, created_at, updated_at)
                VALUES ('fixture-loop', 'Fixture', 'C:/project', 'main', 'Goal', '[]', '[]', '[]',
                        ?1, ?2, '[]', '{}', 1, '2026-01-01', '2026-01-01')
                "#,
                params![worker, verifier],
            )
            .expect("seed loop definition");
    }

    #[test]
    fn update_changes_display_name_model_and_base_url_but_not_provider_or_interface_format() {
        let (_directory, repository) = fixture("agent update basic");
        register_test_agent(&repository, "my-agent");

        let updated = repository
            .update(
                "my-agent",
                &UpdateApiAgentInput {
                    display_name: "Renamed Agent".to_string(),
                    model_id: "gpt-updated".to_string(),
                    base_url: Some("https://example.test".to_string()),
                    new_api_key: None,
                },
            )
            .expect("update");

        assert_eq!(updated.display_name(), "Renamed Agent");
        assert_eq!(updated.provider(), "Test");
        let config = repository
            .provider_config("my-agent")
            .expect("provider config")
            .expect("config present");
        assert_eq!(config.model_id, "gpt-updated");
        assert_eq!(config.base_url.as_deref(), Some("https://example.test"));
        assert_eq!(config.interface_format, INTERFACE_FORMAT_ANTHROPIC);
    }

    #[test]
    fn update_on_a_nonexistent_agent_errors() {
        let (_directory, repository) = fixture("agent update missing");

        let result = repository.update(
            "does-not-exist",
            &UpdateApiAgentInput {
                display_name: "X".to_string(),
                model_id: "X".to_string(),
                base_url: None,
                new_api_key: None,
            },
        );

        assert!(matches!(
            result,
            Err(AgentRuntimeApplicationError::AgentNotFound(_))
        ));
    }

    #[test]
    fn onepiece_configuration_replaces_provider_and_interface_in_place() {
        let (_directory, repository) = fixture("onepiece provider replacement");

        let updated = repository
            .save_onepiece_provider_config(&StoredOnePieceProviderConfig {
                provider: "OpenAI Proxy".to_string(),
                model_id: Some("gpt-test".to_string()),
                interface_format: Some(INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string()),
                base_url: Some("https://gateway.example.test/v1".to_string()),
                auto_approve_tools: false,
            })
            .expect("configure OnePiece");

        assert_eq!(updated.id().as_str(), "onepiece");
        assert_eq!(updated.provider(), "OpenAI Proxy");
        assert_eq!(updated.origin(), AgentOrigin::Builtin);
        assert_eq!(
            repository
                .onepiece_provider_config()
                .expect("OnePiece config"),
            StoredOnePieceProviderConfig {
                provider: "OpenAI Proxy".to_string(),
                model_id: Some("gpt-test".to_string()),
                interface_format: Some(INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string()),
                base_url: Some("https://gateway.example.test/v1".to_string()),
                auto_approve_tools: false,
            }
        );
    }

    #[test]
    fn onepiece_reset_preserves_identity_and_all_reference_classes() {
        let (_directory, repository) = fixture("onepiece reset references");
        repository
            .save_onepiece_provider_config(&StoredOnePieceProviderConfig {
                provider: "Anthropic".to_string(),
                model_id: Some("claude-test".to_string()),
                interface_format: Some(INTERFACE_FORMAT_ANTHROPIC.to_string()),
                base_url: None,
                auto_approve_tools: true,
            })
            .expect("configure OnePiece");
        seed_session(&repository, "onepiece");
        seed_memory(&repository, "onepiece");
        seed_usage_record(&repository, "onepiece");
        seed_loop_definition(&repository, "onepiece", true);
        repository
            .connection()
            .expect("connection")
            .execute(
                r#"
                INSERT INTO skill_api_agent_bindings
                    (skill_id, scope, workspace_path, agent_id, created_at, updated_at)
                VALUES ('onepiece-skill', 'global', '', 'onepiece', '2026-01-01', '2026-01-01')
                "#,
                [],
            )
            .expect("seed skill binding");

        let reset = repository
            .reset_onepiece_provider_config()
            .expect("reset OnePiece");

        assert_eq!(reset.id().as_str(), "onepiece");
        assert_eq!(reset.provider(), "VaneHub");
        assert_eq!(reset.origin(), AgentOrigin::Builtin);
        assert_eq!(
            repository
                .onepiece_provider_config()
                .expect("OnePiece config"),
            StoredOnePieceProviderConfig {
                provider: "VaneHub".to_string(),
                model_id: None,
                interface_format: None,
                base_url: None,
                auto_approve_tools: false,
            }
        );
        let connection = repository.connection().expect("connection");
        for (table, predicate, expected) in [
            ("sessions", "agent_id = 'onepiece'", 2),
            ("agent_memories", "agent_id = 'onepiece'", 1),
            ("model_invocations", "agent_id = 'onepiece'", 1),
            ("loop_definitions", "worker_agent_id = 'onepiece'", 1),
            ("skill_api_agent_bindings", "agent_id = 'onepiece'", 1),
        ] {
            let count: i64 = connection
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE {predicate}"),
                    [],
                    |row| row.get(0),
                )
                .expect("count preserved references");
            assert_eq!(count, expected, "{table} reference was not preserved");
        }
    }

    #[test]
    fn onepiece_builtin_delete_and_non_builtin_configuration_are_rejected() {
        let (_directory, repository) = fixture("onepiece lifecycle guards");

        assert!(matches!(
            repository.delete("onepiece"),
            Err(AgentRuntimeApplicationError::Validation(_))
        ));
        repository
            .connection()
            .expect("connection")
            .execute(
                "UPDATE agents SET agent_origin = 'user' WHERE id = 'onepiece'",
                [],
            )
            .expect("change origin for guard test");
        let result = repository.save_onepiece_provider_config(&StoredOnePieceProviderConfig {
            provider: "Anthropic".to_string(),
            model_id: Some("claude-test".to_string()),
            interface_format: Some(INTERFACE_FORMAT_ANTHROPIC.to_string()),
            base_url: None,
            auto_approve_tools: false,
        });
        assert!(matches!(
            result,
            Err(AgentRuntimeApplicationError::AgentNotFound(id)) if id == "onepiece"
        ));
    }

    #[test]
    fn delete_removes_an_unreferenced_agent_and_its_mode_and_tag_rows() {
        let (_directory, repository) = fixture("agent delete unreferenced");
        register_test_agent(&repository, "my-agent");

        repository.delete("my-agent").expect("delete");

        assert!(repository.find("my-agent").expect("find").is_none());
        let connection = repository.connection().expect("connection");
        let modes: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM agent_modes WHERE agent_id = 'my-agent'",
                [],
                |row| row.get(0),
            )
            .expect("count modes");
        assert_eq!(modes, 0);
    }

    #[test]
    fn delete_on_a_nonexistent_agent_errors() {
        let (_directory, repository) = fixture("agent delete missing");

        let result = repository.delete("does-not-exist");

        assert!(matches!(
            result,
            Err(AgentRuntimeApplicationError::AgentNotFound(_))
        ));
    }

    #[test]
    fn delete_is_rejected_and_unapplied_when_referenced_by_a_session() {
        let (_directory, repository) = fixture("agent delete session ref");
        register_test_agent(&repository, "my-agent");
        seed_session(&repository, "my-agent");

        let result = repository.delete("my-agent");

        assert!(matches!(
            result,
            Err(AgentRuntimeApplicationError::Validation(_))
        ));
        assert!(repository.find("my-agent").expect("find").is_some());
    }

    #[test]
    fn delete_is_rejected_when_referenced_by_a_memory() {
        let (_directory, repository) = fixture("agent delete memory ref");
        register_test_agent(&repository, "my-agent");
        seed_memory(&repository, "my-agent");

        let result = repository.delete("my-agent");

        assert!(matches!(
            result,
            Err(AgentRuntimeApplicationError::Validation(_))
        ));
        assert!(repository.find("my-agent").expect("find").is_some());
    }

    #[test]
    fn delete_is_rejected_when_referenced_by_a_usage_record() {
        let (_directory, repository) = fixture("agent delete usage ref");
        register_test_agent(&repository, "my-agent");
        seed_usage_record(&repository, "my-agent");

        let result = repository.delete("my-agent");

        assert!(matches!(
            result,
            Err(AgentRuntimeApplicationError::Validation(_))
        ));
        assert!(repository.find("my-agent").expect("find").is_some());
    }

    #[test]
    fn delete_is_rejected_when_assigned_as_a_loop_worker() {
        let (_directory, repository) = fixture("agent delete loop worker ref");
        register_test_agent(&repository, "my-agent");
        seed_loop_definition(&repository, "my-agent", true);

        let result = repository.delete("my-agent");

        assert!(matches!(
            result,
            Err(AgentRuntimeApplicationError::Validation(_))
        ));
        assert!(repository.find("my-agent").expect("find").is_some());
    }

    #[test]
    fn delete_is_rejected_when_assigned_as_a_loop_verifier() {
        let (_directory, repository) = fixture("agent delete loop verifier ref");
        register_test_agent(&repository, "my-agent");
        seed_loop_definition(&repository, "my-agent", false);

        let result = repository.delete("my-agent");

        assert!(matches!(
            result,
            Err(AgentRuntimeApplicationError::Validation(_))
        ));
        assert!(repository.find("my-agent").expect("find").is_some());
    }

    #[test]
    fn delete_removes_skill_api_agent_bindings() {
        let (_directory, repository) = fixture("agent delete skill bindings");
        register_test_agent(&repository, "my-agent");
        let connection = repository.connection().expect("connection");
        connection
            .execute(
                r#"
                INSERT INTO skill_api_agent_bindings
                    (skill_id, scope, workspace_path, agent_id, created_at, updated_at)
                VALUES ('some-skill', 'global', '', 'my-agent', '2026-01-01', '2026-01-01')
                "#,
                [],
            )
            .expect("seed skill binding");
        connection
            .execute(
                r#"
                INSERT INTO skill_agent_bindings
                    (skill_id, scope, workspace_path, agent_id, mounted_path, status, created_at, updated_at)
                VALUES ('some-skill', 'global', '', 'my-agent', '.skills/some-skill', 'pending', '2026-01-01', '2026-01-01')
                "#,
                [],
            )
            .expect("seed legacy CLI binding");
        connection
            .execute(
                r#"
                INSERT INTO skill_agent_mount_paths
                    (agent_id, mount_path, created_at, updated_at)
                VALUES ('my-agent', '.skills', '2026-01-01', '2026-01-01')
                "#,
                [],
            )
            .expect("seed legacy mount path");
        drop(connection);

        repository.delete("my-agent").expect("delete");

        let connection = repository.connection().expect("connection");
        let bindings: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM skill_api_agent_bindings WHERE agent_id = 'my-agent'",
                [],
                |row| row.get(0),
            )
            .expect("count bindings");
        assert_eq!(bindings, 0);
        let legacy_bindings: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM skill_agent_bindings WHERE agent_id = 'my-agent'",
                [],
                |row| row.get(0),
            )
            .expect("count legacy bindings");
        let mount_paths: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM skill_agent_mount_paths WHERE agent_id = 'my-agent'",
                [],
                |row| row.get(0),
            )
            .expect("count mount paths");
        assert_eq!(legacy_bindings, 0);
        assert_eq!(mount_paths, 0);
    }
}
