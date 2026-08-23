// The settings flow that moves bindings lands with Task Group 7; see `sqlite_definitions.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! The SQLite adapter for user enablement.
//!
//! Two methods write, and the difference between them is the whole file. `set` is a move: it takes
//! the revision the caller read, compares it inside the write transaction, and refuses if someone
//! else moved the binding in between. `seed_default` is not a move at all — it writes only where
//! there is nothing, and a binding that already exists is left exactly as it is.
//!
//! Keeping them apart is deliberate. A single `upsert_default` would be one `ON CONFLICT DO
//! UPDATE` away from silently re-enabling, on some future upgrade, a Hook the user turned off —
//! and the user would find out when it ran.

use crate::contexts::tooling::lifecycle_hooks::application::HookBindingRepository;
use crate::contexts::tooling::lifecycle_hooks::domain::{
    decide_seed, HookBinding, HookBindingError, HookGlobalId, HookScope, SeedOutcome,
};
use crate::platform::database::{begin_write_transaction, NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension, Transaction};
use std::sync::Arc;

use super::is_foreign_key_violation;

/// The revision a binding that does not exist yet is treated as having.
///
/// A caller that read "no binding" and then writes passes this, so creating and moving go through
/// the same compare-and-swap and a create cannot silently overwrite a binding that appeared in
/// between.
pub(crate) const ABSENT_BINDING_REVISION: i64 = 0;

pub(crate) struct SqliteHookBindingRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteHookBindingRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, HookBindingError> {
        self.database
            .connection()
            .map_err(|error| HookBindingError::Storage(error.to_string()))
    }
}

/// A foreign-key failure is the database saying no such subject, which is a domain answer rather
/// than a storage failure.
fn binding_error(error: rusqlite::Error) -> HookBindingError {
    if is_foreign_key_violation(&error) {
        HookBindingError::UnknownSubject
    } else {
        HookBindingError::Storage(error.to_string())
    }
}

/// Reads the stored `(enabled, revision, updated_at)` for one binding, inside whatever transaction
/// the caller is holding.
fn read_binding(
    transaction: &Transaction<'_>,
    hook: &HookGlobalId,
    scope: &HookScope,
) -> Result<Option<(bool, i64, String)>, HookBindingError> {
    transaction
        .query_row(
            "SELECT enabled, revision, updated_at FROM lifecycle_hook_bindings \
             WHERE hook_global_id = ?1 AND scope_kind = ?2 AND scope_key = ?3",
            params![hook.as_str(), scope.kind().as_str(), scope.key()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? != 0,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(binding_error)
}

impl HookBindingRepository for SqliteHookBindingRepository {
    fn binding(
        &self,
        hook: &HookGlobalId,
        scope: &HookScope,
    ) -> Result<Option<HookBinding>, HookBindingError> {
        let connection = self.connection()?;
        let row: Option<(i64, i64, String)> = connection
            .query_row(
                "SELECT enabled, revision, updated_at FROM lifecycle_hook_bindings \
                 WHERE hook_global_id = ?1 AND scope_kind = ?2 AND scope_key = ?3",
                params![hook.as_str(), scope.kind().as_str(), scope.key()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(binding_error)?;

        Ok(row.map(|(enabled, revision, updated_at)| HookBinding {
            hook: hook.clone(),
            scope: scope.clone(),
            enabled: enabled != 0,
            revision,
            updated_at,
        }))
    }

    fn set(
        &self,
        hook: &HookGlobalId,
        scope: &HookScope,
        enabled: bool,
        expected_revision: i64,
        at: &str,
    ) -> Result<HookBinding, HookBindingError> {
        let connection = self.connection()?;
        let transaction = begin_write_transaction(&connection)
            .map_err(|error| HookBindingError::Storage(error.to_string()))?;

        // Read inside the transaction, so two editors racing cannot both see the same revision.
        let current = read_binding(&transaction, hook, scope)?;
        let current_revision = current
            .as_ref()
            .map_or(ABSENT_BINDING_REVISION, |(_, revision, _)| *revision);
        if current_revision != expected_revision {
            return Err(HookBindingError::StaleRevision {
                expected: expected_revision,
                actual: current_revision,
            });
        }

        let revision = current_revision + 1;
        transaction
            .execute(
                "INSERT INTO lifecycle_hook_bindings \
                     (hook_global_id, scope_kind, scope_key, enabled, revision, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT(hook_global_id, scope_kind, scope_key) DO UPDATE SET \
                     enabled = excluded.enabled, \
                     revision = excluded.revision, \
                     updated_at = excluded.updated_at",
                params![
                    hook.as_str(),
                    scope.kind().as_str(),
                    scope.key(),
                    i64::from(enabled),
                    revision,
                    at,
                ],
            )
            .map_err(binding_error)?;
        transaction
            .commit()
            .map_err(|error| HookBindingError::Storage(error.to_string()))?;

        Ok(HookBinding {
            hook: hook.clone(),
            scope: scope.clone(),
            enabled,
            revision,
            updated_at: at.to_string(),
        })
    }

    fn seed_default(
        &self,
        hook: &HookGlobalId,
        scope: &HookScope,
        enabled: bool,
        at: &str,
    ) -> Result<SeedOutcome, HookBindingError> {
        let connection = self.connection()?;
        let transaction = begin_write_transaction(&connection)
            .map_err(|error| HookBindingError::Storage(error.to_string()))?;

        let existing =
            read_binding(&transaction, hook, scope)?.map(|(enabled, revision, at)| HookBinding {
                hook: hook.clone(),
                scope: scope.clone(),
                enabled,
                revision,
                updated_at: at,
            });

        let outcome = decide_seed(existing.as_ref());
        if outcome == SeedOutcome::Seeded {
            // A plain INSERT, not an upsert. If a binding appeared between the read and here, the
            // primary key refuses the write rather than overwriting it -- which is the same answer
            // `Preserved` would have given, arrived at by the database instead of by a check.
            transaction
                .execute(
                    "INSERT INTO lifecycle_hook_bindings \
                         (hook_global_id, scope_kind, scope_key, enabled, revision, updated_at) \
                     VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                    params![
                        hook.as_str(),
                        scope.kind().as_str(),
                        scope.key(),
                        i64::from(enabled),
                        at,
                    ],
                )
                .map_err(binding_error)?;
        }
        transaction
            .commit()
            .map_err(|error| HookBindingError::Storage(error.to_string()))?;
        Ok(outcome)
    }

    fn bindings(&self, hook: &HookGlobalId) -> Result<Vec<HookBinding>, HookBindingError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT scope_kind, scope_key, enabled, revision, updated_at \
                 FROM lifecycle_hook_bindings WHERE hook_global_id = ?1 \
                 ORDER BY scope_kind, scope_key",
            )
            .map_err(binding_error)?;
        let rows = statement
            .query_map(params![hook.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(binding_error)?;

        let mut bindings = Vec::new();
        for row in rows {
            let (kind, key, enabled, revision, updated_at) = row.map_err(binding_error)?;
            bindings.push(HookBinding {
                hook: hook.clone(),
                scope: HookScope::parse(&kind, &key)
                    .map_err(|error| HookBindingError::Storage(error.code().to_string()))?,
                enabled: enabled != 0,
                revision,
                updated_at,
            });
        }
        Ok(bindings)
    }
}
