use crate::contexts::tooling::plugin_integrations::application::PluginIntegrationApplicationService;

pub(crate) use crate::contexts::tooling::plugin_integrations::application::{
    PluginIntegrationApplicationError as PluginIntegrationError, PluginIntegrationOverview,
};
pub(crate) use crate::contexts::tooling::plugin_integrations::domain::{
    PluginIntegrationDefinition, PluginIntegrationEnvironment, PluginIntegrationId,
    PluginIntegrationState, PluginIntegrationStatus, PluginIntegrationTestResult,
};

#[derive(Clone)]
pub(crate) struct PluginIntegrationApi {
    service: PluginIntegrationApplicationService,
}

impl PluginIntegrationApi {
    pub(crate) fn new(service: PluginIntegrationApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn overview(&self) -> PluginIntegrationOverview {
        self.service.overview()
    }

    pub(crate) fn refresh(&self) -> PluginIntegrationOverview {
        self.service.refresh()
    }

    pub(crate) fn test_readiness(
        &self,
        integration_id: &str,
    ) -> Result<PluginIntegrationTestResult, PluginIntegrationError> {
        self.service.test_readiness(integration_id)
    }
}
