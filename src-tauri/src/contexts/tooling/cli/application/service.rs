use super::{
    CliApplicationError, CliClockPort, CliDetectionPort, CliExecutableLocatorPort, CliLogCategory,
    CliLogEvent, CliLogLevel, CliLoggingPort, CliOperationPort, CliOperationRequest,
    CliOperationResult, CliOperationType, CliStatusRepository, PreparedCliRefresh,
};
use crate::contexts::tooling::cli::domain::{definition, ToolDefinition, CLI_TOOL_DEFINITIONS};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct CliApplicationService {
    repository: Arc<dyn CliStatusRepository>,
    detection: Arc<dyn CliDetectionPort>,
    executable_locator: Arc<dyn CliExecutableLocatorPort>,
    operations: Arc<dyn CliOperationPort>,
    logging: Arc<dyn CliLoggingPort>,
    clock: Arc<dyn CliClockPort>,
}

pub(crate) struct CliApplicationPorts {
    pub(crate) repository: Arc<dyn CliStatusRepository>,
    pub(crate) detection: Arc<dyn CliDetectionPort>,
    pub(crate) executable_locator: Arc<dyn CliExecutableLocatorPort>,
    pub(crate) operations: Arc<dyn CliOperationPort>,
    pub(crate) logging: Arc<dyn CliLoggingPort>,
    pub(crate) clock: Arc<dyn CliClockPort>,
}

impl CliApplicationService {
    pub(crate) fn new(ports: CliApplicationPorts) -> Self {
        Self {
            repository: ports.repository,
            detection: ports.detection,
            executable_locator: ports.executable_locator,
            operations: ports.operations,
            logging: ports.logging,
            clock: ports.clock,
        }
    }

    pub(crate) fn needs_initial_refresh(&self) -> Result<bool, CliApplicationError> {
        self.repository
            .has_cached_statuses()
            .map(|has_cached_statuses| !has_cached_statuses)
    }

    pub(crate) fn resolve_executable(
        &self,
        agent_id: &str,
    ) -> Result<Option<String>, CliApplicationError> {
        let definition = definition(agent_id).ok_or_else(|| unsupported_agent_error(agent_id))?;
        let status = self.repository.load(definition)?;
        Ok(self
            .executable_locator
            .resolve(definition, status.detected_path.as_deref()))
    }

    pub(crate) fn prepare_refresh(
        &self,
        agent_id: Option<String>,
        message: String,
    ) -> Result<PreparedCliRefresh, CliApplicationError> {
        if let Some(agent_id) = agent_id.as_deref() {
            definition(agent_id).ok_or_else(|| unsupported_agent_error(agent_id))?;
        }
        let operation = self.operations.start(&CliOperationRequest {
            operation_type: CliOperationType::Refresh,
            related_agent_id: agent_id.clone(),
            message,
        })?;
        Ok(PreparedCliRefresh {
            operation,
            agent_id,
        })
    }

    pub(crate) fn execute_refresh(
        &self,
        prepared: PreparedCliRefresh,
    ) -> Result<(), CliApplicationError> {
        let operation_id = prepared.operation.id;
        self.emit_log(
            &operation_id,
            None,
            CliLogLevel::Info,
            "Starting CLI detection refresh.",
        );
        let definitions = CLI_TOOL_DEFINITIONS.into_iter().filter(|definition| {
            prepared
                .agent_id
                .as_deref()
                .is_none_or(|agent_id| agent_id == definition.agent_id)
        });
        let mut refreshed = Vec::new();
        let mut failed = Vec::new();

        for definition in definitions {
            self.emit_log(
                &operation_id,
                Some(definition.agent_id),
                CliLogLevel::Info,
                format!(
                    "Checking {} ({})",
                    definition.display_name, definition.executable_name
                ),
            );
            match self.detect_and_save(definition, &operation_id) {
                Ok(warnings) => {
                    if warnings.is_empty() {
                        self.emit_log(
                            &operation_id,
                            Some(definition.agent_id),
                            CliLogLevel::Info,
                            format!("{} detection succeeded.", definition.display_name),
                        );
                    } else {
                        self.emit_log(
                            &operation_id,
                            Some(definition.agent_id),
                            CliLogLevel::Warn,
                            format!(
                                "{} refresh completed with warnings: {}",
                                definition.display_name,
                                warnings.join("; ")
                            ),
                        );
                    }
                    refreshed.push(definition.agent_id.to_string());
                }
                Err(error) => {
                    self.emit_log(
                        &operation_id,
                        Some(definition.agent_id),
                        CliLogLevel::Error,
                        format!("Failed to persist CLI detection result: {error}"),
                    );
                    failed.push(definition.agent_id.to_string());
                }
            }
        }

        self.emit_log(
            &operation_id,
            None,
            CliLogLevel::Info,
            "CLI detection refresh finished.",
        );
        self.operations.complete(
            &operation_id,
            &CliOperationResult::Refresh {
                agent_ids: refreshed,
                failed,
            },
        )
    }

    fn detect_and_save(
        &self,
        definition: ToolDefinition,
        operation_id: &str,
    ) -> Result<Vec<String>, CliApplicationError> {
        let mut detection = self.detection.detect(definition, operation_id)?;
        for event in detection.events {
            self.publish_log(event);
        }
        detection
            .status
            .associate_detection(operation_id, self.clock.now());
        self.repository.save(&detection.status)?;
        Ok(detection.warnings)
    }

    fn emit_log(
        &self,
        operation_id: &str,
        agent_id: Option<&str>,
        level: CliLogLevel,
        message: impl Into<String>,
    ) {
        let event = CliLogEvent {
            operation_id: operation_id.to_string(),
            agent_id: agent_id.map(str::to_string),
            level,
            category: CliLogCategory::Operation,
            message: message.into(),
            context: Default::default(),
        };
        self.publish_log(event);
    }

    fn publish_log(&self, event: CliLogEvent) {
        if event.category == CliLogCategory::Operation {
            let _ = self.operations.append_log(&event);
        }
        let _ = self.logging.record(&event);
    }
}

fn unsupported_agent_error(agent_id: &str) -> CliApplicationError {
    CliApplicationError::Validation(format!("unsupported CLI agent id: {agent_id}"))
}
