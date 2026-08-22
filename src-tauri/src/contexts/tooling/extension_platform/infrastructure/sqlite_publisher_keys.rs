// Assembled in bootstrap with the settings surface in task 12; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! SQLite adapter for trusted publisher keys.
//!
//! A row whose stored fields no longer make sense — key material that is not 32 bytes, an
//! unreadable source or trust state — is dropped from a list read rather than failing it. The
//! alternative is one corrupt row making every trusted key invisible, and invisible keys mean
//! every signed package suddenly reads as signed by an unknown publisher. Dropping resolves that
//! one key to "not trusted", which is the fail-closed answer for the row that is actually broken.

use crate::contexts::tooling::extension_platform::application::TrustedPublisherKeyRepository;
use crate::contexts::tooling::extension_platform::domain::{
    parse_publisher_key_material, PublisherId, PublisherKeyFingerprint, PublisherKeyLabel,
    PublisherKeySource, PublisherTrustState, TrustedPublisherKey,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rusqlite::{params, OptionalExtension, Row};
use std::sync::Arc;

const COLUMNS: &str = "fingerprint, publisher, key_material, label, source, trust_state, \
                       first_seen_at, last_seen_at, revoked_at, revocation_reason";

pub(crate) struct SqlitePublisherKeyRepository {
    database: Arc<NativeDatabase>,
}

impl SqlitePublisherKeyRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, String> {
        self.database
            .connection()
            .map_err(|error| error.to_string())
    }
}

/// What one row says, before it is known to be readable as a key.
struct StoredRow {
    fingerprint: String,
    publisher: String,
    key_material: String,
    label: String,
    source: String,
    trust_state: String,
    first_seen_at: String,
    last_seen_at: String,
    revoked_at: Option<String>,
    revocation_reason: Option<String>,
}

fn read_row(row: &Row<'_>) -> rusqlite::Result<StoredRow> {
    Ok(StoredRow {
        fingerprint: row.get(0)?,
        publisher: row.get(1)?,
        key_material: row.get(2)?,
        label: row.get(3)?,
        source: row.get(4)?,
        trust_state: row.get(5)?,
        first_seen_at: row.get(6)?,
        last_seen_at: row.get(7)?,
        revoked_at: row.get(8)?,
        revocation_reason: row.get(9)?,
    })
}

/// Rebuilds a key from a row, or `None` if the row no longer describes one.
///
/// The stored fingerprint is checked against the one the key bytes actually produce. A row whose
/// two disagree has been edited or corrupted, and honouring it would let whoever edited the
/// database file choose which key a fingerprint resolves to.
fn to_key(row: StoredRow) -> Option<TrustedPublisherKey> {
    let key = parse_publisher_key_material(&row.key_material).ok()?;
    if key.fingerprint().as_str() != row.fingerprint {
        return None;
    }
    let trust_state = match row.trust_state.as_str() {
        "trusted" => PublisherTrustState::Trusted,
        "revoked" => PublisherTrustState::Revoked,
        _ => return None,
    };
    Some(TrustedPublisherKey {
        publisher: PublisherId::parse(&row.publisher).ok()?,
        key,
        label: PublisherKeyLabel::parse(&row.label).ok()?,
        source: PublisherKeySource::parse(&row.source)?,
        trust_state,
        first_seen_at: row.first_seen_at,
        last_seen_at: row.last_seen_at,
        revoked_at: row.revoked_at,
        revocation_reason: row.revocation_reason,
    })
}

impl TrustedPublisherKeyRepository for SqlitePublisherKeyRepository {
    fn list(&self) -> Result<Vec<TrustedPublisherKey>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {COLUMNS} FROM extension_platform_publisher_keys \
                 ORDER BY publisher, fingerprint"
            ))
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], read_row)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(rows.into_iter().filter_map(to_key).collect())
    }

    fn find(
        &self,
        fingerprint: &PublisherKeyFingerprint,
    ) -> Result<Option<TrustedPublisherKey>, String> {
        let connection = self.connection()?;
        let row = connection
            .query_row(
                &format!(
                    "SELECT {COLUMNS} FROM extension_platform_publisher_keys WHERE fingerprint = ?1"
                ),
                params![fingerprint.as_str()],
                read_row,
            )
            .optional()
            .map_err(|error| error.to_string())?;
        Ok(row.and_then(to_key))
    }

    fn upsert(&self, key: &TrustedPublisherKey) -> Result<(), String> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO extension_platform_publisher_keys \
                     (fingerprint, publisher, key_material, label, source, trust_state, \
                      first_seen_at, last_seen_at, revoked_at, revocation_reason) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
                 ON CONFLICT(fingerprint) DO UPDATE SET \
                     label = excluded.label, \
                     source = excluded.source, \
                     last_seen_at = excluded.last_seen_at",
                params![
                    key.fingerprint().as_str(),
                    key.publisher.as_str(),
                    STANDARD.encode(key.key.as_bytes()),
                    key.label.as_str(),
                    key.source.as_str(),
                    key.trust_state.as_str(),
                    key.first_seen_at,
                    key.last_seen_at,
                    key.revoked_at,
                    key.revocation_reason,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn revoke(
        &self,
        fingerprint: &PublisherKeyFingerprint,
        revoked_at: &str,
        reason: Option<&str>,
    ) -> Result<(), String> {
        let connection = self.connection()?;
        // `WHERE trust_state = 'trusted'` is what makes this idempotent: a second revocation is a
        // no-op rather than a rewrite, so the recorded moment stays the moment trust was actually
        // withdrawn.
        connection
            .execute(
                "UPDATE extension_platform_publisher_keys \
                 SET trust_state = 'revoked', revoked_at = ?2, revocation_reason = ?3 \
                 WHERE fingerprint = ?1 AND trust_state = 'trusted'",
                params![fingerprint.as_str(), revoked_at, reason],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
