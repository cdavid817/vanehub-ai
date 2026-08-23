//! SQLite adapters for connector storage, the credential-store port, and the schema they use.

mod active_snapshot;
#[cfg(test)]
mod active_snapshot_tests;
mod credentials;
#[cfg(test)]
mod credentials_tests;
mod persistence_schema;
#[cfg(test)]
mod sqlite_connectors_tests;
mod sqlite_definitions;
mod sqlite_instances;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use active_snapshot::{ExtensionPlatformActiveConnector, UnknownActiveConnector};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use credentials::{OsConnectorCredentials, CONNECTOR_CREDENTIAL_SERVICE};
pub(crate) use persistence_schema::apply_connector_schema;
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_definitions::{
    SqliteConnectorDefinitionRepository, SqliteConnectorSubjectRepository,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use sqlite_instances::{
    SqliteConnectorBindingRepository, SqliteConnectorInstanceRepository,
};

/// Whether a failure is SQLite refusing a reference that does not exist.
///
/// Matched on the extended result code rather than on the message text: the message is a
/// human-facing string that has changed across SQLite releases, and the code is part of the
/// interface. A foreign-key refusal misread as a storage failure turns "no such connector" into
/// "the database is broken", and the caller retries instead of reporting.
pub(crate) fn is_foreign_key_violation(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY
    )
}
