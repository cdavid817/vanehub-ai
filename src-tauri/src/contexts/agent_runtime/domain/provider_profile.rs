#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfileRuntimeKind {
    Cloud,
    Local,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndpointSource {
    Catalog,
    Configured,
    Discovered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthenticationMode {
    Required,
    Optional,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProfilePrivacy {
    Cloud,
    Local,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapabilityState {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapabilityProvenance {
    Configured,
    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProfileCapability {
    pub(crate) state: CapabilityState,
    pub(crate) provenance: CapabilityProvenance,
}

impl ProfileCapability {
    pub(crate) const fn configured(state: CapabilityState) -> Self {
        Self {
            state,
            provenance: CapabilityProvenance::Configured,
        }
    }

    pub(crate) const fn is_supported(self) -> bool {
        matches!(self.state, CapabilityState::Supported)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EndpointCapabilities {
    pub(crate) text_generation: ProfileCapability,
    pub(crate) tool_calling: ProfileCapability,
    pub(crate) image_input: ProfileCapability,
    pub(crate) structured_output: ProfileCapability,
    pub(crate) reasoning_field: ProfileCapability,
}

impl EndpointCapabilities {
    #[cfg(test)]
    pub(crate) const fn conservative_text() -> Self {
        Self {
            text_generation: ProfileCapability::configured(CapabilityState::Supported),
            tool_calling: ProfileCapability::configured(CapabilityState::Unknown),
            image_input: ProfileCapability::configured(CapabilityState::Unknown),
            structured_output: ProfileCapability::configured(CapabilityState::Unknown),
            reasoning_field: ProfileCapability::configured(CapabilityState::Unknown),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextCapacityProvenance {
    Verified,
    ConfiguredEstimate,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProfileContextCapacity {
    pub(crate) context_window_tokens: Option<u64>,
    pub(crate) reserved_output_tokens: u64,
    pub(crate) provenance: ContextCapacityProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EndpointProfileSnapshot {
    pub(crate) id: String,
    pub(crate) agent_id: String,
    pub(crate) runtime_kind: ProfileRuntimeKind,
    pub(crate) endpoint_source: EndpointSource,
    pub(crate) base_url: String,
    pub(crate) interface_format: String,
    pub(crate) model_id: String,
    pub(crate) authentication_mode: AuthenticationMode,
    pub(crate) credential_present: bool,
    pub(crate) timeout_ms: u64,
    pub(crate) privacy: ProfilePrivacy,
    pub(crate) capabilities: EndpointCapabilities,
    pub(crate) context_capacity: ProfileContextCapacity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProviderProfileError {
    Required(&'static str),
    InvalidBaseUrl,
    LocalEndpointNotLoopback,
    RuntimePrivacyMismatch,
    CredentialRequired,
    CredentialForbidden,
    InvalidTimeout,
    InvalidContextCapacity,
    TextGenerationUnsupported,
}

impl EndpointProfileSnapshot {
    pub(crate) fn new(mut value: Self) -> Result<Self, ProviderProfileError> {
        value.id = required(value.id, "Profile id")?;
        value.agent_id = required(value.agent_id, "Agent id")?;
        value.model_id = required(value.model_id, "Model id")?;
        value.interface_format = required(value.interface_format, "Interface format")?;
        value.base_url = normalize_base_url(&value.base_url)?;
        if value.interface_format != "openai-compatible" {
            return Err(ProviderProfileError::Required(
                "OpenAI-compatible interface",
            ));
        }
        if !(100..=120_000).contains(&value.timeout_ms) {
            return Err(ProviderProfileError::InvalidTimeout);
        }
        if value.runtime_kind == ProfileRuntimeKind::Local && !is_loopback_url(&value.base_url) {
            return Err(ProviderProfileError::LocalEndpointNotLoopback);
        }
        if !matches!(
            (value.runtime_kind, value.privacy),
            (ProfileRuntimeKind::Cloud, ProfilePrivacy::Cloud)
                | (ProfileRuntimeKind::Local, ProfilePrivacy::Local)
                | (ProfileRuntimeKind::Private, ProfilePrivacy::Private)
        ) {
            return Err(ProviderProfileError::RuntimePrivacyMismatch);
        }
        match (value.authentication_mode, value.credential_present) {
            (AuthenticationMode::Required, false) => {
                return Err(ProviderProfileError::CredentialRequired)
            }
            (AuthenticationMode::None, true) => {
                return Err(ProviderProfileError::CredentialForbidden)
            }
            _ => {}
        }
        validate_capacity(value.context_capacity)?;
        if value.capabilities.text_generation.state == CapabilityState::Unsupported {
            return Err(ProviderProfileError::TextGenerationUnsupported);
        }
        Ok(value)
    }

    pub(crate) fn is_local(&self) -> bool {
        self.runtime_kind == ProfileRuntimeKind::Local
    }
}

fn required(value: String, label: &'static str) -> Result<String, ProviderProfileError> {
    let value = value.trim().to_string();
    if value.is_empty() || value.chars().any(char::is_control) {
        Err(ProviderProfileError::Required(label))
    } else {
        Ok(value)
    }
}

fn normalize_base_url(value: &str) -> Result<String, ProviderProfileError> {
    let value = value.trim().trim_end_matches('/');
    let rest = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .ok_or(ProviderProfileError::InvalidBaseUrl)?;
    let authority = rest.split('/').next().unwrap_or_default();
    if authority.is_empty()
        || authority.contains('@')
        || authority.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        return Err(ProviderProfileError::InvalidBaseUrl);
    }
    Ok(value.to_string())
}

fn is_loopback_url(value: &str) -> bool {
    let rest = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))
        .unwrap_or_default();
    let host = rest.split('/').next().unwrap_or_default();
    let host = host.rsplit_once(':').map_or(host, |(name, _)| name);
    matches!(
        host.to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "[::1]"
    )
}

fn validate_capacity(value: ProfileContextCapacity) -> Result<(), ProviderProfileError> {
    match (value.context_window_tokens, value.provenance) {
        (None, ContextCapacityProvenance::Unknown) if value.reserved_output_tokens == 0 => Ok(()),
        (Some(window), provenance)
            if provenance != ContextCapacityProvenance::Unknown
                && (1_024..=10_000_000).contains(&window)
                && value.reserved_output_tokens < window =>
        {
            Ok(())
        }
        _ => Err(ProviderProfileError::InvalidContextCapacity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_profile() -> EndpointProfileSnapshot {
        EndpointProfileSnapshot {
            id: "local-qwen".to_string(),
            agent_id: "onepiece".to_string(),
            runtime_kind: ProfileRuntimeKind::Local,
            endpoint_source: EndpointSource::Configured,
            base_url: "http://127.0.0.1:11434/v1/".to_string(),
            interface_format: "openai-compatible".to_string(),
            model_id: "qwen".to_string(),
            authentication_mode: AuthenticationMode::None,
            credential_present: false,
            timeout_ms: 30_000,
            privacy: ProfilePrivacy::Local,
            capabilities: EndpointCapabilities::conservative_text(),
            context_capacity: ProfileContextCapacity {
                context_window_tokens: Some(32_768),
                reserved_output_tokens: 4_096,
                provenance: ContextCapacityProvenance::ConfiguredEstimate,
            },
        }
    }

    #[test]
    fn local_profile_normalizes_and_preserves_configured_provenance() {
        let profile = EndpointProfileSnapshot::new(local_profile()).expect("valid local profile");
        assert_eq!(profile.base_url, "http://127.0.0.1:11434/v1");
        assert!(profile.is_local());
        assert_eq!(
            profile.context_capacity.provenance,
            ContextCapacityProvenance::ConfiguredEstimate
        );
    }

    #[test]
    fn invalid_location_auth_and_capacity_are_rejected() {
        let mut profile = local_profile();
        profile.base_url = "http://192.168.1.9:11434".to_string();
        assert_eq!(
            EndpointProfileSnapshot::new(profile),
            Err(ProviderProfileError::LocalEndpointNotLoopback)
        );

        let mut profile = local_profile();
        profile.credential_present = true;
        assert_eq!(
            EndpointProfileSnapshot::new(profile),
            Err(ProviderProfileError::CredentialForbidden)
        );

        let mut profile = local_profile();
        profile.context_capacity.reserved_output_tokens = 32_768;
        assert_eq!(
            EndpointProfileSnapshot::new(profile),
            Err(ProviderProfileError::InvalidContextCapacity)
        );
    }

    #[test]
    fn model_identity_does_not_supply_capability_or_capacity() {
        let mut first = local_profile();
        first.model_id = "shared-model".to_string();
        let mut second = first.clone();
        second.context_capacity = ProfileContextCapacity {
            context_window_tokens: None,
            reserved_output_tokens: 0,
            provenance: ContextCapacityProvenance::Unknown,
        };
        second.capabilities.tool_calling =
            ProfileCapability::configured(CapabilityState::Unsupported);
        let first = EndpointProfileSnapshot::new(first).expect("first");
        let second = EndpointProfileSnapshot::new(second).expect("second");
        assert_ne!(first.context_capacity, second.context_capacity);
        assert_ne!(
            first.capabilities.tool_calling,
            second.capabilities.tool_calling
        );
    }
}
