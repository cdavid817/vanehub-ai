use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, AgentSkillPort, BoundSkillPrompt,
};
use crate::contexts::tooling::skills::api::SkillApi;

/// Wraps `tooling::skills`' public facade to satisfy `agent_runtime`'s own `AgentSkillPort` —
/// mirrors `RuntimeEffectivePromptAdapter`'s existing pattern for depending on another context's
/// API through an `agent_runtime`-owned port rather than that context's types directly.
#[derive(Clone)]
pub(crate) struct RuntimeAgentSkillAdapter {
    skills: SkillApi,
}

impl RuntimeAgentSkillAdapter {
    pub(crate) fn new(skills: SkillApi) -> Self {
        Self { skills }
    }
}

impl AgentSkillPort for RuntimeAgentSkillAdapter {
    fn bound_skill_prompts(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Vec<BoundSkillPrompt>, AgentRuntimeApplicationError> {
        self.skills
            .bound_skill_prompts_for_api_agent(agent_id, workspace_path)
            .map(|prompts| {
                prompts
                    .into_iter()
                    .map(|prompt| BoundSkillPrompt {
                        id: prompt.id,
                        name: prompt.name,
                        body: prompt.body,
                    })
                    .collect()
            })
            .map_err(|error| AgentRuntimeApplicationError::Skill(error.to_string()))
    }
}
