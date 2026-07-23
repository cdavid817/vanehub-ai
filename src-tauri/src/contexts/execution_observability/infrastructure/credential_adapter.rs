use crate::contexts::execution_observability::application::{
    ExecutionTelemetryError, ObservabilityCredentialPort,
};
use crate::platform::credentials::OsCredentialStore;

const OTLP_AUTH_ACCOUNT: &str = "execution-observability-otlp-auth";

#[derive(Debug, Clone)]
pub(crate) struct OsObservabilityCredentialAdapter {
    store: OsCredentialStore,
}

impl OsObservabilityCredentialAdapter {
    pub(crate) fn new() -> Self {
        Self {
            store: OsCredentialStore::new("io.vanehub.ai"),
        }
    }

    pub(crate) fn load_otlp_auth(
        &self,
    ) -> Result<Option<zeroize::Zeroizing<String>>, ExecutionTelemetryError> {
        self.store
            .get(OTLP_AUTH_ACCOUNT)
            .map_err(|error| ExecutionTelemetryError::Unavailable(error.to_string()))
    }
}

impl ObservabilityCredentialPort for OsObservabilityCredentialAdapter {
    fn load_otlp_auth(
        &self,
    ) -> Result<Option<zeroize::Zeroizing<String>>, ExecutionTelemetryError> {
        OsObservabilityCredentialAdapter::load_otlp_auth(self)
    }

    fn set_otlp_auth(&self, secret: &str) -> Result<(), ExecutionTelemetryError> {
        self.store
            .set(OTLP_AUTH_ACCOUNT, secret)
            .map_err(|error| ExecutionTelemetryError::Unavailable(error.to_string()))
    }

    fn delete_otlp_auth(&self) -> Result<(), ExecutionTelemetryError> {
        self.store
            .delete(OTLP_AUTH_ACCOUNT)
            .map_err(|error| ExecutionTelemetryError::Unavailable(error.to_string()))
    }

    fn has_otlp_auth(&self) -> Result<bool, ExecutionTelemetryError> {
        self.store
            .get(OTLP_AUTH_ACCOUNT)
            .map(|secret| secret.is_some())
            .map_err(|error| ExecutionTelemetryError::Unavailable(error.to_string()))
    }
}
