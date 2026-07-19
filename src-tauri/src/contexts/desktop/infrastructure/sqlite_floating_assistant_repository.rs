use crate::contexts::desktop::application::{
    FloatingAssistantApplicationError, FloatingAssistantRepository,
};
use crate::contexts::desktop::domain::{FloatingAssistantAnchor, FloatingAssistantConfig};
use crate::platform::clock::SystemClock;
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection};

#[derive(Clone)]
pub(crate) struct SqliteFloatingAssistantRepository {
    database: NativeDatabase,
}

impl SqliteFloatingAssistantRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl FloatingAssistantRepository for SqliteFloatingAssistantRepository {
    fn load(&self) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        load_config(&connection)
    }

    fn save(
        &self,
        config: &FloatingAssistantConfig,
        updated_at: &str,
    ) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        let anchor = config.anchor();
        connection
            .execute(
                "UPDATE floating_assistant_config
                 SET enabled = ?1, anchor_x = ?2, anchor_y = ?3, monitor_name = ?4, updated_at = ?5
                 WHERE id = 1",
                params![
                    i64::from(config.enabled()),
                    anchor.map(FloatingAssistantAnchor::x),
                    anchor.map(FloatingAssistantAnchor::y),
                    anchor.and_then(FloatingAssistantAnchor::monitor_name),
                    updated_at,
                ],
            )
            .map_err(repository_error)?;
        Ok(config.clone())
    }
}

fn load_config(
    connection: &Connection,
) -> Result<FloatingAssistantConfig, FloatingAssistantApplicationError> {
    let (enabled, x, y, monitor_name) = connection
        .query_row(
            "SELECT enabled, anchor_x, anchor_y, monitor_name
             FROM floating_assistant_config WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? != 0,
                    row.get::<_, Option<f64>>(1)?,
                    row.get::<_, Option<f64>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .map_err(repository_error)?;
    let anchor = match (x, y) {
        (Some(x), Some(y)) => FloatingAssistantAnchor::new(x, y, monitor_name),
        _ => None,
    };
    Ok(FloatingAssistantConfig::new(enabled, anchor))
}

fn repository_error(error: impl std::fmt::Display) -> FloatingAssistantApplicationError {
    FloatingAssistantApplicationError::Repository(error.to_string())
}

pub(crate) fn apply_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS floating_assistant_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
            anchor_x REAL,
            anchor_y REAL,
            monitor_name TEXT,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO floating_assistant_config (id, enabled, updated_at)
         VALUES (1, 0, ?1)",
        params![SystemClock.rfc3339()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::desktop::application::FloatingAssistantRepository;
    use crate::test_support::TempDirectory;

    fn repository() -> (
        TempDirectory,
        NativeDatabase,
        SqliteFloatingAssistantRepository,
    ) {
        let directory = TempDirectory::new("floating-assistant-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteFloatingAssistantRepository::new(database.clone());
        (directory, database, repository)
    }

    #[test]
    fn defaults_disabled_and_round_trips_the_existing_row_shape() {
        let (_directory, _database, repository) = repository();
        assert_eq!(
            repository.load().expect("default"),
            FloatingAssistantConfig::disabled()
        );

        let config = FloatingAssistantConfig::new(
            true,
            FloatingAssistantAnchor::new(1280.5, 720.25, Some("DISPLAY1".to_string())),
        );
        repository
            .save(&config, "2026-07-18T12:00:00Z")
            .expect("save");

        assert_eq!(repository.load().expect("reload"), config);
    }

    #[test]
    fn invalid_legacy_coordinates_map_to_no_anchor() {
        let (_directory, database, repository) = repository();
        database
            .connection()
            .expect("connection")
            .execute(
                "UPDATE floating_assistant_config
                 SET enabled = 1, anchor_x = ?1, anchor_y = 1, monitor_name = 'DISPLAY1'",
                [f64::NAN],
            )
            .expect("corrupt fixture");

        let config = repository.load().expect("normalized");

        assert!(config.enabled());
        assert!(config.anchor().is_none());
    }
}
