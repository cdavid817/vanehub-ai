use rusqlite::Connection;

use crate::platform::database::DatabaseError;

/// Records when a rebuild last ran, so the maintenance screen can say it.
///
/// On the existing singleton row rather than in a table of its own: it answers the same question
/// that row already answers -- how far maintenance has got -- and a second table would let the two
/// disagree about whether a rebuild happened.
///
/// A separate migration from the one that created the row because that one has shipped. Editing it
/// in place would leave every installation that already ran it without the column.
pub(crate) fn apply_reconciliation_schema(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        ALTER TABLE personalization_migration_state ADD COLUMN last_reconciled_at TEXT;
        "#,
    )
    .map_err(|error| DatabaseError::Storage(error.to_string()))
}
