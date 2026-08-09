use super::{AgentRuntimeDomainError, InteractionMode};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AgentProviderId(String);

impl AgentProviderId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, AgentRuntimeDomainError> {
        Ok(Self(required(value.into(), "Agent provider id")?))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderFamily {
    CodingCli,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderMetadata {
    id: AgentProviderId,
    display_name: String,
    family: ProviderFamily,
}

impl ProviderMetadata {
    pub(crate) fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        family: ProviderFamily,
    ) -> Result<Self, AgentRuntimeDomainError> {
        Ok(Self {
            id: AgentProviderId::parse(id)?,
            display_name: required(display_name.into(), "Agent provider display name")?,
            family,
        })
    }

    pub(crate) fn id(&self) -> &AgentProviderId {
        &self.id
    }

    pub(crate) fn display_name(&self) -> &str {
        &self.display_name
    }

    pub(crate) fn family(&self) -> ProviderFamily {
        self.family
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderUsageCapability {
    HeadlessReported,
    HeadlessAndTerminalReported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderCapabilities {
    interaction_modes: Vec<InteractionMode>,
    session_resume: bool,
    structured_output: bool,
    terminal: bool,
    usage: ProviderUsageCapability,
    permissions: bool,
    model_selection: bool,
    reasoning: bool,
    sandbox: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderCapabilityInput {
    pub(crate) interaction_modes: Vec<InteractionMode>,
    pub(crate) session_resume: bool,
    pub(crate) structured_output: bool,
    pub(crate) terminal: bool,
    pub(crate) usage: ProviderUsageCapability,
    pub(crate) permissions: bool,
    pub(crate) model_selection: bool,
    pub(crate) reasoning: bool,
    pub(crate) sandbox: bool,
}

impl ProviderCapabilities {
    pub(crate) fn new(input: ProviderCapabilityInput) -> Result<Self, AgentRuntimeDomainError> {
        let mut interaction_modes = Vec::new();
        for mode in input.interaction_modes {
            if !interaction_modes.contains(&mode) {
                interaction_modes.push(mode);
            }
        }
        if interaction_modes.is_empty() {
            return Err(AgentRuntimeDomainError::InvalidProviderCapability(
                "at least one interaction mode is required".to_string(),
            ));
        }
        if input.terminal && !interaction_modes.contains(&InteractionMode::Cli) {
            return Err(AgentRuntimeDomainError::InvalidProviderCapability(
                "terminal support requires CLI interaction mode".to_string(),
            ));
        }
        if input.sandbox && !input.permissions {
            return Err(AgentRuntimeDomainError::InvalidProviderCapability(
                "sandbox support requires permission controls".to_string(),
            ));
        }
        Ok(Self {
            interaction_modes,
            session_resume: input.session_resume,
            structured_output: input.structured_output,
            terminal: input.terminal,
            usage: input.usage,
            permissions: input.permissions,
            model_selection: input.model_selection,
            reasoning: input.reasoning,
            sandbox: input.sandbox,
        })
    }

    pub(crate) fn interaction_modes(&self) -> &[InteractionMode] {
        &self.interaction_modes
    }

    pub(crate) fn session_resume(&self) -> bool {
        self.session_resume
    }

    pub(crate) fn structured_output(&self) -> bool {
        self.structured_output
    }

    pub(crate) fn terminal(&self) -> bool {
        self.terminal
    }

    pub(crate) fn usage(&self) -> ProviderUsageCapability {
        self.usage
    }

    pub(crate) fn permissions(&self) -> bool {
        self.permissions
    }

    pub(crate) fn model_selection(&self) -> bool {
        self.model_selection
    }

    pub(crate) fn reasoning(&self) -> bool {
        self.reasoning
    }

    pub(crate) fn sandbox(&self) -> bool {
        self.sandbox
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderReadinessPrerequisites {
    executable_names: Vec<String>,
    managed_sdk_dependency_id: Option<String>,
}

impl ProviderReadinessPrerequisites {
    pub(crate) fn new(
        executable_names: Vec<String>,
        managed_sdk_dependency_id: Option<String>,
    ) -> Result<Self, AgentRuntimeDomainError> {
        let mut normalized = Vec::new();
        for executable in executable_names {
            let executable = required(executable, "Agent provider executable name")?;
            if !normalized.contains(&executable) {
                normalized.push(executable);
            }
        }
        if normalized.is_empty() {
            return Err(AgentRuntimeDomainError::InvalidProviderCapability(
                "a CLI provider requires at least one executable name".to_string(),
            ));
        }
        Ok(Self {
            executable_names: normalized,
            managed_sdk_dependency_id: managed_sdk_dependency_id
                .map(|value| required(value, "Managed SDK dependency id"))
                .transpose()?,
        })
    }

    pub(crate) fn executable_names(&self) -> &[String] {
        &self.executable_names
    }

    pub(crate) fn managed_sdk_dependency_id(&self) -> Option<&str> {
        self.managed_sdk_dependency_id.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderSessionRef {
    provider_id: AgentProviderId,
    external_id: String,
}

impl ProviderSessionRef {
    pub(crate) fn new(
        provider_id: AgentProviderId,
        external_id: impl Into<String>,
    ) -> Result<Self, AgentRuntimeDomainError> {
        Ok(Self {
            provider_id,
            external_id: required(external_id.into(), "Provider session id")?,
        })
    }

    pub(crate) fn provider_id(&self) -> &AgentProviderId {
        &self.provider_id
    }

    pub(crate) fn external_id(&self) -> &str {
        &self.external_id
    }
}

fn required(value: String, label: &'static str) -> Result<String, AgentRuntimeDomainError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(AgentRuntimeDomainError::RequiredValue(label))
    } else if value.chars().any(char::is_control) {
        Err(AgentRuntimeDomainError::ControlCharacters(label))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities() -> ProviderCapabilityInput {
        ProviderCapabilityInput {
            interaction_modes: vec![InteractionMode::Cli, InteractionMode::Cli],
            session_resume: true,
            structured_output: true,
            terminal: true,
            usage: ProviderUsageCapability::HeadlessReported,
            permissions: true,
            model_selection: true,
            reasoning: false,
            sandbox: true,
        }
    }

    #[test]
    fn provider_values_validate_and_normalize_input() {
        assert!(AgentProviderId::parse(" \n ").is_err());
        assert!(ProviderMetadata::new("codex-cli", "\u{0}", ProviderFamily::CodingCli).is_err());
        let metadata = ProviderMetadata::new("codex-cli", " Codex CLI ", ProviderFamily::CodingCli)
            .expect("metadata");
        assert_eq!(metadata.id().as_str(), "codex-cli");
        assert_eq!(metadata.display_name(), "Codex CLI");

        let capabilities = ProviderCapabilities::new(capabilities()).expect("capabilities");
        assert_eq!(capabilities.interaction_modes(), &[InteractionMode::Cli]);
        assert!(capabilities.terminal());

        let readiness = ProviderReadinessPrerequisites::new(
            vec!["codex".to_string(), "codex".to_string()],
            Some("codex-sdk".to_string()),
        )
        .expect("readiness");
        assert_eq!(readiness.executable_names(), &["codex".to_string()]);
        assert_eq!(readiness.managed_sdk_dependency_id(), Some("codex-sdk"));

        let session =
            ProviderSessionRef::new(metadata.id().clone(), "thread-1").expect("provider session");
        assert_eq!(session.provider_id(), metadata.id());
        assert_eq!(session.external_id(), "thread-1");
    }

    #[test]
    fn inconsistent_capabilities_are_rejected() {
        let mut input = capabilities();
        input.interaction_modes = vec![InteractionMode::Api];
        assert!(ProviderCapabilities::new(input).is_err());

        let mut input = capabilities();
        input.permissions = false;
        assert!(ProviderCapabilities::new(input).is_err());

        assert!(ProviderReadinessPrerequisites::new(Vec::new(), None).is_err());
        assert!(ProviderSessionRef::new(
            AgentProviderId::parse("codex-cli").expect("provider id"),
            " \t "
        )
        .is_err());
    }
}
