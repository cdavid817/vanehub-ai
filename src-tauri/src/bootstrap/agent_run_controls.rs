use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::operations::api::{AgentRun, AgentRunsApi, OperationsError};
use crate::contexts::task_orchestration::api::TaskOrchestrationApi;

#[derive(Clone)]
pub(crate) struct AgentRunControlsApi {
    runs: AgentRunsApi,
    agents: AgentRuntimeApi,
    plans: TaskOrchestrationApi,
}

impl AgentRunControlsApi {
    pub(crate) fn new(
        runs: AgentRunsApi,
        agents: AgentRuntimeApi,
        plans: TaskOrchestrationApi,
    ) -> Self {
        Self {
            runs,
            agents,
            plans,
        }
    }

    pub(crate) fn cancel(&self, run_id: &str, version: u64) -> Result<AgentRun, OperationsError> {
        let run = self.runs.get(run_id)?;
        if run.version != version {
            return self.runs.cancel(run_id, version);
        }
        match run.owner.owner_type.as_str() {
            "loop_run" => {
                self.agents.cancel_loop(&run.owner.owner_id).map_err(|_| {
                    OperationsError::Internal("run owner cancellation failed".into())
                })?;
            }
            "plan_run" => {
                self.plans
                    .cancel_plan_run(&run.owner.owner_id, &chrono::Utc::now().to_rfc3339())
                    .map_err(|_| {
                        OperationsError::Internal("run owner cancellation failed".into())
                    })?;
            }
            "session_generation" => {
                let session_id = run
                    .links
                    .iter()
                    .find(|link| link.link_type == "session")
                    .map(|link| link.link_id.as_str())
                    .ok_or_else(|| {
                        OperationsError::Invalid("agent run is missing its session link".into())
                    })?;
                self.agents.stop_generation(session_id).map_err(|_| {
                    OperationsError::Internal("run owner cancellation failed".into())
                })?;
            }
            _ => return self.runs.cancel(run_id, version),
        }
        self.runs.get(run_id)
    }

    pub(crate) fn resume(&self, run_id: &str, version: u64) -> Result<AgentRun, OperationsError> {
        let run = self.runs.get(run_id)?;
        if run.version != version {
            return self.runs.resume(run_id, version);
        }
        match run.owner.owner_type.as_str() {
            "loop_run" => {
                self.agents
                    .resume_loop(&run.owner.owner_id)
                    .map_err(|_| OperationsError::Internal("run owner resume failed".into()))?;
            }
            "plan_run" => {
                self.plans
                    .resume_plan_run(&run.owner.owner_id, &chrono::Utc::now().to_rfc3339())
                    .map_err(|_| OperationsError::Internal("run owner resume failed".into()))?;
            }
            _ => return self.runs.resume(run_id, version),
        }
        self.runs.get(run_id)
    }
}
