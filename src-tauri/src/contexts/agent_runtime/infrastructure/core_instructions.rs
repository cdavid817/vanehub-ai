pub(crate) const ONEPIECE_CORE_VERSION: &str = "1.0.0";
pub(crate) const ONEPIECE_CORE_MARKDOWN: &str = include_str!("onepiece-core-v1.md");

use crate::contexts::agent_runtime::application::{
    AgentCoreInstructions, AgentCoreInstructionsPort, AgentRuntimeApplicationError,
};

#[derive(Default)]
pub(crate) struct NativeAgentCoreInstructionsAdapter;

impl AgentCoreInstructionsPort for NativeAgentCoreInstructionsAdapter {
    fn instructions_for(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentCoreInstructions>, AgentRuntimeApplicationError> {
        Ok((agent_id == "onepiece").then(|| AgentCoreInstructions {
            version: ONEPIECE_CORE_VERSION.to_string(),
            markdown: ONEPIECE_CORE_MARKDOWN.to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_onepiece_core_is_versioned_and_within_budget() {
        assert_eq!(ONEPIECE_CORE_VERSION, "1.0.0");
        assert!(!ONEPIECE_CORE_MARKDOWN.trim().is_empty());
        assert!(ONEPIECE_CORE_MARKDOWN.chars().count() <= 8_000);
    }
}
