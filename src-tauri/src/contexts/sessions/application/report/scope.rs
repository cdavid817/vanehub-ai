//! What a report may be asked for.
//!
//! Every bound here exists because the request comes from outside. A report reads from five
//! contexts and aggregates across all of them, so an unbounded request is not a slow query — it is
//! five slow queries and a join, and the caller who sent it is a renderer that will ask again on
//! the next keystroke.
//!
//! Refused rather than clamped, throughout. A clamp answers a different question than the one that
//! was asked, and returns it under the asked question's name: a caller who requested three hundred
//! runs and silently received fifty would report on fifty and say three hundred.

use super::models::ReportGroupBy;

/// How many runs one report may span.
///
/// Generous for a session — most have a handful — and finite because `runIds` arrives from a
/// client. A report over ten thousand runs is not a report anybody reads; it is a denial of
/// service with a friendly name.
pub(crate) const MAX_REPORT_RUNS: usize = 200;
/// How many seats one report may span. Seats are few by construction; this is a guard, not a limit
/// anybody reaches.
pub(crate) const MAX_REPORT_SEATS: usize = 64;

/// The validated request.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReportScope {
    pub(crate) session_id: String,
    /// Empty means every run in the session. A concrete list narrows to exactly those.
    pub(crate) run_ids: Vec<String>,
    pub(crate) seat_ids: Vec<String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) group_by: ReportGroupBy,
}

/// What a request was refused for, as a stable code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportScopeError {
    MissingSession,
    TooManyRuns,
    TooManySeats,
    InvalidRange,
    InvalidGroupBy,
}

impl ReportScopeError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::MissingSession => "report_session_required",
            Self::TooManyRuns => "report_too_many_runs",
            Self::TooManySeats => "report_too_many_seats",
            Self::InvalidRange => "report_invalid_range",
            Self::InvalidGroupBy => "report_invalid_group_by",
        }
    }
}

/// The unvalidated request, as it arrives.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReportScopeRequest {
    pub(crate) session_id: String,
    pub(crate) run_ids: Vec<String>,
    pub(crate) seat_ids: Vec<String>,
    pub(crate) from: Option<String>,
    pub(crate) to: Option<String>,
    pub(crate) group_by: Option<String>,
}

pub(crate) fn validate_report_scope(
    request: ReportScopeRequest,
) -> Result<ReportScope, ReportScopeError> {
    let session_id = request.session_id.trim().to_string();
    if session_id.is_empty() {
        // A report over every session is not a thing this surface offers, and treating a missing
        // session as "all of them" would answer a question nobody asked with everything there is.
        return Err(ReportScopeError::MissingSession);
    }

    let run_ids = distinct_non_empty(request.run_ids);
    if run_ids.len() > MAX_REPORT_RUNS {
        return Err(ReportScopeError::TooManyRuns);
    }
    let seat_ids = distinct_non_empty(request.seat_ids);
    if seat_ids.len() > MAX_REPORT_SEATS {
        return Err(ReportScopeError::TooManySeats);
    }

    let from = timestamp(request.from)?;
    let to = timestamp(request.to)?;
    if let (Some(start), Some(end)) = (&from, &to) {
        // An inverted range selects nothing, and a report over nothing is indistinguishable from a
        // session that did nothing. Refusing says which one it is.
        if parse_ms(start) > parse_ms(end) {
            return Err(ReportScopeError::InvalidRange);
        }
    }

    let group_by = match request.group_by.as_deref().map(str::trim) {
        None | Some("") => ReportGroupBy::default(),
        // An unrecognised dimension is refused rather than defaulted. Falling back to `run` would
        // return a report grouped one way under a request that asked for another.
        Some(value) => ReportGroupBy::parse(value).ok_or(ReportScopeError::InvalidGroupBy)?,
    };

    Ok(ReportScope {
        session_id,
        run_ids,
        seat_ids,
        from,
        to,
        group_by,
    })
}

/// Trimmed, de-duplicated, and with blanks dropped.
///
/// A blank id in the list would narrow to records carrying an empty correlation, which is none of
/// them — so a request with one stray empty string would report on nothing and look like a session
/// that did nothing.
fn distinct_non_empty(values: Vec<String>) -> Vec<String> {
    let mut seen = Vec::new();
    for value in values {
        let value = value.trim().to_string();
        if value.is_empty() || seen.contains(&value) {
            continue;
        }
        seen.push(value);
    }
    seen
}

fn timestamp(value: Option<String>) -> Result<Option<String>, ReportScopeError> {
    let Some(value) = value else { return Ok(None) };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if parse_ms(&value).is_none() {
        return Err(ReportScopeError::InvalidRange);
    }
    Ok(Some(value))
}

fn parse_ms(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|parsed| parsed.timestamp_millis())
}
