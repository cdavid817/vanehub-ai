use crate::contexts::operations::api::RunOwnerRecoveryPort;
use crate::contexts::operations::api::{AgentRunsApi, OperationsApi};
use crate::contexts::operations::application::{MissionControlService, SessionLogQueryService};
use crate::contexts::operations::infrastructure::{
    persistent_operation_service, persistent_run_service, BoundedLogIndexDiagnostics,
    SqliteLogIndexRepository, SqliteMissionControlRepository, SystemLogIndexClock,
    TauriBackfillPublisher, UnifiedLogSourceReader, UuidLogIndexIds,
};
use crate::contexts::operations::log_api::SessionLogApi;
use crate::platform::database::NativeDatabase;
use std::sync::Arc;

pub(crate) fn assemble_operations_api(database: NativeDatabase) -> OperationsApi {
    OperationsApi::new(persistent_operation_service(database))
}

#[cfg(test)]
pub(crate) fn assemble_agent_runs_api(
    database: NativeDatabase,
    evidence: Arc<dyn crate::contexts::operations::api::OperationsEvidencePort>,
) -> AgentRunsApi {
    AgentRunsApi::new(
        persistent_run_service(database.clone(), evidence),
        MissionControlService::new(std::sync::Arc::new(SqliteMissionControlRepository::new(
            database,
        ))),
    )
}

pub(crate) fn assemble_agent_runs_api_with_recovery(
    database: NativeDatabase,
    recovery: Arc<dyn RunOwnerRecoveryPort>,
    evidence: Arc<dyn crate::contexts::operations::api::OperationsEvidencePort>,
) -> AgentRunsApi {
    AgentRunsApi::new(
        persistent_run_service(database.clone(), evidence).with_recovery_port(recovery),
        MissionControlService::new(Arc::new(SqliteMissionControlRepository::new(database))),
    )
}

/// Assembles the indexed session-log read boundary.
///
/// Every concrete choice — SQLite for the index, JSONL files for the source, Tauri for the notice —
/// is made here and nowhere else. The application service that owns query semantics never learns
/// any of them, which is what lets those semantics be tested without a database or a window.
pub(crate) fn assemble_session_log_api(
    database: NativeDatabase,
    app: tauri::AppHandle,
    log_directory: std::path::PathBuf,
) -> SessionLogApi {
    SessionLogApi::new(Arc::new(SessionLogQueryService::new(
        Arc::new(SqliteLogIndexRepository::new(database)),
        Arc::new(UnifiedLogSourceReader::new(log_directory)),
        Arc::new(SystemLogIndexClock),
        Arc::new(UuidLogIndexIds),
        Arc::new(BoundedLogIndexDiagnostics::default()),
        Arc::new(TauriBackfillPublisher::new(app)),
    )))
}

/// Brings the index up to date with the retained log files, off the startup path.
///
/// Startup stays responsive while this runs, and queries answer with `indexing` coverage until it
/// finishes — which is the honest thing to report: the rows returned are real, and the set is not
/// yet final.
pub(crate) fn start_log_index_repair_job(logs: SessionLogApi) {
    tauri::async_runtime::spawn(async move {
        logs.repair_blocking().await;
    });
}
