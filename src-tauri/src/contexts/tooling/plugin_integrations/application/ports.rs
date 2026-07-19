use super::{PluginIntegrationApplicationError, PluginIntegrationDiagnostic};
use crate::contexts::tooling::plugin_integrations::domain::{
    PluginIntegrationToolOutcome, PluginIntegrationToolPlan,
};

pub(crate) trait PluginIntegrationToolPort: Send + Sync {
    fn execute(&self, plan: PluginIntegrationToolPlan) -> PluginIntegrationToolOutcome;
}

pub(crate) trait PluginIntegrationClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait PluginIntegrationLoggingPort: Send + Sync {
    fn record(
        &self,
        diagnostic: &PluginIntegrationDiagnostic,
    ) -> Result<(), PluginIntegrationApplicationError>;
}
