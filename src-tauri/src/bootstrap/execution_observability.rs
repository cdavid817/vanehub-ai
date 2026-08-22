use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::execution_observability::api::evidence::ExecutionEvidenceApi;
use crate::contexts::execution_observability::api::{
    ExecutionObservabilityApi, ExecutionTelemetryPort,
};
use crate::contexts::execution_observability::application::EvaluationRepositoryPort;
use crate::contexts::execution_observability::infrastructure::OsObservabilityCredentialAdapter;
use crate::contexts::execution_observability::infrastructure::{
    DomainEvidenceRedactionValidator, NativeEvaluationAgentAdapter,
    NativeEvaluationVerifierAdapter, RateLimitedEvidenceDiagnostics, SqliteEvaluationRepository,
    SqliteEvidenceRepository, SqliteExecutionTimelineRepository, SystemEvidenceClock,
    TauriEvidenceNoticePublisher, UuidEvidenceIdGenerator,
};
use crate::contexts::execution_observability::EvaluationApi;
use crate::contexts::operations::api::{AgentRunsApi, OperationsApi};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub(crate) fn assemble_execution_observability_api(
    database: NativeDatabase,
) -> ExecutionObservabilityApi {
    ExecutionObservabilityApi::new(
        Arc::new(SqliteExecutionTimelineRepository::new(database)),
        Arc::new(OsObservabilityCredentialAdapter::new()),
    )
}

/// Assembles the execution evidence capability from the pieces the process already owns.
///
/// The shared database handle is reused rather than opened again: a second file would give the
/// journal its own transaction boundary, and an evidence row that commits while the work it
/// describes rolls back is worse than no row at all.
///
/// The returned API is the only handle anything outside the context receives. Commands take it
/// from managed state, so no handler builds a repository, and none of them is rebuilt per request.
pub(crate) fn assemble_execution_evidence_api(
    database: NativeDatabase,
    app: tauri::AppHandle,
    logging: Arc<dyn DiagnosticLogPort>,
) -> ExecutionEvidenceApi {
    ExecutionEvidenceApi::new(
        Arc::new(SqliteEvidenceRepository::new(database)),
        Arc::new(SystemEvidenceClock),
        Arc::new(UuidEvidenceIdGenerator),
        Arc::new(DomainEvidenceRedactionValidator),
        Arc::new(TauriEvidenceNoticePublisher::new(app)),
        Arc::new(RateLimitedEvidenceDiagnostics::new(logging)),
    )
}

/// How long evidence is retained. Longer than the timeline's default because evidence is
/// metadata-only and cheap to keep, and a coverage window shorter than the work a user is still
/// reviewing turns "we deleted it" into an answer indistinguishable from "it never happened".
const EVIDENCE_RETENTION_DAYS: i64 = 30;

/// Rebuilds projections once at startup, then prunes on a schedule.
///
/// Replay is first and unconditional: a process that died mid-write can leave a projection that
/// disagrees with the journal, and every query answers from the projection. It appends nothing and
/// publishes nothing, so running it on a clean store costs a pass over the journal and no more.
pub(crate) fn start_evidence_maintenance_job(
    evidence: ExecutionEvidenceApi,
    fallback_log_directory: PathBuf,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = evidence.replay_projections() {
            write_evidence_maintenance_log(
                &fallback_log_directory,
                LogSeverity::Warn,
                "Execution evidence projections could not be rebuilt at startup",
                None,
            );
            // The reason code, never the storage message: it names tables and paths.
            let _ = error;
        }
        loop {
            run_evidence_retention_cycle(&evidence, &fallback_log_directory);
            sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}

fn run_evidence_retention_cycle(
    evidence: &ExecutionEvidenceApi,
    fallback_log_directory: &std::path::Path,
) {
    let cutoff =
        (chrono::Utc::now() - chrono::Duration::days(EVIDENCE_RETENTION_DAYS)).to_rfc3339();
    match evidence.maintain_retention(&cutoff) {
        Ok(summary) if summary.deleted_events > 0 => write_evidence_maintenance_log(
            fallback_log_directory,
            LogSeverity::Info,
            "Execution evidence retention removed expired events",
            Some(summary.deleted_events),
        ),
        Ok(_) => {}
        Err(_) => write_evidence_maintenance_log(
            fallback_log_directory,
            LogSeverity::Warn,
            "Execution evidence retention was deferred after a storage error",
            None,
        ),
    }
}

fn write_evidence_maintenance_log(
    fallback_log_directory: &std::path::Path,
    severity: LogSeverity,
    message: &str,
    deleted_events: Option<usize>,
) {
    let logging = UnifiedLoggingAdapter::active(fallback_log_directory.to_path_buf());
    let mut context = BTreeMap::from([("source".to_string(), "scheduled-maintenance".to_string())]);
    if let Some(deleted_events) = deleted_events {
        context.insert("deletedEvents".to_string(), deleted_events.to_string());
    }
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity,
        category: "execution.evidence.retention".to_string(),
        message: message.to_string(),
        context,
    });
}

pub(crate) fn assemble_evaluation_api(
    database: NativeDatabase,
    operations: OperationsApi,
    runs: AgentRunsApi,
    runtime: AgentRuntimeApi,
    sessions: SessionsApi,
    workspaces: WorkspaceApi,
    run_root: PathBuf,
) -> EvaluationApi {
    EvaluationApi::new(
        Arc::new(SqliteEvaluationRepository::new(database)),
        operations,
        runs,
        runtime.clone(),
        NativeEvaluationAgentAdapter::new(sessions, runtime.clone()),
        NativeEvaluationVerifierAdapter::new(runtime),
        workspaces,
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("evaluation-fixtures"),
        run_root,
    )
}

pub(crate) fn relay_telemetry(
    data_root: &std::path::Path,
) -> Option<Arc<dyn ExecutionTelemetryPort>> {
    let database = NativeDatabase::new(data_root.to_path_buf()).ok()?;
    Some(Arc::new(SqliteExecutionTimelineRepository::new(database)))
}

pub(crate) fn start_execution_retention_job(
    database: NativeDatabase,
    fallback_log_directory: PathBuf,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            run_retention_cycle(&database, &fallback_log_directory);
            sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}

fn run_retention_cycle(database: &NativeDatabase, fallback_log_directory: &std::path::Path) {
    let repository = SqliteExecutionTimelineRepository::new(database.clone());
    let result = repository.load_settings().and_then(|settings| {
        repository.maintain_retention(&chrono::Utc::now().to_rfc3339(), settings.retention_days)
    });
    match result {
        Ok(outcome) if outcome.ran && outcome.deleted_runs > 0 => {
            write_retention_log(
                fallback_log_directory,
                LogSeverity::Info,
                "Execution timeline retention removed expired runs",
                Some(outcome.deleted_runs),
            );
        }
        Ok(_) => {}
        Err(_) => write_retention_log(
            fallback_log_directory,
            LogSeverity::Warn,
            "Execution timeline retention was deferred after a storage error",
            None,
        ),
    }
    let evaluations: Arc<dyn EvaluationRepositoryPort> =
        Arc::new(SqliteEvaluationRepository::new(database.clone()));
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();
    let _ = evaluations.retain_since(&cutoff);
}

fn write_retention_log(
    fallback_log_directory: &std::path::Path,
    severity: LogSeverity,
    message: &str,
    deleted_runs: Option<usize>,
) {
    let logging = UnifiedLoggingAdapter::active(fallback_log_directory.to_path_buf());
    let mut context = BTreeMap::from([("source".to_string(), "scheduled-maintenance".to_string())]);
    if let Some(deleted_runs) = deleted_runs {
        context.insert("deletedRuns".to_string(), deleted_runs.to_string());
    }
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity,
        category: "execution_observability.retention".to_string(),
        message: message.to_string(),
        context,
    });
}
