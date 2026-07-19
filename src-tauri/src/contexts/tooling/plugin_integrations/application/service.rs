use super::{
    PluginIntegrationApplicationError, PluginIntegrationClockPort, PluginIntegrationDiagnostic,
    PluginIntegrationLoggingPort, PluginIntegrationOverview, PluginIntegrationToolPort,
};
use crate::contexts::tooling::plugin_integrations::domain::{
    definitions, evaluate_readiness, native_environment, readiness_plan, PluginIntegrationId,
    PluginIntegrationState, PluginIntegrationTestResult,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct PluginIntegrationApplicationService {
    tools: Arc<dyn PluginIntegrationToolPort>,
    logging: Arc<dyn PluginIntegrationLoggingPort>,
    clock: Arc<dyn PluginIntegrationClockPort>,
}

impl PluginIntegrationApplicationService {
    pub(crate) fn new(
        tools: Arc<dyn PluginIntegrationToolPort>,
        logging: Arc<dyn PluginIntegrationLoggingPort>,
        clock: Arc<dyn PluginIntegrationClockPort>,
    ) -> Self {
        Self {
            tools,
            logging,
            clock,
        }
    }

    pub(crate) fn overview(&self) -> PluginIntegrationOverview {
        PluginIntegrationOverview {
            definitions: definitions().to_vec(),
            states: definitions()
                .iter()
                .map(|definition| PluginIntegrationState::initial(definition.id))
                .collect(),
            environment: native_environment(),
        }
    }

    pub(crate) fn refresh(&self) -> PluginIntegrationOverview {
        self.overview()
    }

    pub(crate) fn test_readiness(
        &self,
        integration_id: &str,
    ) -> Result<PluginIntegrationTestResult, PluginIntegrationApplicationError> {
        let integration_id = PluginIntegrationId::parse(integration_id)?;
        let outcome = self.tools.execute(readiness_plan(integration_id));
        let result = evaluate_readiness(integration_id, &outcome, self.clock.now());
        let diagnostic = PluginIntegrationDiagnostic::readiness(
            result.integration_id,
            result.status,
            result.message.clone(),
            result.checked_at.clone(),
        );

        // Readiness remains observable even if diagnostic persistence is temporarily unavailable.
        let _ = self.logging.record(&diagnostic);
        Ok(result)
    }
}
