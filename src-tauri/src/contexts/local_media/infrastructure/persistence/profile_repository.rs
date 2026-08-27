use rusqlite::{params, OptionalExtension};
use std::sync::Arc;

use crate::contexts::local_media::application::ports::{
    LocalMediaClock, LocalMediaProfileRepository,
};
use crate::contexts::local_media::domain::{
    FasterWhisperProfile, LocalMediaError, LocalMediaErrorCode, LocalMediaProfile,
    PaddleOcrProfile, SherpaOnnxTtsProfile, DEFAULT_PROFILE_ID,
};
use crate::platform::database::NativeDatabase;

pub(crate) struct SqliteLocalMediaProfileRepository {
    database: NativeDatabase,
    clock: Arc<dyn LocalMediaClock>,
}

impl SqliteLocalMediaProfileRepository {
    pub(crate) fn new(database: NativeDatabase, clock: Arc<dyn LocalMediaClock>) -> Self {
        Self { database, clock }
    }
}

/// A stored engine configuration that will not parse is replaced by its default rather than
/// failing the read.
///
/// The alternative is a settings page that cannot open, which leaves the user with no way to fix
/// the row that broke it. A disabled default is inert -- no worker starts from it -- so the
/// recovery path is strictly safer than the failure path.
fn parse_or_default<T: serde::de::DeserializeOwned + Default>(raw: &str) -> T {
    serde_json::from_str(raw).unwrap_or_default()
}

fn encode<T: serde::Serialize>(value: &T) -> Result<String, LocalMediaError> {
    serde_json::to_string(value)
        .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::TempStorageFailed))
}

fn storage_error(_error: rusqlite::Error) -> LocalMediaError {
    // The rusqlite message can carry the database path, so only the stable code survives.
    LocalMediaError::new(LocalMediaErrorCode::TempStorageFailed)
}

impl LocalMediaProfileRepository for SqliteLocalMediaProfileRepository {
    fn load(&self) -> Result<LocalMediaProfile, LocalMediaError> {
        let connection = self
            .database
            .connection()
            .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::TempStorageFailed))?;

        let existing = connection
            .query_row(
                "SELECT revision, enabled, ocr_config_json, stt_config_json, tts_config_json, updated_at
                 FROM local_media_profiles WHERE profile_id = ?1",
                params![DEFAULT_PROFILE_ID],
                |row| {
                    Ok(LocalMediaProfile {
                        profile_id: DEFAULT_PROFILE_ID.to_string(),
                        revision: row.get(0)?,
                        enabled: row.get::<_, i64>(1)? != 0,
                        ocr: parse_or_default::<PaddleOcrProfile>(&row.get::<_, String>(2)?),
                        stt: parse_or_default::<FasterWhisperProfile>(&row.get::<_, String>(3)?),
                        tts: parse_or_default::<SherpaOnnxTtsProfile>(&row.get::<_, String>(4)?),
                        updated_at: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(storage_error)?;

        if let Some(profile) = existing {
            return Ok(profile);
        }

        // First use. The defaults are written rather than only returned so that the revision the
        // settings page sends back on its first save refers to a row that exists.
        let defaults = LocalMediaProfile::disabled_default(self.clock.now_iso());
        connection
            .execute(
                "INSERT OR IGNORE INTO local_media_profiles
                 (profile_id, revision, enabled, ocr_config_json, stt_config_json, tts_config_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    DEFAULT_PROFILE_ID,
                    defaults.revision,
                    i64::from(defaults.enabled),
                    encode(&defaults.ocr)?,
                    encode(&defaults.stt)?,
                    encode(&defaults.tts)?,
                    defaults.updated_at,
                ],
            )
            .map_err(storage_error)?;
        Ok(defaults)
    }

    fn save(
        &self,
        profile: &LocalMediaProfile,
        expected_revision: i64,
    ) -> Result<LocalMediaProfile, LocalMediaError> {
        let connection = self
            .database
            .connection()
            .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::TempStorageFailed))?;

        let mut next = profile.clone();
        next.profile_id = DEFAULT_PROFILE_ID.to_string();
        next.revision = expected_revision + 1;
        next.updated_at = self.clock.now_iso();

        // The revision guard is the `WHERE` clause, not a preceding `SELECT`. A read-then-write
        // would leave a window in which two saves both observe the same revision and the second
        // silently overwrites the first.
        let updated = connection
            .execute(
                "UPDATE local_media_profiles
                 SET revision = ?2, enabled = ?3, ocr_config_json = ?4, stt_config_json = ?5,
                     tts_config_json = ?6, updated_at = ?7
                 WHERE profile_id = ?1 AND revision = ?8",
                params![
                    DEFAULT_PROFILE_ID,
                    next.revision,
                    i64::from(next.enabled),
                    encode(&next.ocr)?,
                    encode(&next.stt)?,
                    encode(&next.tts)?,
                    next.updated_at,
                    expected_revision,
                ],
            )
            .map_err(storage_error)?;

        if updated == 0 {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::ProfileRevisionConflict,
            ));
        }
        Ok(next)
    }
}

#[cfg(test)]
#[path = "profile_repository_tests.rs"]
mod tests;
