use crate::contexts::tooling::plugin_integrations::domain::{
    PluginIntegrationDefinition, PluginIntegrationEnvironment, PluginIntegrationId,
    PluginIntegrationState, PluginIntegrationStatus,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginIntegrationOverview {
    pub(crate) definitions: Vec<PluginIntegrationDefinition>,
    pub(crate) states: Vec<PluginIntegrationState>,
    pub(crate) environment: PluginIntegrationEnvironment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginIntegrationDiagnosticLevel {
    Info,
    Warn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginIntegrationDiagnostic {
    pub(crate) integration_id: PluginIntegrationId,
    pub(crate) operation: &'static str,
    pub(crate) status: PluginIntegrationStatus,
    pub(crate) level: PluginIntegrationDiagnosticLevel,
    pub(crate) message: String,
    pub(crate) checked_at: String,
    pub(crate) context: BTreeMap<String, String>,
}

impl PluginIntegrationDiagnostic {
    pub(crate) fn readiness(
        integration_id: PluginIntegrationId,
        status: PluginIntegrationStatus,
        message: String,
        checked_at: String,
    ) -> Self {
        Self {
            integration_id,
            operation: "readiness-check",
            status,
            level: if status.configured() {
                PluginIntegrationDiagnosticLevel::Info
            } else {
                PluginIntegrationDiagnosticLevel::Warn
            },
            message,
            checked_at,
            context: BTreeMap::new(),
        }
    }
}
