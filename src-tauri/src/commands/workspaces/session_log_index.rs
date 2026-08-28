//! The reads the Logs tab needs beyond a page of rows.
//!
//! Grouped in one file because they are one contract against one index: an authoritative row, a
//! summary figure, a resume watermark, what the index can claim, and the repair that fills it in.
//! Each is a thin translation; none of them decides anything.

use super::session_log_mapper::{coverage_to_dto, log_command_error, SessionLogCoverageDto};
use crate::commands::error::CommandError;
use crate::contexts::operations::log_api::{IndexedSessionLogDetail, SessionLogApi};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogRecordDto {
    pub(crate) id: String,
    pub(crate) timestamp: String,
    pub(crate) level: &'static str,
    pub(crate) category: String,
    pub(crate) message: String,
    pub(crate) context: std::collections::BTreeMap<String, String>,
    pub(crate) coverage: SessionLogCoverageDto,
}

fn detail_to_dto(detail: IndexedSessionLogDetail) -> SessionLogRecordDto {
    SessionLogRecordDto {
        id: detail.record.record_id,
        timestamp: detail.record.occurred_at,
        level: detail.record.level.token(),
        category: detail.record.category,
        message: detail.record.message,
        context: detail.record.context,
        coverage: coverage_to_dto(detail.coverage),
    }
}

/// The authoritative row behind a live notice.
///
/// A notice says which record; this says what it is. One shape for a row rather than two that can
/// disagree, and no log content on the event bus.
#[tauri::command]
pub(crate) fn get_session_log_record(
    logs: State<'_, SessionLogApi>,
    record_id: String,
) -> Result<SessionLogRecordDto, CommandError> {
    logs.record(&record_id)
        .map(detail_to_dto)
        .map_err(log_command_error)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogSummaryDto {
    pub(crate) session_id: String,
    pub(crate) new_errors: u32,
    pub(crate) coverage: SessionLogCoverageDto,
}

/// The narrow figure the workspace summary badge shows.
///
/// Carries its coverage so a zero next to `partial` renders as "not observed" rather than as "no
/// errors happened" — and so the badge never has to mount the Logs query to produce a number.
#[tauri::command]
pub(crate) fn get_session_log_summary(
    logs: State<'_, SessionLogApi>,
    session_id: String,
) -> Result<SessionLogSummaryDto, CommandError> {
    logs.summary(&session_id)
        .map(|summary| SessionLogSummaryDto {
            session_id: summary.session_id,
            new_errors: summary.new_errors,
            coverage: coverage_to_dto(summary.coverage),
        })
        .map_err(log_command_error)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogBootstrapDto {
    pub(crate) watermark_sequence: i64,
    pub(crate) coverage: SessionLogCoverageDto,
}

/// Where a subscriber resumes from.
///
/// Read after the listener is already registered. The other order loses every notice published in
/// the window, and the sequences the subscriber then sees are contiguous — so nothing downstream
/// can tell anything was missed.
#[tauri::command]
pub(crate) fn get_session_log_subscription_bootstrap(
    logs: State<'_, SessionLogApi>,
) -> Result<SessionLogBootstrapDto, CommandError> {
    logs.subscription_bootstrap()
        .map(|bootstrap| SessionLogBootstrapDto {
            watermark_sequence: bootstrap.watermark_sequence,
            coverage: coverage_to_dto(bootstrap.coverage),
        })
        .map_err(log_command_error)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogExportSourcesDto {
    pub(crate) source_files: Vec<String>,
    pub(crate) oldest_available_at: Option<String>,
    pub(crate) newest_available_at: Option<String>,
    pub(crate) redacted: bool,
}

/// What an export would read: the redacted files, never the index.
///
/// Exposed so the Logs tab can say what an export covers before running it. Serving an export from
/// the projection would hand the user whatever it happened to hold — a subset during repair, a
/// stale set after a directory change — under a name that promises the log.
#[tauri::command]
pub(crate) fn get_session_log_export_sources(
    logs: State<'_, SessionLogApi>,
) -> Result<SessionLogExportSourcesDto, CommandError> {
    logs.export_preparation()
        .map(|preparation| SessionLogExportSourcesDto {
            source_files: preparation.source_files,
            oldest_available_at: preparation.oldest_available_at,
            newest_available_at: preparation.newest_available_at,
            redacted: preparation.redacted,
        })
        .map_err(log_command_error)
}

#[tauri::command]
pub(crate) fn get_session_log_coverage(
    logs: State<'_, SessionLogApi>,
    session_id: Option<String>,
) -> Result<SessionLogCoverageDto, CommandError> {
    logs.coverage(session_id.as_deref())
        .map(coverage_to_dto)
        .map_err(log_command_error)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogRepairDto {
    pub(crate) operation_id: String,
    pub(crate) state: &'static str,
    pub(crate) files_completed: u32,
    pub(crate) files_total: u32,
    pub(crate) records_indexed: u64,
    pub(crate) reason_code: Option<String>,
}

fn repair_to_dto(
    status: crate::contexts::operations::log_api::SessionLogBackfillStatus,
) -> SessionLogRepairDto {
    SessionLogRepairDto {
        operation_id: status.operation_id,
        state: status.state.token(),
        files_completed: status.files_completed,
        files_total: status.files_total,
        records_indexed: status.records_indexed,
        reason_code: status.reason_code,
    }
}

#[tauri::command]
pub(crate) fn get_session_log_repair_status(
    logs: State<'_, SessionLogApi>,
) -> Result<SessionLogRepairDto, CommandError> {
    Ok(repair_to_dto(logs.backfill_status()))
}

/// Runs one bounded repair pass off the main thread.
///
/// Reading and parsing log files is disk-bound; on a command thread it would freeze the window for
/// as long as the corpus took.
#[tauri::command]
pub(crate) async fn repair_session_log_index(
    logs: State<'_, SessionLogApi>,
) -> Result<SessionLogRepairDto, CommandError> {
    Ok(repair_to_dto(logs.inner().clone().repair_blocking().await))
}

/// Asks the running repair to stop. Committed checkpoints stay, so resuming is cheap.
#[tauri::command]
pub(crate) fn cancel_session_log_repair(
    logs: State<'_, SessionLogApi>,
) -> Result<(), CommandError> {
    logs.cancel_repair();
    Ok(())
}
