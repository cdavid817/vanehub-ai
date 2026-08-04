use super::*;
use crate::contexts::communications::domain::{
    ChatBindingKey, CheckpointKey, ConnectorCheckpoint, ConnectorConfig, ConnectorHealth,
    ConnectorKind, InboundEventIdentity, NormalizedInbound, RoutingSettings,
};
use async_trait::async_trait;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

#[derive(Default)]
struct FakeRepository {
    configurations: Mutex<HashMap<ConnectorKind, ConnectorConfig>>,
    routing: Mutex<Option<RoutingSettings>>,
    events: Mutex<HashSet<(ConnectorKind, String)>>,
    cleanup_cutoffs: Mutex<Vec<String>>,
    cleanup_limits: Mutex<Vec<usize>>,
    checkpoints: Mutex<HashMap<(ConnectorKind, String), String>>,
    fail_configuration_save: AtomicBool,
}

impl CommunicationsRepository for FakeRepository {
    fn list_configurations(&self) -> Result<Vec<ConnectorConfig>, CommunicationsApplicationError> {
        Ok(self
            .configurations
            .lock()
            .expect("configurations")
            .values()
            .cloned()
            .collect())
    }

    fn find_configuration(
        &self,
        kind: ConnectorKind,
    ) -> Result<Option<ConnectorConfig>, CommunicationsApplicationError> {
        Ok(self
            .configurations
            .lock()
            .expect("configurations")
            .get(&kind)
            .cloned())
    }

    fn save_configuration(
        &self,
        configuration: &ConnectorConfig,
        _updated_at: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        if self.fail_configuration_save.load(Ordering::Acquire) {
            return Err(CommunicationsApplicationError::failure(
                "configuration-save-failed",
            ));
        }
        self.configurations
            .lock()
            .expect("configurations")
            .insert(configuration.kind, configuration.clone());
        Ok(())
    }

    fn delete_configuration(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        self.configurations
            .lock()
            .expect("configurations")
            .remove(&kind);
        Ok(())
    }

    fn load_routing(&self) -> Result<Option<RoutingSettings>, CommunicationsApplicationError> {
        Ok(self.routing.lock().expect("routing").clone())
    }

    fn save_routing(
        &self,
        routing: &RoutingSettings,
        _updated_at: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        *self.routing.lock().expect("routing") = Some(routing.clone());
        Ok(())
    }

    fn claim_event(
        &self,
        event: &InboundEventIdentity,
        _received_at: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        Ok(self
            .events
            .lock()
            .expect("events")
            .insert((event.connector(), event.event_id().to_string())))
    }

    fn cleanup_dedup_before(
        &self,
        cutoff: &str,
        limit: usize,
    ) -> Result<usize, CommunicationsApplicationError> {
        self.cleanup_cutoffs
            .lock()
            .expect("cleanup")
            .push(cutoff.to_string());
        self.cleanup_limits
            .lock()
            .expect("cleanup limits")
            .push(limit);
        Ok(0)
    }

    fn load_checkpoint(
        &self,
        key: &CheckpointKey,
    ) -> Result<Option<String>, CommunicationsApplicationError> {
        Ok(self
            .checkpoints
            .lock()
            .expect("checkpoints")
            .get(&(key.connector(), key.name().to_string()))
            .cloned())
    }

    fn save_checkpoint(
        &self,
        checkpoint: &ConnectorCheckpoint,
        _updated_at: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        self.checkpoints.lock().expect("checkpoints").insert(
            (
                checkpoint.key().connector(),
                checkpoint.key().name().to_string(),
            ),
            checkpoint.value().to_string(),
        );
        Ok(())
    }
}

#[derive(Default)]
struct FakeCredentials {
    values: Mutex<HashMap<ConnectorKind, ConnectorCredential>>,
    fail_store: AtomicBool,
}

impl CommunicationsCredentialPort for FakeCredentials {
    fn load(
        &self,
        kind: ConnectorKind,
    ) -> Result<Option<ConnectorCredential>, CommunicationsApplicationError> {
        Ok(self.values.lock().expect("credentials").get(&kind).cloned())
    }

    fn store(
        &self,
        kind: ConnectorKind,
        secret: &str,
    ) -> Result<ConnectorCredential, CommunicationsApplicationError> {
        if self.fail_store.load(Ordering::Acquire) {
            return Err(CommunicationsApplicationError::failure(
                "communications-credential-write-failed",
            ));
        }
        let credential = ConnectorCredential {
            reference: format!("im/{}/default", kind.as_str()),
            secret: Zeroizing::new(secret.to_string()),
        };
        self.values
            .lock()
            .expect("credentials")
            .insert(kind, credential.clone());
        Ok(credential)
    }

    fn delete(&self, kind: ConnectorKind) -> Result<(), CommunicationsApplicationError> {
        self.values.lock().expect("credentials").remove(&kind);
        Ok(())
    }
}

#[derive(Default)]
struct FakeTransports {
    health: Mutex<Vec<ConnectorHealth>>,
    actions: Mutex<Vec<String>>,
    fail_test: AtomicBool,
    fail_replace_starts: AtomicUsize,
    fail_stops: AtomicUsize,
}

fn consume_failure(counter: &AtomicUsize) -> bool {
    counter
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            current.checked_sub(1)
        })
        .is_ok()
}

#[async_trait]
impl CommunicationsTransportPort for FakeTransports {
    async fn health(&self) -> Vec<ConnectorHealth> {
        self.health.lock().expect("health").clone()
    }

    async fn replace_and_start(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError> {
        assert!(!definition.secret.is_empty());
        self.actions.lock().expect("actions").push(format!(
            "replace-start:{}",
            definition.configuration.kind.as_str()
        ));
        if consume_failure(&self.fail_replace_starts) {
            Err(CommunicationsApplicationError::failure(
                "replacement-start-failed",
            ))
        } else {
            Ok(())
        }
    }

    async fn stop(&self, kind: ConnectorKind) -> Result<(), CommunicationsApplicationError> {
        self.actions
            .lock()
            .expect("actions")
            .push(format!("stop:{}", kind.as_str()));
        if consume_failure(&self.fail_stops) {
            Err(CommunicationsApplicationError::failure("stop-failed"))
        } else {
            Ok(())
        }
    }

    async fn clear_connector_data(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        self.actions
            .lock()
            .expect("actions")
            .push(format!("clear-data:{}", kind.as_str()));
        Ok(())
    }

    async fn test(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError> {
        assert!(!definition.secret.is_empty());
        self.actions
            .lock()
            .expect("actions")
            .push(format!("test:{}", definition.configuration.kind.as_str()));
        if self.fail_test.load(Ordering::Acquire) {
            Err(CommunicationsApplicationError::failure("telegram-http-503"))
        } else {
            Ok(())
        }
    }

    async fn shutdown(&self) -> Result<(), CommunicationsApplicationError> {
        self.actions
            .lock()
            .expect("actions")
            .push("shutdown".to_string());
        Ok(())
    }
}

#[derive(Default)]
struct FakeAgents {
    validations: Mutex<Vec<RoutingSettings>>,
    executions: Mutex<Vec<(String, String)>>,
}

impl CommunicationsAgentExecutionPort for FakeAgents {
    fn validate_routing(
        &self,
        routing: &RoutingSettings,
    ) -> Result<RoutingSettings, CommunicationsApplicationError> {
        let routing = routing.normalized()?;
        self.validations
            .lock()
            .expect("validations")
            .push(routing.clone());
        Ok(routing)
    }

    fn execute(
        &self,
        request: AgentExecutionRequest,
    ) -> Result<AgentExecutionResult, CommunicationsApplicationError> {
        self.executions
            .lock()
            .expect("executions")
            .push((request.session_id, request.text));
        Ok(AgentExecutionResult {
            reply: "final reply".to_string(),
            message_id: "message-1".to_string(),
        })
    }
}

#[derive(Default)]
struct FakeSessions {
    bindings: Mutex<HashMap<(ConnectorKind, String), String>>,
    resolutions: Mutex<Vec<(ConnectorKind, String, String)>>,
    resets: Mutex<Vec<Option<ConnectorKind>>>,
}

impl CommunicationsSessionBindingPort for FakeSessions {
    fn find(&self, key: &ChatBindingKey) -> Result<Option<String>, CommunicationsApplicationError> {
        Ok(self
            .bindings
            .lock()
            .expect("bindings")
            .get(&(key.connector(), key.external_chat_id().to_string()))
            .cloned())
    }

    fn create_if_missing(
        &self,
        key: &ChatBindingKey,
        routing: &RoutingSettings,
    ) -> Result<String, CommunicationsApplicationError> {
        if let Some(session_id) = self.find(key)? {
            return Ok(session_id);
        }
        self.resolutions.lock().expect("resolutions").push((
            key.connector(),
            key.external_chat_id().to_string(),
            routing.project_path.clone(),
        ));
        let session_id = "session-1".to_string();
        self.bindings.lock().expect("bindings").insert(
            (key.connector(), key.external_chat_id().to_string()),
            session_id.clone(),
        );
        Ok(session_id)
    }

    fn reset(&self, kind: Option<ConnectorKind>) -> Result<(), CommunicationsApplicationError> {
        self.resets.lock().expect("resets").push(kind);
        Ok(())
    }
}

#[derive(Default)]
struct FakeOperations {
    events: Mutex<Vec<String>>,
}

impl CommunicationsOperationPort for FakeOperations {
    fn start(
        &self,
        kind: ConnectorKind,
        action: &'static str,
    ) -> Result<CommunicationsOperation, CommunicationsApplicationError> {
        self.events
            .lock()
            .expect("operations")
            .push(format!("start:{}:{action}", kind.as_str()));
        Ok(CommunicationsOperation {
            id: format!("operation-{}-{action}", kind.as_str()),
        })
    }

    fn complete(&self, operation_id: &str) -> Result<(), CommunicationsApplicationError> {
        self.events
            .lock()
            .expect("operations")
            .push(format!("complete:{operation_id}"));
        Ok(())
    }

    fn fail(
        &self,
        operation_id: &str,
        safe_code: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        self.events
            .lock()
            .expect("operations")
            .push(format!("fail:{operation_id}:{safe_code}"));
        Ok(())
    }
}

struct FakeClock;

impl CommunicationsClockPort for FakeClock {
    fn now_rfc3339(&self) -> String {
        "2026-07-18T10:00:00Z".to_string()
    }

    fn days_ago_rfc3339(&self, days: u32) -> String {
        format!("cutoff-{days}")
    }
}

#[derive(Default)]
struct FakeLogging {
    entries: Mutex<Vec<CommunicationsLog>>,
}

impl CommunicationsLoggingPort for FakeLogging {
    fn record(&self, log: CommunicationsLog) -> Result<(), CommunicationsApplicationError> {
        self.entries.lock().expect("logs").push(log);
        Ok(())
    }
}

struct Fixture {
    service: CommunicationsApplicationService,
    repository: Arc<FakeRepository>,
    credentials: Arc<FakeCredentials>,
    transports: Arc<FakeTransports>,
    agents: Arc<FakeAgents>,
    sessions: Arc<FakeSessions>,
    operations: Arc<FakeOperations>,
    logging: Arc<FakeLogging>,
}

fn fixture() -> Fixture {
    let repository = Arc::new(FakeRepository::default());
    *repository.routing.lock().expect("routing") =
        Some(RoutingSettings::new("codex-cli", "C:/repo").expect("routing"));
    let credentials = Arc::new(FakeCredentials::default());
    let transports = Arc::new(FakeTransports::default());
    let agents = Arc::new(FakeAgents::default());
    let sessions = Arc::new(FakeSessions::default());
    let operations = Arc::new(FakeOperations::default());
    let logging = Arc::new(FakeLogging::default());
    let service = CommunicationsApplicationService::new(CommunicationsApplicationPorts {
        repository: repository.clone(),
        credentials: credentials.clone(),
        transports: transports.clone(),
        agents: agents.clone(),
        sessions: sessions.clone(),
        operations: operations.clone(),
        clock: Arc::new(FakeClock),
        logging: logging.clone(),
    });
    Fixture {
        service,
        repository,
        credentials,
        transports,
        agents,
        sessions,
        operations,
        logging,
    }
}

fn request(secret: Option<&str>, enabled: bool) -> SaveConnectorRequest {
    SaveConnectorRequest {
        kind: ConnectorKind::Telegram,
        enabled,
        display_name: Some("Support bot".to_string()),
        public_config: json!({"apiBase": "https://api.telegram.org"}),
        credential_patch: secret.map(|secret| {
            std::collections::BTreeMap::from([("botToken".to_string(), secret.to_string())])
        }),
    }
}

fn inbound(direct: bool) -> NormalizedInbound {
    NormalizedInbound {
        connector: ConnectorKind::Telegram,
        event_id: "event-1".to_string(),
        chat_id: "chat-1".to_string(),
        sender_id: "sender-1".to_string(),
        text: "status please".to_string(),
        direct,
        reply_context: None,
    }
}

#[tokio::test]
async fn management_validates_then_persists_credentials_configuration_and_runtime() {
    let fixture = fixture();
    let configuration = fixture
        .service
        .save_connector(request(Some("private-token"), true))
        .await
        .expect("save");

    assert!(configuration.enabled);
    assert_eq!(
        configuration.credential_ref.as_deref(),
        Some("im/telegram/default")
    );
    assert_eq!(
        fixture
            .credentials
            .values
            .lock()
            .expect("credentials")
            .get(&ConnectorKind::Telegram)
            .expect("credential")
            .secret
            .as_str(),
        r#"{"botToken":"private-token"}"#
    );
    assert_eq!(
        fixture
            .transports
            .actions
            .lock()
            .expect("actions")
            .as_slice(),
        ["replace-start:telegram"]
    );
    assert_eq!(fixture.agents.validations.lock().expect("agents").len(), 1);
    let logs = fixture.logging.entries.lock().expect("logs");
    assert_eq!(logs.len(), 1);
    assert!(!format!("{logs:?}").contains("private-token"));
}

#[tokio::test]
async fn clear_connector_stops_runtime_then_purges_connector_owned_data() {
    let fixture = fixture();
    fixture
        .repository
        .configurations
        .lock()
        .expect("configurations")
        .insert(
            ConnectorKind::WeChat,
            ConnectorConfig {
                kind: ConnectorKind::WeChat,
                enabled: true,
                display_name: Some("Personal WeChat".to_string()),
                public_config: json!({}),
                credential_ref: Some("im/weixin/default".to_string()),
            },
        );
    fixture
        .credentials
        .store(ConnectorKind::WeChat, "previous-token")
        .expect("seed credential");

    fixture
        .service
        .clear_connector(ConnectorKind::WeChat)
        .await
        .expect("clear connector");

    assert!(fixture
        .credentials
        .load(ConnectorKind::WeChat)
        .expect("load credential")
        .is_none());
    let configuration = fixture
        .repository
        .find_configuration(ConnectorKind::WeChat)
        .expect("configuration")
        .expect("stored configuration");
    assert!(!configuration.enabled);
    assert!(configuration.credential_ref.is_none());
    assert_eq!(
        fixture
            .transports
            .actions
            .lock()
            .expect("actions")
            .as_slice(),
        ["stop:weixin", "clear-data:weixin"]
    );
}

#[tokio::test]
async fn clear_connector_restores_the_previous_runtime_when_persistence_fails() {
    let fixture = fixture();
    let previous = ConnectorConfig {
        kind: ConnectorKind::WeChat,
        enabled: true,
        display_name: Some("Personal WeChat".to_string()),
        public_config: json!({}),
        credential_ref: Some("im/weixin/default".to_string()),
    };
    fixture
        .repository
        .configurations
        .lock()
        .expect("configurations")
        .insert(ConnectorKind::WeChat, previous.clone());
    fixture
        .credentials
        .store(ConnectorKind::WeChat, "previous-token")
        .expect("seed credential");
    fixture
        .repository
        .fail_configuration_save
        .store(true, Ordering::Release);

    let error = fixture
        .service
        .clear_connector(ConnectorKind::WeChat)
        .await
        .expect_err("configuration save fails");

    assert_eq!(error.safe_code(), "configuration-save-failed");
    assert_eq!(
        fixture
            .repository
            .find_configuration(ConnectorKind::WeChat)
            .expect("configuration"),
        Some(previous)
    );
    assert_eq!(
        fixture
            .credentials
            .load(ConnectorKind::WeChat)
            .expect("credential")
            .expect("restored credential")
            .secret
            .as_str(),
        "previous-token"
    );
    assert_eq!(
        fixture
            .transports
            .actions
            .lock()
            .expect("actions")
            .as_slice(),
        ["stop:weixin", "replace-start:weixin"]
    );
}

#[tokio::test]
async fn failed_configuration_save_restores_the_previous_secret_without_starting() {
    let fixture = fixture();
    fixture
        .credentials
        .store(ConnectorKind::Telegram, "previous-token")
        .expect("seed credential");
    fixture
        .repository
        .fail_configuration_save
        .store(true, Ordering::Release);

    let error = fixture
        .service
        .save_connector(request(Some("replacement-token"), false))
        .await
        .expect_err("save fails");
    assert_eq!(error.safe_code(), "configuration-save-failed");
    assert_eq!(
        fixture
            .credentials
            .load(ConnectorKind::Telegram)
            .expect("load")
            .expect("credential")
            .secret
            .as_str(),
        "previous-token"
    );
    assert!(fixture
        .transports
        .actions
        .lock()
        .expect("actions")
        .is_empty());
}

#[tokio::test]
async fn replacement_start_failure_restores_configuration_credential_and_previous_runtime() {
    let fixture = fixture();
    let previous = ConnectorConfig {
        kind: ConnectorKind::Telegram,
        enabled: true,
        display_name: Some("Previous bot".to_string()),
        public_config: json!({}),
        credential_ref: Some("im/telegram/default".to_string()),
    };
    fixture
        .repository
        .configurations
        .lock()
        .expect("configurations")
        .insert(ConnectorKind::Telegram, previous.clone());
    fixture
        .credentials
        .store(ConnectorKind::Telegram, "previous-token")
        .expect("seed credential");
    fixture
        .transports
        .fail_replace_starts
        .store(1, Ordering::Release);

    let error = fixture
        .service
        .save_connector(request(Some("replacement-token"), true))
        .await
        .expect_err("replacement start fails");

    assert_eq!(error.safe_code(), "replacement-start-failed");
    assert_eq!(
        fixture
            .repository
            .find_configuration(ConnectorKind::Telegram)
            .expect("configuration"),
        Some(previous)
    );
    assert_eq!(
        fixture
            .credentials
            .load(ConnectorKind::Telegram)
            .expect("credential")
            .expect("stored credential")
            .secret
            .as_str(),
        "previous-token"
    );
    assert_eq!(
        fixture
            .transports
            .actions
            .lock()
            .expect("actions")
            .as_slice(),
        ["replace-start:telegram", "replace-start:telegram"]
    );
    let logs = fixture.logging.entries.lock().expect("logs");
    assert!(logs.iter().any(|log| {
        log.event == "communications.connector.lifecycle-primary"
            && log.safe_code.as_deref() == Some("replacement-start-failed")
    }));
    assert!(logs.iter().any(|log| {
        log.event == "communications.connector.lifecycle-rollback" && log.safe_code.is_none()
    }));
}

#[tokio::test]
async fn rollback_failure_records_distinct_redacted_primary_and_compensation_outcomes() {
    let fixture = fixture();
    fixture
        .repository
        .configurations
        .lock()
        .expect("configurations")
        .insert(
            ConnectorKind::Telegram,
            ConnectorConfig {
                kind: ConnectorKind::Telegram,
                enabled: true,
                display_name: None,
                public_config: json!({}),
                credential_ref: Some("im/telegram/default".to_string()),
            },
        );
    fixture
        .credentials
        .store(ConnectorKind::Telegram, "previous-token")
        .expect("seed credential");
    fixture
        .transports
        .fail_replace_starts
        .store(2, Ordering::Release);

    fixture
        .service
        .save_connector(request(Some("replacement-token"), true))
        .await
        .expect_err("primary failure");

    let logs = fixture.logging.entries.lock().expect("logs");
    assert!(logs.iter().any(|log| {
        log.event == "communications.connector.lifecycle-primary"
            && log.safe_code.as_deref() == Some("replacement-start-failed")
    }));
    assert!(logs.iter().any(|log| {
        log.event == "communications.connector.lifecycle-rollback"
            && log.safe_code.as_deref() == Some("replacement-start-failed")
    }));
    assert!(!format!("{logs:?}").contains("replacement-token"));
}

#[tokio::test]
async fn runtime_failures_finish_operations_and_emit_only_safe_diagnostics() {
    let fixture = fixture();
    fixture
        .repository
        .save_configuration(
            &ConnectorConfig {
                kind: ConnectorKind::Telegram,
                enabled: false,
                display_name: None,
                public_config: json!({}),
                credential_ref: Some("im/telegram/default".to_string()),
            },
            "2026-01-02T03:04:05Z",
        )
        .expect("configuration");
    fixture
        .credentials
        .store(ConnectorKind::Telegram, "private-token")
        .expect("credential");
    fixture.transports.fail_test.store(true, Ordering::Release);

    let error = fixture
        .service
        .test_connector(ConnectorKind::Telegram)
        .await
        .expect_err("test failure");
    assert_eq!(error.safe_code(), "telegram-http-503");
    assert_eq!(
        fixture
            .transports
            .actions
            .lock()
            .expect("actions")
            .as_slice(),
        ["test:telegram"]
    );
    let operations = fixture.operations.events.lock().expect("operations");
    assert_eq!(operations.len(), 2);
    assert!(operations[0].starts_with("start:telegram:test"));
    assert!(operations[1].contains("telegram-http-503"));
    let logs = fixture.logging.entries.lock().expect("logs");
    assert_eq!(logs[0].safe_code.as_deref(), Some("telegram-http-503"));
    assert!(!format!("{logs:?}").contains("private-token"));
}

#[tokio::test]
async fn saved_connector_startup_reports_each_connector_without_first_failure_abort() {
    let fixture = fixture();
    for kind in [ConnectorKind::Telegram, ConnectorKind::Feishu] {
        fixture
            .repository
            .configurations
            .lock()
            .expect("configurations")
            .insert(
                kind,
                ConnectorConfig {
                    kind,
                    enabled: true,
                    display_name: None,
                    public_config: json!({}),
                    credential_ref: None,
                },
            );
    }
    fixture
        .credentials
        .store(ConnectorKind::Feishu, "feishu-secret")
        .expect("feishu credential");

    let results = fixture
        .service
        .start_saved_connectors()
        .await
        .expect("startup outcomes");

    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|result| {
        result.kind == ConnectorKind::Telegram
            && result.safe_error_code.as_deref() == Some("connector-credentials-required")
    }));
    assert!(results.iter().any(|result| {
        result.kind == ConnectorKind::Feishu && result.safe_error_code.is_none()
    }));
    assert_eq!(
        fixture
            .transports
            .actions
            .lock()
            .expect("actions")
            .as_slice(),
        ["replace-start:feishu"]
    );
}

#[test]
fn router_uses_dedup_routing_binding_and_agent_ports() {
    let fixture = fixture();
    assert!(fixture
        .service
        .claim_inbound(ConnectorKind::Telegram, "event-1")
        .expect("first claim"));
    assert!(!fixture
        .service
        .claim_inbound(ConnectorKind::Telegram, "event-1")
        .expect("duplicate"));
    assert_eq!(
        fixture.service.maintain_deduplication().expect("cleanup"),
        0
    );
    assert_eq!(
        fixture.service.maintain_deduplication().expect("throttled"),
        0
    );
    assert_eq!(
        fixture
            .repository
            .cleanup_cutoffs
            .lock()
            .expect("cleanup")
            .as_slice(),
        ["cutoff-7"]
    );
    assert_eq!(
        fixture
            .repository
            .cleanup_limits
            .lock()
            .expect("cleanup limits")
            .as_slice(),
        [super::service::DEDUP_MAINTENANCE_BATCH]
    );

    assert_eq!(
        fixture.service.route_inbound(inbound(true)).expect("route"),
        InboundRouteOutcome::Reply {
            text: "final reply".to_string(),
            session_id: "session-1".to_string(),
            message_id: "message-1".to_string(),
        }
    );
    assert_eq!(
        fixture
            .service
            .route_inbound(inbound(false))
            .expect("ignore"),
        InboundRouteOutcome::Ignored
    );
    assert_eq!(
        fixture.sessions.resolutions.lock().expect("sessions").len(),
        1
    );
    assert_eq!(fixture.agents.executions.lock().expect("agents").len(), 1);

    fixture
        .service
        .reset_bindings(Some(ConnectorKind::Telegram))
        .expect("reset");
    assert_eq!(
        fixture.sessions.resets.lock().expect("resets").as_slice(),
        [Some(ConnectorKind::Telegram)]
    );
}

#[tokio::test]
async fn partial_credential_patch_preserves_omitted_fields_and_splits_legacy_payload() {
    let fixture = fixture();
    fixture
        .credentials
        .store(
            ConnectorKind::Feishu,
            r#"{"appId":"legacy-app","appSecret":"legacy-secret"}"#,
        )
        .expect("seed legacy credential");

    let configuration = fixture
        .service
        .save_connector(SaveConnectorRequest {
            kind: ConnectorKind::Feishu,
            enabled: false,
            display_name: None,
            public_config: json!({}),
            credential_patch: Some(std::collections::BTreeMap::from([(
                "appSecret".to_string(),
                "replacement-secret".to_string(),
            )])),
        })
        .await
        .expect("patch credential");

    assert_eq!(configuration.public_config["appId"], "legacy-app");
    assert_eq!(
        fixture
            .credentials
            .load(ConnectorKind::Feishu)
            .expect("load")
            .expect("credential")
            .secret
            .as_str(),
        r#"{"appSecret":"replacement-secret"}"#
    );
}

#[tokio::test]
async fn incomplete_credential_patch_is_rejected_before_runtime_mutation() {
    let fixture = fixture();
    let error = fixture
        .service
        .save_connector(SaveConnectorRequest {
            kind: ConnectorKind::Feishu,
            enabled: false,
            display_name: None,
            public_config: json!({}),
            credential_patch: Some(std::collections::BTreeMap::from([(
                "appId".to_string(),
                "only-public-field".to_string(),
            )])),
        })
        .await
        .expect_err("incomplete patch");

    assert_eq!(error.safe_code(), "credential-field-missing-appSecret");
    assert!(fixture
        .transports
        .actions
        .lock()
        .expect("actions")
        .is_empty());
    assert!(fixture
        .credentials
        .values
        .lock()
        .expect("credentials")
        .is_empty());
}

#[tokio::test]
async fn credential_store_failure_does_not_fall_back_to_plaintext_configuration() {
    let fixture = fixture();
    fixture
        .credentials
        .fail_store
        .store(true, Ordering::Release);

    let error = fixture
        .service
        .save_connector(request(Some("private-token"), false))
        .await
        .expect_err("credential store failure");

    assert_eq!(error.safe_code(), "communications-credential-write-failed");
    assert!(fixture
        .repository
        .configurations
        .lock()
        .expect("configurations")
        .is_empty());
}

#[test]
fn existing_binding_ignores_changed_routing_defaults() {
    let fixture = fixture();
    fixture.sessions.bindings.lock().expect("bindings").insert(
        (ConnectorKind::Telegram, "chat-1".to_string()),
        "session-existing".to_string(),
    );
    *fixture.repository.routing.lock().expect("routing") =
        Some(RoutingSettings::new("opencode", "C:/new-default").expect("changed routing"));

    let outcome = fixture.service.route_inbound(inbound(true)).expect("route");

    assert_eq!(
        outcome,
        InboundRouteOutcome::Reply {
            text: "final reply".to_string(),
            session_id: "session-existing".to_string(),
            message_id: "message-1".to_string(),
        }
    );
    assert!(fixture
        .sessions
        .resolutions
        .lock()
        .expect("resolutions")
        .is_empty());
    assert!(fixture
        .agents
        .validations
        .lock()
        .expect("validations")
        .is_empty());
    assert_eq!(
        fixture
            .agents
            .executions
            .lock()
            .expect("executions")
            .as_slice(),
        [("session-existing".to_string(), "status please".to_string())]
    );
}

#[tokio::test]
async fn connector_listing_derives_unconfigured_status_without_exposing_credentials() {
    let fixture = fixture();
    let connectors = fixture.service.list_connectors().await.expect("connectors");
    assert_eq!(connectors.len(), ConnectorKind::ALL.len());
    assert!(connectors
        .iter()
        .all(|connector| !connector.has_credentials));
    assert!(connectors.iter().all(|connector| connector.health.lifecycle
        == crate::contexts::communications::domain::ConnectorLifecycle::Unconfigured));
}
