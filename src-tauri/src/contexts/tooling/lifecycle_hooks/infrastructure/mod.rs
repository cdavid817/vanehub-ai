//! SQLite adapters for Hook storage, and the schema they are written against.

mod persistence_schema;
mod sqlite_bindings;
mod sqlite_definitions;
mod sqlite_executions;
#[cfg(test)]
mod sqlite_hooks_tests;

pub(crate) use persistence_schema::apply_lifecycle_hook_schema;
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_bindings::{SqliteHookBindingRepository, ABSENT_BINDING_REVISION};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_definitions::{SqliteHookDefinitionRepository, SqliteHookSubjectRepository};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_executions::SqliteHookExecutionRepository;

/// Whether a failure is SQLite refusing a reference that does not exist.
///
/// Matched on the extended result code rather than on the message text. The message is a
/// human-facing string that has changed across SQLite releases before; the code is part of the
/// interface. Getting this wrong in the safe direction is worse than it sounds — a foreign-key
/// refusal misread as a storage failure turns "no such Hook" into "the database is broken", and
/// the caller retries instead of reporting.
pub(crate) fn is_foreign_key_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY
    )
}
