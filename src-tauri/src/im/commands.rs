use super::adapters::dingtalk::DingTalkAdapter;
use super::adapters::dingtalk_raw::RawDingTalkStream;
use super::adapters::feishu::FeishuAdapter;
use super::adapters::feishu_raw::RawFeishuLongConnection;
use super::adapters::http::ReqwestHttpTransport;
use super::adapters::telegram::{TelegramAdapter, TelegramCheckpoint};
use super::adapters::wechat::{WeChatAdapter, WeChatQrStatus, WeChatSessionStore};
use super::adapters::wecom::WeComAdapter;
use super::adapters::wecom_raw::RawWeComLongConnection;
use super::credentials::{credential_account, get_connector_credential, CredentialStore, OsCredentialStore};
use super::models::{
    builtin_descriptors, ConnectorConfig, ConnectorDescriptor, ConnectorHealth, ConnectorKind,
    ConnectorLifecycle, RoutingSettings,
};
use super::runtime::{ConnectorAdapter, ConnectorRuntimeError, ImRuntimeManager};
use super::storage::{clear_connector_credentials, save_connector_with_secret, ImRepository};
use crate::{
    active_log_dir_from_conn, inspect_project_inner, load_agent, logging, AppError,
    AvailabilityState, InteractionMode, RegistryStore,
};
use base64::Engine;
use chrono::{Duration as ChronoDuration, Utc};
use qrcode::render::svg;
use qrcode::QrCode;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::State;
use zeroize::Zeroizing;

const DEFAULT_PROFILE: &str = "default";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorView {
    pub descriptor: ConnectorDescriptor,
    pub config: ConnectorConfig,
    pub health: ConnectorHealth,
    pub has_credentials: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveConnectorInput {
    pub kind: ConnectorKind,
    pub enabled: bool,
    pub display_name: Option<String>,
    pub public_config: Value,
    pub credentials: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeChatAuthorizationView {
    pub status: String,
    pub image_data_url: Option<String>,
    pub expires_at: Option<String>,
    pub safe_error_code: Option<String>,
}

struct PendingWeChatAuthorization {
    payload: Zeroizing<String>,
    image_data_url: String,
    expires_at: chrono::DateTime<Utc>,
    status: String,
}

#[derive(Default)]
pub struct WeChatAuthorizationState {
    pending: Mutex<Option<PendingWeChatAuthorization>>,
}

#[tauri::command]
pub async fn list_im_connectors(
    store: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, Arc<ImRuntimeManager>>,
) -> Result<Vec<ConnectorView>, String> {
    let (configs, credential_presence) = {
        let store = store
            .lock()
            .map_err(|_| "storage-lock-failed".to_string())?;
        let conn = store.connection().map_err(safe_app_error)?;
        let repository = ImRepository::new(&conn);
        let credentials = OsCredentialStore;
        let mut configs = HashMap::new();
        let mut credential_presence = HashMap::new();
        for kind in ConnectorKind::ALL {
            let config = repository
                .connector(kind)
                .map_err(safe_app_error)?
                .unwrap_or_else(|| default_config(kind));
            let has_credentials = config.credential_ref.is_some()
                && get_connector_credential(&credentials, kind, DEFAULT_PROFILE)
                    .map_err(safe_app_error)?
                    .is_some();
            configs.insert(kind, config);
            credential_presence.insert(kind, has_credentials);
        }
        (configs, credential_presence)
    };
    let health = runtime
        .health()
        .await
        .into_iter()
        .map(|health| (health.kind, health))
        .collect::<HashMap<_, _>>();
    Ok(builtin_descriptors()
        .into_iter()
        .map(|descriptor| {
            let kind = descriptor.kind;
            let config = configs
                .get(&kind)
                .cloned()
                .unwrap_or_else(|| default_config(kind));
            let has_credentials = *credential_presence.get(&kind).unwrap_or(&false);
            let mut health = health.get(&kind).cloned().unwrap_or(ConnectorHealth {
                kind,
                lifecycle: if !has_credentials {
                    ConnectorLifecycle::Unconfigured
                } else if config.enabled {
                    ConnectorLifecycle::Error
                } else {
                    ConnectorLifecycle::Disabled
                },
                generation: 0,
                safe_error_code: config.enabled.then(|| "connector-not-started".to_string()),
                updated_at: Utc::now().to_rfc3339(),
            });
            if !has_credentials {
                health.lifecycle = ConnectorLifecycle::Unconfigured;
                health.safe_error_code = None;
            }
            ConnectorView {
                descriptor,
                config,
                health,
                has_credentials,
            }
        })
        .collect())
}

#[tauri::command]
pub fn get_im_routing(
    store: State<'_, Mutex<RegistryStore>>,
) -> Result<Option<RoutingSettings>, String> {
    let store = store
        .lock()
        .map_err(|_| "storage-lock-failed".to_string())?;
    ImRepository::new(&store.connection().map_err(safe_app_error)?)
        .routing()
        .map_err(safe_app_error)
}

#[tauri::command]
pub fn save_im_routing(
    store: State<'_, Mutex<RegistryStore>>,
    routing: RoutingSettings,
) -> Result<RoutingSettings, String> {
    let store = store
        .lock()
        .map_err(|_| "storage-lock-failed".to_string())?;
    let conn = store.connection().map_err(safe_app_error)?;
    let normalized = validate_routing(&conn, &routing)?;
    ImRepository::new(&conn)
        .save_routing(&normalized)
        .map_err(safe_app_error)?;
    Ok(normalized)
}

#[tauri::command]
pub async fn save_im_connector(
    store: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, Arc<ImRuntimeManager>>,
    input: SaveConnectorInput,
) -> Result<ConnectorConfig, String> {
    if input.enabled {
        let store = store
            .lock()
            .map_err(|_| "storage-lock-failed".to_string())?;
        let conn = store.connection().map_err(safe_app_error)?;
        validate_saved_routing(&conn)?;
    }
    runtime.stop(input.kind).await.map_err(safe_runtime_error)?;
    let (config, db_path) = {
        let store = store
            .lock()
            .map_err(|_| "storage-lock-failed".to_string())?;
        let conn = store.connection().map_err(safe_app_error)?;
        let account = credential_account(input.kind, DEFAULT_PROFILE);
        let secret = input
            .credentials
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| "credential-payload-invalid".to_string())?;
        let config = ConnectorConfig {
            kind: input.kind,
            enabled: input.enabled,
            display_name: input.display_name,
            public_config: input.public_config,
            credential_ref: Some(account),
        };
        save_connector_with_secret(&conn, &OsCredentialStore, config.clone(), secret.as_deref())
            .map_err(safe_app_error)?;
        (config, store.db_path.clone())
    };
    if config.enabled {
        register_and_start(&runtime, config.kind, db_path).await?;
    }
    Ok(config)
}

#[tauri::command]
pub async fn set_im_connector_enabled(
    store: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, Arc<ImRuntimeManager>>,
    kind: ConnectorKind,
    enabled: bool,
) -> Result<(), String> {
    if !enabled {
        runtime.stop(kind).await.map_err(safe_runtime_error)?;
    }
    let db_path = {
        let store = store
            .lock()
            .map_err(|_| "storage-lock-failed".to_string())?;
        let conn = store.connection().map_err(safe_app_error)?;
        let repository = ImRepository::new(&conn);
        let mut config = repository
            .connector(kind)
            .map_err(safe_app_error)?
            .unwrap_or_else(|| default_config(kind));
        if enabled && config.credential_ref.is_none() {
            return Err("connector-credentials-required".to_string());
        }
        if enabled {
            validate_saved_routing(&conn)?;
        }
        config.enabled = enabled;
        repository.save_connector(&config).map_err(safe_app_error)?;
        store.db_path.clone()
    };
    if enabled {
        register_and_start(&runtime, kind, db_path).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn restart_im_connector(
    store: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, Arc<ImRuntimeManager>>,
    kind: ConnectorKind,
) -> Result<(), String> {
    runtime.stop(kind).await.map_err(safe_runtime_error)?;
    let db_path = store
        .lock()
        .map_err(|_| "storage-lock-failed".to_string())?
        .db_path
        .clone();
    register_and_start(&runtime, kind, db_path).await
}

#[tauri::command]
pub async fn test_im_connector(
    store: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, Arc<ImRuntimeManager>>,
    kind: ConnectorKind,
) -> Result<(), String> {
    let (db_path, was_enabled) = {
        let store = store
            .lock()
            .map_err(|_| "storage-lock-failed".to_string())?;
        let conn = store.connection().map_err(safe_app_error)?;
        let enabled = ImRepository::new(&conn)
            .connector(kind)
            .map_err(safe_app_error)?
            .is_some_and(|config| config.enabled);
        (store.db_path.clone(), enabled)
    };
    runtime.stop(kind).await.map_err(safe_runtime_error)?;
    runtime.register(build_adapter(kind, db_path.clone())?).await;
    let test_result = runtime
        .test_connection(kind, Duration::from_secs(45))
        .await
        .map_err(safe_runtime_error);
    if was_enabled {
        let restart_result = register_and_start(&runtime, kind, db_path).await;
        if test_result.is_ok() {
            restart_result?;
        }
    }
    test_result
}

#[tauri::command]
pub async fn clear_im_connector(
    store: State<'_, Mutex<RegistryStore>>,
    runtime: State<'_, Arc<ImRuntimeManager>>,
    kind: ConnectorKind,
) -> Result<(), String> {
    runtime.stop(kind).await.map_err(safe_runtime_error)?;
    let store = store
        .lock()
        .map_err(|_| "storage-lock-failed".to_string())?;
    let conn = store.connection().map_err(safe_app_error)?;
    clear_connector_credentials(&conn, &OsCredentialStore, kind).map_err(safe_app_error)?;
    if kind == ConnectorKind::WeChat {
        let _ = OsCredentialStore.delete(&credential_account(kind, "session-contexts"));
    }
    let repository = ImRepository::new(&conn);
    if let Some(mut config) = repository.connector(kind).map_err(safe_app_error)? {
        config.enabled = false;
        repository.save_connector(&config).map_err(safe_app_error)?;
    }
    Ok(())
}

#[tauri::command]
pub fn reset_im_bindings(
    store: State<'_, Mutex<RegistryStore>>,
    kind: Option<ConnectorKind>,
) -> Result<(), String> {
    let store = store
        .lock()
        .map_err(|_| "storage-lock-failed".to_string())?;
    let conn = store.connection().map_err(safe_app_error)?;
    match kind {
        Some(kind) => conn
            .execute(
                "DELETE FROM im_session_bindings WHERE connector = ?1",
                [kind.as_str()],
            )
            .map_err(|_| "binding-reset-failed".to_string())?,
        None => conn
            .execute("DELETE FROM im_session_bindings", [])
            .map_err(|_| "binding-reset-failed".to_string())?,
    };
    Ok(())
}

#[tauri::command]
pub async fn begin_wechat_authorization(
    authorization: State<'_, WeChatAuthorizationState>,
) -> Result<WeChatAuthorizationView, String> {
    let transport =
        ReqwestHttpTransport::new(Duration::from_secs(45)).map_err(safe_runtime_error)?;
    let qr = WeChatAdapter::create_qr_code(&transport)
        .await
        .map_err(safe_runtime_error)?;
    let code = QrCode::new(qr.qr_url.as_bytes()).map_err(|_| "wechat-qr-invalid".to_string())?;
    let image = code.render::<svg::Color>().min_dimensions(256, 256).build();
    let image_data_url = format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(image.as_bytes())
    );
    let expires_at = Utc::now() + ChronoDuration::minutes(5);
    *authorization
        .pending
        .lock()
        .map_err(|_| "wechat-authorization-lock-failed".to_string())? =
        Some(PendingWeChatAuthorization {
            payload: Zeroizing::new(qr.qr_payload),
            image_data_url: image_data_url.clone(),
            expires_at,
            status: "waiting".to_string(),
        });
    Ok(WeChatAuthorizationView {
        status: "waiting".to_string(),
        image_data_url: Some(image_data_url),
        expires_at: Some(expires_at.to_rfc3339()),
        safe_error_code: None,
    })
}

#[tauri::command]
pub async fn poll_wechat_authorization(
    store: State<'_, Mutex<RegistryStore>>,
    authorization: State<'_, WeChatAuthorizationState>,
) -> Result<WeChatAuthorizationView, String> {
    let (payload, image_data_url, expires_at) = {
        let pending = authorization
            .pending
            .lock()
            .map_err(|_| "wechat-authorization-lock-failed".to_string())?;
        let pending = pending
            .as_ref()
            .ok_or_else(|| "wechat-authorization-not-started".to_string())?;
        (
            pending.payload.to_string(),
            pending.image_data_url.clone(),
            pending.expires_at,
        )
    };
    if Utc::now() >= expires_at {
        cancel_wechat_authorization(authorization)?;
        return Ok(WeChatAuthorizationView {
            status: "expired".to_string(),
            image_data_url: None,
            expires_at: None,
            safe_error_code: None,
        });
    }
    let transport =
        ReqwestHttpTransport::new(Duration::from_secs(45)).map_err(safe_runtime_error)?;
    let status = WeChatAdapter::poll_qr_status(&transport, &payload)
        .await
        .map_err(safe_runtime_error)?;
    let mut view = WeChatAuthorizationView {
        status: "waiting".to_string(),
        image_data_url: Some(image_data_url),
        expires_at: Some(expires_at.to_rfc3339()),
        safe_error_code: None,
    };
    match status {
        WeChatQrStatus::Waiting => {}
        WeChatQrStatus::Scanned => view.status = "scanned".to_string(),
        WeChatQrStatus::Expired => {
            view.status = "expired".to_string();
            view.image_data_url = None;
        }
        WeChatQrStatus::Error(code) => {
            view.status = "error".to_string();
            view.safe_error_code = Some(code);
        }
        WeChatQrStatus::Confirmed(credentials) => {
            let store = store
                .lock()
                .map_err(|_| "storage-lock-failed".to_string())?;
            let conn = store.connection().map_err(safe_app_error)?;
            let repository = ImRepository::new(&conn);
            let config = repository
                .connector(ConnectorKind::WeChat)
                .map_err(safe_app_error)?
                .unwrap_or_else(|| default_config(ConnectorKind::WeChat));
            let secret = serde_json::to_string(&json!({
                "botToken": credentials.bot_token,
                "baseUrl": credentials.base_url,
                "botId": credentials.bot_id
            }))
                .map_err(|_| "credential-payload-invalid".to_string())?;
            save_connector_with_secret(&conn, &OsCredentialStore, config, Some(&secret))
                .map_err(safe_app_error)?;
            view.status = "confirmed".to_string();
            view.image_data_url = None;
        }
    }
    if matches!(view.status.as_str(), "confirmed" | "expired" | "error") {
        cancel_wechat_authorization(authorization)?;
    } else if let Ok(mut pending) = authorization.pending.lock() {
        if let Some(pending) = pending.as_mut() {
            pending.status = view.status.clone();
        }
    }
    Ok(view)
}

#[tauri::command]
pub fn cancel_wechat_authorization(
    authorization: State<'_, WeChatAuthorizationState>,
) -> Result<(), String> {
    *authorization
        .pending
        .lock()
        .map_err(|_| "wechat-authorization-lock-failed".to_string())? = None;
    Ok(())
}

pub async fn start_saved_connectors(runtime: Arc<ImRuntimeManager>, db_path: PathBuf) {
    let enabled = {
        let Ok(conn) = Connection::open(&db_path) else {
            return;
        };
        let repository = ImRepository::new(&conn);
        ConnectorKind::ALL
            .into_iter()
            .filter(|kind| {
                repository
                    .connector(*kind)
                    .ok()
                    .flatten()
                    .is_some_and(|config| config.enabled)
            })
            .collect::<Vec<_>>()
    };
    for kind in enabled {
        let _ = register_and_start(&runtime, kind, db_path.clone()).await;
    }
}

fn record_connector_diagnostic(
    db_path: &std::path::Path,
    kind: ConnectorKind,
    operation: &str,
    safe_code: &str,
) {
    let Ok(conn) = Connection::open(db_path) else {
        return;
    };
    let Ok(log_dir) = active_log_dir_from_conn(&conn) else {
        return;
    };
    let mut context = BTreeMap::new();
    context.insert("connector".to_string(), kind.as_str().to_string());
    context.insert("operation".to_string(), operation.to_string());
    context.insert("safeCode".to_string(), safe_code.to_string());
    context.insert("retryCount".to_string(), "0".to_string());
    let _ = logging::write_message(
        &log_dir,
        logging::LogLevel::Error,
        "im.connector",
        "IM connector operation failed",
        context,
    );
}

async fn register_and_start(
    runtime: &Arc<ImRuntimeManager>,
    kind: ConnectorKind,
    db_path: PathBuf,
) -> Result<(), String> {
    let result = register_and_start_inner(runtime, kind, db_path.clone()).await;
    if let Err(safe_code) = &result {
        record_connector_diagnostic(&db_path, kind, "start", safe_code);
    }
    result
}

async fn register_and_start_inner(
    runtime: &Arc<ImRuntimeManager>,
    kind: ConnectorKind,
    db_path: PathBuf,
) -> Result<(), String> {
    let conn = Connection::open(&db_path).map_err(|_| "storage-open-failed".to_string())?;
    validate_saved_routing(&conn)?;
    runtime.register(build_adapter(kind, db_path)?).await;
    runtime.start(kind).await.map_err(safe_runtime_error)
}

fn validate_saved_routing(conn: &Connection) -> Result<RoutingSettings, String> {
    let routing = ImRepository::new(conn)
        .routing()
        .map_err(safe_app_error)?
        .ok_or_else(|| "im-routing-required".to_string())?;
    validate_routing(conn, &routing)
}

fn validate_routing(conn: &Connection, routing: &RoutingSettings) -> Result<RoutingSettings, String> {
    let agent = load_agent(conn, routing.agent_id.trim()).map_err(safe_app_error)?;
    if !matches!(agent.availability_state, AvailabilityState::Available) {
        return Err("default-agent-unavailable".to_string());
    }
    if !agent
        .supported_interaction_modes
        .iter()
        .any(|mode| matches!(mode, InteractionMode::Cli))
    {
        return Err("default-agent-no-cli".to_string());
    }
    let inspection = inspect_project_inner(routing.project_path.trim()).map_err(safe_app_error)?;
    Ok(RoutingSettings {
        agent_id: agent.id,
        project_path: inspection.path,
    })
}

fn build_adapter(
    kind: ConnectorKind,
    db_path: PathBuf,
) -> Result<Arc<dyn ConnectorAdapter>, String> {
    let secret = get_connector_credential(&OsCredentialStore, kind, DEFAULT_PROFILE)
        .map_err(safe_app_error)?
        .ok_or_else(|| "connector-credentials-required".to_string())?;
    let fields: BTreeMap<String, String> = serde_json::from_str(secret.as_str())
        .map_err(|_| "credential-payload-invalid".to_string())?;
    let http: Arc<dyn super::adapters::http::HttpTransport> =
        Arc::new(ReqwestHttpTransport::new(Duration::from_secs(40)).map_err(safe_runtime_error)?);
    match kind {
        ConnectorKind::Telegram => Ok(Arc::new(
            TelegramAdapter::new(
                required(&fields, "botToken")?,
                http,
                Arc::new(DbTelegramCheckpoint { db_path }),
            )
            .map_err(safe_runtime_error)?,
        )),
        ConnectorKind::Feishu => Ok(Arc::new(
            FeishuAdapter::new(
                required(&fields, "appId")?,
                required(&fields, "appSecret")?,
                http,
                Arc::new(RawFeishuLongConnection::default()),
            )
            .map_err(safe_runtime_error)?,
        )),
        ConnectorKind::DingTalk => Ok(Arc::new(
            DingTalkAdapter::new(
                required(&fields, "appKey")?,
                required(&fields, "appSecret")?,
                fields.get("robotCode").map(String::as_str),
                http,
                Arc::new(RawDingTalkStream::default()),
            )
            .map_err(safe_runtime_error)?,
        )),
        ConnectorKind::WeCom => Ok(Arc::new(
            WeComAdapter::new(
                required(&fields, "botId")?,
                required(&fields, "secret")?,
                Arc::new(RawWeComLongConnection::default()),
            )
            .map_err(safe_runtime_error)?,
        )),
        ConnectorKind::WeChat => Ok(Arc::new(
            WeChatAdapter::new_with_base_url(
                required(&fields, "botToken")?,
                fields.get("baseUrl").map(String::as_str).unwrap_or("https://ilinkai.weixin.qq.com"),
                http,
                Arc::new(DbWeChatSession::new(db_path)),
            )
            .map_err(safe_runtime_error)?,
        )),
    }
}

fn required<'a>(fields: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    fields
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("credential-field-missing-{key}"))
}

fn default_config(kind: ConnectorKind) -> ConnectorConfig {
    ConnectorConfig {
        kind,
        enabled: false,
        display_name: None,
        public_config: json!({}),
        credential_ref: None,
    }
}

fn safe_app_error(error: AppError) -> String {
    match error {
        AppError::Validation(message) => message,
        AppError::AgentNotFound(_) => "default-agent-not-found".to_string(),
        _ => "native-im-operation-failed".to_string(),
    }
}

fn safe_runtime_error(error: ConnectorRuntimeError) -> String {
    error.safe_code
}

struct DbTelegramCheckpoint {
    db_path: PathBuf,
}

impl TelegramCheckpoint for DbTelegramCheckpoint {
    fn load_offset(&self) -> Result<i64, ConnectorRuntimeError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-open-failed"))?;
        let value = ImRepository::new(&conn)
            .checkpoint(ConnectorKind::Telegram, "offset")
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-read-failed"))?;
        Ok(value.and_then(|value| value.parse().ok()).unwrap_or(0))
    }

    fn save_offset(&self, offset: i64) -> Result<(), ConnectorRuntimeError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-open-failed"))?;
        ImRepository::new(&conn)
            .save_checkpoint(ConnectorKind::Telegram, "offset", &offset.to_string())
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-write-failed"))
    }
}

struct DbWeChatSession {
    db_path: PathBuf,
    context_lock: Mutex<()>,
}

impl DbWeChatSession {
    fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            context_lock: Mutex::new(()),
        }
    }

    fn context_account() -> String {
        credential_account(ConnectorKind::WeChat, "session-contexts")
    }
}

impl WeChatSessionStore for DbWeChatSession {
    fn load_cursor(&self) -> Result<String, ConnectorRuntimeError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-open-failed"))?;
        Ok(ImRepository::new(&conn)
            .checkpoint(ConnectorKind::WeChat, "cursor")
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-read-failed"))?
            .unwrap_or_default())
    }

    fn save_cursor(&self, cursor: &str) -> Result<(), ConnectorRuntimeError> {
        let conn = Connection::open(&self.db_path)
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-open-failed"))?;
        ImRepository::new(&conn)
            .save_checkpoint(ConnectorKind::WeChat, "cursor", cursor)
            .map_err(|_| ConnectorRuntimeError::new("checkpoint-write-failed"))
    }

    fn load_context(&self, chat_id: &str) -> Result<Option<String>, ConnectorRuntimeError> {
        let _guard = self
            .context_lock
            .lock()
            .map_err(|_| ConnectorRuntimeError::new("context-lock-failed"))?;
        let values = load_wechat_contexts()?;
        Ok(values.get(&stable_hash(chat_id)).cloned())
    }

    fn save_context(&self, chat_id: &str, context: &str) -> Result<(), ConnectorRuntimeError> {
        let _guard = self
            .context_lock
            .lock()
            .map_err(|_| ConnectorRuntimeError::new("context-lock-failed"))?;
        let mut values = load_wechat_contexts()?;
        values.insert(stable_hash(chat_id), context.to_string());
        let serialized = serde_json::to_string(&values)
            .map_err(|_| ConnectorRuntimeError::new("context-serialize-failed"))?;
        OsCredentialStore
            .set(&Self::context_account(), &serialized)
            .map_err(|_| ConnectorRuntimeError::new("context-write-failed"))
    }
}

fn load_wechat_contexts() -> Result<BTreeMap<String, String>, ConnectorRuntimeError> {
    let value = OsCredentialStore
        .get(&DbWeChatSession::context_account())
        .map_err(|_| ConnectorRuntimeError::new("context-read-failed"))?;
    match value {
        Some(value) => serde_json::from_str(value.as_str())
            .map_err(|_| ConnectorRuntimeError::new("context-invalid")),
        None => Ok(BTreeMap::new()),
    }
}

fn stable_hash(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
