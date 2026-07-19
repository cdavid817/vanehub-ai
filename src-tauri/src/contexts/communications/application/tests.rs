use super::*;
use crate::contexts::communications::domain::{
    ChatBindingKey, CheckpointKey, ConnectorCheckpoint, ConnectorConfig, ConnectorHealth,
    ConnectorKind, InboundEventIdentity, NormalizedInbound, RoutingSettings,
};
use async_trait::async_trait;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

#[derive(Default)]
struct FakeRepository {
    configurations: Mutex<HashMap<ConnectorKind, ConnectorConfig>>,
    routing: Mutex<Option<RoutingSettings>>,
    events: Mutex<HashSet<(ConnectorKind, String)>>,
    cleanup_cutoffs: Mutex<Vec<String>>,
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

    fn cleanup_dedup_before(&self, cutoff: &str) -> Result<usize, CommunicationsApplicationError> {
        self.cleanup_cutoffs
            .lock()
            .expect("cleanup")
            .push(cutoff.to_string());
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
}

#[async_trait]
impl CommunicationsTransportPort for FakeTransports {
    async fn health(&self) -> Vec<ConnectorHealth> {
        self.health.lock().expect("health").clone()
    }

    async fn start(
        &self,
        definition: ConnectorRuntimeDefinition,
    ) -> Result<(), CommunicationsApplicationError> {
        assert!(!definition.secret.is_empty());
        self.actions
            .lock()
            .expect("actions")
            .push(format!("start:{}", definition.configuration.kind.as_str()));
        Ok(())
    }

    async fn stop(&self, kind: ConnectorKind) -> Result<(), CommunicationsApplicationError> {
        self.actions
            .lock()
            .expect("actions")
            .push(format!("stop:{}", kind.as_str()));
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
        assert_eq!(request.routing.agent_id, "codex-cli");
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
    resolutions: Mutex<Vec<(ConnectorKind, String, String)>>,
    resets: Mutex<Vec<Option<ConnectorKind>>>,
}

impl CommunicationsSessionBindingPort for FakeSessions {
    fn resolve_or_create(
        &self,
        key: &ChatBindingKey,
        routing: &RoutingSettings,
    ) -> Result<String, CommunicationsApplicationError> {
        self.resolutions.lock().expect("resolutions").push((
            key.connector(),
            key.external_chat_id().to_string(),
            routing.project_path.clone(),
        ));
        Ok("session-1".to_string())
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
        replacement_secret: secret.map(str::to_string),
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
        "private-token"
    );
    assert_eq!(
        fixture
            .transports
            .actions
            .lock()
            .expect("actions")
            .as_slice(),
        ["stop:telegram", "start:telegram"]
    );
    assert_eq!(fixture.agents.validations.lock().expect("agents").len(), 1);
    let logs = fixture.logging.entries.lock().expect("logs");
    assert_eq!(logs.len(), 1);
    assert!(!format!("{logs:?}").contains("private-token"));
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
    assert_eq!(
        fixture
            .transports
            .actions
            .lock()
            .expect("actions")
            .as_slice(),
        ["stop:telegram"]
    );
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
    let operations = fixture.operations.events.lock().expect("operations");
    assert_eq!(operations.len(), 2);
    assert!(operations[0].starts_with("start:telegram:test"));
    assert!(operations[1].contains("telegram-http-503"));
    let logs = fixture.logging.entries.lock().expect("logs");
    assert_eq!(logs[0].safe_code.as_deref(), Some("telegram-http-503"));
    assert!(!format!("{logs:?}").contains("private-token"));
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
        fixture
            .repository
            .cleanup_cutoffs
            .lock()
            .expect("cleanup")
            .as_slice(),
        ["cutoff-7"]
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
