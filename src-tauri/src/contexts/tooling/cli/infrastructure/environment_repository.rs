//! SQLite persistence for environments, catalogs, and action plans.
//!
//! Two properties this file exists to guarantee:
//!
//! - **Atomic plan consumption.** `begin_action_plan_execution` validates revision, state, expiry,
//!   and fingerprint and moves `draft -> executing` inside one immediate transaction. Two callers
//!   racing the same plan cannot both be admitted, which is why the check does not live in the
//!   service as a read-then-write.
//! - **Fallible decoding.** Every row becomes a domain value through an explicit conversion that
//!   can fail. A malformed or unknown-version document yields a typed storage error, never a panic
//!   and never a half-built snapshot.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::Value;

use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;
use crate::contexts::tooling::cli::application::environment_ports::CliEnvironmentRepository;
use crate::contexts::tooling::cli::domain::bulk::CliBulkActionPlan;
use crate::contexts::tooling::cli::domain::catalog::CliVersionCatalog;
use crate::contexts::tooling::cli::domain::ids::{
    CliActionPlanId, CliBulkPlanId, CliSourceId, CliToolId,
};
use crate::contexts::tooling::cli::domain::plan::{CliActionPlan, CliActionPlanState};
use crate::contexts::tooling::cli::domain::snapshot::CliEnvironmentSnapshot;
use crate::platform::database::NativeDatabase;

use super::environment_serde::{
    decode_bulk_plan, decode_catalog, decode_plan, decode_plan_state, decode_snapshot,
    encode_bulk_plan, encode_catalog, encode_plan, encode_snapshot, legacy_row_to_stale_snapshot,
};

/// The only scope this change persists. Stored explicitly so adding a second one later is a data
/// change rather than a schema change.
const LOCAL_DESKTOP: &str = "local-desktop";

/// The fingerprint carried by a snapshot reconstructed from a legacy row.
///
/// Deliberately not a real fingerprint: it can never equal a computed one, so a legacy snapshot is
/// always rejected as the basis for a mutation until a genuine refresh replaces it.
const LEGACY_FINGERPRINT: &str = "legacy-import";

pub(crate) struct SqliteCliEnvironmentRepository {
    database: NativeDatabase,
}

impl SqliteCliEnvironmentRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<crate::platform::database::PooledSqlite, CliEnvironmentError> {
        self.database.connection().map_err(storage_error)
    }

    /// Reads every leftover pre-change `cli_tool_status` row.
    ///
    /// Read-only by construction: the legacy table is never written from here, so a legacy row can
    /// never become the new write model -- the first real refresh overwrites the snapshot instead.
    ///
    /// One query serves both `list_snapshots` and `load_snapshot` on purpose. When only the
    /// single-agent path consulted the legacy table, the runtime resolved a launch from a leftover
    /// row that the CLI Management page -- which lists -- never showed, which is the page-and-
    /// runtime disagreement this context exists to end, rebuilt out of the compatibility shim.
    /// The table holds one row per managed agent, so reading all of them to answer one is free.
    fn legacy_snapshots(
        &self,
        connection: &crate::platform::database::PooledSqlite,
    ) -> Result<Vec<CliEnvironmentSnapshot>, CliEnvironmentError> {
        let mut statement = connection
            .prepare(
                "SELECT agent_id, detected_path, current_version, last_checked_at
                 FROM cli_tool_status ORDER BY agent_id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(storage_error)?;

        let mut snapshots = Vec::new();
        for row in rows {
            let (raw_agent_id, detected_path, current_version, last_checked_at) =
                row.map_err(storage_error)?;
            // A legacy row naming an agent this build no longer manages is skipped rather than
            // failed on: it cannot be shown or launched either way, and refusing to list would let
            // one obsolete row hide every tool that is still supported.
            let Ok(agent_id) = CliToolId::new(&raw_agent_id) else {
                continue;
            };
            let checked_at = last_checked_at
                .map(|raw| {
                    DateTime::parse_from_rfc3339(&raw)
                        .map(|parsed| parsed.with_timezone(&Utc))
                        .map_err(|error| {
                            CliEnvironmentError::Storage(format!(
                                "{raw_agent_id}: legacy last_checked_at: {error}"
                            ))
                        })
                })
                .transpose()?;
            snapshots.push(legacy_row_to_stale_snapshot(
                agent_id,
                LEGACY_FINGERPRINT,
                detected_path,
                current_version,
                checked_at,
            ));
        }
        Ok(snapshots)
    }
}

fn storage_error(error: impl std::fmt::Display) -> CliEnvironmentError {
    CliEnvironmentError::Storage(error.to_string())
}

/// Reads a JSON column into a domain value, or reports which row could not be decoded.
///
/// The row identity is included because a single unreadable document must be diagnosable without
/// dumping its contents, which may carry paths.
fn decode_column<T>(
    raw: &str,
    identity: &str,
    decode: impl Fn(Value) -> Result<T, String>,
) -> Result<T, CliEnvironmentError> {
    let value: Value = serde_json::from_str(raw)
        .map_err(|error| CliEnvironmentError::Storage(format!("{identity}: {error}")))?;
    decode(value).map_err(|reason| CliEnvironmentError::Storage(format!("{identity}: {reason}")))
}

/// Rebuilds a plan from its stored document with the `state` column as the authority.
///
/// The column is what the CHECK constraint, the `state` index and every maintenance sweep operate
/// on; the document copy is a denormalized convenience that can lag behind it.
fn plan_from_row(
    raw: &str,
    raw_state: &str,
    identity: &str,
) -> Result<CliActionPlan, CliEnvironmentError> {
    let plan = decode_column(raw, identity, decode_plan)?;
    let state = decode_plan_state(raw_state)
        .map_err(|reason| CliEnvironmentError::Storage(format!("{identity}: {reason}")))?;
    Ok(CliActionPlan { state, ..plan })
}

impl CliEnvironmentRepository for SqliteCliEnvironmentRepository {
    fn list_snapshots(&self) -> Result<Vec<CliEnvironmentSnapshot>, CliEnvironmentError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT agent_id, snapshot_json FROM cli_environment_snapshots
                 WHERE scope_id = ?1 ORDER BY agent_id",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([LOCAL_DESKTOP], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage_error)?;

        let mut snapshots = Vec::new();
        for row in rows {
            let (agent_id, raw) = row.map_err(storage_error)?;
            snapshots.push(decode_column(&raw, &agent_id, decode_snapshot)?);
        }
        // Same rule as the single-agent read: a legacy row counts only where no authoritative
        // snapshot exists, so a refreshed tool is never described by the old table.
        for legacy in self.legacy_snapshots(&connection)? {
            if !snapshots
                .iter()
                .any(|snapshot| snapshot.agent_id == legacy.agent_id)
            {
                snapshots.push(legacy);
            }
        }
        snapshots.sort_by(|left, right| left.agent_id.as_str().cmp(right.agent_id.as_str()));
        Ok(snapshots)
    }

    fn load_snapshot(
        &self,
        agent_id: &CliToolId,
    ) -> Result<Option<CliEnvironmentSnapshot>, CliEnvironmentError> {
        let connection = self.connection()?;
        let raw: Option<String> = connection
            .query_row(
                "SELECT snapshot_json FROM cli_environment_snapshots
                 WHERE agent_id = ?1 AND scope_id = ?2",
                params![agent_id.as_str(), LOCAL_DESKTOP],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        match raw {
            Some(raw) => decode_column(&raw, agent_id.as_str(), decode_snapshot).map(Some),
            // No authoritative snapshot yet: an upgrading user still has a legacy row, and showing
            // it as stale beats showing nothing until the first refresh lands.
            None => Ok(self
                .legacy_snapshots(&connection)?
                .into_iter()
                .find(|snapshot| &snapshot.agent_id == agent_id)),
        }
    }

    fn save_snapshot_atomic(
        &self,
        snapshot: &CliEnvironmentSnapshot,
    ) -> Result<(), CliEnvironmentError> {
        let document = encode_snapshot(snapshot);
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO cli_environment_snapshots
                 (agent_id, scope_id, schema_version, environment_fingerprint, snapshot_json,
                  checked_at, last_operation_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(agent_id, scope_id) DO UPDATE SET
                     schema_version = excluded.schema_version,
                     environment_fingerprint = excluded.environment_fingerprint,
                     snapshot_json = excluded.snapshot_json,
                     checked_at = excluded.checked_at,
                     last_operation_id = excluded.last_operation_id",
                params![
                    snapshot.agent_id.as_str(),
                    LOCAL_DESKTOP,
                    snapshot.schema_version,
                    snapshot.environment_fingerprint,
                    document.to_string(),
                    snapshot.checked_at.map(|value| value.to_rfc3339()),
                    snapshot.last_operation_id,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn load_catalog(
        &self,
        agent_id: &CliToolId,
        source_id: &CliSourceId,
        channel: Option<&str>,
    ) -> Result<Option<CliVersionCatalog>, CliEnvironmentError> {
        let connection = self.connection()?;
        let raw: Option<String> = connection
            .query_row(
                "SELECT catalog_json FROM cli_version_catalogs
                 WHERE agent_id = ?1 AND scope_id = ?2 AND source_id = ?3 AND channel = ?4",
                params![
                    agent_id.as_str(),
                    LOCAL_DESKTOP,
                    source_id.as_str(),
                    channel.unwrap_or_default()
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        raw.map(|raw| {
            decode_column(
                &raw,
                &format!("{}/{}", agent_id.as_str(), source_id.as_str()),
                decode_catalog,
            )
        })
        .transpose()
    }

    fn save_catalog(&self, catalog: &CliVersionCatalog) -> Result<(), CliEnvironmentError> {
        let document = encode_catalog(catalog);
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO cli_version_catalogs
                 (agent_id, scope_id, source_id, channel, catalog_json, fetched_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(agent_id, scope_id, source_id, channel) DO UPDATE SET
                     catalog_json = excluded.catalog_json,
                     fetched_at = excluded.fetched_at,
                     expires_at = excluded.expires_at",
                params![
                    catalog.agent_id.as_str(),
                    LOCAL_DESKTOP,
                    catalog.source_id.as_str(),
                    catalog.channel.clone().unwrap_or_default(),
                    document.to_string(),
                    catalog.fetched_at.to_rfc3339(),
                    catalog.expires_at.to_rfc3339(),
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn create_action_plan(&self, plan: &CliActionPlan) -> Result<(), CliEnvironmentError> {
        let connection = self.connection()?;
        insert_plan(&connection, plan, None).map_err(storage_error)?;
        Ok(())
    }

    fn load_action_plan(
        &self,
        plan_id: &CliActionPlanId,
    ) -> Result<Option<CliActionPlan>, CliEnvironmentError> {
        let connection = self.connection()?;
        let row: Option<(String, String)> = connection
            .query_row(
                "SELECT plan_json, state FROM cli_action_plans
                 WHERE plan_id = ?1 AND plan_kind = 'action'",
                params![plan_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        row.map(|(raw, state)| plan_from_row(&raw, &state, plan_id.as_str()))
            .transpose()
    }

    fn list_draft_plans(
        &self,
        agent_id: &CliToolId,
    ) -> Result<Vec<CliActionPlan>, CliEnvironmentError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT plan_id, plan_json FROM cli_action_plans
                 WHERE agent_id = ?1 AND plan_kind = 'action' AND state = 'draft'
                 ORDER BY created_at",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map([agent_id.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage_error)?;

        let mut plans = Vec::new();
        for row in rows {
            let (plan_id, raw) = row.map_err(storage_error)?;
            // The query already filtered on the column, so the row's authoritative state is `draft`
            // whatever the document happens to say.
            plans.push(plan_from_row(
                &raw,
                CliActionPlanState::Draft.as_str(),
                &plan_id,
            )?);
        }
        Ok(plans)
    }

    fn begin_action_plan_execution(
        &self,
        plan_id: &CliActionPlanId,
        expected_revision: u32,
        current_fingerprint: &str,
        now: DateTime<Utc>,
    ) -> Result<CliActionPlan, CliEnvironmentError> {
        let mut connection = self.connection()?;
        // Immediate: the write lock is taken before the read, so two callers cannot both observe
        // `draft` and both proceed.
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;

        let row: Option<(String, String)> = transaction
            .query_row(
                "SELECT plan_json, state FROM cli_action_plans
                 WHERE plan_id = ?1 AND plan_kind = 'action'",
                params![plan_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        let Some((raw, raw_state)) = row else {
            return Err(CliEnvironmentError::PlanNotFound);
        };
        let plan = plan_from_row(&raw, &raw_state, plan_id.as_str())?;

        // The domain owns every admission rule; this only supplies the atomicity around it.
        plan.admit_execution(expected_revision, current_fingerprint, now)?;

        let admitted = CliActionPlan {
            state: CliActionPlanState::Executing,
            ..plan
        };
        transaction
            .execute(
                "UPDATE cli_action_plans SET state = 'executing', started_at = ?2, plan_json = ?3
                 WHERE plan_id = ?1",
                params![
                    plan_id.as_str(),
                    now.to_rfc3339(),
                    encode_plan(&admitted).to_string()
                ],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(admitted)
    }

    fn finish_action_plan(
        &self,
        plan_id: &CliActionPlanId,
        state: CliActionPlanState,
        now: DateTime<Utc>,
    ) -> Result<(), CliEnvironmentError> {
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(storage_error)?;

        let row: Option<(String, String)> = transaction
            .query_row(
                "SELECT plan_json, state FROM cli_action_plans WHERE plan_id = ?1",
                params![plan_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        if let Some((raw, raw_state)) = row {
            let plan = plan_from_row(&raw, &raw_state, plan_id.as_str())?;
            let finished = CliActionPlan { state, ..plan };
            transaction
                .execute(
                    "UPDATE cli_action_plans SET state = ?2, completed_at = ?3, plan_json = ?4
                     WHERE plan_id = ?1",
                    params![
                        plan_id.as_str(),
                        state.as_str(),
                        now.to_rfc3339(),
                        encode_plan(&finished).to_string()
                    ],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(())
    }

    fn create_bulk_plan_atomic(
        &self,
        bulk: &CliBulkActionPlan,
        item_plans: &[CliActionPlan],
    ) -> Result<(), CliEnvironmentError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;

        // All or nothing: a batch that fails halfway must not leave item plans behind that nothing
        // references and nothing will ever expire.
        insert_bulk(&transaction, bulk).map_err(storage_error)?;
        for plan in item_plans {
            insert_plan_tx(&transaction, plan, Some(bulk.id.as_str())).map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(())
    }

    fn load_bulk_plan(
        &self,
        plan_id: &CliBulkPlanId,
    ) -> Result<Option<CliBulkActionPlan>, CliEnvironmentError> {
        let connection = self.connection()?;
        let raw: Option<String> = connection
            .query_row(
                "SELECT plan_json FROM cli_action_plans WHERE plan_id = ?1 AND plan_kind = 'bulk'",
                params![plan_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        raw.map(|raw| decode_column(&raw, plan_id.as_str(), decode_bulk_plan))
            .transpose()
    }

    fn expire_stale_plans(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Result<usize, CliEnvironmentError> {
        let connection = self.connection()?;
        // Bounded: maintenance runs off the command boundary but must still not stall behind an
        // unbounded scan of a table that only ever grows.
        let changed = connection
            .execute(
                "UPDATE cli_action_plans SET state = 'expired'
                 WHERE plan_id IN (
                     SELECT plan_id FROM cli_action_plans
                     WHERE state = 'draft' AND expires_at <= ?1
                     ORDER BY expires_at LIMIT ?2
                 )",
                params![now.to_rfc3339(), limit as i64],
            )
            .map_err(storage_error)?;
        Ok(changed)
    }
}

/// The owned column values for one plan row.
///
/// Materialized up front because `params!` borrows and several of these values are formatted on
/// the fly -- a helper returning the macro's output would be returning references to temporaries.
struct PlanRow {
    plan_id: String,
    agent_id: String,
    revision: u32,
    state: &'static str,
    fingerprint: String,
    document: String,
    created_at: String,
    expires_at: String,
    bulk_plan_id: Option<String>,
}

impl PlanRow {
    fn of(plan: &CliActionPlan, bulk_plan_id: Option<&str>) -> Self {
        Self {
            plan_id: plan.id.as_str().to_string(),
            agent_id: plan.agent_id.as_str().to_string(),
            revision: plan.revision,
            state: plan.state.as_str(),
            fingerprint: plan.environment_fingerprint.clone(),
            document: encode_plan(plan).to_string(),
            created_at: plan.created_at.to_rfc3339(),
            expires_at: plan.expires_at.to_rfc3339(),
            bulk_plan_id: bulk_plan_id.map(str::to_string),
        }
    }
}

const INSERT_PLAN_SQL: &str = "INSERT INTO cli_action_plans
    (plan_id, plan_kind, agent_id, scope_id, revision, state, environment_fingerprint,
     plan_json, created_at, expires_at, bulk_plan_id)
    VALUES (?1, 'action', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)";

fn insert_plan(
    connection: &Connection,
    plan: &CliActionPlan,
    bulk_plan_id: Option<&str>,
) -> rusqlite::Result<()> {
    let row = PlanRow::of(plan, bulk_plan_id);
    connection.execute(
        INSERT_PLAN_SQL,
        params![
            row.plan_id,
            row.agent_id,
            LOCAL_DESKTOP,
            row.revision,
            row.state,
            row.fingerprint,
            row.document,
            row.created_at,
            row.expires_at,
            row.bulk_plan_id,
        ],
    )?;
    Ok(())
}

/// Attaching an item plan to its batch.
///
/// An upsert rather than an insert, because bulk preparation runs the ordinary single-action
/// planning path for every eligible tool -- which persists each plan before the batch exists -- and
/// then records the batch. Inserting again failed on the primary key and took the whole batch down
/// with it, on a real database. The in-memory double never saw it: its map overwrote by key.
const ATTACH_PLAN_SQL: &str = "INSERT INTO cli_action_plans
    (plan_id, plan_kind, agent_id, scope_id, revision, state, environment_fingerprint,
     plan_json, created_at, expires_at, bulk_plan_id)
    VALUES (?1, 'action', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
    ON CONFLICT(plan_id) DO UPDATE SET
      revision = excluded.revision,
      state = excluded.state,
      environment_fingerprint = excluded.environment_fingerprint,
      plan_json = excluded.plan_json,
      expires_at = excluded.expires_at,
      bulk_plan_id = excluded.bulk_plan_id";

fn insert_plan_tx(
    transaction: &Transaction<'_>,
    plan: &CliActionPlan,
    bulk_plan_id: Option<&str>,
) -> rusqlite::Result<()> {
    let row = PlanRow::of(plan, bulk_plan_id);
    transaction.execute(
        ATTACH_PLAN_SQL,
        params![
            row.plan_id,
            row.agent_id,
            LOCAL_DESKTOP,
            row.revision,
            row.state,
            row.fingerprint,
            row.document,
            row.created_at,
            row.expires_at,
            row.bulk_plan_id,
        ],
    )?;
    Ok(())
}

fn insert_bulk(transaction: &Transaction<'_>, bulk: &CliBulkActionPlan) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO cli_action_plans
         (plan_id, plan_kind, agent_id, scope_id, revision, state, environment_fingerprint,
          plan_json, created_at, expires_at)
         VALUES (?1, 'bulk', NULL, ?2, ?3, 'draft', ?4, ?5, ?6, ?7)",
        params![
            bulk.id.as_str(),
            LOCAL_DESKTOP,
            bulk.revision,
            bulk.environment_fingerprint,
            encode_bulk_plan(bulk).to_string(),
            bulk.created_at.to_rfc3339(),
            bulk.expires_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "environment_repository_tests.rs"]
mod tests;
