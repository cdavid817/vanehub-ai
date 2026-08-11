use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct JsonRpcActorLimits {
    command_capacity: usize,
    inbound_capacity: usize,
    outbound_capacity: usize,
    notification_capacity: usize,
    event_capacity: usize,
    max_pending: usize,
}

impl JsonRpcActorLimits {
    pub(crate) fn new(
        command_capacity: usize,
        inbound_capacity: usize,
        outbound_capacity: usize,
        notification_capacity: usize,
        event_capacity: usize,
        max_pending: usize,
    ) -> Result<Self, JsonRpcError> {
        if [
            command_capacity,
            inbound_capacity,
            outbound_capacity,
            notification_capacity,
            event_capacity,
            max_pending,
        ]
        .contains(&0)
        {
            return Err(JsonRpcError::InvalidLimits);
        }
        Ok(Self {
            command_capacity,
            inbound_capacity,
            outbound_capacity,
            notification_capacity,
            event_capacity,
            max_pending,
        })
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonRpcError {
    #[error("JSON-RPC actor limits are invalid")]
    InvalidLimits,
    #[error("JSON-RPC queue is full")]
    QueueFull,
    #[error("JSON-RPC actor has stopped")]
    ActorStopped,
    #[error("JSON-RPC typed payload is invalid")]
    InvalidPayload,
    #[error("JSON-RPC message violates the protocol")]
    Protocol,
    #[error("JSON-RPC request id space is exhausted")]
    IdExhausted,
    #[error("JSON-RPC request timed out")]
    Timeout,
    #[error("JSON-RPC request was cancelled")]
    Cancelled,
    #[error("JSON-RPC peer returned error code {code}")]
    RemoteError { code: i64 },
}

#[derive(Clone)]
pub(crate) struct JsonRpcRequestControl {
    deadline: Duration,
    cleanup_reserve: Duration,
    cancelled: Arc<AtomicBool>,
}

impl JsonRpcRequestControl {
    pub(crate) fn standard(cancelled: Arc<AtomicBool>) -> Self {
        Self {
            deadline: Duration::from_secs(10),
            cleanup_reserve: Duration::from_millis(250),
            cancelled,
        }
    }

    pub(crate) fn new(
        deadline: Duration,
        cleanup_reserve: Duration,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Self, JsonRpcError> {
        if deadline.is_zero() || cleanup_reserve.is_zero() || cleanup_reserve >= deadline {
            return Err(JsonRpcError::InvalidLimits);
        }
        Ok(Self {
            deadline,
            cleanup_reserve,
            cancelled,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JsonRpcNotification {
    pub(crate) method: String,
    pub(crate) params: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JsonRpcProtocolEvent {
    MalformedMessage,
    UnknownResponse,
    NotificationDropped,
    OutboundDropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct JsonRpcErrorObject {
    code: i64,
    message: &'static str,
}

impl JsonRpcErrorObject {
    pub(crate) const fn new(code: i64, message: &'static str) -> Self {
        Self { code, message }
    }

    pub(crate) const fn method_not_found() -> Self {
        Self::new(-32601, "Method not found")
    }
}

pub(crate) trait ServerRequestHandler: Send + Sync + 'static {
    fn handle(&self, method: &str, params: Value) -> Result<Value, JsonRpcErrorObject>;
}

#[derive(Clone)]
pub(crate) struct JsonRpcClient {
    commands: mpsc::Sender<ActorCommand>,
}

impl JsonRpcClient {
    pub(crate) async fn notify<P>(&self, method: &str, params: P) -> Result<(), JsonRpcError>
    where
        P: Serialize,
    {
        if method.is_empty() {
            return Err(JsonRpcError::Protocol);
        }
        let params = serde_json::to_value(params).map_err(|_| JsonRpcError::InvalidPayload)?;
        let (respond, receive) = oneshot::channel();
        self.commands
            .try_send(ActorCommand::Notify {
                method: method.to_owned(),
                params,
                respond,
            })
            .map_err(map_send_error)?;
        receive.await.map_err(|_| JsonRpcError::ActorStopped)?
    }

    pub(crate) async fn request<P, R>(&self, method: &str, params: P) -> Result<R, JsonRpcError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let pending = self.start_request(method, params).await?;
        let value = pending
            .receive
            .await
            .map_err(|_| JsonRpcError::ActorStopped)??;
        decode_response(value)
    }

    pub(crate) async fn request_with_control<P, R>(
        &self,
        method: &str,
        params: P,
        control: JsonRpcRequestControl,
    ) -> Result<R, JsonRpcError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        if control.cancelled.load(Ordering::Acquire) {
            return Err(JsonRpcError::Cancelled);
        }
        let pending = self.start_request(method, params).await?;
        let id = pending.id;
        let mut receive = pending.receive;
        let active_wait = control.deadline - control.cleanup_reserve;
        tokio::select! {
            result = &mut receive => {
                let value = result.map_err(|_| JsonRpcError::ActorStopped)??;
                decode_response(value)
            }
            () = wait_for_cancellation(control.cancelled) => {
                self.cancel_and_cleanup(id, control.cleanup_reserve).await;
                Err(JsonRpcError::Cancelled)
            }
            () = tokio::time::sleep(active_wait) => {
                self.cancel_and_cleanup(id, control.cleanup_reserve).await;
                Err(JsonRpcError::Timeout)
            }
        }
    }

    async fn start_request<P>(
        &self,
        method: &str,
        params: P,
    ) -> Result<PendingRequest, JsonRpcError>
    where
        P: Serialize,
    {
        if method.is_empty() {
            return Err(JsonRpcError::Protocol);
        }
        let params = serde_json::to_value(params).map_err(|_| JsonRpcError::InvalidPayload)?;
        let (registered, registration) = oneshot::channel();
        let (respond, receive) = oneshot::channel();
        self.commands
            .try_send(ActorCommand::Request {
                method: method.to_owned(),
                params,
                registered,
                respond,
            })
            .map_err(map_send_error)?;
        let id = registration
            .await
            .map_err(|_| JsonRpcError::ActorStopped)??;
        Ok(PendingRequest { id, receive })
    }

    async fn cancel_and_cleanup(&self, id: u64, cleanup_reserve: Duration) {
        let commands = self.commands.clone();
        let cleanup = async move {
            let (acknowledge, acknowledged) = oneshot::channel();
            if commands
                .send(ActorCommand::Cancel { id, acknowledge })
                .await
                .is_ok()
            {
                let _ = acknowledged.await;
            }
        };
        let _ = tokio::time::timeout(cleanup_reserve, cleanup).await;
    }

    pub(crate) async fn pending_count(&self) -> Result<usize, JsonRpcError> {
        let (respond, receive) = oneshot::channel();
        self.commands
            .try_send(ActorCommand::PendingCount { respond })
            .map_err(map_send_error)?;
        receive.await.map_err(|_| JsonRpcError::ActorStopped)
    }
}

pub(crate) struct JsonRpcTransport {
    inbound: mpsc::Sender<Vec<u8>>,
    outbound: mpsc::Receiver<Vec<u8>>,
    notifications: mpsc::Receiver<JsonRpcNotification>,
    protocol_events: mpsc::Receiver<JsonRpcProtocolEvent>,
}

#[cfg_attr(test, allow(dead_code))]
pub(crate) struct JsonRpcEvents {
    notifications: mpsc::Receiver<JsonRpcNotification>,
    protocol_events: mpsc::Receiver<JsonRpcProtocolEvent>,
}

impl JsonRpcTransport {
    pub(crate) async fn send_inbound(&self, message: Vec<u8>) -> Result<(), JsonRpcError> {
        self.inbound
            .send(message)
            .await
            .map_err(|_| JsonRpcError::ActorStopped)
    }

    pub(crate) async fn recv_outbound(&mut self) -> Option<Vec<u8>> {
        self.outbound.recv().await
    }

    pub(crate) async fn recv_notification(&mut self) -> Option<JsonRpcNotification> {
        self.notifications.recv().await
    }

    pub(crate) async fn recv_protocol_event(&mut self) -> Option<JsonRpcProtocolEvent> {
        self.protocol_events.recv().await
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        mpsc::Sender<Vec<u8>>,
        mpsc::Receiver<Vec<u8>>,
        JsonRpcEvents,
    ) {
        (
            self.inbound,
            self.outbound,
            JsonRpcEvents {
                notifications: self.notifications,
                protocol_events: self.protocol_events,
            },
        )
    }
}

#[cfg_attr(test, allow(dead_code))]
impl JsonRpcEvents {
    pub(crate) async fn recv_notification(&mut self) -> Option<JsonRpcNotification> {
        self.notifications.recv().await
    }

    pub(crate) async fn recv_protocol_event(&mut self) -> Option<JsonRpcProtocolEvent> {
        self.protocol_events.recv().await
    }

    pub(crate) fn into_receivers(
        self,
    ) -> (
        mpsc::Receiver<JsonRpcNotification>,
        mpsc::Receiver<JsonRpcProtocolEvent>,
    ) {
        (self.notifications, self.protocol_events)
    }
}

pub(crate) fn spawn_json_rpc_actor(
    limits: JsonRpcActorLimits,
    handler: Arc<dyn ServerRequestHandler>,
) -> (JsonRpcClient, JsonRpcTransport) {
    let (commands_tx, commands_rx) = mpsc::channel(limits.command_capacity);
    let (inbound_tx, inbound_rx) = mpsc::channel(limits.inbound_capacity);
    let (outbound_tx, outbound_rx) = mpsc::channel(limits.outbound_capacity);
    let (notifications_tx, notifications_rx) = mpsc::channel(limits.notification_capacity);
    let (events_tx, events_rx) = mpsc::channel(limits.event_capacity);
    tokio::spawn(run_actor(
        limits.max_pending,
        commands_rx,
        inbound_rx,
        outbound_tx,
        notifications_tx,
        events_tx,
        handler,
    ));
    (
        JsonRpcClient {
            commands: commands_tx,
        },
        JsonRpcTransport {
            inbound: inbound_tx,
            outbound: outbound_rx,
            notifications: notifications_rx,
            protocol_events: events_rx,
        },
    )
}

enum ActorCommand {
    Notify {
        method: String,
        params: Value,
        respond: oneshot::Sender<Result<(), JsonRpcError>>,
    },
    Request {
        method: String,
        params: Value,
        registered: oneshot::Sender<Result<u64, JsonRpcError>>,
        respond: oneshot::Sender<Result<Value, JsonRpcError>>,
    },
    Cancel {
        id: u64,
        acknowledge: oneshot::Sender<()>,
    },
    PendingCount {
        respond: oneshot::Sender<usize>,
    },
}

struct PendingRequest {
    id: u64,
    receive: oneshot::Receiver<Result<Value, JsonRpcError>>,
}

struct ActorState {
    next_id: u64,
    max_pending: usize,
    pending: BTreeMap<u64, oneshot::Sender<Result<Value, JsonRpcError>>>,
}

async fn run_actor(
    max_pending: usize,
    mut commands: mpsc::Receiver<ActorCommand>,
    mut inbound: mpsc::Receiver<Vec<u8>>,
    outbound: mpsc::Sender<Vec<u8>>,
    notifications: mpsc::Sender<JsonRpcNotification>,
    events: mpsc::Sender<JsonRpcProtocolEvent>,
    handler: Arc<dyn ServerRequestHandler>,
) {
    let mut state = ActorState {
        next_id: 1,
        max_pending,
        pending: BTreeMap::new(),
    };
    let mut commands_open = true;
    loop {
        tokio::select! {
            command = commands.recv(), if commands_open => match command {
                Some(command) => handle_command(command, &mut state, &outbound, &events),
                None => commands_open = false,
            },
            message = inbound.recv() => match message {
                Some(message) => handle_inbound(
                    &message,
                    &mut state,
                    &outbound,
                    &notifications,
                    &events,
                    handler.as_ref(),
                ),
                None => break,
            },
        }
    }
    for (_, respond) in state.pending {
        let _ = respond.send(Err(JsonRpcError::ActorStopped));
    }
}

fn handle_command(
    command: ActorCommand,
    state: &mut ActorState,
    outbound: &mpsc::Sender<Vec<u8>>,
    events: &mpsc::Sender<JsonRpcProtocolEvent>,
) {
    state.pending.retain(|_, respond| !respond.is_closed());
    match command {
        ActorCommand::Notify {
            method,
            params,
            respond,
        } => {
            let message = json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params,
            });
            let result = serde_json::to_vec(&message)
                .map_err(|_| JsonRpcError::InvalidPayload)
                .and_then(|bytes| outbound.try_send(bytes).map_err(map_send_error));
            let _ = respond.send(result);
        }
        ActorCommand::PendingCount { respond } => {
            let _ = respond.send(state.pending.len());
        }
        ActorCommand::Cancel { id, acknowledge } => {
            if state.pending.remove(&id).is_some() {
                let cancellation = json!({
                    "jsonrpc": "2.0",
                    "method": "$/cancelRequest",
                    "params": {"id": id},
                });
                match serde_json::to_vec(&cancellation) {
                    Ok(bytes) => {
                        if outbound.try_send(bytes).is_err() {
                            emit_event(events, JsonRpcProtocolEvent::OutboundDropped);
                        }
                    }
                    Err(_) => emit_event(events, JsonRpcProtocolEvent::OutboundDropped),
                }
            }
            let _ = acknowledge.send(());
        }
        ActorCommand::Request {
            method,
            params,
            registered,
            respond,
        } => {
            if state.pending.len() >= state.max_pending {
                let _ = registered.send(Err(JsonRpcError::QueueFull));
                return;
            }
            let id = state.next_id;
            let Some(next_id) = id.checked_add(1) else {
                let _ = registered.send(Err(JsonRpcError::IdExhausted));
                return;
            };
            let message = json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            });
            let Ok(bytes) = serde_json::to_vec(&message) else {
                let _ = registered.send(Err(JsonRpcError::InvalidPayload));
                return;
            };
            if let Err(error) = outbound.try_send(bytes) {
                let _ = registered.send(Err(map_send_error(error)));
                return;
            }
            state.next_id = next_id;
            state.pending.insert(id, respond);
            if registered.send(Ok(id)).is_err() {
                state.pending.remove(&id);
            }
        }
    }
}

fn handle_inbound(
    bytes: &[u8],
    state: &mut ActorState,
    outbound: &mpsc::Sender<Vec<u8>>,
    notifications: &mpsc::Sender<JsonRpcNotification>,
    events: &mpsc::Sender<JsonRpcProtocolEvent>,
    handler: &dyn ServerRequestHandler,
) {
    state.pending.retain(|_, respond| !respond.is_closed());
    let Ok(message) = serde_json::from_slice::<Value>(bytes) else {
        emit_event(events, JsonRpcProtocolEvent::MalformedMessage);
        return;
    };
    let Some(object) = message.as_object() else {
        emit_event(events, JsonRpcProtocolEvent::MalformedMessage);
        return;
    };
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        emit_event(events, JsonRpcProtocolEvent::MalformedMessage);
        return;
    }
    if let Some(method) = object.get("method") {
        let Some(method) = method.as_str() else {
            emit_event(events, JsonRpcProtocolEvent::MalformedMessage);
            return;
        };
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        if let Some(id) = object.get("id") {
            handle_server_request(id, method, params, outbound, events, handler);
        } else if notifications
            .try_send(JsonRpcNotification {
                method: method.to_owned(),
                params,
            })
            .is_err()
        {
            emit_event(events, JsonRpcProtocolEvent::NotificationDropped);
        }
        return;
    }
    handle_response(object, state, events);
}

fn handle_server_request(
    id: &Value,
    method: &str,
    params: Value,
    outbound: &mpsc::Sender<Vec<u8>>,
    events: &mpsc::Sender<JsonRpcProtocolEvent>,
    handler: &dyn ServerRequestHandler,
) {
    if !(id.is_string() || id.as_u64().is_some() || id.as_i64().is_some()) {
        emit_event(events, JsonRpcProtocolEvent::MalformedMessage);
        return;
    }
    let response = match handler.handle(method, params) {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(error) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": error.code, "message": error.message},
        }),
    };
    let Ok(bytes) = serde_json::to_vec(&response) else {
        emit_event(events, JsonRpcProtocolEvent::MalformedMessage);
        return;
    };
    if outbound.try_send(bytes).is_err() {
        emit_event(events, JsonRpcProtocolEvent::OutboundDropped);
    }
}

fn handle_response(
    object: &serde_json::Map<String, Value>,
    state: &mut ActorState,
    events: &mpsc::Sender<JsonRpcProtocolEvent>,
) {
    let Some(id) = object.get("id").and_then(Value::as_u64) else {
        emit_event(events, JsonRpcProtocolEvent::MalformedMessage);
        return;
    };
    let Some(respond) = state.pending.remove(&id) else {
        emit_event(events, JsonRpcProtocolEvent::UnknownResponse);
        return;
    };
    let result = match (object.get("result"), object.get("error")) {
        (Some(result), None) => Ok(result.clone()),
        (None, Some(error)) => error
            .get("code")
            .and_then(Value::as_i64)
            .map(|code| Err(JsonRpcError::RemoteError { code }))
            .unwrap_or(Err(JsonRpcError::Protocol)),
        _ => Err(JsonRpcError::Protocol),
    };
    let _ = respond.send(result);
}

fn emit_event(events: &mpsc::Sender<JsonRpcProtocolEvent>, event: JsonRpcProtocolEvent) {
    let _ = events.try_send(event);
}

fn map_send_error<T>(error: mpsc::error::TrySendError<T>) -> JsonRpcError {
    match error {
        mpsc::error::TrySendError::Full(_) => JsonRpcError::QueueFull,
        mpsc::error::TrySendError::Closed(_) => JsonRpcError::ActorStopped,
    }
}

fn decode_response<R: DeserializeOwned>(value: Value) -> Result<R, JsonRpcError> {
    serde_json::from_value(value).map_err(|_| JsonRpcError::InvalidPayload)
}

async fn wait_for_cancellation(cancelled: Arc<AtomicBool>) {
    let mut interval = tokio::time::interval(Duration::from_millis(10));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if cancelled.load(Ordering::Acquire) {
            return;
        }
        interval.tick().await;
    }
}
