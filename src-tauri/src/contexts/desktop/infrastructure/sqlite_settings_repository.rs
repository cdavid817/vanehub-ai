use crate::contexts::desktop::application::{
    DesktopSettingsApplicationError, DesktopSettingsRepository, StoredDesktopSetting,
};
use crate::contexts::desktop::domain::{
    AutomaticArchivalSettings, DesktopSettingKey, DesktopSettingMutation,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Clone)]
pub(crate) struct SqliteDesktopSettingsRepository {
    database: NativeDatabase,
}

impl SqliteDesktopSettingsRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn load_folder_opener_preferences(
        &self,
    ) -> Result<(Option<String>, Option<String>), DesktopSettingsApplicationError> {
        let connection = self.database.connection().map_err(database_error)?;
        let load = |key: &str| {
            connection
                .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                    row.get(0)
                })
                .optional()
                .map_err(repository_error)
        };
        Ok((
            load("defaultFolderOpenerId")?,
            load("enabledFolderOpenerIds")?,
        ))
    }

    pub(crate) fn save_folder_opener_preferences(
        &self,
        default_id: &str,
        enabled_ids: &str,
        updated_at: &str,
    ) -> Result<(), DesktopSettingsApplicationError> {
        let mut connection = self.database.connection().map_err(database_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        upsert_setting(
            &transaction,
            "defaultFolderOpenerId",
            default_id,
            updated_at,
        )
        .map_err(repository_error)?;
        upsert_setting(
            &transaction,
            "enabledFolderOpenerIds",
            enabled_ids,
            updated_at,
        )
        .map_err(repository_error)?;
        transaction.commit().map_err(repository_error)
    }
}

impl DesktopSettingsRepository for SqliteDesktopSettingsRepository {
    fn load_settings(&self) -> Result<Vec<StoredDesktopSetting>, DesktopSettingsApplicationError> {
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare("SELECT key, value FROM settings ORDER BY key")
            .map_err(repository_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(repository_error)?;
        let mut settings = Vec::new();
        for row in rows {
            let (key, value) = row.map_err(repository_error)?;
            if let Ok(key) = DesktopSettingKey::parse(&key) {
                settings.push(StoredDesktopSetting { key, value });
            }
        }
        Ok(settings)
    }

    fn save_setting(
        &self,
        mutation: &DesktopSettingMutation,
        updated_at: &str,
    ) -> Result<(), DesktopSettingsApplicationError> {
        let connection = self.database.connection().map_err(database_error)?;
        upsert_setting(
            &connection,
            mutation.key().as_str(),
            &mutation.persisted_value(),
            updated_at,
        )
        .map_err(repository_error)
    }

    fn save_automatic_archival(
        &self,
        settings: AutomaticArchivalSettings,
        updated_at: &str,
    ) -> Result<(), DesktopSettingsApplicationError> {
        let mut connection = self.database.connection().map_err(database_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        upsert_setting(
            &transaction,
            DesktopSettingKey::AutomaticArchivalEnabled.as_str(),
            &settings.enabled().to_string(),
            updated_at,
        )
        .map_err(repository_error)?;
        upsert_setting(
            &transaction,
            DesktopSettingKey::AutomaticArchivalInactiveDays.as_str(),
            &settings.inactive_days().to_string(),
            updated_at,
        )
        .map_err(repository_error)?;
        transaction.commit().map_err(repository_error)
    }
}

fn upsert_setting(
    connection: &Connection,
    key: &str,
    value: &str,
    updated_at: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        r#"
        INSERT INTO settings (key, value, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?3)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
        "#,
        params![key, value, updated_at],
    )?;
    Ok(())
}

fn database_error(
    error: crate::platform::database::DatabaseError,
) -> DesktopSettingsApplicationError {
    DesktopSettingsApplicationError::Repository(error.to_string())
}

fn repository_error(error: rusqlite::Error) -> DesktopSettingsApplicationError {
    DesktopSettingsApplicationError::Repository(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn fixture() -> (
        TempDirectory,
        NativeDatabase,
        SqliteDesktopSettingsRepository,
    ) {
        let directory = TempDirectory::new("desktop-settings-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteDesktopSettingsRepository::new(database.clone());
        (directory, database, repository)
    }

    #[test]
    fn maps_only_owned_setting_rows_and_preserves_existing_storage_contract() {
        let (_directory, database, repository) = fixture();
        let connection = database.connection().expect("connection");
        connection
            .execute(
                "INSERT INTO settings (key, value, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
                params!["applicationLanguage", "en", "fixture-time"],
            )
            .expect("known setting");
        connection
            .execute(
                "INSERT INTO settings (key, value, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
                params!["futureSetting", "preserve-me", "fixture-time"],
            )
            .expect("future setting");

        let settings = repository.load_settings().expect("settings");

        assert!(settings.contains(&StoredDesktopSetting {
            key: DesktopSettingKey::ApplicationLanguage,
            value: "en".to_string(),
        }));
        assert!(!settings
            .iter()
            .any(|setting| setting.value == "preserve-me"));
        let future: String = connection
            .query_row(
                "SELECT value FROM settings WHERE key = 'futureSetting'",
                [],
                |row| row.get(0),
            )
            .expect("future setting remains");
        assert_eq!(future, "preserve-me");
    }

    #[test]
    fn upsert_keeps_created_at_and_updates_value_and_timestamp() {
        let (_directory, database, repository) = fixture();
        let mutation = DesktopSettingMutation::parse("fontSize", "16px").expect("mutation");
        repository
            .save_setting(&mutation, "2026-07-18T10:00:00Z")
            .expect("first save");
        let mutation = DesktopSettingMutation::parse("fontSize", "18px").expect("mutation");
        repository
            .save_setting(&mutation, "2026-07-18T11:00:00Z")
            .expect("second save");

        let connection = database.connection().expect("connection");
        let row: (String, String, String) = connection
            .query_row(
                "SELECT value, created_at, updated_at FROM settings WHERE key = 'fontSize'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("setting row");
        assert_eq!(
            row,
            (
                "18px".to_string(),
                "2026-07-18T10:00:00Z".to_string(),
                "2026-07-18T11:00:00Z".to_string(),
            )
        );
    }

    #[test]
    fn automatic_archival_settings_are_committed_atomically() {
        let (_directory, database, repository) = fixture();
        let connection = database.connection().expect("connection");
        connection
            .execute_batch(
                r#"
                CREATE TRIGGER reject_archival_days
                BEFORE UPDATE OF value ON settings
                WHEN NEW.key = 'automaticArchivalInactiveDays'
                BEGIN
                    SELECT RAISE(ABORT, 'fixture failure');
                END;
                "#,
            )
            .expect("failure trigger");
        drop(connection);

        let result = repository.save_automatic_archival(
            AutomaticArchivalSettings::new(false, 30).expect("archival"),
            "2026-07-18T12:00:00Z",
        );

        assert!(result.is_err());
        let connection = database.connection().expect("connection");
        let rows: (String, String) = connection
            .query_row(
                "SELECT enabled.value, days.value FROM settings enabled JOIN settings days ON days.key = 'automaticArchivalInactiveDays' WHERE enabled.key = 'automaticArchivalEnabled'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("archival rows");
        assert_eq!(rows, ("true".to_string(), "10".to_string()));
    }

    #[test]
    fn folder_opener_preferences_are_committed_atomically() {
        let (_directory, database, repository) = fixture();
        let connection = database.connection().expect("connection");
        connection.execute_batch(
            "CREATE TRIGGER reject_opener_enabled BEFORE INSERT ON settings WHEN NEW.key = 'enabledFolderOpenerIds' BEGIN SELECT RAISE(ABORT, 'fixture failure'); END;",
        ).expect("failure trigger");
        drop(connection);

        assert!(repository
            .save_folder_opener_preferences(
                "vscode",
                "vscode,file-explorer",
                "2026-07-19T00:00:00Z"
            )
            .is_err());
        let connection = database.connection().expect("connection");
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key = 'defaultFolderOpenerId'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(count, 0);
    }
}
