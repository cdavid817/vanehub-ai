use crate::contexts::tooling::extensions::application::{
    ExtensionApplicationError, ExtensionRepository, InstalledExtension,
};
use crate::contexts::tooling::extensions::domain::{
    definitions, EnablementPlan, ExtensionCapabilityId, ExtensionFrameworkId,
    ExtensionFrameworkState, ExtensionLifecycleStatus, ExtensionRuntimeObservation,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection};

#[derive(Clone)]
pub(crate) struct SqliteExtensionRepository {
    database: NativeDatabase,
}

impl SqliteExtensionRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<Connection, ExtensionApplicationError> {
        self.database.connection().map_err(repository_error)
    }
}

impl ExtensionRepository for SqliteExtensionRepository {
    fn list_states(&self) -> Result<Vec<ExtensionFrameworkState>, ExtensionApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT framework_id, capability_id, lifecycle_status, installed, enabled, port, \
                 install_path, installed_version, last_health_check, last_error, last_operation_id \
                 FROM extension_framework_state ORDER BY rowid",
            )
            .map_err(repository_error)?;
        let states = statement
            .query_map([], read_state)
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        Ok(states)
    }

    fn record_transition(
        &self,
        framework_id: ExtensionFrameworkId,
        status: ExtensionLifecycleStatus,
        operation_id: &str,
        at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        update_one(
            &self.connection()?,
            "UPDATE extension_framework_state SET lifecycle_status = ?1, last_operation_id = ?2, \
             last_error = NULL, updated_at = ?3 WHERE framework_id = ?4",
            params![status.as_str(), operation_id, at, framework_id.as_str()],
        )
    }

    fn record_installation(
        &self,
        framework_id: ExtensionFrameworkId,
        installed: &InstalledExtension,
        at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        update_one(
            &self.connection()?,
            "UPDATE extension_framework_state SET lifecycle_status = 'installed', installed = 1, \
             enabled = 0, install_path = ?1, installed_version = ?2, last_error = NULL, \
             updated_at = ?3 WHERE framework_id = ?4",
            params![
                installed.install_path,
                installed.installed_version,
                at,
                framework_id.as_str()
            ],
        )
    }

    fn record_removal(
        &self,
        framework_id: ExtensionFrameworkId,
        at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        update_one(
            &self.connection()?,
            "UPDATE extension_framework_state SET lifecycle_status = 'not-installed', installed = 0, \
             enabled = 0, install_path = NULL, installed_version = NULL, last_health_check = NULL, \
             last_error = NULL, updated_at = ?1 WHERE framework_id = ?2",
            params![at, framework_id.as_str()],
        )
    }

    fn apply_enablement(
        &self,
        plan: &EnablementPlan,
        at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        let connection = self.connection()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(repository_error)?;
        if plan.disable_capability_peers {
            transaction
                .execute(
                    "UPDATE extension_framework_state SET enabled = 0 WHERE capability_id = ?1",
                    [plan.capability_id.as_str()],
                )
                .map_err(repository_error)?;
        }
        let changed = transaction
            .execute(
                "UPDATE extension_framework_state SET enabled = ?1, updated_at = ?2 \
                 WHERE framework_id = ?3",
                params![i64::from(plan.enabled), at, plan.framework_id.as_str()],
            )
            .map_err(repository_error)?;
        if changed != 1 {
            return Err(missing_state(plan.framework_id));
        }
        transaction.commit().map_err(repository_error)
    }

    fn record_runtime_observation(
        &self,
        framework_id: ExtensionFrameworkId,
        observation: &ExtensionRuntimeObservation,
        checked_at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        let status = if observation.owned_process_running {
            ExtensionLifecycleStatus::Running
        } else {
            ExtensionLifecycleStatus::Installed
        };
        update_one(
            &self.connection()?,
            "UPDATE extension_framework_state SET lifecycle_status = ?1, last_health_check = ?2, \
             last_error = ?3, updated_at = ?2 WHERE framework_id = ?4",
            params![
                status.as_str(),
                checked_at,
                observation.error,
                framework_id.as_str()
            ],
        )
    }

    fn record_self_test(
        &self,
        framework_id: ExtensionFrameworkId,
        checked_at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        update_one(
            &self.connection()?,
            "UPDATE extension_framework_state SET last_health_check = ?1, last_error = NULL, \
             updated_at = ?1 WHERE framework_id = ?2",
            params![checked_at, framework_id.as_str()],
        )
    }

    fn record_failure(
        &self,
        framework_id: ExtensionFrameworkId,
        error: &str,
        at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        update_one(
            &self.connection()?,
            "UPDATE extension_framework_state SET lifecycle_status = 'error', last_error = ?1, \
             updated_at = ?2 WHERE framework_id = ?3",
            params![error, at, framework_id.as_str()],
        )
    }
}

pub(crate) fn apply_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS extension_framework_state (
            framework_id TEXT PRIMARY KEY,
            capability_id TEXT NOT NULL,
            lifecycle_status TEXT NOT NULL DEFAULT 'not-installed',
            installed INTEGER NOT NULL DEFAULT 0,
            enabled INTEGER NOT NULL DEFAULT 0,
            port INTEGER NOT NULL,
            install_path TEXT,
            installed_version TEXT,
            last_health_check TEXT,
            last_error TEXT,
            last_operation_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    for definition in definitions() {
        let now = chrono::Utc::now().to_rfc3339();
        connection.execute(
            "INSERT OR IGNORE INTO extension_framework_state \
             (framework_id, capability_id, lifecycle_status, installed, enabled, port, created_at, updated_at) \
             VALUES (?1, ?2, 'not-installed', 0, 0, ?3, ?4, ?4)",
            params![
                definition.id.as_str(),
                definition.capability_id.as_str(),
                i64::from(definition.default_port),
                now
            ],
        )?;
    }
    Ok(())
}

fn read_state(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExtensionFrameworkState> {
    let framework_id = row.get::<_, String>(0)?;
    let capability_id = row.get::<_, String>(1)?;
    Ok(ExtensionFrameworkState {
        framework_id: parse_framework_id(&framework_id, 0)?,
        capability_id: parse_capability_id(&capability_id, 1)?,
        status: ExtensionLifecycleStatus::parse(&row.get::<_, String>(2)?),
        installed: row.get::<_, i64>(3)? != 0,
        enabled: row.get::<_, i64>(4)? != 0,
        port: row.get(5)?,
        install_path: row.get(6)?,
        installed_version: row.get(7)?,
        last_health_check: row.get(8)?,
        last_error: row.get(9)?,
        last_operation_id: row.get(10)?,
    })
}

fn parse_framework_id(value: &str, column: usize) -> rusqlite::Result<ExtensionFrameworkId> {
    ExtensionFrameworkId::parse(value)
        .ok_or_else(|| invalid_text_column(column, value, "extension framework id"))
}

fn parse_capability_id(value: &str, column: usize) -> rusqlite::Result<ExtensionCapabilityId> {
    ExtensionCapabilityId::parse(value)
        .ok_or_else(|| invalid_text_column(column, value, "extension capability id"))
}

fn invalid_text_column(column: usize, value: &str, kind: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        rusqlite::types::Type::Text,
        format!("unknown {kind}: {value}").into(),
    )
}

fn update_one(
    connection: &Connection,
    sql: &str,
    parameters: impl rusqlite::Params,
) -> Result<(), ExtensionApplicationError> {
    let changed = connection
        .execute(sql, parameters)
        .map_err(repository_error)?;
    if changed == 1 {
        Ok(())
    } else {
        Err(ExtensionApplicationError::Repository(
            "extension framework state is missing".to_string(),
        ))
    }
}

fn missing_state(framework_id: ExtensionFrameworkId) -> ExtensionApplicationError {
    ExtensionApplicationError::Repository(format!(
        "extension framework state is missing for {}",
        framework_id.as_str()
    ))
}

fn repository_error(error: impl std::fmt::Display) -> ExtensionApplicationError {
    ExtensionApplicationError::Repository(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn repository(name: &str) -> (TempDirectory, SqliteExtensionRepository) {
        let directory = TempDirectory::new(name);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteExtensionRepository::new(database);
        (directory, repository)
    }

    #[test]
    fn schema_and_state_mapping_preserve_existing_rows() {
        let (_directory, repository) = repository("extension-sqlite-round-trip");
        let before = repository.list_states().expect("states");
        assert_eq!(before.len(), 3);
        assert_eq!(before[0].framework_id, ExtensionFrameworkId::Paddleocr);
        assert_eq!(before[0].port, 9875);

        repository
            .record_installation(
                ExtensionFrameworkId::Paddleocr,
                &InstalledExtension {
                    install_path: "C:/managed/paddleocr".to_string(),
                    installed_version: "3.2.0".to_string(),
                },
                "installed-at",
            )
            .expect("installed");
        let installed = repository
            .list_states()
            .expect("states")
            .into_iter()
            .find(|state| state.framework_id == ExtensionFrameworkId::Paddleocr)
            .expect("paddleocr");
        assert!(installed.installed);
        assert!(!installed.enabled);
        assert_eq!(installed.installed_version.as_deref(), Some("3.2.0"));
    }

    #[test]
    fn enablement_and_removal_are_atomic_and_capability_scoped() {
        let (_directory, repository) = repository("extension-sqlite-lifecycle");
        repository
            .record_installation(
                ExtensionFrameworkId::FasterWhisper,
                &InstalledExtension {
                    install_path: "C:/managed/faster-whisper".to_string(),
                    installed_version: "1.1.0".to_string(),
                },
                "installed-at",
            )
            .expect("installed");
        repository
            .apply_enablement(
                &EnablementPlan {
                    framework_id: ExtensionFrameworkId::FasterWhisper,
                    capability_id: ExtensionCapabilityId::Asr,
                    enabled: true,
                    disable_capability_peers: true,
                },
                "enabled-at",
            )
            .expect("enabled");
        assert!(
            repository
                .list_states()
                .expect("states")
                .into_iter()
                .find(|state| state.framework_id == ExtensionFrameworkId::FasterWhisper)
                .expect("state")
                .enabled
        );

        repository
            .record_removal(ExtensionFrameworkId::FasterWhisper, "removed-at")
            .expect("removed");
        let removed = repository
            .list_states()
            .expect("states")
            .into_iter()
            .find(|state| state.framework_id == ExtensionFrameworkId::FasterWhisper)
            .expect("state");
        assert!(!removed.installed);
        assert!(!removed.enabled);
        assert!(removed.install_path.is_none());
    }
}
