use super::transports::normalize_fixture;
use crate::contexts::communications::api::{CommunicationsApi, InboundRouteOutcome};
use crate::contexts::communications::domain::{
    builtin_descriptors, split_text, ConnectorKind, NormalizedInbound,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Mutex;

const ACTIVATION: &str = "VANEHUB_FEISHU_IM_FIXTURE";
const RECORDED_DIRECT_TEXT: &str = include_str!("transports/fixtures/feishu-direct-text.json");
const FIXTURE_CHAT_ID: &str = "desktop-e2e-feishu-chat-v1";
const FIXTURE_SENDER_ID: &str = "desktop-e2e-feishu-sender-v1";
const FEISHU_MAX_OUTBOUND_CHARS: usize = 20_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FeishuFixtureError {
    Validation(String),
    Storage(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeishuFixtureSetupResult {
    ready: bool,
    connector: ConnectorKind,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeishuFixtureEvent {
    #[serde(default)]
    pub(crate) connector: Option<ConnectorKind>,
    pub(crate) event_id: String,
    pub(crate) text: String,
    #[serde(default = "default_direct")]
    pub(crate) direct: bool,
    #[serde(default)]
    pub(crate) malformed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeishuFixtureLedgerEntry {
    pub(crate) sequence: u64,
    pub(crate) status: String,
    pub(crate) duplicate: bool,
    pub(crate) outbound_chunks: usize,
    pub(crate) safe_error_code: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum FixtureFault {
    #[default]
    None,
    Disconnected,
    OutboundFailure,
}

#[derive(Debug, Default)]
struct FakeConnectedFeishuTransport {
    connected: bool,
    fault: FixtureFault,
    sequence: u64,
    ledger: Vec<FeishuFixtureLedgerEntry>,
}

impl FakeConnectedFeishuTransport {
    fn record(
        &mut self,
        status: &str,
        duplicate: bool,
        outbound_chunks: usize,
        safe_error_code: Option<&str>,
    ) -> FeishuFixtureLedgerEntry {
        self.sequence += 1;
        let entry = FeishuFixtureLedgerEntry {
            sequence: self.sequence,
            status: status.to_string(),
            duplicate,
            outbound_chunks,
            safe_error_code: safe_error_code.map(str::to_string),
        };
        self.ledger.push(entry.clone());
        entry
    }
}

#[derive(Debug, Default)]
pub(crate) struct FeishuDesktopFixture {
    transport: Mutex<FakeConnectedFeishuTransport>,
}

impl FeishuDesktopFixture {
    fn require_activation() -> Result<(), FeishuFixtureError> {
        if std::env::var_os(ACTIVATION).is_some_and(|value| value == "1") {
            Ok(())
        } else {
            Err(FeishuFixtureError::Validation(
                "feishu-im-fixture-disabled".to_string(),
            ))
        }
    }

    pub(crate) fn connect(&self) -> Result<(), &'static str> {
        let mut transport = self.transport.lock().map_err(|_| "fixture-lock-failed")?;
        transport.connected = true;
        transport.fault = FixtureFault::None;
        Ok(())
    }

    pub(crate) fn setup(
        &self,
        database: &NativeDatabase,
        session_id: &str,
        connector: ConnectorKind,
    ) -> Result<FeishuFixtureSetupResult, FeishuFixtureError> {
        Self::require_activation()?;
        self.connect().map_err(storage)?;
        let now = chrono::Utc::now().to_rfc3339();
        let chat_hash = Sha256::digest(fixture_chat_id(connector).as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let connection = database
            .connection()
            .map_err(|_| storage("feishu-im-fixture-database-unavailable"))?;
        let exists = connection
            .query_row(
                "SELECT 1 FROM sessions WHERE id = ?1",
                params![session_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|_| storage("feishu-im-fixture-session-lookup-failed"))?
            .is_some();
        if !exists {
            return Err(FeishuFixtureError::Validation(
                "feishu-im-fixture-session-not-found".to_string(),
            ));
        }
        connection
            .execute(
                "INSERT INTO im_session_connector_access \
                 (session_id, connector, enabled, updated_at) VALUES (?1, ?2, 1, ?3) \
                 ON CONFLICT(session_id, connector) DO UPDATE SET enabled = 1, updated_at = ?3",
                params![session_id, connector.as_str(), now],
            )
            .map_err(|_| storage("feishu-im-fixture-access-write-failed"))?;
        connection
            .execute(
                "INSERT INTO im_session_bindings \
                 (connector, external_chat_hash, session_id, state, completion_notifications, \
                  delivery_credential_ref, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, 'active', 0, NULL, ?4, ?4) \
                 ON CONFLICT(connector, external_chat_hash) DO UPDATE SET \
                  session_id = ?3, state = 'active', updated_at = ?4",
                params![connector.as_str(), chat_hash, session_id, now],
            )
            .map_err(|_| storage("feishu-im-fixture-binding-write-failed"))?;
        Ok(FeishuFixtureSetupResult {
            ready: true,
            connector,
        })
    }

    pub(crate) async fn inject(
        &self,
        api: &CommunicationsApi,
        input: FeishuFixtureEvent,
    ) -> Result<FeishuFixtureLedgerEntry, FeishuFixtureError> {
        Self::require_activation()?;
        if input.event_id.trim().is_empty() || input.text.trim().is_empty() {
            return Err(FeishuFixtureError::Validation(
                "feishu-im-fixture-invalid-event".to_string(),
            ));
        }
        let inbound = match self.normalize(input) {
            Ok(inbound) => inbound,
            Err("fixture-disconnected") => {
                return self
                    .record("reconnecting", false, 0, Some("fixture-disconnected"))
                    .map_err(storage)
            }
            Err("fixture-event-invalid") => {
                return self
                    .record("malformed", false, 0, Some("fixture-event-invalid"))
                    .map_err(storage)
            }
            Err(error) => return Err(FeishuFixtureError::Validation(error.to_string())),
        };
        let api = api.clone();
        let inbound_connector = inbound.connector;
        let event_id = inbound.event_id.clone();
        let claimed = tauri::async_runtime::spawn_blocking({
            let api = api.clone();
            move || api.claim_inbound(inbound_connector, &event_id)
        })
        .await
        .map_err(|_| storage("feishu-im-fixture-claim-task-failed"))?
        .map_err(|error| storage(error.safe_code()))?;
        if !claimed {
            return self.record("duplicate", true, 0, None).map_err(storage);
        }
        let outcome = tauri::async_runtime::spawn_blocking(move || api.route_inbound(inbound))
            .await
            .map_err(|_| storage("feishu-im-fixture-route-task-failed"))?;
        match outcome {
            Ok(InboundRouteOutcome::Reply { text, .. }) => {
                let limit = builtin_descriptors()
                    .into_iter()
                    .find(|descriptor| descriptor.kind == inbound_connector)
                    .map(|descriptor| descriptor.max_outbound_chars)
                    .unwrap_or(FEISHU_MAX_OUTBOUND_CHARS);
                self.record("delivered", false, split_text(&text, limit).len(), None)
            }
            Ok(InboundRouteOutcome::SystemReply { .. }) => {
                self.record("system-reply", false, 1, None)
            }
            Ok(InboundRouteOutcome::Ignored) => self.record("ignored", false, 0, None),
            Err(error) => self.record("rejected", false, 0, Some(error.safe_code())),
        }
        .map_err(storage)
    }

    pub(crate) fn normalize(
        &self,
        event: FeishuFixtureEvent,
    ) -> Result<NormalizedInbound, &'static str> {
        let transport = self.transport.lock().map_err(|_| "fixture-lock-failed")?;
        if !transport.connected || transport.fault == FixtureFault::Disconnected {
            return Err("fixture-disconnected");
        }
        drop(transport);
        let connector = event.connector.unwrap_or(ConnectorKind::Feishu);
        if connector != ConnectorKind::Feishu {
            if event.malformed {
                return Err("fixture-event-invalid");
            }
            return Ok(NormalizedInbound {
                connector,
                event_id: event.event_id,
                chat_id: fixture_chat_id(connector).to_string(),
                sender_id: fixture_sender_id(connector).to_string(),
                text: event.text,
                direct: event.direct,
                reply_context: None,
            });
        }
        let mut payload: Value =
            serde_json::from_str(RECORDED_DIRECT_TEXT).map_err(|_| "fixture-recording-invalid")?;
        payload["header"]["event_id"] = json!(event.event_id);
        payload["event"]["sender"]["sender_id"]["open_id"] = json!(FIXTURE_SENDER_ID);
        payload["event"]["message"]["chat_id"] = json!(FIXTURE_CHAT_ID);
        payload["event"]["message"]["chat_type"] =
            json!(if event.direct { "p2p" } else { "group" });
        payload["event"]["message"]["content"] = if event.malformed {
            json!("{malformed-recorded-content")
        } else {
            json!(json!({ "text": event.text }).to_string())
        };
        normalize_fixture(ConnectorKind::Feishu, &payload.to_string())
            .map_err(|_| "fixture-event-invalid")
    }

    pub(crate) fn set_fault(&self, fault: &str) -> Result<(), &'static str> {
        let fault = match fault {
            "none" => FixtureFault::None,
            "disconnected" => FixtureFault::Disconnected,
            "outbound-failure" => FixtureFault::OutboundFailure,
            _ => return Err("fixture-fault-invalid"),
        };
        self.transport
            .lock()
            .map_err(|_| "fixture-lock-failed")?
            .fault = fault;
        Ok(())
    }

    pub(crate) fn record(
        &self,
        status: &str,
        duplicate: bool,
        outbound_chunks: usize,
        safe_error_code: Option<&str>,
    ) -> Result<FeishuFixtureLedgerEntry, &'static str> {
        let mut transport = self.transport.lock().map_err(|_| "fixture-lock-failed")?;
        if status == "delivered" && transport.fault == FixtureFault::OutboundFailure {
            return Ok(transport.record(
                "outbound-failed",
                duplicate,
                outbound_chunks,
                Some("fixture-outbound-failed"),
            ));
        }
        Ok(transport.record(status, duplicate, outbound_chunks, safe_error_code))
    }

    pub(crate) fn ledger(&self) -> Result<Vec<FeishuFixtureLedgerEntry>, &'static str> {
        self.transport
            .lock()
            .map(|transport| transport.ledger.clone())
            .map_err(|_| "fixture-lock-failed")
    }

    pub(crate) fn reset(&self) -> Result<(), &'static str> {
        *self.transport.lock().map_err(|_| "fixture-lock-failed")? =
            FakeConnectedFeishuTransport::default();
        Ok(())
    }

    pub(crate) fn set_fixture_fault(&self, fault: &str) -> Result<(), FeishuFixtureError> {
        Self::require_activation()?;
        self.set_fault(fault).map_err(|_| {
            FeishuFixtureError::Validation("feishu-im-fixture-invalid-fault".to_string())
        })
    }

    pub(crate) fn fixture_ledger(
        &self,
    ) -> Result<Vec<FeishuFixtureLedgerEntry>, FeishuFixtureError> {
        Self::require_activation()?;
        self.ledger().map_err(storage)
    }

    pub(crate) fn reset_fixture(&self) -> Result<(), FeishuFixtureError> {
        Self::require_activation()?;
        self.reset().map_err(storage)
    }
}

fn storage(code: &str) -> FeishuFixtureError {
    FeishuFixtureError::Storage(code.to_string())
}

fn default_direct() -> bool {
    true
}

fn fixture_chat_id(connector: ConnectorKind) -> &'static str {
    match connector {
        ConnectorKind::Feishu => FIXTURE_CHAT_ID,
        ConnectorKind::Telegram => "desktop-e2e-telegram-chat-v1",
        ConnectorKind::DingTalk => "desktop-e2e-dingtalk-chat-v1",
        ConnectorKind::WeCom => "desktop-e2e-wecom-chat-v1",
        ConnectorKind::WeChat => "desktop-e2e-weixin-chat-v1",
    }
}

fn fixture_sender_id(connector: ConnectorKind) -> &'static str {
    match connector {
        ConnectorKind::Feishu => FIXTURE_SENDER_ID,
        ConnectorKind::Telegram => "desktop-e2e-telegram-sender-v1",
        ConnectorKind::DingTalk => "desktop-e2e-dingtalk-sender-v1",
        ConnectorKind::WeCom => "desktop-e2e-wecom-sender-v1",
        ConnectorKind::WeChat => "desktop-e2e-weixin-sender-v1",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorded_events_use_the_real_normalizer_and_safe_fixture_identities() {
        let fixture = FeishuDesktopFixture::default();
        fixture.connect().unwrap();
        let inbound = fixture
            .normalize(FeishuFixtureEvent {
                connector: None,
                event_id: "fixture-event-1".to_string(),
                text: "fixture text".to_string(),
                direct: true,
                malformed: false,
            })
            .unwrap();
        assert_eq!(inbound.connector, ConnectorKind::Feishu);
        assert_eq!(inbound.event_id, "fixture-event-1");
        assert_eq!(inbound.chat_id, FIXTURE_CHAT_ID);
        assert_eq!(inbound.sender_id, FIXTURE_SENDER_ID);
        assert_eq!(inbound.text, "fixture text");
        assert!(inbound.direct);

        let telegram = fixture
            .normalize(FeishuFixtureEvent {
                connector: Some(ConnectorKind::Telegram),
                event_id: "fixture-telegram-1".to_string(),
                text: "telegram fixture text".to_string(),
                direct: true,
                malformed: false,
            })
            .unwrap();
        assert_eq!(telegram.connector, ConnectorKind::Telegram);
        assert_eq!(telegram.chat_id, fixture_chat_id(ConnectorKind::Telegram));
        assert_eq!(telegram.text, "telegram fixture text");

        let malformed = fixture.normalize(FeishuFixtureEvent {
            connector: None,
            event_id: "fixture-malformed-1".to_string(),
            text: "not retained".to_string(),
            direct: true,
            malformed: true,
        });
        assert_eq!(malformed.unwrap_err(), "fixture-event-invalid");
    }

    #[test]
    fn fake_transport_requires_connection_and_retains_only_safe_metadata() {
        let fixture = FeishuDesktopFixture::default();
        assert_eq!(
            fixture
                .normalize(FeishuFixtureEvent {
                    connector: None,
                    event_id: "private-event".to_string(),
                    text: "private text".to_string(),
                    direct: true,
                    malformed: false,
                })
                .unwrap_err(),
            "fixture-disconnected"
        );
        fixture.connect().unwrap();
        fixture.set_fault("outbound-failure").unwrap();
        let entry = fixture.record("delivered", false, 2, None).unwrap();
        assert_eq!(entry.status, "outbound-failed");
        let serialized = serde_json::to_string(&fixture.ledger().unwrap()).unwrap();
        assert!(!serialized.contains("private-event"));
        assert!(!serialized.contains("private text"));
    }
}
