use super::dto;
use crate::contexts::communications::domain::ConnectorKind;
use crate::contexts::operations::api::{OperationKind, OperationTask};
use crate::contexts::operations::domain::OperationStatus;
use crate::contexts::sessions::api::{
    CategoryRecord, ChatConfigurationValues, MessageRecord, NewRemoteWorkspace, NewSessionRequest,
    NewSessionWorkspace, NewWorktree, SessionActivation, SessionChatConfiguration,
    SessionCreationOperation, SessionExportFormat, SessionExportResult, SessionLifecycle,
    SessionOwner, SessionRecord, SessionSearchMatchKind, SessionSearchResult,
    SessionUsageStatistics, SessionsError, UsageStatisticsRange,
};

pub(super) fn creation_request(input: dto::CreateSessionInput) -> NewSessionRequest {
    NewSessionRequest {
        agent_id: input.agent_id,
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
        interaction_mode: interaction_mode(&session.interaction_mode)?,
        lifecycle_state: lifecycle_state(session.aggregate.lifecycle()),
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
            permission_mode: config.permission_mode,
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
) -> Result<dto::ChatConfig, SessionsError> {
    Ok(dto::ChatConfig {
        agent_id: configuration.agent_id,
        interaction_mode: interaction_mode(&configuration.interaction_mode)?,
        permission_mode: configuration.values.permission_mode,
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
        id: record.message.id().as_str().to_string(),
        session_id: record.message.session_id().as_str().to_string(),
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
    }
}

pub(super) fn messages_to_dto(records: Vec<MessageRecord>) -> Vec<dto::ChatMessage> {
    records.into_iter().map(message_to_dto).collect()
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

fn interaction_mode(value: &str) -> Result<dto::InteractionMode, SessionsError> {
    match value {
        "browser" => Ok(dto::InteractionMode::Browser),
        "native-desktop" => Ok(dto::InteractionMode::NativeDesktop),
        "cli" => Ok(dto::InteractionMode::Cli),
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
            created_at: "100".to_string(),
            updated_at: "101".to_string(),
        };

        let value = serde_json::to_value(session_to_dto(record).expect("map session"))
            .expect("serialize session");

        assert_eq!(value["interactionMode"], "native-desktop");
        assert_eq!(value["lifecycleState"], "running");
        assert_eq!(value["source"]["connector"], "ding-talk");
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
                )
                .expect("file reference")])
                .expect("references"),
            ),
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
}
