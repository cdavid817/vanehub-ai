//! Additive schema for source-aware CLI environments.
//!
//! Three tables, all new. `cli_tool_status` is deliberately left alone: it stays readable so a
//! first run after upgrading can map an old row into a stale snapshot, and dropping it would make
//! that impossible while gaining nothing.
//!
//! Every payload is a versioned JSON document rather than a wide column set. The snapshot shape is
//! still moving, and a schema migration per field would be a migration per field; the schema
//! version inside the document is what makes decoding fallible instead of silently wrong.

use rusqlite::Connection;

use crate::platform::database::DatabaseError;

/// Versioned environment snapshots, one per (tool, scope).
pub(crate) fn apply_environment_snapshot_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS cli_environment_snapshots (
            agent_id TEXT NOT NULL,
            scope_id TEXT NOT NULL,
            schema_version INTEGER NOT NULL,
            environment_fingerprint TEXT NOT NULL,
            snapshot_json TEXT NOT NULL,
            checked_at TEXT,
            last_operation_id TEXT,
            PRIMARY KEY (agent_id, scope_id)
        );
        "#,
    )?;
    Ok(())
}

/// Per-source version catalogs. Keyed by source and channel so one tool's npm and WinGet catalogs
/// can never overwrite each other -- the storage shape enforces what the domain requires.
pub(crate) fn apply_version_catalog_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS cli_version_catalogs (
            agent_id TEXT NOT NULL,
            scope_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            channel TEXT NOT NULL DEFAULT '',
            catalog_json TEXT NOT NULL,
            fetched_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            PRIMARY KEY (agent_id, scope_id, source_id, channel)
        );

        CREATE INDEX IF NOT EXISTS idx_cli_version_catalogs_expiry
            ON cli_version_catalogs(expires_at);
        "#,
    )?;
    Ok(())
}

/// Single-use action plans, including bulk plans and the item plans they own.
pub(crate) fn apply_action_plan_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS cli_action_plans (
            plan_id TEXT PRIMARY KEY,
            plan_kind TEXT NOT NULL CHECK(plan_kind IN ('action','bulk')),
            agent_id TEXT,
            scope_id TEXT NOT NULL,
            revision INTEGER NOT NULL,
            state TEXT NOT NULL CHECK(
                state IN ('draft','executing','completed','failed','cancelled','expired')
            ),
            environment_fingerprint TEXT NOT NULL,
            plan_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            started_at TEXT,
            completed_at TEXT,
            operation_id TEXT,
            bulk_plan_id TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_cli_action_plans_expiry
            ON cli_action_plans(state, expires_at);

        CREATE INDEX IF NOT EXISTS idx_cli_action_plans_agent_state
            ON cli_action_plans(agent_id, state);
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "environment_schema_tests.rs"]
mod tests;
