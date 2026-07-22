use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SshConnection {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) default_path: String,
    pub(crate) auth_mode: String,
    pub(crate) key_path: Option<String>,
    pub(crate) has_password: bool,
    pub(crate) test_status: String,
    pub(crate) last_connected_at: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveSshConnectionInput {
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) user: String,
    pub(crate) default_path: String,
    pub(crate) auth_mode: String,
    pub(crate) key_path: Option<String>,
    pub(crate) password: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SshConnectionTestResult {
    pub(crate) status: String,
    pub(crate) message: String,
    pub(crate) tested_at: String,
}
