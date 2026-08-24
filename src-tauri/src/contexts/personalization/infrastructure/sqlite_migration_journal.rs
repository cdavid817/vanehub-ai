use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension, Row};

use crate::contexts::personalization::application::{
    MigrationJournalPort, PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::{
    LegacySourceFingerprint, LegacySourceId, LegacySourceLocator, LegacyTableKind, MemoryId,
    MigrationJournalEntry, MigrationStage,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

fn storage(error: impl std::fmt::Display) -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage(error.to_string())
}

const JOURNAL_COLUMNS: &str = "source_id, locator_kind, locator_path, locator_table, \
     locator_row_id, target_memory_id, stage, backup_relative_path, source_raw_sha256, \
     source_byte_length, last_error_code";

fn read_entry(row: &Row<'_>) -> rusqlite::Result<Result<MigrationJournalEntry>> {
    let source_id: String = row.get(0)?;
    let locator_kind: String = row.get(1)?;
    let locator_path: Option<String> = row.get(2)?;
    let locator_table: Option<String> = row.get(3)?;
    let locator_row_id: Option<String> = row.get(4)?;
    let target_memory_id: Option<String> = row.get(5)?;
    let stage: String = row.get(6)?;
    let backup_relative_path: Option<String> = row.get(7)?;
    let source_raw_sha256: Option<String> = row.get(8)?;
    let source_byte_length: Option<i64> = row.get(9)?;
    let last_error_code: Option<String> = row.get(10)?;

    Ok((|| {
        let locator = match locator_kind.as_str() {
            "file" => LegacySourceLocator::markdown(locator_path.as_deref().ok_or_else(|| {
                PersonalizationApplicationError::Storage(
                    "a file journal row has no locator path".to_string(),
                )
            })?)?,
            "row" => LegacySourceLocator::sqlite_row(
                LegacyTableKind::parse(locator_table.as_deref().unwrap_or_default())?,
                locator_row_id.as_deref().ok_or_else(|| {
                    PersonalizationApplicationError::Storage(
                        "a row journal row has no locator row id".to_string(),
                    )
                })?,
            )?,
            other => {
                return Err(PersonalizationApplicationError::Storage(format!(
                    "unknown journal locator kind {other:?}"
                )))
            }
        };

        // A fingerprint is only meaningful with both halves. A digest without a length would let a
        // partial row pass a check it never actually performed.
        let source_fingerprint = match (source_raw_sha256, source_byte_length) {
            (Some(raw_sha256), Some(byte_length)) => Some(LegacySourceFingerprint {
                raw_sha256,
                byte_length: u64::try_from(byte_length).unwrap_or_default(),
            }),
            _ => None,
        };

        Ok(MigrationJournalEntry {
            source_id: LegacySourceId::parse(&source_id)?,
            locator,
            target_memory_id: target_memory_id
                .as_deref()
                .map(MemoryId::parse)
                .transpose()?,
            stage: MigrationStage::parse(&stage)?,
            backup_relative_path,
            source_fingerprint,
            last_error_code,
        })
    })())
}

/// Migration progress per discovered source.
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

/// Splits a locator into the four nullable columns that describe it.
fn locator_columns(
    locator: &LegacySourceLocator,
) -> (
    &'static str,
    Option<String>,
    Option<&'static str>,
    Option<String>,
) {
    match locator {
        LegacySourceLocator::MarkdownFile {
            normalized_relative_path,
        } => (
            "file",
            Some(normalized_relative_path.as_str().to_string()),
            None,
            None,
        ),
        LegacySourceLocator::SqliteRow { table, row_id } => {
            ("row", None, Some(table.as_str()), Some(row_id.clone()))
        }
    }
}

impl MigrationJournalPort for SqliteMigrationJournal {
    fn get(&self, source_id: &LegacySourceId) -> Result<Option<MigrationJournalEntry>> {
        let conn = self.connection()?;
        let statement = format!(
            "SELECT {JOURNAL_COLUMNS} FROM personalization_memory_migration_journal \
             WHERE source_id = ?1"
        );
        conn.query_row(&statement, params![source_id.as_str()], read_entry)
            .optional()
            .map_err(storage)?
            .transpose()
    }

    fn find_by_memory(&self, memory_id: &MemoryId) -> Result<Vec<MigrationJournalEntry>> {
        let conn = self.connection()?;
        let statement = format!(
            "SELECT {JOURNAL_COLUMNS} FROM personalization_memory_migration_journal \
             WHERE target_memory_id = ?1 ORDER BY source_id"
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
        let (kind, path, table, row_id) = locator_columns(&entry.locator);
        conn.execute(
            "INSERT INTO personalization_memory_migration_journal (
                 source_id, locator_kind, locator_path, locator_table, locator_row_id,
                 target_memory_id, stage, backup_relative_path, source_raw_sha256,
                 source_byte_length, last_error_code, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
             ON CONFLICT(source_id) DO UPDATE SET
                 target_memory_id = excluded.target_memory_id,
                 stage = excluded.stage,
                 backup_relative_path = excluded.backup_relative_path,
                 source_raw_sha256 = excluded.source_raw_sha256,
                 source_byte_length = excluded.source_byte_length,
                 last_error_code = excluded.last_error_code,
                 updated_at = excluded.updated_at",
            params![
                entry.source_id.as_str(),
                kind,
                path,
                table,
                row_id,
                entry.target_memory_id.as_ref().map(MemoryId::as_str),
                entry.stage.as_str(),
                entry.backup_relative_path.as_deref(),
                entry
                    .source_fingerprint
                    .as_ref()
                    .map(|fingerprint| fingerprint.raw_sha256.as_str()),
                entry
                    .source_fingerprint
                    .as_ref()
                    .map(|fingerprint| i64::try_from(fingerprint.byte_length).unwrap_or(i64::MAX)),
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
             ORDER BY source_id"
        );
        let mut prepared = conn.prepare(&statement).map_err(storage)?;
        let rows = prepared.query_map([], read_entry).map_err(storage)?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(storage)??);
        }
        Ok(entries)
    }

    fn remove(&self, source_id: &LegacySourceId) -> Result<bool> {
        let conn = self.connection()?;
        let removed = conn
            .execute(
                "DELETE FROM personalization_memory_migration_journal WHERE source_id = ?1",
                params![source_id.as_str()],
            )
            .map_err(storage)?;
        Ok(removed > 0)
    }
}
