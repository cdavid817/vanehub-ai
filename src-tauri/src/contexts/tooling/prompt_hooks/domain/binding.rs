use super::PromptHookDomainError;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ManagedCliAgentId {
    ClaudeCode,
    CodexCli,
    GeminiCli,
    OpenCode,
}

impl ManagedCliAgentId {
    pub(crate) const ALL: [Self; 4] = [
        Self::ClaudeCode,
        Self::CodexCli,
        Self::GeminiCli,
        Self::OpenCode,
    ];

    pub(crate) fn parse(value: &str) -> Result<Self, PromptHookDomainError> {
        Self::ALL
            .into_iter()
            .find(|agent_id| agent_id.as_str() == value)
            .ok_or_else(|| PromptHookDomainError::UnsupportedAgent(value.to_string()))
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::CodexCli => "codex-cli",
            Self::GeminiCli => "gemini-cli",
            Self::OpenCode => "opencode",
        }
    }

    pub(crate) fn display_name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::CodexCli => "Codex CLI",
            Self::GeminiCli => "Gemini CLI",
            Self::OpenCode => "OpenCode",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PromptHookBindings(Vec<ManagedCliAgentId>);

impl PromptHookBindings {
    pub(crate) fn new(values: &[String]) -> Result<Self, PromptHookDomainError> {
        let mut seen = HashSet::new();
        let mut bindings = Vec::new();
        for value in values {
            let agent_id = ManagedCliAgentId::parse(value)?;
            if seen.insert(agent_id) {
                bindings.push(agent_id);
            }
        }
        Ok(Self(bindings))
    }

    pub(crate) fn all() -> Self {
        Self(ManagedCliAgentId::ALL.to_vec())
    }

    pub(crate) fn contains(&self, agent_id: ManagedCliAgentId) -> bool {
        self.0.contains(&agent_id)
    }

    pub(crate) fn to_strings(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|agent_id| agent_id.as_str().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_accept_only_stable_ids_and_deduplicate_in_input_order() {
        let bindings = PromptHookBindings::new(&[
            "codex-cli".to_string(),
            "claude-code".to_string(),
            "codex-cli".to_string(),
        ])
        .expect("bindings");
        assert_eq!(bindings.to_strings(), ["codex-cli", "claude-code"]);
        assert!(bindings.contains(ManagedCliAgentId::CodexCli));
        assert_eq!(
            PromptHookBindings::new(&["Codex".to_string()]),
            Err(PromptHookDomainError::UnsupportedAgent("Codex".to_string()))
        );
    }

    #[test]
    fn builtin_binding_set_contains_exactly_the_four_managed_clis() {
        assert_eq!(
            PromptHookBindings::all().to_strings(),
            ["claude-code", "codex-cli", "gemini-cli", "opencode"]
        );
    }
}
