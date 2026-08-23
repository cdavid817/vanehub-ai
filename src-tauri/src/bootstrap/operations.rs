use crate::contexts::operations::api::RunOwnerRecoveryPort;
use crate::contexts::operations::api::{AgentRunsApi, OperationsApi};
use crate::contexts::operations::application::MissionControlService;
use crate::contexts::operations::infrastructure::{
    persistent_operation_service, persistent_run_service, SqliteMissionControlRepository,
};
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
