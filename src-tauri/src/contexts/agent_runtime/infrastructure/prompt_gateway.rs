use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, EffectivePrompt, EffectivePromptGateway, PromptTrace,
};
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;

#[derive(Clone)]
pub(crate) struct RuntimeEffectivePromptAdapter {
    prompts: PromptHookApi,
}

impl RuntimeEffectivePromptAdapter {
    pub(crate) fn new(prompts: PromptHookApi) -> Self {
        Self { prompts }
    }
}

impl EffectivePromptGateway for RuntimeEffectivePromptAdapter {
    fn assemble(
        &self,
        agent_id: &str,
        session_id: &str,
        user_prompt: &str,
    ) -> Result<EffectivePrompt, AgentRuntimeApplicationError> {
        let assembled = self
            .prompts
            .effective_prompt(agent_id, Some(session_id), user_prompt)
            .map_err(|error| AgentRuntimeApplicationError::Prompt(error.to_string()))?;
        Ok(EffectivePrompt {
            content: assembled.effective_prompt,
            trace: assembled
                .trace
                .into_iter()
                .map(|trace| PromptTrace {
                    hook_id: trace.hook_id.as_str().to_string(),
                    status: trace.status.as_str().to_string(),
                    content_hash: trace.content_hash,
                    token_estimate: trace
                        .token_estimate
                        .and_then(|value| usize::try_from(value).ok()),
                    reason: trace.reason,
                })
                .collect(),
        })
    }
}
