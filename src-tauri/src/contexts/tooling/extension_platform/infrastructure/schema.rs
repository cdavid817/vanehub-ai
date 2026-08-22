//! Additive schema for Extension Platform capability gates.

use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// Stores desired state only.
///
/// Build availability is deliberately absent: it comes from `cfg!` at evaluation time, so a
/// database carried between a build that had a Cargo feature and one that did not can never
/// assert a capability the running binary lacks. Rows are seeded lazily — a gate with no row is
/// disabled, which is also the value a fresh install must have.
pub(crate) fn apply_feature_gate_schema(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS extension_platform_feature_gates (
            feature TEXT PRIMARY KEY,
            desired_enabled INTEGER NOT NULL DEFAULT 0,
            revision INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            updated_by TEXT NOT NULL,
            reason TEXT
        );

        CREATE TABLE IF NOT EXISTS extension_platform_feature_gate_audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            feature TEXT NOT NULL,
            previous_enabled INTEGER NOT NULL,
            new_enabled INTEGER NOT NULL,
            revision INTEGER NOT NULL,
            recorded_at TEXT NOT NULL,
            actor TEXT NOT NULL,
            reason TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_extension_platform_feature_gate_audit_feature
            ON extension_platform_feature_gate_audit (feature, id DESC);
        "#,
    )?;
    Ok(())
}

/// The Developer Mode switch, and every change to it.
///
/// One row, enforced by the primary-key check rather than by convention: a second row would be a
/// second answer to "is unsigned content admitted?", and whichever one a query happened to return
/// would decide it.
///
/// Deliberately not an eighth capability gate. The seven gates say which parts of the Extension
/// Platform are built and switched on; this says whether content with no provenance may be
/// installed at all. Filing it with them would put an admission policy behind a rollout switch.
pub(crate) fn apply_developer_mode_schema(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS extension_platform_developer_mode (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            enabled INTEGER NOT NULL DEFAULT 0,
            revision INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            updated_by TEXT NOT NULL,
            reason TEXT
        );

        CREATE TABLE IF NOT EXISTS extension_platform_developer_mode_audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            previous_enabled INTEGER NOT NULL,
            new_enabled INTEGER NOT NULL,
            revision INTEGER NOT NULL,
            recorded_at TEXT NOT NULL,
            actor TEXT NOT NULL,
            reason TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_extension_platform_developer_mode_audit_recorded
            ON extension_platform_developer_mode_audit (id DESC);
        "#,
    )?;
    Ok(())
}

/// Trusted publisher keys and their provenance.
///
/// Public key bytes, not secrets: a publisher key verifies signatures and cannot make them, so
/// SQLite is the right home and the credential store is not involved. The rule that raw secrets
/// never reach SQLite is untouched because no secret exists on this path.
///
/// Keyed by fingerprint, which is derived from the key bytes. Two rows can therefore never
/// disagree about which key a fingerprint names, and a lookup by the fingerprint an envelope
/// declares hits at most one row. `key_material` is stored as base64 rather than a blob so the
/// row is readable in a database browser during an incident, which is when someone is most likely
/// to need it and least able to write a decoder.
pub(crate) fn apply_publisher_key_schema(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS extension_platform_publisher_keys (
            fingerprint TEXT PRIMARY KEY,
            publisher TEXT NOT NULL,
            key_material TEXT NOT NULL,
            label TEXT NOT NULL,
            source TEXT NOT NULL,
            trust_state TEXT NOT NULL,
            first_seen_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL,
            revoked_at TEXT,
            revocation_reason TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_extension_platform_publisher_keys_publisher
            ON extension_platform_publisher_keys (publisher, fingerprint);
        "#,
    )?;
    Ok(())
}

/// Records that the published gate set went stale.
///
/// Separate from the mutation audit because a degradation has no feature, no prior/new state, and
/// no revision. Only a stable degradation kind and error code are stored — a storage failure's
/// message can carry a path, and this table is written on exactly the path where that is most
/// likely.
pub(crate) fn apply_feature_gate_degradation_schema(
    conn: &Connection,
) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS extension_platform_feature_gate_degradations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            degradation TEXT NOT NULL,
            code TEXT NOT NULL,
            recorded_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_extension_platform_feature_gate_degradations_recorded
            ON extension_platform_feature_gate_degradations (id DESC);
        "#,
    )?;
    Ok(())
}
