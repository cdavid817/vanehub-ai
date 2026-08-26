/// Named only where a test double has to write the type down.
///
/// The production implementation lives beside the session export it delegates to and reaches the
/// trait through the application module, so nothing outside this context needs the name.
#[cfg(test)]
pub(crate) use super::application::ReportExportPort;
/// The session-run report: its service, the questions it asks, and the shapes it answers with.
///
/// The ports are published alongside the service because bootstrap implements them. That is the
/// whole arrangement: the sessions context states what a report needs in its own vocabulary, and
/// the layer that is allowed to know every context supplies the answers.
pub(crate) use super::application::{
    AgentReportRow, ChangeSummary, ChangeSummaryPort, CommandReport, ExecutionEvidencePort,
    ExecutionEvidenceSummary, FailureReportRow, LogFailurePort, LogFailureSummary,
    ObservabilityTimingPort, ReportClock, ReportCoverage, ReportCoverageState, ReportEvidenceLink,
    ReportScope, ReportScopeRequest, ReportSectionCoverage, ReportSourceError, ReportSourceResult,
    ReportUsagePort, ReportUsageSummary, RunOutcomePort, RunOutcomeSummary, SessionRunReport,
    SessionRunReportService, TimingSummary, ToolReportRow, VerificationReport,
};
pub(crate) use super::application::{
    ArchivalPolicy, CategoryRecord, ChatConfigurationValues, CompleteMessageRequest,
    CompletedInvocationAccounting, CreateMessageRequest, DurableGenerationStartRequest,
    DurableGenerationTerminalRequest, FailMessageRequest, FileReferenceInput,
    GenerationStartResult, GenerationTerminalResult, GenerationTerminalStatus,
    InvocationDetailQuery, LoopRoleSessionRequest, MessageRecord, MessageTokenUsage,
    MessageUsageRecord, NewModelInvocation, NewRemoteWorkspace, NewSessionRequest,
    NewSessionWorkspace, NewUsageObservation, NewWorktree, PreparedNewSessionCreation,
    RuntimeMessageSnapshot, RuntimeSessionSnapshot, SessionChatConfiguration,
    SessionCreationOperation, SessionExportFormat, SessionExportRequest, SessionExportResult,
    SessionListScope, SessionMaintenanceResult, SessionRecord, SessionRecoveryProjection,
    SessionRecoverySummary, SessionRunnerTarget, SessionSearchMatchKind, SessionSearchResult,
    SessionUsageAccountingKind, SessionUsageStatistics, SessionUsageSummary, SessionUsageUnit,
    SessionsApplicationError as SessionsError, TokenUsageObservation, UpdateSessionSeatsRequest,
    UsageAccountingSummary, UsageBreakdownDimension, UsageCursor, UsageCursorAdvance,
    UsageDetailPage, UsageEntityCounts, UsageMeasureAggregate, UsageQualityAggregate,
    UsageStatisticsRange, UsageSummaryQuery,
};
use super::application::{ReviewApplicationService, SessionsApplicationService};
pub(crate) use super::application::{
    SessionEvidencePort, SessionEvidenceSignal, SessionReviewDecision, SessionUsageEvidenceQuality,
    SessionVerificationOutcome,
};
pub(crate) use super::domain::{
    AccountingUnit, LoopSessionRole, MeasurementKind, MeasurementQuality, RecoveryDecision,
    RecoveryEvidenceReference, RecoveryReasonCode, RecoveryTrigger, SessionActivation,
    SessionLifecycle, SessionOwner, SessionRecoveryReport, SessionRecoveryStatus, SessionSeat,
    SessionSeatRoleSnapshot, TokenDimensions, TokenOverlap, UsageInteractionKind, UsagePurpose,
    UsageStatus,
};
use serde_json::Value;

#[derive(Clone)]
pub(crate) struct SessionsApi {
    service: SessionsApplicationService,
    review: Option<ReviewApplicationService>,
}

impl SessionsApi {
    pub(crate) fn start_model_invocation(
        &self,
        invocation: &NewModelInvocation,
    ) -> Result<super::application::ModelInvocationRecord, SessionsError> {
        self.service.start_model_invocation(invocation)
    }

    pub(crate) fn finalize_model_invocation(
        &self,
        invocation_id: &str,
        status: UsageStatus,
        completed_at: &str,
    ) -> Result<super::application::ModelInvocationRecord, SessionsError> {
        self.service
            .finalize_model_invocation(invocation_id, status, completed_at)
    }

    pub(crate) fn record_token_observation(
        &self,
        observation: &NewUsageObservation,
    ) -> Result<TokenUsageObservation, SessionsError> {
        self.service.record_token_observation(observation)
    }

    pub(crate) fn advance_usage_cursor(
        &self,
        advance: &UsageCursorAdvance,
    ) -> Result<UsageCursor, SessionsError> {
        self.service.advance_usage_cursor(advance)
    }

    pub(crate) fn find_usage_cursor(
        &self,
        source_id: &str,
    ) -> Result<Option<UsageCursor>, SessionsError> {
        self.service.find_usage_cursor(source_id)
    }

    pub(crate) fn recovery_summary(
        &self,
        session_id: &str,
    ) -> Result<SessionRecoverySummary, SessionsError> {
        self.service.recovery_summary(session_id)
    }

    pub(crate) fn list_recovery_reports(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<SessionRecoveryReport>, SessionsError> {
        self.service.list_recovery_reports(session_id, limit)
    }

    pub(crate) fn acknowledge_recovery(
        &self,
        session_id: &str,
        expected_recovery_revision: u64,
    ) -> Result<super::application::AcknowledgeRecoveryResult, SessionsError> {
        self.service
            .acknowledge_recovery(session_id, expected_recovery_revision)
    }

    pub(crate) fn recovery_projection(
        &self,
        session_id: &str,
        execution_run_id: Option<&str>,
    ) -> Result<SessionRecoveryProjection, SessionsError> {
        self.service
            .recovery_projection(session_id, execution_run_id)
    }

    pub(crate) fn new(service: SessionsApplicationService) -> Self {
        Self {
            service,
            review: None,
        }
    }

    pub(crate) fn with_review(mut self, review: ReviewApplicationService) -> Self {
        self.review = Some(review);
        self
    }

    fn review(
        &self,
    ) -> Result<&ReviewApplicationService, super::application::ReviewApplicationError> {
        self.review.as_ref().ok_or_else(|| {
            super::application::ReviewApplicationError::Repository(
                "review service is unavailable".to_string(),
            )
        })
    }

    pub(crate) fn open_review(
        &self,
        session_id: &str,
    ) -> Result<super::domain::ReviewSession, super::application::ReviewApplicationError> {
        self.review()?.open(session_id)
    }

    pub(crate) fn find_review(
        &self,
        review_id: &str,
    ) -> Result<super::domain::ReviewSession, super::application::ReviewApplicationError> {
        self.review()?.find(review_id)
    }

    /// The session's active review without creating one.
    ///
    /// `open_review` snapshots the workspace and writes; this reads. A report needs the read, since
    /// a session with no review must be reported as having none rather than acquiring one by being
    /// reported on.
    pub(crate) fn find_active_review(
        &self,
        session_id: &str,
    ) -> Result<Option<super::domain::ReviewSession>, super::application::ReviewApplicationError>
    {
        self.review()?.find_active(session_id)
    }

    pub(crate) fn add_review_comment(
        &self,
        request: super::application::AddReviewCommentRequest,
    ) -> Result<super::domain::ReviewComment, super::application::ReviewApplicationError> {
        self.review()?.add_comment(request)
    }

    pub(crate) fn set_review_decision(
        &self,
        review_id: &str,
        decision: super::domain::ReviewDecision,
    ) -> Result<super::domain::ReviewSession, super::application::ReviewApplicationError> {
        self.review()?.set_decision(review_id, decision)
    }

    pub(crate) fn set_review_hunk_decision(
        &self,
        review_id: &str,
        request: super::application::SetHunkDecisionRequest,
    ) -> Result<super::domain::ReviewHunkDecision, super::application::ReviewApplicationError> {
        self.review()?.set_hunk_decision(review_id, request)
    }

    pub(crate) fn resolve_review_comment(
        &self,
        review_id: &str,
        comment_id: &str,
    ) -> Result<super::domain::ReviewSession, super::application::ReviewApplicationError> {
        self.review()?.resolve_comment(review_id, comment_id)
    }

    pub(crate) fn select_review_comment(
        &self,
        review_id: &str,
        comment_id: &str,
        selected: bool,
    ) -> Result<super::domain::ReviewSession, super::application::ReviewApplicationError> {
        self.review()?
            .select_comment(review_id, comment_id, selected)
    }

    pub(crate) fn send_review_feedback(
        &self,
        review_id: &str,
        acknowledge_stale: bool,
    ) -> Result<String, super::application::ReviewApplicationError> {
        self.review()?.send_feedback(review_id, acknowledge_stale)
    }

    pub(crate) fn start_review_action(
        &self,
        review_id: &str,
        action: super::application::ReviewAction,
    ) -> Result<String, super::application::ReviewApplicationError> {
        self.review()?.start_action(review_id, action)
    }

    pub(crate) fn complete_review_action(
        &self,
        review_id: &str,
        action: super::application::ReviewAction,
        operation_id: &str,
        findings: Vec<super::application::ReviewActionFindingInput>,
    ) -> Result<super::domain::ReviewSession, super::application::ReviewApplicationError> {
        self.review()?
            .project_action_findings(review_id, action, operation_id, findings)
    }

    pub(crate) fn prepare_creation(
        &self,
        request: NewSessionRequest,
    ) -> Result<PreparedNewSessionCreation, SessionsError> {
        self.service.prepare_new_session_creation(request)
    }

    pub(crate) fn execute_creation(
        &self,
        prepared: PreparedNewSessionCreation,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.execute_new_session_creation(prepared)
    }

    pub(crate) fn create_loop_role_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.create_loop_role_session(request)
    }

    pub(crate) fn list_current(&self) -> Result<Vec<SessionRecord>, SessionsError> {
        self.service.list_sessions(SessionListScope::Current)
    }

    pub(crate) fn list_archived(&self) -> Result<Vec<SessionRecord>, SessionsError> {
        self.service.list_sessions(SessionListScope::Archived)
    }

    pub(crate) fn search(
        &self,
        query: &str,
        limit: Option<i64>,
    ) -> Result<Vec<SessionSearchResult>, SessionsError> {
        self.service.search_sessions(query, limit)
    }

    pub(crate) fn active(&self) -> Result<Option<SessionRecord>, SessionsError> {
        self.service.active_session()
    }

    pub(crate) fn find(&self, session_id: &str) -> Result<Option<SessionRecord>, SessionsError> {
        self.service.find_session(session_id)
    }

    pub(crate) fn rebind_remote_ssh_connection(
        &self,
        session_id: &str,
        connection_id: &str,
    ) -> Result<SessionRecord, SessionsError> {
        self.service
            .rebind_remote_session(session_id, connection_id)
    }

    #[expect(
        dead_code,
        reason = "remote Shell routing consumes this binding guard in task 4.1"
    )]
    pub(crate) fn require_current_remote_ssh_binding(
        &self,
        session_id: &str,
    ) -> Result<super::application::SessionSshBinding, SessionsError> {
        self.service.require_current_remote_ssh_binding(session_id)
    }

    pub(crate) fn current_runner_target(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionRunnerTarget>, SessionsError> {
        self.service.current_runner_target(session_id)
    }

    pub(crate) fn switch(&self, session_id: &str) -> Result<SessionRecord, SessionsError> {
        self.service.switch_session(session_id)
    }

    pub(crate) fn rename(
        &self,
        session_id: &str,
        title: String,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.rename_session(session_id, title)
    }

    pub(crate) fn update_seats(
        &self,
        request: UpdateSessionSeatsRequest,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.update_session_seats(request)
    }

    pub(crate) fn set_pinned(
        &self,
        session_id: &str,
        pinned: bool,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.set_session_pinned(session_id, pinned)
    }

    pub(crate) fn set_archived(
        &self,
        session_id: &str,
        archived: bool,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.set_session_archived(session_id, archived)
    }

    pub(crate) fn delete(&self, session_id: &str) -> Result<(), SessionsError> {
        self.service.delete_session(session_id)
    }

    pub(crate) fn list_categories(&self) -> Result<Vec<CategoryRecord>, SessionsError> {
        self.service.list_categories()
    }

    pub(crate) fn create_category(&self, name: String) -> Result<CategoryRecord, SessionsError> {
        self.service.create_category(name)
    }

    pub(crate) fn rename_category(
        &self,
        category_id: &str,
        name: String,
    ) -> Result<CategoryRecord, SessionsError> {
        self.service.rename_category(category_id, name)
    }

    pub(crate) fn delete_category(&self, category_id: &str) -> Result<(), SessionsError> {
        self.service.delete_category(category_id)
    }

    pub(crate) fn assign_category(
        &self,
        session_id: &str,
        category_id: Option<&str>,
    ) -> Result<SessionRecord, SessionsError> {
        self.service.assign_category(session_id, category_id)
    }

    pub(crate) fn load_chat_configuration(
        &self,
        session_id: &str,
    ) -> Result<SessionChatConfiguration, SessionsError> {
        self.service.load_chat_configuration(session_id)
    }

    pub(crate) fn save_chat_configuration(
        &self,
        configuration: SessionChatConfiguration,
    ) -> Result<SessionChatConfiguration, SessionsError> {
        self.service.save_chat_configuration(configuration)
    }

    pub(crate) fn validate_chat_configuration(
        &self,
        configuration: SessionChatConfiguration,
    ) -> Result<SessionChatConfiguration, SessionsError> {
        self.service.validate_chat_configuration(configuration)
    }

    pub(crate) fn validate_seat_chat_configuration(
        &self,
        configuration: SessionChatConfiguration,
    ) -> Result<SessionChatConfiguration, SessionsError> {
        self.service.validate_seat_chat_configuration(configuration)
    }

    pub(crate) fn runtime_session(
        &self,
        session_id: &str,
    ) -> Result<Option<RuntimeSessionSnapshot>, SessionsError> {
        self.service
            .find_session(session_id)
            .map(|record| record.as_ref().map(RuntimeSessionSnapshot::from_record))
    }

    pub(crate) fn runtime_message(
        &self,
        message_id: &str,
    ) -> Result<Option<RuntimeMessageSnapshot>, SessionsError> {
        self.service
            .find_message(message_id)
            .map(|record| record.as_ref().map(RuntimeMessageSnapshot::from_record))
    }

    #[allow(dead_code)]
    pub(crate) fn create_message(
        &self,
        request: CreateMessageRequest,
    ) -> Result<MessageRecord, SessionsError> {
        self.service.create_message(request)
    }

    pub(crate) fn start_generation(
        &self,
        request: DurableGenerationStartRequest,
    ) -> Result<GenerationStartResult, SessionsError> {
        self.service.start_generation(request)
    }

    pub(crate) fn terminalize_generation(
        &self,
        request: DurableGenerationTerminalRequest,
    ) -> Result<GenerationTerminalResult, SessionsError> {
        self.service.terminalize_generation(request)
    }

    pub(crate) fn compose_prompt(
        &self,
        session_id: &str,
        content: &str,
        references: Vec<FileReferenceInput>,
    ) -> Result<String, SessionsError> {
        self.service.compose_prompt(session_id, content, references)
    }

    pub(crate) fn list_messages(
        &self,
        session_id: &str,
        limit: Option<i64>,
        before_id: Option<String>,
    ) -> Result<Vec<MessageRecord>, SessionsError> {
        self.service.list_messages(session_id, limit, before_id)
    }

    pub(crate) fn complete_message(
        &self,
        request: CompleteMessageRequest,
    ) -> Result<MessageRecord, SessionsError> {
        self.service.complete_message(request)
    }

    pub(crate) fn fail_message(
        &self,
        request: FailMessageRequest,
    ) -> Result<MessageRecord, SessionsError> {
        self.service.fail_message(request)
    }

    pub(crate) fn append_message_content(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), SessionsError> {
        self.service
            .append_message_content(message_id, content_delta)
    }

    pub(crate) fn append_message_thinking(
        &self,
        message_id: &str,
        content_delta: &str,
    ) -> Result<(), SessionsError> {
        self.service
            .append_message_thinking(message_id, content_delta)
    }

    pub(crate) fn append_message_tool_use(
        &self,
        message_id: &str,
        tool_use: Value,
    ) -> Result<(), SessionsError> {
        self.service.append_message_tool_use(message_id, tool_use)
    }

    pub(crate) fn append_message_rich_block(
        &self,
        message_id: &str,
        block: Value,
    ) -> Result<(), SessionsError> {
        self.service.append_message_rich_block(message_id, block)
    }

    pub(crate) fn cancel_streaming_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, SessionsError> {
        self.service.cancel_streaming_messages(session_id)
    }

    pub(crate) fn update_runtime_lifecycle(
        &self,
        session_id: &str,
        lifecycle: SessionLifecycle,
    ) -> Result<(), SessionsError> {
        self.service.update_runtime_lifecycle(session_id, lifecycle)
    }

    pub(crate) fn update_runtime_session_id(
        &self,
        session_id: &str,
        runtime_session_id: &str,
    ) -> Result<(), SessionsError> {
        self.service
            .update_runtime_session_id(session_id, runtime_session_id)
    }

    pub(crate) fn update_seat_provider_thread_id(
        &self,
        session_id: &str,
        seat_id: &str,
        provider_thread_id: &str,
    ) -> Result<(), SessionsError> {
        self.service
            .update_seat_provider_thread_id(session_id, seat_id, provider_thread_id)
    }

    pub(crate) fn clear_seat_provider_thread_id(
        &self,
        session_id: &str,
        seat_id: &str,
    ) -> Result<(), SessionsError> {
        self.service
            .clear_seat_provider_thread_id(session_id, seat_id)
    }

    pub(crate) fn clear_runtime_session_id(&self, session_id: &str) -> Result<(), SessionsError> {
        self.service.clear_runtime_session_id(session_id)
    }

    pub(crate) fn export(
        &self,
        request: SessionExportRequest,
    ) -> Result<SessionExportResult, SessionsError> {
        self.service.export_session(request)
    }

    pub(crate) fn usage_statistics(
        &self,
        range: UsageStatisticsRange,
    ) -> Result<SessionUsageStatistics, SessionsError> {
        self.service.usage_statistics(range)
    }

    pub(crate) fn session_usage_summary(
        &self,
        session_id: &str,
    ) -> Result<SessionUsageSummary, SessionsError> {
        self.service.session_usage_summary(session_id)
    }

    pub(crate) fn token_usage_summary(
        &self,
        query: &UsageSummaryQuery,
    ) -> Result<UsageAccountingSummary, SessionsError> {
        self.service.token_usage_summary(query)
    }

    pub(crate) fn token_usage_details(
        &self,
        query: &InvocationDetailQuery,
    ) -> Result<UsageDetailPage, SessionsError> {
        self.service.invocation_usage_details(query)
    }

    pub(crate) fn run_maintenance(
        &self,
        policy: ArchivalPolicy,
    ) -> Result<SessionMaintenanceResult, SessionsError> {
        self.service.run_maintenance(policy)
    }
}
