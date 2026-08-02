use super::super::application::CliConfigRepository;
use super::super::domain::{
    AppliedStateRecord, CliConfigDriftState, CliConfigError, CliConfigPayload, ProfileRecord,
};
use crate::platform::database::{table_has_column, NativeDatabase};
use rusqlite::{params, Connection, OptionalExtension, Row};

#[derive(Clone)]
pub(crate) struct SqliteCliConfigRepository {
    database: NativeDatabase,
}

impl SqliteCliConfigRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<crate::platform::database::PooledSqlite, CliConfigError> {
        self.database
            .connection()
            .map_err(|_| CliConfigError::Repository)
    }
}

pub(crate) fn apply_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS cli_config_profiles (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            name TEXT NOT NULL,
            payload_version INTEGER NOT NULL,
            payload_json TEXT NOT NULL,
            managed_keys_json TEXT NOT NULL,
            source_preset_id TEXT,
            source_preset_version INTEGER,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            sort_position INTEGER NOT NULL DEFAULT 0,
            UNIQUE (agent_id, name),
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_cli_config_profiles_agent_sort
            ON cli_config_profiles(agent_id, sort_position, created_at);

        CREATE TABLE IF NOT EXISTS cli_config_applied_state (
            agent_id TEXT PRIMARY KEY,
            profile_id TEXT,
            projection_fingerprint TEXT NOT NULL,
            live_fingerprint TEXT NOT NULL,
            drift_state TEXT NOT NULL,
            applied_at TEXT NOT NULL,
            applied_payload_json TEXT,
            managed_keys_json TEXT NOT NULL DEFAULT '[]',
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE,
            FOREIGN KEY (profile_id) REFERENCES cli_config_profiles(id) ON DELETE SET NULL
        );
        CREATE INDEX IF NOT EXISTS idx_cli_config_applied_profile
            ON cli_config_applied_state(profile_id);
        "#,
    )?;
    Ok(())
}

pub(crate) fn apply_applied_snapshot_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    if !table_has_column(
        connection,
        "cli_config_applied_state",
        "applied_payload_json",
    )? {
        connection.execute_batch(
            "ALTER TABLE cli_config_applied_state ADD COLUMN applied_payload_json TEXT;",
        )?;
    }
    if !table_has_column(connection, "cli_config_applied_state", "managed_keys_json")? {
        connection.execute_batch(
            "ALTER TABLE cli_config_applied_state ADD COLUMN managed_keys_json TEXT NOT NULL DEFAULT '[]';",
        )?;
    }
    Ok(())
}

impl CliConfigRepository for SqliteCliConfigRepository {
    fn list_profiles(&self, agent_id: &str) -> Result<Vec<ProfileRecord>, CliConfigError> {
        let connection = self.connection()?;
        list_profiles_on(&connection, agent_id)
    }

    fn get_profile(
        &self,
        agent_id: &str,
        profile_id: &str,
    ) -> Result<ProfileRecord, CliConfigError> {
        let connection = self.connection()?;
        get_profile_on(&connection, agent_id, profile_id)
    }

    fn profile_id_exists(&self, profile_id: &str) -> Result<bool, CliConfigError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cli_config_profiles WHERE id = ?1)",
                [profile_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|_| CliConfigError::Repository)
    }

    fn save_profile(&self, profile: &ProfileRecord) -> Result<(), CliConfigError> {
        let connection = self.connection()?;
        save_profile_on(&connection, profile)
    }

    fn delete_profile(&self, agent_id: &str, profile_id: &str) -> Result<(), CliConfigError> {
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "DELETE FROM cli_config_profiles WHERE agent_id = ?1 AND id = ?2",
                params![agent_id, profile_id],
            )
            .map_err(|_| CliConfigError::Repository)?;
        if changed == 0 {
            return Err(CliConfigError::NotFound);
        }
        Ok(())
    }

    fn applied_state(&self, agent_id: &str) -> Result<Option<AppliedStateRecord>, CliConfigError> {
        let connection = self.connection()?;
        applied_state_on(&connection, agent_id)
    }

    fn save_applied_state(&self, state: &AppliedStateRecord) -> Result<(), CliConfigError> {
        let connection = self.connection()?;
        let applied_payload_json = state
            .applied_payload
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| CliConfigError::Repository)?;
        let managed_keys_json =
            serde_json::to_string(&state.managed_keys).map_err(|_| CliConfigError::Repository)?;
        connection
            .execute(
                "INSERT INTO cli_config_applied_state
                    (agent_id, profile_id, projection_fingerprint, live_fingerprint, drift_state,
                     applied_at, applied_payload_json, managed_keys_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(agent_id) DO UPDATE SET
                    profile_id = excluded.profile_id,
                    projection_fingerprint = excluded.projection_fingerprint,
                    live_fingerprint = excluded.live_fingerprint,
                    drift_state = excluded.drift_state,
                    applied_at = excluded.applied_at,
                    applied_payload_json = excluded.applied_payload_json,
                    managed_keys_json = excluded.managed_keys_json",
                params![
                    state.agent_id,
                    state.profile_id,
                    state.projection_fingerprint,
                    state.live_fingerprint,
                    drift_state_name(&state.drift_state),
                    state.applied_at,
                    applied_payload_json,
                    managed_keys_json,
                ],
            )
            .map_err(|_| CliConfigError::Repository)?;
        Ok(())
    }

    fn clear_applied_state(&self, agent_id: &str) -> Result<(), CliConfigError> {
        self.connection()?
            .execute(
                "DELETE FROM cli_config_applied_state WHERE agent_id = ?1",
                [agent_id],
            )
            .map_err(|_| CliConfigError::Repository)?;
        Ok(())
    }
}

fn list_profiles_on(
    connection: &Connection,
    agent_id: &str,
) -> Result<Vec<ProfileRecord>, CliConfigError> {
    let mut statement = connection
        .prepare(
            "SELECT id, agent_id, name, payload_version, payload_json, managed_keys_json,
                    source_preset_id, source_preset_version, created_at, updated_at, sort_position
             FROM cli_config_profiles WHERE agent_id = ?1
             ORDER BY sort_position, created_at, id",
        )
        .map_err(|_| CliConfigError::Repository)?;
    let rows = statement
        .query_map([agent_id], profile_from_row)
        .map_err(|_| CliConfigError::Repository)?;
    let profiles = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CliConfigError::Repository)?;
    Ok(profiles)
}

fn get_profile_on(
    connection: &Connection,
    agent_id: &str,
    profile_id: &str,
) -> Result<ProfileRecord, CliConfigError> {
    connection
        .query_row(
            "SELECT id, agent_id, name, payload_version, payload_json, managed_keys_json,
                    source_preset_id, source_preset_version, created_at, updated_at, sort_position
             FROM cli_config_profiles WHERE agent_id = ?1 AND id = ?2",
            params![agent_id, profile_id],
            profile_from_row,
        )
        .optional()
        .map_err(|_| CliConfigError::Repository)?
        .ok_or(CliConfigError::NotFound)
}

fn save_profile_on(connection: &Connection, profile: &ProfileRecord) -> Result<(), CliConfigError> {
    let payload_json =
        serde_json::to_string(&profile.payload).map_err(|_| CliConfigError::Repository)?;
    let managed_keys_json =
        serde_json::to_string(&profile.managed_keys).map_err(|_| CliConfigError::Repository)?;
    connection
        .execute(
            "INSERT INTO cli_config_profiles
                (id, agent_id, name, payload_version, payload_json, managed_keys_json,
                 source_preset_id, source_preset_version, created_at, updated_at, sort_position)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                payload_version = excluded.payload_version,
                payload_json = excluded.payload_json,
                managed_keys_json = excluded.managed_keys_json,
                source_preset_id = excluded.source_preset_id,
                source_preset_version = excluded.source_preset_version,
                updated_at = excluded.updated_at,
                sort_position = excluded.sort_position
             WHERE cli_config_profiles.agent_id = excluded.agent_id",
            params![
                profile.id,
                profile.agent_id,
                profile.name,
                profile.payload_version,
                payload_json,
                managed_keys_json,
                profile.source_preset_id,
                profile.source_preset_version,
                profile.created_at,
                profile.updated_at,
                profile.sort_position,
            ],
        )
        .map_err(|_| CliConfigError::Repository)?;
    Ok(())
}

fn applied_state_on(
    connection: &Connection,
    agent_id: &str,
) -> Result<Option<AppliedStateRecord>, CliConfigError> {
    connection
        .query_row(
            "SELECT agent_id, profile_id, projection_fingerprint, live_fingerprint,
                    drift_state, applied_at, applied_payload_json, managed_keys_json
             FROM cli_config_applied_state WHERE agent_id = ?1",
            [agent_id],
            |row| {
                let drift: String = row.get(4)?;
                let applied_payload_json: Option<String> = row.get(6)?;
                let managed_keys_json: String = row.get(7)?;
                let applied_payload = applied_payload_json
                    .map(|json| {
                        serde_json::from_str::<CliConfigPayload>(&json).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                json.len(),
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })
                    })
                    .transpose()?;
                let managed_keys = serde_json::from_str::<Vec<String>>(&managed_keys_json)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            managed_keys_json.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(AppliedStateRecord {
                    agent_id: row.get(0)?,
                    profile_id: row.get(1)?,
                    projection_fingerprint: row.get(2)?,
                    live_fingerprint: row.get(3)?,
                    drift_state: drift_state_from_name(&drift),
                    applied_at: row.get(5)?,
                    applied_payload,
                    managed_keys,
                })
            },
        )
        .optional()
        .map_err(|_| CliConfigError::Repository)
}

fn profile_from_row(row: &Row<'_>) -> rusqlite::Result<ProfileRecord> {
    let payload_json: String = row.get(4)?;
    let managed_keys_json: String = row.get(5)?;
    let payload = serde_json::from_str::<CliConfigPayload>(&payload_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            payload_json.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })?;
    let managed_keys =
        serde_json::from_str::<Vec<String>>(&managed_keys_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                managed_keys_json.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
    Ok(ProfileRecord {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        name: row.get(2)?,
        payload_version: row.get(3)?,
        payload,
        managed_keys,
        source_preset_id: row.get(6)?,
        source_preset_version: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        sort_position: row.get(10)?,
    })
}

fn drift_state_name(state: &CliConfigDriftState) -> &'static str {
    match state {
        CliConfigDriftState::Detached => "detached",
        CliConfigDriftState::Applied => "applied",
        CliConfigDriftState::Drifted => "drifted",
        CliConfigDriftState::Malformed => "malformed",
        CliConfigDriftState::Missing => "missing",
    }
}

fn drift_state_from_name(value: &str) -> CliConfigDriftState {
    match value {
        "applied" => CliConfigDriftState::Applied,
        "drifted" => CliConfigDriftState::Drifted,
        "malformed" => CliConfigDriftState::Malformed,
        "missing" => CliConfigDriftState::Missing,
        _ => CliConfigDriftState::Detached,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli_config::domain::{ClaudeAuthMode, PAYLOAD_VERSION};
    use std::collections::BTreeMap;

    fn connection() -> Connection {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                "PRAGMA foreign_keys = ON;
                 CREATE TABLE agents (id TEXT PRIMARY KEY);
                 INSERT INTO agents VALUES ('claude-code'), ('codex-cli'), ('opencode');",
            )
            .expect("agents");
        apply_schema(&connection).expect("schema");
        connection
    }

    fn profile() -> ProfileRecord {
        ProfileRecord {
            id: "deepseek".into(),
            agent_id: "claude-code".into(),
            name: "DeepSeek".into(),
            payload_version: PAYLOAD_VERSION,
            payload: CliConfigPayload::ClaudeCode {
                base_url: "https://api.deepseek.com/anthropic".into(),
                auth_mode: ClaudeAuthMode::AuthToken,
                model: "deepseek-chat".into(),
                haiku_model: "deepseek-chat".into(),
                sonnet_model: "deepseek-chat".into(),
                opus_model: "deepseek-chat".into(),
                advanced_env: BTreeMap::new(),
            },
            managed_keys: vec!["ANTHROPIC_MODEL".into()],
            source_preset_id: Some("claude-code-deepseek".into()),
            source_preset_version: Some(1),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            sort_position: 0,
        }
    }

    #[test]
    fn profile_and_applied_state_round_trip() {
        let connection = connection();
        let profile = profile();
        save_profile_on(&connection, &profile).expect("save");
        assert_eq!(
            list_profiles_on(&connection, "claude-code")
                .expect("list")
                .len(),
            1
        );
        assert!(list_profiles_on(&connection, "codex-cli")
            .expect("isolated")
            .is_empty());

        let applied = AppliedStateRecord {
            agent_id: "claude-code".into(),
            profile_id: Some(profile.id.clone()),
            projection_fingerprint: "projection".into(),
            live_fingerprint: "live".into(),
            drift_state: CliConfigDriftState::Applied,
            applied_at: "2026-01-01T00:00:00Z".into(),
            applied_payload: Some(profile.payload.clone()),
            managed_keys: profile.managed_keys.clone(),
        };
        let applied_payload_json =
            serde_json::to_string(&applied.applied_payload).expect("applied payload");
        let managed_keys_json = serde_json::to_string(&applied.managed_keys).expect("managed keys");
        connection
            .execute(
                "INSERT INTO cli_config_applied_state
                    (agent_id, profile_id, projection_fingerprint, live_fingerprint, drift_state,
                     applied_at, applied_payload_json, managed_keys_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    "claude-code",
                    "deepseek",
                    "projection",
                    "live",
                    "applied",
                    applied.applied_at,
                    applied_payload_json,
                    managed_keys_json,
                ],
            )
            .expect("applied");
        let restored = applied_state_on(&connection, "claude-code")
            .expect("state")
            .expect("applied state");
        assert_eq!(restored.profile_id, Some("deepseek".into()));
        assert_eq!(restored.applied_payload, applied.applied_payload);
        assert_eq!(restored.managed_keys, applied.managed_keys);
    }

    #[test]
    fn schema_preserves_unrelated_tables_and_enforces_agent_ownership() {
        let connection = connection();
        connection
            .execute_batch("CREATE TABLE legacy(value TEXT); INSERT INTO legacy VALUES ('kept');")
            .expect("legacy");
        apply_schema(&connection).expect("repeat schema");
        assert_eq!(
            connection
                .query_row("SELECT value FROM legacy", [], |row| row
                    .get::<_, String>(0))
                .expect("legacy row"),
            "kept"
        );
        let mut invalid = profile();
        invalid.agent_id = "gemini-cli".into();
        assert!(save_profile_on(&connection, &invalid).is_err());
    }
}
