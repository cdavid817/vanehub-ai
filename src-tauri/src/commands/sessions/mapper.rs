use super::dto;
use crate::contexts::communications::domain::ConnectorKind;
use crate::contexts::operations::api::{OperationKind, OperationTask};
use crate::contexts::operations::domain::OperationStatus;
use crate::contexts::sessions::api::{
    CategoryRecord, ChatConfigurationValues, MessageRecord, NewRemoteWorkspace, NewSessionRequest,
    NewSessionWorkspace, NewWorktree, SessionActivation, SessionChatConfiguration,
    SessionCreationOperation, SessionExportFormat, SessionExportResult, SessionLifecycle,
    SessionOwner, SessionRecord, SessionRecoveryReport as DomainSessionRecoveryReport,
    SessionRecoveryStatus, SessionRecoverySummary, SessionSearchMatchKind, SessionSearchResult,
    SessionSeat, SessionSeatRoleSnapshot, SessionUsageStatistics, SessionsError,
    UsageStatisticsRange,
};
use crate::contexts::skill_evolution_evidence::api::FeedbackSummary;
use std::collections::BTreeMap;

pub(super) fn recovery_report_to_dto(
    report: &DomainSessionRecoveryReport,
) -> dto::SessionRecoveryReport {
    dto::SessionRecoveryReport {
        report_id: report.report_id().to_string(),
        session_id: report.session_id().to_string(),
        recovery_revision: report.recovery_revision(),
        trigger: report.trigger(),
        observed_lifecycle: report.observed_lifecycle().to_string(),
        observed_execution_run_id: report.observed_execution_run_id().map(str::to_string),
        decision: report.decision(),
        reason_codes: report.reason_codes().to_vec(),
        evidence_refs: report.evidence_refs().to_vec(),
        created_at: report.created_at().to_string(),
    }
}

pub(super) fn recovery_summary_to_dto(
    summary: SessionRecoverySummary,
) -> Result<dto::SessionRecoverySummary, SessionsError> {
    Ok(dto::SessionRecoverySummary {
        session: session_to_dto(summary.session)?,
        latest_report: summary.latest_report.as_ref().map(recovery_report_to_dto),
    })
}

pub(super) fn creation_request(input: dto::CreateSessionInput) -> NewSessionRequest {
    NewSessionRequest {
        agent_id: input.agent_id,
        seats: input
            .seats
            .into_iter()
            .map(|seat| SessionSeat {
                seat_id: seat.seat_id.unwrap_or_default(),
                agent_id: seat.agent_id,
                role_id: seat.role_id,
                role_snapshot: seat.role_snapshot.map(role_snapshot_from_dto),
                joined_at: seat.joined_at.unwrap_or_default(),
                left_at: seat.left_at,
                provider_thread_id: None,
            })
            .collect(),
        interaction_mode: input.interaction_mode.as_str().to_string(),
        title: input.title,
        workspace: NewSessionWorkspace {
            folder: input.folder,
            project_path: input.project_path,
            remote_workspace: input.remote_workspace.map(|workspace| NewRemoteWorkspace {
                host: workspace.host,
                port: workspace.port,
                user: workspace.user,
                path: workspace.path,
                display_name: workspace.display_name,
                ssh_connection_id: workspace.ssh_connection_id,
            }),
            worktree: input.worktree.map(|worktree| NewWorktree {
                enabled: worktree.enabled,
                name: worktree.name,
            }),
        },
        owner: SessionOwner::desktop(),
        activation: SessionActivation::Activate,
    }
}

pub(super) fn creation_operation_to_dto(operation: &SessionCreationOperation) -> OperationTask {
    OperationTask {
        id: operation.id.clone(),
        execution_run_id: None,
        trace_id: None,
        kind: OperationKind::Workspace,
        status: OperationStatus::Running,
        related_entity_id: operation.related_entity_id.clone(),
        message: operation.message.clone(),
        logs: Vec::new(),
        result: None,
        error: None,
        created_at: operation.created_at.clone(),
        updated_at: operation.updated_at.clone(),
    }
}

pub(super) fn session_to_dto(session: SessionRecord) -> Result<dto::Session, SessionsError> {
    Ok(dto::Session {
        id: session.id().to_string(),
        title: session.aggregate.title().as_str().to_string(),
        agent_id: session.agent_id,
        seats: session
            .seats
            .into_iter()
            .map(|seat| dto::SessionSeat {
                seat_id: Some(seat.seat_id),
                agent_id: seat.agent_id,
                role_id: seat.role_id,
                role_snapshot: seat.role_snapshot.map(role_snapshot_to_dto),
                joined_at: Some(seat.joined_at),
                left_at: seat.left_at,
            })
            .collect(),
        interaction_mode: interaction_mode(&session.interaction_mode)?,
        lifecycle_state: lifecycle_state(session.aggregate.lifecycle()),
        recovery_status: recovery_status(session.aggregate.recovery().status()),
        recovery_revision: session.aggregate.recovery().recovery_revision(),
        state_revision: session.aggregate.recovery().state_revision(),
        history_revision: session.aggregate.recovery().history_revision(),
        active_execution_run_id: session
            .aggregate
            .recovery()
            .active_execution_run_id()
            .map(str::to_string),
        folder: session.workspace.folder,
        project_path: session.workspace.project_path,
        worktree_path: session.workspace.worktree_path,
        worktree_name: session.workspace.worktree_name,
        worktree_branch: session.workspace.worktree_branch,
        remote_workspace: session.workspace.remote_workspace.map(|workspace| {
            dto::RemoteWorkspace {
                host: workspace.host,
                port: workspace.port,
                user: workspace.user,
                path: workspace.path,
                display_name: workspace.display_name,
                uri: workspace.uri,
            }
        }),
        remote_ssh_connection_id: session
            .workspace
            .remote_ssh_binding
            .as_ref()
            .map(|binding| binding.connection_id.clone()),
        remote_ssh_connection_revision: session
            .workspace
            .remote_ssh_binding
            .as_ref()
            .map(|binding| binding.revision),
        runtime_session_id: session.runtime_session_id,
        category_id: session
            .aggregate
            .category_id()
            .map(|category_id| category_id.as_str().to_string()),
        source: dto::SessionSource {
            kind: session.aggregate.owner().kind().to_string(),
            connector: session
                .aggregate
                .owner()
                .connector_id()
                .and_then(ConnectorKind::parse),
        },
        execution_origin: dto::SessionExecutionOrigin {
            kind: session.execution_origin_kind,
            id: session.execution_origin_id,
        },
        pinned: session.aggregate.is_pinned(),
        archived: session.aggregate.is_archived(),
        created_at: session.created_at,
        updated_at: session.updated_at,
    })
}

pub(super) fn sessions_to_dto(
    sessions: Vec<SessionRecord>,
) -> Result<Vec<dto::Session>, SessionsError> {
    sessions.into_iter().map(session_to_dto).collect()
}

pub(super) fn search_results_to_dto(
    results: Vec<SessionSearchResult>,
) -> Result<Vec<dto::SessionSearchResult>, SessionsError> {
    results
        .into_iter()
        .map(|result| {
            Ok(dto::SessionSearchResult {
                session: session_to_dto(result.session)?,
                matches: result
                    .matches
                    .into_iter()
                    .map(|item| dto::SessionSearchMatch {
                        kind: match item.kind {
                            SessionSearchMatchKind::Title => "title",
                            SessionSearchMatchKind::Project => "project",
                            SessionSearchMatchKind::Message => "message",
                        }
                        .to_string(),
                        excerpt: item.excerpt,
                        message_id: item.message_id,
                    })
                    .collect(),
            })
        })
        .collect()
}

pub(super) fn category_to_dto(record: CategoryRecord) -> dto::SessionCategory {
    dto::SessionCategory {
        id: record.category.id().as_str().to_string(),
        name: record.category.name().as_str().to_string(),
        sort_order: record.category.sort_order(),
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

pub(super) fn categories_to_dto(records: Vec<CategoryRecord>) -> Vec<dto::SessionCategory> {
    records.into_iter().map(category_to_dto).collect()
}

pub(super) fn chat_configuration_request(
    session_id: String,
    config: dto::ChatConfig,
) -> SessionChatConfiguration {
    SessionChatConfiguration {
        session_id,
        agent_id: config.agent_id,
        interaction_mode: config.interaction_mode.as_str().to_string(),
        values: ChatConfigurationValues {
            execution_mode: config.execution_mode,
            provider_id: config.provider_id,
            model_id: config.model_id,
            reasoning_depth: config.reasoning_depth,
            streaming: config.streaming,
            thinking: config.thinking,
            long_context: config.long_context,
        },
    }
}

pub(super) fn chat_configuration_to_dto(
    configuration: SessionChatConfiguration,
    template: crate::contexts::permissions::api::PolicyTemplateName,
) -> Result<dto::ChatConfig, SessionsError> {
    let mode = crate::contexts::agent_runtime::application::SessionExecutionMode::parse(
        &configuration.values.execution_mode,
    )
    .ok_or_else(|| SessionsError::Validation("Unsupported execution mode.".to_string()))?;
    let effective = crate::contexts::agent_runtime::application::resolve_effective_execution_policy(
        template, mode,
    );
    Ok(dto::ChatConfig {
        agent_id: configuration.agent_id,
        interaction_mode: interaction_mode(&configuration.interaction_mode)?,
        execution_mode: configuration.values.execution_mode,
        agent_policy: Some(template.as_str().to_string()),
        effective_execution_policy: Some(effective.as_str().to_string()),
        provider_id: configuration.values.provider_id,
        model_id: configuration.values.model_id,
        reasoning_depth: configuration.values.reasoning_depth,
        streaming: configuration.values.streaming,
        thinking: configuration.values.thinking,
        long_context: configuration.values.long_context,
    })
}

pub(super) fn message_to_dto(record: MessageRecord) -> dto::ChatMessage {
    let file_references = record
        .message
        .file_references()
        .as_slice()
        .iter()
        .map(|reference| dto::ChatFileReference {
            id: reference.id().to_string(),
            path: reference.path().to_string(),
            name: reference.name().to_string(),
            size_bytes: reference.size_bytes(),
            content_hash: reference.content_hash().map(str::to_string),
            start_line: reference.line_range().map(|range| range.start()),
            end_line: reference.line_range().map(|range| range.end()),
        })
        .collect::<Vec<_>>();
    let tool_use = record.tool_use.and_then(|items| {
        items
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<dto::ToolUseBlock>, _>>()
            .ok()
    });

    dto::ChatMessage {
        speaker_seat_id: record.speaker_seat_id,
        id: record.message.id().as_str().to_string(),
        session_id: record.message.session_id().as_str().to_string(),
        seat_index: record.seat_index,
        role: record.message.role().as_str().to_string(),
        content: record.content,
        status: record.message.status().as_str().to_string(),
        tool_use,
        thinking_content: record.thinking_content,
        rich_blocks: record.rich_blocks,
        token_usage: record.token_usage.map(|usage| dto::TokenUsage {
            input: usage.input,
            output: usage.output,
        }),
        file_references: (!file_references.is_empty()).then_some(file_references),
        error: record.error,
        created_at: record.created_at,
        updated_at: record.updated_at,
        session_sequence: record.message.session_sequence(),
        execution_run_id: record.message.execution_run_id().map(str::to_string),
        feedback: None,
    }
}

pub(super) fn seats_from_dto(seats: Vec<dto::SessionSeat>) -> Vec<SessionSeat> {
    seats
        .into_iter()
        .map(|seat| SessionSeat {
            seat_id: seat.seat_id.unwrap_or_default(),
            agent_id: seat.agent_id,
            role_id: seat.role_id,
            role_snapshot: seat.role_snapshot.map(role_snapshot_from_dto),
            joined_at: seat.joined_at.unwrap_or_default(),
            left_at: seat.left_at,
            provider_thread_id: None,
        })
        .collect()
}

fn role_snapshot_from_dto(snapshot: dto::SessionSeatRoleSnapshot) -> SessionSeatRoleSnapshot {
    SessionSeatRoleSnapshot {
        role_name: snapshot.role_name,
        avatar: snapshot.avatar,
        color: snapshot.color,
        responsibility: snapshot.responsibility,
        agent_name: snapshot.agent_name,
        model_family: snapshot.model_family,
        cross_family_reviewer: snapshot.cross_family_reviewer,
    }
}

fn role_snapshot_to_dto(snapshot: SessionSeatRoleSnapshot) -> dto::SessionSeatRoleSnapshot {
    dto::SessionSeatRoleSnapshot {
        role_name: snapshot.role_name,
        avatar: snapshot.avatar,
        color: snapshot.color,
        responsibility: snapshot.responsibility,
        agent_name: snapshot.agent_name,
        model_family: snapshot.model_family,
        cross_family_reviewer: snapshot.cross_family_reviewer,
    }
}

pub(super) fn messages_to_dto(records: Vec<MessageRecord>) -> Vec<dto::ChatMessage> {
    records.into_iter().map(message_to_dto).collect()
}

pub(super) fn message_ids(records: &[MessageRecord]) -> Vec<String> {
    records
        .iter()
        .map(|record| record.message.id().as_str().to_string())
        .collect()
}

pub(super) fn messages_to_dto_with_feedback(
    records: Vec<MessageRecord>,
    feedback: &BTreeMap<String, FeedbackSummary>,
) -> Vec<dto::ChatMessage> {
    messages_to_dto(records)
        .into_iter()
        .map(|mut message| {
            message.feedback = feedback
                .get(&message.id)
                .map(|summary| dto::MessageFeedback {
                    state: summary
                        .state
                        .map(|state| format!("{state:?}").to_lowercase()),
                    revision: summary.revision,
                    correction_note: summary.sanitized_note.clone(),
                });
            message
        })
        .collect()
}

pub(super) fn export_format(format: dto::SessionExportFormat) -> SessionExportFormat {
    match format {
        dto::SessionExportFormat::Json => SessionExportFormat::Json,
        dto::SessionExportFormat::Markdown => SessionExportFormat::Markdown,
    }
}

pub(super) fn export_result_to_dto(result: SessionExportResult) -> dto::SessionExportResult {
    dto::SessionExportResult {
        status: result.status.to_string(),
        path: result.path,
        content: result.content,
    }
}

pub(super) fn usage_range(range: dto::UsageStatisticsRange) -> UsageStatisticsRange {
    match range {
        dto::UsageStatisticsRange::Today => UsageStatisticsRange::Today,
        dto::UsageStatisticsRange::Last7Days => UsageStatisticsRange::Last7Days,
        dto::UsageStatisticsRange::Last30Days => UsageStatisticsRange::Last30Days,
        dto::UsageStatisticsRange::All => UsageStatisticsRange::All,
    }
}

pub(super) fn usage_statistics_to_dto(statistics: SessionUsageStatistics) -> dto::UsageStatistics {
    dto::UsageStatistics {
        range: match statistics.range {
            UsageStatisticsRange::Today => dto::UsageStatisticsRange::Today,
            UsageStatisticsRange::Last7Days => dto::UsageStatisticsRange::Last7Days,
            UsageStatisticsRange::Last30Days => dto::UsageStatisticsRange::Last30Days,
            UsageStatisticsRange::All => dto::UsageStatisticsRange::All,
        },
        reported: dto::ReportedTokenTotals {
            input_tokens: statistics.reported.input_tokens,
            output_tokens: statistics.reported.output_tokens,
            cache_read_tokens: statistics.reported.cache_read_tokens,
            cache_creation_tokens: statistics.reported.cache_creation_tokens,
            total_tokens: statistics.reported.total_tokens,
        },
        estimated: dto::EstimatedCharacterTotals {
            input_characters: statistics.estimated.input_characters,
            output_characters: statistics.estimated.output_characters,
            total_characters: statistics.estimated.total_characters,
        },
        coverage: dto::UsageCoverage {
            reported_responses: statistics.coverage.reported_responses,
            estimated_responses: statistics.coverage.estimated_responses,
            total_responses: statistics.coverage.total_responses,
            reported_percent: statistics.coverage.reported_percent,
        },
        counted_sessions: statistics.counted_sessions,
        daily: statistics
            .daily
            .into_iter()
            .map(|point| dto::UsageStatisticsPoint {
                date: point.date,
                reported: dto::ReportedTokenTotals {
                    input_tokens: point.reported.input_tokens,
                    output_tokens: point.reported.output_tokens,
                    cache_read_tokens: point.reported.cache_read_tokens,
                    cache_creation_tokens: point.reported.cache_creation_tokens,
                    total_tokens: point.reported.total_tokens,
                },
                estimated: dto::EstimatedCharacterTotals {
                    input_characters: point.estimated.input_characters,
                    output_characters: point.estimated.output_characters,
                    total_characters: point.estimated.total_characters,
                },
                response_count: point.response_count,
            })
            .collect(),
        by_agent: statistics
            .by_agent
            .into_iter()
            .map(|agent| dto::UsageAgentBreakdown {
                agent_id: agent.agent_id,
                reported: dto::ReportedTokenTotals {
                    input_tokens: agent.reported.input_tokens,
                    output_tokens: agent.reported.output_tokens,
                    cache_read_tokens: agent.reported.cache_read_tokens,
                    cache_creation_tokens: agent.reported.cache_creation_tokens,
                    total_tokens: agent.reported.total_tokens,
                },
                estimated: dto::EstimatedCharacterTotals {
                    input_characters: agent.estimated.input_characters,
                    output_characters: agent.estimated.output_characters,
                    total_characters: agent.estimated.total_characters,
                },
                response_count: agent.response_count,
            })
            .collect(),
        generated_at: statistics.generated_at,
    }
}

pub(super) fn session_usage_summary_to_dto(
    summary: crate::contexts::sessions::api::SessionUsageSummary,
) -> dto::SessionUsageSummary {
    dto::SessionUsageSummary {
        session_id: summary.session_id,
        reported: dto::ReportedTokenTotals {
            input_tokens: summary.reported.input_tokens,
            output_tokens: summary.reported.output_tokens,
            cache_read_tokens: summary.reported.cache_read_tokens,
            cache_creation_tokens: summary.reported.cache_creation_tokens,
            total_tokens: summary.reported.total_tokens,
        },
        estimated: dto::EstimatedCharacterTotals {
            input_characters: summary.estimated.input_characters,
            output_characters: summary.estimated.output_characters,
            total_characters: summary.estimated.total_characters,
        },
        coverage: dto::UsageCoverage {
            reported_responses: summary.coverage.reported_responses,
            estimated_responses: summary.coverage.estimated_responses,
            total_responses: summary.coverage.total_responses,
            reported_percent: summary.coverage.reported_percent,
        },
        response_count: summary.response_count,
        generated_at: summary.generated_at,
    }
}

pub(super) fn token_usage_query(
    input: dto::TokenUsageSummaryInput,
) -> Result<crate::contexts::sessions::api::UsageSummaryQuery, SessionsError> {
    use crate::contexts::sessions::api::{MeasurementQuality, UsagePurpose, UsageStatus};
    Ok(crate::contexts::sessions::api::UsageSummaryQuery {
        session_id: input.session_id,
        message_id: input.message_id,
        generation_id: input.generation_id,
        agent_id: input.agent_id,
        provider_id: input.provider_id,
        model_id: input.model_id,
        purpose: input
            .purpose
            .as_deref()
            .map(|value| match value {
                "assistant-initial" => Ok(UsagePurpose::AssistantInitial),
                "tool-continuation" => Ok(UsagePurpose::ToolContinuation),
                "context-compaction" => Ok(UsagePurpose::ContextCompaction),
                "memory-extraction" => Ok(UsagePurpose::MemoryExtraction),
                "retry" => Ok(UsagePurpose::Retry),
                "terminal-interval" => Ok(UsagePurpose::TerminalInterval),
                _ => Err(invalid_usage_filter("purpose", value)),
            })
            .transpose()?,
        quality: input
            .quality
            .as_deref()
            .map(|value| match value {
                "reported" => Ok(MeasurementQuality::Reported),
                "reported-derived" => Ok(MeasurementQuality::ReportedDerived),
                "estimated" => Ok(MeasurementQuality::Estimated),
                _ => Err(invalid_usage_filter("quality", value)),
            })
            .transpose()?,
        status: input
            .status
            .as_deref()
            .map(|value| match value {
                "running" => Ok(UsageStatus::Running),
                "succeeded" => Ok(UsageStatus::Succeeded),
                "failed" => Ok(UsageStatus::Failed),
                "cancelled" => Ok(UsageStatus::Cancelled),
                _ => Err(invalid_usage_filter("status", value)),
            })
            .transpose()?,
        range_start: input.range_start,
        range_end: input.range_end,
        breakdown_limit: input.breakdown_limit.unwrap_or(10).clamp(1, 50),
        generated_at: String::new(),
    })
}

pub(super) fn token_usage_details_query(
    input: dto::TokenUsageDetailsInput,
) -> Result<crate::contexts::sessions::api::InvocationDetailQuery, SessionsError> {
    Ok(crate::contexts::sessions::api::InvocationDetailQuery {
        session_id: input.session_id,
        agent_id: input.agent_id,
        provider_id: input.provider_id,
        model_id: input.model_id,
        purpose: usage_purpose(input.purpose.as_deref())?,
        quality: usage_quality_filter(input.quality.as_deref())?,
        status: usage_status(input.status.as_deref())?,
        after_id: input.after_id,
        limit: input.limit.unwrap_or(25).clamp(1, 100),
    })
}

pub(super) fn token_usage_details_to_dto(
    page: crate::contexts::sessions::api::UsageDetailPage,
) -> dto::TokenUsageDetailsPage {
    dto::TokenUsageDetailsPage {
        schema_version: 1,
        invocations: page
            .invocations
            .into_iter()
            .map(|record| {
                let invocation = record.invocation;
                dto::ModelInvocation {
                    id: invocation.id,
                    generation_id: invocation.generation_id,
                    run_id: invocation.run_id,
                    operation_id: invocation.operation_id,
                    session_id: invocation.session_id,
                    message_id: invocation.message_id,
                    agent_id: invocation.agent_id,
                    provider_id: invocation.provider_id,
                    profile_id: invocation.profile_id,
                    endpoint_id: invocation.endpoint_id,
                    model_id: invocation.model_id,
                    interaction_kind: interaction_kind(invocation.interaction_kind).to_string(),
                    purpose: purpose_value(invocation.purpose).to_string(),
                    request_sequence: invocation.request_sequence,
                    attempt: invocation.attempt,
                    status: status_value(record.status).to_string(),
                    started_at: invocation.started_at,
                    completed_at: record.completed_at,
                }
            })
            .collect(),
        observations: page
            .observations
            .into_iter()
            .map(|record| {
                let observation = record.observation;
                dto::UsageObservation {
                    id: observation.id,
                    invocation_id: observation.invocation_id,
                    quality: quality_value(observation.quality).to_string(),
                    unit: accounting_unit(observation.unit).to_string(),
                    measurement_kind: measurement_kind(observation.measurement_kind).to_string(),
                    dimensions: token_dimensions(observation.dimensions),
                    cache_overlap: overlap_value(observation.cache_overlap).to_string(),
                    reasoning_overlap: overlap_value(observation.reasoning_overlap).to_string(),
                    normalization_version: observation.normalization_version,
                    source: observation.source,
                    source_revision: observation.source_revision,
                    event_at: observation.event_at,
                    observed_at: observation.observed_at,
                }
            })
            .collect(),
        next_cursor: page.next_cursor,
    }
}

pub(super) fn token_usage_summary_to_dto(
    summary: crate::contexts::sessions::api::UsageAccountingSummary,
) -> dto::TokenUsageSummary {
    dto::TokenUsageSummary {
        schema_version: 1,
        totals: usage_quality(summary.totals),
        user_response: usage_quality(summary.user_response),
        internal: usage_quality(summary.internal),
        counts: usage_counts(summary.counts),
        daily: summary
            .daily
            .into_iter()
            .map(|point| dto::UsageDailyPoint {
                local_date: point.local_date,
                totals: usage_quality(point.totals),
                counts: usage_counts(point.counts),
            })
            .collect(),
        breakdowns: summary
            .breakdowns
            .into_iter()
            .map(|breakdown| dto::UsageBreakdown {
                dimension: usage_breakdown_dimension(breakdown.dimension).to_string(),
                entries: breakdown
                    .entries
                    .into_iter()
                    .map(|entry| dto::UsageBreakdownEntry {
                        key: entry.key,
                        totals: usage_quality(entry.totals),
                        counts: usage_counts(entry.counts),
                    })
                    .collect(),
            })
            .collect(),
        generated_at: summary.generated_at,
    }
}

fn usage_quality(
    totals: crate::contexts::sessions::api::UsageQualityAggregate,
) -> dto::UsageQualityTotals {
    dto::UsageQualityTotals {
        reported: usage_measure(totals.reported),
        reported_derived: usage_measure(totals.reported_derived),
        estimated: usage_measure(totals.estimated),
    }
}

fn usage_measure(
    measure: crate::contexts::sessions::api::UsageMeasureAggregate,
) -> dto::UsageMeasure {
    dto::UsageMeasure {
        unit: accounting_unit(measure.unit).to_string(),
        dimensions: token_dimensions(measure.dimensions),
        headline_total: measure.headline_total,
        call_count: measure.call_count,
        observation_count: measure.observation_count,
    }
}

fn usage_counts(
    counts: crate::contexts::sessions::api::UsageEntityCounts,
) -> dto::UsageEntityCounts {
    dto::UsageEntityCounts {
        calls: counts.calls,
        generations: counts.generations,
        sessions: counts.sessions,
    }
}

fn usage_breakdown_dimension(
    dimension: crate::contexts::sessions::api::UsageBreakdownDimension,
) -> &'static str {
    use crate::contexts::sessions::api::UsageBreakdownDimension;
    match dimension {
        UsageBreakdownDimension::Agent => "agent",
        UsageBreakdownDimension::Provider => "provider",
        UsageBreakdownDimension::Model => "model",
        UsageBreakdownDimension::Purpose => "purpose",
        UsageBreakdownDimension::Quality => "quality",
        UsageBreakdownDimension::Status => "status",
    }
}

fn invalid_usage_filter(field: &str, value: &str) -> SessionsError {
    SessionsError::Validation(format!("unsupported Token usage {field}: {value}"))
}

fn accounting_unit(unit: crate::contexts::sessions::api::AccountingUnit) -> &'static str {
    match unit {
        crate::contexts::sessions::api::AccountingUnit::Tokens => "tokens",
        crate::contexts::sessions::api::AccountingUnit::Characters => "characters",
    }
}

fn token_dimensions(
    value: crate::contexts::sessions::api::TokenDimensions,
) -> dto::TokenDimensions {
    dto::TokenDimensions {
        input: value.input,
        output: value.output,
        cached_input: value.cached_input,
        cache_write_input: value.cache_write_input,
        reasoning_output: value.reasoning_output,
        provider_total: value.provider_total,
    }
}

fn interaction_kind(value: crate::contexts::sessions::api::UsageInteractionKind) -> &'static str {
    match value {
        crate::contexts::sessions::api::UsageInteractionKind::ManagedCli => "managed-cli",
        crate::contexts::sessions::api::UsageInteractionKind::TerminalCli => "terminal-cli",
        crate::contexts::sessions::api::UsageInteractionKind::NativeApi => "native-api",
    }
}

fn measurement_kind(value: crate::contexts::sessions::api::MeasurementKind) -> &'static str {
    match value {
        crate::contexts::sessions::api::MeasurementKind::Interval => "interval",
        crate::contexts::sessions::api::MeasurementKind::CumulativeSnapshot => {
            "cumulative-snapshot"
        }
    }
}

fn overlap_value(value: crate::contexts::sessions::api::TokenOverlap) -> &'static str {
    match value {
        crate::contexts::sessions::api::TokenOverlap::Subset => "subset",
        crate::contexts::sessions::api::TokenOverlap::Exclusive => "exclusive",
        crate::contexts::sessions::api::TokenOverlap::Unknown => "unknown",
    }
}

fn purpose_value(value: crate::contexts::sessions::api::UsagePurpose) -> &'static str {
    match value {
        crate::contexts::sessions::api::UsagePurpose::AssistantInitial => "assistant-initial",
        crate::contexts::sessions::api::UsagePurpose::ToolContinuation => "tool-continuation",
        crate::contexts::sessions::api::UsagePurpose::ContextCompaction => "context-compaction",
        crate::contexts::sessions::api::UsagePurpose::MemoryExtraction => "memory-extraction",
        crate::contexts::sessions::api::UsagePurpose::SubagentDelegation => "subagent-delegation",
        crate::contexts::sessions::api::UsagePurpose::Retry => "retry",
        crate::contexts::sessions::api::UsagePurpose::TerminalInterval => "terminal-interval",
    }
}

fn quality_value(value: crate::contexts::sessions::api::MeasurementQuality) -> &'static str {
    match value {
        crate::contexts::sessions::api::MeasurementQuality::Reported => "reported",
        crate::contexts::sessions::api::MeasurementQuality::ReportedDerived => "reported-derived",
        crate::contexts::sessions::api::MeasurementQuality::Estimated => "estimated",
    }
}

fn status_value(value: crate::contexts::sessions::api::UsageStatus) -> &'static str {
    match value {
        crate::contexts::sessions::api::UsageStatus::Running => "running",
        crate::contexts::sessions::api::UsageStatus::Succeeded => "succeeded",
        crate::contexts::sessions::api::UsageStatus::Failed => "failed",
        crate::contexts::sessions::api::UsageStatus::Cancelled => "cancelled",
    }
}

fn usage_purpose(
    value: Option<&str>,
) -> Result<Option<crate::contexts::sessions::api::UsagePurpose>, SessionsError> {
    value
        .map(|value| match value {
            "assistant-initial" => {
                Ok(crate::contexts::sessions::api::UsagePurpose::AssistantInitial)
            }
            "tool-continuation" => {
                Ok(crate::contexts::sessions::api::UsagePurpose::ToolContinuation)
            }
            "context-compaction" => {
                Ok(crate::contexts::sessions::api::UsagePurpose::ContextCompaction)
            }
            "memory-extraction" => {
                Ok(crate::contexts::sessions::api::UsagePurpose::MemoryExtraction)
            }
            "retry" => Ok(crate::contexts::sessions::api::UsagePurpose::Retry),
            "terminal-interval" => {
                Ok(crate::contexts::sessions::api::UsagePurpose::TerminalInterval)
            }
            _ => Err(invalid_usage_filter("purpose", value)),
        })
        .transpose()
}

fn usage_quality_filter(
    value: Option<&str>,
) -> Result<Option<crate::contexts::sessions::api::MeasurementQuality>, SessionsError> {
    value
        .map(|value| match value {
            "reported" => Ok(crate::contexts::sessions::api::MeasurementQuality::Reported),
            "reported-derived" => {
                Ok(crate::contexts::sessions::api::MeasurementQuality::ReportedDerived)
            }
            "estimated" => Ok(crate::contexts::sessions::api::MeasurementQuality::Estimated),
            _ => Err(invalid_usage_filter("quality", value)),
        })
        .transpose()
}

fn usage_status(
    value: Option<&str>,
) -> Result<Option<crate::contexts::sessions::api::UsageStatus>, SessionsError> {
    value
        .map(|value| match value {
            "running" => Ok(crate::contexts::sessions::api::UsageStatus::Running),
            "succeeded" => Ok(crate::contexts::sessions::api::UsageStatus::Succeeded),
            "failed" => Ok(crate::contexts::sessions::api::UsageStatus::Failed),
            "cancelled" => Ok(crate::contexts::sessions::api::UsageStatus::Cancelled),
            _ => Err(invalid_usage_filter("status", value)),
        })
        .transpose()
}

fn interaction_mode(value: &str) -> Result<dto::InteractionMode, SessionsError> {
    match value {
        "browser" => Ok(dto::InteractionMode::Browser),
        "native-desktop" => Ok(dto::InteractionMode::NativeDesktop),
        "cli" => Ok(dto::InteractionMode::Cli),
        "api" => Ok(dto::InteractionMode::Api),
        other => Err(SessionsError::UnsupportedInteractionMode(other.to_string())),
    }
}

fn lifecycle_state(value: SessionLifecycle) -> dto::SessionLifecycleState {
    match value {
        SessionLifecycle::Idle => dto::SessionLifecycleState::Idle,
        SessionLifecycle::Starting => dto::SessionLifecycleState::Starting,
        SessionLifecycle::Running => dto::SessionLifecycleState::Running,
        SessionLifecycle::Failed => dto::SessionLifecycleState::Failed,
        SessionLifecycle::Stopped => dto::SessionLifecycleState::Stopped,
    }
}

fn recovery_status(value: SessionRecoveryStatus) -> dto::SessionRecoveryStatus {
    match value {
        SessionRecoveryStatus::Clean => dto::SessionRecoveryStatus::Clean,
        SessionRecoveryStatus::Reconciling => dto::SessionRecoveryStatus::Reconciling,
        SessionRecoveryStatus::ActionRequired => dto::SessionRecoveryStatus::ActionRequired,
        SessionRecoveryStatus::Quarantined => dto::SessionRecoveryStatus::Quarantined,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::sessions::application::{
        EstimatedCharacterTotals, MessageTokenUsage, ReportedTokenTotals, SessionRemoteWorkspace,
        SessionUsageAgentBreakdown, SessionUsageCoverage, SessionUsagePoint, SessionUsageSummary,
        SessionWorkspace,
    };
    use crate::contexts::sessions::domain::{
        FileReference, FileReferenceSet, MessageId, MessageRole, MessageStatus, SessionAggregate,
        SessionId, SessionMessage, SessionTitle,
    };

    #[test]
    fn session_mapping_preserves_the_existing_camel_case_contract() {
        let record = SessionRecord {
            aggregate: SessionAggregate::rehydrate(
                SessionId::parse("session-1").expect("session id"),
                SessionTitle::for_creation(Some("Fixture")),
                SessionLifecycle::Running,
                SessionOwner::connector("dingtalk").expect("owner"),
                None,
                true,
                false,
            ),
            agent_id: "codex-cli".to_string(),
            seats: Vec::new(),
            interaction_mode: "native-desktop".to_string(),
            workspace: SessionWorkspace {
                folder: Some("ssh://dev@example.com/work/app".to_string()),
                remote_workspace: Some(SessionRemoteWorkspace {
                    host: "example.com".to_string(),
                    port: None,
                    user: Some("dev".to_string()),
                    path: "/work/app".to_string(),
                    display_name: "App".to_string(),
                    uri: "ssh://dev@example.com/work/app".to_string(),
                }),
                remote_ssh_binding: Some(
                    crate::contexts::sessions::application::SessionSshBinding {
                        connection_id: "ssh-fixture".to_string(),
                        revision: 4,
                    },
                ),
                ..Default::default()
            },
            runtime_session_id: None,
            execution_origin_kind: "user".to_string(),
            execution_origin_id: None,
            created_at: "100".to_string(),
            updated_at: "101".to_string(),
        };

        let value = serde_json::to_value(session_to_dto(record).expect("map session"))
            .expect("serialize session");

        assert_eq!(value["interactionMode"], "native-desktop");
        assert_eq!(value["lifecycleState"], "running");
        assert_eq!(value["source"]["connector"], "dingtalk");
        assert_eq!(value["remoteWorkspace"]["displayName"], "App");
        assert_eq!(value["remoteSshConnectionId"], "ssh-fixture");
        assert_eq!(value["remoteSshConnectionRevision"], 4);
        assert!(value.get("interaction_mode").is_none());
    }

    #[test]
    fn creation_mapping_uses_desktop_ownership_and_active_session_semantics() {
        let input = serde_json::from_value::<dto::CreateSessionInput>(serde_json::json!({
            "agentId": "codex-cli",
            "interactionMode": "cli",
            "projectPath": "D:\\code\\project",
            "worktree": { "enabled": true, "name": "feature-one" }
        }))
        .expect("deserialize input");

        let request = creation_request(input);

        assert_eq!(request.interaction_mode, "cli");
        assert_eq!(request.owner, SessionOwner::desktop());
        assert_eq!(request.activation, SessionActivation::Activate);
        assert_eq!(
            request
                .workspace
                .worktree
                .as_ref()
                .and_then(|worktree| worktree.name.as_deref()),
            Some("feature-one")
        );
    }

    #[test]
    fn message_mapping_preserves_optional_rich_content_and_file_references() {
        let record = MessageRecord {
            message: SessionMessage::rehydrate(
                MessageId::parse("message-1").expect("message id"),
                SessionId::parse("session-1").expect("session id"),
                MessageRole::Assistant,
                MessageStatus::Completed,
                FileReferenceSet::new(vec![FileReference::new(
                    "reference-1",
                    "src/main.rs",
                    "main.rs",
                    Some(12),
                    Some("hash".to_string()),
                    None,
                )
                .expect("file reference")])
                .expect("references"),
            ),
            speaker_seat_id: None,
            seat_index: None,
            seat_round_id: None,
            parent_execution_run_id: None,
            content: "done".to_string(),
            thinking_content: Some("reasoning".to_string()),
            tool_use: Some(vec![serde_json::json!({
                "id": "tool-1",
                "name": "read",
                "input": { "path": "src/main.rs" },
                "output": null,
                "status": "completed"
            })]),
            rich_blocks: Some(vec![serde_json::json!({ "kind": "card" })]),
            token_usage: Some(MessageTokenUsage {
                input: 3,
                output: 5,
            }),
            error: None,
            created_at: "100".to_string(),
            updated_at: "101".to_string(),
        };

        let value = serde_json::to_value(message_to_dto(record)).expect("serialize message");

        assert_eq!(value["sessionId"], "session-1");
        assert_eq!(value["thinkingContent"], "reasoning");
        assert_eq!(value["toolUse"][0]["name"], "read");
        assert_eq!(value["tokenUsage"]["input"], 3);
        assert_eq!(value["fileReferences"][0]["contentHash"], "hash");
        assert!(value.get("session_id").is_none());
    }

    #[test]
    fn usage_mapping_preserves_the_modern_accounting_contract() {
        let reported = ReportedTokenTotals {
            input_tokens: 2,
            output_tokens: 3,
            cache_read_tokens: 5,
            cache_creation_tokens: 7,
            total_tokens: 17,
        };
        let estimated = EstimatedCharacterTotals {
            input_characters: 11,
            output_characters: 13,
            total_characters: 24,
        };
        let statistics = SessionUsageStatistics {
            range: UsageStatisticsRange::Last7Days,
            reported: reported.clone(),
            estimated: estimated.clone(),
            coverage: SessionUsageCoverage {
                reported_responses: 1,
                estimated_responses: 2,
                total_responses: 3,
                reported_percent: 33.3,
            },
            counted_sessions: 2,
            daily: vec![SessionUsagePoint {
                date: "2026-07-18".to_string(),
                reported: reported.clone(),
                estimated: estimated.clone(),
                response_count: 3,
            }],
            by_agent: vec![SessionUsageAgentBreakdown {
                agent_id: "codex-cli".to_string(),
                reported,
                estimated,
                response_count: 3,
            }],
            generated_at: "2026-07-18T10:00:00+08:00".to_string(),
        };

        let value =
            serde_json::to_value(usage_statistics_to_dto(statistics)).expect("serialize usage");

        assert_eq!(value["range"], "last7Days");
        assert_eq!(value["reported"]["totalTokens"], 17);
        assert_eq!(value["estimated"]["totalCharacters"], 24);
        assert_eq!(value["coverage"]["reportedPercent"], 33.3);
        assert_eq!(value["daily"][0]["responseCount"], 3);
        assert_eq!(value["byAgent"][0]["agentId"], "codex-cli");
        assert!(value.get("total_tokens").is_none());
    }

    #[test]
    fn session_usage_summary_mapping_preserves_camel_case_accounting_contract() {
        let summary = SessionUsageSummary {
            session_id: "session-1".to_string(),
            reported: ReportedTokenTotals {
                input_tokens: 2,
                output_tokens: 3,
                cache_read_tokens: 5,
                cache_creation_tokens: 7,
                total_tokens: 17,
            },
            estimated: EstimatedCharacterTotals {
                input_characters: 11,
                output_characters: 13,
                total_characters: 24,
            },
            coverage: SessionUsageCoverage {
                reported_responses: 1,
                estimated_responses: 2,
                total_responses: 3,
                reported_percent: 33.3,
            },
            response_count: 3,
            generated_at: "2026-07-20T10:00:00+08:00".to_string(),
        };

        let value = serde_json::to_value(session_usage_summary_to_dto(summary))
            .expect("serialize session usage summary");

        assert_eq!(value["sessionId"], "session-1");
        assert_eq!(value["reported"]["totalTokens"], 17);
        assert_eq!(value["estimated"]["totalCharacters"], 24);
        assert_eq!(value["coverage"]["reportedPercent"], 33.3);
        assert_eq!(value["responseCount"], 3);
        assert!(value.get("session_id").is_none());
    }

    #[test]
    fn token_usage_queries_validate_filters_and_bound_page_sizes() {
        let input = serde_json::from_value::<dto::TokenUsageDetailsInput>(serde_json::json!({
            "sessionId": "session-1",
            "purpose": "tool-continuation",
            "quality": "reported-derived",
            "status": "failed",
            "limit": 500
        }))
        .expect("deserialize detail query");

        let query = token_usage_details_query(input).expect("map detail query");
        assert_eq!(query.session_id.as_deref(), Some("session-1"));
        assert_eq!(
            query.purpose,
            Some(crate::contexts::sessions::api::UsagePurpose::ToolContinuation)
        );
        assert_eq!(
            query.quality,
            Some(crate::contexts::sessions::api::MeasurementQuality::ReportedDerived)
        );
        assert_eq!(
            query.status,
            Some(crate::contexts::sessions::api::UsageStatus::Failed)
        );
        assert_eq!(query.limit, 100);

        let invalid = serde_json::from_value::<dto::TokenUsageSummaryInput>(serde_json::json!({
            "quality": "precise"
        }))
        .expect("deserialize summary query");
        assert!(matches!(
            token_usage_query(invalid),
            Err(SessionsError::Validation(message)) if message.contains("quality")
        ));
    }

    #[test]
    fn empty_token_usage_details_keep_versioned_camel_case_contract() {
        let value = serde_json::to_value(token_usage_details_to_dto(
            crate::contexts::sessions::api::UsageDetailPage {
                invocations: Vec::new(),
                observations: Vec::new(),
                next_cursor: Some("invocation-9".to_string()),
            },
        ))
        .expect("serialize Token usage details");

        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["nextCursor"], "invocation-9");
        assert!(value["invocations"].as_array().is_some_and(Vec::is_empty));
        assert!(value.get("schema_version").is_none());
    }

    #[test]
    fn export_mapping_preserves_format_and_result_fields() {
        assert_eq!(
            export_format(dto::SessionExportFormat::Markdown),
            SessionExportFormat::Markdown
        );
        let value = serde_json::to_value(export_result_to_dto(SessionExportResult {
            status: "exported",
            path: Some("D:\\exports\\fixture.md".to_string()),
            content: None,
        }))
        .expect("serialize export result");

        assert_eq!(value["status"], "exported");
        assert_eq!(value["path"], "D:\\exports\\fixture.md");
        assert!(value["content"].is_null());
    }

    #[test]
    fn chat_configuration_reports_the_effective_policy() {
        let configuration = SessionChatConfiguration {
            session_id: "session-1".to_string(),
            agent_id: "codex-cli".to_string(),
            interaction_mode: "cli".to_string(),
            values: ChatConfigurationValues {
                execution_mode: "execute".to_string(),
                provider_id: Some("openai".to_string()),
                model_id: Some("gpt-5-5".to_string()),
                reasoning_depth: Some("high".to_string()),
                streaming: true,
                thinking: true,
                long_context: true,
            },
        };

        let dto = chat_configuration_to_dto(
            configuration,
            crate::contexts::permissions::api::PolicyTemplateName::Readonly,
        )
        .expect("configuration DTO");

        assert_eq!(dto.agent_policy.as_deref(), Some("readonly"));
        assert_eq!(dto.effective_execution_policy.as_deref(), Some("readonly"));
    }
}
