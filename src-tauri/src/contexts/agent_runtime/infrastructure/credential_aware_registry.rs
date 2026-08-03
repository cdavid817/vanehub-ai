use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentRegistryRepository,
    AgentRuntimeApplicationError, ApiCredentialPort,
};
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AvailabilityAssessment,
};
use std::sync::Arc;

pub(crate) struct CredentialAwareAgentRegistry {
    inner: Arc<dyn AgentRegistryRepository>,
    credentials: Arc<dyn ApiCredentialPort>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
}

impl CredentialAwareAgentRegistry {
    pub(crate) fn new(
        inner: Arc<dyn AgentRegistryRepository>,
        credentials: Arc<dyn ApiCredentialPort>,
        logging: Arc<dyn AgentLoggingPort>,
        clock: Arc<dyn AgentClockPort>,
    ) -> Self {
        Self {
            inner,
            credentials,
            logging,
            clock,
        }
    }

    fn decorate(&self, agent: AgentDefinition) -> AgentDefinition {
        if agent.launch().kind_str() != "api"
            || agent.availability().state() == AgentAvailability::Unavailable
        {
            return agent;
        }
        match self.credentials.fetch(agent.id().as_str()) {
            Ok(Some(_)) => agent.with_availability(AvailabilityAssessment::new(
                AgentAvailability::Available,
                None,
            )),
            Ok(None) => agent.with_availability(AvailabilityAssessment::new(
                AgentAvailability::NeedsAuthentication,
                Some("API agent requires a provider credential.".to_string()),
            )),
            Err(_) => {
                let _ = self.logging.record(AgentLog {
                    level: AgentLogLevel::Warn,
                    category: "session.runtime.api.credentials".to_string(),
                    message: "Credential availability could not be inspected; the agent was made non-selectable.".to_string(),
                    agent_id: Some(agent.id().as_str().to_string()),
                    session_id: None,
                    operation_id: None,
                    run_id: None,
                    trace_id: None,
                    span_id: None,
                    occurred_at: self.clock.now(),
                });
                agent.with_availability(AvailabilityAssessment::new(
                    AgentAvailability::Unavailable,
                    Some("Credential availability could not be determined.".to_string()),
                ))
            }
        }
    }
}

impl AgentRegistryRepository for CredentialAwareAgentRegistry {
    fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
        self.inner.list().map(|agents| {
            agents
                .into_iter()
                .map(|agent| self.decorate(agent))
                .collect()
        })
    }

    fn find(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
        self.inner
            .find(agent_id)
            .map(|agent| agent.map(|agent| self.decorate(agent)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::{
        AgentDefinitionInput, AgentOrigin, InteractionMode, LaunchMetadata,
    };
    use std::sync::Mutex;

    struct Registry(Vec<AgentDefinition>);

    impl AgentRegistryRepository for Registry {
        fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
            Ok(self.0.clone())
        }

        fn find(
            &self,
            agent_id: &str,
        ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
            Ok(self
                .0
                .iter()
                .find(|agent| agent.id().as_str() == agent_id)
                .cloned())
        }
    }

    struct Credentials(Result<Option<String>, AgentRuntimeApplicationError>);

    impl ApiCredentialPort for Credentials {
        fn store(
            &self,
            _agent_id: &str,
            _api_key: &str,
        ) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }

        fn fetch(&self, _agent_id: &str) -> Result<Option<String>, AgentRuntimeApplicationError> {
            self.0.clone()
        }

        fn remove(&self, _agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
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
            "2026-08-02T00:00:00Z".to_string()
        }
    }

    fn agent(id: &str, kind: &str, availability: AgentAvailability) -> AgentDefinition {
        AgentDefinition::new_with_origin(
            AgentDefinitionInput {
                id: id.to_string(),
                display_name: id.to_string(),
                provider: "provider".to_string(),
                managed_sdk_dependency_id: None,
                launch: LaunchMetadata::new(kind.to_string(), None, None, None).expect("launch"),
                supported_interaction_modes: vec![if kind == "api" {
                    InteractionMode::Api
                } else {
                    InteractionMode::Cli
                }],
                availability: AvailabilityAssessment::new(availability, None),
                capability_tags: Vec::new(),
            },
            if id == "onepiece" {
                AgentOrigin::Builtin
            } else {
                AgentOrigin::User
            },
        )
        .expect("agent")
    }

    fn decorated(
        agents: Vec<AgentDefinition>,
        credential: Result<Option<String>, AgentRuntimeApplicationError>,
        logs: Arc<Logs>,
    ) -> CredentialAwareAgentRegistry {
        CredentialAwareAgentRegistry::new(
            Arc::new(Registry(agents)),
            Arc::new(Credentials(credential)),
            logs,
            Arc::new(Clock),
        )
    }

    #[test]
    fn complete_api_agents_are_available_only_with_a_credential() {
        let logs = Arc::new(Logs::default());
        let missing = decorated(
            vec![agent("onepiece", "api", AgentAvailability::Available)],
            Ok(None),
            logs.clone(),
        )
        .find("onepiece")
        .expect("find")
        .expect("agent");
        assert_eq!(
            missing.availability().state(),
            AgentAvailability::NeedsAuthentication
        );

        let present = decorated(
            vec![agent("onepiece", "api", AgentAvailability::Available)],
            Ok(Some("secret-never-logged".to_string())),
            logs,
        )
        .find("onepiece")
        .expect("find")
        .expect("agent");
        assert_eq!(present.availability().state(), AgentAvailability::Available);
    }

    #[test]
    fn credential_failure_degrades_only_api_agents_and_logs_no_secret() {
        let logs = Arc::new(Logs::default());
        let agents = decorated(
            vec![
                agent("onepiece", "api", AgentAvailability::Available),
                agent("codex-cli", "cli", AgentAvailability::Available),
            ],
            Err(AgentRuntimeApplicationError::Credential(
                "secret-value".to_string(),
            )),
            logs.clone(),
        )
        .list()
        .expect("list");
        assert_eq!(
            agents[0].availability().state(),
            AgentAvailability::Unavailable
        );
        assert_eq!(
            agents[1].availability().state(),
            AgentAvailability::Available
        );
        let recorded = logs.0.lock().expect("logs");
        assert_eq!(recorded.len(), 1);
        assert!(!recorded[0].message.contains("secret-value"));
    }

    #[test]
    fn structurally_incomplete_api_agent_remains_unavailable() {
        let logs = Arc::new(Logs::default());
        let value = decorated(
            vec![agent("onepiece", "api", AgentAvailability::Unavailable)],
            Ok(Some("credential".to_string())),
            logs,
        )
        .find("onepiece")
        .expect("find")
        .expect("agent");
        assert_eq!(value.availability().state(), AgentAvailability::Unavailable);
    }
}
