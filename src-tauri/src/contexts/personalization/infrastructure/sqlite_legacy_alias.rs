use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::{params, OptionalExtension};

use crate::contexts::personalization::application::{
    LegacyAddressAliasPort, PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::{LegacyAddressKey, MemoryId};
use crate::platform::database::{NativeDatabase, PooledSqlite};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

fn storage(error: impl std::fmt::Display) -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage(error.to_string())
}

/// Compatibility addressing only.
///
/// Deliberately a separate store from the migration journal. They answer different questions and
/// have different lifetimes: this one exists for as long as a pre-governance caller does, and is
/// keyed by something a caller supplies rather than by something a scan discovered.
#[derive(Clone)]
pub(crate) struct SqliteLegacyAddressAlias {
    database: NativeDatabase,
}

impl SqliteLegacyAddressAlias {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite> {
        self.database.connection().map_err(storage)
    }
}

impl LegacyAddressAliasPort for SqliteLegacyAddressAlias {
    fn get(&self, address: &LegacyAddressKey) -> Result<Option<MemoryId>> {
        let conn = self.connection()?;
        let stored: Option<String> = conn
            .query_row(
                "SELECT target_memory_id FROM personalization_legacy_memory_alias \
                 WHERE legacy_address_key = ?1",
                params![address.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;
        Ok(stored.as_deref().map(MemoryId::parse).transpose()?)
    }

    fn put(&self, address: &LegacyAddressKey, target: &MemoryId, now: DateTime<Utc>) -> Result<()> {
        let conn = self.connection()?;
        let now = now.to_rfc3339_opts(SecondsFormat::Millis, true);
        conn.execute(
            "INSERT INTO personalization_legacy_memory_alias
                 (legacy_address_key, target_memory_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(legacy_address_key) DO UPDATE SET
                 target_memory_id = excluded.target_memory_id,
                 updated_at = excluded.updated_at",
            params![address.as_str(), target.as_str(), now],
        )
        .map_err(storage)?;
        Ok(())
    }

    fn remove(&self, address: &LegacyAddressKey) -> Result<bool> {
        let conn = self.connection()?;
        let removed = conn
            .execute(
                "DELETE FROM personalization_legacy_memory_alias WHERE legacy_address_key = ?1",
                params![address.as_str()],
            )
            .map_err(storage)?;
        Ok(removed > 0)
    }

    fn list_all(&self) -> Result<Vec<(LegacyAddressKey, MemoryId)>> {
        let conn = self.connection()?;
        let mut prepared = conn
            .prepare(
                "SELECT legacy_address_key, target_memory_id \
                 FROM personalization_legacy_memory_alias ORDER BY legacy_address_key",
            )
            .map_err(storage)?;
        let rows = prepared
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage)?;
        let mut aliases = Vec::new();
        for row in rows {
            let (address, target) = row.map_err(storage)?;
            aliases.push((
                LegacyAddressKey::parse(&address)?,
                MemoryId::parse(&target)?,
            ));
        }
        Ok(aliases)
    }
}
