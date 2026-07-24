use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, EffectivePrompt, EffectivePromptGateway, PromptExecutionOutcome,
    PromptExecutionReport, PromptTrace,
};
use crate::contexts::tooling::prompt_hooks::api::{
    ManagedCliAgentId, PromptHookApi, PromptHookExecutionObservation, PromptHookExecutionOutcome,
    PromptHookId,
};

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
                    version: trace.version,
                    content_hash: trace.content_hash,
                    token_estimate: trace
                        .token_estimate
                        .and_then(|value| usize::try_from(value).ok()),
                    reason: trace.reason,
                })
                .collect(),
        })
    }

    fn record_execution(
        &self,
        report: PromptExecutionReport,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let agent_id = ManagedCliAgentId::parse(&report.agent_id)
            .map_err(|error| AgentRuntimeApplicationError::Prompt(error.to_string()))?;
        let outcome = match report.outcome {
            PromptExecutionOutcome::Succeeded => PromptHookExecutionOutcome::Succeeded,
            PromptExecutionOutcome::Failed => PromptHookExecutionOutcome::Failed,
            PromptExecutionOutcome::Cancelled => PromptHookExecutionOutcome::Cancelled,
        };
        let observations = report
            .versions
            .into_iter()
            .map(|reference| {
                Ok(PromptHookExecutionObservation {
                    invocation_id: report.invocation_id.clone(),
                    hook_id: PromptHookId::parse(reference.hook_id)
                        .map_err(|error| AgentRuntimeApplicationError::Prompt(error.to_string()))?,
                    version: reference.version,
                    outcome,
                    elapsed_ms: report.elapsed_ms.max(0),
                    agent_id,
                    created_at: report.created_at.clone(),
                })
            })
            .collect::<Result<Vec<_>, AgentRuntimeApplicationError>>()?;
        self.prompts
            .record_execution_observations(&observations)
            .map_err(|error| AgentRuntimeApplicationError::Prompt(error.to_string()))
    }
}
