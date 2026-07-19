use super::credential_adapter::credential_account;
use super::runtime_manager::{ConnectorAdapter, ConnectorRuntimeManager};
use super::sqlite_repository::SqliteCommunicationsRepository;
use super::transports::dingtalk::DingTalkAdapter;
use super::transports::dingtalk_raw::RawDingTalkStream;
use super::transports::feishu::FeishuAdapter;
use super::transports::feishu_raw::RawFeishuLongConnection;
use super::transports::http::{HttpTransport, ReqwestHttpTransport};
use super::transports::telegram::{TelegramAdapter, TelegramCheckpoint};
use super::transports::wechat::{WeChatAdapter, WeChatSessionStore};
use super::transports::wecom::WeComAdapter;
use super::transports::wecom_raw::RawWeComLongConnection;
use super::transports::ConnectorRuntimeError;
use crate::contexts::communications::application::{
    CommunicationsApplicationError, CommunicationsRepository, CommunicationsTransportPort,
    ConnectorRuntimeDefinition,
};
use crate::contexts::communications::domain::{
    CheckpointKey, ConnectorCheckpoint, ConnectorHealth, ConnectorKind,
};
use crate::platform::credentials::OsCredentialStore;
use async_trait::async_trait;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const CONNECTION_TEST_TIMEOUT: Duration = Duration::from_secs(45);
const HTTP_TIMEOUT: Duration = Duration::from_secs(40);
const CREDENTIAL_SERVICE_NAME: &str = "io.vanehub.ai.im";

#[derive(Clone)]
pub(crate) struct CommunicationsTransportAdapter {
    runtime: Arc<ConnectorRuntimeManager>,
    repository: SqliteCommunicationsRepository,
}

impl CommunicationsTransportAdapter {
    pub(crate) fn new(
        runtime: Arc<ConnectorRuntimeManager>,
        repository: SqliteCommunicationsRepository,
    ) -> Self {
        Self {
            runtime,
            repository,
        }
    }

    fn build_adapter(
        &self,
        definition: &ConnectorRuntimeDefinition,
    ) -> Result<Arc<dyn ConnectorAdapter>, CommunicationsApplicationError> {
        let kind = definition.configuration.kind;
        let fields: BTreeMap<String, String> = serde_json::from_str(definition.secret.as_str())
            .map_err(|_| CommunicationsApplicationError::failure("credential-payload-invalid"))?;
        let http: Arc<dyn HttpTransport> =
            Arc::new(ReqwestHttpTransport::new(HTTP_TIMEOUT).map_err(runtime_error)?);
        match kind {
            ConnectorKind::Telegram => Ok(Arc::new(
                TelegramAdapter::new(
                    required(&fields, "botToken")?,
                    http,
                    Arc::new(DbTelegramCheckpoint::new(self.repository.clone())),
                )
                .map_err(runtime_error)?,
            )),
            ConnectorKind::Feishu => Ok(Arc::new(
                FeishuAdapter::new(
                    required(&fields, "appId")?,
                    required(&fields, "appSecret")?,
                    http,
                    Arc::new(RawFeishuLongConnection::default()),
                )
                .map_err(runtime_error)?,
            )),
            ConnectorKind::DingTalk => Ok(Arc::new(
                DingTalkAdapter::new(
                    required(&fields, "appKey")?,
                    required(&fields, "appSecret")?,
                    fields.get("robotCode").map(String::as_str),
                    http,
                    Arc::new(RawDingTalkStream::default()),
                )
                .map_err(runtime_error)?,
            )),
            ConnectorKind::WeCom => Ok(Arc::new(
                WeComAdapter::new(
                    required(&fields, "botId")?,
                    required(&fields, "secret")?,
                    Arc::new(RawWeComLongConnection::default()),
                )
                .map_err(runtime_error)?,
            )),
            ConnectorKind::WeChat => Ok(Arc::new(
                WeChatAdapter::new_with_base_url(
                    required(&fields, "botToken")?,
                    fields
                        .get("baseUrl")
                        .map(String::as_str)
                        .unwrap_or("https://ilinkai.weixin.qq.com"),
                    http,
                    Arc::new(DbWeChatSession::new(self.repository.clone())),
                )
                .map_err(runtime_error)?,
            )),
        }
    }
}

#[async_trait]
impl CommunicationsTransportPort for CommunicationsTransportAdapter {
    async fn health(&self) -> Vec<ConnectorHealth> {
        self.runtime.health().await
    }

    async fn start(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError> {
        let kind = definition.configuration.kind;
        let adapter = self.build_adapter(&definition)?;
        self.runtime.register(adapter).await;
        self.runtime.start(kind).await.map_err(runtime_error)
    }

    async fn stop(&self, kind: ConnectorKind) -> Result<(), CommunicationsApplicationError> {
        match self.runtime.stop(kind).await {
            Err(error) if error.safe_code == "connector-not-registered" => Ok(()),
            result => result.map_err(runtime_error),
        }
    }

    async fn test(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError> {
        let kind = definition.configuration.kind;
        let enabled = definition.configuration.enabled;
        self.stop(kind).await?;
        self.runtime
            .register(self.build_adapter(&definition)?)
            .await;
        let result = self
            .runtime
            .test_connection(kind, CONNECTION_TEST_TIMEOUT)
            .await
            .map_err(runtime_error);
        if enabled {
            let restart = self.runtime.start(kind).await.map_err(runtime_error);
            if result.is_ok() {
                restart?;
            }
        }
        result
    }

    async fn shutdown(&self) -> Result<(), CommunicationsApplicationError> {
        self.runtime.shutdown().await.map_err(runtime_error)
    }
}

fn required<'a>(
    fields: &'a BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, CommunicationsApplicationError> {
    fields
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            CommunicationsApplicationError::failure(format!("credential-field-missing-{key}"))
        })
}

fn runtime_error(error: ConnectorRuntimeError) -> CommunicationsApplicationError {
    match error.user_message {
        Some(message) => CommunicationsApplicationError::user_visible(error.safe_code, message),
        None => CommunicationsApplicationError::failure(error.safe_code),
    }
}

struct DbTelegramCheckpoint {
    repository: SqliteCommunicationsRepository,
}

impl DbTelegramCheckpoint {
    fn new(repository: SqliteCommunicationsRepository) -> Self {
        Self { repository }
    }
}

impl TelegramCheckpoint for DbTelegramCheckpoint {
    fn load_offset(&self) -> Result<i64, ConnectorRuntimeError> {
        let key = CheckpointKey::new(ConnectorKind::Telegram, "offset")
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-read-failed"))?;
        let value = self
            .repository
            .load_checkpoint(&key)
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-read-failed"))?;
        Ok(value.and_then(|value| value.parse().ok()).unwrap_or(0))
    }

    fn save_offset(&self, offset: i64) -> Result<(), ConnectorRuntimeError> {
        let key = CheckpointKey::new(ConnectorKind::Telegram, "offset")
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-write-failed"))?;
        self.repository
            .save_checkpoint(
                &ConnectorCheckpoint::new(key, offset.to_string()),
                &Utc::now().to_rfc3339(),
            )
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-write-failed"))
    }
}

struct DbWeChatSession {
    repository: SqliteCommunicationsRepository,
    credentials: OsCredentialStore,
    context_lock: Mutex<()>,
}

impl DbWeChatSession {
    fn new(repository: SqliteCommunicationsRepository) -> Self {
        Self {
            repository,
            credentials: OsCredentialStore::new(CREDENTIAL_SERVICE_NAME),
            context_lock: Mutex::new(()),
        }
    }

    fn context_account() -> String {
        credential_account(ConnectorKind::WeChat, "session-contexts")
    }

    fn load_contexts(&self) -> Result<BTreeMap<String, String>, ConnectorRuntimeError> {
        match self
            .credentials
            .get(&Self::context_account())
            .map_err(|_| ConnectorRuntimeError::new("context-read-failed"))?
        {
            Some(value) => serde_json::from_str(value.as_str())
                .map_err(|_| ConnectorRuntimeError::new("context-invalid")),
            None => Ok(BTreeMap::new()),
        }
    }
}

impl WeChatSessionStore for DbWeChatSession {
    fn load_cursor(&self) -> Result<String, ConnectorRuntimeError> {
        let key = CheckpointKey::new(ConnectorKind::WeChat, "cursor")
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-read-failed"))?;
        Ok(self
            .repository
            .load_checkpoint(&key)
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-read-failed"))?
            .unwrap_or_default())
    }

    fn save_cursor(&self, cursor: &str) -> Result<(), ConnectorRuntimeError> {
        let key = CheckpointKey::new(ConnectorKind::WeChat, "cursor")
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-write-failed"))?;
        self.repository
            .save_checkpoint(
                &ConnectorCheckpoint::new(key, cursor.to_string()),
                &Utc::now().to_rfc3339(),
            )
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-write-failed"))
    }

    fn load_context(&self, chat_id: &str) -> Result<Option<String>, ConnectorRuntimeError> {
        let _guard = self
            .context_lock
            .lock()
            .map_err(|_| ConnectorRuntimeError::new("context-lock-failed"))?;
        Ok(self.load_contexts()?.get(&stable_hash(chat_id)).cloned())
    }

    fn save_context(&self, chat_id: &str, context: &str) -> Result<(), ConnectorRuntimeError> {
        let _guard = self
            .context_lock
            .lock()
            .map_err(|_| ConnectorRuntimeError::new("context-lock-failed"))?;
        let mut values = self.load_contexts()?;
        values.insert(stable_hash(chat_id), context.to_string());
        let serialized = serde_json::to_string(&values)
            .map_err(|_| ConnectorRuntimeError::new("context-serialize-failed"))?;
        self.credentials
            .set(&Self::context_account(), &serialized)
            .map_err(|_| ConnectorRuntimeError::new("context-write-failed"))
    }
}

fn stable_hash(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
