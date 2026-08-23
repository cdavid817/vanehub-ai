use crate::contexts::tooling::skills::application::{
    AgentMountConfiguration, BuiltinCleanupStatus, BuiltinReconciliationOutcome,
    BuiltinReconciliationState, ManagedSkillSource, SkillAgentBinding, SkillAgentKind,
    SkillApiBindingRepository, SkillApplicationError, SkillCompatibleAgent, SkillDriftReport,
    SkillReconciliationRepository, SkillRecord, SkillRepository,
};
use crate::contexts::tooling::skills::domain::{
    default_mount_path, SkillAvailability, SkillDelegationAgentRuntime, SkillDriftIssueType,
    SkillId, SkillKey, SkillLayer, SkillLocation, SkillMetadata, SkillMountPath, SkillOrigin,
    SkillScope, SkillSource,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::{begin_write_transaction, NativeDatabase};
use rusqlite::{params, Connection, Row, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) struct SqliteSkillRepository {
    database: NativeDatabase,
}

impl SqliteSkillRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl SkillRepository for SqliteSkillRepository {
    fn list(&self, location: &SkillLocation) -> Result<Vec<SkillRecord>, SkillApplicationError> {
        reconcile_workspace_aliases(&self.database, location)?;
        let connection = self.database.connection().map_err(app_error)?;
        list_records(&connection, location)
    }

    fn get(&self, key: &SkillKey) -> Result<Option<SkillRecord>, SkillApplicationError> {
        reconcile_workspace_aliases(&self.database, &key.location)?;
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT id, scope, workspace_path, source, enabled, skill_dir, skill_md_path,
                       content_hash, metadata_json, created_at, updated_at
                FROM skills
                WHERE id = ?1 AND scope = ?2 AND workspace_path = ?3
                "#,
            )
            .map_err(repository_error)?;
        let mut rows = statement
            .query(params![
                key.id.as_str(),
                key.location.scope.as_str(),
                key.location.storage_workspace_key(),
            ])
            .map_err(repository_error)?;
        let Some(row) = rows.next().map_err(repository_error)? else {
            return Ok(None);
        };
        let row = SkillRow::read(row).map_err(repository_error)?;
        Ok(Some(row.into_record(&connection)?))
    }

    fn deleted_builtin_ids(&self) -> Result<Vec<SkillId>, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare("SELECT skill_id FROM deleted_builtin_skills ORDER BY skill_id")
            .map_err(repository_error)?;
        let values = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        values
            .into_iter()
            .map(|value| SkillId::parse(value).map_err(domain_data_error))
            .collect()
    }

    fn agent_mount_configurations(
        &self,
    ) -> Result<Vec<AgentMountConfiguration>, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT agents.id, skill_agent_mount_paths.mount_path
                FROM agents
                LEFT JOIN skill_agent_mount_paths
                  ON skill_agent_mount_paths.agent_id = agents.id
                WHERE agents.launch_kind <> 'api'
                ORDER BY agents.id
                "#,
            )
            .map_err(repository_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        rows.into_iter()
            .map(|(agent_id, configured_path)| {
                Ok(AgentMountConfiguration {
                    agent_id,
                    configured_path: configured_path
                        .map(SkillMountPath::parse)
                        .transpose()
                        .map_err(domain_data_error)?,
                })
            })
            .collect()
    }

    fn is_api_agent(&self, agent_id: &str) -> Result<bool, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM agents WHERE id = ?1 AND launch_kind = 'api')",
                params![agent_id],
                |row| row.get(0),
            )
            .map_err(repository_error)
    }

    fn compatible_agents(&self) -> Result<Vec<SkillCompatibleAgent>, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                "SELECT id, display_name, launch_kind, interface_format
                 FROM agents ORDER BY launch_kind, id",
            )
            .map_err(repository_error)?;
        let agents = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(repository_error)?
            .map(|row| {
                let (id, display_name, launch_kind, interface_format) =
                    row.map_err(repository_error)?;
                let is_api = launch_kind == "api";
                Ok(SkillCompatibleAgent {
                    id,
                    display_name,
                    kind: if is_api {
                        SkillAgentKind::Api
                    } else {
                        SkillAgentKind::Cli
                    },
                    delegation_runtime: SkillDelegationAgentRuntime::classify(
                        is_api,
                        interface_format.as_deref(),
                    ),
                })
            })
            .collect();
        agents
    }

    fn api_agent_bindings_for_location(
        &self,
        location: &SkillLocation,
    ) -> Result<BTreeMap<String, Vec<String>>, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT skill_id, agent_id
                FROM skill_api_agent_bindings
                WHERE scope = ?1 AND workspace_path = ?2
                ORDER BY skill_id, agent_id
                "#,
            )
            .map_err(repository_error)?;
        let rows = statement
            .query_map(
                params![location.scope.as_str(), location.storage_workspace_key()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(repository_error)?;
        let mut bindings = BTreeMap::<String, Vec<String>>::new();
        for row in rows {
            let (skill_id, agent_id) = row.map_err(repository_error)?;
            bindings.entry(skill_id).or_default().push(agent_id);
        }
        Ok(bindings)
    }

    fn enabled_skills_bound_to(
        &self,
        agent_id: &str,
    ) -> Result<Vec<SkillRecord>, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT skills.id, skills.scope, skills.workspace_path, skills.source,
                       skills.enabled, skills.skill_dir, skills.skill_md_path,
                       skills.content_hash, skills.metadata_json, skills.created_at,
                       skills.updated_at
                FROM skills
                INNER JOIN skill_agent_bindings
                  ON skills.id = skill_agent_bindings.skill_id
                 AND skills.scope = skill_agent_bindings.scope
                 AND skills.workspace_path = skill_agent_bindings.workspace_path
                WHERE skill_agent_bindings.agent_id = ?1 AND skills.enabled = 1
                ORDER BY skills.scope, skills.workspace_path, skills.id
                "#,
            )
            .map_err(repository_error)?;
        let rows = statement
            .query_map(params![agent_id], SkillRow::read)
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        rows.into_iter()
            .map(|row| row.into_record(&connection))
            .collect()
    }

    fn save_skills(
        &self,
        records: &[SkillRecord],
        clear_deleted_builtin_ids: &[SkillId],
    ) -> Result<(), SkillApplicationError> {
        let mut connection = self.database.connection().map_err(app_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        save_records(&transaction, records)?;
        clear_tombstones(&transaction, clear_deleted_builtin_ids)?;
        transaction.commit().map_err(repository_error)
    }

    fn delete_skill(
        &self,
        key: &SkillKey,
        record_builtin_tombstone: bool,
        deleted_at: &str,
    ) -> Result<(), SkillApplicationError> {
        let mut connection = self.database.connection().map_err(app_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        if record_builtin_tombstone {
            transaction
                .execute(
                    "INSERT OR REPLACE INTO deleted_builtin_skills (skill_id, deleted_at) VALUES (?1, ?2)",
                    params![key.id.as_str(), deleted_at],
                )
                .map_err(repository_error)?;
        }
        transaction
            .execute(
                r#"
                DELETE FROM skill_agent_bindings
                WHERE skill_id = ?1 AND scope = ?2 AND workspace_path = ?3
                "#,
                key_params(key),
            )
            .map_err(repository_error)?;
        transaction
            .execute(
                r#"
                DELETE FROM skill_api_agent_bindings
                WHERE skill_id = ?1 AND scope = ?2 AND workspace_path = ?3
                "#,
                key_params(key),
            )
            .map_err(repository_error)?;
        transaction
            .execute(
                "DELETE FROM skills WHERE id = ?1 AND scope = ?2 AND workspace_path = ?3",
                key_params(key),
            )
            .map_err(repository_error)?;
        transaction.commit().map_err(repository_error)
    }

    fn save_mount_path(
        &self,
        agent_id: &str,
        mount_path: &SkillMountPath,
        affected_records: &[SkillRecord],
        updated_at: &str,
    ) -> Result<(), SkillApplicationError> {
        let mut connection = self.database.connection().map_err(app_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        transaction
            .execute(
                r#"
                INSERT INTO skill_agent_mount_paths (agent_id, mount_path, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?3)
                ON CONFLICT(agent_id) DO UPDATE SET
                    mount_path = excluded.mount_path,
                    updated_at = excluded.updated_at
                "#,
                params![agent_id, mount_path.as_str(), updated_at],
            )
            .map_err(repository_error)?;
        save_records(&transaction, affected_records)?;
        transaction.commit().map_err(repository_error)
    }

    fn save_drift_snapshot(&self, report: &SkillDriftReport) -> Result<(), SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        save_report(&connection, report, &SystemClock.rfc3339())
    }

    fn save_synchronization(
        &self,
        records: &[SkillRecord],
        clear_deleted_builtin_ids: &[SkillId],
        report: &SkillDriftReport,
    ) -> Result<(), SkillApplicationError> {
        let mut connection = self.database.connection().map_err(app_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        save_records(&transaction, records)?;
        clear_tombstones(&transaction, clear_deleted_builtin_ids)?;
        save_report(&transaction, report, &SystemClock.rfc3339())?;
        transaction.commit().map_err(repository_error)
    }
}

impl SkillReconciliationRepository for SqliteSkillRepository {
    fn builtin_reconciliation(
        &self,
        id: &SkillId,
    ) -> Result<Option<BuiltinReconciliationState>, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT journal.reconciliation_version, journal.outcome,
                       journal.system_revision, journal.legacy_revision,
                       journal.cleanup_status, journal.backup_path, journal.error_code,
                       state.enabled, state.deletion_intent, state.effective_layer,
                       state.origin, state.availability, journal.updated_at
                FROM skill_builtin_reconciliation journal
                INNER JOIN skill_runtime_state state
                  ON state.skill_id = journal.skill_id
                 AND state.scope = 'global'
                 AND state.workspace_path = ''
                WHERE journal.skill_id = ?1
                "#,
            )
            .map_err(repository_error)?;
        let mut rows = statement
            .query(params![id.as_str()])
            .map_err(repository_error)?;
        let Some(row) = rows.next().map_err(repository_error)? else {
            return Ok(None);
        };
        let outcome = row.get::<_, String>(1).map_err(repository_error)?;
        let cleanup = row.get::<_, String>(4).map_err(repository_error)?;
        let layer = row.get::<_, String>(9).map_err(repository_error)?;
        let origin = row.get::<_, String>(10).map_err(repository_error)?;
        let availability = row.get::<_, String>(11).map_err(repository_error)?;
        Ok(Some(BuiltinReconciliationState {
            skill_id: id.clone(),
            reconciliation_version: row.get::<_, i64>(0).map_err(repository_error)? as u32,
            outcome: BuiltinReconciliationOutcome::parse(&outcome).ok_or_else(|| {
                invalid_data(format!("unknown reconciliation outcome: {outcome}"))
            })?,
            system_revision: row.get(2).map_err(repository_error)?,
            legacy_revision: row.get(3).map_err(repository_error)?,
            cleanup_status: BuiltinCleanupStatus::parse(&cleanup)
                .ok_or_else(|| invalid_data(format!("unknown cleanup status: {cleanup}")))?,
            backup_path: row.get(5).map_err(repository_error)?,
            error_code: row.get(6).map_err(repository_error)?,
            enabled: row.get::<_, i32>(7).map_err(repository_error)? != 0,
            deletion_intent: row.get::<_, i32>(8).map_err(repository_error)? != 0,
            effective_layer: SkillLayer::parse(&layer)
                .ok_or_else(|| invalid_data(format!("unknown effective layer: {layer}")))?,
            origin: SkillOrigin::parse(&origin)
                .ok_or_else(|| invalid_data(format!("unknown Skill origin: {origin}")))?,
            availability: SkillAvailability::parse(&availability)
                .ok_or_else(|| invalid_data(format!("unknown availability: {availability}")))?,
            updated_at: row.get(12).map_err(repository_error)?,
        }))
    }

    fn save_builtin_reconciliation(
        &self,
        state: &BuiltinReconciliationState,
        record: Option<&SkillRecord>,
        clear_tombstone: bool,
    ) -> Result<(), SkillApplicationError> {
        let mut connection = self.database.connection().map_err(app_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        if let Some(record) = record {
            save_record(&transaction, record)?;
        }
        if clear_tombstone {
            clear_tombstones(&transaction, std::slice::from_ref(&state.skill_id))?;
        }
        save_reconciliation_state(&transaction, state)?;
        bump_catalog_revision(&transaction, &state.updated_at)?;
        transaction.commit().map_err(repository_error)
    }

    fn complete_builtin_cleanup(
        &self,
        id: &SkillId,
        backup_path: Option<&str>,
        updated_at: &str,
    ) -> Result<(), SkillApplicationError> {
        let mut connection = self.database.connection().map_err(app_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        transaction
            .execute(
                r#"
                UPDATE skill_builtin_reconciliation
                SET cleanup_status = 'complete', backup_path = ?2, updated_at = ?3
                WHERE skill_id = ?1
                "#,
                params![id.as_str(), backup_path, updated_at],
            )
            .map_err(repository_error)?;
        bump_catalog_revision(&transaction, updated_at)?;
        transaction.commit().map_err(repository_error)
    }
}

fn save_reconciliation_state(
    transaction: &Transaction<'_>,
    state: &BuiltinReconciliationState,
) -> Result<(), SkillApplicationError> {
    transaction
        .execute(
            r#"
            INSERT INTO skill_builtin_reconciliation (
                skill_id, reconciliation_version, outcome, system_revision, legacy_revision,
                cleanup_status, backup_path, error_code, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(skill_id) DO UPDATE SET
                reconciliation_version = excluded.reconciliation_version,
                outcome = excluded.outcome,
                system_revision = excluded.system_revision,
                legacy_revision = excluded.legacy_revision,
                cleanup_status = excluded.cleanup_status,
                backup_path = excluded.backup_path,
                error_code = excluded.error_code,
                updated_at = excluded.updated_at
            "#,
            params![
                state.skill_id.as_str(),
                state.reconciliation_version,
                state.outcome.as_str(),
                state.system_revision,
                state.legacy_revision,
                state.cleanup_status.as_str(),
                state.backup_path,
                state.error_code,
                state.updated_at,
            ],
        )
        .map_err(repository_error)?;
    transaction
        .execute(
            r#"
            INSERT INTO skill_runtime_state (
                skill_id, scope, workspace_path, enabled, deletion_intent, effective_layer,
                origin, availability, reconciliation_version, state_revision, updated_at
            ) VALUES (?1, 'global', '', ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)
            ON CONFLICT(skill_id, scope, workspace_path) DO UPDATE SET
                enabled = excluded.enabled,
                deletion_intent = excluded.deletion_intent,
                effective_layer = excluded.effective_layer,
                origin = excluded.origin,
                availability = excluded.availability,
                reconciliation_version = excluded.reconciliation_version,
                state_revision = skill_runtime_state.state_revision + 1,
                updated_at = excluded.updated_at
            "#,
            params![
                state.skill_id.as_str(),
                state.enabled as i32,
                state.deletion_intent as i32,
                state.effective_layer.as_str(),
                state.origin.as_str(),
                state.availability.as_str(),
                state.reconciliation_version,
                state.updated_at,
            ],
        )
        .map_err(repository_error)?;
    Ok(())
}

fn bump_catalog_revision(
    transaction: &Transaction<'_>,
    updated_at: &str,
) -> Result<(), SkillApplicationError> {
    transaction
        .execute(
            "UPDATE skill_catalog_revision SET revision = revision + 1, updated_at = ?1 WHERE singleton = 1",
            params![updated_at],
        )
        .map_err(repository_error)?;
    Ok(())
}

impl SkillApiBindingRepository for SqliteSkillRepository {
    fn bind_api_agent(
        &self,
        key: &SkillKey,
        agent_id: &str,
        now: &str,
    ) -> Result<(), SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .execute(
                r#"
                INSERT INTO skill_api_agent_bindings
                (skill_id, scope, workspace_path, agent_id, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?5)
                ON CONFLICT(skill_id, scope, workspace_path, agent_id) DO UPDATE SET
                    updated_at = excluded.updated_at
                "#,
                params![
                    key.id.as_str(),
                    key.location.scope.as_str(),
                    key.location.storage_workspace_key(),
                    agent_id,
                    now,
                ],
            )
            .map_err(repository_error)?;
        Ok(())
    }

    fn unbind_api_agent(
        &self,
        key: &SkillKey,
        agent_id: &str,
    ) -> Result<(), SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .execute(
                r#"
                DELETE FROM skill_api_agent_bindings
                WHERE skill_id = ?1 AND scope = ?2 AND workspace_path = ?3 AND agent_id = ?4
                "#,
                params![
                    key.id.as_str(),
                    key.location.scope.as_str(),
                    key.location.storage_workspace_key(),
                    agent_id,
                ],
            )
            .map_err(repository_error)?;
        Ok(())
    }

    fn api_agent_bindings(&self, key: &SkillKey) -> Result<Vec<String>, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT agent_id FROM skill_api_agent_bindings
                WHERE skill_id = ?1 AND scope = ?2 AND workspace_path = ?3
                ORDER BY agent_id
                "#,
            )
            .map_err(repository_error)?;
        let agent_ids = statement
            .query_map(key_params(key), |row| row.get::<_, String>(0))
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        Ok(agent_ids)
    }

    fn enabled_skills_bound_to_api_agent(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Vec<SkillRecord>, SkillApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT skills.id, skills.scope, skills.workspace_path, skills.source,
                       skills.enabled, skills.skill_dir, skills.skill_md_path,
                       skills.content_hash, skills.metadata_json, skills.created_at,
                       skills.updated_at
                FROM skills
                INNER JOIN skill_api_agent_bindings
                  ON skills.id = skill_api_agent_bindings.skill_id
                 AND skills.scope = skill_api_agent_bindings.scope
                 AND skills.workspace_path = skill_api_agent_bindings.workspace_path
                WHERE skill_api_agent_bindings.agent_id = ?1
                  AND skills.enabled = 1
                  AND (
                    skills.scope = 'global'
                    OR (skills.scope = 'workspace' AND skills.workspace_path = ?2)
                  )
                ORDER BY skills.scope, skills.workspace_path, skills.id
                "#,
            )
            .map_err(repository_error)?;
        let rows = statement
            .query_map(
                params![agent_id, workspace_path.unwrap_or("")],
                SkillRow::read,
            )
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        rows.into_iter()
            .map(|row| row.into_record(&connection))
            .collect()
    }
}

pub(crate) fn apply_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS skills (
            id TEXT NOT NULL,
            scope TEXT NOT NULL,
            workspace_path TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            skill_dir TEXT NOT NULL,
            skill_md_path TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            metadata_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (id, scope, workspace_path)
        );

        CREATE TABLE IF NOT EXISTS skill_agent_bindings (
            skill_id TEXT NOT NULL,
            scope TEXT NOT NULL,
            workspace_path TEXT NOT NULL DEFAULT '',
            agent_id TEXT NOT NULL,
            mounted_path TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (skill_id, scope, workspace_path, agent_id)
        );

        CREATE TABLE IF NOT EXISTS skill_api_agent_bindings (
            skill_id TEXT NOT NULL,
            scope TEXT NOT NULL,
            workspace_path TEXT NOT NULL DEFAULT '',
            agent_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (skill_id, scope, workspace_path, agent_id)
        );

        CREATE TABLE IF NOT EXISTS skill_agent_mount_paths (
            agent_id TEXT PRIMARY KEY,
            mount_path TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS deleted_builtin_skills (
            skill_id TEXT PRIMARY KEY,
            deleted_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS skill_drift_snapshots (
            scope TEXT NOT NULL,
            workspace_path TEXT NOT NULL DEFAULT '',
            drift_hash TEXT NOT NULL,
            issues_json TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (scope, workspace_path)
        );

        CREATE INDEX IF NOT EXISTS idx_skills_scope_workspace_source_id
          ON skills(scope, workspace_path, source, id);
        CREATE INDEX IF NOT EXISTS idx_skill_agent_bindings_agent_scope_workspace
          ON skill_agent_bindings(agent_id, scope, workspace_path);
        CREATE INDEX IF NOT EXISTS idx_skill_api_bindings_agent_scope_workspace_skill
          ON skill_api_agent_bindings(agent_id, scope, workspace_path, skill_id);
        "#,
    )?;
    apply_effective_runtime_schema(connection)?;
    Ok(())
}

pub(crate) fn apply_effective_runtime_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS skill_runtime_state (
            skill_id TEXT NOT NULL,
            scope TEXT NOT NULL,
            workspace_path TEXT NOT NULL DEFAULT '',
            enabled INTEGER NOT NULL DEFAULT 1,
            deletion_intent INTEGER NOT NULL DEFAULT 0,
            effective_layer TEXT,
            origin TEXT,
            availability TEXT,
            reconciliation_version INTEGER NOT NULL DEFAULT 0,
            state_revision INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (skill_id, scope, workspace_path)
        );

        CREATE TABLE IF NOT EXISTS skill_builtin_reconciliation (
            skill_id TEXT PRIMARY KEY,
            reconciliation_version INTEGER NOT NULL,
            outcome TEXT NOT NULL,
            system_revision TEXT NOT NULL,
            legacy_revision TEXT,
            cleanup_status TEXT NOT NULL,
            backup_path TEXT,
            error_code TEXT,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS skill_catalog_revision (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            revision INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );

        INSERT OR IGNORE INTO skill_catalog_revision (singleton, revision, updated_at)
        VALUES (1, 1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

        INSERT OR IGNORE INTO skill_runtime_state (
            skill_id, scope, workspace_path, enabled, deletion_intent, effective_layer,
            origin, availability, reconciliation_version, state_revision, updated_at
        )
        SELECT id, scope, workspace_path, enabled, 0,
               CASE WHEN source = 'builtin' THEN 'system' ELSE 'user' END,
               CASE source
                   WHEN 'builtin' THEN 'shipped'
                   WHEN 'imported' THEN 'imported'
                   ELSE 'created'
               END,
               CASE WHEN enabled = 1 THEN 'available' ELSE 'disabled' END,
               0, 1, updated_at
        FROM skills;

        INSERT OR IGNORE INTO skill_runtime_state (
            skill_id, scope, workspace_path, enabled, deletion_intent, effective_layer,
            origin, availability, reconciliation_version, state_revision, updated_at
        )
        SELECT skill_id, 'global', '', 1, 1, 'system', 'shipped', 'disabled', 0, 1,
               deleted_at
        FROM deleted_builtin_skills;

        UPDATE skill_runtime_state
        SET deletion_intent = 1,
            state_revision = CASE WHEN state_revision < 1 THEN 1 ELSE state_revision END
        WHERE scope = 'global' AND workspace_path = ''
          AND skill_id IN (SELECT skill_id FROM deleted_builtin_skills);

        CREATE INDEX IF NOT EXISTS idx_skill_runtime_state_scope_workspace
          ON skill_runtime_state(scope, workspace_path, skill_id);
        CREATE INDEX IF NOT EXISTS idx_skill_reconciliation_outcome
          ON skill_builtin_reconciliation(reconciliation_version, outcome, skill_id);
        "#,
    )?;
    Ok(())
}

fn list_records(
    connection: &Connection,
    location: &SkillLocation,
) -> Result<Vec<SkillRecord>, SkillApplicationError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, scope, workspace_path, source, enabled, skill_dir, skill_md_path,
                   content_hash, metadata_json, created_at, updated_at
            FROM skills
            WHERE scope = ?1 AND workspace_path = ?2
            ORDER BY source ASC, id ASC
            "#,
        )
        .map_err(repository_error)?;
    let rows = statement
        .query_map(
            params![location.scope.as_str(), location.storage_workspace_key()],
            SkillRow::read,
        )
        .map_err(repository_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(repository_error)?;
    let mut records = rows
        .into_iter()
        .map(SkillRow::into_record_without_bindings)
        .collect::<Result<Vec<_>, _>>()?;
    let bindings = load_bindings_for_location(connection, location)?;
    for record in &mut records {
        record.bindings = bindings
            .get(record.key.id.as_str())
            .cloned()
            .unwrap_or_default();
    }
    Ok(records)
}

pub(crate) fn apply_reliability_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS skill_api_agent_bindings (
            skill_id TEXT NOT NULL,
            scope TEXT NOT NULL,
            workspace_path TEXT NOT NULL DEFAULT '',
            agent_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (skill_id, scope, workspace_path, agent_id)
        );

        DELETE FROM skill_agent_bindings
        WHERE NOT EXISTS (
            SELECT 1 FROM skills
            WHERE skills.id = skill_agent_bindings.skill_id
              AND skills.scope = skill_agent_bindings.scope
              AND skills.workspace_path = skill_agent_bindings.workspace_path
        ) OR NOT EXISTS (
            SELECT 1 FROM agents
            WHERE agents.id = skill_agent_bindings.agent_id
              AND agents.launch_kind <> 'api'
        );

        DELETE FROM skill_api_agent_bindings
        WHERE NOT EXISTS (
            SELECT 1 FROM skills
            WHERE skills.id = skill_api_agent_bindings.skill_id
              AND skills.scope = skill_api_agent_bindings.scope
              AND skills.workspace_path = skill_api_agent_bindings.workspace_path
        ) OR NOT EXISTS (
            SELECT 1 FROM agents
            WHERE agents.id = skill_api_agent_bindings.agent_id
              AND agents.launch_kind = 'api'
        );

        DELETE FROM skill_agent_mount_paths
        WHERE NOT EXISTS (
            SELECT 1 FROM agents
            WHERE agents.id = skill_agent_mount_paths.agent_id
              AND agents.launch_kind <> 'api'
        );

        CREATE INDEX IF NOT EXISTS idx_skills_scope_workspace_source_id
          ON skills(scope, workspace_path, source, id);
        CREATE INDEX IF NOT EXISTS idx_skill_agent_bindings_agent_scope_workspace
          ON skill_agent_bindings(agent_id, scope, workspace_path);
        CREATE INDEX IF NOT EXISTS idx_skill_api_bindings_agent_scope_workspace_skill
          ON skill_api_agent_bindings(agent_id, scope, workspace_path, skill_id);
        "#,
    )?;
    Ok(())
}

struct SkillRow {
    id: String,
    scope: String,
    workspace_path: String,
    source: String,
    enabled: bool,
    skill_dir: String,
    skill_md_path: String,
    content_hash: String,
    metadata_json: String,
    created_at: String,
    updated_at: String,
}

impl SkillRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            scope: row.get(1)?,
            workspace_path: row.get(2)?,
            source: row.get(3)?,
            enabled: row.get::<_, i32>(4)? != 0,
            skill_dir: row.get(5)?,
            skill_md_path: row.get(6)?,
            content_hash: row.get(7)?,
            metadata_json: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
        })
    }

    fn into_record(self, connection: &Connection) -> Result<SkillRecord, SkillApplicationError> {
        let mut record = self.into_record_without_bindings()?;
        record.bindings = load_bindings(connection, &record.key)?;
        Ok(record)
    }

    fn into_record_without_bindings(self) -> Result<SkillRecord, SkillApplicationError> {
        let scope = SkillScope::parse(&self.scope)
            .ok_or_else(|| invalid_data(format!("unknown Skill scope: {}", self.scope)))?;
        let location = SkillLocation::new(
            scope,
            (scope == SkillScope::Workspace).then_some(self.workspace_path.as_str()),
        )
        .map_err(domain_data_error)?;
        let id = SkillId::parse(self.id).map_err(domain_data_error)?;
        let metadata = metadata_from_json(&self.metadata_json)?;
        if metadata.id != id {
            return Err(invalid_data("Skill metadata id does not match its row key"));
        }
        let key = SkillKey::new(id, location);
        Ok(SkillRecord {
            key,
            source: SkillSource::parse(&self.source)
                .ok_or_else(|| invalid_data(format!("unknown Skill source: {}", self.source)))?,
            enabled: self.enabled,
            managed_source: ManagedSkillSource {
                skill_dir: self.skill_dir,
                skill_md_path: self.skill_md_path,
                content_hash: self.content_hash,
            },
            metadata,
            bindings: Vec::new(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            resolved_metadata: None,
        })
    }
}

fn load_bindings_for_location(
    connection: &Connection,
    location: &SkillLocation,
) -> Result<BTreeMap<String, Vec<SkillAgentBinding>>, SkillApplicationError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT bindings.skill_id, bindings.agent_id, bindings.mounted_path, bindings.status,
                   mount_paths.mount_path
            FROM skill_agent_bindings bindings
            LEFT JOIN skill_agent_mount_paths mount_paths
              ON mount_paths.agent_id = bindings.agent_id
            WHERE bindings.scope = ?1 AND bindings.workspace_path = ?2
            ORDER BY bindings.skill_id, bindings.agent_id
            "#,
        )
        .map_err(repository_error)?;
    let rows = statement
        .query_map(
            params![location.scope.as_str(), location.storage_workspace_key()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .map_err(repository_error)?;
    let mut result = BTreeMap::<String, Vec<SkillAgentBinding>>::new();
    for row in rows {
        let (skill_id, agent_id, mounted_path, status, configured_path) =
            row.map_err(repository_error)?;
        let mount_path = SkillMountPath::parse(
            configured_path.unwrap_or_else(|| default_mount_path(&agent_id).to_string()),
        )
        .map_err(domain_data_error)?;
        result.entry(skill_id).or_default().push(SkillAgentBinding {
            agent_id,
            mount_path,
            mounted_path,
            mounted: status == "mounted",
        });
    }
    Ok(result)
}

fn load_bindings(
    connection: &Connection,
    key: &SkillKey,
) -> Result<Vec<SkillAgentBinding>, SkillApplicationError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT bindings.agent_id, bindings.mounted_path, bindings.status,
                   mount_paths.mount_path
            FROM skill_agent_bindings bindings
            LEFT JOIN skill_agent_mount_paths mount_paths
              ON mount_paths.agent_id = bindings.agent_id
            WHERE bindings.skill_id = ?1
              AND bindings.scope = ?2
              AND bindings.workspace_path = ?3
            ORDER BY bindings.agent_id
            "#,
        )
        .map_err(repository_error)?;
    let rows = statement
        .query_map(key_params(key), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(repository_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(repository_error)?;
    rows.into_iter()
        .map(|(agent_id, mounted_path, status, configured_path)| {
            let mount_path = SkillMountPath::parse(
                configured_path.unwrap_or_else(|| default_mount_path(&agent_id).to_string()),
            )
            .map_err(domain_data_error)?;
            Ok(SkillAgentBinding {
                agent_id,
                mount_path,
                mounted_path,
                mounted: status == "mounted",
            })
        })
        .collect()
}

fn save_records(
    transaction: &Transaction<'_>,
    records: &[SkillRecord],
) -> Result<(), SkillApplicationError> {
    for record in records {
        save_record(transaction, record)?;
    }
    Ok(())
}

fn save_record(
    transaction: &Transaction<'_>,
    record: &SkillRecord,
) -> Result<(), SkillApplicationError> {
    transaction
        .execute(
            r#"
            INSERT INTO skills
            (id, scope, workspace_path, source, enabled, skill_dir, skill_md_path, content_hash,
             metadata_json, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id, scope, workspace_path) DO UPDATE SET
                source = excluded.source,
                enabled = excluded.enabled,
                skill_dir = excluded.skill_dir,
                skill_md_path = excluded.skill_md_path,
                content_hash = excluded.content_hash,
                metadata_json = excluded.metadata_json,
                updated_at = excluded.updated_at
            "#,
            params![
                record.key.id.as_str(),
                record.key.location.scope.as_str(),
                record.key.location.storage_workspace_key(),
                record.source.as_str(),
                record.enabled as i32,
                record.managed_source.skill_dir,
                record.managed_source.skill_md_path,
                record.managed_source.content_hash,
                metadata_to_json(&record.metadata)?,
                record.created_at,
                record.updated_at,
            ],
        )
        .map_err(repository_error)?;
    transaction
        .execute(
            r#"
            DELETE FROM skill_agent_bindings
            WHERE skill_id = ?1 AND scope = ?2 AND workspace_path = ?3
            "#,
            key_params(&record.key),
        )
        .map_err(repository_error)?;
    for binding in &record.bindings {
        transaction
            .execute(
                r#"
                INSERT INTO skill_agent_bindings
                (skill_id, scope, workspace_path, agent_id, mounted_path, status,
                 created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
                "#,
                params![
                    record.key.id.as_str(),
                    record.key.location.scope.as_str(),
                    record.key.location.storage_workspace_key(),
                    binding.agent_id,
                    binding.mounted_path,
                    binding_status(record.enabled, binding.mounted),
                    record.updated_at,
                ],
            )
            .map_err(repository_error)?;
    }
    Ok(())
}

fn clear_tombstones(
    transaction: &Transaction<'_>,
    ids: &[SkillId],
) -> Result<(), SkillApplicationError> {
    for id in ids {
        transaction
            .execute(
                "DELETE FROM deleted_builtin_skills WHERE skill_id = ?1",
                params![id.as_str()],
            )
            .map_err(repository_error)?;
    }
    Ok(())
}

fn save_report(
    connection: &Connection,
    report: &SkillDriftReport,
    updated_at: &str,
) -> Result<(), SkillApplicationError> {
    connection
        .execute(
            r#"
            INSERT INTO skill_drift_snapshots
            (scope, workspace_path, drift_hash, issues_json, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(scope, workspace_path) DO UPDATE SET
                drift_hash = excluded.drift_hash,
                issues_json = excluded.issues_json,
                updated_at = excluded.updated_at
            "#,
            params![
                report.location.scope.as_str(),
                report.location.storage_workspace_key(),
                report.drift_hash,
                issues_to_json(report)?,
                updated_at,
            ],
        )
        .map_err(repository_error)?;
    Ok(())
}

fn key_params(key: &SkillKey) -> [&str; 3] {
    [
        key.id.as_str(),
        key.location.scope.as_str(),
        key.location.storage_workspace_key(),
    ]
}

fn binding_status(enabled: bool, mounted: bool) -> &'static str {
    if mounted {
        "mounted"
    } else if enabled {
        "pending"
    } else {
        "disabled"
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedMetadata {
    id: String,
    name: String,
    description: String,
    category: String,
    version: String,
    triggers: Vec<String>,
}

impl From<&SkillMetadata> for PersistedMetadata {
    fn from(metadata: &SkillMetadata) -> Self {
        Self {
            id: metadata.id.as_str().to_string(),
            name: metadata.name.clone(),
            description: metadata.description.clone(),
            category: metadata.category.clone(),
            version: metadata.version.clone(),
            triggers: metadata.triggers.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedIssue<'a> {
    skill_id: &'a str,
    #[serde(rename = "type")]
    issue_type: PersistedIssueType,
    agent_id: Option<&'a str>,
    path: Option<&'a str>,
    message: &'a str,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum PersistedIssueType {
    MissingSource,
    MetadataChanged,
    UnregisteredSource,
    MissingMount,
    Conflict,
    DeletedBuiltin,
}

fn metadata_to_json(metadata: &SkillMetadata) -> Result<String, SkillApplicationError> {
    serde_json::to_string(&PersistedMetadata::from(metadata)).map_err(json_error)
}

fn metadata_from_json(value: &str) -> Result<SkillMetadata, SkillApplicationError> {
    let value = serde_json::from_str::<PersistedMetadata>(value).map_err(json_error)?;
    SkillMetadata::new(
        value.id,
        value.name,
        value.description,
        value.category,
        value.version,
        value.triggers,
    )
    .map_err(domain_data_error)
}

fn issues_to_json(report: &SkillDriftReport) -> Result<String, SkillApplicationError> {
    let issues = report
        .issues
        .iter()
        .map(|issue| PersistedIssue {
            skill_id: &issue.skill_id,
            issue_type: match issue.issue_type {
                SkillDriftIssueType::MissingSource => PersistedIssueType::MissingSource,
                SkillDriftIssueType::MetadataChanged => PersistedIssueType::MetadataChanged,
                SkillDriftIssueType::UnregisteredSource => PersistedIssueType::UnregisteredSource,
                SkillDriftIssueType::MissingMount => PersistedIssueType::MissingMount,
                SkillDriftIssueType::Conflict => PersistedIssueType::Conflict,
                SkillDriftIssueType::DeletedBuiltin => PersistedIssueType::DeletedBuiltin,
            },
            agent_id: issue.agent_id.as_deref(),
            path: issue.path.as_deref(),
            message: issue.message,
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&issues).map_err(json_error)
}

fn app_error(error: crate::platform::database::DatabaseError) -> SkillApplicationError {
    match error {
        crate::platform::database::DatabaseError::Database(error) => repository_error(error),
        crate::platform::database::DatabaseError::Storage(message) => {
            SkillApplicationError::Repository(message)
        }
    }
}

fn repository_error(error: rusqlite::Error) -> SkillApplicationError {
    SkillApplicationError::Repository(error.to_string())
}

fn json_error(error: serde_json::Error) -> SkillApplicationError {
    invalid_data(error.to_string())
}

fn domain_data_error(error: impl std::fmt::Display) -> SkillApplicationError {
    invalid_data(error.to_string())
}

fn invalid_data(message: impl Into<String>) -> SkillApplicationError {
    SkillApplicationError::Repository(format!("Invalid persisted Skill data: {}", message.into()))
}

fn reconcile_workspace_aliases(
    database: &NativeDatabase,
    location: &SkillLocation,
) -> Result<(), SkillApplicationError> {
    if location.scope != SkillScope::Workspace {
        return Ok(());
    }
    let canonical_key = location.storage_workspace_key();
    let canonical_comparison = if cfg!(windows) {
        canonical_key.to_lowercase()
    } else {
        canonical_key.to_string()
    };
    let connection = database.connection().map_err(app_error)?;
    let mut statement = connection
        .prepare("SELECT DISTINCT workspace_path FROM skills WHERE scope = 'workspace'")
        .map_err(repository_error)?;
    let persisted = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(repository_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(repository_error)?;
    drop(statement);
    let mut aliases = persisted
        .into_iter()
        .filter(|path| path != canonical_key)
        .filter(|path| {
            std::path::Path::new(path)
                .canonicalize()
                .ok()
                .map(|resolved| canonical_path_key(&resolved) == canonical_comparison)
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    if aliases.is_empty() {
        return Ok(());
    }
    aliases.sort();
    let connection = connection;
    let transaction = begin_write_transaction(&connection)
        .map_err(|error| SkillApplicationError::Repository(error.to_string()))?;
    for alias in &aliases {
        let duplicate_ids: i64 = transaction
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM skills alias
                INNER JOIN skills canonical
                  ON canonical.id = alias.id
                 AND canonical.scope = 'workspace'
                 AND canonical.workspace_path = ?1
                WHERE alias.scope = 'workspace' AND alias.workspace_path = ?2
                "#,
                params![canonical_key, alias],
                |row| row.get(0),
            )
            .map_err(repository_error)?;
        let duplicate_snapshots: i64 = transaction
            .query_row(
                r#"
                SELECT (
                    EXISTS(SELECT 1 FROM skill_drift_snapshots
                           WHERE scope = 'workspace' AND workspace_path = ?1)
                    AND
                    EXISTS(SELECT 1 FROM skill_drift_snapshots
                           WHERE scope = 'workspace' AND workspace_path = ?2)
                )
                "#,
                params![canonical_key, alias],
                |row| row.get(0),
            )
            .map_err(repository_error)?;
        if duplicate_ids > 0 || duplicate_snapshots > 0 {
            return Err(SkillApplicationError::Validation(format!(
                "Conflicting legacy Workspace Skill aliases require manual reconciliation: {alias}"
            )));
        }
        for table in [
            "skill_agent_bindings",
            "skill_api_agent_bindings",
            "skill_drift_snapshots",
            "skills",
        ] {
            transaction
                .execute(
                    &format!(
                        "UPDATE {table} SET workspace_path = ?1 \
                         WHERE scope = 'workspace' AND workspace_path = ?2"
                    ),
                    params![canonical_key, alias],
                )
                .map_err(repository_error)?;
        }
    }
    transaction.commit().map_err(repository_error)
}

fn canonical_path_key(path: &std::path::Path) -> String {
    let value = path.to_string_lossy();
    let value = value.strip_prefix(r"\\?\").unwrap_or(&value).to_string();
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::domain::{SkillDriftIssue, SkillDriftIssueType};
    use crate::test_support::TempDirectory;
    use rusqlite::trace::{TraceEvent, TraceEventCodes};
    use std::cell::Cell;

    thread_local! {
        static SKILL_LIST_STATEMENT_COUNT: Cell<usize> = const { Cell::new(0) };
    }

    fn count_skill_list_statement(event: TraceEvent<'_>) {
        if matches!(event, TraceEvent::Stmt(_, _)) {
            SKILL_LIST_STATEMENT_COUNT.with(|count| count.set(count.get() + 1));
        }
    }

    fn measured_list(fixture: &Fixture) -> (Vec<SkillRecord>, usize) {
        let connection = fixture.database.connection().expect("database connection");
        SKILL_LIST_STATEMENT_COUNT.with(|count| count.set(0));
        connection.trace_v2(
            TraceEventCodes::SQLITE_TRACE_STMT,
            Some(count_skill_list_statement),
        );
        let records = list_records(&connection, &location()).expect("batch list");
        connection.trace_v2(TraceEventCodes::empty(), None);
        let count = SKILL_LIST_STATEMENT_COUNT.with(Cell::get);
        (records, count)
    }

    struct Fixture {
        _directory: TempDirectory,
        database: NativeDatabase,
        repository: SqliteSkillRepository,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let directory = TempDirectory::new(label);
            let database =
                NativeDatabase::new(directory.path().to_path_buf()).expect("test database");
            database.connection().expect("migrated database");
            Self {
                repository: SqliteSkillRepository::new(database.clone()),
                database,
                _directory: directory,
            }
        }
    }

    fn location() -> SkillLocation {
        SkillLocation::new(SkillScope::Workspace, Some("D:/fixture")).expect("workspace location")
    }

    fn record(value: &str, agent_id: Option<&str>) -> SkillRecord {
        let id = SkillId::parse(value).expect("Skill id");
        SkillRecord {
            key: SkillKey::new(id.clone(), location()),
            source: SkillSource::User,
            enabled: true,
            managed_source: ManagedSkillSource {
                skill_dir: format!("D:/fixture/.vanehub/skills/{value}"),
                skill_md_path: format!("D:/fixture/.vanehub/skills/{value}/SKILL.md"),
                content_hash: "fixture-hash".to_string(),
            },
            metadata: SkillMetadata::new(
                value,
                "Fixture Skill",
                "Fixture description",
                "testing",
                "1.0.0",
                vec!["fixture".to_string()],
            )
            .expect("metadata"),
            bindings: agent_id
                .map(|agent_id| {
                    vec![SkillAgentBinding {
                        agent_id: agent_id.to_string(),
                        mount_path: SkillMountPath::parse(".codex/skills").expect("mount path"),
                        mounted_path: format!("D:/fixture/.codex/skills/{value}"),
                        mounted: true,
                    }]
                })
                .unwrap_or_default(),
            created_at: "2026-07-18T00:00:00Z".to_string(),
            updated_at: "2026-07-18T00:00:00Z".to_string(),
            resolved_metadata: None,
        }
    }

    #[test]
    fn repository_round_trip_preserves_skill_metadata_binding_and_mount_contracts() {
        let fixture = Fixture::new("Skill SQLite round trip");
        let expected = record("fixture-skill", Some("codex-cli"));

        fixture
            .repository
            .save_skills(std::slice::from_ref(&expected), &[])
            .expect("save Skill");
        let loaded = fixture.repository.list(&location()).expect("list Skills");

        assert_eq!(loaded, vec![expected]);
        assert_eq!(
            fixture
                .repository
                .agent_mount_configurations()
                .expect("mount configurations")
                .len(),
            5
        );
    }

    #[test]
    fn behavior_write_rolls_back_skill_when_binding_persistence_fails() {
        let fixture = Fixture::new("Skill SQLite atomic write");
        fixture
            .database
            .connection()
            .expect("database connection")
            .execute_batch(
                r#"
                CREATE TRIGGER reject_fixture_binding
                BEFORE INSERT ON skill_agent_bindings
                WHEN NEW.skill_id = 'atomic-skill'
                BEGIN
                    SELECT RAISE(ABORT, 'fixture binding rejection');
                END;
                "#,
            )
            .expect("failure trigger");

        let error = fixture
            .repository
            .save_skills(&[record("atomic-skill", Some("codex-cli"))], &[])
            .expect_err("atomic write failure");

        assert!(error.to_string().contains("fixture binding rejection"));
        let connection = fixture.database.connection().expect("database connection");
        let skill_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM skills WHERE id = 'atomic-skill'",
                [],
                |row| row.get(0),
            )
            .expect("Skill count");
        assert_eq!(skill_count, 0);
    }

    #[test]
    fn representative_legacy_rows_remain_readable_without_domain_fallbacks() {
        let fixture = Fixture::new("Skill SQLite legacy row");
        let connection = fixture.database.connection().expect("database connection");
        connection
            .execute(
                r#"
                INSERT INTO skills
                (id, scope, workspace_path, source, enabled, skill_dir, skill_md_path,
                 content_hash, metadata_json, created_at, updated_at)
                VALUES (?1, 'workspace', 'D:/fixture', 'imported', 0, ?2, ?3, 'legacy-hash',
                        ?4, '1700000000', '1700000001')
                "#,
                params![
                    "legacy-skill",
                    "D:/fixture/.vanehub/skills/legacy-skill",
                    "D:/fixture/.vanehub/skills/legacy-skill/SKILL.md",
                    r#"{"id":"legacy-skill","name":"Legacy","description":"Readable","category":"testing","version":"1.0.0","triggers":["legacy"]}"#,
                ],
            )
            .expect("legacy row");

        let loaded = fixture.repository.list(&location()).expect("legacy list");

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].key.id.as_str(), "legacy-skill");
        assert_eq!(loaded[0].source, SkillSource::Imported);
        assert!(!loaded[0].enabled);
        assert_eq!(loaded[0].metadata.name, "Legacy");
    }

    #[test]
    fn synchronization_commits_records_tombstones_and_drift_snapshot_together() {
        let fixture = Fixture::new("Skill SQLite synchronization");
        let deleted = SkillId::parse("code-review").expect("builtin id");
        let connection = fixture.database.connection().expect("database connection");
        connection
            .execute(
                "INSERT INTO deleted_builtin_skills (skill_id, deleted_at) VALUES (?1, ?2)",
                params![deleted.as_str(), "2026-07-18T00:00:00Z"],
            )
            .expect("tombstone");
        drop(connection);
        let report = SkillDriftReport {
            location: location(),
            issues: vec![SkillDriftIssue {
                skill_id: "sync-skill".to_string(),
                issue_type: SkillDriftIssueType::MissingMount,
                agent_id: Some("codex-cli".to_string()),
                path: Some("D:/fixture/.codex/skills/sync-skill".to_string()),
                message: "Agent mount is missing",
            }],
            drift_hash: "sync-hash".to_string(),
        };

        fixture
            .repository
            .save_synchronization(&[record("sync-skill", None)], &[deleted], &report)
            .expect("synchronization");

        assert!(fixture
            .repository
            .deleted_builtin_ids()
            .expect("tombstones")
            .is_empty());
        let connection = fixture.database.connection().expect("database connection");
        let snapshot: (String, String) = connection
            .query_row(
                "SELECT drift_hash, issues_json FROM skill_drift_snapshots WHERE scope = 'workspace' AND workspace_path = 'D:/fixture'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("drift snapshot");
        assert_eq!(snapshot.0, "sync-hash");
        assert!(snapshot.1.contains("missing-mount"));
        assert!(!snapshot.1.contains("issue_type"));
    }

    #[test]
    fn synchronization_rolls_back_records_and_tombstones_when_snapshot_persistence_fails() {
        let fixture = Fixture::new("Skill SQLite synchronization rollback");
        let deleted = SkillId::parse("code-review").expect("builtin id");
        let connection = fixture.database.connection().expect("database connection");
        connection
            .execute(
                "INSERT INTO deleted_builtin_skills (skill_id, deleted_at) VALUES (?1, ?2)",
                params![deleted.as_str(), "2026-07-18T00:00:00Z"],
            )
            .expect("tombstone");
        connection
            .execute_batch(
                r#"
                CREATE TRIGGER reject_skill_drift_snapshot
                BEFORE INSERT ON skill_drift_snapshots
                BEGIN
                    SELECT RAISE(ABORT, 'injected drift snapshot failure');
                END;
                "#,
            )
            .expect("failure injection trigger");
        drop(connection);
        let report = SkillDriftReport {
            location: location(),
            issues: Vec::new(),
            drift_hash: "clean-after-repair".to_string(),
        };

        fixture
            .repository
            .save_synchronization(
                &[record("rollback-skill", None)],
                std::slice::from_ref(&deleted),
                &report,
            )
            .expect_err("injected snapshot failure must abort synchronization");

        assert!(fixture
            .repository
            .list(&location())
            .expect("Skill rows")
            .is_empty());
        assert_eq!(
            fixture
                .repository
                .deleted_builtin_ids()
                .expect("tombstones"),
            vec![deleted]
        );
        let connection = fixture.database.connection().expect("database connection");
        let snapshot_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM skill_drift_snapshots", [], |row| {
                row.get(0)
            })
            .expect("snapshot count");
        assert_eq!(snapshot_count, 0);
    }

    #[test]
    fn api_agent_lookup_includes_seeded_builtin_onepiece_and_excludes_cli_agents() {
        let fixture = Fixture::new("Skill API agent lookup");

        assert!(fixture
            .repository
            .is_api_agent("onepiece")
            .expect("OnePiece lookup"));
        assert!(!fixture
            .repository
            .is_api_agent("codex-cli")
            .expect("CLI lookup"));
        assert!(!fixture
            .repository
            .is_api_agent("missing-agent")
            .expect("missing lookup"));
    }

    #[test]
    fn api_agent_binding_round_trips_and_unbind_removes_it() {
        let fixture = Fixture::new("Skill API agent binding round trip");
        let expected = record("fixture-skill", None);
        fixture
            .repository
            .save_skills(std::slice::from_ref(&expected), &[])
            .expect("save Skill");

        fixture
            .repository
            .bind_api_agent(&expected.key, "my-api-agent", "2026-07-18T00:00:00Z")
            .expect("bind");
        assert_eq!(
            fixture
                .repository
                .api_agent_bindings(&expected.key)
                .expect("bindings"),
            vec!["my-api-agent".to_string()]
        );

        fixture
            .repository
            .unbind_api_agent(&expected.key, "my-api-agent")
            .expect("unbind");
        assert!(fixture
            .repository
            .api_agent_bindings(&expected.key)
            .expect("bindings")
            .is_empty());
    }

    #[test]
    fn binding_an_already_bound_skill_updates_rather_than_duplicates() {
        let fixture = Fixture::new("Skill API agent binding idempotent");
        let expected = record("fixture-skill", None);
        fixture
            .repository
            .save_skills(std::slice::from_ref(&expected), &[])
            .expect("save Skill");

        fixture
            .repository
            .bind_api_agent(&expected.key, "my-api-agent", "2026-07-18T00:00:00Z")
            .expect("bind once");
        fixture
            .repository
            .bind_api_agent(&expected.key, "my-api-agent", "2026-07-19T00:00:00Z")
            .expect("bind again");

        assert_eq!(
            fixture
                .repository
                .api_agent_bindings(&expected.key)
                .expect("bindings"),
            vec!["my-api-agent".to_string()]
        );
    }

    #[test]
    fn unbinding_a_never_bound_pair_is_a_no_op_not_an_error() {
        let fixture = Fixture::new("Skill API agent unbind no-op");
        let expected = record("fixture-skill", None);
        fixture
            .repository
            .save_skills(std::slice::from_ref(&expected), &[])
            .expect("save Skill");

        fixture
            .repository
            .unbind_api_agent(&expected.key, "never-bound-agent")
            .expect("unbind never-bound pair");
    }

    #[test]
    fn enabled_skills_bound_to_api_agent_excludes_disabled_skills() {
        let fixture = Fixture::new("Skill API agent enabled filter");
        let mut disabled = record("disabled-skill", None);
        disabled.enabled = false;
        let enabled = record("enabled-skill", None);
        fixture
            .repository
            .save_skills(&[disabled.clone(), enabled.clone()], &[])
            .expect("save Skills");
        fixture
            .repository
            .bind_api_agent(&disabled.key, "my-api-agent", "2026-07-18T00:00:00Z")
            .expect("bind disabled");
        fixture
            .repository
            .bind_api_agent(&enabled.key, "my-api-agent", "2026-07-18T00:00:00Z")
            .expect("bind enabled");

        let bound = fixture
            .repository
            .enabled_skills_bound_to_api_agent("my-api-agent", Some("D:/fixture"))
            .expect("enabled skills");

        assert_eq!(bound.len(), 1);
        assert_eq!(bound[0].key.id.as_str(), "enabled-skill");
    }

    #[test]
    fn deleting_a_skill_cascades_to_its_api_agent_bindings() {
        let fixture = Fixture::new("Skill API agent binding cascade delete");
        let expected = record("fixture-skill", None);
        fixture
            .repository
            .save_skills(std::slice::from_ref(&expected), &[])
            .expect("save Skill");
        fixture
            .repository
            .bind_api_agent(&expected.key, "my-api-agent", "2026-07-18T00:00:00Z")
            .expect("bind");

        fixture
            .repository
            .delete_skill(&expected.key, false, "2026-07-18T00:00:00Z")
            .expect("delete Skill");

        assert!(fixture
            .repository
            .api_agent_bindings(&expected.key)
            .expect("bindings")
            .is_empty());
    }

    #[test]
    fn effective_runtime_migration_preserves_legacy_enablement_and_deletion_intent() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE skills (
                    id TEXT NOT NULL,
                    scope TEXT NOT NULL,
                    workspace_path TEXT NOT NULL DEFAULT '',
                    source TEXT NOT NULL,
                    enabled INTEGER NOT NULL,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY (id, scope, workspace_path)
                );
                CREATE TABLE deleted_builtin_skills (
                    skill_id TEXT PRIMARY KEY,
                    deleted_at TEXT NOT NULL
                );
                INSERT INTO skills VALUES
                    ('code-review', 'global', '', 'builtin', 0, '2026-08-01T00:00:00Z'),
                    ('custom-skill', 'workspace', 'D:/work', 'user', 1, '2026-08-02T00:00:00Z');
                INSERT INTO deleted_builtin_skills VALUES
                    ('readme-generation', '2026-08-03T00:00:00Z');
                "#,
            )
            .expect("legacy schema");

        apply_effective_runtime_schema(&connection).expect("effective runtime migration");
        apply_effective_runtime_schema(&connection).expect("idempotent migration");

        let disabled: (i64, String, String) = connection
            .query_row(
                "SELECT enabled, effective_layer, availability FROM skill_runtime_state WHERE skill_id = 'code-review'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("disabled state");
        let deleted: (i64, i64) = connection
            .query_row(
                "SELECT enabled, deletion_intent FROM skill_runtime_state WHERE skill_id = 'readme-generation'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("deletion state");
        let revision: i64 = connection
            .query_row(
                "SELECT revision FROM skill_catalog_revision WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .expect("revision");
        assert_eq!(disabled, (0, "system".to_string(), "disabled".to_string()));
        assert_eq!(deleted, (1, 1));
        assert_eq!(revision, 1);
    }

    #[test]
    fn reliability_migration_cleans_invalid_rows_and_creates_lookup_indexes() {
        let fixture = Fixture::new("Skill reliability migration");
        let connection = fixture.database.connection().expect("database connection");
        connection
            .execute(
                "INSERT INTO agents (id, display_name, provider, launch_kind) VALUES ('api-only', 'API', 'Test', 'api')",
                [],
            )
            .expect("API Agent");
        connection
            .execute(
                "INSERT INTO skill_agent_bindings \
                 (skill_id, scope, workspace_path, agent_id, mounted_path, status, created_at, updated_at) \
                 VALUES ('missing-skill', 'global', '', 'api-only', '.skills/missing', 'pending', 'now', 'now')",
                [],
            )
            .expect("invalid CLI binding");
        connection
            .execute(
                "INSERT INTO skill_agent_mount_paths (agent_id, mount_path, created_at, updated_at) \
                 VALUES ('api-only', '.skills', 'now', 'now')",
                [],
            )
            .expect("invalid mount path");

        apply_reliability_schema(&connection).expect("reliability migration");

        let invalid_rows: i64 = connection
            .query_row(
                "SELECT \
                    (SELECT COUNT(*) FROM skill_agent_bindings WHERE agent_id = 'api-only') + \
                    (SELECT COUNT(*) FROM skill_agent_mount_paths WHERE agent_id = 'api-only')",
                [],
                |row| row.get(0),
            )
            .expect("invalid row count");
        let indexes = connection
            .prepare(
                "SELECT name FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx_skill%'",
            )
            .expect("index query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("index rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("index names");
        assert_eq!(invalid_rows, 0);
        assert!(indexes.contains(&"idx_skills_scope_workspace_source_id".to_string()));
        assert!(indexes.contains(&"idx_skill_agent_bindings_agent_scope_workspace".to_string()));
        assert!(indexes.contains(&"idx_skill_api_bindings_agent_scope_workspace_skill".to_string()));
    }

    #[test]
    fn list_statement_count_is_constant_as_skill_count_grows() {
        let small = Fixture::new("Skill batch overview small");
        small
            .repository
            .save_skills(&[record("skill-small", Some("codex-cli"))], &[])
            .expect("save one Skill");
        let (small_records, small_statement_count) = measured_list(&small);

        let large = Fixture::new("Skill batch overview large");
        let records = (0..100)
            .map(|index| record(&format!("skill-{index}"), Some("codex-cli")))
            .collect::<Vec<_>>();
        large
            .repository
            .save_skills(&records, &[])
            .expect("save many Skills");
        let (large_records, large_statement_count) = measured_list(&large);
        let api_bindings = large
            .repository
            .api_agent_bindings_for_location(&location())
            .expect("batch API bindings");
        let agents = large
            .repository
            .compatible_agents()
            .expect("compatible agents");

        assert_eq!(small_records.len(), 1);
        assert_eq!(large_records.len(), 100);
        assert!(large_records.iter().all(|skill| skill.bindings.len() == 1));
        assert_eq!(small_statement_count, 2);
        assert_eq!(large_statement_count, small_statement_count);
        assert!(api_bindings.is_empty());
        assert!(!agents.is_empty());
    }

    #[test]
    fn workspace_aliases_merge_when_unambiguous_and_report_duplicate_identity_conflicts() {
        let fixture = Fixture::new("Skill workspace aliases");
        let workspace = TempDirectory::new("Skill canonical workspace");
        let canonical_path = canonical_path_key(
            &workspace
                .path()
                .canonicalize()
                .expect("canonical workspace"),
        );
        let alias_path = workspace.path().join(".").to_string_lossy().to_string();
        let canonical_location =
            SkillLocation::new(SkillScope::Workspace, Some(&canonical_path)).expect("canonical");
        let alias_location =
            SkillLocation::new(SkillScope::Workspace, Some(&alias_path)).expect("alias");
        let mut aliased = record("aliased-skill", None);
        aliased.key.location = alias_location.clone();
        fixture
            .repository
            .save_skills(std::slice::from_ref(&aliased), &[])
            .expect("save alias");

        let merged = fixture
            .repository
            .list(&canonical_location)
            .expect("unambiguous merge");
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].key.location, canonical_location);

        let mut alias_duplicate = record("duplicate-skill", None);
        alias_duplicate.key.location = alias_location;
        let mut canonical_duplicate = record("duplicate-skill", None);
        canonical_duplicate.key.location = canonical_location.clone();
        fixture
            .repository
            .save_skills(&[alias_duplicate, canonical_duplicate], &[])
            .expect("save conflicting aliases");

        let error = fixture
            .repository
            .list(&canonical_location)
            .expect_err("ambiguous alias conflict");
        assert!(matches!(error, SkillApplicationError::Validation(_)));
    }
}
