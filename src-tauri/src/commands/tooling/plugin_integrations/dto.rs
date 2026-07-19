use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PluginIntegrationId {
    Github,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PluginIntegrationStatus {
    Configured,
    NotConfigured,
    MissingCli,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginIntegrationSetupStep {
    pub(crate) id: String,
    pub(crate) label_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginIntegrationDefinition {
    pub(crate) id: PluginIntegrationId,
    pub(crate) name_key: String,
    pub(crate) description_key: String,
    pub(crate) version: String,
    pub(crate) provider: String,
    pub(crate) icon: String,
    pub(crate) docs_url: String,
    pub(crate) setup_steps: Vec<PluginIntegrationSetupStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginIntegrationState {
    pub(crate) integration_id: PluginIntegrationId,
    pub(crate) status: PluginIntegrationStatus,
    pub(crate) configured: bool,
    pub(crate) can_test: bool,
    pub(crate) last_checked_at: Option<String>,
    pub(crate) status_reason_key: Option<String>,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginIntegrationEnvironment {
    pub(crate) runtime: String,
    pub(crate) native_checks_available: bool,
    pub(crate) reason_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginIntegrationOverview {
    pub(crate) definitions: Vec<PluginIntegrationDefinition>,
    pub(crate) states: Vec<PluginIntegrationState>,
    pub(crate) environment: PluginIntegrationEnvironment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginIntegrationRequest {
    pub(crate) integration_id: PluginIntegrationId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginIntegrationTestResult {
    pub(crate) integration_id: PluginIntegrationId,
    pub(crate) status: PluginIntegrationStatus,
    pub(crate) configured: bool,
    pub(crate) message: String,
    pub(crate) checked_at: String,
}
