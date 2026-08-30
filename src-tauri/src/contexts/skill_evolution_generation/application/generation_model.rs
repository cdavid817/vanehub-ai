use std::{future::Future, pin::Pin};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationModelStage {
    PlanMutation,
    SynthesizeStructuredDraft,
    RepairStructuredDraft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationModelInvocationV1 {
    pub(crate) stage: GenerationModelStage,
    pub(crate) required_profile_id: String,
    pub(crate) required_model_id: String,
    pub(crate) system_instruction: String,
    pub(crate) sanitized_json: String,
    pub(crate) max_input_characters: usize,
    pub(crate) max_output_tokens: u32,
    pub(crate) timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationModelResponseV1 {
    pub(crate) profile_id: String,
    pub(crate) provider_protocol: String,
    pub(crate) model_id: String,
    pub(crate) response_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationModelError {
    ProviderUnavailable,
    Timeout,
    RateLimited,
    InvalidRequest,
    ProviderFailure,
}

pub(crate) trait GenerationStructuredModelPort: Send + Sync {
    fn evaluate<'a>(
        &'a self,
        request: GenerationModelInvocationV1,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GenerationModelResponseV1, GenerationModelError>>
                + Send
                + 'a,
        >,
    >;
}
