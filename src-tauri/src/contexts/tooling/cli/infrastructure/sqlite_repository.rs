use super::support::{current_environment_type, install_command_for, is_direct_cli_executable};
use crate::contexts::tooling::cli::application::{
    CliApplicationError, CliStatusRepository, CliToolStatus,
};
use crate::contexts::tooling::cli::domain::{
    derive_conflict_state, derive_lifecycle_eligibility, ConflictState, EnvironmentType,
    InstallSource, Installation, LifecycleEligibility, ToolDefinition, VersionCheckStatus,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone)]
pub(crate) struct SqliteCliStatusRepository {
    database: NativeDatabase,
}

impl SqliteCliStatusRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl CliStatusRepository for SqliteCliStatusRepository {
    fn load(&self, definition: ToolDefinition) -> Result<CliToolStatus, CliApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let row = connection
            .query_row(
                r#"
                SELECT installed, current_version, latest_version, available_versions,
                       detected_path, last_checked_at, last_error, last_operation_id,
                       version_check_status, environment_type, installations,
                       active_installation_path, conflict_state, lifecycle_eligibility
                FROM cli_tool_status WHERE agent_id = ?1
                "#,
                params![definition.agent_id],
                PersistedCliStatusRow::read,
            )
            .optional()
            .map_err(database_error)?;
        Ok(status_from_row(definition, row))
    }

    fn save(&self, status: &CliToolStatus) -> Result<(), CliApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let available_versions = serde_json::to_string(&status.available_versions)
            .map_err(|error| CliApplicationError::Validation(error.to_string()))?;
        let installations = status
            .installations
            .iter()
            .map(PersistedInstallation::from)
            .collect::<Vec<_>>();
        let installations = serde_json::to_string(&installations)
            .map_err(|error| CliApplicationError::Validation(error.to_string()))?;
        connection
            .execute(
                r#"
                INSERT INTO cli_tool_status (
                    agent_id, installed, current_version, latest_version, available_versions,
                    detected_path, last_checked_at, last_error, last_operation_id,
                    version_check_status, environment_type, installations,
                    active_installation_path, conflict_state, lifecycle_eligibility
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
                ON CONFLICT(agent_id) DO UPDATE SET
                    installed = excluded.installed,
                    current_version = excluded.current_version,
                    latest_version = excluded.latest_version,
                    available_versions = excluded.available_versions,
                    detected_path = excluded.detected_path,
                    last_checked_at = excluded.last_checked_at,
                    last_error = excluded.last_error,
                    last_operation_id = excluded.last_operation_id,
                    version_check_status = excluded.version_check_status,
                    environment_type = excluded.environment_type,
                    installations = excluded.installations,
                    active_installation_path = excluded.active_installation_path,
                    conflict_state = excluded.conflict_state,
                    lifecycle_eligibility = excluded.lifecycle_eligibility
                "#,
                params![
                    status.agent_id,
                    status.installed.map(i32::from),
                    status.current_version,
                    status.latest_version,
                    available_versions,
                    status.detected_path,
                    status.last_checked_at,
                    status.last_error,
                    status.last_operation_id,
                    version_check_status_str(status.version_check_status),
                    environment_type_str(status.environment_type),
                    installations,
                    status.active_installation_path,
                    conflict_state_str(status.conflict_state),
                    lifecycle_eligibility_str(status.lifecycle_eligibility),
                ],
            )
            .map_err(database_error)?;
        Ok(())
    }

    fn has_cached_statuses(&self) -> Result<bool, CliApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .query_row("SELECT COUNT(*) FROM cli_tool_status", [], |row| {
                row.get::<_, i64>(0)
            })
            .map(|count| count > 0)
            .map_err(database_error)
    }
}

struct PersistedCliStatusRow {
    installed: Option<i64>,
    current_version: Option<String>,
    latest_version: Option<String>,
    available_versions: String,
    detected_path: Option<String>,
    last_checked_at: Option<String>,
    last_error: Option<String>,
    last_operation_id: Option<String>,
    version_check_status: String,
    environment_type: String,
    installations: String,
    active_installation_path: Option<String>,
    conflict_state: String,
    lifecycle_eligibility: String,
}

impl PersistedCliStatusRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            installed: row.get(0)?,
            current_version: row.get(1)?,
            latest_version: row.get(2)?,
            available_versions: row.get(3)?,
            detected_path: row.get(4)?,
            last_checked_at: row.get(5)?,
            last_error: row.get(6)?,
            last_operation_id: row.get(7)?,
            version_check_status: row.get(8)?,
            environment_type: row.get(9)?,
            installations: row.get(10)?,
            active_installation_path: row.get(11)?,
            conflict_state: row.get(12)?,
            lifecycle_eligibility: row.get(13)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedInstallation {
    path: String,
    version: Option<String>,
    runnable: bool,
    error: Option<String>,
    source: String,
    environment_type: String,
    is_active: bool,
}

impl From<&Installation> for PersistedInstallation {
    fn from(installation: &Installation) -> Self {
        Self {
            path: installation.path.clone(),
            version: installation.version.clone(),
            runnable: installation.runnable,
            error: installation.error.clone(),
            source: install_source_str(installation.source).to_string(),
            environment_type: environment_type_str(installation.environment_type).to_string(),
            is_active: installation.is_active,
        }
    }
}

impl From<PersistedInstallation> for Installation {
    fn from(installation: PersistedInstallation) -> Self {
        Self {
            path: installation.path,
            version: installation.version,
            runnable: installation.runnable,
            error: installation.error,
            source: parse_install_source(&installation.source),
            environment_type: parse_environment_type(&installation.environment_type),
            is_active: installation.is_active,
        }
    }
}

fn status_from_row(
    definition: ToolDefinition,
    row: Option<PersistedCliStatusRow>,
) -> CliToolStatus {
    let Some(row) = row else {
        return CliToolStatus::unavailable(
            definition,
            current_environment_type(),
            install_command_for(definition),
        );
    };
    let installations = serde_json::from_str::<Vec<PersistedInstallation>>(&row.installations)
        .unwrap_or_default()
        .into_iter()
        .map(Installation::from)
        .collect();
    let status = CliToolStatus {
        agent_id: definition.agent_id.to_string(),
        display_name: definition.display_name.to_string(),
        provider: definition.provider.to_string(),
        executable_name: definition.executable_name.to_string(),
        package_name: definition.package_name.to_string(),
        installed: row.installed.map(|value| value != 0),
        current_version: row.current_version,
        latest_version: row.latest_version,
        available_versions: serde_json::from_str(&row.available_versions).unwrap_or_default(),
        detected_path: row.detected_path,
        install_command: install_command_for(definition),
        last_checked_at: row.last_checked_at,
        last_error: row.last_error,
        last_operation_id: row.last_operation_id,
        version_check_status: parse_version_check_status(&row.version_check_status),
        environment_type: parse_environment_type(&row.environment_type),
        installations,
        active_installation_path: row.active_installation_path,
        conflict_state: parse_conflict_state(&row.conflict_state),
        lifecycle_eligibility: parse_lifecycle_eligibility(&row.lifecycle_eligibility),
    };
    sanitize_cached_status(definition, status)
}

fn sanitize_cached_status(definition: ToolDefinition, mut status: CliToolStatus) -> CliToolStatus {
    if !cfg!(target_os = "windows") {
        return status;
    }
    let before_count = status.installations.len();
    status
        .installations
        .retain(|installation| is_direct_cli_executable(Path::new(&installation.path)));
    if status
        .installations
        .iter()
        .filter(|installation| installation.is_active)
        .count()
        != 1
        || status.installations.len() != before_count
    {
        for (index, installation) in status.installations.iter_mut().enumerate() {
            installation.is_active = index == 0;
        }
    }
    let active = status
        .installations
        .iter()
        .find(|installation| installation.is_active);
    let installed = !status.installations.is_empty();
    status.installed = Some(installed);
    status.detected_path = active.map(|installation| installation.path.clone());
    status.active_installation_path = status.detected_path.clone();
    status.current_version = active.and_then(|installation| installation.version.clone());
    status.conflict_state = derive_conflict_state(&status.installations);
    status.lifecycle_eligibility = derive_lifecycle_eligibility(definition, installed, active);
    if status
        .last_error
        .as_deref()
        .is_some_and(is_stale_windows_direct_launch_error)
    {
        status.last_error = None;
        status.last_operation_id = None;
        status.version_check_status = if installed {
            VersionCheckStatus::Succeeded
        } else {
            VersionCheckStatus::NotDetected
        };
    }
    status
}

fn is_stale_windows_direct_launch_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    error.contains("不是有效的 Win32 应用程序")
        || lower.contains("not a valid win32 application")
        || lower.contains("os error 193")
        || lower.contains("command timed out")
}

fn parse_version_check_status(value: &str) -> VersionCheckStatus {
    match value {
        "succeeded" => VersionCheckStatus::Succeeded,
        "failed" => VersionCheckStatus::Failed,
        "unsupported" => VersionCheckStatus::Unsupported,
        _ => VersionCheckStatus::NotDetected,
    }
}

fn version_check_status_str(value: VersionCheckStatus) -> &'static str {
    match value {
        VersionCheckStatus::Unsupported => "unsupported",
        VersionCheckStatus::NotDetected => "not-detected",
        VersionCheckStatus::Succeeded => "succeeded",
        VersionCheckStatus::Failed => "failed",
    }
}

fn parse_environment_type(value: &str) -> EnvironmentType {
    match value {
        "windows" => EnvironmentType::Windows,
        "macos" => EnvironmentType::Macos,
        "linux" => EnvironmentType::Linux,
        _ => EnvironmentType::Unknown,
    }
}

fn environment_type_str(value: EnvironmentType) -> &'static str {
    match value {
        EnvironmentType::Windows => "windows",
        EnvironmentType::Macos => "macos",
        EnvironmentType::Linux => "linux",
        EnvironmentType::Unknown => "unknown",
    }
}

fn parse_install_source(value: &str) -> InstallSource {
    match value {
        "npm" => InstallSource::Npm,
        "winget" => InstallSource::Winget,
        "desktop" => InstallSource::Desktop,
        "homebrew" => InstallSource::Homebrew,
        "volta" => InstallSource::Volta,
        "bun" => InstallSource::Bun,
        "vendor" => InstallSource::Vendor,
        "system" => InstallSource::System,
        _ => InstallSource::Unknown,
    }
}

fn install_source_str(value: InstallSource) -> &'static str {
    match value {
        InstallSource::Npm => "npm",
        InstallSource::Winget => "winget",
        InstallSource::Desktop => "desktop",
        InstallSource::Homebrew => "homebrew",
        InstallSource::Volta => "volta",
        InstallSource::Bun => "bun",
        InstallSource::Vendor => "vendor",
        InstallSource::System => "system",
        InstallSource::Unknown => "unknown",
    }
}

fn parse_conflict_state(value: &str) -> ConflictState {
    match value {
        "multiple" => ConflictState::Multiple,
        "version-mismatch" => ConflictState::VersionMismatch,
        "runnable-mismatch" => ConflictState::RunnableMismatch,
        _ => ConflictState::None,
    }
}

fn conflict_state_str(value: ConflictState) -> &'static str {
    match value {
        ConflictState::None => "none",
        ConflictState::Multiple => "multiple",
        ConflictState::VersionMismatch => "version-mismatch",
        ConflictState::RunnableMismatch => "runnable-mismatch",
    }
}

fn parse_lifecycle_eligibility(value: &str) -> LifecycleEligibility {
    match value {
        "npm" => LifecycleEligibility::Npm,
        "wget" => LifecycleEligibility::Wget,
        "winget" => LifecycleEligibility::Winget,
        "manual" => LifecycleEligibility::Manual,
        _ => LifecycleEligibility::Unavailable,
    }
}

fn lifecycle_eligibility_str(value: LifecycleEligibility) -> &'static str {
    match value {
        LifecycleEligibility::Npm => "npm",
        LifecycleEligibility::Wget => "wget",
        LifecycleEligibility::Winget => "winget",
        LifecycleEligibility::Manual => "manual",
        LifecycleEligibility::Unavailable => "unavailable",
    }
}

fn app_error(error: crate::platform::database::DatabaseError) -> CliApplicationError {
    match error {
        crate::platform::database::DatabaseError::Database(error) => {
            CliApplicationError::Database(error.to_string())
        }
        crate::platform::database::DatabaseError::Storage(message) => {
            CliApplicationError::Storage(message)
        }
    }
}

fn database_error(error: rusqlite::Error) -> CliApplicationError {
    CliApplicationError::Database(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli::domain::{definition, CLI_TOOL_DEFINITIONS};
    use crate::test_support::TempDirectory;

    fn repository(name: &str) -> (TempDirectory, SqliteCliStatusRepository) {
        let directory = TempDirectory::new(name);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        (directory, SqliteCliStatusRepository::new(database))
    }

    fn installed_status() -> CliToolStatus {
        let definition = definition("codex-cli").expect("definition");
        let path = "C:\\Users\\dev\\AppData\\Roaming\\npm\\codex.cmd".to_string();
        let mut status = CliToolStatus::unavailable(
            definition,
            EnvironmentType::Windows,
            install_command_for(definition),
        );
        status.installed = Some(true);
        status.current_version = Some("1.2.3".to_string());
        status.latest_version = Some("1.3.0".to_string());
        status.available_versions = vec!["1.3.0".to_string(), "1.2.3".to_string()];
        status.detected_path = Some(path.clone());
        status.last_checked_at = Some("123".to_string());
        status.version_check_status = VersionCheckStatus::Succeeded;
        status.environment_type = EnvironmentType::Windows;
        status.installations = vec![Installation {
            path: path.clone(),
            version: Some("1.2.3".to_string()),
            runnable: true,
            error: None,
            source: InstallSource::Npm,
            environment_type: EnvironmentType::Windows,
            is_active: true,
        }];
        status.active_installation_path = Some(path);
        status.lifecycle_eligibility = LifecycleEligibility::Npm;
        status
    }

    #[test]
    fn empty_repository_returns_catalog_default_without_startup_cache() {
        let (_directory, repository) = repository("cli-sqlite-empty");

        let status = repository
            .load(CLI_TOOL_DEFINITIONS[0])
            .expect("default status");

        assert_eq!(status.agent_id, "claude-code");
        assert_eq!(status.installed, None);
        assert_eq!(status.version_check_status, VersionCheckStatus::NotDetected);
        assert!(!repository.has_cached_statuses().expect("cache state"));
    }

    #[test]
    fn sqlite_status_round_trip_preserves_storage_contract() {
        let (_directory, repository) = repository("cli-sqlite-round-trip");
        let status = installed_status();

        repository.save(&status).expect("save");
        let loaded = repository
            .load(definition("codex-cli").expect("definition"))
            .expect("load");

        assert!(repository.has_cached_statuses().expect("cache state"));
        assert_eq!(loaded.current_version.as_deref(), Some("1.2.3"));
        assert_eq!(loaded.available_versions, vec!["1.3.0", "1.2.3"]);
        assert_eq!(loaded.installations, status.installations);
        assert_eq!(loaded.lifecycle_eligibility, LifecycleEligibility::Npm);
        let connection = repository.database.connection().expect("connection");
        let installations: String = connection
            .query_row(
                "SELECT installations FROM cli_tool_status WHERE agent_id = 'codex-cli'",
                [],
                |row| row.get(0),
            )
            .expect("installations");
        assert!(installations.contains("\"environmentType\":\"windows\""));
        assert!(installations.contains("\"source\":\"npm\""));
    }

    #[test]
    fn legacy_unknown_values_and_malformed_json_use_documented_fallbacks() {
        let (_directory, repository) = repository("cli-sqlite-legacy");
        let connection = repository.database.connection().expect("connection");
        connection
            .execute(
                r#"
                INSERT INTO cli_tool_status (
                    agent_id, available_versions, version_check_status, environment_type,
                    installations, conflict_state, lifecycle_eligibility
                ) VALUES ('codex-cli', 'not-json', 'other', 'other', 'not-json', 'other', 'other')
                "#,
                [],
            )
            .expect("legacy row");

        let loaded = repository
            .load(definition("codex-cli").expect("definition"))
            .expect("load");

        assert!(loaded.available_versions.is_empty());
        assert!(loaded.installations.is_empty());
        assert_eq!(loaded.version_check_status, VersionCheckStatus::NotDetected);
        assert_eq!(loaded.environment_type, EnvironmentType::Unknown);
        assert_eq!(loaded.conflict_state, ConflictState::None);
        assert_eq!(
            loaded.lifecycle_eligibility,
            if cfg!(target_os = "windows") {
                LifecycleEligibility::Npm
            } else {
                LifecycleEligibility::Unavailable
            }
        );
    }
}
