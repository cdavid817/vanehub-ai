use crate::platform::database::{table_has_column, DatabaseError};
use rusqlite::Connection;

/// Where a hunk decision and a file's Viewed state live.
///
/// Both are additions to the review aggregate that migration 75 established, and both are kept in
/// their own tables rather than as columns on `review_files`. The reason is the thing this change
/// is trying to fix: a review decision and a hunk decision are independent, and storing one beside
/// the other invites the code that writes one to write the other "while it is there". They are
/// separate rows because they are separate facts.
///
/// Every row carries the snapshot fingerprint it was recorded against. Without it a decision is a
/// claim about a diff nobody can identify any more — the file has been edited since, the hunk it
/// named may not exist, and the row would still read as a current decision. With it, 13.3 can
/// refuse a stale write and 13.5 can tell a file that was viewed from a file that was viewed
/// before it changed.
///
/// Additive and idempotent. Every statement is `IF NOT EXISTS`, nothing existing is altered, and
/// no row is synthesised: a database that has been reviewed for months arrives with no decisions
/// and no Viewed state, which is the truth — nothing recorded them before this.
pub(crate) fn apply_review_decision_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS review_hunk_decisions (
            review_id TEXT NOT NULL REFERENCES review_sessions(id) ON DELETE CASCADE,
            relative_path TEXT NOT NULL,
            hunk_fingerprint TEXT NOT NULL,
            snapshot_fingerprint TEXT NOT NULL,
            decision TEXT NOT NULL CHECK(
                decision IN ('pending', 'accepted', 'changes_requested')
            ),
            decided_at TEXT NOT NULL,
            PRIMARY KEY (review_id, relative_path, hunk_fingerprint)
        );

        -- Every decision recorded against one snapshot. This is what a header count reads and what
        -- a staleness check sweeps; the primary key answers "this hunk" and cannot answer "this
        -- review, as of now" without scanning the review's whole decision set.
        CREATE INDEX IF NOT EXISTS idx_review_hunk_decisions_snapshot
            ON review_hunk_decisions(review_id, snapshot_fingerprint);

        CREATE TABLE IF NOT EXISTS review_file_states (
            review_id TEXT NOT NULL REFERENCES review_sessions(id) ON DELETE CASCADE,
            relative_path TEXT NOT NULL,
            snapshot_fingerprint TEXT NOT NULL,
            viewed INTEGER NOT NULL CHECK(viewed IN (0, 1)),
            viewed_at TEXT,
            PRIMARY KEY (review_id, relative_path),
            -- A time for something that is not true is not a smaller truth, it is a wrong one. A
            -- file that is not viewed has no moment at which it was, and a row carrying one would
            -- let a later reader render "viewed 10 minutes ago" beside an unviewed file.
            CHECK (
                (viewed = 1 AND viewed_at IS NOT NULL)
                OR (viewed = 0 AND viewed_at IS NULL)
            )
        );

        -- "viewed files out of the files that are currently changed" is one query over this index.
        -- The snapshot column comes before `viewed` because the count is always scoped to the
        -- current snapshot first: rows witnessed to an older one are not unviewed, they are about
        -- a different diff.
        CREATE INDEX IF NOT EXISTS idx_review_file_states_progress
            ON review_file_states(review_id, snapshot_fingerprint, viewed);
        "#,
    )?;
    Ok(())
}

/// What decides whether a file's Viewed mark still applies.
///
/// Separate from `snapshot_fingerprint`, and the separation is the whole behaviour. A review's
/// snapshot fingerprint covers every changed file, so it moves whenever *any* of them is written
/// to — witnessing Viewed to it would un-view all twelve files because an agent touched one, which
/// makes "8 files · 4 unviewed" say nothing a reviewer can use. The file's own witness moves only
/// when that file changes.
///
/// The snapshot column stays and stays truthful: it records which review snapshot the reviewer was
/// looking at when they marked the file, which is worth keeping and is not what the reset keys on.
///
/// Nullable because it is added to a table that already exists. Nothing has ever written a row —
/// 13.5 is the first writer — so there is nothing to backfill and no default that would be a
/// guess. A row without one is a row from before this column, and `NULL` says exactly that.
pub(crate) fn apply_review_file_witness_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    if !table_has_column(connection, "review_file_states", "file_witness")? {
        connection.execute(
            "ALTER TABLE review_file_states ADD COLUMN file_witness TEXT",
            [],
        )?;
    }
    connection.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_review_file_states_witness
            ON review_file_states(review_id, file_witness, viewed);",
    )?;
    Ok(())
}

/// Creates the file witness when the version gate skipped its migration, for the same reason the
/// decision schema carries one.
pub(crate) fn repair_missing_review_file_witness(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    // Guarded on the table rather than the column: a database that never got migration 90 has no
    // table to alter, and the decision repair above runs first and creates it.
    if !table_exists(connection, "review_file_states")? {
        return Ok(());
    }
    if table_has_column(connection, "review_file_states", "file_witness")? {
        return Ok(());
    }
    apply_review_file_witness_schema(connection)
}

fn table_exists(connection: &Connection, name: &str) -> Result<bool, DatabaseError> {
    let present: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(present > 0)
}

/// Creates the decision schema when the version gate skipped its migration.
///
/// Parallel worktrees share one application database and `main` already records 83 under a
/// different name, so a developer whose database was migrated by another branch has version 83 in
/// `schema_migrations` and none of these tables. `apply_migration` is version-gated, so the
/// statements above would never run and the first decision anybody recorded would fail on a table
/// that does not exist — with a history that looks complete.
///
/// This rewrites no history. It re-asserts the invariant the skipped migration was supposed to
/// establish, which is the same repair versions 54 and 81 already carry for the same reason.
pub(crate) fn repair_missing_review_decision_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    let present: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'review_hunk_decisions'",
        [],
        |row| row.get(0),
    )?;
    if present > 0 {
        return Ok(());
    }
    apply_review_decision_schema(connection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::database::migrate;
    use rusqlite::params;

    /// A migrated database with one session and one active review to hang decisions off.
    fn reviewed() -> Connection {
        let connection = Connection::open_in_memory().expect("in-memory sqlite");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("enable foreign keys");
        migrate(&connection).expect("migrate");
        connection
            .execute(
                "INSERT INTO agents (id, display_name, provider, launch_kind)                  VALUES ('agent-1', 'Agent', 'Test', 'api')
                 ON CONFLICT(id) DO NOTHING",
                [],
            )
            .expect("seed agent");
        connection
            .execute(
                "INSERT INTO sessions                  (id, title, agent_id, interaction_mode, lifecycle_state, created_at, updated_at)                  VALUES ('session-1', 'Review', 'agent-1', 'chat', 'idle',                  '2026-08-27T00:00:00Z', '2026-08-27T00:00:00Z')",
                [],
            )
            .expect("seed session");
        connection
            .execute(
                "INSERT INTO review_sessions \
                 (id, session_id, workspace_id, base_revision, head_revision, fingerprint, \
                  status, decision, created_at, updated_at) \
                 VALUES ('review-1', 'session-1', 'workspace-1', NULL, NULL, 'snapshot-a', \
                 'active', 'pending', '2026-08-27T00:00:00Z', '2026-08-27T00:00:00Z')",
                [],
            )
            .expect("seed review");
        connection
    }

    fn count(connection: &Connection, kind: &str, name: &str) -> i64 {
        connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = ?1 AND name = ?2",
                params![kind, name],
                |row| row.get(0),
            )
            .expect("sqlite_master lookup")
    }

    fn record_hunk(
        connection: &Connection,
        snapshot: &str,
        decision: &str,
    ) -> rusqlite::Result<()> {
        connection
            .execute(
                "INSERT INTO review_hunk_decisions \
                 (review_id, relative_path, hunk_fingerprint, snapshot_fingerprint, decision, \
                  decided_at) \
                 VALUES ('review-1', 'src/main.rs', 'hunk-1', ?1, ?2, '2026-08-27T00:01:00Z')",
                params![snapshot, decision],
            )
            .map(|_| ())
    }

    #[test]
    fn the_migration_creates_both_decision_tables_and_the_indexes_that_answer_progress() {
        let connection = reviewed();

        for table in ["review_hunk_decisions", "review_file_states"] {
            assert_eq!(count(&connection, "table", table), 1, "{table} must exist");
        }
        for index in [
            "idx_review_hunk_decisions_snapshot",
            "idx_review_file_states_progress",
            "idx_review_file_states_witness",
        ] {
            assert_eq!(count(&connection, "index", index), 1, "{index} must exist");
        }
    }

    // The header reads "viewed files out of the files that changed", once per render of the Review
    // tab. Without the index that is a scan of every decision the review ever recorded.
    // Migration 84's column, and the reason it is not the snapshot one beside it: a review
    // snapshot covers every changed file, so a mark witnessed to it is cleared by an edit to a
    // different file.
    #[test]
    fn a_file_view_row_carries_its_own_witness_beside_the_snapshot_it_was_made_in() {
        let connection = reviewed();
        connection
            .execute(
                "INSERT INTO review_file_states \
                 (review_id, relative_path, snapshot_fingerprint, file_witness, viewed, viewed_at) \
                 VALUES ('review-1', 'src/main.rs', 'snapshot-a', 'file-witness-1', 1, \
                 '2026-08-27T00:00:00Z')",
                [],
            )
            .expect("view row");

        let (snapshot, witness): (String, String) = connection
            .query_row(
                "SELECT snapshot_fingerprint, file_witness FROM review_file_states",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("row");
        assert_eq!(snapshot, "snapshot-a");
        assert_ne!(witness, snapshot);
    }

    // Nullable on purpose: nothing had ever written a row when the column arrived, so there was
    // nothing to backfill and no default that would not be a guess.
    #[test]
    fn a_row_may_predate_the_witness_column() {
        let connection = reviewed();
        connection
            .execute(
                "INSERT INTO review_file_states \
                 (review_id, relative_path, snapshot_fingerprint, viewed, viewed_at) \
                 VALUES ('review-1', 'src/main.rs', 'snapshot-a', 0, NULL)",
                [],
            )
            .expect("row without a witness");
    }

    #[test]
    fn the_witness_repair_restores_a_skipped_migration() {
        let connection = reviewed();
        // The version gate recorded 84 for another branch's migration, so the column never landed.
        connection
            .execute_batch(
                "DROP INDEX idx_review_file_states_witness; \
                 ALTER TABLE review_file_states DROP COLUMN file_witness;",
            )
            .expect("simulate the skipped migration");

        repair_missing_review_file_witness(&connection).expect("repair");
        assert_eq!(
            count(&connection, "index", "idx_review_file_states_witness"),
            1
        );
        // And again over a database that already has it.
        repair_missing_review_file_witness(&connection).expect("second repair");
    }

    #[test]
    fn the_witness_repair_does_nothing_when_the_table_itself_is_missing() {
        let connection = reviewed();
        connection
            .execute_batch("DROP TABLE review_file_states;")
            .expect("simulate a database that never got migration 90");

        // The decision repair creates the table; this one must not fail trying to alter a table
        // that is not there yet, because the two run in that order at startup.
        repair_missing_review_file_witness(&connection).expect("repair with no table");
    }

    #[test]
    fn the_viewed_progress_count_uses_its_index() {
        let connection = reviewed();
        let plan: String = connection
            .query_row(
                "EXPLAIN QUERY PLAN SELECT COUNT(*) FROM review_file_states \
                 WHERE review_id = 'review-1' AND snapshot_fingerprint = 'snapshot-a' \
                 AND viewed = 1",
                [],
                |row| row.get(3),
            )
            .expect("query plan");
        assert!(
            plan.contains("idx_review_file_states_progress"),
            "expected the progress index, got: {plan}"
        );
    }

    #[test]
    fn a_hunk_decision_replaces_the_previous_one_for_the_same_hunk() {
        let connection = reviewed();
        record_hunk(&connection, "snapshot-a", "accepted").expect("first decision");
        connection
            .execute(
                "INSERT INTO review_hunk_decisions \
                 (review_id, relative_path, hunk_fingerprint, snapshot_fingerprint, decision, \
                  decided_at) \
                 VALUES ('review-1', 'src/main.rs', 'hunk-1', 'snapshot-a', 'changes_requested', \
                 '2026-08-27T00:02:00Z') \
                 ON CONFLICT(review_id, relative_path, hunk_fingerprint) DO UPDATE SET \
                 decision = excluded.decision, decided_at = excluded.decided_at",
                [],
            )
            .expect("second decision");

        // One row, not two. A hunk has one decision; a key that allowed a second would leave two
        // answers to the same question with nothing to say which one the reader meant.
        let (rows, decision): (i64, String) = connection
            .query_row(
                "SELECT COUNT(*), MAX(decision) FROM review_hunk_decisions",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("count decisions");
        assert_eq!(rows, 1);
        assert_eq!(decision, "changes_requested");
    }

    #[test]
    fn a_decision_outside_the_three_the_review_uses_is_refused() {
        let connection = reviewed();
        // The same three the review-level decision allows. A hunk that could be 'skipped' while a
        // review could not would be a fourth state the header has no way to count.
        assert!(record_hunk(&connection, "snapshot-a", "maybe").is_err());
        for decision in ["pending", "accepted", "changes_requested"] {
            connection
                .execute("DELETE FROM review_hunk_decisions", [])
                .expect("clear");
            record_hunk(&connection, "snapshot-a", decision).expect("allowed decision");
        }
    }

    #[test]
    fn a_file_that_is_not_viewed_cannot_carry_a_time_at_which_it_was() {
        let connection = reviewed();
        let insert = |viewed: i64, at: Option<&str>| {
            connection.execute(
                "INSERT OR REPLACE INTO review_file_states \
                 (review_id, relative_path, snapshot_fingerprint, viewed, viewed_at) \
                 VALUES ('review-1', 'src/main.rs', 'snapshot-a', ?1, ?2)",
                params![viewed, at],
            )
        };

        assert!(insert(0, Some("2026-08-27T00:03:00Z")).is_err());
        assert!(insert(1, None).is_err());
        insert(1, Some("2026-08-27T00:03:00Z")).expect("viewed with a time");
        insert(0, None).expect("unviewed with no time");
    }

    #[test]
    fn decisions_and_viewed_state_go_away_with_the_review_they_describe() {
        let connection = reviewed();
        record_hunk(&connection, "snapshot-a", "accepted").expect("decision");
        connection
            .execute(
                "INSERT INTO review_file_states \
                 (review_id, relative_path, snapshot_fingerprint, viewed, viewed_at) \
                 VALUES ('review-1', 'src/main.rs', 'snapshot-a', 1, '2026-08-27T00:03:00Z')",
                [],
            )
            .expect("viewed state");

        connection
            .execute("DELETE FROM review_sessions WHERE id = 'review-1'", [])
            .expect("delete review");

        // Retention deletes reviews, and a decision that outlived its review is unreachable data
        // that still counts against every bound. The cascade is what keeps 15.5 from needing a
        // second sweep for rows nothing can reach.
        for table in ["review_hunk_decisions", "review_file_states"] {
            let remaining: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count remaining");
            assert_eq!(remaining, 0, "{table} kept a row for a review that is gone");
        }
    }

    #[test]
    fn the_repair_restores_a_skipped_migration_and_changes_nothing_when_it_did_not_skip() {
        let connection = reviewed();
        record_hunk(&connection, "snapshot-a", "accepted").expect("decision");

        // Running the repair over a database that already has the schema must not touch the rows.
        repair_missing_review_decision_schema(&connection).expect("repair over existing schema");
        let kept: i64 = connection
            .query_row("SELECT COUNT(*) FROM review_hunk_decisions", [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(kept, 1, "the repair must not clear what was recorded");

        // And over the case it exists for: the version gate recorded 83 for another branch's
        // migration, so these tables were never created.
        connection
            .execute_batch("DROP TABLE review_hunk_decisions; DROP TABLE review_file_states;")
            .expect("simulate the skipped migration");
        repair_missing_review_decision_schema(&connection).expect("repair after a skip");
        for table in ["review_hunk_decisions", "review_file_states"] {
            assert_eq!(
                count(&connection, "table", table),
                1,
                "{table} must be restored"
            );
        }
    }
}
