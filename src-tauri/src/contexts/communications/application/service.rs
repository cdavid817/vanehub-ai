use super::lifecycle_coordinator::ConnectorLifecycleCoordinator;
use super::{
    AgentExecutionRequest, CommunicationsAgentExecutionPort, CommunicationsApplicationError,
    CommunicationsClockPort, CommunicationsCredentialPort, CommunicationsLog,
    CommunicationsLogLevel, CommunicationsLoggingPort, CommunicationsOperationPort,
    CommunicationsRepository, CommunicationsSessionBindingPort, CommunicationsTransportPort,
    ConnectorCredential, ConnectorRuntimeDefinition, ConnectorStartupResult, ConnectorSummary,
    InboundRouteOutcome, SaveConnectorRequest,
};
use crate::contexts::communications::domain::{
    builtin_descriptors, connector_field_definitions, ChatBindingKey, ConnectorConfig,
    ConnectorFieldStorage, ConnectorHealth, ConnectorKind, ConnectorLifecycle, InboundDisposition,
    InboundEventIdentity, NormalizedInbound, RoutingSettings,
};

/// Synchronous snapshot of connector configuration + credential presence, captured by
/// `connector_snapshot` on the blocking pool so the async executor never waits on rusqlite
/// or credential storage.
pub(crate) struct ConnectorSnapshot {
    pub(crate) configurations: HashMap<ConnectorKind, ConnectorConfig>,
    pub(crate) credentials: HashMap<ConnectorKind, bool>,
}
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use zeroize::Zeroizing;

const DEDUP_RETENTION_DAYS: u32 = 7;
pub(super) const DEDUP_MAINTENANCE_BATCH: usize = 512;
const DEDUP_MAINTENANCE_MIN_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60 * 60);

#[derive(Clone)]
pub(crate) struct CommunicationsApplicationPorts {
    pub(crate) repository: Arc<dyn CommunicationsRepository>,
    pub(crate) credentials: Arc<dyn CommunicationsCredentialPort>,
    pub(crate) transports: Arc<dyn CommunicationsTransportPort>,
    pub(crate) agents: Arc<dyn CommunicationsAgentExecutionPort>,
    pub(crate) sessions: Arc<dyn CommunicationsSessionBindingPort>,
    pub(crate) operations: Arc<dyn CommunicationsOperationPort>,
    pub(crate) clock: Arc<dyn CommunicationsClockPort>,
    pub(crate) logging: Arc<dyn CommunicationsLoggingPort>,
}

#[derive(Clone)]
pub(crate) struct CommunicationsApplicationService {
    ports: CommunicationsApplicationPorts,
    lifecycle: ConnectorLifecycleCoordinator,
    last_dedup_maintenance: Arc<Mutex<Option<std::time::Instant>>>,
}

impl CommunicationsApplicationService {
    pub(crate) fn new(ports: CommunicationsApplicationPorts) -> Self {
        Self {
            ports,
            lifecycle: ConnectorLifecycleCoordinator::default(),
            last_dedup_maintenance: Arc::new(Mutex::new(None)),
        }
    }

    /// Test-facing convenience that runs the snapshot, health lookup, and assembly inline.
    /// Production code calls `CommunicationsApi::list_connectors`, which keeps the blocking
    /// snapshot on `spawn_blocking`; this synchronous-flavored path exists so unit tests can
    /// exercise the same assembly without a Tauri runtime.
    #[cfg(test)]
    pub(crate) async fn list_connectors(
        &self,
    ) -> Result<Vec<ConnectorSummary>, CommunicationsApplicationError> {
        let snapshot = self.connector_snapshot()?;
        let health = self
            .ports
            .transports
            .health()
            .await
            .into_iter()
            .map(|health| (health.kind, health))
            .collect::<HashMap<_, _>>();
        Ok(self.assemble_connectors(snapshot, health))
    }

    /// Synchronous DB + credential snapshot used by `list_connectors`. Separated so the
    /// blocking I/O can run on `spawn_blocking` from the api layer instead of stalling the
    /// async executor (each `list_configurations` / `credentials.load` is synchronous
    /// rusqlite / credential storage access).
    pub(crate) fn connector_snapshot(
        &self,
    ) -> Result<ConnectorSnapshot, CommunicationsApplicationError> {
        let configurations = self
            .ports
            .repository
            .list_configurations()?
            .into_iter()
            .map(|configuration| (configuration.kind, configuration))
            .collect::<HashMap<_, _>>();
        let credentials: HashMap<ConnectorKind, bool> = builtin_descriptors()
            .into_iter()
            .map(|descriptor| {
                let has_credentials = self.ports.credentials.load(descriptor.kind)?.is_some();
                Ok::<_, CommunicationsApplicationError>((descriptor.kind, has_credentials))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(ConnectorSnapshot {
            configurations,
            credentials,
        })
    }

    /// Live transport health for every connector kind. Async because it queries the
    /// running transports; exposed so the api layer can await it between the synchronous
    /// snapshot and the assembly step.
    pub(crate) async fn transport_health(&self) -> Vec<ConnectorHealth> {
        self.ports.transports.health().await
    }

    /// Pure assembly of connector summaries from a DB/credential snapshot and the live
    /// transport health. Synchronous and allocation-only — safe to run on the blocking pool.
    pub(crate) fn assemble_connectors(
        &self,
        snapshot: ConnectorSnapshot,
        health: HashMap<ConnectorKind, ConnectorHealth>,
    ) -> Vec<ConnectorSummary> {
        let ConnectorSnapshot {
            configurations,
            credentials,
        } = snapshot;
        let now = self.ports.clock.now_rfc3339();
        builtin_descriptors()
            .into_iter()
            .map(|descriptor| {
                let kind = descriptor.kind;
                let configuration = configurations
                    .get(&kind)
                    .cloned()
                    .unwrap_or_else(|| default_configuration(kind));
                let has_credentials = *credentials.get(&kind).unwrap_or(&false);
                let mut connector_health =
                    health
                        .get(&kind)
                        .cloned()
                        .unwrap_or_else(|| ConnectorHealth {
                            kind,
                            lifecycle: if configuration.enabled {
                                ConnectorLifecycle::Error
                            } else {
                                ConnectorLifecycle::Disabled
                            },
                            generation: 0,
                            safe_error_code: configuration
                                .enabled
                                .then(|| "connector-not-started".to_string()),
                            updated_at: now.clone(),
                        });
                if !has_credentials {
                    connector_health.lifecycle = ConnectorLifecycle::Unconfigured;
                    connector_health.safe_error_code = None;
                }
                ConnectorSummary {
                    descriptor,
                    configuration,
                    health: connector_health,
                    has_credentials,
                }
            })
            .collect()
    }

    pub(crate) fn routing(
        &self,
    ) -> Result<Option<RoutingSettings>, CommunicationsApplicationError> {
        self.ports.repository.load_routing()
    }

    pub(crate) fn save_routing(
        &self,
        routing: &RoutingSettings,
    ) -> Result<RoutingSettings, CommunicationsApplicationError> {
        let routing = self.ports.agents.validate_routing(routing)?;
        let updated_at = self.ports.clock.now_rfc3339();
        self.ports.repository.save_routing(&routing, &updated_at)?;
        Ok(routing)
    }

    pub(crate) async fn save_connector(
        &self,
        request: SaveConnectorRequest,
    ) -> Result<ConnectorConfig, CommunicationsApplicationError> {
        let _lifecycle = self.lifecycle.lock(request.kind).await;
        let previous_configuration = self.ports.repository.find_configuration(request.kind)?;
        let previous_credential = self.ports.credentials.load(request.kind)?;
        let candidate = prepare_connector_candidate(
            request.kind,
            request.public_config,
            previous_credential.as_ref(),
            request.credential_patch.as_ref(),
        )?;
        let credential_changed = candidate.secret.as_ref().map(|secret| secret.as_str())
            != previous_credential
                .as_ref()
                .map(|credential| credential.secret.as_str());
        let mut configuration = ConnectorConfig {
            kind: request.kind,
            enabled: request.enabled,
            display_name: request.display_name,
            public_config: candidate.public_config,
            credential_ref: previous_credential
                .as_ref()
                .map(|credential| credential.reference.clone()),
        };
        configuration.validate()?;
        if configuration.enabled {
            self.require_routing()?;
            if candidate.secret.is_none() {
                return Err(credentials_required());
            }
        }

        let replacement = match candidate.secret {
            Some(secret) if credential_changed => Some(
                self.ports
                    .credentials
                    .store(request.kind, secret.as_str())?,
            ),
            _ => None,
        };
        if let Some(credential) = &replacement {
            configuration.credential_ref = Some(credential.reference.clone());
        }
        let updated_at = self.ports.clock.now_rfc3339();
        if let Err(error) = self
            .ports
            .repository
            .save_configuration(&configuration, &updated_at)
        {
            if credential_changed {
                self.restore_credential(request.kind, previous_credential.as_ref())?;
            }
            return Err(error);
        }
        let runtime_result = if configuration.enabled {
            let credential = replacement
                .or_else(|| previous_credential.clone())
                .ok_or_else(credentials_required)?;
            self.ports
                .transports
                .replace_and_start(runtime_definition(configuration.clone(), credential))
                .await
        } else {
            self.ports.transports.stop(request.kind).await
        };
        if let Err(primary) = runtime_result {
            let rollback = self
                .restore_connector_snapshot(
                    request.kind,
                    previous_configuration.as_ref(),
                    previous_credential.as_ref(),
                )
                .await;
            self.record_lifecycle_rollback(request.kind, &primary, rollback.as_ref().err());
            return Err(primary);
        }
        self.record(
            CommunicationsLogLevel::Info,
            "communications.connector.saved",
            "Connector configuration saved.",
            Some(request.kind),
            None,
            None,
        );
        Ok(configuration)
    }

    pub(crate) async fn set_connector_enabled(
        &self,
        kind: ConnectorKind,
        enabled: bool,
    ) -> Result<(), CommunicationsApplicationError> {
        let _lifecycle = self.lifecycle.lock(kind).await;
        let previous_configuration = self.ports.repository.find_configuration(kind)?;
        let mut configuration = previous_configuration
            .clone()
            .unwrap_or_else(|| default_configuration(kind));
        let credential = self.ports.credentials.load(kind)?;
        if enabled {
            self.require_routing()?;
            if credential.is_none() {
                return Err(credentials_required());
            }
        }
        configuration.enabled = enabled;
        configuration.credential_ref = credential
            .as_ref()
            .map(|credential| credential.reference.clone());
        let updated_at = self.ports.clock.now_rfc3339();
        self.ports
            .repository
            .save_configuration(&configuration, &updated_at)?;
        let runtime_result = if enabled {
            self.ports
                .transports
                .replace_and_start(runtime_definition(
                    configuration,
                    credential.clone().ok_or_else(credentials_required)?,
                ))
                .await
        } else {
            self.ports.transports.stop(kind).await
        };
        if let Err(primary) = runtime_result {
            let rollback = self
                .restore_connector_snapshot(
                    kind,
                    previous_configuration.as_ref(),
                    credential.as_ref(),
                )
                .await;
            self.record_lifecycle_rollback(kind, &primary, rollback.as_ref().err());
            return Err(primary);
        }
        Ok(())
    }

    pub(crate) async fn clear_connector(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        let _lifecycle = self.lifecycle.lock(kind).await;
        let previous_configuration = self.ports.repository.find_configuration(kind)?;
        let previous_credential = self.ports.credentials.load(kind)?;
        if let Err(primary) = self.ports.transports.stop(kind).await {
            let rollback = self
                .restore_connector_snapshot(
                    kind,
                    previous_configuration.as_ref(),
                    previous_credential.as_ref(),
                )
                .await;
            self.record_lifecycle_rollback(kind, &primary, rollback.as_ref().err());
            return Err(primary);
        }
        if let Err(primary) = self.ports.credentials.delete(kind) {
            let rollback = self
                .restore_connector_snapshot(
                    kind,
                    previous_configuration.as_ref(),
                    previous_credential.as_ref(),
                )
                .await;
            self.record_lifecycle_rollback(kind, &primary, rollback.as_ref().err());
            return Err(primary);
        }
        if let Some(mut configuration) = previous_configuration.clone() {
            configuration.enabled = false;
            configuration.credential_ref = None;
            let updated_at = self.ports.clock.now_rfc3339();
            if let Err(error) = self
                .ports
                .repository
                .save_configuration(&configuration, &updated_at)
            {
                let rollback = self
                    .restore_connector_snapshot(
                        kind,
                        previous_configuration.as_ref(),
                        previous_credential.as_ref(),
                    )
                    .await;
                self.record_lifecycle_rollback(kind, &error, rollback.as_ref().err());
                return Err(error);
            }
        }
        self.ports.transports.clear_connector_data(kind).await?;
        Ok(())
    }

    pub(crate) async fn test_connector(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        let _lifecycle = self.lifecycle.lock(kind).await;
        let operation = self.ports.operations.start(kind, "test")?;
        let result = match self.load_runtime_definition(kind) {
            Ok(definition) => self.ports.transports.test(definition).await,
            Err(error) => Err(error),
        };
        self.finish_operation(kind, "test", &operation.id, &result);
        result
    }

    pub(crate) async fn restart_connector(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        let _lifecycle = self.lifecycle.lock(kind).await;
        self.require_routing()?;
        let operation = self.ports.operations.start(kind, "restart")?;
        let result = match self.load_runtime_definition(kind) {
            Ok(definition) => self.ports.transports.replace_and_start(definition).await,
            Err(error) => Err(error),
        };
        self.finish_operation(kind, "restart", &operation.id, &result);
        result
    }

    pub(crate) async fn start_saved_connectors(
        &self,
    ) -> Result<Vec<ConnectorStartupResult>, CommunicationsApplicationError> {
        let configurations = self.ports.repository.list_configurations()?;
        let starts = configurations
            .into_iter()
            .filter(|configuration| configuration.enabled)
            .map(|configuration| async move {
                let kind = configuration.kind;
                let _lifecycle = self.lifecycle.lock(kind).await;
                let result = match self.require_routing() {
                    Ok(_) => match self.ports.credentials.load(kind) {
                        Ok(Some(credential)) => {
                            self.ports
                                .transports
                                .replace_and_start(runtime_definition(configuration, credential))
                                .await
                        }
                        Ok(None) => Err(credentials_required()),
                        Err(error) => Err(error),
                    },
                    Err(error) => Err(error),
                };
                if let Err(error) = &result {
                    self.record(
                        CommunicationsLogLevel::Error,
                        "communications.connector.start",
                        "Connector startup failed.",
                        Some(kind),
                        Some(error.safe_code()),
                        None,
                    );
                }
                ConnectorStartupResult {
                    kind,
                    safe_error_code: result.err().map(|error| error.safe_code().to_string()),
                }
            });
        let mut results = futures_util::future::join_all(starts).await;
        results.sort_by_key(|result| result.kind.as_str());
        Ok(results)
    }

    pub(crate) async fn shutdown(&self) -> Result<(), CommunicationsApplicationError> {
        let _lifecycles = futures_util::future::join_all(
            builtin_descriptors()
                .into_iter()
                .map(|descriptor| self.lifecycle.lock(descriptor.kind)),
        )
        .await;
        self.ports.transports.shutdown().await
    }

    pub(crate) fn claim_inbound(
        &self,
        connector: ConnectorKind,
        event_id: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        let event = InboundEventIdentity::new(connector, event_id)?;
        let received_at = self.ports.clock.now_rfc3339();
        self.ports.repository.claim_event(&event, &received_at)
    }

    pub(crate) fn maintain_deduplication(&self) -> Result<usize, CommunicationsApplicationError> {
        let now = std::time::Instant::now();
        let mut last = self.last_dedup_maintenance.lock().map_err(|_| {
            CommunicationsApplicationError::failure("dedup-maintenance-lock-failed")
        })?;
        if last.is_some_and(|last| now.duration_since(last) < DEDUP_MAINTENANCE_MIN_INTERVAL) {
            return Ok(0);
        }
        let cutoff = self.ports.clock.days_ago_rfc3339(DEDUP_RETENTION_DAYS);
        let removed = self
            .ports
            .repository
            .cleanup_dedup_before(&cutoff, DEDUP_MAINTENANCE_BATCH)?;
        *last = Some(now);
        Ok(removed)
    }

    pub(crate) fn route_inbound(
        &self,
        inbound: NormalizedInbound,
    ) -> Result<InboundRouteOutcome, CommunicationsApplicationError> {
        if inbound.disposition() != InboundDisposition::Deliver {
            return Ok(InboundRouteOutcome::Ignored);
        }
        let key = ChatBindingKey::new(inbound.connector, inbound.chat_id)?;
        let session_id = match self.ports.sessions.find(&key)? {
            Some(session_id) => session_id,
            None => {
                let routing = self.require_routing()?;
                self.ports.sessions.create_if_missing(&key, &routing)?
            }
        };
        let result = self.ports.agents.execute(AgentExecutionRequest {
            session_id: session_id.clone(),
            text: inbound.text,
        })?;
        Ok(InboundRouteOutcome::Reply {
            text: result.reply,
            session_id,
            message_id: result.message_id,
        })
    }

    pub(crate) fn reset_bindings(
        &self,
        kind: Option<ConnectorKind>,
    ) -> Result<(), CommunicationsApplicationError> {
        self.ports.sessions.reset(kind)
    }

    fn require_routing(&self) -> Result<RoutingSettings, CommunicationsApplicationError> {
        let routing = self.ports.repository.load_routing()?.ok_or_else(|| {
            CommunicationsApplicationError::user_visible(
                "routing-not-configured",
                "IM routing is not configured in VaneHub settings.",
            )
        })?;
        self.ports.agents.validate_routing(&routing)
    }

    fn load_runtime_definition(
        &self,
        kind: ConnectorKind,
    ) -> Result<ConnectorRuntimeDefinition, CommunicationsApplicationError> {
        let configuration = self
            .ports
            .repository
            .find_configuration(kind)?
            .unwrap_or_else(|| default_configuration(kind));
        configuration.validate()?;
        let credential = self
            .ports
            .credentials
            .load(kind)?
            .ok_or_else(credentials_required)?;
        Ok(runtime_definition(configuration, credential))
    }

    fn restore_credential(
        &self,
        kind: ConnectorKind,
        previous: Option<&ConnectorCredential>,
    ) -> Result<(), CommunicationsApplicationError> {
        match previous {
            Some(previous) => self
                .ports
                .credentials
                .store(kind, previous.secret.as_str())
                .map(|_| ()),
            None => self.ports.credentials.delete(kind),
        }
    }

    fn restore_configuration(
        &self,
        kind: ConnectorKind,
        previous: Option<&ConnectorConfig>,
    ) -> Result<(), CommunicationsApplicationError> {
        match previous {
            Some(previous) => self
                .ports
                .repository
                .save_configuration(previous, &self.ports.clock.now_rfc3339()),
            None => self.ports.repository.delete_configuration(kind),
        }
    }

    async fn restore_connector_snapshot(
        &self,
        kind: ConnectorKind,
        previous_configuration: Option<&ConnectorConfig>,
        previous_credential: Option<&ConnectorCredential>,
    ) -> Result<(), CommunicationsApplicationError> {
        let mut first_error = None;
        if let Err(error) = self.restore_credential(kind, previous_credential) {
            first_error = Some(error);
        }
        if let Err(error) = self.restore_configuration(kind, previous_configuration) {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
        let runtime_result = match (previous_configuration, previous_credential) {
            (Some(configuration), Some(credential)) if configuration.enabled => {
                self.ports
                    .transports
                    .replace_and_start(runtime_definition(
                        configuration.clone(),
                        credential.clone(),
                    ))
                    .await
            }
            _ => self.ports.transports.stop(kind).await,
        };
        if let Err(error) = runtime_result {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    fn record_lifecycle_rollback(
        &self,
        kind: ConnectorKind,
        primary: &CommunicationsApplicationError,
        rollback: Option<&CommunicationsApplicationError>,
    ) {
        self.record(
            CommunicationsLogLevel::Error,
            "communications.connector.lifecycle-primary",
            "Connector lifecycle mutation failed.",
            Some(kind),
            Some(primary.safe_code()),
            None,
        );
        self.record(
            if rollback.is_some() {
                CommunicationsLogLevel::Error
            } else {
                CommunicationsLogLevel::Info
            },
            "communications.connector.lifecycle-rollback",
            if rollback.is_some() {
                "Connector lifecycle rollback failed."
            } else {
                "Connector lifecycle rollback completed."
            },
            Some(kind),
            rollback.map(CommunicationsApplicationError::safe_code),
            None,
        );
    }

    fn finish_operation(
        &self,
        kind: ConnectorKind,
        action: &'static str,
        operation_id: &str,
        result: &Result<(), CommunicationsApplicationError>,
    ) {
        match result {
            Ok(()) => {
                let _ = self.ports.operations.complete(operation_id);
                self.record(
                    CommunicationsLogLevel::Info,
                    "communications.connector.operation",
                    format!("Connector {action} completed."),
                    Some(kind),
                    None,
                    Some(operation_id),
                );
            }
            Err(error) => {
                let _ = self.ports.operations.fail(operation_id, error.safe_code());
                self.record(
                    CommunicationsLogLevel::Error,
                    "communications.connector.operation",
                    format!("Connector {action} failed."),
                    Some(kind),
                    Some(error.safe_code()),
                    Some(operation_id),
                );
            }
        }
    }

    fn record(
        &self,
        level: CommunicationsLogLevel,
        event: &'static str,
        message: impl Into<String>,
        connector: Option<ConnectorKind>,
        safe_code: Option<&str>,
        operation_id: Option<&str>,
    ) {
        let _ = self.ports.logging.record(CommunicationsLog {
            level,
            event,
            message: message.into(),
            connector,
            safe_code: safe_code.map(str::to_string),
            operation_id: operation_id.map(str::to_string),
            timestamp: self.ports.clock.now_rfc3339(),
        });
    }
}

fn default_configuration(kind: ConnectorKind) -> ConnectorConfig {
    ConnectorConfig {
        kind,
        enabled: false,
        display_name: None,
        public_config: serde_json::json!({}),
        credential_ref: None,
    }
}

fn runtime_definition(
    configuration: ConnectorConfig,
    credential: ConnectorCredential,
) -> ConnectorRuntimeDefinition {
    ConnectorRuntimeDefinition {
        configuration,
        secret: credential.secret,
    }
}

fn credentials_required() -> CommunicationsApplicationError {
    CommunicationsApplicationError::failure("connector-credentials-required")
}

struct ConnectorCandidate {
    public_config: Value,
    secret: Option<Zeroizing<String>>,
}

fn prepare_connector_candidate(
    kind: ConnectorKind,
    public_config: Value,
    previous: Option<&ConnectorCredential>,
    patch: Option<&BTreeMap<String, String>>,
) -> Result<ConnectorCandidate, CommunicationsApplicationError> {
    let definitions = connector_field_definitions(kind);
    let mut public_config = match public_config {
        Value::Object(values) => values,
        _ => {
            return Err(CommunicationsApplicationError::failure(
                "public-config-invalid",
            ))
        }
    };
    let mut fields = decode_stored_fields(kind, previous)?;

    for definition in definitions {
        if definition.storage != ConnectorFieldStorage::Public {
            public_config.remove(definition.key);
            continue;
        }
        if let Some(value) = public_config.get(definition.key) {
            let value = value.as_str().ok_or_else(|| {
                CommunicationsApplicationError::failure(format!(
                    "credential-field-invalid-{}",
                    definition.key
                ))
            })?;
            if !value.trim().is_empty() {
                fields.insert(definition.key.to_string(), value.trim().to_string());
            }
        }
    }

    if let Some(patch) = patch {
        for (key, value) in patch {
            if !definitions.iter().any(|definition| definition.key == key) {
                return Err(CommunicationsApplicationError::failure(format!(
                    "credential-field-unknown-{key}"
                )));
            }
            let value = value.trim();
            if !value.is_empty() {
                fields.insert(key.clone(), value.to_string());
            }
        }
    }

    if previous.is_some() || patch.is_some() || !fields.is_empty() {
        for definition in definitions.iter().filter(|definition| definition.required) {
            if !fields
                .get(definition.key)
                .is_some_and(|value| !value.trim().is_empty())
            {
                return Err(CommunicationsApplicationError::failure(format!(
                    "credential-field-missing-{}",
                    definition.key
                )));
            }
        }
    }

    let mut secret_fields = BTreeMap::new();
    for definition in definitions {
        match definition.storage {
            ConnectorFieldStorage::Public => {
                if let Some(value) = fields.get(definition.key) {
                    public_config.insert(definition.key.to_string(), Value::String(value.clone()));
                }
            }
            ConnectorFieldStorage::Secret => {
                public_config.remove(definition.key);
                if let Some(value) = fields.get(definition.key) {
                    secret_fields.insert(definition.key.to_string(), value.clone());
                }
            }
        }
    }

    let secret = if secret_fields.is_empty() {
        None
    } else {
        Some(Zeroizing::new(
            serde_json::to_string(&secret_fields).map_err(|_| {
                CommunicationsApplicationError::failure("credential-payload-invalid")
            })?,
        ))
    };
    Ok(ConnectorCandidate {
        public_config: Value::Object(public_config),
        secret,
    })
}

fn decode_stored_fields(
    kind: ConnectorKind,
    previous: Option<&ConnectorCredential>,
) -> Result<BTreeMap<String, String>, CommunicationsApplicationError> {
    let Some(previous) = previous else {
        return Ok(BTreeMap::new());
    };
    if let Ok(fields) = serde_json::from_str::<BTreeMap<String, String>>(previous.secret.as_str()) {
        return Ok(fields);
    }
    let required_secrets = connector_field_definitions(kind)
        .iter()
        .filter(|field| field.required && field.storage == ConnectorFieldStorage::Secret)
        .collect::<Vec<_>>();
    if required_secrets.len() == 1 && !previous.secret.trim().is_empty() {
        return Ok(BTreeMap::from([(
            required_secrets[0].key.to_string(),
            previous.secret.to_string(),
        )]));
    }
    Err(CommunicationsApplicationError::failure(
        "credential-payload-invalid",
    ))
}
