use std::{future::Future, pin::Pin};

use crate::contexts::{
    agent_runtime::api::{
        AgentRuntimeApi, StructuredModelEvaluationError, StructuredModelEvaluationRequest,
        StructuredModelPurpose,
    },
    skill_evolution_generation::application::{
        GenerationModelError, GenerationModelInvocationV1, GenerationModelResponseV1,
        GenerationStructuredModelPort,
    },
};

#[derive(Clone)]
pub(crate) struct ConfiguredGenerationModel {
    runtime: AgentRuntimeApi,
}

impl ConfiguredGenerationModel {
    pub(crate) fn new(runtime: AgentRuntimeApi) -> Self {
        Self { runtime }
    }
}

impl GenerationStructuredModelPort for ConfiguredGenerationModel {
    fn evaluate<'a>(
        &'a self,
        request: GenerationModelInvocationV1,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GenerationModelResponseV1, GenerationModelError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            self.runtime
                .evaluate_structured_model(StructuredModelEvaluationRequest {
                    purpose: StructuredModelPurpose::SkillEvolutionGeneration,
                    required_profile_id: Some(request.required_profile_id),
                    required_model_id: Some(request.required_model_id),
                    system_instruction: request.system_instruction,
                    sanitized_json: request.sanitized_json,
                    max_input_characters: request.max_input_characters,
                    max_output_tokens: request.max_output_tokens,
                    timeout_ms: request.timeout_ms,
                })
                .map(|result| GenerationModelResponseV1 {
                    profile_id: result.profile_id,
                    provider_protocol: result.provider_protocol,
                    model_id: result.model_id,
                    response_json: result.response_json,
                })
                .map_err(map_error)
        })
    }
}

fn map_error(error: StructuredModelEvaluationError) -> GenerationModelError {
    match error {
        StructuredModelEvaluationError::ProviderUnavailable => {
            GenerationModelError::ProviderUnavailable
        }
        StructuredModelEvaluationError::Timeout => GenerationModelError::Timeout,
        StructuredModelEvaluationError::RateLimited => GenerationModelError::RateLimited,
        StructuredModelEvaluationError::InvalidRequest => GenerationModelError::InvalidRequest,
        StructuredModelEvaluationError::ProviderFailure => GenerationModelError::ProviderFailure,
    }
}
