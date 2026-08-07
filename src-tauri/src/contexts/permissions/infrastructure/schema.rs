//! Schema for the `permissions` context (migration 44 — renumbered from 42 on merge with main,
//! after `agent-memory-shared-pool` and `retrieval-vector-index` independently claimed 42/43
//! first) — three tables, per design.md's D4
//! correction: a template's rules are a pure function of its name, so no table persists them;
//! `agent_principals.template_name` is the only durable template-related state.

use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_permissions_core_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS agent_principals (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL UNIQUE,
            template_name TEXT NOT NULL DEFAULT 'standard',
            parent_principal_id TEXT,
            budget_config TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS permission_grants (
            id TEXT PRIMARY KEY,
            principal_id TEXT NOT NULL REFERENCES agent_principals(id) ON DELETE CASCADE,
            action TEXT NOT NULL,
            resource TEXT NOT NULL,
            effect TEXT NOT NULL,
            scope TEXT NOT NULL,
            session_id TEXT,
            project_key TEXT,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_permission_grants_lookup
            ON permission_grants(principal_id, action, resource);

        CREATE TABLE IF NOT EXISTS approval_audit (
            id TEXT PRIMARY KEY,
            principal_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            generation_id TEXT NOT NULL,
            action TEXT NOT NULL,
            resource TEXT NOT NULL,
            effect TEXT NOT NULL,
            risk_level TEXT NOT NULL,
            decider TEXT NOT NULL,
            channel TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_approval_audit_principal
            ON approval_audit(principal_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

/// One-time backfill (design.md Migration Plan step 2): every existing `agents` row becomes an
/// `agent_principals` row, `template_name = 'trusted'` if its legacy `auto_approve_tools` flag
/// was set, `'standard'` otherwise — the acceptance criterion this migration is judged against
/// (`permissions-core`'s "Legacy per-agent trust flag migrates to an equivalent policy
/// assignment").
pub(crate) fn backfill_principals_from_legacy_trust_flag(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    let now = crate::platform::clock::SystemClock.rfc3339();
    connection.execute(
        r#"
        INSERT INTO agent_principals (id, agent_id, template_name, created_at, updated_at)
        SELECT
            'principal-migrated-' || id,
            id,
            CASE WHEN auto_approve_tools = 1 THEN 'trusted' ELSE 'standard' END,
            ?1,
            ?1
        FROM agents
        WHERE id NOT IN (SELECT agent_id FROM agent_principals)
        "#,
        [now],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE agents (
                    id TEXT PRIMARY KEY,
                    auto_approve_tools INTEGER NOT NULL DEFAULT 0
                );
                INSERT INTO agents (id, auto_approve_tools) VALUES ('agent-trusted', 1);
                INSERT INTO agents (id, auto_approve_tools) VALUES ('agent-untrusted', 0);
                "#,
            )
            .expect("seed agents table");
        connection
    }

    #[test]
    fn schema_is_idempotent() {
        let connection = seeded_connection();
        apply_permissions_core_schema(&connection).expect("first apply");
        apply_permissions_core_schema(&connection).expect("second apply");

        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN \
                 ('agent_principals', 'permission_grants', 'approval_audit')",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(table_count, 3);
    }

    #[test]
    fn backfill_maps_trust_flag_to_template_name() {
        let connection = seeded_connection();
        apply_permissions_core_schema(&connection).expect("schema");
        backfill_principals_from_legacy_trust_flag(&connection).expect("backfill");

        let trusted_template: String = connection
            .query_row(
                "SELECT template_name FROM agent_principals WHERE agent_id = 'agent-trusted'",
                [],
                |row| row.get(0),
            )
            .expect("trusted principal");
        assert_eq!(trusted_template, "trusted");

        let untrusted_template: String = connection
            .query_row(
                "SELECT template_name FROM agent_principals WHERE agent_id = 'agent-untrusted'",
                [],
                |row| row.get(0),
            )
            .expect("untrusted principal");
        assert_eq!(untrusted_template, "standard");
    }

    #[test]
    fn backfill_is_safe_to_run_more_than_once() {
        let connection = seeded_connection();
        apply_permissions_core_schema(&connection).expect("schema");
        backfill_principals_from_legacy_trust_flag(&connection).expect("first backfill");
        backfill_principals_from_legacy_trust_flag(&connection).expect("second backfill");

        let principal_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM agent_principals", [], |row| {
                row.get(0)
            })
            .expect("principal count");
        assert_eq!(principal_count, 2);
    }
}
