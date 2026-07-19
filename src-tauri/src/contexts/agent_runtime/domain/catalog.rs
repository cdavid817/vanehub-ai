use super::AgentRuntimeDomainError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct AgentId(String);

impl AgentId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, AgentRuntimeDomainError> {
        let value = required_value(value.into(), "Agent id")?;
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum InteractionMode {
    Browser,
    NativeDesktop,
    Cli,
}

impl InteractionMode {
    pub(crate) fn parse(value: &str) -> Result<Self, AgentRuntimeDomainError> {
        match value {
            "browser" => Ok(Self::Browser),
            "native-desktop" => Ok(Self::NativeDesktop),
            "cli" => Ok(Self::Cli),
            other => Err(AgentRuntimeDomainError::UnsupportedInteractionMode(
                other.to_string(),
            )),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Browser => "browser",
            Self::NativeDesktop => "native-desktop",
            Self::Cli => "cli",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LaunchKind {
    Cli,
    Browser,
    NativeDesktop,
    Other(String),
}

impl LaunchKind {
    fn parse(value: String) -> Result<Self, AgentRuntimeDomainError> {
        let value = required_value(value, "Launch kind")?;
        Ok(match value.as_str() {
            "cli" => Self::Cli,
            "browser" => Self::Browser,
            "native-desktop" => Self::NativeDesktop,
            _ => Self::Other(value),
        })
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Cli => "cli",
            Self::Browser => "browser",
            Self::NativeDesktop => "native-desktop",
            Self::Other(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LaunchMetadata {
    kind: LaunchKind,
    command: Option<String>,
    url: Option<String>,
    executable_name: Option<String>,
}

impl LaunchMetadata {
    pub(crate) fn new(
        kind: String,
        command: Option<String>,
        url: Option<String>,
        executable_name: Option<String>,
    ) -> Result<Self, AgentRuntimeDomainError> {
        Ok(Self {
            kind: LaunchKind::parse(kind)?,
            command: optional_value(command, "Launch command")?,
            url: optional_value(url, "Launch URL")?,
            executable_name: optional_value(executable_name, "Executable name")?,
        })
    }

    #[cfg(test)]
    pub(crate) fn kind(&self) -> &LaunchKind {
        &self.kind
    }

    pub(crate) fn kind_str(&self) -> &str {
        self.kind.as_str()
    }

    pub(crate) fn command(&self) -> Option<&str> {
        self.command.as_deref()
    }

    pub(crate) fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    pub(crate) fn executable_name(&self) -> Option<&str> {
        self.executable_name.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentAvailability {
    Available,
    Unavailable,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained for the stable agent availability contract"
        )
    )]
    NeedsAuthentication,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AvailabilityAssessment {
    state: AgentAvailability,
    reason: Option<String>,
}

impl AvailabilityAssessment {
    pub(crate) fn new(state: AgentAvailability, reason: Option<String>) -> Self {
        Self { state, reason }
    }

    pub(crate) fn assess(probe: AvailabilityProbe) -> Self {
        match probe.managed_sdk {
            ManagedSdkStatus::Unrecognized(id) => Self::new(
                AgentAvailability::Unavailable,
                Some(format!("Managed SDK dependency '{id}' is not recognized.")),
            ),
            ManagedSdkStatus::Missing(id) => Self::new(
                AgentAvailability::Unavailable,
                Some(format!("Managed SDK dependency '{id}' is not installed.")),
            ),
            ManagedSdkStatus::NotRequired | ManagedSdkStatus::Available => match probe.executable {
                ExecutableStatus::Available => Self::new(AgentAvailability::Available, None),
                ExecutableStatus::Missing(name) => Self::new(
                    AgentAvailability::Unavailable,
                    Some(format!("Command '{name}' was not found on PATH.")),
                ),
                ExecutableStatus::NotDeclared => Self::new(AgentAvailability::Unknown, None),
            },
        }
    }

    pub(crate) fn state(&self) -> AgentAvailability {
        self.state
    }

    pub(crate) fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManagedSdkStatus {
    NotRequired,
    Available,
    Missing(String),
    Unrecognized(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExecutableStatus {
    NotDeclared,
    Available,
    Missing(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AvailabilityProbe {
    pub(crate) managed_sdk: ManagedSdkStatus,
    pub(crate) executable: ExecutableStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentDefinitionInput {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) provider: String,
    pub(crate) managed_sdk_dependency_id: Option<String>,
    pub(crate) launch: LaunchMetadata,
    pub(crate) supported_interaction_modes: Vec<InteractionMode>,
    pub(crate) availability: AvailabilityAssessment,
    pub(crate) capability_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentDefinition {
    id: AgentId,
    display_name: String,
    provider: String,
    managed_sdk_dependency_id: Option<String>,
    launch: LaunchMetadata,
    supported_interaction_modes: Vec<InteractionMode>,
    availability: AvailabilityAssessment,
    capability_tags: Vec<String>,
}

impl AgentDefinition {
    pub(crate) fn new(input: AgentDefinitionInput) -> Result<Self, AgentRuntimeDomainError> {
        let mut modes = Vec::new();
        for mode in input.supported_interaction_modes {
            if !modes.contains(&mode) {
                modes.push(mode);
            }
        }
        let mut tags = Vec::new();
        for tag in input.capability_tags {
            let tag = required_value(tag, "Capability tag")?;
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        Ok(Self {
            id: AgentId::parse(input.id)?,
            display_name: required_value(input.display_name, "Agent display name")?,
            provider: required_value(input.provider, "Agent provider")?,
            managed_sdk_dependency_id: optional_value(
                input.managed_sdk_dependency_id,
                "Managed SDK dependency id",
            )?,
            launch: input.launch,
            supported_interaction_modes: modes,
            availability: input.availability,
            capability_tags: tags,
        })
    }

    pub(crate) fn ensure_selectable(
        &self,
        mode: InteractionMode,
    ) -> Result<(), AgentRuntimeDomainError> {
        if matches!(
            self.availability.state(),
            AgentAvailability::Unavailable | AgentAvailability::NeedsAuthentication
        ) {
            return Err(AgentRuntimeDomainError::AgentUnavailable(
                self.availability
                    .reason()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{} is not available.", self.display_name)),
            ));
        }
        if !self.supports(mode) {
            return Err(AgentRuntimeDomainError::InteractionModeNotSupported {
                agent_id: self.id.as_str().to_string(),
                mode: mode.as_str().to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn supports(&self, mode: InteractionMode) -> bool {
        self.supported_interaction_modes.contains(&mode)
    }

    pub(crate) fn has_capability(&self, tag: &str) -> bool {
        self.capability_tags
            .iter()
            .any(|candidate| candidate == tag)
    }

    pub(crate) fn id(&self) -> &AgentId {
        &self.id
    }

    pub(crate) fn display_name(&self) -> &str {
        &self.display_name
    }

    pub(crate) fn provider(&self) -> &str {
        &self.provider
    }

    pub(crate) fn managed_sdk_dependency_id(&self) -> Option<&str> {
        self.managed_sdk_dependency_id.as_deref()
    }

    pub(crate) fn launch(&self) -> &LaunchMetadata {
        &self.launch
    }

    pub(crate) fn supported_interaction_modes(&self) -> &[InteractionMode] {
        &self.supported_interaction_modes
    }

    pub(crate) fn availability(&self) -> &AvailabilityAssessment {
        &self.availability
    }

    pub(crate) fn capability_tags(&self) -> &[String] {
        &self.capability_tags
    }
}

fn required_value(value: String, label: &'static str) -> Result<String, AgentRuntimeDomainError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(AgentRuntimeDomainError::RequiredValue(label))
    } else if value.chars().any(char::is_control) {
        Err(AgentRuntimeDomainError::ControlCharacters(label))
    } else {
        Ok(value)
    }
}

fn optional_value(
    value: Option<String>,
    label: &'static str,
) -> Result<Option<String>, AgentRuntimeDomainError> {
    value.map(|value| required_value(value, label)).transpose()
}
