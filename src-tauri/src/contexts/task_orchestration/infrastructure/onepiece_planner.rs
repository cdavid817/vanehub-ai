use crate::contexts::agent_runtime::api::{AgentRuntimeApi, OnePiecePlanningRequest};
use crate::contexts::task_orchestration::application::{
    PlanApplicationError, PlanGenerationPort, PlanGenerationRequest, PlanGenerationResponse,
};
const PLANNER_INSTRUCTION_VERSION: u32 = 1;

#[derive(Clone)]
pub(crate) struct OnePiecePlanGenerator {
    agents: AgentRuntimeApi,
}

impl OnePiecePlanGenerator {
    pub(crate) fn new(agents: AgentRuntimeApi) -> Self {
        Self { agents }
    }
}

impl PlanGenerationPort for OnePiecePlanGenerator {
    fn generate(
        &self,
        request: &PlanGenerationRequest,
    ) -> Result<PlanGenerationResponse, PlanApplicationError> {
        let result = self
            .agents
            .generate_onepiece_plan(&OnePiecePlanningRequest {
                instruction_version: PLANNER_INSTRUCTION_VERSION,
                prompt: request.prompt.clone(),
            })
            .map_err(runtime_error)?;
        Ok(PlanGenerationResponse {
            content: result.content,
            active_profile_id: result.profile_id,
        })
    }
}

fn runtime_error(error: impl std::fmt::Display) -> PlanApplicationError {
    PlanApplicationError::Storage(error.to_string())
}
