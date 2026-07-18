use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginIntegrationId {
    Github,
}

impl PluginIntegrationId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PluginIntegrationStatus {
    Configured,
    NotConfigured,
    MissingCli,
    Unavailable,
    Error,
}

impl PluginIntegrationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::NotConfigured => "not-configured",
            Self::MissingCli => "missing-cli",
            Self::Unavailable => "unavailable",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginIntegrationSetupStep {
    pub id: String,
    pub label_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginIntegrationDefinition {
    pub id: PluginIntegrationId,
    pub name_key: String,
    pub description_key: String,
    pub version: String,
    pub provider: String,
    pub icon: String,
    pub docs_url: String,
    pub setup_steps: Vec<PluginIntegrationSetupStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginIntegrationState {
    pub integration_id: PluginIntegrationId,
    pub status: PluginIntegrationStatus,
    pub configured: bool,
    pub can_test: bool,
    pub last_checked_at: Option<String>,
    pub status_reason_key: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginIntegrationEnvironment {
    pub runtime: String,
    pub native_checks_available: bool,
    pub reason_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginIntegrationOverview {
    pub definitions: Vec<PluginIntegrationDefinition>,
    pub states: Vec<PluginIntegrationState>,
    pub environment: PluginIntegrationEnvironment,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginIntegrationRequest {
    pub integration_id: PluginIntegrationId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginIntegrationTestResult {
    pub integration_id: PluginIntegrationId,
    pub status: PluginIntegrationStatus,
    pub configured: bool,
    pub message: String,
    pub checked_at: String,
}
