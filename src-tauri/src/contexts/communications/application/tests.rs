use super::*;
use crate::contexts::communications::domain::{
    BindingState, ChatBindingKey, CheckpointKey, ConnectorCheckpoint, ConnectorConfig,
    ConnectorHealth, ConnectorKind, ConnectorLifecycle, InboundEventIdentity, NormalizedInbound,
    PairingIntent, RoutingSettings, SessionBinding, SessionConnectorAccess,
};
use async_trait::async_trait;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration as StdDuration;
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
    pairing_intents: Mutex<Vec<PairingIntent>>,
    managed_bindings: Mutex<HashMap<(ConnectorKind, String), SessionBinding>>,
    notification_deliveries: Mutex<HashSet<(String, String, ConnectorKind)>>,
    delivery_references: Mutex<HashMap<String, String>>,
    session_access: Mutex<HashMap<(String, ConnectorKind), SessionConnectorAccess>>,
    fail_session_access: AtomicBool,
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

    fn binding_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionBinding>, CommunicationsApplicationError> {
        Ok(self
            .managed_bindings
            .lock()
            .expect("bindings")
            .values()
            .find(|binding| binding.session_id == session_id)
            .cloned())
    }

    fn binding_for_chat(
        &self,
        key: &ChatBindingKey,
    ) -> Result<Option<SessionBinding>, CommunicationsApplicationError> {
        Ok(self
            .managed_bindings
            .lock()
            .expect("bindings")
            .get(&(key.connector(), key.external_chat_id().to_string()))
            .cloned())
    }

    fn session_access(
        &self,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<SessionConnectorAccess, CommunicationsApplicationError> {
        if self.fail_session_access.load(Ordering::Acquire) {
            return Err(CommunicationsApplicationError::failure(
                "communications-repository-failed",
            ));
        }
        Ok(self
            .session_access
            .lock()
            .expect("session access")
            .get(&(session_id.to_string(), connector))
            .cloned()
            .unwrap_or_else(|| SessionConnectorAccess::disabled(session_id, connector)))
    }

    fn set_session_access(
        &self,
        session_id: &str,
        connector: ConnectorKind,
        enabled: bool,
        updated_at: &str,
    ) -> Result<SessionConnectorAccess, CommunicationsApplicationError> {
        let access = SessionConnectorAccess {
            session_id: session_id.to_string(),
            connector,
            enabled,
            updated_at: updated_at.to_string(),
        };
        self.session_access
            .lock()
            .expect("session access")
            .insert((session_id.to_string(), connector), access.clone());
        Ok(access)
    }

    fn save_pairing_intent(
        &self,
        intent: &PairingIntent,
    ) -> Result<(), CommunicationsApplicationError> {
        let mut intents = self.pairing_intents.lock().expect("pairings");
        intents.retain(|candidate| {
            candidate.session_id != intent.session_id || candidate.connector != intent.connector
        });
        intents.push(intent.clone());
        Ok(())
    }

    fn pairing_intents(
        &self,
        connector: ConnectorKind,
        now: &str,
    ) -> Result<Vec<PairingIntent>, CommunicationsApplicationError> {
        Ok(self
            .pairing_intents
            .lock()
            .expect("pairings")
            .iter()
            .filter(|intent| intent.connector == connector && intent.expires_at.as_str() > now)
            .cloned()
            .collect())
    }

    fn consume_pairing_intent(
        &self,
        intent_id: &str,
        key: &ChatBindingKey,
        now: &str,
        replace: bool,
        delivery_credential_ref: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        let mut intents = self.pairing_intents.lock().expect("pairings");
        let index = intents
            .iter()
            .position(|intent| {
                intent.id == intent_id
                    && intent.connector == key.connector()
                    && intent.expires_at.as_str() > now
            })
            .ok_or_else(|| CommunicationsApplicationError::failure("im-pairing-invalid"))?;
        let intent = intents[index].clone();
        let mut bindings = self.managed_bindings.lock().expect("bindings");
        let conflict = bindings.iter().any(|((kind, chat), binding)| {
            (*kind == key.connector()
                && chat == key.external_chat_id()
                && binding.session_id != intent.session_id)
                || (binding.session_id == intent.session_id
                    && chat != key.external_chat_id()
                    && binding.is_active())
        });
        if conflict && !replace {
            return Err(CommunicationsApplicationError::failure(
                "im-binding-replacement-required",
            ));
        }
        if replace {
            bindings.retain(|(kind, chat), binding| {
                binding.session_id != intent.session_id
                    && !(*kind == key.connector() && chat == key.external_chat_id())
            });
        }
        let binding = SessionBinding {
            connector: key.connector(),
            session_id: intent.session_id,
            state: BindingState::Active,
            completion_notifications: false,
            created_at: now.to_string(),
            updated_at: now.to_string(),
        };
        bindings.insert(
            (key.connector(), key.external_chat_id().to_string()),
            binding.clone(),
        );
        self.delivery_references
            .lock()
            .expect("delivery references")
            .insert(
                binding.session_id.clone(),
                delivery_credential_ref.to_string(),
            );
        intents.remove(index);
        Ok(binding)
    }

    fn binding_delivery_reference(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, CommunicationsApplicationError> {
        Ok(self
            .delivery_references
            .lock()
            .expect("delivery references")
            .get(session_id)
            .cloned())
    }

    fn replacement_delivery_references(
        &self,
        session_id: &str,
        _key: &ChatBindingKey,
    ) -> Result<Vec<String>, CommunicationsApplicationError> {
        Ok(self
            .delivery_references
            .lock()
            .expect("delivery references")
            .get(session_id)
            .cloned()
            .into_iter()
            .collect())
    }

    fn cancel_pairing(
        &self,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<bool, CommunicationsApplicationError> {
        let mut intents = self.pairing_intents.lock().expect("pairings");
        let before = intents.len();
        intents.retain(|intent| intent.session_id != session_id || intent.connector != connector);
        Ok(before != intents.len())
    }

    fn set_binding_state(
        &self,
        session_id: &str,
        state: BindingState,
        updated_at: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        let mut bindings = self.managed_bindings.lock().expect("bindings");
        let binding = bindings
            .values_mut()
            .find(|binding| binding.session_id == session_id)
            .ok_or_else(|| CommunicationsApplicationError::failure("im-binding-not-found"))?;
        binding.state = state;
        binding.updated_at = updated_at.to_string();
        Ok(binding.clone())
    }

    fn set_completion_notifications(
        &self,
        session_id: &str,
        enabled: bool,
        updated_at: &str,
    ) -> Result<SessionBinding, CommunicationsApplicationError> {
        let mut bindings = self.managed_bindings.lock().expect("bindings");
        let binding = bindings
            .values_mut()
            .find(|binding| binding.session_id == session_id)
            .ok_or_else(|| CommunicationsApplicationError::failure("im-binding-not-found"))?;
        binding.completion_notifications = enabled;
        binding.updated_at = updated_at.to_string();
        Ok(binding.clone())
    }

    fn remove_session_binding(
        &self,
        session_id: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        let mut bindings = self.managed_bindings.lock().expect("bindings");
        let before = bindings.len();
        bindings.retain(|_, binding| binding.session_id != session_id);
        self.delivery_references
            .lock()
            .expect("delivery references")
            .remove(session_id);
        Ok(before != bindings.len())
    }

    fn claim_notification_delivery(
        &self,
        message_id: &str,
        session_id: &str,
        connector: ConnectorKind,
        _delivered_at: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        Ok(self
            .notification_deliveries
            .lock()
            .expect("deliveries")
            .insert((message_id.to_string(), session_id.to_string(), connector)))
    }

    fn release_notification_delivery(
        &self,
        message_id: &str,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        self.notification_deliveries
            .lock()
            .expect("deliveries")
            .remove(&(message_id.to_string(), session_id.to_string(), connector));
        Ok(())
    }
}

#[derive(Default)]
struct FakeCredentials {
    values: Mutex<HashMap<ConnectorKind, ConnectorCredential>>,
    delivery_handles: Mutex<HashMap<String, String>>,
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

    fn store_delivery_handle(
        &self,
        kind: ConnectorKind,
        binding_id: &str,
        handle: &str,
    ) -> Result<String, CommunicationsApplicationError> {
        let reference = format!("binding/{}/{binding_id}", kind.as_str());
        self.delivery_handles
            .lock()
            .expect("delivery handles")
            .insert(reference.clone(), handle.to_string());
        Ok(reference)
    }

    fn load_delivery_handle(
        &self,
        reference: &str,
    ) -> Result<Option<zeroize::Zeroizing<String>>, CommunicationsApplicationError> {
        Ok(self
            .delivery_handles
            .lock()
            .expect("delivery handles")
            .get(reference)
            .cloned()
            .map(Zeroizing::new))
    }

    fn delete_delivery_handle(
        &self,
        reference: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        self.delivery_handles
            .lock()
            .expect("delivery handles")
            .remove(reference);
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

    async fn send_notification(
        &self,
        kind: ConnectorKind,
        _chat_id: &str,
        text: &str,
    ) -> Result<(), CommunicationsApplicationError> {
        self.actions
            .lock()
            .expect("actions")
            .push(format!("notify:{}:{text}", kind.as_str()));
        Ok(())
    }
}

#[derive(Default)]
struct FakeAgents {
    validations: Mutex<Vec<RoutingSettings>>,
    executions: Mutex<Vec<(ConnectorKind, String, String)>>,
    invalid_mentions: Mutex<Option<Vec<String>>>,
    execution_gate: Mutex<Option<Arc<ExecutionGate>>>,
}

struct ExecutionGate {
    started: Barrier,
    release: Barrier,
}

impl ExecutionGate {
    fn new() -> Self {
        Self {
            started: Barrier::new(2),
            release: Barrier::new(2),
        }
    }
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
    ) -> Result<AgentExecutionOutcome, CommunicationsApplicationError> {
        self.executions.lock().expect("executions").push((
            request.connector,
            request.session_id,
            request.text,
        ));
        if let Some(gate) = self.execution_gate.lock().expect("execution gate").clone() {
            gate.started.wait();
            gate.release.wait();
        }
        if let Some(valid_mentions) = self
            .invalid_mentions
            .lock()
            .expect("invalid mentions")
            .take()
        {
            return Ok(AgentExecutionOutcome::InvalidSeat { valid_mentions });
        }
        Ok(AgentExecutionOutcome::Reply(AgentExecutionResult {
            reply: "final reply".to_string(),
            message_id: "message-1".to_string(),
        }))
    }
}

#[derive(Default)]
struct FakeSessions {
    resolutions: Mutex<Vec<(ConnectorKind, String, String)>>,
    resets: Mutex<Vec<Option<ConnectorKind>>>,
    missing: AtomicBool,
}

impl CommunicationsSessionBindingPort for FakeSessions {
    fn reset(&self, kind: Option<ConnectorKind>) -> Result<(), CommunicationsApplicationError> {
        self.resets.lock().expect("resets").push(kind);
        Ok(())
    }

    fn exists(&self, _session_id: &str) -> Result<bool, CommunicationsApplicationError> {
        Ok(!self.missing.load(Ordering::Acquire))
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
    fixture_with_copy(CommunicationsCopy {
        unbound: "This chat is not connected. Start pairing from the session IM panel, then send /bind CODE here.",
        paused: "This IM connection is paused. Resume it from the session IM panel.",
        stale: "This connection is no longer available. Start a new pairing from an active session.",
        pairing_invalid: "The pairing code is invalid or expired.",
        pairing_established: "IM connection established.",
        completion: "The session task has completed.",
        invalid_seat: "The mentioned seat is unavailable. Valid seats:",
    })
}

fn fixture_with_copy(copy: CommunicationsCopy) -> Fixture {
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
        copy: Arc::new(move || copy),
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

fn simplified_chinese_copy() -> CommunicationsCopy {
    CommunicationsCopy {
        unbound: "此聊天尚未连接。请在会话 IM 面板中开始配对，然后在此发送 /bind 配对码。",
        paused: "此 IM 连接已暂停。请在会话 IM 面板中恢复。",
        stale: "此连接已不可用。请从有效会话重新开始配对。",
        pairing_invalid: "配对码无效或已过期。",
        pairing_established: "IM 连接已建立。",
        completion: "会话任务已完成。",
        invalid_seat: "提及的席位不可用。可用席位：",
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

fn inbound_text(text: &str) -> NormalizedInbound {
    NormalizedInbound {
        text: text.to_string(),
        ..inbound(true)
    }
}

fn feishu_inbound_text(text: &str) -> NormalizedInbound {
    NormalizedInbound {
        connector: ConnectorKind::Feishu,
        text: text.to_string(),
        ..inbound(true)
    }
}

fn session_binding(session_id: &str, state: BindingState) -> SessionBinding {
    SessionBinding {
        connector: ConnectorKind::Telegram,
        session_id: session_id.to_string(),
        state,
        completion_notifications: false,
        created_at: "2026-07-18T10:00:00Z".to_string(),
        updated_at: "2026-07-18T10:00:00Z".to_string(),
    }
}

fn install_feishu_binding(fixture: &Fixture, state: BindingState, notifications: bool) {
    fixture
        .repository
        .managed_bindings
        .lock()
        .expect("bindings")
        .insert(
            (ConnectorKind::Feishu, "chat-1".to_string()),
            SessionBinding {
                connector: ConnectorKind::Feishu,
                session_id: "session-1".to_string(),
                state,
                completion_notifications: notifications,
                created_at: "2026-07-18T10:00:00Z".to_string(),
                updated_at: "2026-07-18T10:00:00Z".to_string(),
            },
        );
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
    assert!(fixture
        .agents
        .validations
        .lock()
        .expect("agents")
        .is_empty());
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
    *fixture.repository.routing.lock().expect("routing") = None;
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

#[tokio::test]
async fn pairing_is_expiring_single_use_and_routes_only_after_binding() {
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
        .store(ConnectorKind::Telegram, "private-token")
        .expect("credential");
    fixture
        .transports
        .health
        .lock()
        .expect("health")
        .push(ConnectorHealth {
            kind: ConnectorKind::Telegram,
            lifecycle: crate::contexts::communications::domain::ConnectorLifecycle::Connected,
            generation: 1,
            safe_error_code: None,
            updated_at: "2026-07-18T10:00:00Z".to_string(),
        });

    let unbound = fixture
        .service
        .route_inbound(inbound(true))
        .expect("guidance");
    assert!(matches!(unbound, InboundRouteOutcome::SystemReply { .. }));
    assert!(fixture.agents.executions.lock().expect("agents").is_empty());
    assert!(fixture
        .sessions
        .resolutions
        .lock()
        .expect("sessions")
        .is_empty());

    let pairing = fixture
        .service
        .begin_pairing("session-1", ConnectorKind::Telegram, false)
        .await
        .expect("pairing");
    assert_eq!(pairing.code.len(), 8);
    assert_eq!(pairing.expires_at, "2026-07-18T10:10:00+00:00");
    assert!(
        !format!("{:?}", fixture.logging.entries.lock().expect("logs")).contains(&pairing.code)
    );

    assert_eq!(
        fixture
            .service
            .route_inbound(inbound_text(&format!("/bind {}", pairing.code)))
            .expect("bind"),
        InboundRouteOutcome::SystemReply {
            text: "IM connection established.".to_string(),
        }
    );
    assert!(matches!(
        fixture
            .service
            .route_inbound(inbound_text(&format!("/bind {}", pairing.code)))
            .expect("single use"),
        InboundRouteOutcome::SystemReply { text } if text.contains("invalid or expired")
    ));
    assert!(matches!(
        fixture.service.route_inbound(inbound(true)).expect("route"),
        InboundRouteOutcome::Reply { session_id, .. } if session_id == "session-1"
    ));
    fixture
        .service
        .set_completion_notifications("session-1", true)
        .expect("notifications");
    assert!(fixture
        .service
        .notify_session_completion("session-1", "desktop-message-1", false)
        .await
        .expect("notification"));
    assert!(!fixture
        .service
        .notify_session_completion("session-1", "desktop-message-1", false)
        .await
        .expect("idempotent"));
    assert!(!fixture
        .service
        .notify_session_completion("session-1", "im-message-1", true)
        .await
        .expect("loop prevention"));
    let logs = format!("{:?}", fixture.logging.entries.lock().expect("logs"));
    for forbidden in [
        pairing.code.as_str(),
        "chat-1",
        "status please",
        "final reply",
    ] {
        assert!(!logs.contains(forbidden));
    }
}

#[tokio::test]
async fn feishu_access_gates_pairing_routing_and_notifications() {
    let fixture = fixture();
    fixture
        .repository
        .configurations
        .lock()
        .expect("configurations")
        .insert(
            ConnectorKind::Feishu,
            ConnectorConfig {
                kind: ConnectorKind::Feishu,
                enabled: true,
                display_name: None,
                public_config: json!({}),
                credential_ref: Some("im/feishu/default".to_string()),
            },
        );
    fixture
        .credentials
        .store(ConnectorKind::Feishu, "private-token")
        .expect("credential");
    fixture
        .transports
        .health
        .lock()
        .expect("health")
        .push(ConnectorHealth {
            kind: ConnectorKind::Feishu,
            lifecycle: crate::contexts::communications::domain::ConnectorLifecycle::Connected,
            generation: 1,
            safe_error_code: None,
            updated_at: "2026-07-18T10:00:00Z".to_string(),
        });

    let disabled = fixture
        .service
        .begin_pairing("session-1", ConnectorKind::Feishu, false)
        .await
        .expect_err("disabled by default");
    assert_eq!(disabled.safe_code(), "im-session-disabled");

    let enabled = fixture
        .service
        .set_session_access("session-1", ConnectorKind::Feishu, true)
        .expect("enable access");
    assert!(enabled.enabled);
    let pairing = fixture
        .service
        .begin_pairing("session-1", ConnectorKind::Feishu, false)
        .await
        .expect("pairing");

    fixture
        .service
        .set_session_access("session-1", ConnectorKind::Feishu, false)
        .expect("disable before bind");
    let blocked_bind = fixture
        .service
        .route_inbound(feishu_inbound_text(&format!("/bind {}", pairing.code)))
        .expect_err("binding consumption must be gated");
    assert_eq!(blocked_bind.safe_code(), "im-session-disabled");

    fixture
        .service
        .set_session_access("session-1", ConnectorKind::Feishu, true)
        .expect("re-enable");
    assert!(matches!(
        fixture
            .service
            .route_inbound(feishu_inbound_text(&format!("/bind {}", pairing.code)))
            .expect("bind"),
        InboundRouteOutcome::SystemReply { .. }
    ));
    fixture
        .service
        .set_completion_notifications("session-1", true)
        .expect("notifications");

    fixture
        .service
        .set_session_access("session-1", ConnectorKind::Feishu, false)
        .expect("disable bound session");
    let blocked_route = fixture
        .service
        .route_inbound(feishu_inbound_text("status please"))
        .expect_err("routing must be gated");
    assert_eq!(blocked_route.safe_code(), "im-session-disabled");
    assert!(fixture.agents.executions.lock().expect("agents").is_empty());
    let blocked_notification = fixture
        .service
        .notify_session_completion("session-1", "message-disabled", false)
        .await
        .expect_err("notifications must be gated");
    assert_eq!(blocked_notification.safe_code(), "im-session-disabled");
    assert!(fixture
        .transports
        .actions
        .lock()
        .expect("actions")
        .iter()
        .all(|action| !action.starts_with("notify:")));

    fixture
        .service
        .set_session_access("session-1", ConnectorKind::Feishu, true)
        .expect("enable bound session");
    assert!(matches!(
        fixture
            .service
            .route_inbound(feishu_inbound_text("status please"))
            .expect("route"),
        InboundRouteOutcome::Reply { session_id, .. } if session_id == "session-1"
    ));
    assert!(fixture
        .service
        .notify_session_completion("session-1", "message-enabled", false)
        .await
        .expect("notification"));
    assert_eq!(
        fixture.agents.executions.lock().expect("agents").as_slice(),
        [(
            ConnectorKind::Feishu,
            "session-1".to_string(),
            "status please".to_string(),
        )]
    );
    *fixture
        .agents
        .invalid_mentions
        .lock()
        .expect("invalid mentions") = Some(vec!["架构师".to_string(), "实现者".to_string()]);
    assert_eq!(
        fixture
            .service
            .route_inbound(feishu_inbound_text("@已移除席位 继续"))
            .expect("safe invalid-seat response"),
        InboundRouteOutcome::SystemReply {
            text: "The mentioned seat is unavailable. Valid seats: @架构师, @实现者".to_string(),
        }
    );
}

#[test]
fn session_access_toggle_preserves_a_manual_binding_pause() {
    let fixture = fixture();
    install_feishu_binding(&fixture, BindingState::Active, false);
    fixture
        .service
        .set_session_access("session-1", ConnectorKind::Feishu, true)
        .expect("enable access");
    fixture
        .service
        .set_binding_paused("session-1", true)
        .expect("manual pause");

    fixture
        .service
        .set_session_access("session-1", ConnectorKind::Feishu, false)
        .expect("disable access");
    fixture
        .service
        .set_session_access("session-1", ConnectorKind::Feishu, true)
        .expect("re-enable access");

    let binding = fixture
        .repository
        .binding_for_session("session-1")
        .expect("binding lookup")
        .expect("binding");
    assert_eq!(binding.state, BindingState::Paused);
    assert!(matches!(
        fixture
            .service
            .route_inbound(feishu_inbound_text("status please"))
            .expect("paused guidance"),
        InboundRouteOutcome::SystemReply { text } if text.contains("paused")
    ));
    assert!(fixture.agents.executions.lock().expect("agents").is_empty());
}

#[tokio::test]
async fn feishu_pairing_requires_connected_transport_health() {
    let fixture = fixture();
    fixture
        .repository
        .configurations
        .lock()
        .expect("configurations")
        .insert(
            ConnectorKind::Feishu,
            ConnectorConfig {
                kind: ConnectorKind::Feishu,
                enabled: true,
                display_name: None,
                public_config: json!({}),
                credential_ref: Some("im/feishu/default".to_string()),
            },
        );
    fixture
        .credentials
        .store(ConnectorKind::Feishu, "private-token")
        .expect("credential");
    fixture
        .service
        .set_session_access("session-1", ConnectorKind::Feishu, true)
        .expect("enable access");

    let unavailable = fixture
        .service
        .begin_pairing("session-1", ConnectorKind::Feishu, false)
        .await
        .expect_err("disconnected transport");
    assert_eq!(unavailable.safe_code(), "im-connector-not-ready");
    assert!(fixture
        .repository
        .pairing_intents
        .lock()
        .expect("pairings")
        .is_empty());

    fixture
        .transports
        .health
        .lock()
        .expect("health")
        .push(ConnectorHealth {
            kind: ConnectorKind::Feishu,
            lifecycle: ConnectorLifecycle::Connected,
            generation: 1,
            safe_error_code: None,
            updated_at: "2026-07-18T10:00:00Z".to_string(),
        });
    assert!(fixture
        .service
        .begin_pairing("session-1", ConnectorKind::Feishu, false)
        .await
        .is_ok());
}

#[tokio::test]
async fn session_access_repository_failures_deny_pairing_routing_and_notifications() {
    let fixture = fixture();
    install_feishu_binding(&fixture, BindingState::Active, true);
    fixture
        .repository
        .session_access
        .lock()
        .expect("session access")
        .insert(
            ("session-1".to_string(), ConnectorKind::Feishu),
            SessionConnectorAccess {
                session_id: "session-1".to_string(),
                connector: ConnectorKind::Feishu,
                enabled: true,
                updated_at: "2026-07-18T10:00:00Z".to_string(),
            },
        );
    fixture
        .repository
        .fail_session_access
        .store(true, Ordering::Release);

    let pairing = fixture
        .service
        .begin_pairing("session-1", ConnectorKind::Feishu, false)
        .await
        .expect_err("repository failure must deny pairing");
    assert_eq!(pairing.safe_code(), "communications-repository-failed");
    let routing = fixture
        .service
        .route_inbound(feishu_inbound_text("status please"))
        .expect_err("repository failure must deny routing");
    assert_eq!(routing.safe_code(), "communications-repository-failed");
    let notification = fixture
        .service
        .notify_session_completion("session-1", "message-1", false)
        .await
        .expect_err("repository failure must suppress notification");
    assert_eq!(notification.safe_code(), "communications-repository-failed");
    assert!(fixture.agents.executions.lock().expect("agents").is_empty());
    assert!(fixture
        .transports
        .actions
        .lock()
        .expect("actions")
        .iter()
        .all(|action| !action.starts_with("notify:")));
}

#[test]
fn completed_disable_excludes_subsequent_inbound_during_a_concurrent_turn() {
    let fixture = fixture();
    install_feishu_binding(&fixture, BindingState::Active, false);
    fixture
        .service
        .set_session_access("session-1", ConnectorKind::Feishu, true)
        .expect("enable access");
    let gate = Arc::new(ExecutionGate::new());
    *fixture
        .agents
        .execution_gate
        .lock()
        .expect("execution gate") = Some(gate.clone());

    let inbound_service = fixture.service.clone();
    let inbound_thread = thread::spawn(move || {
        inbound_service.route_inbound(feishu_inbound_text("admitted before disable"))
    });
    gate.started.wait();

    let disable_service = fixture.service.clone();
    let (disabled_tx, disabled_rx) = mpsc::channel();
    let disable_thread = thread::spawn(move || {
        let result = disable_service.set_session_access("session-1", ConnectorKind::Feishu, false);
        disabled_tx.send(result).expect("disable result receiver");
    });
    assert!(disabled_rx
        .recv_timeout(StdDuration::from_millis(50))
        .is_err());

    gate.release.wait();
    assert!(matches!(
        inbound_thread
            .join()
            .expect("inbound thread")
            .expect("inbound"),
        InboundRouteOutcome::Reply { .. }
    ));
    assert!(
        !disabled_rx
            .recv_timeout(StdDuration::from_secs(1))
            .expect("disable completion")
            .expect("disable access")
            .enabled
    );
    disable_thread.join().expect("disable thread");
    *fixture
        .agents
        .execution_gate
        .lock()
        .expect("execution gate") = None;

    let blocked = fixture
        .service
        .route_inbound(feishu_inbound_text("arrived after disable"))
        .expect_err("completed disable must exclude subsequent inbound");
    assert_eq!(blocked.safe_code(), "im-session-disabled");
    assert_eq!(fixture.agents.executions.lock().expect("agents").len(), 1);
}

#[tokio::test]
async fn inbound_status_and_completion_messages_use_injected_locale_copy() {
    let fixture = fixture_with_copy(simplified_chinese_copy());
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
        .store(ConnectorKind::Telegram, "private-token")
        .expect("credential");
    fixture
        .transports
        .health
        .lock()
        .expect("health")
        .push(ConnectorHealth {
            kind: ConnectorKind::Telegram,
            lifecycle: crate::contexts::communications::domain::ConnectorLifecycle::Connected,
            generation: 1,
            safe_error_code: None,
            updated_at: "2026-07-18T10:00:00Z".to_string(),
        });

    assert_eq!(
        fixture
            .service
            .route_inbound(inbound(true))
            .expect("unbound"),
        InboundRouteOutcome::SystemReply {
            text: simplified_chinese_copy().unbound.to_string(),
        }
    );
    assert_eq!(
        fixture
            .service
            .route_inbound(inbound_text("/bind INVALID"))
            .expect("invalid pairing"),
        InboundRouteOutcome::SystemReply {
            text: simplified_chinese_copy().pairing_invalid.to_string(),
        }
    );
    let pairing = fixture
        .service
        .begin_pairing("session-1", ConnectorKind::Telegram, false)
        .await
        .expect("pairing");
    assert_eq!(
        fixture
            .service
            .route_inbound(inbound_text(&format!("/bind {}", pairing.code)))
            .expect("bind"),
        InboundRouteOutcome::SystemReply {
            text: simplified_chinese_copy().pairing_established.to_string(),
        }
    );
    fixture
        .service
        .set_binding_paused("session-1", true)
        .expect("pause");
    assert_eq!(
        fixture
            .service
            .route_inbound(inbound(true))
            .expect("paused"),
        InboundRouteOutcome::SystemReply {
            text: simplified_chinese_copy().paused.to_string(),
        }
    );
    fixture
        .service
        .set_binding_paused("session-1", false)
        .expect("resume");
    fixture
        .service
        .set_completion_notifications("session-1", true)
        .expect("notifications");
    assert!(fixture
        .service
        .notify_session_completion("session-1", "localized-message", false)
        .await
        .expect("notification"));
    assert!(fixture
        .transports
        .actions
        .lock()
        .expect("actions")
        .contains(&"notify:telegram:会话任务已完成。".to_string()));
}

#[test]
fn stale_session_binding_is_removed_without_agent_execution() {
    let fixture = fixture();
    fixture
        .repository
        .managed_bindings
        .lock()
        .expect("bindings")
        .insert(
            (ConnectorKind::Telegram, "chat-1".to_string()),
            session_binding("deleted-session", BindingState::Active),
        );
    fixture.sessions.missing.store(true, Ordering::Release);

    let outcome = fixture
        .service
        .route_inbound(inbound(true))
        .expect("guidance");

    assert!(matches!(outcome, InboundRouteOutcome::SystemReply { .. }));
    assert!(fixture.agents.executions.lock().expect("agents").is_empty());
    assert!(fixture
        .repository
        .managed_bindings
        .lock()
        .expect("bindings")
        .is_empty());
}

#[test]
fn paused_and_removed_bindings_block_delivery() {
    let fixture = fixture();
    fixture
        .repository
        .managed_bindings
        .lock()
        .expect("bindings")
        .insert(
            (ConnectorKind::Telegram, "chat-1".to_string()),
            session_binding("session-1", BindingState::Active),
        );

    let paused = fixture
        .service
        .set_binding_paused("session-1", true)
        .expect("pause");
    assert_eq!(paused.state, BindingState::Paused);
    assert!(matches!(
        fixture.service.route_inbound(inbound(true)).expect("paused"),
        InboundRouteOutcome::SystemReply { text } if text.contains("paused")
    ));
    fixture
        .service
        .set_completion_notifications("session-1", true)
        .expect("notifications");
    assert!(fixture
        .service
        .session_binding("session-1")
        .expect("snapshot")
        .binding
        .is_some_and(|binding| binding.completion_notifications));
    assert!(fixture.service.remove_binding("session-1").expect("remove"));
    assert!(fixture
        .service
        .session_binding("session-1")
        .expect("snapshot")
        .binding
        .is_none());
}

#[test]
fn router_uses_dedup_routing_binding_and_agent_ports() {
    let fixture = fixture();
    fixture
        .repository
        .managed_bindings
        .lock()
        .expect("bindings")
        .insert(
            (ConnectorKind::Telegram, "chat-1".to_string()),
            session_binding("session-1", BindingState::Active),
        );
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
    assert!(fixture
        .sessions
        .resolutions
        .lock()
        .expect("sessions")
        .is_empty());
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
    fixture
        .repository
        .managed_bindings
        .lock()
        .expect("bindings")
        .insert(
            (ConnectorKind::Telegram, "chat-1".to_string()),
            session_binding("session-existing", BindingState::Active),
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
        [(
            ConnectorKind::Telegram,
            "session-existing".to_string(),
            "status please".to_string(),
        )]
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
