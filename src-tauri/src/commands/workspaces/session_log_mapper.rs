//! Between the Logs tab's wire shape and the operations context's indexed vocabulary.
//!
//! The public DTO does not change: the same command names, the same field names, the same page
//! shape the Logs tab already reads. What changed is where the rows come from — the index rather
//! than a file scan — and that is deliberately invisible from here.

use super::dto;
use crate::contexts::operations::log_api::{
    IndexedLogLevel, IndexedSessionLogPage, IndexedSessionLogQuery, IndexedSessionLogRecord,
    LogSortDirection, OperationsLogError, SessionLogCoverage, SessionLogFilters,
    SessionLogQueryScope,
};
use serde::Serialize;

/// The coverage a page carries, as the Logs tab reads it.
///
/// Additive to the existing page shape rather than replacing it. A client that predates this field
/// keeps working; one that reads it stops rendering an incomplete answer as a definitive one.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionLogCoverageDto {
    pub(crate) state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) oldest_available_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) newest_available_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) indexed_through: Option<String>,
    pub(crate) dropped_count: u32,
    pub(crate) truncated: bool,
    pub(crate) reason_codes: Vec<String>,
}

pub(crate) fn coverage_to_dto(coverage: SessionLogCoverage) -> SessionLogCoverageDto {
    SessionLogCoverageDto {
        state: coverage.state().token(),
        oldest_available_at: coverage.oldest_available_at,
        newest_available_at: coverage.newest_available_at,
        indexed_through: coverage.indexed_through,
        dropped_count: coverage.dropped_count,
        truncated: coverage.truncated,
        reason_codes: coverage.reason_codes,
    }
}

fn level_from_dto(level: dto::WorkspaceLogLevel) -> IndexedLogLevel {
    match level {
        dto::WorkspaceLogLevel::Error => IndexedLogLevel::Error,
        dto::WorkspaceLogLevel::Warn => IndexedLogLevel::Warn,
        dto::WorkspaceLogLevel::Info => IndexedLogLevel::Info,
        dto::WorkspaceLogLevel::Debug => IndexedLogLevel::Debug,
    }
}

fn level_to_dto(level: IndexedLogLevel) -> dto::WorkspaceLogLevel {
    match level {
        IndexedLogLevel::Error => dto::WorkspaceLogLevel::Error,
        IndexedLogLevel::Warn => dto::WorkspaceLogLevel::Warn,
        IndexedLogLevel::Info => dto::WorkspaceLogLevel::Info,
        IndexedLogLevel::Debug => dto::WorkspaceLogLevel::Debug,
    }
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

pub(crate) fn indexed_query_from_dto(query: dto::SessionLogQuery) -> IndexedSessionLogQuery {
    IndexedSessionLogQuery {
        scope: SessionLogQueryScope {
            session_id: blank_to_none(Some(query.session_id)),
            seat_id: blank_to_none(query.seat_id),
            ..SessionLogQueryScope::default()
        },
        filters: SessionLogFilters {
            levels: query.levels.into_iter().map(level_from_dto).collect(),
            search: blank_to_none(Some(query.search)),
            sort: match query.sort {
                Some(dto::SessionLogSortDto::OldestFirst) => LogSortDirection::OldestFirst,
                // Absent and `newestFirst` are the same request, which is what keeps a client that
                // predates the field fingerprinting identically to one that sends the default.
                Some(dto::SessionLogSortDto::NewestFirst) | None => LogSortDirection::NewestFirst,
            },
            ..SessionLogFilters::default()
        },
        cursor: blank_to_none(query.cursor),
        limit: query.limit,
    }
}

fn record_to_dto(record: IndexedSessionLogRecord) -> dto::SessionLogEntry {
    dto::SessionLogEntry {
        // The record's own id, which is also what a live notice names. One identity, so a row a
        // notice announced and a row a page returned are recognisably the same row.
        id: record.record_id,
        timestamp: record.occurred_at,
        level: level_to_dto(record.level),
        category: record.category,
        message: record.message,
        context: record.context,
    }
}

pub(crate) fn indexed_page_to_dto(page: IndexedSessionLogPage) -> dto::SessionLogPage {
    dto::SessionLogPage {
        items: page.items.into_iter().map(record_to_dto).collect(),
        truncated: page.truncated,
        next_cursor: page.next_cursor,
        coverage: Some(coverage_to_dto(page.coverage)),
    }
}

/// A typed log failure, as a command returns it.
///
/// The message is the error's stable code rather than prose: a client acts differently on a stale
/// cursor than on an unavailable index, and a sentence would make that a string-matching exercise
/// that breaks on translation.
pub(crate) fn log_command_error(error: OperationsLogError) -> crate::commands::error::CommandError {
    use crate::commands::error::CommandError;
    match error {
        OperationsLogError::RecordNotFound => CommandError::storage(error.code()),
        _ => CommandError::validation(error.code()),
    }
}
