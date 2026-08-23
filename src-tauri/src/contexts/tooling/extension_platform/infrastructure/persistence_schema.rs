//! Extension Platform persistence beyond the capability gates, publisher keys, and the snapshot
//! pointer that migrations 82–86 already established.
//!
//! Two things this file does that a schema file usually does not, both deliberate.
//!
//! It **rebuilds `extension_platform_installations`**. Migration 86 created it without foreign
//! keys, and SQLite cannot add one to an existing table. A pointer without integrity is exactly
//! what the active-pointer rule exists to prevent, so the table is recreated with the references
//! and its rows copied. Migration 86 is unchanged and unchangeable; this is the additive repair.
//!
//! It uses `ON DELETE RESTRICT` everywhere and `CASCADE` nowhere. Every row a pointer points at is
//! evidence, and cascade is the mechanism by which evidence disappears because something else was
//! removed. Deleting an installation with a live generation should fail and force whoever is doing
//! it to say what should happen to the generation.

use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// Packages, version claims, snapshot detail, runtime generations, and operation witnesses.
pub(crate) fn apply_extension_persistence_schema(conn: &Connection) -> Result<(), DatabaseError> {
    apply_version_claims(conn)?;
    apply_packages(conn)?;
    apply_snapshot_detail(conn)?;
    rebuild_installations_with_references(conn)?;
    apply_runtime_generations(conn)?;
    apply_operation_witnesses(conn)?;
    Ok(())
}

/// One version of one extension binds to exactly one set of bytes, forever.
///
/// The primary key is the binding. A second claim on the same version either offers the same hash
/// — idempotent — or a different one, which the repository refuses. `attempted_conflicts` keeps
/// the refused hash rather than discarding it: "this version was claimed twice with different
/// bytes" is the finding, and a refusal that leaves no trace is a finding nobody can act on.
fn apply_version_claims(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS extension_platform_version_claims (
            publisher TEXT NOT NULL,
            extension_id TEXT NOT NULL,
            version TEXT NOT NULL,
            package_hash TEXT NOT NULL,
            provenance TEXT NOT NULL,
            first_claimed_at TEXT NOT NULL,
            PRIMARY KEY (publisher, extension_id, version)
        );

        CREATE TABLE IF NOT EXISTS extension_platform_version_claim_conflicts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            publisher TEXT NOT NULL,
            extension_id TEXT NOT NULL,
            version TEXT NOT NULL,
            bound_package_hash TEXT NOT NULL,
            offered_package_hash TEXT NOT NULL,
            offered_provenance TEXT NOT NULL,
            observed_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_extension_platform_version_claim_conflicts_version
            ON extension_platform_version_claim_conflicts (extension_id, version, id DESC);
        "#,
    )?;
    Ok(())
}

/// Bytes, by their own digest.
///
/// Separate from snapshots because one set of bytes can be claimed by more than one snapshot over
/// time — a reinstall, a rollback — and because the package is evidence about what arrived, while
/// a snapshot is evidence about what was installed.
fn apply_packages(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS extension_platform_packages (
            package_hash TEXT PRIMARY KEY,
            byte_length INTEGER NOT NULL CHECK (byte_length >= 0),
            signature_state TEXT NOT NULL,
            publisher_key_fingerprint TEXT,
            first_seen_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

/// What a snapshot declared, frozen at the moment it was published.
///
/// Recorded rather than re-read from the manifest on demand: the manifest lives in the package on
/// disk, and an installation has to be describable when that disk is unavailable, when the package
/// has been removed, and when the reading code has changed.
fn apply_snapshot_detail(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS idx_extension_platform_snapshots_identity
            ON extension_platform_snapshots (extension_id, version, package_hash);

        CREATE TABLE IF NOT EXISTS extension_platform_snapshot_dependencies (
            snapshot_id TEXT NOT NULL
                REFERENCES extension_platform_snapshots (snapshot_id) ON DELETE RESTRICT,
            dependency_kind TEXT NOT NULL,
            dependency_id TEXT NOT NULL,
            version_requirement TEXT NOT NULL,
            optional INTEGER NOT NULL CHECK (optional IN (0, 1)),
            PRIMARY KEY (snapshot_id, dependency_kind, dependency_id)
        );

        CREATE TABLE IF NOT EXISTS extension_platform_snapshot_contributions (
            snapshot_id TEXT NOT NULL
                REFERENCES extension_platform_snapshots (snapshot_id) ON DELETE RESTRICT,
            global_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            local_id TEXT NOT NULL,
            contribution_digest TEXT,
            PRIMARY KEY (snapshot_id, global_id)
        );

        CREATE INDEX IF NOT EXISTS idx_extension_platform_snapshot_contributions_global
            ON extension_platform_snapshot_contributions (global_id);
        "#,
    )?;
    repair_snapshot_contribution_digest(conn)
}

/// Migration 91: records which schema version wrote each operation witness.
///
/// Retention must never remove a row a newer build wrote. Without a version on the row that is
/// unknowable, and the failure is asymmetric: a downgrade that prunes what it cannot interpret
/// destroys the record the upgrade was keeping, while a downgrade that keeps it loses nothing.
///
/// A separate migration rather than a column added to 86, which is already committed. `NOT NULL
/// DEFAULT 1` is the honest backfill: every existing row *was* written by schema version 1.
pub(crate) fn apply_operation_witness_bounds(conn: &Connection) -> Result<(), DatabaseError> {
    if crate::platform::database::table_has_column(
        conn,
        "extension_platform_operation_witnesses",
        "schema_version",
    )? {
        return Ok(());
    }
    conn.execute_batch(
        "ALTER TABLE extension_platform_operation_witnesses              ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 1;",
    )?;
    Ok(())
}

/// Adds `contribution_digest` to a contributions table created before it existed.
///
/// Migration 87 gained the column while this change was still unreleased, so a database that had
/// already recorded 86 would never see it — `apply_transactional_migration` skips a version it has
/// applied, and `CREATE TABLE IF NOT EXISTS` would not add it either. That database is a developer
/// database on this branch, and this repository's databases are shared across worktrees, so
/// "delete it and start again" would destroy unrelated branches' state.
///
/// Called both from migration 87 and unconditionally from `migrate`, and a no-op wherever the
/// column is present.
///
/// The column is nullable rather than `NOT NULL DEFAULT ''`: a row written before the column
/// existed genuinely has no declaration, and an empty string would be a digest that matches
/// nothing while looking like a value. `NULL` reads as "this snapshot declares no digest for that
/// contribution", which makes the consumer's answer `unavailable` — the conservative one.
pub(crate) fn repair_snapshot_contribution_digest(conn: &Connection) -> Result<(), DatabaseError> {
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type = 'table' AND name = 'extension_platform_snapshot_contributions'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)?;
    if !table_exists
        || crate::platform::database::table_has_column(
            conn,
            "extension_platform_snapshot_contributions",
            "contribution_digest",
        )?
    {
        return Ok(());
    }

    conn.execute_batch(
        "ALTER TABLE extension_platform_snapshot_contributions ADD COLUMN contribution_digest TEXT;",
    )?;
    Ok(())
}

/// Recreates the installations table with the references migration 86 could not add.
///
/// Guarded on the references being absent, so a database that already has them is untouched and a
/// re-run is a no-op. The copy is inside the caller's transaction: `apply_transactional_migration`
/// wraps this, so a failure anywhere leaves the original table in place.
fn rebuild_installations_with_references(conn: &Connection) -> Result<(), DatabaseError> {
    let has_references: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_list('extension_platform_installations')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)?;
    if has_references {
        return Ok(());
    }

    conn.execute_batch(
        r#"
        CREATE TABLE extension_platform_installations_rebuilt (
            installation_id TEXT PRIMARY KEY,
            extension_id TEXT NOT NULL UNIQUE,
            active_snapshot_id TEXT NOT NULL
                REFERENCES extension_platform_snapshots (snapshot_id) ON DELETE RESTRICT,
            previous_snapshot_id TEXT
                REFERENCES extension_platform_snapshots (snapshot_id) ON DELETE RESTRICT,
            revision INTEGER NOT NULL,
            updated_at TEXT NOT NULL
        );

        INSERT INTO extension_platform_installations_rebuilt
            (installation_id, extension_id, active_snapshot_id, previous_snapshot_id, revision,
             updated_at)
        SELECT installation_id, extension_id, active_snapshot_id, previous_snapshot_id, revision,
               updated_at
        FROM extension_platform_installations;

        DROP TABLE extension_platform_installations;

        ALTER TABLE extension_platform_installations_rebuilt
            RENAME TO extension_platform_installations;
        "#,
    )?;
    Ok(())
}

/// One activation of one installation's runtime, and the single pointer that says which is live.
///
/// The pointer's reference is composite. `generation_id` alone would be satisfied by any row in
/// the table, so installation A could be pointed at installation B's generation and the database
/// would agree; `(generation_id, installation_id)` cannot.
fn apply_runtime_generations(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS extension_platform_runtime_generations (
            generation_id TEXT PRIMARY KEY,
            installation_id TEXT NOT NULL
                REFERENCES extension_platform_installations (installation_id) ON DELETE RESTRICT,
            snapshot_id TEXT NOT NULL
                REFERENCES extension_platform_snapshots (snapshot_id) ON DELETE RESTRICT,
            started_at TEXT NOT NULL,
            UNIQUE (generation_id, installation_id)
        );

        CREATE INDEX IF NOT EXISTS idx_extension_platform_runtime_generations_installation
            ON extension_platform_runtime_generations (installation_id, started_at DESC);

        CREATE TABLE IF NOT EXISTS extension_platform_active_runtime_generations (
            installation_id TEXT PRIMARY KEY
                REFERENCES extension_platform_installations (installation_id) ON DELETE RESTRICT,
            generation_id TEXT NOT NULL,
            revision INTEGER NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (generation_id, installation_id)
                REFERENCES extension_platform_runtime_generations (generation_id, installation_id)
                ON DELETE RESTRICT
        );
        "#,
    )?;
    Ok(())
}

/// What each preview was bound to.
///
/// Keyed by an application-generated `witness_id` rather than by the digest. The digest covers the
/// *state* a confirmation is bound to and deliberately not the operation, so that confirm can
/// recompute it — which means two operations previewing the same unchanged world produce the same
/// digest. A digest primary key would make the second collide with the first.
///
/// `operation_id` carries no reference. Operations live in an in-memory registry; there is no
/// table to point at, and this change does not create one.
fn apply_operation_witnesses(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS extension_platform_operation_witnesses (
            witness_id TEXT PRIMARY KEY,
            operation_id TEXT NOT NULL,
            witness_digest TEXT NOT NULL,
            extension_id TEXT NOT NULL,
            version TEXT NOT NULL,
            package_hash TEXT NOT NULL,
            manifest_digest TEXT NOT NULL,
            signature_state TEXT NOT NULL,
            trust_profile TEXT NOT NULL,
            issued_at TEXT NOT NULL,
            UNIQUE (operation_id, witness_digest)
        );

        CREATE INDEX IF NOT EXISTS idx_extension_platform_operation_witnesses_extension
            ON extension_platform_operation_witnesses (extension_id, issued_at DESC);
        "#,
    )?;
    Ok(())
}
