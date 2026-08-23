//! Migration 88: Hook subjects, versioned definitions, user bindings, and execution evidence.
//!
//! Four choices here are load-bearing, and all four are cheaper to get right in the DDL than to
//! defend in every repository method afterwards.
//!
//! **A binding's scope is two `NOT NULL` columns, not one nullable one.** SQLite treats `NULL` as
//! distinct from every other `NULL` in a unique index, so `(hook_global_id, scope)` with `NULL`
//! meaning "global" admits unlimited global bindings for one Hook — each invisible to the others,
//! and whichever a reader happened to see first would decide whether the Hook ran. `scope_kind`
//! plus a `scope_key` that is `''` for global makes the key total, and a `CHECK` keeps the two
//! spellings from diverging.
//!
//! **Bindings and executions reference the *subject*, never a definition revision.** A binding
//! pinned to `(hook, snapshot)` would be orphaned by every upgrade and would vanish while a
//! definition was momentarily unavailable, taking a user's enablement with it.
//!
//! **`snapshot_id` carries no foreign key.** `extension_platform` owns snapshots. An enforced
//! reference would couple two subdomains' storage and let one's deletions reach into the other's
//! evidence; the gap is made up by a read-only reconciliation, not by a constraint.
//!
//! **`ON DELETE RESTRICT` everywhere, `CASCADE` nowhere.** Deleting a subject that still has
//! executions should fail and force whoever is doing it to say what should happen to the evidence.

use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// Hook subjects, definition revisions, bindings, and executions.
pub(crate) fn apply_lifecycle_hook_schema(conn: &Connection) -> Result<(), DatabaseError> {
    apply_subjects(conn)?;
    apply_definition_revisions(conn)?;
    apply_bindings(conn)?;
    apply_executions(conn)?;
    Ok(())
}

/// The stable identity everything else attaches to.
///
/// `first_seen_at` is written once. Re-seeding a built-in is not a new sighting, and rewriting it
/// would erase the only record of when the Hook entered this installation.
fn apply_subjects(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS lifecycle_hook_subjects (
            hook_global_id TEXT PRIMARY KEY,
            origin TEXT NOT NULL CHECK (origin IN ('builtin', 'extension')),
            first_seen_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

/// What one Hook is, in one snapshot. Immutable.
///
/// The primary key is the pair, so a second recording either offers the same digest — idempotent —
/// or a different one, which the repository refuses without touching the stored row.
fn apply_definition_revisions(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS lifecycle_hook_definition_revisions (
            hook_global_id TEXT NOT NULL
                REFERENCES lifecycle_hook_subjects (hook_global_id) ON DELETE RESTRICT,
            snapshot_id TEXT NOT NULL,
            event TEXT NOT NULL,
            definition_digest TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            PRIMARY KEY (hook_global_id, snapshot_id)
        );

        CREATE INDEX IF NOT EXISTS idx_lifecycle_hook_definition_revisions_snapshot
            ON lifecycle_hook_definition_revisions (snapshot_id);
        "#,
    )?;
    Ok(())
}

/// User enablement, per scope. The one table here a user owns.
///
/// The `CHECK` is what makes "one global binding per Hook" a fact rather than a convention: global
/// must carry the empty key and a narrower scope must carry a non-empty one, so the two cannot
/// produce a second spelling of the same row.
fn apply_bindings(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS lifecycle_hook_bindings (
            hook_global_id TEXT NOT NULL
                REFERENCES lifecycle_hook_subjects (hook_global_id) ON DELETE RESTRICT,
            scope_kind TEXT NOT NULL CHECK (scope_kind IN ('global', 'project', 'agent')),
            scope_key TEXT NOT NULL,
            enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
            revision INTEGER NOT NULL CHECK (revision >= 1),
            updated_at TEXT NOT NULL,
            PRIMARY KEY (hook_global_id, scope_kind, scope_key),
            CHECK (
                (scope_kind = 'global' AND scope_key = '')
                OR (scope_kind <> 'global' AND scope_key <> '')
            )
        );
        "#,
    )?;
    Ok(())
}

/// That a Hook ran, when, and how it ended.
///
/// There is no payload column, no message column, and no path column — not a redacted one, none.
/// A repository with a free-text field has to be trusted to redact and every caller has to
/// remember to; the one that forgets writes a prompt into a row that outlives the session it came
/// from. `outcome_code` is the only thing an outcome can say, and its grammar is enforced by the
/// domain newtype that produces it.
///
/// `sequence` is unique per subject and assigned as `MAX + 1` under a write lock. Timestamps are
/// not an ordering: two executions inside one clock tick tie, and the clock can go backwards.
fn apply_executions(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS lifecycle_hook_executions (
            execution_id TEXT PRIMARY KEY,
            hook_global_id TEXT NOT NULL
                REFERENCES lifecycle_hook_subjects (hook_global_id) ON DELETE RESTRICT,
            sequence INTEGER NOT NULL CHECK (sequence >= 1),
            status TEXT NOT NULL CHECK (
                status IN ('pending', 'running', 'succeeded', 'failed', 'timed_out', 'denied')
            ),
            terminal INTEGER NOT NULL CHECK (terminal IN (0, 1)),
            outcome_code TEXT,
            duration_ms INTEGER CHECK (duration_ms IS NULL OR duration_ms >= 0),
            started_at TEXT NOT NULL,
            finished_at TEXT
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_lifecycle_hook_executions_sequence
            ON lifecycle_hook_executions (hook_global_id, sequence);

        CREATE INDEX IF NOT EXISTS idx_lifecycle_hook_executions_recent
            ON lifecycle_hook_executions (hook_global_id, sequence DESC);

        CREATE INDEX IF NOT EXISTS idx_lifecycle_hook_executions_prunable
            ON lifecycle_hook_executions (hook_global_id, terminal, sequence);
        "#,
    )?;
    Ok(())
}
