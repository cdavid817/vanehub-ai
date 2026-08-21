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
