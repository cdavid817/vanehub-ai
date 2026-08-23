//! Walking `Installation -> Active Generation Pointer -> Runtime Generation -> Snapshot`.
//!
//! Three statements rather than one join, because the three "no" answers are different answers and
//! a single query that returned no rows could not tell them apart: nothing installed contributes
//! this id, something does but is not running, and something is running but does not declare it.
//! Collapsing them would make "installed but not activated" indistinguishable from "uninstalled",
//! and a consumer would report the wrong reason for a Hook that is not firing.
//!
//! All three run inside **one deferred read transaction**. Under WAL each bare statement takes its
//! own snapshot, so without the transaction an activation committing between statements would let
//! this return a state that never existed — the owner from before the switch, the generation from
//! after it. Each statement would be individually consistent, which is what makes that class of
//! bug survive review. The transaction fixes the snapshot at the first read.
//!
//! The owner lookup deliberately matches contributions from **any** snapshot of the extension. It
//! is used only to identify *which* installation owns the id; the snapshot that is running is then
//! read from the pointer. Using the matched contribution's own snapshot would reintroduce exactly
//! the defect this exists to fix -- a recorded-but-not-activated version answering for a running
//! one.

use crate::contexts::tooling::extension_platform::application::ActiveContributionReader;
use crate::contexts::tooling::extension_platform::domain::{
    ActiveContribution, ActiveContributionError,
};
use crate::platform::database::{begin_read_transaction, NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension, Transaction};
use std::sync::Arc;

pub(crate) struct SqliteActiveContributionReader {
    database: Arc<NativeDatabase>,
}

impl SqliteActiveContributionReader {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, ActiveContributionError> {
        self.database
            .connection()
            .map_err(|error| ActiveContributionError::Storage(error.to_string()))
    }

    /// The three reads, against one snapshot.
    ///
    /// `after_owner_lookup` runs once the read snapshot is established and before the pointer is
    /// followed. It is `&|| {}` in production and exists so a test can hold the transaction open
    /// across a concurrent activation; without a seam there, snapshot isolation is a property that
    /// can only be argued for, never demonstrated.
    fn read(
        transaction: &Transaction<'_>,
        global_id: &str,
        after_owner_lookup: &dyn Fn(),
    ) -> Result<ActiveContribution, ActiveContributionError> {
        // Which installation owns this id? Two owners is an impossible state the database does not
        // forbid -- its key is `(snapshot_id, global_id)` -- so it is refused rather than resolved.
        let installations: Vec<String> = transaction
            .prepare(
                "SELECT DISTINCT installation.installation_id \
                 FROM extension_platform_snapshot_contributions AS contribution \
                 JOIN extension_platform_snapshots AS snapshot \
                     ON snapshot.snapshot_id = contribution.snapshot_id \
                 JOIN extension_platform_installations AS installation \
                     ON installation.extension_id = snapshot.extension_id \
                 WHERE contribution.global_id = ?1 \
                 ORDER BY installation.installation_id LIMIT 2",
            )
            .map_err(storage)?
            .query_map(params![global_id], |row| row.get::<_, String>(0))
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;

        after_owner_lookup();

        let installation = match installations.as_slice() {
            [] => return Ok(ActiveContribution::NotInstalled),
            [single] => single.clone(),
            _ => return Err(ActiveContributionError::AmbiguousOwner),
        };

        // What is that installation running? The composite foreign key on the pointer guarantees
        // the generation belongs to this installation, and the generation's own reference
        // guarantees its snapshot exists, so a dangling active snapshot is unrepresentable here.
        let running: Option<String> = transaction
            .query_row(
                "SELECT generation.snapshot_id \
                 FROM extension_platform_active_runtime_generations AS active \
                 JOIN extension_platform_runtime_generations AS generation \
                     ON generation.generation_id = active.generation_id \
                    AND generation.installation_id = active.installation_id \
                 WHERE active.installation_id = ?1",
                params![installation],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage)?;

        let Some(snapshot_id) = running else {
            return Ok(ActiveContribution::NoActiveGeneration);
        };

        // What does the running snapshot declare for this id? A missing row and a row with no
        // digest are the same answer: nothing to dispatch.
        let declared_digest: Option<String> = transaction
            .query_row(
                "SELECT contribution_digest FROM extension_platform_snapshot_contributions \
                 WHERE snapshot_id = ?1 AND global_id = ?2",
                params![snapshot_id, global_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(storage)?
            .flatten();

        Ok(ActiveContribution::Running {
            snapshot_id,
            declared_digest,
        })
    }

    /// `active`, with a pause between establishing the read snapshot and following the pointer.
    #[cfg(test)]
    pub(crate) fn active_pausing_after_owner_lookup(
        &self,
        global_id: &str,
        pause: &dyn Fn(),
    ) -> Result<ActiveContribution, ActiveContributionError> {
        let connection = self.connection()?;
        let transaction = begin_read_transaction(&connection)
            .map_err(|error| ActiveContributionError::Storage(error.to_string()))?;
        Self::read(&transaction, global_id, pause)
    }
}

fn storage(error: rusqlite::Error) -> ActiveContributionError {
    ActiveContributionError::Storage(error.to_string())
}

impl ActiveContributionReader for SqliteActiveContributionReader {
    fn active(&self, global_id: &str) -> Result<ActiveContribution, ActiveContributionError> {
        let connection = self.connection()?;
        let transaction = begin_read_transaction(&connection)
            .map_err(|error| ActiveContributionError::Storage(error.to_string()))?;
        // Dropped without commit, which is the whole lifecycle of a read: rolling back a
        // transaction that wrote nothing releases the snapshot and changes nothing.
        Self::read(&transaction, global_id, &|| {})
    }
}
