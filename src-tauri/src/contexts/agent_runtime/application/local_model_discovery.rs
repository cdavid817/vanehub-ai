use super::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentRuntimeApplicationError,
    AgentTaskPort, LocalEndpointVerificationRequest, LocalModelDiscoveryPort,
    LocalModelDiscoveryResult, LocalModelEndpointCandidate,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct LocalModelDiscoveryService {
    discovery: Arc<dyn LocalModelDiscoveryPort>,
    operations: Arc<dyn AgentTaskPort>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
}

impl LocalModelDiscoveryService {
    pub(crate) fn new(
        discovery: Arc<dyn LocalModelDiscoveryPort>,
        operations: Arc<dyn AgentTaskPort>,
        logging: Arc<dyn AgentLoggingPort>,
        clock: Arc<dyn AgentClockPort>,
    ) -> Self {
        Self {
            discovery,
            operations,
            logging,
            clock,
        }
    }

    pub(crate) fn discover_loopback(
        &self,
    ) -> Result<LocalModelDiscoveryResult, AgentRuntimeApplicationError> {
        let operation = self
            .operations
            .start_agent_launch("onepiece", "Discovering loopback model endpoints")?;
        match self.discovery.discover_loopback() {
            Ok(endpoints) => {
                self.operations.append_log(
                    &operation.id,
                    format!("discovery-complete endpoints={}", endpoints.len()),
                )?;
                self.record("local-model-discovery-complete", &operation.id)?;
                self.operations.complete(&operation.id)?;
                Ok(LocalModelDiscoveryResult {
                    operation_id: operation.id,
                    endpoints,
                })
            }
            Err(error) => {
                self.record("local-model-discovery-failed", &operation.id)?;
                self.operations
                    .fail(&operation.id, "Local model discovery failed".to_string())?;
                Err(error)
            }
        }
    }

    pub(crate) fn verify_endpoint(
        &self,
        request: LocalEndpointVerificationRequest,
    ) -> Result<(String, LocalModelEndpointCandidate), AgentRuntimeApplicationError> {
        let operation = self
            .operations
            .start_agent_launch("onepiece", "Verifying model endpoint metadata")?;
        match self.discovery.verify_endpoint(request) {
            Ok(endpoint) => {
                self.record("local-model-verification-complete", &operation.id)?;
                self.operations.complete(&operation.id)?;
                Ok((operation.id, endpoint))
            }
            Err(error) => {
                self.record("local-model-verification-failed", &operation.id)?;
                self.operations.fail(
                    &operation.id,
                    "Model endpoint verification failed".to_string(),
                )?;
                Err(error)
            }
        }
    }

    fn record(
        &self,
        category: &str,
        operation_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.logging.record(AgentLog {
            level: AgentLogLevel::Info,
            category: category.to_string(),
            message: category.to_string(),
            agent_id: Some("onepiece".to_string()),
            session_id: None,
            operation_id: Some(operation_id.to_string()),
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: self.clock.now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{
        AgentOperation, LocalModelEndpointCandidate, OnePieceDiscoveredModel,
    };
    use std::sync::Mutex;

    struct Discovery;

    impl LocalModelDiscoveryPort for Discovery {
        fn discover_loopback(
            &self,
        ) -> Result<Vec<LocalModelEndpointCandidate>, AgentRuntimeApplicationError> {
            Ok(vec![LocalModelEndpointCandidate {
                service_kind: "ollama".to_string(),
                base_url: "http://127.0.0.1:11434".to_string(),
                models: vec![OnePieceDiscoveredModel {
                    id: "SECRET_MODEL_ID".to_string(),
                    display_name: "SECRET_MODEL_ID".to_string(),
                }],
                latency_bucket: "under-100ms".to_string(),
            }])
        }

        fn verify_endpoint(
            &self,
            _request: LocalEndpointVerificationRequest,
        ) -> Result<LocalModelEndpointCandidate, AgentRuntimeApplicationError> {
            self.discover_loopback().map(|mut values| values.remove(0))
        }
    }

    #[derive(Default)]
    struct Operations {
        lines: Mutex<Vec<String>>,
    }

    impl AgentTaskPort for Operations {
        fn start_agent_launch(
            &self,
            agent_id: &str,
            message: &str,
        ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
            Ok(AgentOperation {
                id: "operation-1".to_string(),
                related_agent_id: Some(agent_id.to_string()),
                message: Some(message.to_string()),
            })
        }

        fn start_agent_generation(
            &self,
            _agent_id: &str,
            _session_id: &str,
            _message_id: &str,
        ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
            unreachable!("discovery never starts generation")
        }

        fn append_log(
            &self,
            _operation_id: &str,
            line: String,
        ) -> Result<(), AgentRuntimeApplicationError> {
            self.lines.lock().expect("lines").push(line);
            Ok(())
        }

        fn complete(&self, _operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }

        fn fail(
            &self,
            _operation_id: &str,
            _error: String,
        ) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }

        fn cancel(&self, _operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct Logs(Mutex<Vec<AgentLog>>);

    impl AgentLoggingPort for Logs {
        fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
            self.0.lock().expect("logs").push(log);
            Ok(())
        }
    }

    struct Clock;

    impl AgentClockPort for Clock {
        fn now(&self) -> String {
            "2026-08-17T00:00:00Z".to_string()
        }
    }

    #[test]
    fn operation_and_diagnostics_contain_only_safe_metadata() {
        let operations = Arc::new(Operations::default());
        let logs = Arc::new(Logs::default());
        let service = LocalModelDiscoveryService::new(
            Arc::new(Discovery),
            operations.clone(),
            logs.clone(),
            Arc::new(Clock),
        );
        let result = service.discover_loopback().expect("discovery");
        assert_eq!(result.operation_id, "operation-1");
        let serialized = format!(
            "{:?}{:?}",
            operations.lines.lock().expect("lines"),
            logs.0.lock().expect("logs")
        );
        assert!(!serialized.contains("http://"));
        assert!(!serialized.contains("SECRET_MODEL_ID"));
        assert!(!serialized.contains("SECRET_SOURCE_MARKER"));
        assert!(serialized.contains("local-model-discovery-complete"));
    }
}
