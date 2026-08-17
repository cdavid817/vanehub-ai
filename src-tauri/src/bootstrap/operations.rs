use crate::contexts::operations::api::{AgentRunsApi, OperationsApi};
use crate::contexts::operations::application::MissionControlService;
use crate::contexts::operations::infrastructure::{
    persistent_operation_service, persistent_run_service, SqliteMissionControlRepository,
};
use crate::platform::database::NativeDatabase;

pub(crate) fn assemble_operations_api(database: NativeDatabase) -> OperationsApi {
    OperationsApi::new(persistent_operation_service(database))
}

pub(crate) fn assemble_agent_runs_api(database: NativeDatabase) -> AgentRunsApi {
    AgentRunsApi::new(
        persistent_run_service(database.clone()),
        MissionControlService::new(std::sync::Arc::new(SqliteMissionControlRepository::new(
            database,
        ))),
    )
}
