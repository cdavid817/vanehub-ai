use crate::contexts::operations::api::{
    DiagnosticLog, DiagnosticLogPort, LogSeverity, OperationKind, OperationLog, OperationLogPort,
    OperationsApi,
};
use crate::contexts::tooling::cli::application::{
    CliApplicationError, CliClockPort, CliLogCategory, CliLogEvent, CliLogLevel, CliLoggingPort,
    CliMutationPort, CliOperationPort, CliOperationRequest, CliOperationResult,
    StartedCliOperation,
};
use crate::contexts::tooling::cli::domain::MutationClaims;
use crate::platform::clock::SystemClock;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemCliClock;

impl CliClockPort for SystemCliClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}

#[derive(Clone)]
pub(crate) struct CliOperationAdapter {
    operations: OperationsApi,
}

impl CliOperationAdapter {
    pub(crate) fn new(operations: OperationsApi) -> Self {
        Self { operations }
    }
}

impl CliOperationPort for CliOperationAdapter {
    fn start(
        &self,
        request: &CliOperationRequest,
    ) -> Result<StartedCliOperation, CliApplicationError> {
        self.operations
            .start(
                OperationKind::Agent,
                request.related_agent_id.clone(),
                Some(request.message.clone()),
            )
            .map(|operation| StartedCliOperation {
                id: operation.id,
                related_entity_id: operation.related_entity_id,
                message: operation.message,
                created_at: operation.created_at,
                updated_at: operation.updated_at,
            })
            .map_err(operation_error)
    }

    fn append_log(&self, event: &CliLogEvent) -> Result<(), CliApplicationError> {
        self.operations
            .append_log(&event.operation_id, event.message.clone())
            .map(|_| ())
            .map_err(operation_error)
    }

    fn complete(
        &self,
        operation_id: &str,
        result: &CliOperationResult,
    ) -> Result<(), CliApplicationError> {
        let payload = serde_json::to_value(OperationPayload::from(result))
            .map_err(|error| CliApplicationError::Validation(error.to_string()))?;
        self.operations
            .complete(operation_id, Some(payload))
            .map(|_| ())
            .map_err(operation_error)
    }

    fn fail(&self, operation_id: &str, error: String) -> Result<(), CliApplicationError> {
        self.operations
            .fail(operation_id, error)
            .map(|_| ())
            .map_err(operation_error)
    }
}

#[derive(Clone)]
pub(crate) struct UnifiedCliLoggingAdapter {
    diagnostics: Arc<dyn DiagnosticLogPort>,
    operations: Arc<dyn OperationLogPort>,
}

impl UnifiedCliLoggingAdapter {
    pub(crate) fn new(
        diagnostics: Arc<dyn DiagnosticLogPort>,
        operations: Arc<dyn OperationLogPort>,
    ) -> Self {
        Self {
            diagnostics,
            operations,
        }
    }
}

impl CliLoggingPort for UnifiedCliLoggingAdapter {
    fn record(&self, event: &CliLogEvent) -> Result<(), CliApplicationError> {
        let severity = log_severity(event.level);
        let mut context = event.context.clone();
        if let Some(agent_id) = &event.agent_id {
            context.insert("agentId".to_string(), agent_id.clone());
        }
        match event.category {
            CliLogCategory::Operation => self
                .operations
                .write_operation(OperationLog {
                    operation_id: event.operation_id.clone(),
                    severity,
                    category: "cli.operation".to_string(),
                    message: event.message.clone(),
                    context,
                })
                .map_err(logging_error),
            CliLogCategory::Diagnostic => {
                context.insert("operationId".to_string(), event.operation_id.clone());
                self.diagnostics
                    .write_diagnostic(DiagnosticLog {
                        severity,
                        category: "cli.diagnostic".to_string(),
                        message: event.message.clone(),
                        context,
                    })
                    .map_err(logging_error)
            }
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct CliMutationAdapter {
    claims: Arc<Mutex<MutationClaims>>,
}

impl CliMutationPort for CliMutationAdapter {
    fn try_acquire(&self, agent_id: &str) -> Result<bool, CliApplicationError> {
        self.claims
            .lock()
            .map(|mut claims| claims.try_acquire(agent_id))
            .map_err(lock_error)
    }

    fn release(&self, agent_id: &str) -> Result<(), CliApplicationError> {
        self.claims
            .lock()
            .map(|mut claims| claims.release(agent_id))
            .map_err(lock_error)
    }

    fn try_acquire_many(&self, agent_ids: &[String]) -> Result<Vec<String>, CliApplicationError> {
        self.claims
            .lock()
            .map(|mut claims| claims.try_acquire_many(agent_ids.iter().map(String::as_str)))
            .map_err(lock_error)
    }

    fn release_many(&self, agent_ids: &[String]) -> Result<(), CliApplicationError> {
        self.claims
            .lock()
            .map(|mut claims| claims.release_many(agent_ids.iter().map(String::as_str)))
            .map_err(lock_error)
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum OperationPayload<'a> {
    Refresh {
        #[serde(rename = "agentIds")]
        agent_ids: &'a [String],
        failed: &'a [String],
    },
    Install {
        #[serde(rename = "agentId")]
        agent_id: &'a str,
        #[serde(rename = "targetVersion")]
        target_version: &'a str,
    },
    UpgradeAll {
        upgraded: &'a [String],
        skipped: &'a [String],
        failed: &'a [String],
    },
}

impl<'a> From<&'a CliOperationResult> for OperationPayload<'a> {
    fn from(result: &'a CliOperationResult) -> Self {
        match result {
            CliOperationResult::Refresh { agent_ids, failed } => {
                Self::Refresh { agent_ids, failed }
            }
            CliOperationResult::Install {
                agent_id,
                target_version,
            } => Self::Install {
                agent_id,
                target_version,
            },
            CliOperationResult::UpgradeAll {
                upgraded,
                skipped,
                failed,
            } => Self::UpgradeAll {
                upgraded,
                skipped,
                failed,
            },
        }
    }
}

fn log_severity(level: CliLogLevel) -> LogSeverity {
    match level {
        CliLogLevel::Error => LogSeverity::Error,
        CliLogLevel::Warn => LogSeverity::Warn,
        CliLogLevel::Info => LogSeverity::Info,
        CliLogLevel::Debug => LogSeverity::Debug,
    }
}

fn operation_error(error: impl std::fmt::Display) -> CliApplicationError {
    CliApplicationError::Operation(error.to_string())
}

fn logging_error(error: impl std::fmt::Display) -> CliApplicationError {
    CliApplicationError::Logging(error.to_string())
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> CliApplicationError {
    CliApplicationError::Internal("CLI mutation state is unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::domain::OperationStatus;
    use crate::contexts::operations::infrastructure::{operation_service, UnifiedLoggingAdapter};
    use crate::test_support::TempDirectory;
    use std::collections::BTreeMap;

    #[test]
    fn operation_adapter_preserves_cli_terminal_payload_contracts() {
        let service = operation_service();
        let adapter = CliOperationAdapter::new(OperationsApi::new(service.clone()));
        let started = adapter
            .start(&CliOperationRequest {
                operation_type:
                    crate::contexts::tooling::cli::application::CliOperationType::Refresh,
                related_agent_id: None,
                message: "Refreshing".to_string(),
            })
            .expect("start");

        adapter
            .complete(
                &started.id,
                &CliOperationResult::Refresh {
                    agent_ids: vec!["codex-cli".to_string()],
                    failed: vec!["opencode".to_string()],
                },
            )
            .expect("complete");

        let operation = service.get(&started.id).expect("operation");
        assert_eq!(operation.status, OperationStatus::Succeeded);
        assert_eq!(
            operation.result,
            Some(serde_json::json!({
                "agentIds": ["codex-cli"],
                "failed": ["opencode"]
            }))
        );
    }

    #[test]
    fn mutation_adapter_serializes_claims_across_clones() {
        let first = CliMutationAdapter::default();
        let second = first.clone();

        assert!(first.try_acquire("codex-cli").expect("first claim"));
        assert!(!second.try_acquire("codex-cli").expect("second claim"));
        second.release("codex-cli").expect("release");
        assert!(first.try_acquire("codex-cli").expect("claim again"));
    }

    #[test]
    fn logging_adapter_preserves_categories_operation_id_and_redaction() {
        let directory = TempDirectory::new("cli-runtime-logging");
        let logging = Arc::new(UnifiedLoggingAdapter::new(directory.path().to_path_buf()));
        let adapter = UnifiedCliLoggingAdapter::new(logging.clone(), logging);
        adapter
            .record(&CliLogEvent {
                operation_id: "op-cli-1".to_string(),
                agent_id: Some("codex-cli".to_string()),
                level: CliLogLevel::Info,
                category: CliLogCategory::Operation,
                message: "installed token=operation-secret".to_string(),
                context: BTreeMap::new(),
            })
            .expect("operation log");
        adapter
            .record(&CliLogEvent {
                operation_id: "op-cli-1".to_string(),
                agent_id: Some("codex-cli".to_string()),
                level: CliLogLevel::Warn,
                category: CliLogCategory::Diagnostic,
                message: "probe failed password=diagnostic-secret".to_string(),
                context: BTreeMap::new(),
            })
            .expect("diagnostic log");

        let raw = std::fs::read_to_string(
            directory
                .path()
                .join(crate::platform::logging::LOG_FILE_NAME),
        )
        .expect("log file");

        assert!(raw.contains("\"category\":\"cli.operation\""));
        assert!(raw.contains("\"category\":\"cli.diagnostic\""));
        assert!(raw.contains("\"operationId\":\"op-cli-1\""));
        assert!(raw.contains("\"agentId\":\"codex-cli\""));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("operation-secret"));
        assert!(!raw.contains("diagnostic-secret"));
    }
}
