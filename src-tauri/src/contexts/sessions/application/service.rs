use super::models::CreateSessionRequest;
use super::ports::configuration_from_preferences;
use super::{
    AcknowledgeRecoveryRequest, AcknowledgeRecoveryResult, ArchivalPolicy, CategoryRecord,
    CompleteMessageRequest, CreateMessageRequest, DurableGenerationStartRequest,
    DurableGenerationTerminalRequest, FailMessageRequest, FileReferenceInput,
    GenerationStartRequest, GenerationStartResult, GenerationTerminalRequest,
    GenerationTerminalResult, LoopRoleSessionRequest, LoopSessionOwnership, MessagePageQuery,
    MessageRecord, MessageUsageRecord, NewSessionRequest, NewSessionWorkspace,
    PreparedNewSessionCreation, SessionApplicationLog, SessionApplicationLogLevel,
    SessionCategoryRepository, SessionChatConfiguration, SessionChatProfilePort, SessionClockPort,
    SessionConfigurationRepository, SessionCreationContextPort, SessionExportFormat,
    SessionExportRequest, SessionExportResult, SessionFileContentPort, SessionIdentityPort,
    SessionListScope, SessionLoggingPort, SessionMaintenanceResult, SessionMessageRepository,
    SessionOperationPort, SessionRecord, SessionRecoveryEvent, SessionRecoveryEventKind,
    SessionRecoveryEventPort, SessionRecoveryProjection, SessionRecoveryReportRepository,
    SessionRecoverySummary, SessionRepository, SessionRuntimePort, SessionSearchQuery,
    SessionSearchResult, SessionSshBinding, SessionTransactionPort, SessionUsageRepository,
    SessionUsageStatistics, SessionUsageSummary, SessionWorkspace, SessionsApplicationError,
    TokenAccountingPort, UpdateSessionSeatsRequest, UsageStatisticsRange,
};
use crate::contexts::sessions::domain::{
    normalize_chat_preferences, restore_chat_preferences, CategoryId, CategoryName, FileLineRange,
    FileReference, FileReferenceSet, MessageId, MessageRole, MessageStatus, SessionActivation,
    SessionAggregate, SessionCategory, SessionId, SessionLifecycle, SessionMessage, SessionOwner,
    SessionSeat, SessionSeatRoleSnapshot, SessionTitle, UsageStatus,
};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SessionApplicationPorts {
    pub(crate) sessions: Arc<dyn SessionRepository>,
    pub(crate) messages: Arc<dyn SessionMessageRepository>,
    pub(crate) categories: Arc<dyn SessionCategoryRepository>,
    pub(crate) configurations: Arc<dyn SessionConfigurationRepository>,
    pub(crate) usage: Arc<dyn SessionUsageRepository>,
    #[allow(dead_code)]
    pub(crate) accounting: Arc<dyn TokenAccountingPort>,
    pub(crate) transactions: Arc<dyn SessionTransactionPort>,
    pub(crate) recovery_reports: Arc<dyn SessionRecoveryReportRepository>,
    pub(crate) recovery_events: Arc<dyn SessionRecoveryEventPort>,
    pub(crate) clock: Arc<dyn SessionClockPort>,
    pub(crate) identities: Arc<dyn SessionIdentityPort>,
    pub(crate) files: Arc<dyn SessionFileContentPort>,
    pub(crate) operations: Arc<dyn SessionOperationPort>,
    pub(crate) logging: Arc<dyn SessionLoggingPort>,
    pub(crate) chat_profiles: Arc<dyn SessionChatProfilePort>,
    pub(crate) creation: Arc<dyn SessionCreationContextPort>,
    pub(crate) eligibility: Arc<dyn super::SessionAgentEligibilityPort>,
    pub(crate) runtime: Arc<dyn SessionRuntimePort>,
}

#[derive(Clone)]
pub(crate) struct SessionsApplicationService {
    ports: SessionApplicationPorts,
}

#[derive(Clone, Copy)]
struct GenerationMessageCorrelation<'a> {
    execution_run_id: &'a str,
    seat_round_id: Option<&'a str>,
    parent_execution_run_id: Option<&'a str>,
    now: &'a str,
}

impl SessionsApplicationService {
    #[allow(dead_code)]
    pub(crate) fn start_model_invocation(
        &self,
        invocation: &super::NewModelInvocation,
    ) -> Result<super::ModelInvocationRecord, SessionsApplicationError> {
        self.ports.accounting.start_invocation(invocation)
    }

    #[allow(dead_code)]
    pub(crate) fn finalize_model_invocation(
        &self,
        invocation_id: &str,
        status: UsageStatus,
        completed_at: &str,
    ) -> Result<super::ModelInvocationRecord, SessionsApplicationError> {
        self.ports
            .accounting
            .finalize_invocation(invocation_id, status, completed_at)
    }

    #[allow(dead_code)]
    pub(crate) fn record_token_observation(
        &self,
        observation: &super::NewUsageObservation,
    ) -> Result<super::TokenUsageObservation, SessionsApplicationError> {
        self.ports.accounting.record_observation(observation)
    }

    #[allow(dead_code)]
    pub(crate) fn advance_usage_cursor(
        &self,
        advance: &super::UsageCursorAdvance,
    ) -> Result<super::UsageCursor, SessionsApplicationError> {
        self.ports.accounting.advance_cursor(advance)
    }

    #[allow(dead_code)]
    pub(crate) fn find_usage_cursor(
        &self,
        source_id: &str,
    ) -> Result<Option<super::UsageCursor>, SessionsApplicationError> {
        self.ports.accounting.find_cursor(source_id)
    }

    #[allow(dead_code)]
    pub(crate) fn invocation_usage_details(
        &self,
        query: &super::InvocationDetailQuery,
    ) -> Result<super::UsageDetailPage, SessionsApplicationError> {
        if let Some(session_id) = query.session_id.as_deref() {
            self.load_session(session_id)?;
        }
        self.ports.accounting.invocation_details(query)
    }

    #[allow(dead_code)]
    pub(crate) fn token_usage_summary(
        &self,
        query: &super::UsageSummaryQuery,
    ) -> Result<super::UsageAccountingSummary, SessionsApplicationError> {
        if let Some(session_id) = query.session_id.as_deref() {
            self.load_session(session_id)?;
        }
        let mut query = query.clone();
        query.generated_at = self.ports.clock.now();
        self.ports.accounting.usage_summary(&query)
    }

    pub(crate) fn recovery_summary(
        &self,
        session_id: &str,
    ) -> Result<SessionRecoverySummary, SessionsApplicationError> {
        let session_id = SessionId::parse(session_id)?;
        let session = self.ports.sessions.find(&session_id)?.ok_or_else(|| {
            SessionsApplicationError::SessionNotFound(session_id.as_str().to_string())
        })?;
        let latest_report = self
            .ports
            .recovery_reports
            .list_reports(&session_id, 1)?
            .into_iter()
            .next();
        Ok(SessionRecoverySummary {
            session,
            latest_report,
        })
    }

    pub(crate) fn list_recovery_reports(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<
        Vec<crate::contexts::sessions::domain::recovery::SessionRecoveryReport>,
        SessionsApplicationError,
    > {
        let session_id = SessionId::parse(session_id)?;
        if self.ports.sessions.find(&session_id)?.is_none() {
            return Err(SessionsApplicationError::SessionNotFound(
                session_id.as_str().to_string(),
            ));
        }
        self.ports
            .recovery_reports
            .list_reports(&session_id, limit.clamp(1, 100))
    }

    pub(crate) fn recovery_projection(
        &self,
        session_id: &str,
        execution_run_id: Option<&str>,
    ) -> Result<SessionRecoveryProjection, SessionsApplicationError> {
        let session_id = SessionId::parse(session_id)?;
        let session = self.ports.sessions.find(&session_id)?.ok_or_else(|| {
            SessionsApplicationError::SessionNotFound(session_id.as_str().to_string())
        })?;
        let report = self
            .ports
            .recovery_reports
            .list_reports(&session_id, 100)?
            .into_iter()
            .find(|report| {
                execution_run_id
                    .map(|run_id| report.observed_execution_run_id() == Some(run_id))
                    .unwrap_or(true)
            });
        let projected_run_id = report
            .as_ref()
            .and_then(|report| report.observed_execution_run_id().map(str::to_string))
            .or_else(|| execution_run_id.map(str::to_string));
        Ok(SessionRecoveryProjection {
            session_id: session_id.as_str().to_string(),
            execution_run_id: projected_run_id,
            lifecycle: session.aggregate.lifecycle().as_str().to_string(),
            recovery_status: session.aggregate.recovery().status().as_str().to_string(),
            recovery_revision: session.aggregate.recovery().recovery_revision(),
            decision: report.map(|report| report.decision()),
        })
    }

    pub(crate) fn acknowledge_recovery(
        &self,
        session_id: &str,
        expected_recovery_revision: u64,
    ) -> Result<AcknowledgeRecoveryResult, SessionsApplicationError> {
        SessionId::parse(session_id)?;
        let result = self
            .ports
            .transactions
            .acknowledge_recovery(&AcknowledgeRecoveryRequest {
                session_id: session_id.to_string(),
                expected_recovery_revision,
                acknowledged_at: self.ports.clock.now(),
            })?;
        let _ = self
            .ports
            .recovery_events
            .publish_recovery_event(SessionRecoveryEvent {
                kind: SessionRecoveryEventKind::Acknowledged,
                session_id: session_id.to_string(),
                recovery_revision: result.report.recovery_revision(),
            });
        Ok(result)
    }

    pub(crate) fn new(ports: SessionApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn prepare_new_session_creation(
        &self,
        request: NewSessionRequest,
    ) -> Result<PreparedNewSessionCreation, SessionsApplicationError> {
        let related_entity_id = request
            .workspace
            .remote_workspace
            .as_ref()
            .and_then(|workspace| self.ports.creation.remote_workspace_uri(workspace))
            .or_else(|| request.workspace.project_path.clone())
            .or_else(|| request.workspace.folder.clone());
        let operation = self
            .ports
            .operations
            .start_session_creation(related_entity_id)?;
        Ok(PreparedNewSessionCreation { operation, request })
    }

    pub(crate) fn execute_new_session_creation(
        &self,
        prepared: PreparedNewSessionCreation,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let operation_id = prepared.operation.id.clone();
        let result = self.create_new_session_record(prepared.request);
        self.finish_session_creation(&operation_id, result)
    }

    pub(crate) fn create_loop_role_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        for (value, label) in [
            (&request.run_id, "Loop run id"),
            (&request.iteration_id, "Loop iteration id"),
            (&request.project_path, "Loop project path"),
            (&request.worktree_path, "Loop worktree path"),
            (&request.worktree_name, "Loop worktree name"),
            (&request.worktree_branch, "Loop worktree branch"),
        ] {
            required_value(value, label)?;
        }
        self.ports
            .eligibility
            .ensure_agent_supports(&request.agent_id, &request.interaction_mode)?;
        let role = request.role;
        self.create_session_record(CreateSessionRequest {
            agent_id: request.agent_id,
            seats: Vec::new(),
            interaction_mode: request.interaction_mode,
            title: Some(format!("Loop {}", role.as_str())),
            workspace: SessionWorkspace {
                folder: Some(request.worktree_path.clone()),
                project_path: Some(request.project_path),
                worktree_path: Some(request.worktree_path),
                worktree_name: Some(request.worktree_name),
                worktree_branch: Some(request.worktree_branch),
                remote_workspace: None,
                remote_ssh_binding: None,
                loop_ownership: Some(LoopSessionOwnership {
                    run_id: request.run_id,
                    iteration_id: request.iteration_id,
                    role,
                }),
            },
            owner: SessionOwner::desktop(),
            activation: SessionActivation::PreserveActive,
        })
    }

    fn finish_session_creation(
        &self,
        operation_id: &str,
        result: Result<SessionRecord, SessionsApplicationError>,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        match result {
            Ok(session) => {
                let _ = self
                    .ports
                    .operations
                    .append_log(operation_id, format!("Created session {}", session.id()));
                let _ = self
                    .ports
                    .operations
                    .complete_session_creation(operation_id, &session);
                Ok(session)
            }
            Err(error) => {
                let message = error.to_string();
                let _ = self.ports.logging.write(SessionApplicationLog {
                    level: SessionApplicationLogLevel::Error,
                    category: "session.create".to_string(),
                    message: message.clone(),
                    session_id: None,
                    operation_id: Some(operation_id.to_string()),
                    execution_run_id: None,
                    recovery_report_id: None,
                });
                let _ = self
                    .ports
                    .operations
                    .append_log(operation_id, message.clone());
                let _ = self
                    .ports
                    .operations
                    .fail_session_creation(operation_id, message);
                Err(error)
            }
        }
    }

    fn create_new_session_record(
        &self,
        request: NewSessionRequest,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        request.owner.validate_activation(request.activation)?;
        if request.agent_id == "onepiece" && request.workspace.remote_workspace.is_some() {
            return Err(SessionsApplicationError::Validation(
                "OnePiece supports local projects and local Git worktrees only.".to_string(),
            ));
        }
        self.ports
            .eligibility
            .ensure_agent_supports(&request.agent_id, &request.interaction_mode)?;
        let workspace = self.prepare_new_session_workspace(&request.workspace)?;
        self.create_session_record(CreateSessionRequest {
            agent_id: request.agent_id,
            seats: request.seats,
            interaction_mode: request.interaction_mode,
            title: request.title,
            workspace,
            owner: request.owner,
            activation: request.activation,
        })
    }

    fn prepare_new_session_workspace(
        &self,
        request: &NewSessionWorkspace,
    ) -> Result<SessionWorkspace, SessionsApplicationError> {
        let remote_workspace = request
            .remote_workspace
            .as_ref()
            .map(|workspace| self.ports.creation.normalize_remote_workspace(workspace))
            .transpose()?;
        let worktree_enabled = request
            .worktree
            .as_ref()
            .is_some_and(|worktree| worktree.enabled);
        self.ports
            .creation
            .ensure_worktree_compatible(remote_workspace.is_some(), worktree_enabled)?;

        let selected_project = if remote_workspace.is_some() {
            None
        } else {
            request
                .project_path
                .as_deref()
                .or(request.folder.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
        };
        let project = selected_project
            .map(|path| self.ports.creation.prepare_project(path))
            .transpose()?;
        if let Some(workspace) = &remote_workspace {
            self.ports.creation.remember_remote_workspace(workspace)?;
        }
        let remote_ssh_binding = match (
            remote_workspace.as_ref(),
            request
                .remote_workspace
                .as_ref()
                .and_then(|workspace| workspace.ssh_connection_id.as_deref()),
        ) {
            (Some(workspace), Some(connection_id)) => {
                Some(self.resolve_ssh_binding(workspace, connection_id)?)
            }
            _ => None,
        };

        let mut workspace = SessionWorkspace {
            folder: project
                .as_ref()
                .map(|project| project.path.clone())
                .or_else(|| {
                    remote_workspace
                        .as_ref()
                        .map(|workspace| workspace.uri.clone())
                })
                .or_else(|| request.folder.clone()),
            project_path: project.as_ref().map(|project| project.path.clone()),
            remote_workspace,
            remote_ssh_binding,
            ..Default::default()
        };
        if worktree_enabled {
            let project = project.as_ref().ok_or_else(|| {
                SessionsApplicationError::Validation("Project unavailable".to_string())
            })?;
            self.ports.creation.ensure_git_worktree_available(project)?;
            let name = request
                .worktree
                .as_ref()
                .and_then(|worktree| worktree.name.as_deref())
                .unwrap_or("");
            let worktree = self.ports.creation.create_worktree(&project.path, name)?;
            workspace.folder = Some(worktree.path.clone());
            workspace.worktree_path = Some(worktree.path);
            workspace.worktree_name = Some(worktree.name);
            workspace.worktree_branch = Some(worktree.branch);
        }
        Ok(workspace)
    }

    fn resolve_ssh_binding(
        &self,
        workspace: &super::SessionRemoteWorkspace,
        connection_id: &str,
    ) -> Result<SessionSshBinding, SessionsApplicationError> {
        let connection_id = connection_id.trim();
        if connection_id.is_empty() {
            return Err(SessionsApplicationError::Validation(
                "SSH connection id cannot be empty.".to_string(),
            ));
        }
        let profile = self
            .ports
            .creation
            .find_ssh_profile(connection_id)?
            .ok_or_else(|| {
                SessionsApplicationError::Validation(format!(
                    "SSH connection not found: {connection_id}"
                ))
            })?;
        let endpoint_matches = profile.host == workspace.host
            && profile.port == workspace.port.unwrap_or(22)
            && workspace.user.as_deref() == Some(profile.user.as_str());
        if !endpoint_matches {
            return Err(SessionsApplicationError::Validation(
                "SSH connection endpoint does not match the remote workspace snapshot.".to_string(),
            ));
        }
        Ok(SessionSshBinding {
            connection_id: profile.connection_id,
            revision: profile.revision,
        })
    }

    fn create_session_record(
        &self,
        request: CreateSessionRequest,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        required_value(&request.agent_id, "Agent id")?;
        required_value(&request.interaction_mode, "Interaction mode")?;
        request.owner.validate_activation(request.activation)?;
        let id = SessionId::parse(self.ports.identities.next_session_id())?;
        let aggregate = SessionAggregate::create(
            id,
            SessionTitle::for_creation(request.title.as_deref()),
            request.owner,
        );
        let now = self.ports.clock.now();
        let mut seats = if request.seats.is_empty() {
            vec![SessionSeat {
                seat_id: String::new(),
                agent_id: request.agent_id.clone(),
                role_id: None,
                role_snapshot: None,
                joined_at: String::new(),
                left_at: None,
            }]
        } else {
            request.seats
        };
        for seat in &mut seats {
            if seat.seat_id.trim().is_empty() {
                seat.seat_id = self.ports.identities.next_seat_id();
            }
            if seat.joined_at.trim().is_empty() {
                seat.joined_at.clone_from(&now);
            }
            if seat.role_snapshot.is_none() {
                seat.role_snapshot = Some(fallback_role_snapshot(&seat.agent_id));
            }
            seat.left_at = None;
        }
        let primary_agent_id = seats[0].agent_id.clone();
        let record = SessionRecord {
            aggregate,
            agent_id: primary_agent_id,
            seats,
            interaction_mode: request.interaction_mode,
            workspace: request.workspace,
            runtime_session_id: None,
            execution_origin_kind: "user".to_string(),
            execution_origin_id: None,
            created_at: now.clone(),
            updated_at: now,
        };
        self.ports
            .transactions
            .create_session(&record, request.activation)
    }

    pub(crate) fn list_sessions(
        &self,
        scope: SessionListScope,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        self.ports.sessions.list(scope)
    }

    #[cfg(test)]
    pub(crate) fn list_sessions_including_loop_owned(
        &self,
        scope: SessionListScope,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        self.ports.sessions.list_including_loop_owned(scope)
    }

    pub(crate) fn search_sessions(
        &self,
        query: &str,
        limit: Option<i64>,
    ) -> Result<Vec<SessionSearchResult>, SessionsApplicationError> {
        let text = query.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let query = SessionSearchQuery {
            text: text.to_string(),
            limit: limit.unwrap_or(50).clamp(1, 100) as usize,
        };
        self.ports.sessions.search(&query)
    }

    pub(crate) fn active_session(&self) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        let Some(session) = self.ports.sessions.active_session()? else {
            return Ok(None);
        };
        if session.aggregate.is_archived() {
            self.ports
                .transactions
                .clear_active_session_if_matches(session.aggregate.id())?;
            return Ok(None);
        }
        Ok(Some(session))
    }

    pub(crate) fn switch_session(
        &self,
        session_id: &str,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let session = self.load_session(session_id)?;
        session.aggregate.activation(SessionActivation::Activate)?;
        self.ports.transactions.activate_session(&session)
    }

    pub(crate) fn rename_session(
        &self,
        session_id: &str,
        title: String,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        session.aggregate.rename(SessionTitle::for_rename(title)?);
        session.updated_at = self.ports.clock.now();
        self.ports.sessions.save(&session)
    }

    pub(crate) fn update_session_seats(
        &self,
        request: UpdateSessionSeatsRequest,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        if request.seats.is_empty() {
            return Err(SessionsApplicationError::Validation(
                "A session must keep at least one active participant.".to_string(),
            ));
        }
        let mut session = self.load_session(&request.session_id)?;
        if session.updated_at != request.expected_updated_at {
            return Err(SessionsApplicationError::SessionRevisionConflict(
                request.session_id,
            ));
        }

        let changed_at = self.ports.clock.now();
        let mut retained = std::collections::HashSet::new();
        let mut additions = Vec::new();
        for requested in request.seats {
            if requested.agent_id.trim().is_empty() {
                return Err(SessionsApplicationError::Validation(
                    "Participant Agent id is required.".to_string(),
                ));
            }
            self.ports
                .eligibility
                .ensure_agent_supports(&requested.agent_id, &session.interaction_mode)?;
            let matched = session.seats.iter().find(|existing| {
                existing.is_active()
                    && !retained.contains(&existing.seat_id)
                    && ((!requested.seat_id.is_empty()
                        && existing.seat_id == requested.seat_id
                        && existing.agent_id == requested.agent_id
                        && existing.role_id == requested.role_id)
                        || (requested.seat_id.is_empty()
                            && existing.agent_id == requested.agent_id
                            && existing.role_id == requested.role_id))
            });
            if let Some(existing) = matched {
                retained.insert(existing.seat_id.clone());
                continue;
            }
            additions.push(SessionSeat {
                seat_id: self.ports.identities.next_seat_id(),
                agent_id: requested.agent_id.clone(),
                role_id: requested.role_id,
                role_snapshot: Some(
                    requested
                        .role_snapshot
                        .unwrap_or_else(|| fallback_role_snapshot(&requested.agent_id)),
                ),
                joined_at: changed_at.clone(),
                left_at: None,
            });
        }

        for seat in &mut session.seats {
            if seat.is_active() && !retained.contains(&seat.seat_id) {
                seat.left_at = Some(changed_at.clone());
            }
        }
        session.seats.extend(additions);
        let first_active = session
            .seats
            .iter()
            .find(|seat| seat.is_active())
            .ok_or_else(|| {
                SessionsApplicationError::Validation(
                    "A session must keep at least one active participant.".to_string(),
                )
            })?;
        session.agent_id = first_active.agent_id.clone();
        session.updated_at = changed_at;
        self.ports
            .sessions
            .save_if_revision(&session, &request.expected_updated_at)?
            .ok_or(SessionsApplicationError::SessionRevisionConflict(
                request.session_id,
            ))
    }

    pub(crate) fn set_session_pinned(
        &self,
        session_id: &str,
        pinned: bool,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        session.aggregate.set_pinned(pinned);
        session.updated_at = self.ports.clock.now();
        self.ports.sessions.save(&session)
    }

    pub(crate) fn set_session_archived(
        &self,
        session_id: &str,
        archived: bool,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        if archived {
            self.ports.runtime.stop_session_activity(session_id)?;
            session.aggregate.archive();
        } else {
            session.aggregate.unarchive();
        }
        session.updated_at = self.ports.clock.now();
        if archived {
            self.ports.transactions.archive_session(&session)
        } else {
            self.ports.sessions.save(&session)
        }
    }

    pub(crate) fn delete_session(&self, session_id: &str) -> Result<(), SessionsApplicationError> {
        let session = self.load_session(session_id)?;
        self.ports.runtime.stop_session_activity(session_id)?;
        self.ports
            .transactions
            .delete_session(session.aggregate.id())
    }

    pub(crate) fn list_categories(&self) -> Result<Vec<CategoryRecord>, SessionsApplicationError> {
        self.ports.categories.list()
    }

    pub(crate) fn create_category(
        &self,
        name: String,
    ) -> Result<CategoryRecord, SessionsApplicationError> {
        let name = CategoryName::parse(name)?;
        if self.ports.categories.name_exists(name.as_str(), None)? {
            return Err(SessionsApplicationError::CategoryNameConflict(
                name.as_str().to_string(),
            ));
        }
        let id = CategoryId::parse(self.ports.identities.next_category_id())?;
        let category = SessionCategory::new(id, name, self.ports.categories.next_sort_order()?);
        let now = self.ports.clock.now();
        self.ports.categories.insert(&CategoryRecord {
            category,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub(crate) fn rename_category(
        &self,
        category_id: &str,
        name: String,
    ) -> Result<CategoryRecord, SessionsApplicationError> {
        let category_id = CategoryId::parse(category_id)?;
        let mut record = self.load_category(&category_id)?;
        let name = CategoryName::parse(name)?;
        if self
            .ports
            .categories
            .name_exists(name.as_str(), Some(&category_id))?
        {
            return Err(SessionsApplicationError::CategoryNameConflict(
                name.as_str().to_string(),
            ));
        }
        record.category.rename(name);
        record.updated_at = self.ports.clock.now();
        self.ports.categories.save(&record)
    }

    pub(crate) fn delete_category(
        &self,
        category_id: &str,
    ) -> Result<(), SessionsApplicationError> {
        let category_id = CategoryId::parse(category_id)?;
        self.load_category(&category_id)?;
        self.ports
            .transactions
            .delete_category(&category_id, &self.ports.clock.now())
    }

    pub(crate) fn assign_category(
        &self,
        session_id: &str,
        category_id: Option<&str>,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        let category_id = category_id.map(CategoryId::parse).transpose()?;
        if let Some(category_id) = &category_id {
            self.load_category(category_id)?;
        }
        session.aggregate.assign_category(category_id);
        session.updated_at = self.ports.clock.now();
        self.ports.sessions.save(&session)
    }

    pub(crate) fn load_chat_configuration(
        &self,
        session_id: &str,
    ) -> Result<SessionChatConfiguration, SessionsApplicationError> {
        let session = self.load_session(session_id)?;
        let persisted = self.ports.configurations.load(session.aggregate.id())?;
        let preferences = persisted
            .as_ref()
            .and_then(|values| {
                restore_chat_preferences(&session.agent_id, values.as_domain_request())
            })
            .map(Ok)
            .unwrap_or_else(|| {
                let workspace_path = session
                    .workspace
                    .worktree_path
                    .as_deref()
                    .or(session.workspace.project_path.as_deref());
                let defaults = self
                    .ports
                    .chat_profiles
                    .defaults_for(&session.agent_id, workspace_path)?;
                normalize_chat_preferences(&session.agent_id, defaults.as_domain_request())
                    .map_err(SessionsApplicationError::from)
            })?;
        Ok(configuration_from_preferences(&session, &preferences))
    }

    pub(crate) fn save_chat_configuration(
        &self,
        configuration: SessionChatConfiguration,
    ) -> Result<SessionChatConfiguration, SessionsApplicationError> {
        let session = self.load_session(&configuration.session_id)?;
        let preferences = normalize_chat_preferences(
            &session.agent_id,
            configuration.values.as_domain_request(),
        )?;
        self.ports.configurations.save(
            session.aggregate.id(),
            &preferences,
            &self.ports.clock.now(),
        )?;
        Ok(configuration_from_preferences(&session, &preferences))
    }

    pub(crate) fn validate_chat_configuration(
        &self,
        configuration: SessionChatConfiguration,
    ) -> Result<SessionChatConfiguration, SessionsApplicationError> {
        let session = self.load_session(&configuration.session_id)?;
        let preferences = normalize_chat_preferences(
            &session.agent_id,
            configuration.values.as_domain_request(),
        )?;
        Ok(configuration_from_preferences(&session, &preferences))
    }

    /// Normalizes a chat configuration against the Agent it names rather than the session's.
    ///
    /// A seat runs its own Agent, so normalizing against the session's — which mirrors only the
    /// first seat — would hand every other seat the wrong model defaults, silently.
    pub(crate) fn validate_seat_chat_configuration(
        &self,
        configuration: SessionChatConfiguration,
    ) -> Result<SessionChatConfiguration, SessionsApplicationError> {
        let session = self.load_session(&configuration.session_id)?;
        if !session
            .seats
            .iter()
            .any(|seat| seat.agent_id == configuration.agent_id)
        {
            return Err(SessionsApplicationError::Validation(format!(
                "Agent '{}' holds no seat in this session.",
                configuration.agent_id
            )));
        }
        let preferences = normalize_chat_preferences(
            &configuration.agent_id,
            configuration.values.as_domain_request(),
        )?;
        Ok(SessionChatConfiguration {
            session_id: configuration.session_id,
            agent_id: configuration.agent_id,
            interaction_mode: configuration.interaction_mode,
            values: super::ChatConfigurationValues::from_preferences(&preferences),
        })
    }

    pub(crate) fn find_session(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        let session_id = SessionId::parse(session_id)?;
        self.ports.sessions.find(&session_id)
    }

    pub(crate) fn rebind_remote_session(
        &self,
        session_id: &str,
        connection_id: &str,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        let workspace = session.workspace.remote_workspace.as_ref().ok_or_else(|| {
            SessionsApplicationError::Validation(
                "Only remote workspace sessions can bind an SSH connection.".to_string(),
            )
        })?;
        session.workspace.remote_ssh_binding =
            Some(self.resolve_ssh_binding(workspace, connection_id)?);
        session.updated_at = self.ports.clock.now();
        self.ports.sessions.save(&session)
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "remote Shell routing consumes this binding guard in task 4.1"
        )
    )]
    pub(crate) fn require_current_remote_ssh_binding(
        &self,
        session_id: &str,
    ) -> Result<SessionSshBinding, SessionsApplicationError> {
        let session = self.load_session(session_id)?;
        let workspace = session.workspace.remote_workspace.as_ref().ok_or_else(|| {
            SessionsApplicationError::Validation(
                "Session does not use a remote workspace.".to_string(),
            )
        })?;
        let binding = session
            .workspace
            .remote_ssh_binding
            .as_ref()
            .ok_or_else(|| {
                SessionsApplicationError::Validation(
                    "Remote session requires an SSH connection binding.".to_string(),
                )
            })?;
        let current = self.resolve_ssh_binding(workspace, &binding.connection_id)?;
        if current.revision != binding.revision {
            return Err(SessionsApplicationError::Validation(
                "Remote session SSH connection binding is stale; explicit rebind is required."
                    .to_string(),
            ));
        }
        Ok(current)
    }

    pub(crate) fn find_message(
        &self,
        message_id: &str,
    ) -> Result<Option<MessageRecord>, SessionsApplicationError> {
        let message_id = MessageId::parse(message_id)?;
        self.ports.messages.find(&message_id)
    }

    #[allow(dead_code)]
    pub(crate) fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        let session = self.load_session(&request.session_id)?;
        session.aggregate.ensure_accepts_messages()?;
        let role = MessageRole::parse(&request.role)?;
        let content = if role == MessageRole::User {
            let content = request.content.trim().to_string();
            if content.is_empty() {
                return Err(SessionsApplicationError::Validation(
                    "Message content cannot be empty.".to_string(),
                ));
            }
            content
        } else {
            request.content
        };
        let references = file_reference_set(request.file_references)?;
        let message = SessionMessage::rehydrate(
            MessageId::parse(self.ports.identities.next_message_id())?,
            session.aggregate.id().clone(),
            role,
            MessageStatus::parse(&request.status)?,
            references,
        );
        let now = self.ports.clock.now();
        self.ports.messages.insert(&MessageRecord {
            message,
            speaker_seat_id: request.speaker_seat_id,
            seat_index: request.seat_index,
            seat_round_id: None,
            parent_execution_run_id: None,
            content,
            thinking_content: None,
            tool_use: None,
            rich_blocks: None,
            token_usage: None,
            error: None,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub(crate) fn start_generation(
        &self,
        request: DurableGenerationStartRequest,
    ) -> Result<GenerationStartResult, SessionsApplicationError> {
        let session = self.load_session(&request.session_id)?;
        let now = self.ports.clock.now();
        let correlation = GenerationMessageCorrelation {
            execution_run_id: &request.execution_run_id,
            seat_round_id: request.seat_round_id.as_deref(),
            parent_execution_run_id: request.parent_execution_run_id.as_deref(),
            now: &now,
        };
        let user_message = request
            .user_message
            .map(|message| {
                self.generation_message_record(&session, message, correlation, MessageRole::User)
            })
            .transpose()?;
        let assistant_message = self.generation_message_record(
            &session,
            request.assistant_message,
            correlation,
            MessageRole::Assistant,
        )?;
        self.ports
            .transactions
            .start_generation(&GenerationStartRequest {
                session_id: request.session_id,
                execution_run_id: request.execution_run_id,
                user_message,
                assistant_message,
                started_at: now,
            })
    }

    pub(crate) fn terminalize_generation(
        &self,
        request: DurableGenerationTerminalRequest,
    ) -> Result<GenerationTerminalResult, SessionsApplicationError> {
        let session_id = SessionId::parse(&request.session_id)?;
        let message_id = MessageId::parse(&request.message_id)?;
        let mut record = self.load_message(&message_id)?;
        record.message.ensure_owned_by(&session_id)?;
        if record.message.execution_run_id() != Some(request.execution_run_id.as_str()) {
            return Err(SessionsApplicationError::Transaction(
                "generation terminal execution correlation does not match the message".to_string(),
            ));
        }
        record
            .message
            .transition_to(request.terminal_status.message_status())?;
        validate_usage(request.usage.as_ref(), &message_id, &session_id)?;
        validate_invocation_usage(request.invocation_usage.as_ref(), &message_id, &session_id)?;
        record.content = request.content;
        record.thinking_content = request.thinking_content;
        record.tool_use = request.tool_use;
        record.rich_blocks = request.rich_blocks;
        record.token_usage = request.token_usage;
        record.error = request.error;
        let finished_at = self.ports.clock.now();
        record.updated_at.clone_from(&finished_at);
        self.ports
            .transactions
            .terminalize_generation(&GenerationTerminalRequest {
                execution_run_id: request.execution_run_id,
                message: record,
                terminal_status: request.terminal_status,
                usage: request.usage,
                invocation_usage: request.invocation_usage,
                finished_at,
            })
    }

    fn generation_message_record(
        &self,
        session: &SessionRecord,
        request: CreateMessageRequest,
        correlation: GenerationMessageCorrelation<'_>,
        expected_role: MessageRole,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        if request.session_id != session.id() {
            return Err(SessionsApplicationError::Validation(
                "Generation message session does not match the durable claim.".to_string(),
            ));
        }
        let role = MessageRole::parse(&request.role)?;
        if role != expected_role {
            return Err(SessionsApplicationError::Validation(
                "Generation message role does not match the durable start slot.".to_string(),
            ));
        }
        let content = if role == MessageRole::User {
            let content = request.content.trim().to_string();
            if content.is_empty() {
                return Err(SessionsApplicationError::Validation(
                    "Message content cannot be empty.".to_string(),
                ));
            }
            content
        } else {
            request.content
        };
        let message = SessionMessage::rehydrate_with_correlation(
            MessageId::parse(self.ports.identities.next_message_id())?,
            session.aggregate.id().clone(),
            role,
            MessageStatus::parse(&request.status)?,
            file_reference_set(request.file_references)?,
            0,
            Some(correlation.execution_run_id.to_string()),
        );
        Ok(MessageRecord {
            message,
            speaker_seat_id: request.speaker_seat_id,
            seat_index: request.seat_index,
            seat_round_id: correlation.seat_round_id.map(str::to_string),
            parent_execution_run_id: correlation.parent_execution_run_id.map(str::to_string),
            content,
            thinking_content: None,
            tool_use: None,
            rich_blocks: None,
            token_usage: None,
            error: None,
            created_at: correlation.now.to_string(),
            updated_at: correlation.now.to_string(),
        })
    }

    pub(crate) fn compose_prompt(
        &self,
        session_id: &str,
        content: &str,
        references: Vec<FileReferenceInput>,
    ) -> Result<String, SessionsApplicationError> {
        self.load_session(session_id)?;
        let references = file_reference_set(references)?;
        if references.as_slice().is_empty() {
            return Ok(content.to_string());
        }
        let mut prompt = content.to_string();
        prompt.push_str("\n\nReferenced files:\n");
        for reference in references.as_slice() {
            let file_content = self
                .ports
                .files
                .read_reference_text(session_id, reference.path())?;
            prompt.push_str(&render_reference_block(
                reference.path(),
                &file_content,
                reference.line_range(),
            ));
        }
        Ok(prompt)
    }

    pub(crate) fn list_messages(
        &self,
        session_id: &str,
        limit: Option<i64>,
        before_id: Option<String>,
    ) -> Result<Vec<MessageRecord>, SessionsApplicationError> {
        self.load_session(session_id)?;
        self.ports.messages.list(&MessagePageQuery {
            session_id: session_id.to_string(),
            limit: limit.unwrap_or(50).clamp(1, 200) as usize,
            before_id,
        })
    }

    pub(crate) fn complete_message(
        &self,
        request: CompleteMessageRequest,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        let session_id = SessionId::parse(&request.session_id)?;
        let message_id = MessageId::parse(&request.message_id)?;
        let mut record = self.load_message(&message_id)?;
        record.message.ensure_owned_by(&session_id)?;
        record.message.transition_to(MessageStatus::Completed)?;
        validate_usage(request.usage.as_ref(), &message_id, &session_id)?;
        validate_invocation_usage(request.invocation_usage.as_ref(), &message_id, &session_id)?;
        record.content = request.content;
        record.thinking_content = request.thinking_content;
        record.tool_use = request.tool_use;
        record.rich_blocks = request.rich_blocks;
        record.token_usage = request.token_usage;
        record.error = None;
        record.updated_at = self.ports.clock.now();
        self.ports.transactions.complete_message(
            &record,
            request.usage.as_ref(),
            request.invocation_usage.as_ref(),
        )
    }

    pub(crate) fn fail_message(
        &self,
        request: FailMessageRequest,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        let session_id = SessionId::parse(&request.session_id)?;
        let message_id = MessageId::parse(&request.message_id)?;
        let mut record = self.load_message(&message_id)?;
        record.message.ensure_owned_by(&session_id)?;
        record.message.transition_to(MessageStatus::Failed)?;
        record.error = Some(request.error);
        record.updated_at = self.ports.clock.now();
        self.ports.messages.save(&record)
    }

    pub(crate) fn append_message_content(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), SessionsApplicationError> {
        let message_id = MessageId::parse(message_id)?;
        let mut record = self.load_message(&message_id)?;
        record.content.push_str(content_delta);
        record.updated_at = self.ports.clock.now();
        self.ports.messages.save_stream_fields(&record)
    }

    pub(crate) fn append_message_thinking(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), SessionsApplicationError> {
        let message_id = MessageId::parse(message_id)?;
        let mut record = self.load_message(&message_id)?;
        record
            .thinking_content
            .get_or_insert_with(String::new)
            .push_str(content_delta);
        record.updated_at = self.ports.clock.now();
        self.ports.messages.save_stream_fields(&record)
    }

    pub(crate) fn append_message_tool_use(
        &self,
        message_id: &str,
        tool_use: Value,
    ) -> Result<(), SessionsApplicationError> {
        let message_id = MessageId::parse(message_id)?;
        let mut record = self.load_message(&message_id)?;
        record.tool_use.get_or_insert_with(Vec::new).push(tool_use);
        record.updated_at = self.ports.clock.now();
        self.ports.messages.save_stream_fields(&record)
    }

    pub(crate) fn append_message_rich_block(
        &self,
        message_id: &str,
        block: Value,
    ) -> Result<(), SessionsApplicationError> {
        let block_id = valid_rich_block_id(&block)?;
        let message_id = MessageId::parse(message_id)?;
        let mut record = self.load_message(&message_id)?;
        let blocks = record.rich_blocks.get_or_insert_with(Vec::new);
        if let Some(index) = blocks
            .iter()
            .position(|candidate| candidate.get("id").and_then(Value::as_str) == Some(block_id))
        {
            blocks[index] = block;
        } else {
            blocks.push(block);
        }
        record.updated_at = self.ports.clock.now();
        self.ports.messages.save_stream_fields(&record)
    }

    pub(crate) fn cancel_streaming_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, SessionsApplicationError> {
        let session_id = SessionId::parse(session_id)?;
        self.ports
            .sessions
            .find(&session_id)?
            .ok_or_else(|| SessionsApplicationError::SessionNotFound(session_id.as_str().into()))?;
        let now = self.ports.clock.now();
        let mut cancelled = self
            .ports
            .messages
            .list_all(&session_id)?
            .into_iter()
            .filter(|record| record.message.status() == MessageStatus::Streaming)
            .collect::<Vec<_>>();
        for record in &mut cancelled {
            record.message.transition_to(MessageStatus::Cancelled)?;
            record.updated_at.clone_from(&now);
        }
        self.ports.transactions.cancel_messages(&cancelled)
    }

    pub(crate) fn update_runtime_lifecycle(
        &self,
        session_id: &str,
        lifecycle: SessionLifecycle,
    ) -> Result<(), SessionsApplicationError> {
        let mut session = self.load_session(session_id)?;
        session.aggregate.transition_to(lifecycle)?;
        session.updated_at = self.ports.clock.now();
        self.ports
            .transactions
            .save_runtime_session(&session)
            .map(|_| ())
    }

    pub(crate) fn update_runtime_session_id(
        &self,
        session_id: &str,
        runtime_session_id: &str,
    ) -> Result<(), SessionsApplicationError> {
        required_value(runtime_session_id, "Runtime session id")?;
        let mut session = self.load_session(session_id)?;
        session.runtime_session_id = Some(runtime_session_id.to_string());
        session.updated_at = self.ports.clock.now();
        self.ports
            .transactions
            .save_runtime_session(&session)
            .map(|_| ())
    }

    pub(crate) fn export_session(
        &self,
        request: SessionExportRequest,
    ) -> Result<SessionExportResult, SessionsApplicationError> {
        let Some(destination_directory) = request
            .destination_directory
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(SessionExportResult {
                status: "cancelled",
                path: None,
                content: None,
            });
        };
        let session = self.load_session(&request.session_id)?;
        let messages = self.ports.messages.list_all(session.aggregate.id())?;
        let payload = ExportPayload::from_records(&session, &messages, self.ports.clock.now());
        let content = payload.render(request.format)?;
        let filename = safe_export_filename(&session, request.format);
        let path = self
            .ports
            .files
            .write_export(destination_directory, &filename, &content)
            .inspect_err(|error| {
                let _ = self.ports.logging.write(SessionApplicationLog {
                    level: SessionApplicationLogLevel::Error,
                    category: "session.export".to_string(),
                    message: error.to_string(),
                    session_id: Some(session.id().to_string()),
                    operation_id: None,
                    execution_run_id: None,
                    recovery_report_id: None,
                });
            })?;
        Ok(SessionExportResult {
            status: "exported",
            path: Some(path),
            content: None,
        })
    }

    pub(crate) fn usage_statistics(
        &self,
        range: UsageStatisticsRange,
    ) -> Result<SessionUsageStatistics, SessionsApplicationError> {
        let range_start = self.ports.clock.usage_range_start(range)?;
        self.ports
            .usage
            .statistics(range, range_start.as_deref(), &self.ports.clock.now())
    }

    pub(crate) fn session_usage_summary(
        &self,
        session_id: &str,
    ) -> Result<SessionUsageSummary, SessionsApplicationError> {
        let session = self.load_session(session_id)?;
        self.ports
            .usage
            .summary_for_session(session.id(), &self.ports.clock.now())
    }

    pub(crate) fn run_maintenance(
        &self,
        policy: ArchivalPolicy,
    ) -> Result<SessionMaintenanceResult, SessionsApplicationError> {
        let mut result = SessionMaintenanceResult::default();
        if policy.enabled {
            if policy.inactive_days <= 0 {
                return Err(SessionsApplicationError::Validation(
                    "Automatic archival inactivity days must be positive.".to_string(),
                ));
            }
            let cutoff = self.ports.clock.inactivity_cutoff(policy.inactive_days)?;
            let archived_at = self.ports.clock.now();
            for mut session in self.ports.sessions.inactive_sessions(&cutoff)? {
                if !session.aggregate.can_archive_automatically() {
                    continue;
                }
                session.aggregate.archive();
                session.updated_at.clone_from(&archived_at);
                self.ports.transactions.archive_session(&session)?;
                let _ = self.ports.logging.write(SessionApplicationLog {
                    level: SessionApplicationLogLevel::Info,
                    category: "session.runtime".to_string(),
                    message: "Automatically archived inactive session.".to_string(),
                    session_id: Some(session.id().to_string()),
                    operation_id: None,
                    execution_run_id: None,
                    recovery_report_id: None,
                });
                result.archived += 1;
            }
        }
        if result.recovered > 0 || result.archived > 0 {
            let _ = self.ports.logging.write(SessionApplicationLog {
                level: SessionApplicationLogLevel::Info,
                category: "session.maintenance".to_string(),
                message: format!(
                    "Session maintenance completed. recovered={} archived={}",
                    result.recovered, result.archived
                ),
                session_id: None,
                operation_id: None,
                execution_run_id: None,
                recovery_report_id: None,
            });
        }
        Ok(result)
    }

    fn load_session(&self, session_id: &str) -> Result<SessionRecord, SessionsApplicationError> {
        let session_id = SessionId::parse(session_id)?;
        self.ports
            .sessions
            .find(&session_id)?
            .ok_or_else(|| SessionsApplicationError::SessionNotFound(session_id.as_str().into()))
    }

    fn load_message(
        &self,
        message_id: &MessageId,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        self.ports
            .messages
            .find(message_id)?
            .ok_or_else(|| SessionsApplicationError::MessageNotFound(message_id.as_str().into()))
    }

    fn load_category(
        &self,
        category_id: &CategoryId,
    ) -> Result<CategoryRecord, SessionsApplicationError> {
        self.ports
            .categories
            .find(category_id)?
            .ok_or_else(|| SessionsApplicationError::CategoryNotFound(category_id.as_str().into()))
    }
}

fn fallback_role_snapshot(agent_id: &str) -> SessionSeatRoleSnapshot {
    SessionSeatRoleSnapshot {
        role_name: None,
        avatar: "🤖".to_string(),
        color: "#7A8899".to_string(),
        responsibility: None,
        agent_name: agent_id.to_string(),
        model_family: "unknown".to_string(),
        cross_family_reviewer: false,
    }
}

fn required_value(value: &str, name: &str) -> Result<(), SessionsApplicationError> {
    if value.trim().is_empty() {
        Err(SessionsApplicationError::Validation(format!(
            "{name} cannot be empty."
        )))
    } else {
        Ok(())
    }
}

fn valid_rich_block_id(block: &Value) -> Result<&str, SessionsApplicationError> {
    let Some(id) = block
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Err(SessionsApplicationError::Validation(
            "Invalid Rich Block payload.".to_string(),
        ));
    };
    let Some(_kind) = block
        .get("kind")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Err(SessionsApplicationError::Validation(
            "Invalid Rich Block payload.".to_string(),
        ));
    };
    if block.get("v").and_then(Value::as_i64) != Some(1) {
        return Err(SessionsApplicationError::Validation(
            "Invalid Rich Block payload.".to_string(),
        ));
    }
    Ok(id)
}

fn file_reference_set(
    references: Vec<FileReferenceInput>,
) -> Result<FileReferenceSet, SessionsApplicationError> {
    FileReferenceSet::new(
        references
            .into_iter()
            .map(|reference| {
                FileReference::new(
                    reference.id,
                    reference.path,
                    reference.name,
                    reference.size_bytes,
                    reference.content_hash,
                    FileLineRange::from_optional_bounds(reference.start_line, reference.end_line)?,
                )
            })
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(Into::into)
}

/// Renders one referenced file for prompt injection. A reference without a range keeps the
/// exact block shape it has always had; a ranged one is clamped to the file and carries
/// 1-based positions so the Agent cites lines that match the user's editor.
pub(super) fn render_reference_block(
    path: &str,
    content: &str,
    range: Option<FileLineRange>,
) -> String {
    let Some(range) = range else {
        return format!("\n--- FILE: {path} ---\n{content}\n--- END FILE: {path} ---\n");
    };
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len() as u32;
    if range.start() > total {
        return format!(
            "\n--- FILE: {path} (lines {}-{} requested; file ends at line {total}) ---\n\n--- END FILE: {path} ---\n",
            range.start(),
            range.end(),
        );
    }
    let end = range.end().min(total);
    let body = lines[(range.start() - 1) as usize..end as usize]
        .iter()
        .enumerate()
        .map(|(offset, line)| format!("{} | {line}", range.start() + offset as u32))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "\n--- FILE: {path} (lines {}-{end}) ---\n{body}\n--- END FILE: {path} ---\n",
        range.start(),
    )
}

fn validate_usage(
    usage: Option<&MessageUsageRecord>,
    message_id: &MessageId,
    session_id: &SessionId,
) -> Result<(), SessionsApplicationError> {
    let Some(usage) = usage else {
        return Ok(());
    };
    if usage.message_id != message_id.as_str() || usage.session_id != session_id.as_str() {
        return Err(SessionsApplicationError::Validation(
            "Usage records must be owned by the completed message and session.".to_string(),
        ));
    }
    if [
        usage.input_count,
        usage.output_count,
        usage.cache_read_count,
        usage.cache_creation_count,
    ]
    .into_iter()
    .any(|value| value < 0)
    {
        return Err(SessionsApplicationError::Validation(
            "Usage counts must be non-negative.".to_string(),
        ));
    }
    Ok(())
}

fn validate_invocation_usage(
    usage: Option<&super::CompletedInvocationAccounting>,
    message_id: &MessageId,
    session_id: &SessionId,
) -> Result<(), SessionsApplicationError> {
    let Some(usage) = usage else {
        return Ok(());
    };
    if usage.invocation.message_id.as_deref() != Some(message_id.as_str())
        || usage.invocation.session_id != session_id.as_str()
        || usage.observation.invocation_id != usage.invocation.id
        || usage.status == UsageStatus::Running
    {
        return Err(SessionsApplicationError::Validation(
            "Invocation usage must belong to the completed message and session.".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn safe_export_filename(session: &SessionRecord, format: SessionExportFormat) -> String {
    let mut title = session
        .aggregate
        .title()
        .as_str()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    while title.contains("--") {
        title = title.replace("--", "-");
    }
    let title = title.trim_matches('-');
    let title = if title.is_empty() { "session" } else { title };
    format!("{}-{}.{}", title, session.id(), format.extension())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportPayload {
    version: i64,
    exported_at: String,
    session: ExportSession,
    messages: Vec<ExportMessage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportSession {
    id: String,
    title: String,
    agent_id: String,
    interaction_mode: String,
    lifecycle_state: String,
    folder: Option<String>,
    project_path: Option<String>,
    worktree_path: Option<String>,
    worktree_name: Option<String>,
    worktree_branch: Option<String>,
    remote_workspace: Option<ExportRemoteWorkspace>,
    runtime_session_id: Option<String>,
    category_id: Option<String>,
    source: ExportSessionSource,
    pinned: bool,
    archived: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportRemoteWorkspace {
    host: String,
    user: Option<String>,
    path: String,
    display_name: String,
    uri: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportSessionSource {
    kind: String,
    connector: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportMessage {
    id: String,
    session_id: String,
    role: String,
    content: String,
    status: String,
    tool_use: Option<Vec<Value>>,
    thinking_content: Option<String>,
    rich_blocks: Option<Vec<Value>>,
    token_usage: Option<super::MessageTokenUsage>,
    file_references: Option<Vec<FileReferenceInput>>,
    error: Option<String>,
    created_at: String,
    updated_at: String,
}

impl ExportPayload {
    fn from_records(
        session: &SessionRecord,
        messages: &[MessageRecord],
        exported_at: String,
    ) -> Self {
        let workspace = &session.workspace;
        let remote_workspace =
            workspace
                .remote_workspace
                .as_ref()
                .map(|workspace| ExportRemoteWorkspace {
                    host: workspace.host.clone(),
                    user: workspace.user.clone(),
                    path: workspace.path.clone(),
                    display_name: workspace.display_name.clone(),
                    uri: workspace.uri.clone(),
                });
        let aggregate = &session.aggregate;
        Self {
            version: 1,
            exported_at,
            session: ExportSession {
                id: session.id().to_string(),
                title: aggregate.title().as_str().to_string(),
                agent_id: session.agent_id.clone(),
                interaction_mode: session.interaction_mode.clone(),
                lifecycle_state: aggregate.lifecycle().as_str().to_string(),
                folder: workspace.folder.clone(),
                project_path: workspace.project_path.clone(),
                worktree_path: workspace.worktree_path.clone(),
                worktree_name: workspace.worktree_name.clone(),
                worktree_branch: workspace.worktree_branch.clone(),
                remote_workspace,
                runtime_session_id: session.runtime_session_id.clone(),
                category_id: aggregate
                    .category_id()
                    .map(|category_id| category_id.as_str().to_string()),
                source: ExportSessionSource {
                    kind: aggregate.owner().kind().to_string(),
                    connector: aggregate
                        .owner()
                        .connector_id()
                        .and_then(export_connector)
                        .map(str::to_string),
                },
                pinned: aggregate.is_pinned(),
                archived: aggregate.is_archived(),
                created_at: session.created_at.clone(),
                updated_at: session.updated_at.clone(),
            },
            messages: messages.iter().map(ExportMessage::from_record).collect(),
        }
    }

    fn render(&self, format: SessionExportFormat) -> Result<String, SessionsApplicationError> {
        match format {
            SessionExportFormat::Json => serde_json::to_string_pretty(self)
                .map_err(|error| SessionsApplicationError::Serialization(error.to_string())),
            SessionExportFormat::Markdown => self.render_markdown(),
        }
    }

    fn render_markdown(&self) -> Result<String, SessionsApplicationError> {
        let mut output = String::new();
        output.push_str(&format!("# {}\n\n", self.session.title));
        output.push_str("## Session\n\n");
        output.push_str(&format!("- ID: `{}`\n", self.session.id));
        output.push_str(&format!("- Agent: `{}`\n", self.session.agent_id));
        output.push_str(&format!(
            "- Interaction mode: `{}`\n",
            self.session.interaction_mode
        ));
        output.push_str(&format!(
            "- Lifecycle: `{}`\n",
            self.session.lifecycle_state
        ));
        output.push_str(&format!("- Archived: `{}`\n", self.session.archived));
        output.push_str(&format!("- Pinned: `{}`\n", self.session.pinned));
        if let Some(category_id) = &self.session.category_id {
            output.push_str(&format!("- Category ID: `{category_id}`\n"));
        }
        if let Some(folder) = &self.session.folder {
            output.push_str(&format!("- Folder: `{folder}`\n"));
        }
        if let Some(project_path) = &self.session.project_path {
            output.push_str(&format!("- Project: `{project_path}`\n"));
        }
        output.push_str(&format!("- Created: `{}`\n", self.session.created_at));
        output.push_str(&format!("- Updated: `{}`\n", self.session.updated_at));
        output.push_str(&format!("- Exported: `{}`\n\n", self.exported_at));
        output.push_str("## Messages\n\n");
        for message in &self.messages {
            output.push_str(&format!(
                "### {} - `{}`\n\n",
                message.role.to_uppercase(),
                message.status
            ));
            output.push_str(&format!("- Message ID: `{}`\n", message.id));
            output.push_str(&format!("- Created: `{}`\n", message.created_at));
            if let Some(usage) = &message.token_usage {
                output.push_str(&format!(
                    "- Token usage: input `{}`, output `{}`\n",
                    usage.input, usage.output
                ));
            }
            if let Some(references) = &message.file_references {
                if !references.is_empty() {
                    output.push_str("- File references:\n");
                    for reference in references {
                        output.push_str(&format!("  - `{}`\n", reference.path));
                    }
                }
            }
            output.push('\n');
            output.push_str(&message.content);
            output.push_str("\n\n");
            if let Some(thinking) = &message.thinking_content {
                output.push_str("#### Thinking\n\n");
                output.push_str(&markdown_code_block("", thinking));
                output.push('\n');
            }
            if let Some(tool_use) = &message.tool_use {
                if !tool_use.is_empty() {
                    output.push_str("#### Tool Use\n\n");
                    let raw = serde_json::to_string_pretty(tool_use).map_err(|error| {
                        SessionsApplicationError::Serialization(error.to_string())
                    })?;
                    output.push_str(&markdown_code_block("json", &raw));
                    output.push('\n');
                }
            }
            if let Some(error) = &message.error {
                output.push_str("#### Error\n\n");
                output.push_str(&markdown_code_block("", error));
                output.push('\n');
            }
        }
        Ok(output)
    }
}

fn export_connector(connector: &str) -> Option<&'static str> {
    match connector {
        "feishu" => Some("feishu"),
        "telegram" => Some("telegram"),
        "dingtalk" => Some("dingtalk"),
        "wecom" => Some("wecom"),
        "weixin" | "wechat" => Some("weixin"),
        _ => None,
    }
}

impl ExportMessage {
    fn from_record(record: &MessageRecord) -> Self {
        let references = record.message.file_references();
        Self {
            id: record.message.id().as_str().to_string(),
            session_id: record.message.session_id().as_str().to_string(),
            role: record.message.role().as_str().to_string(),
            content: record.content.clone(),
            status: record.message.status().as_str().to_string(),
            tool_use: record.tool_use.clone(),
            thinking_content: record.thinking_content.clone(),
            rich_blocks: record.rich_blocks.clone(),
            token_usage: record.token_usage.clone(),
            file_references: (!references.as_slice().is_empty())
                .then(|| super::models::references_from_domain(references)),
            error: record.error.clone(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        }
    }
}

fn markdown_code_block(language: &str, content: &str) -> String {
    let fence = if content.contains("```") {
        "````"
    } else {
        "```"
    };
    format!("{fence}{language}\n{content}\n{fence}\n")
}
