use crate::contexts::tooling::cli::application::CliApplicationService;
pub(crate) use crate::contexts::tooling::cli::application::{
    CliApplicationError as CliError, CliToolStatus, PreparedCliInstall, PreparedCliRefresh,
    PreparedCliUpgradeAll, StartedCliOperation,
};

#[derive(Clone)]
pub(crate) struct CliApi {
    service: CliApplicationService,
}

impl CliApi {
    pub(crate) fn new(service: CliApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn list_tools(&self) -> Result<Vec<CliToolStatus>, CliError> {
        self.service.list_tools()
    }

    pub(crate) fn needs_initial_refresh(&self) -> Result<bool, CliError> {
        self.service.needs_initial_refresh()
    }

    pub(crate) fn resolve_executable(&self, agent_id: &str) -> Result<Option<String>, CliError> {
        self.service.resolve_executable(agent_id)
    }

    pub(crate) fn prepare_refresh(
        &self,
        agent_id: Option<String>,
        message: String,
    ) -> Result<PreparedCliRefresh, CliError> {
        self.service.prepare_refresh(agent_id, message)
    }

    pub(crate) fn execute_refresh(&self, prepared: PreparedCliRefresh) -> Result<(), CliError> {
        self.service.execute_refresh(prepared)
    }

    pub(crate) fn prepare_install(
        &self,
        agent_id: String,
        target_version: String,
        confirmed_active_path: Option<String>,
    ) -> Result<PreparedCliInstall, CliError> {
        self.service
            .prepare_install(agent_id, target_version, confirmed_active_path)
    }

    pub(crate) fn execute_install(&self, prepared: PreparedCliInstall) -> Result<(), CliError> {
        self.service.execute_install(prepared)
    }

    pub(crate) fn prepare_upgrade_all(&self) -> Result<PreparedCliUpgradeAll, CliError> {
        self.service.prepare_upgrade_all()
    }

    pub(crate) fn execute_upgrade_all(
        &self,
        prepared: PreparedCliUpgradeAll,
    ) -> Result<(), CliError> {
        self.service.execute_upgrade_all(prepared)
    }
}
