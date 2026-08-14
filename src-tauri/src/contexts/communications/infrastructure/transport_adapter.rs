use super::credential_adapter::{credential_account, SecureCredentialStore};
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
use super::transports::{ConnectorRuntimeError, SafeDiagnosticSink};
use crate::contexts::communications::application::{
    CommunicationsApplicationError, CommunicationsRepository, CommunicationsTransportPort,
    ConnectorRuntimeDefinition,
};
use crate::contexts::communications::domain::{
    connector_field_definitions, CheckpointKey, ConnectorCheckpoint, ConnectorFieldStorage,
    ConnectorHealth, ConnectorKind,
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
const WECHAT_CONTEXT_RETENTION_DAYS: i64 = 30;
const WECHAT_CONTEXT_MAINTENANCE_BATCH: usize = 128;

pub(crate) fn maintain_wechat_reply_contexts(
    repository: &SqliteCommunicationsRepository,
) -> Result<usize, CommunicationsApplicationError> {
    let cutoff = (Utc::now() - chrono::Duration::days(WECHAT_CONTEXT_RETENTION_DAYS)).to_rfc3339();
    let credentials = OsCredentialStore::new(CREDENTIAL_SERVICE_NAME);
    maintain_wechat_reply_contexts_with(repository, &cutoff, |account| {
        credentials.delete(account).map_err(|_| ())
    })
}

fn maintain_wechat_reply_contexts_with(
    repository: &SqliteCommunicationsRepository,
    cutoff: &str,
    mut delete_credential: impl FnMut(&str) -> Result<(), ()>,
) -> Result<usize, CommunicationsApplicationError> {
    let contexts =
        repository.expired_wechat_reply_contexts(cutoff, WECHAT_CONTEXT_MAINTENANCE_BATCH)?;
    let mut removed = 0;
    for (chat_hash, account, last_used_at) in contexts {
        if !repository.delete_expired_wechat_reply_context(&chat_hash, &account, cutoff)? {
            continue;
        }
        if delete_credential(&account).is_err() {
            repository.touch_wechat_reply_context(&chat_hash, &account, &last_used_at)?;
            return Err(CommunicationsApplicationError::failure(
                "wechat-context-retention-delete-failed",
            ));
        }
        removed += 1;
    }
    Ok(removed)
}

fn clear_wechat_reply_contexts(
    repository: &SqliteCommunicationsRepository,
) -> Result<usize, CommunicationsApplicationError> {
    let credentials = OsCredentialStore::new(CREDENTIAL_SERVICE_NAME);
    clear_wechat_reply_contexts_with(repository, |account| {
        credentials.delete(account).map_err(|_| ())
    })
}

fn clear_wechat_reply_contexts_with(
    repository: &SqliteCommunicationsRepository,
    mut delete_credential: impl FnMut(&str) -> Result<(), ()>,
) -> Result<usize, CommunicationsApplicationError> {
    let mut removed = 0;
    loop {
        let contexts = repository.wechat_reply_contexts(WECHAT_CONTEXT_MAINTENANCE_BATCH)?;
        if contexts.is_empty() {
            return Ok(removed);
        }
        for (chat_hash, account) in contexts {
            delete_credential(&account).map_err(|_| {
                CommunicationsApplicationError::failure("wechat-context-clear-delete-failed")
            })?;
            if repository.delete_wechat_reply_context(&chat_hash, &account)? {
                removed += 1;
            }
        }
    }
}

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

    fn diagnostic_sink(&self) -> SafeDiagnosticSink {
        let runtime = Arc::clone(&self.runtime);
        Arc::new(move |kind, safe_code| runtime.record_protocol_diagnostic(kind, safe_code))
    }

    fn build_adapter(
        &self,
        definition: &ConnectorRuntimeDefinition,
    ) -> Result<Arc<dyn ConnectorAdapter>, CommunicationsApplicationError> {
        let kind = definition.configuration.kind;
        let mut fields: BTreeMap<String, String> = serde_json::from_str(definition.secret.as_str())
            .map_err(|_| CommunicationsApplicationError::failure("credential-payload-invalid"))?;
        for field in connector_field_definitions(kind)
            .iter()
            .filter(|field| field.storage == ConnectorFieldStorage::Public)
        {
            if let Some(value) = definition
                .configuration
                .public_config
                .get(field.key)
                .and_then(serde_json::Value::as_str)
            {
                fields.insert(field.key.to_string(), value.to_string());
            }
        }
        let http: Arc<dyn HttpTransport> =
            Arc::new(ReqwestHttpTransport::new(HTTP_TIMEOUT).map_err(runtime_error)?);
        match kind {
            ConnectorKind::Telegram => Ok(Arc::new(
                TelegramAdapter::new(
                    required(&fields, "botToken")?,
                    http,
                    Arc::new(DbTelegramCheckpoint::new(self.repository.clone())),
                )
                .map_err(runtime_error)?
                .with_diagnostic_sink(self.diagnostic_sink()),
            )),
            ConnectorKind::Feishu => Ok(Arc::new(
                FeishuAdapter::new(
                    required(&fields, "appId")?,
                    required(&fields, "appSecret")?,
                    http,
                    Arc::new(RawFeishuLongConnection::default()),
                )
                .map_err(runtime_error)?
                .with_diagnostic_sink(self.diagnostic_sink()),
            )),
            ConnectorKind::DingTalk => Ok(Arc::new(
                DingTalkAdapter::new(
                    required(&fields, "appKey")?,
                    required(&fields, "appSecret")?,
                    fields.get("robotCode").map(String::as_str),
                    http,
                    Arc::new(RawDingTalkStream::default()),
                )
                .map_err(runtime_error)?
                .with_diagnostic_sink(self.diagnostic_sink()),
            )),
            ConnectorKind::WeCom => Ok(Arc::new(
                WeComAdapter::new(
                    required(&fields, "botId")?,
                    required(&fields, "secret")?,
                    Arc::new(RawWeComLongConnection::default()),
                )
                .map_err(runtime_error)?
                .with_diagnostic_sink(self.diagnostic_sink()),
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
                .map_err(runtime_error)?
                .with_diagnostic_sink(self.diagnostic_sink()),
            )),
        }
    }
}

#[async_trait]
impl CommunicationsTransportPort for CommunicationsTransportAdapter {
    async fn health(&self) -> Vec<ConnectorHealth> {
        self.runtime.health().await
    }

    async fn replace_and_start(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError> {
        let adapter = self.build_adapter(&definition)?;
        self.runtime
            .replace_and_start(adapter)
            .await
            .map_err(runtime_error)
    }

    async fn stop(&self, kind: ConnectorKind) -> Result<(), CommunicationsApplicationError> {
        match self.runtime.stop(kind).await {
            Err(error) if error.safe_code == "connector-not-registered" => Ok(()),
            result => result.map_err(runtime_error),
        }
    }

    async fn clear_connector_data(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        if kind == ConnectorKind::WeChat {
            clear_wechat_reply_contexts(&self.repository)?;
        }
        Ok(())
    }

    async fn test(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError> {
        let adapter = self.build_adapter(&definition)?;
        test_isolated_adapter(adapter, CONNECTION_TEST_TIMEOUT).await
    }

    async fn shutdown(&self) -> Result<(), CommunicationsApplicationError> {
        self.runtime.shutdown().await.map_err(runtime_error)
    }

    async fn send_notification(
        &self,
        kind: ConnectorKind,
        chat_id: &str,
        text: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        self.runtime
            .send_notification(kind, chat_id, text)
            .await
            .map_err(runtime_error)
    }
}

async fn test_isolated_adapter(
    adapter: Arc<dyn ConnectorAdapter>,
    timeout: Duration,
) -> Result<(), CommunicationsApplicationError> {
    tokio::time::timeout(timeout, adapter.test_connection())
        .await
        .map_err(|_| CommunicationsApplicationError::failure("connection-timeout"))?
        .map_err(runtime_error)
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
    credentials: Arc<dyn SecureCredentialStore>,
    context_lock: Mutex<()>,
}

impl DbWeChatSession {
    fn new(repository: SqliteCommunicationsRepository) -> Self {
        Self {
            repository,
            credentials: Arc::new(OsCredentialStore::new(CREDENTIAL_SERVICE_NAME)),
            context_lock: Mutex::new(()),
        }
    }

    #[cfg(test)]
    fn with_store(
        repository: SqliteCommunicationsRepository,
        credentials: Arc<dyn SecureCredentialStore>,
    ) -> Self {
        Self {
            repository,
            credentials,
            context_lock: Mutex::new(()),
        }
    }

    fn context_account() -> String {
        credential_account(ConnectorKind::WeChat, "session-contexts")
    }

    fn chat_context_account(chat_hash: &str) -> String {
        credential_account(
            ConnectorKind::WeChat,
            &format!("session-context-{chat_hash}"),
        )
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
        let chat_hash = stable_hash(chat_id);
        let account = Self::chat_context_account(&chat_hash);
        if let Some(value) = self
            .credentials
            .get(&account)
            .map_err(|_| ConnectorRuntimeError::new("context-read-failed"))?
        {
            self.repository
                .touch_wechat_reply_context(&chat_hash, &account, &Utc::now().to_rfc3339())
                .map_err(|_| ConnectorRuntimeError::new("context-metadata-write-failed"))?;
            return Ok(Some(value.to_string()));
        }
        let mut legacy = self.load_contexts()?;
        let Some(value) = legacy.remove(&chat_hash) else {
            return Ok(None);
        };
        self.credentials
            .set(&account, &value)
            .map_err(|_| ConnectorRuntimeError::new("context-write-failed"))?;
        self.repository
            .touch_wechat_reply_context(&chat_hash, &account, &Utc::now().to_rfc3339())
            .map_err(|_| ConnectorRuntimeError::new("context-metadata-write-failed"))?;
        if legacy.is_empty() {
            self.credentials
                .delete(&Self::context_account())
                .map_err(|_| ConnectorRuntimeError::new("context-write-failed"))?;
        } else {
            let serialized = serde_json::to_string(&legacy)
                .map_err(|_| ConnectorRuntimeError::new("context-serialize-failed"))?;
            self.credentials
                .set(&Self::context_account(), &serialized)
                .map_err(|_| ConnectorRuntimeError::new("context-write-failed"))?;
        }
        Ok(Some(value))
    }

    fn save_context(&self, chat_id: &str, context: &str) -> Result<(), ConnectorRuntimeError> {
        let _guard = self
            .context_lock
            .lock()
            .map_err(|_| ConnectorRuntimeError::new("context-lock-failed"))?;
        let chat_hash = stable_hash(chat_id);
        let account = Self::chat_context_account(&chat_hash);
        let previous = self
            .credentials
            .get(&account)
            .map_err(|_| ConnectorRuntimeError::new("context-read-failed"))?;
        self.credentials
            .set(&account, context)
            .map_err(|_| ConnectorRuntimeError::new("context-write-failed"))?;
        if self
            .repository
            .touch_wechat_reply_context(&chat_hash, &account, &Utc::now().to_rfc3339())
            .is_err()
        {
            match previous {
                Some(previous) => self.credentials.set(&account, previous.as_str()),
                None => self.credentials.delete(&account),
            }
            .map_err(|_| ConnectorRuntimeError::new("context-rollback-failed"))?;
            return Err(ConnectorRuntimeError::new("context-metadata-write-failed"));
        }
        Ok(())
    }
}

fn stable_hash(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::communications::infrastructure::runtime_manager::InboundDelivery;
    use crate::platform::database::NativeDatabase;
    use crate::platform::error::InfrastructureError;
    use crate::test_support::TempDirectory;
    use async_trait::async_trait;
    use tokio::sync::{mpsc, oneshot, watch};

    #[derive(Default)]
    struct MemoryContextStore(Mutex<BTreeMap<String, String>>);

    impl SecureCredentialStore for MemoryContextStore {
        fn set(&self, account: &str, secret: &str) -> Result<(), InfrastructureError> {
            self.0
                .lock()
                .map_err(|_| InfrastructureError::Credential("context-store-lock".to_string()))?
                .insert(account.to_string(), secret.to_string());
            Ok(())
        }

        fn get(
            &self,
            account: &str,
        ) -> Result<Option<zeroize::Zeroizing<String>>, InfrastructureError> {
            Ok(self
                .0
                .lock()
                .map_err(|_| InfrastructureError::Credential("context-store-lock".to_string()))?
                .get(account)
                .cloned()
                .map(zeroize::Zeroizing::new))
        }

        fn delete(&self, account: &str) -> Result<(), InfrastructureError> {
            self.0
                .lock()
                .map_err(|_| InfrastructureError::Credential("context-store-lock".to_string()))?
                .remove(account);
            Ok(())
        }
    }

    struct SlowTestAdapter;

    #[async_trait]
    impl ConnectorAdapter for SlowTestAdapter {
        fn kind(&self) -> ConnectorKind {
            ConnectorKind::Telegram
        }

        fn max_outbound_chars(&self) -> usize {
            4_096
        }

        async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
            std::future::pending().await
        }

        async fn run(
            &self,
            _inbound: mpsc::Sender<InboundDelivery>,
            _shutdown: watch::Receiver<bool>,
            _ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            panic!("isolated connection tests must not start an inbound runtime")
        }

        async fn send_text(
            &self,
            _outbound: crate::contexts::communications::domain::OutboundText,
        ) -> Result<(), ConnectorRuntimeError> {
            panic!("isolated connection tests must not send messages")
        }
    }

    #[tokio::test]
    async fn connection_test_times_out_ephemeral_adapter_without_runtime_mutation() {
        let error = test_isolated_adapter(Arc::new(SlowTestAdapter), Duration::from_millis(1))
            .await
            .expect_err("timeout");
        assert_eq!(error.safe_code(), "connection-timeout");
    }

    #[test]
    fn wechat_context_retention_restores_metadata_when_secure_delete_fails() {
        let directory = TempDirectory::new("wechat-context-retention-rollback");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        database.connection().expect("migrations");
        let repository = SqliteCommunicationsRepository::new(database.clone());
        repository
            .touch_wechat_reply_context("chat-hash", "secure-account", "2026-01-01T00:00:00Z")
            .expect("metadata");

        let error =
            maintain_wechat_reply_contexts_with(&repository, "2026-02-01T00:00:00Z", |_| Err(()))
                .expect_err("secure delete failure");
        assert_eq!(error.safe_code(), "wechat-context-retention-delete-failed");
        assert_eq!(
            repository
                .expired_wechat_reply_contexts("2026-02-01T00:00:00Z", 10)
                .expect("restored metadata")
                .len(),
            1
        );
    }

    #[test]
    fn clearing_wechat_contexts_removes_every_scoped_secret_and_metadata() {
        let directory = TempDirectory::new("wechat-context-clear");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        database.connection().expect("migrations");
        let repository = SqliteCommunicationsRepository::new(database);
        let store = Arc::new(MemoryContextStore::default());
        let context_count = WECHAT_CONTEXT_MAINTENANCE_BATCH + 1;

        for index in 0..context_count {
            let chat_hash = format!("chat-hash-{index:03}");
            let account = DbWeChatSession::chat_context_account(&chat_hash);
            store.set(&account, "private-context").expect("context");
            repository
                .touch_wechat_reply_context(&chat_hash, &account, "2026-08-01T00:00:00Z")
                .expect("metadata");
        }

        let removed = clear_wechat_reply_contexts_with(&repository, |account| {
            store.delete(account).map_err(|_| ())
        })
        .expect("clear contexts");

        assert_eq!(removed, context_count);
        assert!(repository
            .wechat_reply_contexts(1)
            .expect("remaining metadata")
            .is_empty());
        assert!(store.0.lock().expect("store").is_empty());
    }

    #[test]
    fn failed_wechat_context_clear_keeps_remaining_metadata_for_retry() {
        let directory = TempDirectory::new("wechat-context-clear-retry");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        database.connection().expect("migrations");
        let repository = SqliteCommunicationsRepository::new(database);
        let store = Arc::new(MemoryContextStore::default());
        for index in 0..2 {
            let chat_hash = format!("chat-hash-{index}");
            let account = DbWeChatSession::chat_context_account(&chat_hash);
            store.set(&account, "private-context").expect("context");
            repository
                .touch_wechat_reply_context(&chat_hash, &account, "2026-08-01T00:00:00Z")
                .expect("metadata");
        }

        let mut deletes = 0;
        let error = clear_wechat_reply_contexts_with(&repository, |account| {
            deletes += 1;
            if deletes == 2 {
                return Err(());
            }
            store.delete(account).map_err(|_| ())
        })
        .expect_err("second secure delete fails");

        assert_eq!(error.safe_code(), "wechat-context-clear-delete-failed");
        assert_eq!(
            repository
                .wechat_reply_contexts(10)
                .expect("retry metadata")
                .len(),
            1
        );
        assert_eq!(store.0.lock().expect("store").len(), 1);
        assert_eq!(
            clear_wechat_reply_contexts_with(&repository, |account| {
                store.delete(account).map_err(|_| ())
            })
            .expect("retry clear"),
            1
        );
        assert!(repository
            .wechat_reply_contexts(1)
            .expect("remaining metadata")
            .is_empty());
    }

    #[test]
    fn wechat_contexts_migrate_incrementally_per_chat_and_survive_restart() {
        let directory = TempDirectory::new("wechat-context-incremental-migration");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        database.connection().expect("migrations");
        let repository = SqliteCommunicationsRepository::new(database.clone());
        let store = Arc::new(MemoryContextStore::default());
        let first_hash = stable_hash("chat-first");
        let second_hash = stable_hash("chat-second");
        store
            .set(
                &DbWeChatSession::context_account(),
                &serde_json::json!({
                    first_hash.clone(): "context-first",
                    second_hash.clone(): "context-second"
                })
                .to_string(),
            )
            .expect("legacy contexts");

        let session = DbWeChatSession::with_store(repository.clone(), store.clone());
        assert_eq!(
            session.load_context("chat-first").expect("first migration"),
            Some("context-first".to_string())
        );
        let legacy = store
            .get(&DbWeChatSession::context_account())
            .expect("legacy read")
            .expect("remaining legacy");
        assert!(!legacy.contains("context-first"));
        assert!(legacy.contains("context-second"));

        let restarted = DbWeChatSession::with_store(repository, store.clone());
        assert_eq!(
            restarted.load_context("chat-first").expect("restart read"),
            Some("context-first".to_string())
        );
        assert_eq!(
            restarted
                .load_context("chat-second")
                .expect("second migration"),
            Some("context-second".to_string())
        );
        assert!(store
            .get(&DbWeChatSession::context_account())
            .expect("legacy removed")
            .is_none());
        assert!(store
            .get(&DbWeChatSession::chat_context_account(&first_hash))
            .expect("first scoped context")
            .is_some());
        assert!(store
            .get(&DbWeChatSession::chat_context_account(&second_hash))
            .expect("second scoped context")
            .is_some());
    }
}
