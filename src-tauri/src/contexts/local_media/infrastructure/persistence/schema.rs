use rusqlite::Connection;

use crate::platform::database::DatabaseError;

/// Storage for the local-media profile.
///
/// One row, enforced by the primary-key CHECK rather than by convention, because V1 promises a
/// single profile and a second row would silently change which configuration the workers run
/// against. The three engine configurations are JSON columns: they evolve independently of each
/// other and of the schema, and widening them field by field would mean a migration per settings
/// addition. `revision` is a real column so optimistic concurrency can be expressed as a `WHERE`
/// clause instead of a read-modify-write.
pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS local_media_profiles (
            profile_id TEXT PRIMARY KEY CHECK (profile_id = 'default'),
            revision INTEGER NOT NULL,
            enabled INTEGER NOT NULL,
            ocr_config_json TEXT NOT NULL,
            stt_config_json TEXT NOT NULL,
            tts_config_json TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}
