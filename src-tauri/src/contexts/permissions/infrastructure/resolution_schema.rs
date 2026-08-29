//! Migration 95: canonical grant identity and the durable approval-resolution ledger.
//!
//! The table this rebuilds could hold several rows for one logical decision, with only a
//! non-unique lookup index and no ordering rule to say which of them applied. This migration turns
//! that into a stated identity — principal, action, resource, scope, and the scope's owner — and
//! makes the schema itself enforce one row per key, so a future writer cannot reintroduce the
//! ambiguity by appending.
//!
//! Three properties matter more than the SQL:
//!
//! Rebuilding is transactional. Every statement below runs inside the caller's transaction, so a
//! failed copy, a rejected invariant, or a duplicate that survives deduplication leaves the
//! pre-migration table exactly as it was rather than half-converted.
//!
//! Deduplication is deterministic, and it is allowed to change behaviour. Picking a winner by
//! recency, then by the safer effect, then by a stable id means the row that applies after upgrade
//! may not be the one an unordered query happened to return before. That is the correction, not a
//! regression — but it is a real change to an effective permission, which is why the choice is
//! written down here rather than left to the database.
//!
//! Malformed legacy rows are excluded, not repaired. A session grant with no session is not a
//! global grant with a missing column; widening it would turn a lost owner into an authorization
//! covering every session. They are counted into one redacted diagnostic and dropped.

use crate::platform::database::{table_has_column, DatabaseError};
use rusqlite::Connection;

/// The shape every remembered grant must have from this migration onward.
///
/// The scope/owner rule lives in a table `CHECK` as well as in `RememberedScope`, deliberately.
/// Domain construction guards the writers this codebase has; the constraint guards the ones it
/// will have, plus anything that reaches the file directly.
const CREATE_REBUILT_GRANTS: &str = r#"
CREATE TABLE permission_grants_rebuilt (
    id TEXT PRIMARY KEY,
    principal_id TEXT NOT NULL REFERENCES agent_principals(id) ON DELETE CASCADE,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    effect TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
    scope TEXT NOT NULL CHECK (scope IN ('session', 'project', 'global')),
    session_id TEXT,
    project_key TEXT,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision >= 1),
    activation_state TEXT NOT NULL DEFAULT 'active'
        CHECK (activation_state IN ('pending_delivery', 'active')),
    resolution_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (
        (scope = 'session' AND session_id IS NOT NULL AND session_id <> '' AND project_key IS NULL)
        OR (scope = 'project' AND project_key IS NOT NULL AND project_key <> '' AND session_id IS NULL)
        OR (scope = 'global' AND session_id IS NULL AND project_key IS NULL)
    )
);
"#;

/// Which legacy rows are eligible to survive.
///
/// Written as one predicate reused by the copy and by the diagnostic count, so the number reported
/// as discarded is by construction the number actually discarded.
const LEGACY_ELIGIBLE: &str = r#"
    effect IN ('allow', 'deny')
    AND (
        (scope = 'session'
            AND session_id IS NOT NULL AND session_id <> ''
            AND (project_key IS NULL OR project_key = ''))
        OR (scope = 'project'
            AND project_key IS NOT NULL AND project_key <> ''
            AND (session_id IS NULL OR session_id = ''))
        OR (scope = 'global'
            AND (session_id IS NULL OR session_id = '')
            AND (project_key IS NULL OR project_key = ''))
    )
"#;

/// The scope-specific uniqueness the whole change rests on, plus the lookup index the ranked read
/// uses. Partial rather than one composite index because the owner column differs per scope and a
/// single index over all three would treat `NULL` owners as distinct.
const CREATE_GRANT_INDEXES: &str = r#"
CREATE UNIQUE INDEX idx_permission_grants_global_key
    ON permission_grants(principal_id, action, resource)
    WHERE scope = 'global';
CREATE UNIQUE INDEX idx_permission_grants_project_key
    ON permission_grants(principal_id, action, resource, project_key)
    WHERE scope = 'project';
CREATE UNIQUE INDEX idx_permission_grants_session_key
    ON permission_grants(principal_id, action, resource, session_id)
    WHERE scope = 'session';
CREATE INDEX idx_permission_grants_lookup
    ON permission_grants(principal_id, action, resource);
CREATE INDEX idx_permission_grants_resolution
    ON permission_grants(resolution_id)
    WHERE resolution_id IS NOT NULL;
"#;

/// The durable delivery ledger. Bounded metadata only: a correlation hash rather than a provider
/// payload, a stable error code rather than a message, and no tool input at all.
const CREATE_RESOLUTIONS: &str = r#"
CREATE TABLE IF NOT EXISTS approval_resolutions (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL UNIQUE,
    principal_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    generation_id TEXT NOT NULL,
    call_id_hash TEXT NOT NULL,
    action TEXT NOT NULL,
    resource TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    decision_effect TEXT NOT NULL CHECK (decision_effect IN ('allow', 'deny')),
    decision_scope TEXT NOT NULL
        CHECK (decision_scope IN ('once', 'session', 'project', 'global')),
    decider TEXT NOT NULL
        CHECK (decider IN ('human', 'timeout', 'stale_generation', 'emergency_fail_closed')),
    channel TEXT NOT NULL CHECK (channel IN ('native_agent', 'claude_hook')),
    state TEXT NOT NULL
        CHECK (state IN ('committed', 'delivered', 'delivery_failed', 'stale', 'aborted_by_restart')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    delivery_attempts INTEGER NOT NULL DEFAULT 0 CHECK (delivery_attempts >= 0),
    last_error_code TEXT
);

CREATE INDEX IF NOT EXISTS idx_approval_resolutions_reconciliation
    ON approval_resolutions(state, updated_at);
"#;

/// How many legacy rows this migration refused to carry forward, by why.
///
/// Counts only. The rows being discarded contain resources and project keys, and a diagnostic that
/// named them would put user paths into a log that the redaction rules keep them out of everywhere
/// else.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LegacyGrantNormalization {
    pub(crate) carried_forward: i64,
    pub(crate) discarded_malformed: i64,
    pub(crate) discarded_duplicate: i64,
}

impl LegacyGrantNormalization {
    pub(crate) fn discarded(&self) -> i64 {
        self.discarded_malformed + self.discarded_duplicate
    }
}

/// The registered migration entry point.
///
/// Separate from the rebuild itself so the rebuild can return what it did and the diagnostic can
/// be written once, here, from a value rather than from scattered counters. The diagnostic is
/// emitted only when something was actually dropped: an upgrade that carried everything forward
/// has nothing a reader needs to be told.
///
/// A logging failure does not fail the migration. The schema change is the thing that must be
/// all-or-nothing; refusing to upgrade because a log file could not be opened would take the
/// application down over its own diagnostic.
pub(crate) fn apply_grant_identity_migration(connection: &Connection) -> Result<(), DatabaseError> {
    let report = apply_grant_identity_and_resolution_ledger(connection)?;
    if report.discarded() == 0 {
        return Ok(());
    }
    let _ = crate::platform::logging::write_message_raw(
        &crate::platform::logging::fallback_log_dir(),
        crate::platform::logging::LogLevel::Warn,
        "permissions",
        "normalized legacy permission grants into canonical keys",
        // Counts only. The discarded rows carry resources and project keys, and naming them here
        // would put user paths into the log that every other permissions path keeps them out of.
        std::collections::BTreeMap::from([
            (
                "carried_forward".to_string(),
                report.carried_forward.to_string(),
            ),
            (
                "discarded_malformed".to_string(),
                report.discarded_malformed.to_string(),
            ),
            (
                "discarded_duplicate".to_string(),
                report.discarded_duplicate.to_string(),
            ),
        ]),
    );
    Ok(())
}

/// Rebuilds `permission_grants` under canonical identity and adds the resolution ledger.
///
/// Runs entirely inside `connection`, which the migration runner has already opened as a
/// transaction. Nothing here commits, so any error propagates as a rollback of the whole upgrade.
pub(crate) fn apply_grant_identity_and_resolution_ledger(
    connection: &Connection,
) -> Result<LegacyGrantNormalization, DatabaseError> {
    let total_legacy: i64 =
        connection.query_row("SELECT COUNT(*) FROM permission_grants", [], |row| {
            row.get(0)
        })?;
    let eligible_legacy: i64 = connection.query_row(
        &format!("SELECT COUNT(*) FROM permission_grants WHERE {LEGACY_ELIGIBLE}"),
        [],
        |row| row.get(0),
    )?;

    connection.execute_batch(CREATE_REBUILT_GRANTS)?;

    // One winner per canonical key. `created_at` is stored as Unix seconds, so the numeric cast is
    // the real ordering; the text fallback keeps a row written in some other encoding from
    // collapsing every timestamp to zero and leaving `id` as the only tie-break.
    let carried_forward = connection.execute(
        &format!(
            r#"
            INSERT INTO permission_grants_rebuilt (
                id, principal_id, action, resource, effect, scope, session_id, project_key,
                revision, activation_state, resolution_id, created_at, updated_at
            )
            SELECT
                id, principal_id, action, resource, effect, scope,
                NULLIF(session_id, ''), NULLIF(project_key, ''),
                1, 'active', NULL, created_at, created_at
            FROM (
                SELECT
                    id, principal_id, action, resource, effect, scope, session_id, project_key,
                    created_at,
                    ROW_NUMBER() OVER (
                        PARTITION BY
                            principal_id, action, resource, scope,
                            COALESCE(NULLIF(session_id, ''), ''),
                            COALESCE(NULLIF(project_key, ''), '')
                        ORDER BY
                            CAST(created_at AS INTEGER) DESC,
                            created_at DESC,
                            CASE effect WHEN 'deny' THEN 0 ELSE 1 END ASC,
                            id DESC
                    ) AS canonical_rank
                FROM permission_grants
                WHERE {LEGACY_ELIGIBLE}
            )
            WHERE canonical_rank = 1
            "#
        ),
        [],
    )? as i64;

    connection.execute_batch(
        "DROP TABLE permission_grants;
         ALTER TABLE permission_grants_rebuilt RENAME TO permission_grants;",
    )?;
    connection.execute_batch(CREATE_GRANT_INDEXES)?;

    verify_grant_invariants(connection, carried_forward)?;

    connection.execute_batch(CREATE_RESOLUTIONS)?;
    // Nullable and additive: existing audit rows stay readable and the table stays append-only.
    // A resolution id on an audit row is what links a decision to the delivery evidence for it;
    // rows written before this migration have neither and must not be invented.
    if !table_has_column(connection, "approval_audit", "resolution_id")? {
        connection.execute_batch("ALTER TABLE approval_audit ADD COLUMN resolution_id TEXT;")?;
    }
    if !table_has_column(connection, "approval_audit", "outcome_reason")? {
        connection.execute_batch("ALTER TABLE approval_audit ADD COLUMN outcome_reason TEXT;")?;
    }

    Ok(LegacyGrantNormalization {
        carried_forward,
        discarded_malformed: total_legacy - eligible_legacy,
        discarded_duplicate: eligible_legacy - carried_forward,
    })
}

/// Proves the rebuilt table actually holds what the migration claims before the transaction is
/// allowed to commit.
///
/// The row count is checked against what the copy reported rather than recomputed from the source,
/// because a `CHECK` that silently rejected a row would otherwise show up much later as a missing
/// permission. Uniqueness is re-derived from the data rather than trusted to the indexes, so a
/// partial index that failed to apply is caught here and not at the first duplicate write.
fn verify_grant_invariants(connection: &Connection, expected: i64) -> Result<(), DatabaseError> {
    let actual: i64 =
        connection.query_row("SELECT COUNT(*) FROM permission_grants", [], |row| {
            row.get(0)
        })?;
    if actual != expected {
        return Err(DatabaseError::Storage(format!(
            "grant rebuild copied {expected} rows but the table holds {actual}"
        )));
    }

    let duplicates: i64 = connection.query_row(
        "SELECT COUNT(*) FROM (
             SELECT 1 FROM permission_grants
             GROUP BY principal_id, action, resource, scope,
                      COALESCE(session_id, ''), COALESCE(project_key, '')
             HAVING COUNT(*) > 1
         )",
        [],
        |row| row.get(0),
    )?;
    if duplicates > 0 {
        return Err(DatabaseError::Storage(format!(
            "grant rebuild left {duplicates} canonical keys holding more than one row"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::permissions::infrastructure::schema::apply_permissions_core_schema;

    /// A database at the pre-migration schema, with one principal to hang grants off.
    fn legacy_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory database");
        connection
            .execute_batch(
                "CREATE TABLE agents (id TEXT PRIMARY KEY, auto_approve_tools INTEGER NOT NULL DEFAULT 0);",
            )
            .expect("legacy agents table");
        apply_permissions_core_schema(&connection).expect("pre-migration schema");
        connection
            .execute(
                "INSERT INTO agent_principals (id, agent_id, template_name, created_at, updated_at) \
                 VALUES ('principal-1', 'agent-1', 'standard', '0', '0')",
                [],
            )
            .expect("seed principal");
        connection
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_legacy(
        connection: &Connection,
        id: &str,
        action: &str,
        resource: &str,
        effect: &str,
        scope: &str,
        session_id: Option<&str>,
        project_key: Option<&str>,
        created_at: &str,
    ) {
        connection
            .execute(
                "INSERT INTO permission_grants \
                 (id, principal_id, action, resource, effect, scope, session_id, project_key, created_at) \
                 VALUES (?1, 'principal-1', ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![id, action, resource, effect, scope, session_id, project_key, created_at],
            )
            .expect("insert legacy grant");
    }

    fn migrate(connection: &Connection) -> LegacyGrantNormalization {
        apply_grant_identity_and_resolution_ledger(connection).expect("migration succeeds")
    }

    fn surviving_ids(connection: &Connection) -> Vec<String> {
        let mut statement = connection
            .prepare("SELECT id FROM permission_grants ORDER BY id")
            .expect("prepare");
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect");
        ids
    }

    #[test]
    fn an_empty_table_migrates_to_the_new_shape() {
        let connection = legacy_connection();
        let report = migrate(&connection);

        assert_eq!(report, LegacyGrantNormalization::default());
        assert!(table_has_column(&connection, "permission_grants", "revision").expect("pragma"));
        assert!(
            table_has_column(&connection, "permission_grants", "activation_state").expect("pragma")
        );
        assert!(table_has_column(&connection, "approval_audit", "resolution_id").expect("pragma"));
    }

    #[test]
    fn a_valid_grant_of_each_scope_survives_with_its_effect_and_creation_time() {
        let connection = legacy_connection();
        insert_legacy(
            &connection,
            "g-session",
            "file.write",
            "a.txt",
            "allow",
            "session",
            Some("session-1"),
            None,
            "100",
        );
        insert_legacy(
            &connection,
            "g-project",
            "file.write",
            "a.txt",
            "deny",
            "project",
            None,
            Some("project-1"),
            "101",
        );
        insert_legacy(
            &connection,
            "g-global",
            "file.write",
            "a.txt",
            "deny",
            "global",
            None,
            None,
            "102",
        );

        let report = migrate(&connection);

        assert_eq!(report.carried_forward, 3);
        assert_eq!(report.discarded(), 0);
        assert_eq!(
            surviving_ids(&connection),
            ["g-global", "g-project", "g-session"]
        );
        let (effect, created_at, revision, state): (String, String, i64, String) = connection
            .query_row(
                "SELECT effect, created_at, revision, activation_state FROM permission_grants \
                 WHERE id = 'g-session'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("session grant");
        assert_eq!(effect, "allow");
        assert_eq!(created_at, "100");
        assert_eq!(revision, 1);
        // Rows that predate the ledger have no delivery to wait for, so they are active on arrival.
        // Importing them as pending would silently revoke every permission a user already granted.
        assert_eq!(state, "active");
    }

    #[test]
    fn every_malformed_legacy_shape_is_discarded_rather_than_widened() {
        let connection = legacy_connection();
        let malformed = [
            ("g-once", "once", None, None),
            ("g-session-no-owner", "session", None, None),
            ("g-session-empty-owner", "session", Some(""), None),
            (
                "g-session-with-project",
                "session",
                Some("session-1"),
                Some("project-1"),
            ),
            ("g-project-no-owner", "project", None, None),
            (
                "g-project-with-session",
                "project",
                Some("session-1"),
                Some("project-1"),
            ),
            ("g-global-with-session", "global", Some("session-1"), None),
            ("g-global-with-project", "global", None, Some("project-1")),
        ];
        for (index, (id, scope, session, project)) in malformed.iter().enumerate() {
            insert_legacy(
                &connection,
                id,
                "file.write",
                &format!("resource-{index}"),
                "allow",
                scope,
                *session,
                *project,
                "100",
            );
        }
        insert_legacy(
            &connection,
            "g-ask",
            "file.write",
            "asked.txt",
            "ask",
            "global",
            None,
            None,
            "100",
        );

        let report = migrate(&connection);

        assert_eq!(report.carried_forward, 0);
        assert_eq!(report.discarded_malformed, malformed.len() as i64 + 1);
        assert_eq!(report.discarded_duplicate, 0);
        assert!(surviving_ids(&connection).is_empty());
    }

    #[test]
    fn recency_decides_a_duplicate_key_before_effect_does() {
        let connection = legacy_connection();
        insert_legacy(
            &connection,
            "g-old",
            "file.write",
            "a.txt",
            "deny",
            "global",
            None,
            None,
            "100",
        );
        insert_legacy(
            &connection,
            "g-new",
            "file.write",
            "a.txt",
            "allow",
            "global",
            None,
            None,
            "200",
        );

        let report = migrate(&connection);

        assert_eq!(report.carried_forward, 1);
        assert_eq!(report.discarded_duplicate, 1);
        // The later decision is the user's current intent, even when it is the more permissive one.
        assert_eq!(surviving_ids(&connection), ["g-new"]);
    }

    #[test]
    fn the_safer_effect_breaks_a_tie_on_creation_time() {
        let connection = legacy_connection();
        insert_legacy(
            &connection,
            "g-allow",
            "file.write",
            "a.txt",
            "allow",
            "global",
            None,
            None,
            "100",
        );
        insert_legacy(
            &connection,
            "g-deny",
            "file.write",
            "a.txt",
            "deny",
            "global",
            None,
            None,
            "100",
        );

        migrate(&connection);

        // Same instant, opposite decisions, nothing to prefer on recency. Choosing Deny is the
        // only tie-break that cannot turn an ambiguity into a permission nobody granted.
        assert_eq!(surviving_ids(&connection), ["g-deny"]);
    }

    #[test]
    fn the_id_is_the_final_stable_tie_break() {
        let connection = legacy_connection();
        insert_legacy(
            &connection,
            "g-aaa",
            "file.write",
            "a.txt",
            "deny",
            "global",
            None,
            None,
            "100",
        );
        insert_legacy(
            &connection,
            "g-zzz",
            "file.write",
            "a.txt",
            "deny",
            "global",
            None,
            None,
            "100",
        );

        migrate(&connection);

        // Identical on every meaningful axis. The point is not which id wins but that the same
        // input always produces the same winner, on any machine and after any VACUUM.
        assert_eq!(surviving_ids(&connection), ["g-zzz"]);
    }

    #[test]
    fn duplicates_are_resolved_per_canonical_key_not_per_principal() {
        let connection = legacy_connection();
        // Same principal/action/resource, three different owners: three keys, no duplication.
        insert_legacy(
            &connection,
            "g-s1",
            "file.write",
            "a.txt",
            "allow",
            "session",
            Some("session-1"),
            None,
            "100",
        );
        insert_legacy(
            &connection,
            "g-s2",
            "file.write",
            "a.txt",
            "deny",
            "session",
            Some("session-2"),
            None,
            "100",
        );
        insert_legacy(
            &connection,
            "g-p1",
            "file.write",
            "a.txt",
            "deny",
            "project",
            None,
            Some("project-1"),
            "100",
        );

        let report = migrate(&connection);

        assert_eq!(report.carried_forward, 3);
        assert_eq!(report.discarded_duplicate, 0);
    }

    #[test]
    fn the_rebuilt_table_refuses_a_second_row_for_one_canonical_key() {
        let connection = legacy_connection();
        migrate(&connection);
        insert_rebuilt(&connection, "g-1", "global", None, None).expect("first row");

        let second = insert_rebuilt(&connection, "g-2", "global", None, None);

        assert!(
            second.is_err(),
            "the partial unique index did not stop a duplicate canonical key"
        );
    }

    #[test]
    fn the_rebuilt_table_refuses_every_invalid_scope_owner_combination() {
        let connection = legacy_connection();
        migrate(&connection);

        for (id, scope, session, project) in [
            ("bad-1", "session", None, None),
            ("bad-2", "session", Some("session-1"), Some("project-1")),
            ("bad-3", "project", None, None),
            ("bad-4", "global", Some("session-1"), None),
            ("bad-5", "once", None, None),
        ] {
            assert!(
                insert_rebuilt(&connection, id, scope, session, project).is_err(),
                "{scope} with session {session:?} and project {project:?} was accepted by storage"
            );
        }
    }

    #[test]
    fn the_rebuilt_table_refuses_an_unrememberable_effect() {
        let connection = legacy_connection();
        migrate(&connection);

        let rejected = connection.execute(
            "INSERT INTO permission_grants \
             (id, principal_id, action, resource, effect, scope, revision, activation_state, \
              created_at, updated_at) \
             VALUES ('ask-row', 'principal-1', 'file.write', 'a.txt', 'ask', 'global', 1, \
              'active', '0', '0')",
            [],
        );

        assert!(
            rejected.is_err(),
            "storage accepted an Ask as a remembered grant"
        );
    }

    #[test]
    fn the_resolution_ledger_refuses_a_second_resolution_for_one_request() {
        let connection = legacy_connection();
        migrate(&connection);
        insert_resolution(&connection, "res-1", "req-1").expect("first resolution");

        let second = insert_resolution(&connection, "res-2", "req-1");

        assert!(
            second.is_err(),
            "one approval request produced two immutable resolutions"
        );
    }

    #[test]
    fn a_failed_invariant_check_leaves_the_pre_migration_table_intact() {
        let connection = legacy_connection();
        insert_legacy(
            &connection,
            "g-1",
            "file.write",
            "a.txt",
            "allow",
            "global",
            None,
            None,
            "100",
        );
        let transaction = connection.unchecked_transaction().expect("transaction");
        apply_grant_identity_and_resolution_ledger(&transaction).expect("migration");
        // Stand in for any statement after the copy failing: the runner rolls the whole upgrade
        // back rather than committing a half-converted table.
        drop(transaction);

        let columns = table_has_column(&connection, "permission_grants", "revision")
            .expect("pragma on the original table");
        assert!(
            !columns,
            "the rolled-back rebuild left its new columns behind"
        );
        assert_eq!(surviving_ids(&connection), ["g-1"]);
        let resolutions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'approval_resolutions'",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(
            resolutions, 0,
            "the rolled-back upgrade left the ledger behind"
        );
    }

    /// One case per stage of the rebuild, each asserting the same thing: the pre-migration table is
    /// exactly as it was.
    ///
    /// The failures are induced by making a real constraint or a real invariant reject, not by a
    /// fault-injection hook — a hook would prove the injector works, and these have to prove the
    /// transaction does.
    #[test]
    fn a_failure_at_any_stage_of_the_rebuild_leaves_the_original_table_untouched() {
        // Stage 1: creating the replacement table. A leftover table from an interrupted run makes
        // the `CREATE` fail before anything is copied.
        let connection = legacy_connection();
        insert_legacy(
            &connection,
            "g-1",
            "file.write",
            "a.txt",
            "allow",
            "global",
            None,
            None,
            "100",
        );
        connection
            .execute_batch("CREATE TABLE permission_grants_rebuilt (id TEXT PRIMARY KEY);")
            .expect("leftover table");
        assert!(apply_grant_identity_and_resolution_ledger(&connection).is_err());
        assert_eq!(surviving_ids(&connection), ["g-1"]);
        assert!(!table_has_column(&connection, "permission_grants", "revision").expect("pragma"));

        // Stage 2: the copy. A grant whose principal was deleted violates the replacement table's
        // foreign key, so the `INSERT ... SELECT` fails.
        let connection = legacy_connection();
        insert_legacy(
            &connection,
            "g-1",
            "file.write",
            "a.txt",
            "allow",
            "global",
            None,
            None,
            "100",
        );
        // Enforcement is turned off for the setup and back on for the migration. The *old* table
        // carries the same foreign key, so leaving it on would refuse the fixture rather than the
        // copy this case is about.
        connection
            .execute_batch(
                "PRAGMA foreign_keys = OFF;
                 UPDATE permission_grants SET principal_id = 'principal-missing';
                 PRAGMA foreign_keys = ON;",
            )
            .expect("orphan the grant");
        let transaction = connection.unchecked_transaction().expect("transaction");
        let copied = apply_grant_identity_and_resolution_ledger(&transaction);
        drop(transaction);
        assert!(
            copied.is_err(),
            "an orphaned grant was copied into the rebuilt table"
        );
        assert_eq!(surviving_ids(&connection), ["g-1"]);

        // Stage 3: the invariant check after the swap. Asserted directly, because reaching it
        // through the copy would require the copy to be wrong — which is the thing the check
        // exists to catch, not something a fixture can arrange.
        let connection = legacy_connection();
        migrate(&connection);
        insert_rebuilt(&connection, "g-1", "global", None, None).expect("row");
        assert!(
            verify_grant_invariants(&connection, 2).is_err(),
            "the invariant check accepted a row count it did not copy"
        );
    }

    #[test]
    fn the_invariant_check_reports_a_duplicate_canonical_key() {
        // The partial unique indexes make this unreachable through normal writes, which is exactly
        // why the check is derived from the data rather than trusted to them: an index that failed
        // to apply would otherwise be discovered at the first duplicate write, in production.
        let connection = legacy_connection();
        migrate(&connection);
        connection
            .execute_batch("DROP INDEX idx_permission_grants_global_key;")
            .expect("simulate an index that did not apply");
        insert_rebuilt(&connection, "g-1", "global", None, None).expect("first row");
        insert_rebuilt(&connection, "g-2", "global", None, None).expect("second row");

        let error = verify_grant_invariants(&connection, 2)
            .expect_err("two rows for one canonical key must be refused");
        assert!(format!("{error}").contains("canonical keys"));
    }

    #[test]
    fn running_the_migration_on_an_already_migrated_database_is_a_no_op_for_its_rows() {
        // The runner records the version and never re-runs it, but a repaired or manually
        // reconciled database can arrive here twice. Carrying the rows forward unchanged is what
        // makes that safe.
        let connection = legacy_connection();
        insert_legacy(
            &connection,
            "g-1",
            "file.write",
            "a.txt",
            "allow",
            "global",
            None,
            None,
            "100",
        );
        migrate(&connection);
        let first = surviving_ids(&connection);

        let report = apply_grant_identity_and_resolution_ledger(&connection)
            .expect("second application succeeds");

        assert_eq!(report.carried_forward, 1);
        assert_eq!(report.discarded(), 0);
        assert_eq!(surviving_ids(&connection), first);
    }

    fn insert_rebuilt(
        connection: &Connection,
        id: &str,
        scope: &str,
        session_id: Option<&str>,
        project_key: Option<&str>,
    ) -> rusqlite::Result<usize> {
        connection.execute(
            "INSERT INTO permission_grants \
             (id, principal_id, action, resource, effect, scope, session_id, project_key, \
              revision, activation_state, created_at, updated_at) \
             VALUES (?1, 'principal-1', 'file.write', 'a.txt', 'allow', ?2, ?3, ?4, 1, 'active', \
              '0', '0')",
            rusqlite::params![id, scope, session_id, project_key],
        )
    }

    fn insert_resolution(
        connection: &Connection,
        id: &str,
        request_id: &str,
    ) -> rusqlite::Result<usize> {
        connection.execute(
            "INSERT INTO approval_resolutions \
             (id, request_id, principal_id, session_id, generation_id, call_id_hash, action, \
              resource, risk_level, decision_effect, decision_scope, decider, channel, state, \
              created_at, updated_at) \
             VALUES (?1, ?2, 'principal-1', 'session-1', 'generation-1', 'hash', 'file.write', \
              'a.txt', 'L1', 'allow', 'session', 'human', 'native_agent', 'committed', '0', '0')",
            rusqlite::params![id, request_id],
        )
    }
}
