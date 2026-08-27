use crate::contexts::communications::domain::{
    ConnectorConfig, ConnectorDescriptor, ConnectorHealth, ConnectorKind, SessionBinding,
    SessionConnectorAccess,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectorView {
    pub(crate) descriptor: ConnectorDescriptor,
    pub(crate) config: ConnectorConfig,
    pub(crate) health: ConnectorHealth,
    pub(crate) has_credentials: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveConnectorInput {
    pub(crate) kind: ConnectorKind,
    pub(crate) enabled: bool,
    pub(crate) display_name: Option<String>,
    pub(crate) public_config: Value,
    pub(crate) credentials: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeChatAuthorizationView {
    pub(crate) status: String,
    pub(crate) image_data_url: Option<String>,
    pub(crate) expires_at: Option<String>,
    pub(crate) safe_error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PairingStartView {
    pub(crate) connector: ConnectorKind,
    pub(crate) session_id: String,
    pub(crate) code: String,
    pub(crate) expires_at: String,
    pub(crate) replace_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionBindingView {
    pub(crate) binding: Option<SessionBinding>,
    pub(crate) pending_connector: Option<ConnectorKind>,
    pub(crate) access: SessionConnectorAccess,
}
