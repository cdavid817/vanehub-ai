use super::evidence_dto::{
    EvidenceChangesDto, EvidenceCommandErrorDto, EvidenceFiltersDto, EvidenceLogsDto,
    EvidencePairDto, EvidenceRelatedCountsDto, EvidenceRunStateDto, EvidenceScopeDto,
    EvidenceShellsDto, EvidenceSubscriptionBootstrapDto, EvidenceUsageDto, EvidenceVerificationDto,
    ExecutionRecordDetailDto, ExecutionRecordDetailViewDto, ExecutionRecordDto,
    ExecutionRecordPageDto, QueryCoverageDto, WorkspaceEvidenceSummaryDto,
};
use crate::contexts::execution_observability::api::evidence::{
    fidelity_token, parse_fidelity_token, parse_status_token, status_token,
    EvidenceApplicationError, EvidenceCorrelationCounts, EvidenceQueryScope, EvidenceRecordPage,
    EvidenceSeatId, EvidenceSessionId, EvidenceSubscriptionBootstrap, ExecutionRecordDetailFields,
    ExecutionRecordDetailView, ExecutionRecordFilters, ExecutionRecordKind,
    ExecutionRecordProjection, QueryCoverage, WorkspaceEvidenceSummary, DEFAULT_EVIDENCE_PAGE_SIZE,
    MAX_EVIDENCE_PAGE_SIZE,
};
use crate::contexts::execution_observability::api::ExecutionFidelity;
use std::collections::BTreeMap;

/// Stable reason codes the frontend localizes. A handler returns one of these and nothing else,
/// so a message can never carry the cursor, the query, a path, or a payload back to the client.
pub(crate) mod error_codes {
    pub(crate) const INVALID_REQUEST: &str = "evidence_invalid_request";
    pub(crate) const RECORD_NOT_FOUND: &str = "evidence_record_not_found";
    pub(crate) const CURSOR_FILTER_MISMATCH: &str = "cursor_filter_mismatch";
    pub(crate) const INVALID_CURSOR: &str = "evidence_invalid_cursor";
    pub(crate) const UNAVAILABLE: &str = "evidence_unavailable";
}

pub(crate) fn invalid_request() -> EvidenceCommandErrorDto {
    EvidenceCommandErrorDto {
        reason_code: error_codes::INVALID_REQUEST.to_string(),
    }
}

/// Maps an application failure to the one code the frontend is allowed to see.
///
/// `Storage` and `Domain` both collapse to a generic code on purpose: their `Display` text names
/// the SQLite failure or the field that was rejected, and a client that can read either learns the
/// schema and, in the domain case, part of the value that was refused.
pub(crate) fn command_error(error: EvidenceApplicationError) -> EvidenceCommandErrorDto {
    let reason_code = match error {
        EvidenceApplicationError::RecordNotFound => error_codes::RECORD_NOT_FOUND,
        EvidenceApplicationError::CursorFilterMismatch => error_codes::CURSOR_FILTER_MISMATCH,
        EvidenceApplicationError::InvalidCursor => error_codes::INVALID_CURSOR,
        EvidenceApplicationError::Storage(_) => error_codes::UNAVAILABLE,
        EvidenceApplicationError::Domain(_) => error_codes::INVALID_REQUEST,
    };
    EvidenceCommandErrorDto {
        reason_code: reason_code.to_string(),
    }
}

pub(crate) fn parse_session(value: &str) -> Result<EvidenceSessionId, EvidenceCommandErrorDto> {
    EvidenceSessionId::parse(value).map_err(|_| invalid_request())
}

pub(crate) fn parse_scope(
    dto: EvidenceScopeDto,
) -> Result<EvidenceQueryScope, EvidenceCommandErrorDto> {
    let session_id = match dto.session_id.as_deref() {
        Some(value) => Some(parse_session(value)?),
        None => None,
    };
    let seat_id = match dto.seat_id.as_deref() {
        Some(value) => Some(EvidenceSeatId::parse(value).map_err(|_| invalid_request())?),
        None => None,
    };
    Ok(EvidenceQueryScope {
        session_id,
        seat_id,
        run_id: dto.run_id,
        trace_id: dto.trace_id,
        span_id: dto.span_id,
        operation_id: dto.operation_id,
        command_id: dto.command_id,
    })
}

/// An unrecognised filter token is rejected rather than dropped. Silently ignoring it would return
/// a wider result set than the caller asked for, and the caller has no way to tell.
pub(crate) fn parse_filters(
    dto: EvidenceFiltersDto,
) -> Result<ExecutionRecordFilters, EvidenceCommandErrorDto> {
    let mut filters = ExecutionRecordFilters::default();
    for kind in dto.kinds.unwrap_or_default() {
        filters
            .kinds
            .push(ExecutionRecordKind::parse(&kind).ok_or_else(invalid_request)?);
    }
    for status in dto.statuses.unwrap_or_default() {
        filters
            .statuses
            .push(parse_status_token(&status).ok_or_else(invalid_request)?);
    }
    for fidelity in dto.fidelities.unwrap_or_default() {
        filters
            .fidelities
            .push(parse_fidelity_token(&fidelity).ok_or_else(invalid_request)?);
    }
    filters.search = dto.search.filter(|value| !value.trim().is_empty());
    Ok(filters)
}

/// A limit above the maximum is clamped rather than refused: the page is still correct, and the
/// coverage the caller receives already tells it whether more remains.
pub(crate) fn clamp_limit(limit: Option<u32>) -> usize {
    match limit {
        Some(0) | None => DEFAULT_EVIDENCE_PAGE_SIZE,
        Some(value) => (value as usize).min(MAX_EVIDENCE_PAGE_SIZE),
    }
}

pub(crate) fn coverage_dto(coverage: &QueryCoverage) -> QueryCoverageDto {
    QueryCoverageDto {
        state: coverage.state().as_str().to_string(),
        reason_codes: coverage
            .reason_codes()
            .iter()
            .map(|code| code.as_str().to_string())
            .collect(),
        oldest_available_at: coverage.oldest_available_at().map(str::to_string),
        newest_available_at: coverage.newest_available_at().map(str::to_string),
        indexed_through_at: coverage.indexed_through_at().map(str::to_string),
        dropped_count: coverage.dropped_count(),
        truncated: coverage.truncated(),
    }
}

pub(crate) fn summary_dto(summary: &WorkspaceEvidenceSummary) -> WorkspaceEvidenceSummaryDto {
    // The figures this context does not own read as zero next to an `unavailable` coverage that
    // names each of them. Group 8 replaces them with real counts; until then the panel must render
    // "not observed" rather than "none happened".
    let unowned = summary
        .unowned_sources
        .iter()
        .map(|source| source.coverage_state.as_str().to_string())
        .next()
        .unwrap_or_else(|| "unavailable".to_string());
    WorkspaceEvidenceSummaryDto {
        session_id: summary.session_id.as_str().to_string(),
        generated_at: summary.generated_at.clone(),
        coverage: coverage_dto(&summary.coverage),
        run_state: EvidenceRunStateDto {
            status: summary
                .run_status
                .map(status_token)
                .unwrap_or("incomplete")
                .to_string(),
            run_id: summary.run_id.clone(),
            started_at: summary.run_started_at.clone(),
        },
        changes: EvidenceChangesDto {
            changed_files: 0,
            unviewed_files: 0,
        },
        execution_records: EvidencePairDto {
            running: summary.running_records,
            failed: summary.failed_records,
        },
        shells: EvidenceShellsDto { live: 0 },
        logs: EvidenceLogsDto { new_errors: 0 },
        traces: EvidencePairDto {
            running: summary.running_records,
            failed: summary.failed_records,
        },
        verification: EvidenceVerificationDto {
            passed: summary.verification_passed,
            failed: summary.verification_failed,
        },
        usage: EvidenceUsageDto {
            reported_tokens: None,
            coverage: unowned,
        },
    }
}

pub(crate) fn record_dto(
    record: &ExecutionRecordProjection,
    coverage: &QueryCoverage,
) -> ExecutionRecordDto {
    ExecutionRecordDto {
        id: record.record_id.clone(),
        kind: record.kind.as_str().to_string(),
        session_id: record.session_id.as_str().to_string(),
        run_id: record.run_id.clone(),
        trace_id: record.trace_id.clone(),
        span_id: record.span_id.clone(),
        operation_id: record.operation_id.clone(),
        agent_id: record.agent_id.clone(),
        seat_id: record
            .seat_id
            .as_ref()
            .map(|seat| seat.as_str().to_string()),
        // Carried through as an `Option` from the projection to the wire. Nothing along this path
        // substitutes `occurred_at`, `ended_at`, or an end-minus-duration for a start that was
        // never observed.
        started_at: record.started_at.clone(),
        ended_at: record.ended_at.clone(),
        duration_ms: record.duration_ms,
        status: status_token(record.status).to_string(),
        fidelity: fidelity_token(record.fidelity).to_string(),
        coverage: coverage_dto(coverage),
        detail: detail_dto(record),
    }
}

fn detail_dto(record: &ExecutionRecordProjection) -> ExecutionRecordDetailDto {
    match &record.detail {
        ExecutionRecordDetailFields::Command {
            command_id,
            runtime_kind,
            redacted_display,
            cwd_display,
            exit_code,
            signal,
            output_availability,
            output_truncated,
        } => ExecutionRecordDetailDto::Command {
            command_id: command_id.as_str().to_string(),
            runtime_kind: runtime_kind.as_str().to_string(),
            redacted_display: redacted_display.clone(),
            cwd_display: cwd_display.clone(),
            exit_code: *exit_code,
            signal: signal.clone(),
            output_availability: output_availability.as_str().to_string(),
            output_truncated: *output_truncated,
        },
        ExecutionRecordDetailFields::Tool {
            tool_call_id,
            tool_name,
        } => ExecutionRecordDetailDto::Tool {
            tool_call_id: tool_call_id.as_ref().map(|id| id.as_str().to_string()),
            tool_name: tool_name.clone(),
            // Read off the fidelity the producer asserted rather than assumed: a record the
            // runtime observed directly is native, one reconstructed after the fact is not.
            source: match record.fidelity {
                ExecutionFidelity::Inferred => "message-history",
                _ => "native",
            }
            .to_string(),
        },
        ExecutionRecordDetailFields::Verification {
            name,
            outcome,
            passed_count,
            failed_count,
        } => ExecutionRecordDetailDto::Verification {
            verification_name: name.clone(),
            outcome: outcome.as_str().to_string(),
            passed_count: *passed_count,
            failed_count: *failed_count,
        },
        ExecutionRecordDetailFields::Delegation {
            parent_agent_id,
            child_agent_id,
            attempt,
        } => ExecutionRecordDetailDto::Delegation {
            parent_agent_id: parent_agent_id.clone(),
            child_agent_id: child_agent_id.clone(),
            attempt: *attempt,
        },
    }
}

pub(crate) fn page_dto(page: &EvidenceRecordPage) -> ExecutionRecordPageDto {
    ExecutionRecordPageDto {
        items: page
            .items
            .iter()
            .map(|record| record_dto(record, &page.coverage))
            .collect(),
        next_cursor: page.next_cursor.clone(),
        coverage: coverage_dto(&page.coverage),
    }
}

pub(crate) fn detail_view_dto(
    view: &ExecutionRecordDetailView,
    coverage: &QueryCoverage,
) -> ExecutionRecordDetailViewDto {
    ExecutionRecordDetailViewDto {
        record: record_dto(&view.record, coverage),
        related_counts: related_counts_dto(&view.counts),
        // Classifications and counts only; a value the record carried is never promoted here.
        safe_attributes: safe_attributes(&view.record),
        error_reason_code: view.error_reason_code.clone(),
    }
}

fn related_counts_dto(counts: &EvidenceCorrelationCounts) -> EvidenceRelatedCountsDto {
    EvidenceRelatedCountsDto {
        // Logs belong to another context and this one has no port to it, so the count is 0 and the
        // record's coverage carries `evidence_source_not_owned`.
        logs: 0,
        commands: counts.commands,
        files: counts.file_mutations,
        findings: counts.verifications,
        usage_observations: counts.usage_observations,
    }
}

fn safe_attributes(record: &ExecutionRecordProjection) -> BTreeMap<String, String> {
    let mut attributes = BTreeMap::new();
    attributes.insert("kind".to_string(), record.kind.as_str().to_string());
    attributes.insert(
        "fidelity".to_string(),
        fidelity_token(record.fidelity).to_string(),
    );
    attributes.insert(
        "status".to_string(),
        status_token(record.status).to_string(),
    );
    if let ExecutionRecordDetailFields::Command {
        runtime_kind,
        output_availability,
        ..
    } = &record.detail
    {
        attributes.insert("runtimeKind".to_string(), runtime_kind.as_str().to_string());
        attributes.insert(
            "outputAvailability".to_string(),
            output_availability.as_str().to_string(),
        );
    }
    attributes
}

pub(crate) fn bootstrap_dto(
    bootstrap: &EvidenceSubscriptionBootstrap,
) -> EvidenceSubscriptionBootstrapDto {
    EvidenceSubscriptionBootstrapDto {
        session_id: bootstrap.session_id.as_str().to_string(),
        watermark_sequence: bootstrap.watermark_sequence,
        coverage: coverage_dto(&bootstrap.coverage),
    }
}
