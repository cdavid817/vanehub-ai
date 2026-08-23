// The install and activation flows that call these land with Task Group 4; the dispatch engine
// that reads them with Task Group 7.
#![cfg_attr(not(test), allow(dead_code))]

//! SQLite adapters for Hook subjects and their versioned definitions.
//!
//! Both are written so the database enforces the rule and the adapter reports it, rather than the
//! adapter checking and the database trusting. A definition conflict is decided by reading the row
//! that holds the pair *inside the write transaction*, so two installs racing on the same snapshot
//! cannot both see "unrecorded" and both insert.

use crate::contexts::tooling::lifecycle_hooks::application::{
    HookDefinitionRepository, HookSubjectRepository,
};
use crate::contexts::tooling::lifecycle_hooks::domain::{
    decide_definition, DefinitionDigest, DefinitionOutcome, HookDefinitionRevision, HookEvent,
    HookGlobalId, HookOrigin, HookSubject, SnapshotRef,
};
use crate::platform::database::{begin_write_transaction, NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension};
use std::sync::Arc;

use super::is_foreign_key_violation;

pub(crate) struct SqliteHookSubjectRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteHookSubjectRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, String> {
        self.database
            .connection()
            .map_err(|error| error.to_string())
    }
}

impl HookSubjectRepository for SqliteHookSubjectRepository {
    /// `DO NOTHING` rather than an upsert: `first_seen_at` is written once.
    ///
    /// Re-seeding a built-in on every start is not a new sighting, and rewriting the timestamp
    /// would erase the only record of when the Hook entered this installation.
    fn ensure(&self, subject: &HookSubject) -> Result<(), String> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO lifecycle_hook_subjects (hook_global_id, origin, first_seen_at) \
                 VALUES (?1, ?2, ?3) \
                 ON CONFLICT(hook_global_id) DO NOTHING",
                params![
                    subject.hook.as_str(),
                    subject.origin.as_str(),
                    subject.first_seen_at,
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn get(&self, hook: &HookGlobalId) -> Result<Option<HookSubject>, String> {
        let connection = self.connection()?;
        let row: Option<(String, String)> = connection
            .query_row(
                "SELECT origin, first_seen_at FROM lifecycle_hook_subjects \
                 WHERE hook_global_id = ?1",
                params![hook.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;

        let Some((origin, first_seen_at)) = row else {
            return Ok(None);
        };
        Ok(Some(HookSubject {
            hook: hook.clone(),
            origin: HookOrigin::parse(&origin).ok_or_else(|| "invalid_hook_origin".to_string())?,
            first_seen_at,
        }))
    }

    fn all(&self) -> Result<Vec<HookSubject>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT hook_global_id, origin, first_seen_at FROM lifecycle_hook_subjects \
                 ORDER BY hook_global_id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?;

        let mut subjects = Vec::new();
        for row in rows {
            let (hook, origin, first_seen_at) = row.map_err(|error| error.to_string())?;
            subjects.push(HookSubject {
                hook: HookGlobalId::parse(&hook).map_err(|error| error.code().to_string())?,
                origin: HookOrigin::parse(&origin)
                    .ok_or_else(|| "invalid_hook_origin".to_string())?,
                first_seen_at,
            });
        }
        Ok(subjects)
    }
}

pub(crate) struct SqliteHookDefinitionRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteHookDefinitionRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, String> {
        self.database
            .connection()
            .map_err(|error| error.to_string())
    }
}

impl HookDefinitionRepository for SqliteHookDefinitionRepository {
    fn record(&self, revision: &HookDefinitionRevision) -> Result<DefinitionOutcome, String> {
        let connection = self.connection()?;
        let transaction =
            begin_write_transaction(&connection).map_err(|error| error.to_string())?;

        // Read inside the transaction, so two installs racing on the same snapshot cannot both see
        // "unrecorded". A deferred transaction could not do this at all: SQLite refuses the
        // read-to-write lock upgrade without honouring `busy_timeout`.
        let recorded: Option<(String, String, String)> = transaction
            .query_row(
                "SELECT event, definition_digest, recorded_at \
                 FROM lifecycle_hook_definition_revisions \
                 WHERE hook_global_id = ?1 AND snapshot_id = ?2",
                params![revision.hook.as_str(), revision.snapshot.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;

        let held = match recorded {
            Some(row) => Some(read_revision(&revision.hook, &revision.snapshot, row)?),
            None => None,
        };

        let outcome = decide_definition(revision, held.as_ref());
        if matches!(outcome, DefinitionOutcome::Recorded) {
            transaction
                .execute(
                    "INSERT INTO lifecycle_hook_definition_revisions \
                         (hook_global_id, snapshot_id, event, definition_digest, recorded_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        revision.hook.as_str(),
                        revision.snapshot.as_str(),
                        revision.event.as_str(),
                        revision.digest.as_str(),
                        revision.recorded_at,
                    ],
                )
                .map_err(|error| {
                    if is_foreign_key_violation(&error) {
                        "unknown_hook_subject".to_string()
                    } else {
                        error.to_string()
                    }
                })?;
        }
        // A conflict commits nothing, and the stored row is untouched: a rebuild cannot change
        // what an already-installed snapshot means.
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(outcome)
    }

    fn recorded(
        &self,
        hook: &HookGlobalId,
        snapshot: &SnapshotRef,
    ) -> Result<Option<HookDefinitionRevision>, String> {
        let connection = self.connection()?;
        let row: Option<(String, String, String)> = connection
            .query_row(
                "SELECT event, definition_digest, recorded_at \
                 FROM lifecycle_hook_definition_revisions \
                 WHERE hook_global_id = ?1 AND snapshot_id = ?2",
                params![hook.as_str(), snapshot.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;

        row.map(|row| read_revision(hook, snapshot, row))
            .transpose()
    }

    fn revisions(&self, hook: &HookGlobalId) -> Result<Vec<HookDefinitionRevision>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT snapshot_id, event, definition_digest, recorded_at \
                 FROM lifecycle_hook_definition_revisions \
                 WHERE hook_global_id = ?1 ORDER BY recorded_at DESC, snapshot_id DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![hook.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?;

        let mut revisions = Vec::new();
        for row in rows {
            let (snapshot, event, digest, recorded_at) = row.map_err(|error| error.to_string())?;
            let snapshot =
                SnapshotRef::parse(&snapshot).map_err(|error| error.code().to_string())?;
            revisions.push(read_revision(
                hook,
                &snapshot,
                (event, digest, recorded_at),
            )?);
        }
        Ok(revisions)
    }
}

/// Rebuilds a revision from a row, refusing one whose stored values no longer parse.
///
/// A row that cannot be read is an error rather than an absence: "no definition" and "a definition
/// this build cannot describe" call for completely different handling, and treating the second as
/// the first would look like the Hook was never contributed.
fn read_revision(
    hook: &HookGlobalId,
    snapshot: &SnapshotRef,
    row: (String, String, String),
) -> Result<HookDefinitionRevision, String> {
    let (event, digest, recorded_at) = row;
    Ok(HookDefinitionRevision {
        hook: hook.clone(),
        snapshot: snapshot.clone(),
        event: HookEvent::parse(&event).ok_or_else(|| "invalid_hook_event".to_string())?,
        digest: DefinitionDigest::parse(&digest).map_err(|error| error.code().to_string())?,
        recorded_at,
    })
}
