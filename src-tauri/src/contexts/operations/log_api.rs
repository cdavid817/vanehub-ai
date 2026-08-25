//! The published boundary for indexed session-log reads.
//!
//! Separate from `OperationsApi` because these two have different callers and different lifetimes:
//! the operation facade is held by every context that starts work, and this is held by the command
//! layer that answers the Logs tab. Merging them would make every producer depend on the query
//! index it has no business knowing about.

use super::application::SessionLogQueryService;
pub(crate) use super::application::{IndexedLogLevel, LogSortDirection};
pub(crate) use super::application::{
    IndexedSessionLogDetail, IndexedSessionLogPage, IndexedSessionLogQuery,
    IndexedSessionLogRecord, LogFailureQuery, LogFailureSummary, OperationsLogError,
    SafeLogExportPreparation, SessionLogBackfillState, SessionLogBackfillStatus,
    SessionLogCoverage, SessionLogCoverageState, SessionLogFilters, SessionLogQueryScope,
    SessionLogSubscriptionBootstrap, SessionLogSummary,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SessionLogApi {
    service: Arc<SessionLogQueryService>,
}

impl SessionLogApi {
    pub(crate) fn new(service: Arc<SessionLogQueryService>) -> Self {
        Self { service }
    }

    /// Reads a page off the caller's thread.
    ///
    /// The index is SQLite, and a query that had to wait on a busy writer would hold the webview
    /// for as long as the writer took. The wrapper lives here rather than in the command so the
    /// command stays a transport mapping with no scheduling decision of its own.
    pub(crate) async fn query_blocking(
        &self,
        query: IndexedSessionLogQuery,
    ) -> Result<IndexedSessionLogPage, OperationsLogError> {
        let service = self.service.clone();
        tauri::async_runtime::spawn_blocking(move || service.query(&query))
            .await
            .map_err(|_| OperationsLogError::IndexUnavailable("log_query_task_failed"))?
    }

    /// The authoritative row behind a live notice.
    pub(crate) fn record(
        &self,
        record_id: &str,
    ) -> Result<IndexedSessionLogDetail, OperationsLogError> {
        self.service.record(record_id)
    }

    pub(crate) fn summary(
        &self,
        session_id: &str,
    ) -> Result<SessionLogSummary, OperationsLogError> {
        self.service.summary(session_id)
    }

    /// Error rows grouped by category, for a session-run report.
    ///
    /// Published rather than left to a caller's paging loop for the same reason the page query is
    /// bounded: a consumer that read error rows to count them would read the whole corpus, and one
    /// that read a page would report a page count under a session count's name.
    pub(crate) fn failure_summary(
        &self,
        query: &LogFailureQuery,
    ) -> Result<LogFailureSummary, OperationsLogError> {
        self.service.failure_summary(query)
    }

    pub(crate) fn subscription_bootstrap(
        &self,
    ) -> Result<SessionLogSubscriptionBootstrap, OperationsLogError> {
        self.service.subscription_bootstrap()
    }

    /// What an export may read. Redacted files, never the index.
    pub(crate) fn export_preparation(
        &self,
    ) -> Result<SafeLogExportPreparation, OperationsLogError> {
        self.service.export_preparation()
    }

    pub(crate) fn coverage(
        &self,
        session_id: Option<&str>,
    ) -> Result<SessionLogCoverage, OperationsLogError> {
        self.service.coverage(session_id)
    }

    pub(crate) fn backfill_status(&self) -> SessionLogBackfillStatus {
        self.service.backfill_status()
    }

    pub(crate) fn cancel_repair(&self) {
        self.service.cancel_repair();
    }

    /// Runs a repair off the caller's thread.
    ///
    /// Reading and parsing log files is disk-bound and unbounded in wall-clock terms; doing it on a
    /// command thread would freeze the window for as long as the corpus took.
    pub(crate) async fn repair_blocking(&self) -> SessionLogBackfillStatus {
        let service = self.service.clone();
        tauri::async_runtime::spawn_blocking(move || service.repair())
            .await
            .unwrap_or_else(|_| SessionLogBackfillStatus {
                operation_id: String::new(),
                state: SessionLogBackfillState::Failed,
                files_completed: 0,
                files_total: 0,
                records_indexed: 0,
                started_at: None,
                updated_at: None,
                reason_code: Some("log_repair_task_failed".to_string()),
            })
    }
}

/// A log index over a database directory, behind its port.
///
/// Published here rather than letting a caller construct `SqliteLogIndexRepository` directly,
/// because the concrete repository is this context's persistence and reaching for it from another
/// context is exactly what ARCH-NATIVE-003 forbids — including from a test, where the coupling is
/// no less real for being compiled out of the product.
///
/// The one caller is the export-authority fixture in `workspaces`, which has to be able to put the
/// index and the files into states where they disagree. That comparison is the whole point of the
/// fixture, and it cannot be made from inside either context alone.
#[cfg(test)]
pub(crate) fn assemble_log_index_for_tests(
    database_directory: &std::path::Path,
) -> Result<Arc<dyn super::application::SessionLogIndexRepository>, String> {
    let database = crate::platform::database::NativeDatabase::new(database_directory.to_path_buf())
        .map_err(|error| error.to_string())?;
    Ok(Arc::new(
        super::infrastructure::SqliteLogIndexRepository::new(database),
    ))
}
