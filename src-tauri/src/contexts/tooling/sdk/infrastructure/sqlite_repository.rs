use crate::contexts::tooling::sdk::application::{
    SdkApplicationError, SdkLogEvent, SdkOperationLog, SdkRepository,
};
use crate::contexts::tooling::sdk::domain::{SdkDefinition, SdkId, SdkOperationType};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub(crate) struct SqliteSdkRepository {
    database: NativeDatabase,
    dependencies_root: PathBuf,
}

impl SqliteSdkRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self {
            database,
            dependencies_root: dependencies_root(),
        }
    }

    #[cfg(test)]
    fn with_root(database: NativeDatabase, dependencies_root: PathBuf) -> Self {
        Self {
            database,
            dependencies_root,
        }
    }
}

impl SdkRepository for SqliteSdkRepository {
    fn installed_version(
        &self,
        definition: SdkDefinition,
    ) -> Result<Option<String>, SdkApplicationError> {
        let package_json = package_dir(&self.dependencies_root, definition).join("package.json");
        let raw = match std::fs::read_to_string(package_json) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(repository_error(error)),
        };
        let value = serde_json::from_str::<Value>(&raw).map_err(repository_error)?;
        Ok(value
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_string))
    }

    fn install_path(&self, sdk_id: SdkId) -> Result<String, SdkApplicationError> {
        Ok(sdk_dir(&self.dependencies_root, sdk_id)
            .to_string_lossy()
            .to_string())
    }

    fn operation_logs(
        &self,
        sdk_id: Option<SdkId>,
    ) -> Result<Vec<SdkOperationLog>, SdkApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        let (sql, parameter) = match sdk_id {
            Some(sdk_id) => (
                "SELECT sdk_id, operation, line, timestamp FROM sdk_operation_logs WHERE sdk_id = ?1 ORDER BY id",
                Some(sdk_id.as_str()),
            ),
            None => (
                "SELECT sdk_id, operation, line, timestamp FROM sdk_operation_logs ORDER BY id",
                None,
            ),
        };
        let mut statement = connection.prepare(sql).map_err(repository_error)?;
        let rows = match parameter {
            Some(parameter) => statement.query_map([parameter], read_log),
            None => statement.query_map([], read_log),
        }
        .map_err(repository_error)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)
    }

    fn append_operation_log(&self, event: &SdkLogEvent) -> Result<(), SdkApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        connection
            .execute(
                "INSERT INTO sdk_operation_logs (operation_id, sdk_id, operation, level, line, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    event.operation_id,
                    event.sdk_id.as_str(),
                    operation_str(event.operation),
                    log_level_str(event.level),
                    event.line,
                    event.timestamp,
                ],
            )
            .map(|_| ())
            .map_err(repository_error)
    }
}

pub(crate) fn apply_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS sdk_operation_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            operation_id TEXT NOT NULL,
            sdk_id TEXT NOT NULL,
            operation TEXT NOT NULL,
            level TEXT NOT NULL,
            line TEXT NOT NULL,
            timestamp TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_sdk_operation_logs_sdk_id_id
            ON sdk_operation_logs(sdk_id, id);
        "#,
    )?;
    Ok(())
}

pub(super) fn dependencies_root() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vanehub")
        .join("dependencies")
}

pub(super) fn sdk_dir(root: &Path, sdk_id: SdkId) -> PathBuf {
    root.join(sdk_id.as_str())
}

pub(super) fn package_dir(root: &Path, definition: SdkDefinition) -> PathBuf {
    definition.npm_package.split('/').fold(
        sdk_dir(root, definition.id).join("node_modules"),
        |path, part| path.join(part),
    )
}

fn read_log(row: &rusqlite::Row<'_>) -> rusqlite::Result<SdkOperationLog> {
    let sdk_id = row.get::<_, String>(0)?;
    let operation = row.get::<_, String>(1)?;
    Ok(SdkOperationLog {
        sdk_id: parse_sdk_id(&sdk_id, 0)?,
        operation: parse_operation(&operation, 1)?,
        line: row.get(2)?,
        timestamp: row.get(3)?,
    })
}

fn parse_sdk_id(value: &str, column: usize) -> rusqlite::Result<SdkId> {
    SdkId::parse(value).ok_or_else(|| invalid_text_column(column, value, "SDK id"))
}

fn parse_operation(value: &str, column: usize) -> rusqlite::Result<SdkOperationType> {
    match value {
        "install" => Ok(SdkOperationType::Install),
        "update" => Ok(SdkOperationType::Update),
        "rollback" => Ok(SdkOperationType::Rollback),
        "uninstall" => Ok(SdkOperationType::Uninstall),
        _ => Err(invalid_text_column(column, value, "SDK operation")),
    }
}

fn invalid_text_column(column: usize, value: &str, kind: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column,
        rusqlite::types::Type::Text,
        format!("unknown {kind}: {value}").into(),
    )
}

fn operation_str(operation: SdkOperationType) -> &'static str {
    match operation {
        SdkOperationType::Install => "install",
        SdkOperationType::Update => "update",
        SdkOperationType::Rollback => "rollback",
        SdkOperationType::Uninstall => "uninstall",
    }
}

fn log_level_str(level: crate::contexts::tooling::sdk::application::SdkLogLevel) -> &'static str {
    use crate::contexts::tooling::sdk::application::SdkLogLevel;
    match level {
        SdkLogLevel::Error => "error",
        SdkLogLevel::Warn => "warn",
        SdkLogLevel::Info => "info",
        SdkLogLevel::Debug => "debug",
    }
}

fn repository_error(error: impl std::fmt::Display) -> SdkApplicationError {
    SdkApplicationError::Repository(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::sdk::application::{SdkLogLevel, SdkRepository};
    use crate::contexts::tooling::sdk::domain::definition;
    use crate::test_support::TempDirectory;
    use std::collections::BTreeMap;

    #[test]
    fn sqlite_logs_round_trip_and_filter_while_package_json_remains_authoritative() {
        let data = TempDirectory::new("sdk-sqlite-repository-data");
        let dependencies = TempDirectory::new("sdk-sqlite-repository-dependencies");
        let database = NativeDatabase::new(data.path().to_path_buf()).expect("database");
        let repository =
            SqliteSdkRepository::with_root(database, dependencies.path().to_path_buf());
        let claude = definition(SdkId::ClaudeSdk);
        let package = package_dir(dependencies.path(), claude);
        std::fs::create_dir_all(&package).expect("package directory");
        std::fs::write(package.join("package.json"), r#"{"version":"0.2.81"}"#)
            .expect("package json");

        assert_eq!(
            repository
                .installed_version(claude)
                .expect("installed")
                .as_deref(),
            Some("0.2.81")
        );
        repository
            .append_operation_log(&SdkLogEvent {
                operation_id: "sdk-op-1".to_string(),
                sdk_id: SdkId::ClaudeSdk,
                operation: SdkOperationType::Rollback,
                level: SdkLogLevel::Info,
                line: "rolled back".to_string(),
                timestamp: "now".to_string(),
                context: BTreeMap::new(),
            })
            .expect("append log");

        let logs = repository
            .operation_logs(Some(SdkId::ClaudeSdk))
            .expect("logs");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].operation, SdkOperationType::Rollback);
        assert!(repository
            .operation_logs(Some(SdkId::CodexSdk))
            .expect("filtered logs")
            .is_empty());
    }

    #[test]
    fn missing_or_malformed_installation_state_is_bounded() {
        let data = TempDirectory::new("sdk-sqlite-repository-invalid-data");
        let dependencies = TempDirectory::new("sdk-sqlite-repository-invalid-dependencies");
        let repository = SqliteSdkRepository::with_root(
            NativeDatabase::new(data.path().to_path_buf()).expect("database"),
            dependencies.path().to_path_buf(),
        );
        let codex = definition(SdkId::CodexSdk);
        assert_eq!(repository.installed_version(codex).expect("missing"), None);

        let package = package_dir(dependencies.path(), codex);
        std::fs::create_dir_all(&package).expect("package directory");
        std::fs::write(package.join("package.json"), "not-json").expect("invalid json");
        assert!(matches!(
            repository.installed_version(codex),
            Err(SdkApplicationError::Repository(_))
        ));
    }
}
