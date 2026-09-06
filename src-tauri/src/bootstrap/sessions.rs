use crate::commands::sessions::events::NativeSessionRecoveryEvents;
use crate::contexts::agent_runtime::application::AgentRegistryRepository;
use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::{ArchivalPolicy, SessionsApi};
use crate::contexts::sessions::application::{
    PreparedReviewFeedback, ReviewAction, ReviewApplicationError, ReviewApplicationService,
    ReviewFeedbackPort, ReviewHunkWitnessPort, ReviewLogEvent, ReviewLoggingPort,
    ReviewOperationPort, ReviewSnapshotPort, SessionApplicationPorts, SessionDeletionCoordinator,
    SessionDeletionPorts, SessionRecoveryCoordinator, SessionsApplicationService,
};
use crate::contexts::sessions::infrastructure::{
    AgentSessionRuntimeAdapter, InMemoryDeletionPreviewStore, LeaseDeletionOwner,
    SessionAgentEligibilityAdapter, SessionCreationContextAdapter, SessionFileAdapter,
    SessionOperationAdapter, SqliteDeletionJournal, SqliteDeletionReferences,
    SqliteReviewDecisionRepository, SqliteReviewRepository, SqliteSessionChatProfileAdapter,
    SqliteSessionsRepository, SystemDeletionClock, SystemReviewClock, SystemSessionClock,
    UnifiedSessionLoggingAdapter, UuidDeletionIds, UuidReviewIds, UuidSessionIdentities,
    WorkspaceDeletionAdapter,
};
use crate::contexts::tooling::api::CliParameterRuntimeApi;
use crate::contexts::tooling::cli::application::native_config::NativeConfigPort;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use crate::platform::instance_lease::InstanceLease;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

const SESSION_MAINTENANCE_INTERVAL: Duration = Duration::from_secs(60 * 60);

pub(crate) struct SessionRuntimeDependencies {
    pub(crate) app: AppHandle,
    pub(crate) operations: OperationsApi,
    pub(crate) workspaces: WorkspaceApi,
    pub(crate) lease: InstanceLease,
}

pub(crate) fn assemble_sessions_api(
    database: NativeDatabase,
    runtime: SessionRuntimeDependencies,
    cli_parameter_runtime: CliParameterRuntimeApi,
    native_config: Arc<dyn NativeConfigPort>,
    agent_registry: Arc<dyn AgentRegistryRepository>,
    fallback_log_directory: PathBuf,
    evidence: Arc<dyn crate::contexts::sessions::api::SessionEvidencePort>,
) -> (
    SessionsApi,
    AgentSessionRuntimeAdapter,
    SessionRecoveryCoordinator,
) {
    let SessionRuntimeDependencies {
        app,
        operations,
        workspaces,
        lease,
    } = runtime;
    let repository = Arc::new(SqliteSessionsRepository::new(database.clone()));
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    let runtime_adapter = AgentSessionRuntimeAdapter::new(workspaces.clone(), repository.clone())
        .with_operations(operations.clone());
    let clock = Arc::new(SystemSessionClock);
    let session_logging = Arc::new(UnifiedSessionLoggingAdapter::new(logging.clone()));
    let recovery_events = Arc::new(NativeSessionRecoveryEvents::new(app));
    let recovery = SessionRecoveryCoordinator::new(
        repository.clone(),
        repository.clone(),
        Arc::new(runtime_adapter.clone()),
        clock.clone(),
        session_logging.clone(),
    )
    .with_events(recovery_events.clone());
    let service = SessionsApplicationService::new(
        SessionApplicationPorts {
            sessions: repository.clone(),
            messages: repository.clone(),
            categories: repository.clone(),
            configurations: repository.clone(),
            usage: repository.clone(),
            accounting: repository.clone(),
            transactions: repository.clone(),
            recovery_reports: repository.clone(),
            recovery_events: recovery_events.clone(),
            clock,
            identities: Arc::new(UuidSessionIdentities),
            files: Arc::new(SessionFileAdapter::new(workspaces.clone(), logging.clone())),
            operations: Arc::new(SessionOperationAdapter::new(operations.clone())),
            logging: session_logging.clone(),
            chat_profiles: Arc::new(SqliteSessionChatProfileAdapter::new(
                database.clone(),
                cli_parameter_runtime,
                native_config,
            )),
            creation: Arc::new(SessionCreationContextAdapter::new(
                database.clone(),
                workspaces.clone(),
            )),
            eligibility: Arc::new(SessionAgentEligibilityAdapter::new(agent_registry)),
            runtime: Arc::new(runtime_adapter.clone()),
        },
        evidence.clone(),
    );
    // The deletion coordinator is assembled after the service because it reads the same
    // repository, and the service takes it back as its admission port: one object decides both
    // "may this session be deleted" and "may this session start work".
    let deletion_clock = Arc::new(SystemDeletionClock);
    let deletion = SessionDeletionCoordinator::new(SessionDeletionPorts {
        sessions: repository.clone(),
        journal: Arc::new(SqliteDeletionJournal::new(
            database.clone(),
            deletion_clock.clone(),
        )),
        runtime: Arc::new(runtime_adapter.clone()),
        workspace: Arc::new(WorkspaceDeletionAdapter::new(
            workspaces.clone(),
            operations.clone(),
        )),
        references: Arc::new(SqliteDeletionReferences::new(
            database.clone(),
            runtime_adapter.clone(),
        )),
        previews: Arc::new(InMemoryDeletionPreviewStore::default()),
        clock: deletion_clock,
        ids: Arc::new(UuidDeletionIds),
        owner: Arc::new(LeaseDeletionOwner::new(lease)),
        logging: session_logging.clone(),
        events: recovery_events.clone(),
    });
    let service = service.with_execution_admission(Arc::new(deletion.clone()));
    let review = ReviewApplicationService::new(
        Arc::new(SqliteReviewRepository::new(database.clone())),
        Arc::new(SqliteReviewDecisionRepository::new(database)),
        Arc::new(WorkspaceReviewHunkWitnessAdapter(workspaces.clone())),
        Arc::new(SystemReviewClock),
        Arc::new(UuidReviewIds),
        Arc::new(SessionReviewFeedbackAdapter(service.clone())),
        Arc::new(WorkspaceReviewSnapshotAdapter(workspaces.clone())),
        Arc::new(SessionReviewOperationAdapter(operations.clone())),
        Arc::new(SessionReviewLoggingAdapter(logging)),
        evidence.clone(),
    );
    (
        SessionsApi::new(service)
            .with_review(review)
            .with_deletion(deletion),
        runtime_adapter,
        recovery,
    )
}

/// The workspaces context asks sessions whether a session may open a Shell. Bootstrap is the one
/// place allowed to know both, so the bridge lives here rather than in either context.
pub(crate) struct SessionExecutionAdmissionBridge(pub(crate) SessionsApi);

impl crate::contexts::workspaces::api::WorkspaceExecutionAdmissionPort
    for SessionExecutionAdmissionBridge
{
    fn ensure_session_admits_execution(&self, session_id: &str) -> Result<(), &'static str> {
        self.0
            .session_admits_execution(session_id)
            .map_err(|_| crate::contexts::sessions::api::deletion_error_code::SESSION_CLAIMED)
    }
}

struct SessionReviewLoggingAdapter(Arc<dyn DiagnosticLogPort>);

impl ReviewLoggingPort for SessionReviewLoggingAdapter {
    fn record(&self, event: ReviewLogEvent) {
        let mut context = BTreeMap::new();
        context.insert("reviewId".to_string(), event.review_id);
        context.insert("itemCount".to_string(), event.item_count.to_string());
        let _ = self.0.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Info,
            category: "session.code-review".to_string(),
            message: event.kind.to_string(),
            context,
        });
    }
}

struct SessionReviewOperationAdapter(OperationsApi);

impl ReviewOperationPort for SessionReviewOperationAdapter {
    fn start(
        &self,
        review_id: &str,
        action: ReviewAction,
    ) -> Result<String, ReviewApplicationError> {
        let label = match action {
            ReviewAction::ReviewAgent => "review-agent",
            ReviewAction::Tests => "tests",
            ReviewAction::Security => "security",
        };
        self.0
            .start(
                crate::contexts::operations::api::OperationKind::Workspace,
                Some(review_id.to_string()),
                Some(label.to_string()),
            )
            .map(|operation| operation.id)
            .map_err(|error| ReviewApplicationError::Repository(error.to_string()))
    }
}

struct WorkspaceReviewSnapshotAdapter(WorkspaceApi);

impl ReviewSnapshotPort for WorkspaceReviewSnapshotAdapter {
    fn snapshot(
        &self,
        session_id: &str,
    ) -> Result<crate::contexts::sessions::application::CreateReviewRequest, ReviewApplicationError>
    {
        let snapshot = self
            .0
            .create_review_snapshot(session_id)
            .map_err(|error| ReviewApplicationError::Repository(error.to_string()))?;
        let files = snapshot
            .files
            .into_iter()
            .map(|file| {
                crate::contexts::sessions::domain::ReviewFile::try_new(
                    file.path,
                    file.previous_path,
                    file.change_type,
                    file.old_hash,
                    file.new_hash,
                )
                .map_err(ReviewApplicationError::Domain)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(
            crate::contexts::sessions::application::CreateReviewRequest {
                session_id: session_id.to_string(),
                workspace_id: snapshot.workspace_id,
                base_revision: snapshot.base_revision,
                head_revision: snapshot.head_revision,
                fingerprint: snapshot.fingerprint,
                files,
            },
        )
    }
}

struct WorkspaceReviewHunkWitnessAdapter(WorkspaceApi);

impl ReviewHunkWitnessPort for WorkspaceReviewHunkWitnessAdapter {
    fn hunk_fingerprints(
        &self,
        session_id: &str,
        path: &str,
        expected_snapshot: &str,
    ) -> Result<Vec<String>, ReviewApplicationError> {
        // The same bounded load the Review Center renders from, so the fingerprints checked here
        // are the ones the reviewer was shown. A second way of computing them would eventually
        // disagree with the first, and the disagreement would look like every decision being
        // stale.
        let file = self
            .0
            .load_review_file(session_id, path, expected_snapshot)
            .map_err(|error| ReviewApplicationError::Repository(error.to_string()))?;
        Ok(file
            .hunks
            .into_iter()
            .map(|hunk| hunk.fingerprint)
            .collect())
    }
}

struct SessionReviewFeedbackAdapter(SessionsApplicationService);

impl ReviewFeedbackPort for SessionReviewFeedbackAdapter {
    fn send(
        &self,
        session_id: &str,
        feedback: &PreparedReviewFeedback,
    ) -> Result<String, ReviewApplicationError> {
        let mut content = String::from("Review feedback:\n");
        for (index, comment) in feedback.comments.iter().enumerate() {
            let stale = if comment.stale { " [stale]" } else { "" };
            content.push_str(&format!(
                "{}. {}:{}-{}{} {}\n",
                index + 1,
                comment.file_path,
                comment.start_line,
                comment.end_line,
                stale,
                comment.body
            ));
        }
        self.0
            .create_message(
                crate::contexts::sessions::application::CreateMessageRequest {
                    session_id: session_id.to_string(),
                    speaker_seat_id: None,
                    seat_index: None,
                    role: "user".to_string(),
                    status: "completed".to_string(),
                    content,
                    file_references: Vec::new(),
                },
            )
            .map(|message| message.message.id().as_str().to_string())
            .map_err(|error| ReviewApplicationError::Feedback(error.to_string()))
    }
}

pub(crate) fn start_session_maintenance_jobs(
    api: SessionsApi,
    settings: DesktopSettingsApi,
    fallback_log_directory: PathBuf,
) {
    let logging: Arc<dyn DiagnosticLogPort> =
        Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    thread::spawn(move || loop {
        run_session_maintenance_cycle(&api, &settings, logging.as_ref());
        thread::sleep(SESSION_MAINTENANCE_INTERVAL);
    });
}

fn run_session_maintenance_cycle(
    api: &SessionsApi,
    settings: &DesktopSettingsApi,
    logging: &dyn DiagnosticLogPort,
) {
    let policy = match settings.get_automatic_archival_settings() {
        Ok(settings) => ArchivalPolicy {
            enabled: settings.enabled(),
            inactive_days: settings.inactive_days(),
        },
        Err(error) => {
            write_maintenance_error(
                logging,
                format!("Automatic archival settings could not be loaded: {error}"),
            );
            ArchivalPolicy {
                enabled: false,
                inactive_days: 1,
            }
        }
    };
    if let Err(error) = api.run_maintenance(policy) {
        write_maintenance_error(logging, format!("Session maintenance failed: {error}"));
    }
}

fn write_maintenance_error(logging: &dyn DiagnosticLogPort, message: String) {
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Error,
        category: "session.maintenance".to_string(),
        message,
        context: Default::default(),
    });
}
