use crate::contexts::agent_runtime::application::AgentProviderError;
use crate::contexts::agent_runtime::domain::{
    InteractionMode, ProviderCapabilities, ProviderCapabilityInput, ProviderFamily,
    ProviderMetadata, ProviderReadinessPrerequisites, ProviderTransport, ProviderUsageCapability,
};
use serde::Deserialize;

pub(super) const PROVIDER_MANIFEST_SCHEMA_VERSION: u16 = 1;
/// Version 2 adds explicit transports and a three-valued usage declaration. It is a separate
/// schema rather than a relaxation of version 1: a version 1 record keeps rejecting the fields it
/// always rejected, so an old manifest never silently gains meaning it was never reviewed for.
pub(super) const PROVIDER_MANIFEST_SCHEMA_VERSION_V2: u16 = 2;

/// Only the discriminator is read here. Unknown fields are tolerated at this stage because the
/// versioned parser that follows applies `deny_unknown_fields` to the whole document.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestVersionProbe {
    schema_version: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
// `schema_version` was already read by the probe; it stays declared so a missing discriminator
// fails the strict parse rather than being tolerated.
#[allow(dead_code)]
struct ProviderManifestV1 {
    schema_version: u16,
    id: String,
    name: String,
    runtime: ProviderRuntime,
    executables: Vec<String>,
    capabilities: ManifestCapabilities,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
// `schema_version` was already read by the probe; it stays declared so a missing discriminator
// fails the strict parse rather than being tolerated.
#[allow(dead_code)]
struct ProviderManifestV2 {
    schema_version: u16,
    id: String,
    name: String,
    runtime: ProviderRuntime,
    executables: Vec<String>,
    transports: Vec<ManifestTransport>,
    capabilities: ManifestCapabilitiesV2,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ProviderRuntime {
    Cli,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum ManifestTransport {
    Terminal,
    Headless,
    AcpStdio,
}

impl ManifestTransport {
    fn into_domain(self) -> ProviderTransport {
        match self {
            Self::Terminal => ProviderTransport::Terminal,
            Self::Headless => ProviderTransport::Headless,
            Self::AcpStdio => ProviderTransport::AcpStdio,
        }
    }
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

/// Version 2 spells usage out. `false` in version 1 was rejected outright; here "unavailable" is a
/// legitimate, reviewed answer that the runtime renders as such instead of as zero tokens.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum ManifestUsage {
    HeadlessReported,
    HeadlessAndTerminalReported,
    Unavailable,
}

impl ManifestUsage {
    fn into_domain(self) -> ProviderUsageCapability {
        match self {
            Self::HeadlessReported => ProviderUsageCapability::HeadlessReported,
            Self::HeadlessAndTerminalReported => {
                ProviderUsageCapability::HeadlessAndTerminalReported
            }
            Self::Unavailable => ProviderUsageCapability::Unavailable,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestCapabilitiesV2 {
    terminal: bool,
    resume: bool,
    structured_output: bool,
    images: bool,
    usage: ManifestUsage,
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
    pub(super) schema_version: u16,
}

impl ValidatedProviderManifest {
    pub(super) fn parse_json(input: &str) -> Result<Self, AgentProviderError> {
        let probe: ManifestVersionProbe = serde_json::from_str(input)
            .map_err(|error| AgentProviderError::InvalidManifest(safe_json_error(&error)))?;
        match probe.schema_version {
            PROVIDER_MANIFEST_SCHEMA_VERSION => Self::parse_v1(input),
            PROVIDER_MANIFEST_SCHEMA_VERSION_V2 => Self::parse_v2(input),
            other => Err(AgentProviderError::InvalidManifest(format!(
                "unsupported schema version {other}"
            ))),
        }
    }

    fn parse_v1(input: &str) -> Result<Self, AgentProviderError> {
        let manifest: ProviderManifestV1 = serde_json::from_str(input)
            .map_err(|error| AgentProviderError::InvalidManifest(safe_json_error(&error)))?;
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
            schema_version: PROVIDER_MANIFEST_SCHEMA_VERSION,
        })
    }

    fn parse_v2(input: &str) -> Result<Self, AgentProviderError> {
        let manifest: ProviderManifestV2 = serde_json::from_str(input)
            .map_err(|error| AgentProviderError::InvalidManifest(safe_json_error(&error)))?;
        let ProviderRuntime::Cli = manifest.runtime;
        if manifest.capabilities.images {
            return Err(AgentProviderError::InvalidManifest(
                "image capability is not available to CLI SDK version 2".to_string(),
            ));
        }
        if manifest.transports.is_empty() {
            return Err(AgentProviderError::InvalidManifest(
                "at least one transport is required".to_string(),
            ));
        }
        // Protocol streams are structured by construction; a manifest that says otherwise is
        // describing a provider that would have to fall back to text on its ACP stdout, which the
        // SDK forbids.
        if manifest.transports.contains(&ManifestTransport::AcpStdio)
            && !manifest.capabilities.structured_output
        {
            return Err(AgentProviderError::InvalidManifest(
                "acp-stdio transport requires structured output".to_string(),
            ));
        }
        if manifest.capabilities.usage != ManifestUsage::Unavailable
            && !manifest
                .transports
                .iter()
                .any(|transport| *transport != ManifestTransport::Terminal)
        {
            return Err(AgentProviderError::InvalidManifest(
                "a terminal-only provider cannot report managed usage".to_string(),
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
            usage: manifest.capabilities.usage.into_domain(),
            permissions: manifest.capabilities.permissions,
            model_selection: manifest.capabilities.model_selection,
            reasoning: manifest.capabilities.reasoning,
            sandbox: manifest.capabilities.sandbox,
        })
        .and_then(|capabilities| {
            capabilities.with_transports(
                manifest
                    .transports
                    .iter()
                    .map(|transport| transport.into_domain())
                    .collect(),
            )
        })
        .map_err(|error| invalid_contract(&manifest.id, error))?;
        let readiness = ProviderReadinessPrerequisites::new(manifest.executables, None)
            .map_err(|error| invalid_contract(&manifest.id, error))?;
        Ok(Self {
            metadata,
            capabilities,
            readiness,
            schema_version: PROVIDER_MANIFEST_SCHEMA_VERSION_V2,
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

    const VALID_V2: &str = r#"{
      "schemaVersion": 2,
      "id": "fixture-acp",
      "name": "Fixture ACP",
      "runtime": "cli",
      "executables": ["fixture-acp"],
      "transports": ["acp-stdio", "terminal"],
      "capabilities": {
        "terminal": true, "resume": false, "structuredOutput": true,
        "images": false, "usage": "unavailable", "permissions": true,
        "modelSelection": false, "reasoning": false, "sandbox": false
      }
    }"#;

    #[test]
    fn valid_manifest_normalizes_into_domain_values() {
        let manifest = ValidatedProviderManifest::parse_json(VALID).expect("manifest");
        assert_eq!(manifest.metadata.id().as_str(), "fixture-cli");
        assert_eq!(manifest.readiness.executable_names(), &["fixture"]);
        assert!(manifest.capabilities.terminal());
        assert_eq!(manifest.schema_version, 1);
        assert_eq!(
            manifest.capabilities.transports(),
            &[ProviderTransport::Terminal, ProviderTransport::Headless]
        );
    }

    #[test]
    fn manifest_rejects_unknown_duplicate_and_executable_fields() {
        for invalid in [
            VALID.replace("\"schemaVersion\": 1", "\"schemaVersion\": 3"),
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
            // A version 1 record does not learn version 2 vocabulary by being labelled 1.
            VALID.replace(
                "\"runtime\": \"cli\",",
                "\"runtime\": \"cli\", \"transports\": [\"acp-stdio\"],",
            ),
            VALID.replace("\"usage\": true", "\"usage\": \"unavailable\""),
        ] {
            assert!(
                ValidatedProviderManifest::parse_json(&invalid).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn version_two_declares_transports_and_unavailable_usage() {
        let manifest = ValidatedProviderManifest::parse_json(VALID_V2).expect("manifest");
        assert_eq!(manifest.schema_version, 2);
        assert_eq!(
            manifest.capabilities.transports(),
            &[ProviderTransport::AcpStdio, ProviderTransport::Terminal]
        );
        assert_eq!(
            manifest.capabilities.usage(),
            ProviderUsageCapability::Unavailable
        );
        assert!(!manifest.capabilities.session_resume());
        assert_eq!(
            manifest.capabilities.managed_transport(),
            Some(ProviderTransport::AcpStdio)
        );
    }

    #[test]
    fn version_two_stays_data_only_and_internally_consistent() {
        for invalid in [
            VALID_V2.replace(
                "\"runtime\": \"cli\",",
                "\"runtime\": \"cli\", \"arguments\": [\"--acp\"],",
            ),
            VALID_V2.replace(
                "\"runtime\": \"cli\",",
                "\"runtime\": \"cli\", \"env\": {\"API_KEY\": \"x\"},",
            ),
            VALID_V2.replace(
                "\"runtime\": \"cli\",",
                "\"runtime\": \"cli\", \"installUrl\": \"https://example.test/install\",",
            ),
            VALID_V2.replace(
                "\"transports\": [\"acp-stdio\", \"terminal\"]",
                "\"transports\": []",
            ),
            VALID_V2.replace(
                "\"transports\": [\"acp-stdio\", \"terminal\"]",
                "\"transports\": [\"tcp\"]",
            ),
            // Terminal transport declared while the terminal capability is denied.
            VALID_V2.replace("\"terminal\": true", "\"terminal\": false"),
            // ACP without structured output is a contradiction.
            VALID_V2.replace("\"structuredOutput\": true", "\"structuredOutput\": false"),
            VALID_V2.replace("\"usage\": \"unavailable\"", "\"usage\": true"),
            VALID_V2.replace("\"images\": false", "\"images\": true"),
            VALID_V2.replace(
                "\"executables\": [\"fixture-acp\"]",
                "\"executables\": [\"C:\\\\tools\\\\agent.exe\"]",
            ),
        ] {
            assert!(
                ValidatedProviderManifest::parse_json(&invalid).is_err(),
                "{invalid}"
            );
        }
        // Terminal-only providers cannot report managed usage.
        let terminal_only = VALID_V2
            .replace(
                "\"transports\": [\"acp-stdio\", \"terminal\"]",
                "\"transports\": [\"terminal\"]",
            )
            .replace(
                "\"usage\": \"unavailable\"",
                "\"usage\": \"headless-reported\"",
            );
        assert!(ValidatedProviderManifest::parse_json(&terminal_only).is_err());
        let terminal_only_unavailable = VALID_V2.replace(
            "\"transports\": [\"acp-stdio\", \"terminal\"]",
            "\"transports\": [\"terminal\"]",
        );
        let manifest =
            ValidatedProviderManifest::parse_json(&terminal_only_unavailable).expect("manifest");
        assert_eq!(manifest.capabilities.managed_transport(), None);
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
