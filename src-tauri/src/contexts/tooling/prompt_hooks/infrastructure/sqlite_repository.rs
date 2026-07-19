use crate::contexts::tooling::prompt_hooks::application::{
    PromptHookApplicationError, PromptHookGovernance, PromptHookOverride, PromptHookRecord,
    PromptHookRepository, PromptHookTrace, PromptHookTraceStatus,
};
use crate::contexts::tooling::prompt_hooks::domain::{
    ManagedCliAgentId, PromptHookBindings, PromptHookCategory, PromptHookId, PromptHookManifest,
    PromptHookSource, PromptHookStage,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection, ErrorCode, Row, Transaction};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub(crate) struct SqlitePromptHookRepository {
    database: NativeDatabase,
}

impl SqlitePromptHookRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl PromptHookRepository for SqlitePromptHookRepository {
    fn list_user_hooks(&self) -> Result<Vec<PromptHookRecord>, PromptHookApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT id, name, description, category, stage, hook_order, version, enabled,
                       disableable, cli_bindings, governance, template_body, created_at, updated_at
                FROM prompt_hooks_user
                "#,
            )
            .map_err(repository_error)?;
        let records = statement
            .query_map([], UserHookRow::read)
            .map_err(repository_error)?
            .map(|row| {
                row.map_err(repository_error)
                    .and_then(UserHookRow::into_record)
            })
            .collect();
        records
    }

    fn list_builtin_overrides(
        &self,
    ) -> Result<Vec<PromptHookOverride>, PromptHookApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare("SELECT hook_id, enabled, cli_bindings, updated_at FROM prompt_hook_overrides")
            .map_err(repository_error)?;
        let overrides = statement
            .query_map([], OverrideRow::read)
            .map_err(repository_error)?
            .map(|row| {
                row.map_err(repository_error)
                    .and_then(OverrideRow::into_override)
            })
            .collect();
        overrides
    }

    fn create_user_hook(
        &self,
        record: &PromptHookRecord,
    ) -> Result<(), PromptHookApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        match insert_user_hook(&connection, record) {
            Ok(()) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(code, _))
                if code.code == ErrorCode::ConstraintViolation =>
            {
                Err(PromptHookApplicationError::Conflict(
                    record.id().as_str().to_string(),
                ))
            }
            Err(error) => Err(repository_error(error)),
        }
    }

    fn update_user_hook(
        &self,
        record: &PromptHookRecord,
    ) -> Result<(), PromptHookApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let bindings = json_string(&record.manifest.bindings().to_strings())?;
        let governance = json_string(&PersistedGovernance::from(&record.governance))?;
        let changed = connection
            .execute(
                r#"
                UPDATE prompt_hooks_user
                SET name = ?1, description = ?2, category = ?3, stage = ?4, hook_order = ?5,
                    version = ?6, enabled = ?7, disableable = ?8, cli_bindings = ?9,
                    governance = ?10, template_body = ?11, updated_at = ?12
                WHERE id = ?13
                "#,
                params![
                    record.manifest.name().as_str(),
                    record.description,
                    record.manifest.category().as_str(),
                    record.manifest.stage().as_str(),
                    record.manifest.order().value(),
                    record.version,
                    bool_to_i64(record.enabled),
                    bool_to_i64(record.disableable),
                    bindings,
                    governance,
                    record.manifest.template().as_str(),
                    record.updated_at,
                    record.id().as_str(),
                ],
            )
            .map_err(repository_error)?;
        if changed == 0 {
            Err(PromptHookApplicationError::NotFound(
                record.id().as_str().to_string(),
            ))
        } else {
            Ok(())
        }
    }

    fn delete_user_hook(&self, hook_id: &PromptHookId) -> Result<(), PromptHookApplicationError> {
        let mut connection = self.database.connection().map_err(app_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        let changed = transaction
            .execute(
                "DELETE FROM prompt_hooks_user WHERE id = ?1",
                params![hook_id.as_str()],
            )
            .map_err(repository_error)?;
        transaction
            .execute(
                "DELETE FROM prompt_hook_overrides WHERE hook_id = ?1",
                params![hook_id.as_str()],
            )
            .map_err(repository_error)?;
        if changed == 0 {
            return Err(PromptHookApplicationError::NotFound(
                hook_id.as_str().to_string(),
            ));
        }
        transaction.commit().map_err(repository_error)
    }

    fn set_user_enabled(
        &self,
        hook_id: &PromptHookId,
        enabled: bool,
        updated_at: &str,
    ) -> Result<(), PromptHookApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let changed = connection
            .execute(
                "UPDATE prompt_hooks_user SET enabled = ?1, updated_at = ?2 WHERE id = ?3",
                params![bool_to_i64(enabled), updated_at, hook_id.as_str()],
            )
            .map_err(repository_error)?;
        changed_or_not_found(changed, hook_id)
    }

    fn set_user_bindings(
        &self,
        hook_id: &PromptHookId,
        bindings: &PromptHookBindings,
        updated_at: &str,
    ) -> Result<(), PromptHookApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let changed = connection
            .execute(
                "UPDATE prompt_hooks_user SET cli_bindings = ?1, updated_at = ?2 WHERE id = ?3",
                params![
                    json_string(&bindings.to_strings())?,
                    updated_at,
                    hook_id.as_str()
                ],
            )
            .map_err(repository_error)?;
        changed_or_not_found(changed, hook_id)
    }

    fn save_builtin_override(
        &self,
        override_record: &PromptHookOverride,
    ) -> Result<(), PromptHookApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .execute(
                r#"
                INSERT INTO prompt_hook_overrides (hook_id, enabled, cli_bindings, updated_at)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(hook_id) DO UPDATE SET
                    enabled = excluded.enabled,
                    cli_bindings = excluded.cli_bindings,
                    updated_at = excluded.updated_at
                "#,
                params![
                    override_record.hook_id.as_str(),
                    bool_to_i64(override_record.enabled),
                    json_string(&override_record.bindings.to_strings())?,
                    override_record.updated_at,
                ],
            )
            .map_err(repository_error)?;
        Ok(())
    }

    fn save_traces(
        &self,
        traces: &[PromptHookTrace],
        retained_limit: usize,
    ) -> Result<(), PromptHookApplicationError> {
        let mut connection = self.database.connection().map_err(app_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        for trace in traces {
            insert_trace(&transaction, trace).map_err(repository_error)?;
        }
        transaction
            .execute(
                "DELETE FROM prompt_hook_traces WHERE id NOT IN (SELECT id FROM prompt_hook_traces ORDER BY created_at DESC, id DESC LIMIT ?1)",
                params![retained_limit as i64],
            )
            .map_err(repository_error)?;
        transaction.commit().map_err(repository_error)
    }

    fn list_traces(
        &self,
        limit: usize,
    ) -> Result<Vec<PromptHookTrace>, PromptHookApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT id, hook_id, category, stage, status, version, content_hash,
                       token_estimate, reason, agent_id, session_id, created_at
                FROM prompt_hook_traces
                ORDER BY created_at DESC, id DESC
                LIMIT ?1
                "#,
            )
            .map_err(repository_error)?;
        let traces = statement
            .query_map(params![limit as i64], TraceRow::read)
            .map_err(repository_error)?
            .map(|row| row.map_err(repository_error).and_then(TraceRow::into_trace))
            .collect();
        traces
    }
}

pub(crate) fn apply_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS prompt_hook_overrides (
            hook_id TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL,
            cli_bindings TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS prompt_hooks_user (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            category TEXT NOT NULL,
            stage TEXT NOT NULL,
            hook_order INTEGER NOT NULL,
            version INTEGER NOT NULL,
            enabled INTEGER NOT NULL,
            disableable INTEGER NOT NULL,
            cli_bindings TEXT NOT NULL,
            governance TEXT NOT NULL,
            template_body TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS prompt_hook_traces (
            id TEXT PRIMARY KEY,
            hook_id TEXT NOT NULL,
            category TEXT NOT NULL,
            stage TEXT NOT NULL,
            status TEXT NOT NULL,
            version INTEGER,
            content_hash TEXT,
            token_estimate INTEGER,
            reason TEXT,
            agent_id TEXT,
            session_id TEXT,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_prompt_hook_traces_created
            ON prompt_hook_traces(created_at DESC);
        "#,
    )?;
    Ok(())
}

fn insert_user_hook(connection: &Connection, record: &PromptHookRecord) -> rusqlite::Result<()> {
    let bindings = serde_json::to_string(&record.manifest.bindings().to_strings())
        .map_err(json_to_sql_error)?;
    let governance = serde_json::to_string(&PersistedGovernance::from(&record.governance))
        .map_err(json_to_sql_error)?;
    connection.execute(
        r#"
        INSERT INTO prompt_hooks_user (
            id, name, description, category, stage, hook_order, version, enabled, disableable,
            cli_bindings, governance, template_body, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        "#,
        params![
            record.id().as_str(),
            record.manifest.name().as_str(),
            record.description,
            record.manifest.category().as_str(),
            record.manifest.stage().as_str(),
            record.manifest.order().value(),
            record.version,
            bool_to_i64(record.enabled),
            bool_to_i64(record.disableable),
            bindings,
            governance,
            record.manifest.template().as_str(),
            record.created_at,
            record.updated_at,
        ],
    )?;
    Ok(())
}

fn insert_trace(transaction: &Transaction<'_>, trace: &PromptHookTrace) -> rusqlite::Result<()> {
    transaction.execute(
        r#"
        INSERT INTO prompt_hook_traces (
            id, hook_id, category, stage, status, version, content_hash, token_estimate,
            reason, agent_id, session_id, created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
        params![
            trace.id,
            trace.hook_id.as_str(),
            trace.category.as_str(),
            trace.stage.as_str(),
            trace.status.as_str(),
            trace.version,
            trace.content_hash,
            trace.token_estimate,
            trace.reason,
            trace.agent_id.map(ManagedCliAgentId::as_str),
            trace.session_id,
            trace.created_at,
        ],
    )?;
    Ok(())
}

struct UserHookRow {
    id: String,
    name: String,
    description: String,
    category: String,
    stage: String,
    order: i64,
    version: i64,
    enabled: bool,
    disableable: bool,
    bindings_json: String,
    governance_json: String,
    template_body: String,
    created_at: String,
    updated_at: String,
}

impl UserHookRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            category: row.get(3)?,
            stage: row.get(4)?,
            order: row.get(5)?,
            version: row.get(6)?,
            enabled: row.get::<_, i64>(7)? != 0,
            disableable: row.get::<_, i64>(8)? != 0,
            bindings_json: row.get(9)?,
            governance_json: row.get(10)?,
            template_body: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    }

    fn into_record(self) -> Result<PromptHookRecord, PromptHookApplicationError> {
        let category = PromptHookCategory::parse(&self.category).ok_or_else(|| {
            invalid_data(format!("unknown Prompt Hook category: {}", self.category))
        })?;
        let stage = PromptHookStage::parse(&self.stage)
            .ok_or_else(|| invalid_data(format!("unknown Prompt Hook stage: {}", self.stage)))?;
        let binding_values =
            serde_json::from_str::<Vec<String>>(&self.bindings_json).unwrap_or_default();
        let manifest = PromptHookManifest::new(
            self.id,
            self.name,
            category,
            stage,
            self.order,
            self.template_body,
            &binding_values,
        )
        .map_err(invalid_data)?;
        let governance = serde_json::from_str::<PersistedGovernance>(&self.governance_json)
            .unwrap_or_else(|_| PersistedGovernance::fallback_user())
            .into();
        Ok(PromptHookRecord {
            manifest,
            description: self.description,
            version: self.version,
            source: PromptHookSource::User,
            enabled: self.enabled,
            disableable: self.disableable,
            governance,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

struct OverrideRow {
    hook_id: String,
    enabled: bool,
    bindings_json: String,
    updated_at: String,
}

impl OverrideRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            hook_id: row.get(0)?,
            enabled: row.get::<_, i64>(1)? != 0,
            bindings_json: row.get(2)?,
            updated_at: row.get(3)?,
        })
    }

    fn into_override(self) -> Result<PromptHookOverride, PromptHookApplicationError> {
        let binding_values =
            serde_json::from_str::<Vec<String>>(&self.bindings_json).unwrap_or_default();
        Ok(PromptHookOverride {
            hook_id: PromptHookId::parse(self.hook_id).map_err(invalid_data)?,
            enabled: self.enabled,
            bindings: PromptHookBindings::new(&binding_values).map_err(invalid_data)?,
            updated_at: self.updated_at,
        })
    }
}

struct TraceRow {
    id: String,
    hook_id: String,
    category: String,
    stage: String,
    status: String,
    version: Option<i64>,
    content_hash: Option<String>,
    token_estimate: Option<i64>,
    reason: Option<String>,
    agent_id: Option<String>,
    session_id: Option<String>,
    created_at: String,
}

impl TraceRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            hook_id: row.get(1)?,
            category: row.get(2)?,
            stage: row.get(3)?,
            status: row.get(4)?,
            version: row.get(5)?,
            content_hash: row.get(6)?,
            token_estimate: row.get(7)?,
            reason: row.get(8)?,
            agent_id: row.get(9)?,
            session_id: row.get(10)?,
            created_at: row.get(11)?,
        })
    }

    fn into_trace(self) -> Result<PromptHookTrace, PromptHookApplicationError> {
        Ok(PromptHookTrace {
            id: self.id,
            hook_id: PromptHookId::parse(self.hook_id).map_err(invalid_data)?,
            category: PromptHookCategory::parse(&self.category).ok_or_else(|| {
                invalid_data(format!("unknown Prompt Hook category: {}", self.category))
            })?,
            stage: PromptHookStage::parse(&self.stage).ok_or_else(|| {
                invalid_data(format!("unknown Prompt Hook stage: {}", self.stage))
            })?,
            status: PromptHookTraceStatus::parse(&self.status).ok_or_else(|| {
                invalid_data(format!("unknown Prompt Hook trace status: {}", self.status))
            })?,
            version: self.version,
            content_hash: self.content_hash,
            token_estimate: self.token_estimate,
            reason: self.reason,
            agent_id: self
                .agent_id
                .as_deref()
                .map(ManagedCliAgentId::parse)
                .transpose()
                .map_err(invalid_data)?,
            session_id: self.session_id,
            created_at: self.created_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedGovernance {
    safety_tier: String,
    transparency_tier: String,
    governance_tier: String,
}

impl PersistedGovernance {
    fn fallback_user() -> Self {
        Self {
            safety_tier: "readonly".to_string(),
            transparency_tier: "opt-in-view".to_string(),
            governance_tier: "human-gated".to_string(),
        }
    }
}

impl From<&PromptHookGovernance> for PersistedGovernance {
    fn from(value: &PromptHookGovernance) -> Self {
        Self {
            safety_tier: value.safety_tier.clone(),
            transparency_tier: value.transparency_tier.clone(),
            governance_tier: value.governance_tier.clone(),
        }
    }
}

impl From<PersistedGovernance> for PromptHookGovernance {
    fn from(value: PersistedGovernance) -> Self {
        Self {
            safety_tier: value.safety_tier,
            transparency_tier: value.transparency_tier,
            governance_tier: value.governance_tier,
        }
    }
}

fn changed_or_not_found(
    changed: usize,
    hook_id: &PromptHookId,
) -> Result<(), PromptHookApplicationError> {
    if changed == 0 {
        Err(PromptHookApplicationError::NotFound(
            hook_id.as_str().to_string(),
        ))
    } else {
        Ok(())
    }
}

fn bool_to_i64(value: bool) -> i64 {
    i64::from(value)
}

fn json_string<T: Serialize + ?Sized>(value: &T) -> Result<String, PromptHookApplicationError> {
    serde_json::to_string(value).map_err(invalid_data)
}

fn json_to_sql_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
}

fn app_error(error: crate::platform::database::DatabaseError) -> PromptHookApplicationError {
    match error {
        crate::platform::database::DatabaseError::Database(error) => repository_error(error),
        crate::platform::database::DatabaseError::Storage(message) => {
            PromptHookApplicationError::Repository(message)
        }
    }
}

fn repository_error(error: rusqlite::Error) -> PromptHookApplicationError {
    PromptHookApplicationError::Repository(error.to_string())
}

fn invalid_data(error: impl std::fmt::Display) -> PromptHookApplicationError {
    PromptHookApplicationError::Repository(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn repository() -> (TempDirectory, NativeDatabase, SqlitePromptHookRepository) {
        let directory = TempDirectory::new("prompt-hook-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        database.connection().expect("migrations");
        let repository = SqlitePromptHookRepository::new(database.clone());
        (directory, database, repository)
    }

    fn governance() -> PromptHookGovernance {
        PromptHookGovernance {
            safety_tier: "editable".to_string(),
            transparency_tier: "visible-by-default".to_string(),
            governance_tier: "human-gated".to_string(),
        }
    }

    fn record(value: &str) -> PromptHookRecord {
        PromptHookRecord {
            manifest: PromptHookManifest::new(
                value,
                "Fixture Hook",
                PromptHookCategory::Dynamic,
                PromptHookStage::PerTurn,
                450,
                "Fixture {{agentId}}",
                &["codex-cli".to_string()],
            )
            .expect("manifest"),
            description: "Fixture description".to_string(),
            version: 2,
            source: PromptHookSource::User,
            enabled: true,
            disableable: true,
            governance: governance(),
            created_at: "2026-07-17T00:00:00Z".to_string(),
            updated_at: "2026-07-18T00:00:00Z".to_string(),
        }
    }

    fn trace(value: &str, hook_id: &str) -> PromptHookTrace {
        PromptHookTrace {
            id: value.to_string(),
            hook_id: PromptHookId::parse(hook_id).expect("hook id"),
            category: PromptHookCategory::Dynamic,
            stage: PromptHookStage::PerTurn,
            status: PromptHookTraceStatus::Fired,
            version: Some(2),
            content_hash: Some("fixture-hash".to_string()),
            token_estimate: Some(4),
            reason: None,
            agent_id: Some(ManagedCliAgentId::CodexCli),
            session_id: Some("session-1".to_string()),
            created_at: "2026-07-18T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn repository_round_trip_preserves_user_override_and_trace_contracts() {
        let (_directory, _database, repository) = repository();
        let fixture = record("fixture-hook");
        repository
            .create_user_hook(&fixture)
            .expect("create user hook");
        repository
            .save_builtin_override(&PromptHookOverride {
                hook_id: PromptHookId::parse("dynamic-session-config").expect("hook id"),
                enabled: false,
                bindings: PromptHookBindings::new(&["gemini-cli".to_string()]).expect("bindings"),
                updated_at: "2026-07-18T01:00:00Z".to_string(),
            })
            .expect("override");
        let fixture_trace = trace("trace-1", "fixture-hook");
        repository
            .save_traces(std::slice::from_ref(&fixture_trace), 50)
            .expect("traces");

        assert_eq!(repository.list_user_hooks().expect("users"), [fixture]);
        assert_eq!(
            repository
                .list_builtin_overrides()
                .expect("overrides")
                .len(),
            1
        );
        assert_eq!(repository.list_traces(25).expect("traces"), [fixture_trace]);
    }

    #[test]
    fn duplicate_create_does_not_overwrite_the_existing_user_hook() {
        let (_directory, _database, repository) = repository();
        let fixture = record("fixture-hook");
        repository.create_user_hook(&fixture).expect("first create");
        let mut replacement = fixture.clone();
        replacement.description = "replacement".to_string();

        let error = repository
            .create_user_hook(&replacement)
            .expect_err("duplicate create");

        assert_eq!(
            error,
            PromptHookApplicationError::Conflict("fixture-hook".to_string())
        );
        assert_eq!(repository.list_user_hooks().expect("users"), [fixture]);
    }

    #[test]
    fn delete_rolls_back_user_hook_when_related_override_removal_fails() {
        let (_directory, database, repository) = repository();
        let fixture = record("fixture-hook");
        repository
            .create_user_hook(&fixture)
            .expect("create user hook");
        repository
            .save_builtin_override(&PromptHookOverride {
                hook_id: fixture.id().clone(),
                enabled: false,
                bindings: PromptHookBindings::default(),
                updated_at: "2026-07-18T01:00:00Z".to_string(),
            })
            .expect("related override");
        database
            .connection()
            .expect("connection")
            .execute_batch(
                r#"
                CREATE TRIGGER fail_prompt_hook_override_delete
                BEFORE DELETE ON prompt_hook_overrides
                WHEN OLD.hook_id = 'fixture-hook'
                BEGIN
                    SELECT RAISE(ABORT, 'forced override delete failure');
                END;
                "#,
            )
            .expect("failure trigger");

        let error = repository
            .delete_user_hook(fixture.id())
            .expect_err("atomic delete failure");

        assert!(error.to_string().contains("forced override delete failure"));
        assert_eq!(repository.list_user_hooks().expect("users"), [fixture]);
        assert_eq!(
            repository
                .list_builtin_overrides()
                .expect("overrides")
                .len(),
            1
        );
    }

    #[test]
    fn trace_batch_and_retention_are_atomic_when_an_insert_fails() {
        let (_directory, database, repository) = repository();
        let connection = database.connection().expect("connection");
        connection
            .execute_batch(
                r#"
                CREATE TRIGGER fail_prompt_hook_trace
                BEFORE INSERT ON prompt_hook_traces
                WHEN NEW.hook_id = 'failing-hook'
                BEGIN
                    SELECT RAISE(ABORT, 'forced trace failure');
                END;
                "#,
            )
            .expect("failure trigger");

        let error = repository
            .save_traces(
                &[
                    trace("trace-1", "fixture-hook"),
                    trace("trace-2", "failing-hook"),
                ],
                1,
            )
            .expect_err("atomic trace failure");

        assert!(error.to_string().contains("forced trace failure"));
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM prompt_hook_traces", [], |row| {
                row.get(0)
            })
            .expect("trace count");
        assert_eq!(count, 0);
    }

    #[test]
    fn representative_legacy_rows_are_mapped_through_domain_invariants() {
        let (_directory, database, repository) = repository();
        let connection = database.connection().expect("connection");
        connection
            .execute(
                r#"
                INSERT INTO prompt_hooks_user (
                    id, name, description, category, stage, hook_order, version, enabled,
                    disableable, cli_bindings, governance, template_body, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                "#,
                params![
                    "legacy-hook",
                    "Legacy Hook",
                    "Legacy description",
                    "routing",
                    "session-init",
                    725,
                    3,
                    1,
                    1,
                    "[\"opencode\"]",
                    "{\"safetyTier\":\"readonly\",\"transparencyTier\":\"opt-in-view\",\"governanceTier\":\"human-gated\"}",
                    "Legacy {{sampleInput}}",
                    "2026-07-17T00:00:00Z",
                    "2026-07-18T00:00:00Z",
                ],
            )
            .expect("legacy row");

        let hooks = repository.list_user_hooks().expect("legacy hooks");

        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].id().as_str(), "legacy-hook");
        assert_eq!(hooks[0].manifest.category(), PromptHookCategory::Routing);
        assert_eq!(hooks[0].manifest.stage(), PromptHookStage::SessionInit);
        assert_eq!(hooks[0].manifest.bindings().to_strings(), ["opencode"]);
    }
}
