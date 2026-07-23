use super::*;
use crate::contexts::execution_observability::application::{
    ExecutionObservabilityRepositoryPort, ObservabilityCredentialPort,
};
use std::sync::Mutex;

struct FailingSettingsRepository {
    settings: ObservabilitySettings,
}

impl ExecutionObservabilityRepositoryPort for FailingSettingsRepository {
    fn load_settings(&self) -> Result<ObservabilitySettings, ExecutionTelemetryError> {
        Ok(self.settings.clone())
    }

    fn update_settings(
        &self,
        _settings: &ObservabilitySettings,
        _updated_at: &str,
    ) -> Result<(), ExecutionTelemetryError> {
        Err(ExecutionTelemetryError::Storage(
            "simulated settings failure".to_string(),
        ))
    }

    fn list_runs(
        &self,
        _request: &PageRequest,
        _session_id: Option<&str>,
    ) -> Result<Page<ExecutionRun>, ExecutionTelemetryError> {
        Ok(Page {
            items: Vec::new(),
            next_page_token: None,
        })
    }

    fn timeline(
        &self,
        _run_id: &ExecutionRunId,
    ) -> Result<Option<ExecutionTimeline>, ExecutionTelemetryError> {
        Ok(None)
    }
}

#[derive(Default)]
struct MemoryCredentials {
    secret: Mutex<Option<String>>,
}

impl ObservabilityCredentialPort for MemoryCredentials {
    fn load_otlp_auth(
        &self,
    ) -> Result<Option<zeroize::Zeroizing<String>>, ExecutionTelemetryError> {
        Ok(self
            .secret
            .lock()
            .expect("secret")
            .clone()
            .map(zeroize::Zeroizing::new))
    }

    fn set_otlp_auth(&self, secret: &str) -> Result<(), ExecutionTelemetryError> {
        *self.secret.lock().expect("secret") = Some(secret.to_string());
        Ok(())
    }

    fn delete_otlp_auth(&self) -> Result<(), ExecutionTelemetryError> {
        *self.secret.lock().expect("secret") = None;
        Ok(())
    }

    fn has_otlp_auth(&self) -> Result<bool, ExecutionTelemetryError> {
        Ok(self.secret.lock().expect("secret").is_some())
    }
}

#[test]
fn failed_settings_persistence_restores_the_previous_credential() {
    let credentials = Arc::new(MemoryCredentials::default());
    credentials.set_otlp_auth("previous-secret").expect("seed");
    let api = ExecutionObservabilityApi::new(
        Arc::new(FailingSettingsRepository {
            settings: ObservabilitySettings::default(),
        }),
        credentials.clone(),
    );

    let result = api.update_settings(
        &ObservabilitySettings::default(),
        Some("replacement-secret"),
        "2026-07-23T00:00:00Z",
    );

    assert!(result.is_err());
    assert_eq!(
        credentials
            .load_otlp_auth()
            .expect("credential")
            .as_deref()
            .map(String::as_str),
        Some("previous-secret")
    );
}

#[test]
fn invalid_settings_never_mutate_credentials() {
    let credentials = Arc::new(MemoryCredentials::default());
    credentials.set_otlp_auth("previous-secret").expect("seed");
    let api = ExecutionObservabilityApi::new(
        Arc::new(FailingSettingsRepository {
            settings: ObservabilitySettings::default(),
        }),
        credentials.clone(),
    );
    let invalid = ObservabilitySettings {
        retention_days: 91,
        ..ObservabilitySettings::default()
    };

    let result = api.update_settings(&invalid, Some("replacement-secret"), "2026-07-23T00:00:00Z");

    assert!(matches!(
        result,
        Err(ExecutionTelemetryError::InvalidSettings { .. })
    ));
    assert_eq!(
        credentials
            .load_otlp_auth()
            .expect("credential")
            .as_deref()
            .map(String::as_str),
        Some("previous-secret")
    );
}

#[test]
fn capabilities_claim_proxied_relay_only_for_verified_providers() {
    let api = ExecutionObservabilityApi::new(
        Arc::new(FailingSettingsRepository {
            settings: ObservabilitySettings::default(),
        }),
        Arc::new(MemoryCredentials::default()),
    );

    let capabilities = api.observation_capabilities();

    assert!(capabilities
        .iter()
        .filter(|capability| matches!(capability.agent_id.as_str(), "claude-code" | "codex-cli"))
        .all(|capability| {
            capability.relay_supported && capability.mcp_fidelity == ExecutionFidelity::Proxied
        }));
    assert!(capabilities
        .iter()
        .filter(|capability| matches!(capability.agent_id.as_str(), "gemini-cli" | "opencode"))
        .all(|capability| {
            !capability.relay_supported && capability.mcp_fidelity == ExecutionFidelity::Opaque
        }));
}
