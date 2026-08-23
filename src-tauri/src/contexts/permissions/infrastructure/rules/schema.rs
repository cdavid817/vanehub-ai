//! Migration 89: the one canonical authorization-rule store.
//!
//! ## What this deliberately does not touch
//!
//! **`agent_principals.template_name`.** Templates stay where they are, owned by the existing PDP.
//! There is no `source_kind = 'template'`, no seeded rows for the four templates, and a template
//! change does not publish a rule set or move the active pointer. Compiling templates into rules
//! would make every template assignment a new immutable rule set, and would put the four shipped
//! templates behind the same review path as a downloaded extension's rules — which is backwards,
//! since the templates are the host's own fallback.
//!
//! **`permission_grants`.** Not modified, not copied, not rebuilt, and not part of any rule-set
//! digest. Grants remain the Approval Broker's, and are consulted only for an answer that is still
//! `Ask` after rules, template, and hooks. Activating a different rule set does not delete a
//! grant; eligibility is re-decided the next time the grant would be used, which is what lets a
//! rule set be rolled back without destroying a user's remembered answers.
//!
//! ## What the shape enforces
//!
//! An extension rule may only be `ask` or `deny`. The `CHECK` here is the second of two guards —
//! `AuthorizationRule::admit` is the first — because an `Allow` contributed by a downloaded
//! package is a privilege escalation, and a single guard is one refactor away from not existing.
//!
//! The active pointer starts as `NULL` rather than pointing at a fabricated empty rule set. "No
//! rules have been published" and "a published set that happens to be empty" are different facts,
//! and inventing the second would claim a digest nobody produced.

use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// Rule sets, their rules, and the single pointer that says which set is in force.
pub(crate) fn apply_authorization_rule_schema(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS permission_rule_sets (
            rule_set_id TEXT PRIMARY KEY,
            content_digest TEXT NOT NULL UNIQUE,
            rule_count INTEGER NOT NULL CHECK (rule_count >= 0),
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS permission_authorization_rules (
            rule_set_id TEXT NOT NULL
                REFERENCES permission_rule_sets (rule_set_id) ON DELETE RESTRICT,
            source_kind TEXT NOT NULL CHECK (source_kind IN ('user', 'project', 'extension')),
            source_id TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            scope_kind TEXT NOT NULL CHECK (
                scope_kind IN ('global', 'user', 'project', 'principal', 'session')
            ),
            scope_key TEXT NOT NULL,
            operation TEXT NOT NULL,
            matcher TEXT NOT NULL,
            effect TEXT NOT NULL CHECK (effect IN ('deny', 'ask', 'allow')),
            allowed_scopes TEXT NOT NULL,
            priority INTEGER NOT NULL,
            specificity INTEGER NOT NULL,
            expires_at TEXT,
            provenance TEXT NOT NULL CHECK (
                provenance IN (
                    'user_settings', 'project_settings', 'extension_manifest', 'host_default'
                )
            ),
            PRIMARY KEY (rule_set_id, source_kind, source_id, rule_id),
            -- A downloaded package may narrow what is permitted and never widen it.
            CHECK (source_kind <> 'extension' OR effect IN ('ask', 'deny')),
            -- Global carries the empty key and a narrower scope carries a non-empty one, so the
            -- primary key is total and 'global' has exactly one spelling.
            CHECK (
                (scope_kind = 'global' AND scope_key = '')
                OR (scope_kind <> 'global' AND scope_key <> '')
            ),
            -- An 'ask' that may never be remembered and a 'deny' that names grant scopes are both
            -- rules whose two halves disagree.
            CHECK (
                (effect = 'ask' AND allowed_scopes <> '')
                OR (effect <> 'ask' AND allowed_scopes = '')
            )
        );

        CREATE INDEX IF NOT EXISTS idx_permission_authorization_rules_lookup
            ON permission_authorization_rules (rule_set_id, operation, scope_kind, scope_key);

        CREATE TABLE IF NOT EXISTS permission_active_rule_set (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            rule_set_id TEXT
                REFERENCES permission_rule_sets (rule_set_id) ON DELETE RESTRICT,
            revision INTEGER NOT NULL CHECK (revision >= 0),
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    seed_empty_active_pointer(conn)
}

/// Seeds the singleton row pointing at nothing.
///
/// The row exists from the start so that compare-and-swap has a revision to compare against; the
/// pointer inside it is `NULL` because nothing has been published. `DO NOTHING` rather than an
/// upsert: re-running the migration must not reset a pointer that has since been moved.
fn seed_empty_active_pointer(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute(
        "INSERT INTO permission_active_rule_set (id, rule_set_id, revision, updated_at) \
         VALUES (1, NULL, 0, '1970-01-01T00:00:00Z') \
         ON CONFLICT(id) DO NOTHING",
        [],
    )?;
    Ok(())
}
