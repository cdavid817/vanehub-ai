use super::SessionsDomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionExecutionMode {
    Inherit,
    Plan,
    Execute,
}

impl SessionExecutionMode {
    fn parse(value: &str) -> Result<Self, SessionsDomainError> {
        match value {
            "inherit" => Ok(Self::Inherit),
            "plan" => Ok(Self::Plan),
            "execute" => Ok(Self::Execute),
            _ => Err(SessionsDomainError::UnsupportedExecutionMode),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Inherit => "inherit",
            Self::Plan => "plan",
            Self::Execute => "execute",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ReasoningDepth {
    Low,
    Medium,
    High,
    Max,
}

impl ReasoningDepth {
    fn parse(value: &str) -> Result<Self, SessionsDomainError> {
        match value {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "max" => Ok(Self::Max),
            _ => Err(SessionsDomainError::UnsupportedReasoningDepth),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Max => "max",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ChatConfigurationRequest<'a> {
    pub(crate) execution_mode: &'a str,
    pub(crate) provider_id: Option<&'a str>,
    pub(crate) model_id: Option<&'a str>,
    pub(crate) reasoning_depth: Option<&'a str>,
    pub(crate) streaming: bool,
    pub(crate) thinking: bool,
    pub(crate) long_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatPreferences {
    execution_mode: String,
    provider_id: String,
    model_id: String,
    reasoning_depth: Option<String>,
    streaming: bool,
    thinking: bool,
    long_context: bool,
}

impl ChatPreferences {
    pub(crate) fn execution_mode(&self) -> &str {
        &self.execution_mode
    }

    pub(crate) fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub(crate) fn model_id(&self) -> &str {
        &self.model_id
    }

    pub(crate) fn reasoning_depth(&self) -> Option<&str> {
        self.reasoning_depth.as_deref()
    }

    pub(crate) fn streaming(&self) -> bool {
        self.streaming
    }

    pub(crate) fn thinking(&self) -> bool {
        self.thinking
    }

    pub(crate) fn long_context(&self) -> bool {
        self.long_context
    }
}

pub(crate) fn normalize_reasoning(value: Option<&str>) -> Option<String> {
    value
        .and_then(|value| ReasoningDepth::parse(value).ok())
        .map(|value| value.as_str().to_string())
}

fn max_reasoning_for_model(model_id: &str) -> Option<ReasoningDepth> {
    match model_id {
        "claude-opus-4-8" | "claude-sonnet-5" | "gpt-5-5" | "gpt-5-1-codex-max" => {
            Some(ReasoningDepth::Max)
        }
        "claude-sonnet-4-6" | "gpt-5-4" | "gpt-5-2-codex" | "gemini-2-5-pro" => {
            Some(ReasoningDepth::High)
        }
        "gemini-2-5-flash" => Some(ReasoningDepth::Medium),
        _ => None,
    }
}

pub(crate) fn clamp_reasoning_for_model(model_id: &str, value: Option<&str>) -> Option<String> {
    let requested = value.and_then(|value| ReasoningDepth::parse(value).ok())?;
    let maximum = max_reasoning_for_model(model_id)?;
    Some(requested.min(maximum).as_str().to_string())
}

pub(crate) fn normalize_chat_preferences(
    agent_id: &str,
    expected_provider: &str,
    default_model: &str,
    request: ChatConfigurationRequest<'_>,
) -> Result<ChatPreferences, SessionsDomainError> {
    let execution_mode = SessionExecutionMode::parse(request.execution_mode)?;
    let provider_id = request.provider_id.unwrap_or(expected_provider);
    if provider_id != expected_provider {
        return Err(SessionsDomainError::ProviderMismatch {
            provider_id: provider_id.to_string(),
            agent_id: agent_id.to_string(),
        });
    }
    let model_id = request.model_id.unwrap_or(default_model);
    if model_id.trim().is_empty() {
        return Err(SessionsDomainError::UnsupportedModel {
            model_id: model_id.to_string(),
            agent_id: agent_id.to_string(),
        });
    }
    if request
        .reasoning_depth
        .is_some_and(|value| ReasoningDepth::parse(value).is_err())
    {
        return Err(SessionsDomainError::UnsupportedReasoningDepth);
    }
    Ok(ChatPreferences {
        execution_mode: execution_mode.as_str().to_string(),
        provider_id: expected_provider.to_string(),
        model_id: model_id.to_string(),
        reasoning_depth: clamp_reasoning_for_model(model_id, request.reasoning_depth),
        streaming: request.streaming,
        thinking: request.thinking,
        long_context: request.long_context,
    })
}

pub(crate) fn is_valid_chat_snapshot(
    expected_provider: &str,
    execution_mode: &str,
    provider_id: &str,
    model_id: &str,
    reasoning_depth: Option<&str>,
) -> bool {
    provider_id == expected_provider
        && !model_id.trim().is_empty()
        && SessionExecutionMode::parse(execution_mode).is_ok()
        && reasoning_depth.is_none_or(|depth| ReasoningDepth::parse(depth).is_ok())
}

pub(crate) fn restore_chat_preferences(
    expected_provider: &str,
    request: ChatConfigurationRequest<'_>,
) -> Option<ChatPreferences> {
    let provider_id = request.provider_id?;
    let model_id = request.model_id?;
    if !is_valid_chat_snapshot(
        expected_provider,
        request.execution_mode,
        provider_id,
        model_id,
        request.reasoning_depth,
    ) {
        return None;
    }
    Some(ChatPreferences {
        execution_mode: request.execution_mode.to_string(),
        provider_id: provider_id.to_string(),
        model_id: model_id.to_string(),
        reasoning_depth: request.reasoning_depth.map(str::to_string),
        streaming: request.streaming,
        thinking: request.thinking,
        long_context: request.long_context,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request<'a>() -> ChatConfigurationRequest<'a> {
        ChatConfigurationRequest {
            execution_mode: "execute",
            provider_id: Some("google"),
            model_id: Some("gemini-2-5-flash"),
            reasoning_depth: Some("max"),
            streaming: true,
            thinking: true,
            long_context: true,
        }
    }

    fn normalize(
        agent_id: &str,
        request: ChatConfigurationRequest<'_>,
    ) -> Result<ChatPreferences, SessionsDomainError> {
        let (provider, model) = match agent_id {
            "gemini-cli" => ("google", "gemini-2-5-pro"),
            "onepiece" => ("onepiece", "onepiece-active"),
            _ => ("anthropic", "claude-opus-4-8"),
        };
        normalize_chat_preferences(agent_id, provider, model, request)
    }

    #[test]
    fn configuration_identity_and_reasoning_rules_are_agent_authoritative() {
        let preferences = normalize("gemini-cli", request()).expect("preferences");

        assert_eq!(preferences.execution_mode(), "execute");
        assert_eq!(preferences.provider_id(), "google");
        assert_eq!(preferences.model_id(), "gemini-2-5-flash");
        assert_eq!(preferences.reasoning_depth(), Some("medium"));
        assert!(preferences.streaming());
        assert!(preferences.thinking());
        assert!(preferences.long_context());
        let onepiece = normalize(
            "onepiece",
            ChatConfigurationRequest {
                execution_mode: "inherit",
                provider_id: Some("onepiece"),
                model_id: Some("deepseek-chat"),
                reasoning_depth: None,
                streaming: true,
                thinking: true,
                long_context: true,
            },
        )
        .expect("OnePiece preferences");
        assert_eq!(onepiece.provider_id(), "onepiece");
        assert_eq!(onepiece.model_id(), "deepseek-chat");
    }

    #[test]
    fn invalid_permission_provider_model_and_reasoning_are_rejected() {
        let mut invalid = request();
        invalid.execution_mode = "unrestricted";
        assert_eq!(
            normalize("gemini-cli", invalid),
            Err(SessionsDomainError::UnsupportedExecutionMode)
        );

        let mut invalid = request();
        invalid.provider_id = Some("openai");
        assert!(matches!(
            normalize("gemini-cli", invalid),
            Err(SessionsDomainError::ProviderMismatch { .. })
        ));

        let mut custom = request();
        custom.model_id = Some("gpt-5-5");
        // Unknown models are now accepted as custom model IDs
        let preferences = normalize("gemini-cli", custom).expect("custom model accepted");
        assert_eq!(preferences.model_id(), "gpt-5-5");

        let mut invalid = request();
        invalid.reasoning_depth = Some("extreme");
        assert_eq!(
            normalize("gemini-cli", invalid),
            Err(SessionsDomainError::UnsupportedReasoningDepth)
        );
    }

    #[test]
    fn persisted_snapshot_validation_preserves_the_existing_fallback_boundary() {
        assert!(is_valid_chat_snapshot(
            "anthropic",
            "plan",
            "anthropic",
            "claude-sonnet-5",
            Some("high")
        ));
        assert!(!is_valid_chat_snapshot(
            "anthropic",
            "plan",
            "openai",
            "claude-sonnet-5",
            Some("high")
        ));
        assert_eq!(normalize_reasoning(Some("invalid")), None);
        let restored = restore_chat_preferences("google", request()).expect("snapshot");
        assert_eq!(restored.reasoning_depth(), Some("max"));
    }
}
