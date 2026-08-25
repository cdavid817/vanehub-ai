//! What a log failure is allowed to say, and what a query means once it crosses the boundary.
//!
//! An error message from this surface reaches a UI, a log file and, when someone reports a
//! problem, a screenshot. Everything the index knows about a failure — the SQL it ran, the file it
//! read, the cursor bytes it rejected, the text of the record it could not find — is either private
//! to the user or useless to the reader, and all of it is one interpolation away from being in that
//! screenshot. So the message is a code, and this is where that is enforced.

use super::dto;
use super::session_log_mapper::{indexed_query_from_dto, log_command_error};
use crate::commands::error::CommandErrorCategory;
use crate::contexts::operations::log_api::{IndexedLogLevel, LogSortDirection, OperationsLogError};

fn query(session: &str) -> dto::SessionLogQuery {
    dto::SessionLogQuery {
        session_id: session.to_string(),
        seat_id: None,
        run_id: None,
        trace_id: None,
        span_id: None,
        operation_id: None,
        agent_id: None,
        levels: Vec::new(),
        search: String::new(),
        cursor: None,
        limit: None,
        sort: None,
    }
}

/// Every log error crosses as a stable code.
///
/// The codes are matched by the client, so prose would make behaviour depend on a sentence — and a
/// translated sentence would change the behaviour. The scan below is the second half: no error
/// carries the SQL, the path, the cursor it rejected, or the log line it was reading.
#[test]
fn a_log_failure_crosses_as_a_code_and_carries_nothing_else() {
    let errors = [
        OperationsLogError::InvalidCursor,
        OperationsLogError::CursorFilterMismatch,
        OperationsLogError::InvalidQuery("log_page_limit_exceeded"),
        OperationsLogError::RecordNotFound,
        OperationsLogError::IndexUnavailable("log_index_not_backfilled"),
        OperationsLogError::RepairFailed("log_repair_source_unreadable"),
    ];

    // Fragments that would only ever appear by interpolating something the caller must not see.
    let forbidden = [
        "SELECT",
        "FROM",
        "WHERE",
        "unified_log_query_index",
        "sqlite",
        "\\",
        "/Users/",
        "C:",
        ".log",
        "occurred_at_ms",
    ];

    for error in errors {
        let code = error.code();
        let mapped = log_command_error(error);
        let message = mapped.message().to_string();

        assert!(message.contains(code), "{message:?} does not name its code");
        for fragment in forbidden {
            assert!(
                !message.contains(fragment),
                "{message:?} leaks {fragment:?}"
            );
        }
    }
}

/// A missing record is storage, everything else is the caller's request.
///
/// The category is what decides whether the UI offers a retry or a correction, so collapsing the
/// two would offer a retry for a page limit the caller has to change first.
#[test]
fn a_missing_record_is_storage_while_a_bad_request_is_validation() {
    assert_eq!(
        log_command_error(OperationsLogError::RecordNotFound).category(),
        CommandErrorCategory::Infrastructure
    );
    for error in [
        OperationsLogError::InvalidCursor,
        OperationsLogError::CursorFilterMismatch,
        OperationsLogError::InvalidQuery("log_page_limit_exceeded"),
    ] {
        assert_eq!(
            log_command_error(error).category(),
            CommandErrorCategory::Validation,
            "a request the caller can fix was reported as infrastructure"
        );
    }
}

/// Blank is absent, not a filter that matches nothing.
///
/// The Logs tab sends `""` for a cleared search box and for a seat it has not chosen. Carrying
/// those through as filters would return an empty page and read as "no logs", which is the one
/// answer a log viewer must never give wrongly.
#[test]
fn a_blank_filter_is_absent_rather_than_a_filter_that_matches_nothing() {
    let mut input = query("session-1");
    input.search = "   ".to_string();
    input.seat_id = Some(String::new());
    input.cursor = Some("  ".to_string());

    let mapped = indexed_query_from_dto(input);

    assert_eq!(mapped.scope.session_id.as_deref(), Some("session-1"));
    assert!(mapped.scope.seat_id.is_none());
    assert!(mapped.filters.search.is_none());
    // A blank cursor is absent rather than malformed: a cleared client sends it, and refusing it
    // would turn "show me the first page" into an error.
    assert!(mapped.cursor.is_none());
}

/// Every correlation the client can send reaches the scope the index filters on.
///
/// The index has filtered on all of these since it existed; what was missing was the wire shape,
/// so the Logs tab had no way to ask. A field that crossed into the DTO but not into the scope
/// would narrow nothing and return the unfiltered page — which reads as "this run touched every
/// log line", the opposite of what the filter is for.
#[test]
fn every_correlation_filter_reaches_the_scope_the_index_narrows_by() {
    let mut input = query("session-1");
    input.seat_id = Some("seat-1".to_string());
    input.run_id = Some("run-1".to_string());
    input.trace_id = Some("trace-1".to_string());
    input.span_id = Some("span-1".to_string());
    input.operation_id = Some("operation-1".to_string());
    input.agent_id = Some("agent-1".to_string());

    let scope = indexed_query_from_dto(input).scope;

    assert_eq!(scope.session_id.as_deref(), Some("session-1"));
    assert_eq!(scope.seat_id.as_deref(), Some("seat-1"));
    assert_eq!(scope.run_id.as_deref(), Some("run-1"));
    assert_eq!(scope.trace_id.as_deref(), Some("trace-1"));
    assert_eq!(scope.span_id.as_deref(), Some("span-1"));
    assert_eq!(scope.operation_id.as_deref(), Some("operation-1"));
    assert_eq!(scope.agent_id.as_deref(), Some("agent-1"));
}

/// A blank correlation is absent, not a filter matching records that carry an empty one.
#[test]
fn a_blank_correlation_narrows_nothing() {
    let mut input = query("session-1");
    input.run_id = Some(String::new());
    input.trace_id = Some("   ".to_string());

    let scope = indexed_query_from_dto(input).scope;

    assert!(scope.run_id.is_none());
    assert!(scope.trace_id.is_none());
}

/// An absent sort and an explicit `newestFirst` are the same request.
///
/// They have to be, because the sort joins the cursor's filter fingerprint: if they differed, a
/// client that started paginating before it learned to send the field would have every cursor
/// refused the moment it started sending it.
#[test]
fn an_absent_sort_fingerprints_the_same_as_the_default_it_stands_for() {
    let mut absent = query("session-1");
    absent.levels = vec![dto::WorkspaceLogLevel::Error];
    let mut explicit = absent.clone();
    explicit.sort = Some(dto::SessionLogSortDto::NewestFirst);
    let mut reversed = absent.clone();
    reversed.sort = Some(dto::SessionLogSortDto::OldestFirst);

    let absent = indexed_query_from_dto(absent);
    let explicit = indexed_query_from_dto(explicit);
    let reversed = indexed_query_from_dto(reversed);

    assert_eq!(absent.filters.sort, LogSortDirection::NewestFirst);
    assert_eq!(explicit.filters.sort, LogSortDirection::NewestFirst);
    assert_eq!(reversed.filters.sort, LogSortDirection::OldestFirst);
    assert_eq!(
        absent.filters.levels,
        vec![IndexedLogLevel::Error],
        "the level filter did not survive the crossing"
    );
}
