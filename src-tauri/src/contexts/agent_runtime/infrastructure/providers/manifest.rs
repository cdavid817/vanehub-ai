use crate::contexts::agent_runtime::application::AgentProviderError;
use crate::contexts::agent_runtime::domain::{
    InteractionMode, ProviderCapabilities, ProviderCapabilityInput, ProviderFamily,
    ProviderMetadata, ProviderReadinessPrerequisites, ProviderUsageCapability,
};
use serde::Deserialize;

pub(super) const PROVIDER_MANIFEST_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProviderManifestV1 {
    schema_version: u16,
    id: String,
    name: String,
    runtime: ProviderRuntime,
    executables: Vec<String>,
    capabilities: ManifestCapabilities,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ProviderRuntime {
    Cli,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestCapabilities {
    terminal: bool,
    resume: bool,
    structured_output: bool,
    images: bool,
    usage: bool,
    permissions: bool,
    model_selection: bool,
    reasoning: bool,
    sandbox: bool,
}

#[derive(Debug)]
pub(super) struct ValidatedProviderManifest {
    pub(super) metadata: ProviderMetadata,
    pub(super) capabilities: ProviderCapabilities,
    pub(super) readiness: ProviderReadinessPrerequisites,
}

impl ValidatedProviderManifest {
    pub(super) fn parse_json(input: &str) -> Result<Self, AgentProviderError> {
        let manifest: ProviderManifestV1 = serde_json::from_str(input)
            .map_err(|error| AgentProviderError::InvalidManifest(safe_json_error(&error)))?;
        if manifest.schema_version != PROVIDER_MANIFEST_SCHEMA_VERSION {
            return Err(AgentProviderError::InvalidManifest(format!(
                "unsupported schema version {}",
                manifest.schema_version
            )));
        }
        let ProviderRuntime::Cli = manifest.runtime;
        if manifest.capabilities.images {
            return Err(AgentProviderError::InvalidManifest(
                "image capability is not available to CLI SDK version 1".to_string(),
            ));
        }
        if !manifest.capabilities.usage {
            return Err(AgentProviderError::InvalidManifest(
                "usage declaration is required".to_string(),
            ));
        }
        for executable in &manifest.executables {
            validate_executable_basename(executable)?;
        }
        let metadata = ProviderMetadata::new(
            manifest.id.clone(),
            manifest.name,
            ProviderFamily::CodingCli,
        )
        .map_err(|error| invalid_contract(&manifest.id, error))?;
        let capabilities = ProviderCapabilities::new(ProviderCapabilityInput {
            interaction_modes: vec![InteractionMode::Cli],
            session_resume: manifest.capabilities.resume,
            structured_output: manifest.capabilities.structured_output,
            terminal: manifest.capabilities.terminal,
            usage: ProviderUsageCapability::HeadlessReported,
            permissions: manifest.capabilities.permissions,
            model_selection: manifest.capabilities.model_selection,
            reasoning: manifest.capabilities.reasoning,
            sandbox: manifest.capabilities.sandbox,
        })
        .map_err(|error| invalid_contract(&manifest.id, error))?;
        let readiness = ProviderReadinessPrerequisites::new(manifest.executables, None)
            .map_err(|error| invalid_contract(&manifest.id, error))?;
        Ok(Self {
            metadata,
            capabilities,
            readiness,
        })
    }
}

fn validate_executable_basename(value: &str) -> Result<(), AgentProviderError> {
    let value = value.trim();
    let invalid = value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.contains(':')
        || value.chars().any(char::is_control)
        || value.to_ascii_lowercase().ends_with(".sh")
        || value.to_ascii_lowercase().ends_with(".ps1");
    if invalid {
        Err(AgentProviderError::InvalidManifest(
            "executable must be a reviewed basename".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn invalid_contract(id: &str, error: impl std::fmt::Display) -> AgentProviderError {
    AgentProviderError::InvalidContract {
        provider_id: id.to_string(),
        reason: error.to_string(),
    }
}

fn safe_json_error(error: &serde_json::Error) -> String {
    format!(
        "schema violation at line {} column {}",
        error.line(),
        error.column()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{
      "schemaVersion": 1,
      "id": "fixture-cli",
      "name": "Fixture CLI",
      "runtime": "cli",
      "executables": ["fixture"],
      "capabilities": {
        "terminal": true, "resume": true, "structuredOutput": true,
        "images": false, "usage": true, "permissions": true,
        "modelSelection": true, "reasoning": false, "sandbox": false
      }
    }"#;

    #[test]
    fn valid_manifest_normalizes_into_domain_values() {
        let manifest = ValidatedProviderManifest::parse_json(VALID).expect("manifest");
        assert_eq!(manifest.metadata.id().as_str(), "fixture-cli");
        assert_eq!(manifest.readiness.executable_names(), &["fixture"]);
        assert!(manifest.capabilities.terminal());
    }

    #[test]
    fn manifest_rejects_unknown_duplicate_and_executable_fields() {
        for invalid in [
            VALID.replace("\"schemaVersion\": 1", "\"schemaVersion\": 2"),
            VALID.replace("\"id\": \"fixture-cli\",", "\"id\": \"a\", \"id\": \"b\","),
            VALID.replace(
                "\"executables\": [\"fixture\"]",
                "\"executables\": [\"../hook.sh\"]",
            ),
            VALID.replace(
                "\"runtime\": \"cli\",",
                "\"runtime\": \"cli\", \"installHook\": \"run\",",
            ),
            VALID.replace(
                "\"runtime\": \"cli\",",
                "\"runtime\": \"cli\", \"arguments\": [\"--unsafe\"],",
            ),
            VALID.replace(
                "\"runtime\": \"cli\",",
                "\"runtime\": \"cli\", \"environment\": {\"TOKEN\": \"secret\"},",
            ),
            VALID.replace(
                "\"executables\": [\"fixture\"]",
                "\"executables\": [\"/absolute/fixture\"]",
            ),
            VALID.replace(
                "\"executables\": [\"fixture\"]",
                "\"executables\": [\"https://invalid.example/tool\"]",
            ),
        ] {
            assert!(ValidatedProviderManifest::parse_json(&invalid).is_err());
        }
    }

    #[test]
    fn manifest_errors_do_not_echo_sensitive_payloads() {
        let sensitive = VALID.replace(
            "\"runtime\": \"cli\",",
            "\"runtime\": \"cli\", \"token\": \"super-secret-value\",",
        );
        let error = ValidatedProviderManifest::parse_json(&sensitive)
            .expect_err("unknown field")
            .to_string();
        assert!(!error.contains("super-secret-value"));
        assert!(!error.contains("token"));
        assert!(error.contains("schema violation"));
    }
}
