//! Writing the report a reader is looking at into a directory they picked.
//!
//! The exported bytes are the same DTO the read command returns, serialized once. An export that
//! rendered its own structure would be a second view of the same report, free to drift from the one
//! on screen — and the drift would only ever be noticed by somebody comparing a saved file against
//! a panel they no longer have open.
//!
//! The destination is a directory, never a path: the caller picks where, and the application layer
//! picks the name. A caller that supplied both could aim the write at a file that already exists.

use super::{evidence_dto, get_session_run_report, report_dto};
use crate::contexts::sessions::api::SessionRunReportService;
use tauri::State;

/// Exports the bounded report as JSON.
///
/// A dismissed picker arrives as an empty directory and comes back as `cancelled`. That is not a
/// failure: choosing not to export is a choice, and reporting it as an error would put an alert in
/// front of somebody who pressed Escape.
///
/// The parameter list is the wire contract, not a signature somebody chose: it is the read
/// command's arguments plus a destination, and collapsing them into one struct would give the
/// export a different payload shape from the read it exports.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub(crate) fn export_session_run_report(
    reports: State<'_, SessionRunReportService>,
    session_id: Option<String>,
    run_ids: Option<Vec<String>>,
    seat_ids: Option<Vec<String>>,
    from: Option<String>,
    to: Option<String>,
    group_by: Option<String>,
    destination_directory: Option<String>,
) -> Result<report_dto::SessionRunReportExportDto, evidence_dto::EvidenceCommandErrorDto> {
    export_report(
        reports.inner(),
        report_dto::SessionRunReportRequestDto {
            session_id,
            run_ids,
            seat_ids,
            from,
            to,
            group_by,
        },
        destination_directory.unwrap_or_default(),
    )
}

/// The body, separated from the `State` wrapper so tests exercise this code rather than a copy.
pub(super) fn export_report(
    reports: &SessionRunReportService,
    request: report_dto::SessionRunReportRequestDto,
    destination_directory: String,
) -> Result<report_dto::SessionRunReportExportDto, evidence_dto::EvidenceCommandErrorDto> {
    // The scope is validated by building the report, so an over-large request is refused before
    // anything reaches the filesystem rather than after a file has been written.
    let session_id = request.session_id.clone().unwrap_or_default();
    let report = get_session_run_report::session_run_report(reports, request)?;
    let content = serde_json::to_string_pretty(&report).map_err(|_| {
        evidence_dto::EvidenceCommandErrorDto {
            reason_code: "report_export_failed".to_string(),
        }
    })?;

    let path = reports
        .export(&destination_directory, &session_id, &content)
        .map_err(|error| evidence_dto::EvidenceCommandErrorDto {
            reason_code: error.code().to_string(),
        })?;

    Ok(report_dto::SessionRunReportExportDto {
        status: if path.is_some() {
            "exported"
        } else {
            "cancelled"
        },
        path,
    })
}
