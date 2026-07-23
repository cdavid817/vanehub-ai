use super::storage_mapping::storage_error;
use super::SqliteExecutionTimelineRepository;
use crate::contexts::execution_observability::application::ExecutionTelemetryError;
use rusqlite::{params, OptionalExtension};

const MAINTENANCE_INTERVAL_DAYS: f64 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RetentionOutcome {
    pub(crate) ran: bool,
    pub(crate) deleted_runs: usize,
}

impl SqliteExecutionTimelineRepository {
    pub(crate) fn maintain_retention(
        &self,
        now: &str,
        retention_days: u16,
    ) -> Result<RetentionOutcome, ExecutionTelemetryError> {
        if !(1..=90).contains(&retention_days) {
            return Err(storage_error(
                "execution retention must be between 1 and 90 days",
            ));
        }
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| storage_error(error.to_string()))?;
        let last_run = transaction
            .query_row(
                "SELECT last_retention_at FROM execution_observability_settings WHERE singleton_id = 1",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(|error| storage_error(error.to_string()))?
            .flatten();
        if let Some(last_run) = last_run {
            let elapsed_days = transaction
                .query_row(
                    "SELECT julianday(?1) - julianday(?2)",
                    params![now, last_run],
                    |row| row.get::<_, Option<f64>>(0),
                )
                .map_err(|error| storage_error(error.to_string()))?
                .unwrap_or_default();
            if elapsed_days < MAINTENANCE_INTERVAL_DAYS {
                return Ok(RetentionOutcome {
                    ran: false,
                    deleted_runs: 0,
                });
            }
        }

        let deleted_runs = transaction
            .execute(
                "DELETE FROM execution_runs WHERE julianday(started_at) < julianday(?1, '-' || ?2 || ' days')",
                params![now, retention_days],
            )
            .map_err(|error| storage_error(error.to_string()))?;
        transaction
            .execute(
                "UPDATE execution_observability_settings SET retention_days = ?1, last_retention_at = ?2, updated_at = ?2 WHERE singleton_id = 1",
                params![retention_days, now],
            )
            .map_err(|error| storage_error(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| storage_error(error.to_string()))?;
        Ok(RetentionOutcome {
            ran: true,
            deleted_runs,
        })
    }
}
