//! CLI parameters.
//!
//! The monolith this replaced is gone: everything the product uses lives in the four subdomain
//! modules below. What remains here is the legacy `cli_parameter_settings` table's schema, which
//! migration 81 still needs because the v2 profile reader dual-reads legacy rows, and the error
//! type that schema returns. Both retire with the dual-read.

pub(crate) mod api;
pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;

use rusqlite::Connection;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum CliParametersError {
    /// Only the legacy validator produces this; `apply_schema` cannot fail this way.
    #[cfg(test)]
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Repository(String),
}

impl From<rusqlite::Error> for CliParametersError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Repository(error.to_string())
    }
}

pub(crate) fn apply_schema(conn: &Connection) -> Result<(), CliParametersError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS cli_parameter_settings (
            agent_id TEXT NOT NULL,
            parameter_id TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            value_json TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (agent_id, parameter_id),
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
        );
        "#,
    )?;
    Ok(())
}

/// The pre-cutover catalog, validator, renderer and settings facade, kept only so the
/// cutover's equivalence suite can recompute the old argv and so the legacy write path
/// stays under test. Nothing in it is reachable from a command, a launch, or bootstrap.
#[cfg(test)]
pub(crate) mod legacy_baseline;
#[cfg(test)]
pub(crate) use legacy_baseline::{baseline_preview_args, CliParameterLaunchScope};

#[cfg(test)]
mod cutover_regression_tests;
