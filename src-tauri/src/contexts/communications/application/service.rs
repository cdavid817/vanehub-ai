use super::{
    AgentExecutionRequest, CommunicationsAgentExecutionPort, CommunicationsApplicationError,
    CommunicationsClockPort, CommunicationsCredentialPort, CommunicationsLog,
    CommunicationsLogLevel, CommunicationsLoggingPort, CommunicationsOperationPort,
    CommunicationsRepository, CommunicationsSessionBindingPort, CommunicationsTransportPort,
    ConnectorCredential, ConnectorRuntimeDefinition, ConnectorStartupResult, ConnectorSummary,
    InboundRouteOutcome, SaveConnectorRequest,
};
use crate::contexts::communications::domain::{
    builtin_descriptors, ChatBindingKey, ConnectorConfig, ConnectorHealth, ConnectorKind,
    ConnectorLifecycle, InboundDisposition, InboundEventIdentity, NormalizedInbound,
    RoutingSettings,
};
use std::collections::HashMap;
use std::sync::Arc;

const DEDUP_RETENTION_DAYS: u32 = 7;

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
}

impl CommunicationsApplicationService {
    pub(crate) fn new(ports: CommunicationsApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) async fn list_connectors(
        &self,
    ) -> Result<Vec<ConnectorSummary>, CommunicationsApplicationError> {
        let configurations = self
            .ports
            .repository
            .list_configurations()?
            .into_iter()
            .map(|configuration| (configuration.kind, configuration))
            .collect::<HashMap<_, _>>();
        let health = self
            .ports
            .transports
            .health()
            .await
            .into_iter()
            .map(|health| (health.kind, health))
            .collect::<HashMap<_, _>>();
        let now = self.ports.clock.now_rfc3339();
        builtin_descriptors()
            .into_iter()
            .map(|descriptor| {
                let kind = descriptor.kind;
                let configuration = configurations
                    .get(&kind)
                    .cloned()
                    .unwrap_or_else(|| default_configuration(kind));
                let has_credentials = self.ports.credentials.load(kind)?.is_some();
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
                Ok(ConnectorSummary {
                    descriptor,
                    configuration,
                    health: connector_health,
                    has_credentials,
                })
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
        let previous_credential = self.ports.credentials.load(request.kind)?;
        let credential_changed = request.replacement_secret.is_some();
        let mut configuration = ConnectorConfig {
            kind: request.kind,
            enabled: request.enabled,
            display_name: request.display_name,
            public_config: request.public_config,
            credential_ref: previous_credential
                .as_ref()
                .map(|credential| credential.reference.clone()),
        };
        configuration.validate()?;
        if configuration.enabled {
            self.require_routing()?;
            if previous_credential.is_none() && request.replacement_secret.is_none() {
                return Err(credentials_required());
            }
        }

        self.ports.transports.stop(request.kind).await?;
        let replacement = match request.replacement_secret {
            Some(secret) => Some(self.ports.credentials.store(request.kind, &secret)?),
            None => None,
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
        if configuration.enabled {
            let credential = replacement
                .or(previous_credential)
                .ok_or_else(credentials_required)?;
            self.ports
                .transports
                .start(runtime_definition(configuration.clone(), credential))
                .await?;
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
        let mut configuration = self
            .ports
            .repository
            .find_configuration(kind)?
            .unwrap_or_else(|| default_configuration(kind));
        let credential = self.ports.credentials.load(kind)?;
        if enabled {
            self.require_routing()?;
            if credential.is_none() {
                return Err(credentials_required());
            }
        } else {
            self.ports.transports.stop(kind).await?;
        }
        configuration.enabled = enabled;
        configuration.credential_ref = credential
            .as_ref()
            .map(|credential| credential.reference.clone());
        let updated_at = self.ports.clock.now_rfc3339();
        self.ports
            .repository
            .save_configuration(&configuration, &updated_at)?;
        if enabled {
            self.ports
                .transports
                .start(runtime_definition(
                    configuration,
                    credential.ok_or_else(credentials_required)?,
                ))
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn clear_connector(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
        self.ports.transports.stop(kind).await?;
        let previous_configuration = self.ports.repository.find_configuration(kind)?;
        let previous_credential = self.ports.credentials.load(kind)?;
        self.ports.credentials.delete(kind)?;
        if let Some(mut configuration) = previous_configuration {
            configuration.enabled = false;
            configuration.credential_ref = None;
            let updated_at = self.ports.clock.now_rfc3339();
            if let Err(error) = self
                .ports
                .repository
                .save_configuration(&configuration, &updated_at)
            {
                self.restore_credential(kind, previous_credential.as_ref())?;
                return Err(error);
            }
        }
        Ok(())
    }

    pub(crate) async fn test_connector(
        &self,
        kind: ConnectorKind,
    ) -> Result<(), CommunicationsApplicationError> {
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
        self.require_routing()?;
        let operation = self.ports.operations.start(kind, "restart")?;
        let result = match self.load_runtime_definition(kind) {
            Ok(definition) => match self.ports.transports.stop(kind).await {
                Ok(()) => self.ports.transports.start(definition).await,
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        self.finish_operation(kind, "restart", &operation.id, &result);
        result
    }

    pub(crate) async fn start_saved_connectors(
        &self,
    ) -> Result<Vec<ConnectorStartupResult>, CommunicationsApplicationError> {
        let configurations = self.ports.repository.list_configurations()?;
        if configurations
            .iter()
            .any(|configuration| configuration.enabled)
        {
            self.require_routing()?;
        }
        let mut results = Vec::new();
        for configuration in configurations
            .into_iter()
            .filter(|configuration| configuration.enabled)
        {
            let kind = configuration.kind;
            let result = match self.ports.credentials.load(kind)? {
                Some(credential) => {
                    self.ports
                        .transports
                        .start(runtime_definition(configuration, credential))
                        .await
                }
                None => Err(credentials_required()),
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
            results.push(ConnectorStartupResult {
                kind,
                safe_error_code: result.err().map(|error| error.safe_code().to_string()),
            });
        }
        Ok(results)
    }

    pub(crate) async fn shutdown(&self) -> Result<(), CommunicationsApplicationError> {
        self.ports.transports.shutdown().await
    }

    pub(crate) fn claim_inbound(
        &self,
        connector: ConnectorKind,
        event_id: &str,
    ) -> Result<bool, CommunicationsApplicationError> {
        let event = InboundEventIdentity::new(connector, event_id)?;
        let received_at = self.ports.clock.now_rfc3339();
        let claimed = self.ports.repository.claim_event(&event, &received_at)?;
        if claimed {
            let cutoff = self.ports.clock.days_ago_rfc3339(DEDUP_RETENTION_DAYS);
            let _ = self.ports.repository.cleanup_dedup_before(&cutoff);
        }
        Ok(claimed)
    }

    pub(crate) fn route_inbound(
        &self,
        inbound: NormalizedInbound,
    ) -> Result<InboundRouteOutcome, CommunicationsApplicationError> {
        if inbound.disposition() != InboundDisposition::Deliver {
            return Ok(InboundRouteOutcome::Ignored);
        }
        let routing = self.require_routing()?;
        let key = ChatBindingKey::new(inbound.connector, inbound.chat_id)?;
        let session_id = self.ports.sessions.resolve_or_create(&key, &routing)?;
        let result = self.ports.agents.execute(AgentExecutionRequest {
            session_id: session_id.clone(),
            text: inbound.text,
            routing,
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
