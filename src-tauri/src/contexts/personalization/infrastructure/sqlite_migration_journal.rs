use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, Row};

use crate::contexts::personalization::application::{
    MigrationJournalPort, PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::{
    LegacySourceId, MemoryId, MigrationJournalEntry, MigrationStage,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

fn storage(error: impl std::fmt::Display) -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage(error.to_string())
}

const JOURNAL_COLUMNS: &str = "legacy_source_id, memory_id, stage, legacy_backup_path, \
     legacy_content_hash, last_error_code";

fn read_entry(row: &Row<'_>) -> rusqlite::Result<Result<MigrationJournalEntry>> {
    let legacy_source_id: String = row.get(0)?;
    let memory_id: Option<String> = row.get(1)?;
    let stage: String = row.get(2)?;
    let legacy_backup_path: Option<String> = row.get(3)?;
    let legacy_content_hash: Option<String> = row.get(4)?;
    let last_error_code: Option<String> = row.get(5)?;

    Ok((|| {
        Ok(MigrationJournalEntry {
            legacy_source_id: LegacySourceId::parse(&legacy_source_id)?,
            memory_id: memory_id.as_deref().map(MemoryId::parse).transpose()?,
            stage: MigrationStage::parse(&stage)?,
            legacy_backup_path,
            legacy_content_hash,
            last_error_code,
        })
    })())
}

/// The migration journal and legacy-identity alias table.
#[derive(Clone)]
pub(crate) struct SqliteMigrationJournal {
    database: NativeDatabase,
}

impl SqliteMigrationJournal {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite> {
        self.database.connection().map_err(storage)
    }
}

impl MigrationJournalPort for SqliteMigrationJournal {
    fn get(&self, legacy_source_id: &LegacySourceId) -> Result<Option<MigrationJournalEntry>> {
        let conn = self.connection()?;
        let statement = format!(
            "SELECT {JOURNAL_COLUMNS} FROM personalization_memory_migration_journal \
             WHERE legacy_source_id = ?1"
        );
        conn.query_row(&statement, params![legacy_source_id.as_str()], read_entry)
            .optional()
            .map_err(storage)?
            .transpose()
    }

    fn find_by_memory(&self, memory_id: &MemoryId) -> Result<Vec<MigrationJournalEntry>> {
        let conn = self.connection()?;
        let statement = format!(
            "SELECT {JOURNAL_COLUMNS} FROM personalization_memory_migration_journal \
             WHERE memory_id = ?1 ORDER BY legacy_source_id"
        );
        let mut prepared = conn.prepare(&statement).map_err(storage)?;
        let rows = prepared
            .query_map(params![memory_id.as_str()], read_entry)
            .map_err(storage)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(storage)??);
        }
        Ok(entries)
    }

    fn upsert(&self, entry: &MigrationJournalEntry, now: DateTime<Utc>) -> Result<()> {
        let conn = self.connection()?;
        let now = now.to_rfc3339_opts(SecondsFormat::Millis, true);
        conn.execute(
            "INSERT INTO personalization_memory_migration_journal (
                 legacy_source_id, memory_id, stage, legacy_backup_path, legacy_content_hash,
                 last_error_code, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(legacy_source_id) DO UPDATE SET
                 memory_id = excluded.memory_id,
                 stage = excluded.stage,
                 legacy_backup_path = excluded.legacy_backup_path,
                 legacy_content_hash = excluded.legacy_content_hash,
                 last_error_code = excluded.last_error_code,
                 updated_at = excluded.updated_at",
            params![
                entry.legacy_source_id.as_str(),
                entry.memory_id.as_ref().map(MemoryId::as_str),
                entry.stage.as_str(),
                entry.legacy_backup_path.as_deref(),
                entry.legacy_content_hash.as_deref(),
                entry.last_error_code.as_deref(),
                now,
            ],
        )
        .map_err(storage)?;
        Ok(())
    }

    fn list_all(&self) -> Result<Vec<MigrationJournalEntry>> {
        let conn = self.connection()?;
        let statement = format!(
            "SELECT {JOURNAL_COLUMNS} FROM personalization_memory_migration_journal \
             ORDER BY legacy_source_id"
        );
        let mut prepared = conn.prepare(&statement).map_err(storage)?;
        let rows = prepared.query_map([], read_entry).map_err(storage)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(storage)??);
        }
        Ok(entries)
    }

    fn remove(&self, legacy_source_id: &LegacySourceId) -> Result<bool> {
        let conn = self.connection()?;
        let removed = conn
            .execute(
                "DELETE FROM personalization_memory_migration_journal WHERE legacy_source_id = ?1",
                params![legacy_source_id.as_str()],
            )
            .map_err(storage)?;
        Ok(removed > 0)
    }
}
