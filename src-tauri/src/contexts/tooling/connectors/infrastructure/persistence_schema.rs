//! Migration 89: connector subjects, versioned definitions, instances, and bindings.
//!
//! ## What this deliberately does not touch
//!
//! Existing IM connectors, MCP servers, GitHub readiness, and the local OCR/ASR/TTS capabilities
//! are not migrated, not copied, and not dual-written here. Their projections onto this model land
//! with the task groups that own them; a migration that moved them now would be rewriting live
//! user state to fit a model nothing reads yet.
//!
//! ## What the shape enforces
//!
//! **The label a person typed and the key uniqueness is decided on are different columns.**
//! `display_label` keeps their casing and spacing; `label_key` is the case-folded,
//! whitespace-collapsed form, and `(connector_global_id, label_key)` is unique. Neither is
//! identity — that is `instance_id` — so renaming an instance keeps its bindings and its
//! credential.
//!
//! **There is no column for live connection state.** No `connected`, no `connecting`, no
//! `last_error`. Those are properties of a socket: writing them down means every crash leaves a
//! row claiming a connection that does not exist, and every reader has to decide whether to
//! believe it. Storage holds what the user asked for, which is `desired_enabled`.
//!
//! **`credential_handle` is a handle.** The secret lives in the OS credential store. No column
//! here holds one, and none is added later without this comment changing.
//!
//! **`snapshot_id` carries no foreign key.** `extension_platform` owns snapshots; an enforced
//! reference would let one subdomain's deletions reach into another's evidence. Same for
//! `owner_extension_id`.
//!
//! **`ON DELETE RESTRICT` everywhere.** Deleting a subject that still has an instance should fail
//! and force whoever is doing it to say what happens to the credential attached to it.

use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// Connector subjects, definition revisions, instances, and bindings.
pub(crate) fn apply_connector_schema(conn: &Connection) -> Result<(), DatabaseError> {
    apply_subjects(conn)?;
    apply_definition_revisions(conn)?;
    apply_instances(conn)?;
    apply_bindings(conn)?;
    Ok(())
}

/// The stable identity instances attach to.
fn apply_subjects(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS connector_subjects (
            connector_global_id TEXT PRIMARY KEY,
            owner_extension_id TEXT NOT NULL,
            first_seen_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_connector_subjects_owner
            ON connector_subjects (owner_extension_id);
        "#,
    )?;
    Ok(())
}

/// What one connector is, in one snapshot. Immutable.
fn apply_definition_revisions(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS connector_definition_revisions (
            snapshot_id TEXT NOT NULL,
            connector_global_id TEXT NOT NULL
                REFERENCES connector_subjects (connector_global_id) ON DELETE RESTRICT,
            definition_digest TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            PRIMARY KEY (snapshot_id, connector_global_id)
        );

        CREATE INDEX IF NOT EXISTS idx_connector_definition_revisions_connector
            ON connector_definition_revisions (connector_global_id);
        "#,
    )?;
    Ok(())
}

/// One configured connector. The instance references the *subject*, never a versioned definition.
fn apply_instances(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS connector_instances (
            instance_id TEXT PRIMARY KEY,
            connector_global_id TEXT NOT NULL
                REFERENCES connector_subjects (connector_global_id) ON DELETE RESTRICT,
            display_label TEXT NOT NULL,
            label_key TEXT NOT NULL,
            desired_enabled INTEGER NOT NULL CHECK (desired_enabled IN (0, 1)),
            public_configuration TEXT NOT NULL,
            credential_handle TEXT,
            revision INTEGER NOT NULL CHECK (revision >= 1),
            updated_at TEXT NOT NULL,
            UNIQUE (connector_global_id, label_key)
        );
        "#,
    )?;
    Ok(())
}

/// Where an instance is in force.
///
/// The `CHECK` is what makes "one global binding per instance" a fact rather than a convention:
/// global carries the empty key and a narrower target carries a non-empty one, so the two cannot
/// produce a second spelling of the same row.
fn apply_bindings(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS connector_bindings (
            binding_id TEXT PRIMARY KEY,
            instance_id TEXT NOT NULL
                REFERENCES connector_instances (instance_id) ON DELETE RESTRICT,
            target_kind TEXT NOT NULL CHECK (
                target_kind IN ('global', 'project', 'agent', 'session')
            ),
            target_key TEXT NOT NULL,
            enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
            revision INTEGER NOT NULL CHECK (revision >= 1),
            updated_at TEXT NOT NULL,
            UNIQUE (instance_id, target_kind, target_key),
            CHECK (
                (target_kind = 'global' AND target_key = '')
                OR (target_kind <> 'global' AND target_key <> '')
            )
        );
        "#,
    )?;
    Ok(())
}
