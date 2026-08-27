use rusqlite::Connection;

use crate::platform::database::{table_has_column, DatabaseError};

/// Records when a rebuild last ran, so the maintenance screen can say it.
///
/// On the existing singleton row rather than in a table of its own: it answers the same question
/// that row already answers -- how far maintenance has got -- and a second table would let the two
/// disagree about whether a rebuild happened.
///
/// A separate migration from the one that created the row because that one has shipped. Editing it
/// in place would leave every installation that already ran it without the column.
///
/// Applied only when the column is absent, which is not defensive padding. This change was written
/// against migrations 82-84 and renumbered to 87-89 when the CLI lanes it was racing shipped
/// first, so a database that ran the earlier numbering already carries the column while its
/// `schema_migrations` row says otherwise. The same shape appears in the migration tests, which
/// rewind by deleting version rows rather than by dropping tables they know nothing about.
pub(crate) fn apply_reconciliation_schema(conn: &Connection) -> Result<(), DatabaseError> {
    if table_has_column(
        conn,
        "personalization_migration_state",
        "last_reconciled_at",
    )? {
        return Ok(());
    }
    conn.execute_batch(
        r#"
        ALTER TABLE personalization_migration_state ADD COLUMN last_reconciled_at TEXT;
        "#,
    )
    .map_err(|error| DatabaseError::Storage(error.to_string()))
}
