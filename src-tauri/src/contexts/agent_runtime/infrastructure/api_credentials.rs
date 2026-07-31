use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, ApiCredentialPort,
};
use crate::platform::credentials::OsCredentialStore;

const SERVICE_NAME: &str = "vanehub-api-agent";

#[derive(Clone)]
pub(crate) struct OsApiCredentialAdapter {
    store: OsCredentialStore,
}

impl OsApiCredentialAdapter {
    pub(crate) fn new() -> Self {
        Self {
            store: OsCredentialStore::new(SERVICE_NAME),
        }
    }
}

impl Default for OsApiCredentialAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiCredentialPort for OsApiCredentialAdapter {
    fn store(&self, agent_id: &str, api_key: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.store.set(agent_id, api_key).map_err(credential_error)
    }

    fn fetch(&self, agent_id: &str) -> Result<Option<String>, AgentRuntimeApplicationError> {
        self.store
            .get(agent_id)
            .map(|value| value.map(|secret| secret.as_str().to_string()))
            .map_err(credential_error)
    }

    fn remove(&self, agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.store.delete(agent_id).map_err(credential_error)
    }
}

fn credential_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Credential(error.to_string())
}
