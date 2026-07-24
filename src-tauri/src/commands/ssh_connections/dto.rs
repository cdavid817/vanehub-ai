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
    pub(crate) revision: i64,
    pub(crate) host_trust: Option<SshHostTrustMetadata>,
    pub(crate) test_status: String,
    pub(crate) last_connected_at: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SshHostTrustMetadata {
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) algorithm: String,
    pub(crate) fingerprint: String,
    pub(crate) confirmed_at: String,
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SshHostKeyChallenge {
    pub(crate) connection_id: String,
    pub(crate) revision: i64,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) kind: String,
    pub(crate) algorithm: String,
    pub(crate) fingerprint: String,
    pub(crate) previous_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfirmSshHostKeyInput {
    pub(crate) connection_id: String,
    pub(crate) revision: i64,
    pub(crate) fingerprint: String,
}
