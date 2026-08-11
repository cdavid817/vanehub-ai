use crate::contexts::code_intelligence::application::ports::{
    LspConfigurationRepository, WorkspaceTrustRepository,
};
use crate::contexts::code_intelligence::domain::configuration::{
    LanguageConfiguration, LspConfiguration,
};
use crate::contexts::code_intelligence::domain::models::{
    DomainModelError, LanguageFamily, WorkspaceTrust,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone)]
pub(crate) struct SqliteCodeIntelligenceRepository {
    database: NativeDatabase,
}

impl SqliteCodeIntelligenceRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl LspConfigurationRepository for SqliteCodeIntelligenceRepository {
    fn load_configuration(&self) -> Result<LspConfiguration, DomainModelError> {
        let connection = self.database.connection().map_err(|_| storage_error())?;
        load_configuration(&connection)
    }

    fn save_configuration(&self, configuration: &LspConfiguration) -> Result<(), DomainModelError> {
        configuration.validate()?;
        let mut connection = self.database.connection().map_err(|_| storage_error())?;
        let transaction = connection.transaction().map_err(|_| storage_error())?;
        let now = SystemClock.rfc3339();
        transaction
            .execute(
                "UPDATE lsp_configuration SET enabled = ?1, revision = revision + 1, \
                 updated_at = ?2 WHERE id = 1",
                params![configuration.enabled, now],
            )
            .map_err(|_| storage_error())?;
        for (language, language_configuration) in &configuration.languages {
            let arguments = serde_json::to_string(language.startup_arguments())
                .map_err(|_| DomainModelError::InvalidInitializationOptions)?;
            let options = serde_json::to_string(&language_configuration.initialization_options)
                .map_err(|_| DomainModelError::InvalidInitializationOptions)?;
            transaction
                .execute(
                    "INSERT INTO lsp_language_configurations (
                        language_id, enabled, executable_override, startup_arguments_json,
                        initialization_options_json, revision, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)
                     ON CONFLICT(language_id) DO UPDATE SET
                        enabled = excluded.enabled,
                        executable_override = excluded.executable_override,
                        startup_arguments_json = excluded.startup_arguments_json,
                        initialization_options_json = excluded.initialization_options_json,
                        revision = lsp_language_configurations.revision + 1,
                        updated_at = excluded.updated_at",
                    params![
                        language.as_id(),
                        language_configuration.enabled,
                        language_configuration.executable_override,
                        arguments,
                        options,
                        now
                    ],
                )
                .map_err(|_| storage_error())?;
        }
        transaction.commit().map_err(|_| storage_error())
    }
}

impl WorkspaceTrustRepository for SqliteCodeIntelligenceRepository {
    fn list_workspace_trust(&self) -> Result<Vec<WorkspaceTrust>, DomainModelError> {
        let connection = self.database.connection().map_err(|_| storage_error())?;
        let mut statement = connection
            .prepare(
                "SELECT canonical_workspace_root, trusted, revision \
                 FROM lsp_workspace_trust ORDER BY canonical_workspace_root",
            )
            .map_err(|_| storage_error())?;
        let records = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, bool>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|_| storage_error())?
            .map(|row| {
                let (root, trusted, revision) = row.map_err(|_| storage_error())?;
                let revision = u64::try_from(revision).map_err(|_| storage_error())?;
                WorkspaceTrust::new(root, trusted, revision)
            })
            .collect();
        records
    }

    fn set_workspace_trust(
        &self,
        workspace_root: &Path,
        trusted: bool,
    ) -> Result<WorkspaceTrust, DomainModelError> {
        if !workspace_root.is_absolute() {
            return Err(DomainModelError::InvalidWorkspaceRoot);
        }
        let canonical = std::fs::canonicalize(workspace_root)
            .map_err(|_| DomainModelError::InvalidWorkspaceRoot)?;
        if !canonical.is_dir() {
            return Err(DomainModelError::InvalidWorkspaceRoot);
        }
        let canonical = canonical.to_string_lossy().into_owned();
        let connection = self.database.connection().map_err(|_| storage_error())?;
        let now = SystemClock.rfc3339();
        connection
            .execute(
                "INSERT INTO lsp_workspace_trust (
                    canonical_workspace_root, trusted, revision, created_at, updated_at
                 ) VALUES (?1, ?2, 1, ?3, ?3)
                 ON CONFLICT(canonical_workspace_root) DO UPDATE SET
                    trusted = excluded.trusted,
                    revision = lsp_workspace_trust.revision + 1,
                    updated_at = excluded.updated_at",
                params![canonical, trusted, now],
            )
            .map_err(|_| storage_error())?;
        connection
            .query_row(
                "SELECT canonical_workspace_root, trusted, revision \
                 FROM lsp_workspace_trust WHERE canonical_workspace_root = ?1",
                params![canonical],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, bool>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .map_err(|_| storage_error())
            .and_then(|(root, value, revision)| {
                let revision = u64::try_from(revision).map_err(|_| storage_error())?;
                WorkspaceTrust::new(root, value, revision)
            })
    }
}

fn load_configuration(connection: &Connection) -> Result<LspConfiguration, DomainModelError> {
    let enabled = connection
        .query_row(
            "SELECT enabled FROM lsp_configuration WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| storage_error())?;
    let mut statement = connection
        .prepare(
            "SELECT language_id, enabled, executable_override, initialization_options_json \
             FROM lsp_language_configurations ORDER BY language_id",
        )
        .map_err(|_| storage_error())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, bool>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| storage_error())?;
    let mut languages = BTreeMap::new();
    for row in rows {
        let (language, language_enabled, executable_override, options) =
            row.map_err(|_| storage_error())?;
        let language = LanguageFamily::parse(&language)?;
        let initialization_options = serde_json::from_str(&options)
            .map_err(|_| DomainModelError::InvalidInitializationOptions)?;
        languages.insert(
            language,
            LanguageConfiguration {
                enabled: language_enabled,
                executable_override,
                initialization_options,
            },
        );
    }
    let configuration = LspConfiguration { enabled, languages };
    configuration.validate()?;
    Ok(configuration)
}

const fn storage_error() -> DomainModelError {
    DomainModelError::Storage
}
