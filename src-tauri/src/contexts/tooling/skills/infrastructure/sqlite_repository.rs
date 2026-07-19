use crate::contexts::tooling::skills::application::{
    AgentMountConfiguration, ManagedSkillSource, SkillAgentBinding, SkillApplicationError,
    SkillDriftReport, SkillRecord, SkillRepository,
};
use crate::contexts::tooling::skills::domain::{
    default_mount_path, SkillDriftIssueType, SkillId, SkillKey, SkillLocation, SkillMetadata,
    SkillMountPath, SkillScope, SkillSource,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection, Row, Transaction};
use serde::{Deserialize, Serialize};

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
        let connection = self.database.connection().map_err(app_error)?;
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
        rows.into_iter()
            .map(|row| row.into_record(&connection))
            .collect()
    }

    fn get(&self, key: &SkillKey) -> Result<Option<SkillRecord>, SkillApplicationError> {
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
        let bindings = load_bindings(connection, &key)?;
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
            bindings,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::domain::{SkillDriftIssue, SkillDriftIssueType};
    use crate::test_support::TempDirectory;

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
            4
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
}
