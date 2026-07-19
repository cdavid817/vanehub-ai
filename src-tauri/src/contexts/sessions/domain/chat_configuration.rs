use super::SessionsDomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatAgent {
    Claude,
    Codex,
    Gemini,
    OpenCode,
}

impl ChatAgent {
    fn parse(agent_id: &str) -> Result<Self, SessionsDomainError> {
        match agent_id {
            "claude-code" => Ok(Self::Claude),
            "codex-cli" => Ok(Self::Codex),
            "gemini-cli" => Ok(Self::Gemini),
            "opencode" => Ok(Self::OpenCode),
            value => Err(SessionsDomainError::UnsupportedChatAgent(value.to_string())),
        }
    }

    fn provider(self) -> &'static str {
        match self {
            Self::Claude => "anthropic",
            Self::Codex => "openai",
            Self::Gemini => "google",
            Self::OpenCode => "opencode",
        }
    }

    fn default_model(self) -> &'static str {
        match self {
            Self::Claude => "claude-opus-4-8",
            Self::Codex => "gpt-5-5",
            Self::Gemini => "gemini-2-5-pro",
            Self::OpenCode => "opencode-default",
        }
    }

    fn supports(self, model_id: &str) -> bool {
        matches!(
            (self, model_id),
            (
                Self::Claude,
                "claude-opus-4-8" | "claude-sonnet-5" | "claude-sonnet-4-6" | "claude-haiku-4-5"
            ) | (
                Self::Codex,
                "gpt-5-5" | "gpt-5-4" | "gpt-5-2-codex" | "gpt-5-1-codex-max"
            ) | (Self::Gemini, "gemini-2-5-pro" | "gemini-2-5-flash")
                | (Self::OpenCode, "opencode-default")
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PermissionMode {
    Default,
    Plan,
    Agent,
    Auto,
}

impl PermissionMode {
    fn parse(value: &str) -> Result<Self, SessionsDomainError> {
        match value {
            "default" => Ok(Self::Default),
            "plan" => Ok(Self::Plan),
            "agent" => Ok(Self::Agent),
            "auto" => Ok(Self::Auto),
            _ => Err(SessionsDomainError::UnsupportedPermissionMode),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Plan => "plan",
            Self::Agent => "agent",
            Self::Auto => "auto",
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
    pub(crate) permission_mode: &'a str,
    pub(crate) provider_id: Option<&'a str>,
    pub(crate) model_id: Option<&'a str>,
    pub(crate) reasoning_depth: Option<&'a str>,
    pub(crate) streaming: bool,
    pub(crate) thinking: bool,
    pub(crate) long_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatPreferences {
    permission_mode: String,
    provider_id: String,
    model_id: String,
    reasoning_depth: Option<String>,
    streaming: bool,
    thinking: bool,
    long_context: bool,
}

impl ChatPreferences {
    pub(crate) fn permission_mode(&self) -> &str {
        &self.permission_mode
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

pub(crate) fn provider_for_agent(agent_id: &str) -> Result<&'static str, SessionsDomainError> {
    ChatAgent::parse(agent_id).map(ChatAgent::provider)
}

pub(crate) fn default_model_for_agent(agent_id: &str) -> Result<&'static str, SessionsDomainError> {
    ChatAgent::parse(agent_id).map(ChatAgent::default_model)
}

pub(crate) fn model_id_from_cli(agent_id: &str, model: &str) -> Option<&'static str> {
    match (agent_id, model) {
        ("claude-code", "opus") => Some("claude-opus-4-8"),
        ("claude-code", "sonnet") => Some("claude-sonnet-5"),
        ("claude-code", "haiku") => Some("claude-haiku-4-5"),
        ("codex-cli", "gpt-5.5") => Some("gpt-5-5"),
        ("codex-cli", "gpt-5.4") => Some("gpt-5-4"),
        ("codex-cli", "gpt-5.2-codex") => Some("gpt-5-2-codex"),
        ("codex-cli", "gpt-5.1-codex-max") => Some("gpt-5-1-codex-max"),
        ("gemini-cli", "gemini-2.5-pro") => Some("gemini-2-5-pro"),
        ("gemini-cli", "gemini-2.5-flash") => Some("gemini-2-5-flash"),
        _ => None,
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
    request: ChatConfigurationRequest<'_>,
) -> Result<ChatPreferences, SessionsDomainError> {
    let agent = ChatAgent::parse(agent_id)?;
    let permission_mode = PermissionMode::parse(request.permission_mode)?;
    let expected_provider = agent.provider();
    let provider_id = request.provider_id.unwrap_or(expected_provider);
    if provider_id != expected_provider {
        return Err(SessionsDomainError::ProviderMismatch {
            provider_id: provider_id.to_string(),
            agent_id: agent_id.to_string(),
        });
    }
    let model_id = request.model_id.unwrap_or_else(|| agent.default_model());
    if !agent.supports(model_id) {
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
        permission_mode: permission_mode.as_str().to_string(),
        provider_id: expected_provider.to_string(),
        model_id: model_id.to_string(),
        reasoning_depth: clamp_reasoning_for_model(model_id, request.reasoning_depth),
        streaming: request.streaming,
        thinking: request.thinking,
        long_context: request.long_context,
    })
}

pub(crate) fn is_valid_chat_snapshot(
    agent_id: &str,
    permission_mode: &str,
    provider_id: &str,
    model_id: &str,
    reasoning_depth: Option<&str>,
) -> bool {
    let Ok(agent) = ChatAgent::parse(agent_id) else {
        return false;
    };
    provider_id == agent.provider()
        && agent.supports(model_id)
        && PermissionMode::parse(permission_mode).is_ok()
        && reasoning_depth.is_none_or(|depth| ReasoningDepth::parse(depth).is_ok())
}

pub(crate) fn restore_chat_preferences(
    agent_id: &str,
    request: ChatConfigurationRequest<'_>,
) -> Option<ChatPreferences> {
    let provider_id = request.provider_id?;
    let model_id = request.model_id?;
    if !is_valid_chat_snapshot(
        agent_id,
        request.permission_mode,
        provider_id,
        model_id,
        request.reasoning_depth,
    ) {
        return None;
    }
    Some(ChatPreferences {
        permission_mode: request.permission_mode.to_string(),
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
            permission_mode: "agent",
            provider_id: Some("google"),
            model_id: Some("gemini-2-5-flash"),
            reasoning_depth: Some("max"),
            streaming: true,
            thinking: true,
            long_context: true,
        }
    }

    #[test]
    fn configuration_identity_and_reasoning_rules_are_agent_authoritative() {
        let preferences = normalize_chat_preferences("gemini-cli", request()).expect("preferences");

        assert_eq!(preferences.permission_mode(), "agent");
        assert_eq!(preferences.provider_id(), "google");
        assert_eq!(preferences.model_id(), "gemini-2-5-flash");
        assert_eq!(preferences.reasoning_depth(), Some("medium"));
        assert!(preferences.streaming());
        assert!(preferences.thinking());
        assert!(preferences.long_context());
        assert_eq!(provider_for_agent("codex-cli"), Ok("openai"));
        assert_eq!(default_model_for_agent("codex-cli"), Ok("gpt-5-5"));
        assert_eq!(
            model_id_from_cli("claude-code", "sonnet"),
            Some("claude-sonnet-5")
        );
    }

    #[test]
    fn invalid_permission_provider_model_and_reasoning_are_rejected() {
        let mut invalid = request();
        invalid.permission_mode = "unrestricted";
        assert_eq!(
            normalize_chat_preferences("gemini-cli", invalid),
            Err(SessionsDomainError::UnsupportedPermissionMode)
        );

        let mut invalid = request();
        invalid.provider_id = Some("openai");
        assert!(matches!(
            normalize_chat_preferences("gemini-cli", invalid),
            Err(SessionsDomainError::ProviderMismatch { .. })
        ));

        let mut invalid = request();
        invalid.model_id = Some("gpt-5-5");
        assert!(matches!(
            normalize_chat_preferences("gemini-cli", invalid),
            Err(SessionsDomainError::UnsupportedModel { .. })
        ));

        let mut invalid = request();
        invalid.reasoning_depth = Some("extreme");
        assert_eq!(
            normalize_chat_preferences("gemini-cli", invalid),
            Err(SessionsDomainError::UnsupportedReasoningDepth)
        );
    }

    #[test]
    fn persisted_snapshot_validation_preserves_the_existing_fallback_boundary() {
        assert!(is_valid_chat_snapshot(
            "claude-code",
            "plan",
            "anthropic",
            "claude-sonnet-5",
            Some("high")
        ));
        assert!(!is_valid_chat_snapshot(
            "claude-code",
            "plan",
            "openai",
            "claude-sonnet-5",
            Some("high")
        ));
        assert_eq!(normalize_reasoning(Some("invalid")), None);
        let restored = restore_chat_preferences("gemini-cli", request()).expect("snapshot");
        assert_eq!(restored.reasoning_depth(), Some("max"));
    }
}
