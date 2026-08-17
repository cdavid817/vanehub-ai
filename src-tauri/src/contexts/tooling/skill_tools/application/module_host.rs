use super::{
    SkillToolApplicationError, SkillToolCapabilityModePort, SkillToolDispatchOutcome,
    SkillToolExecutionMode, SkillToolHostDispatchPort, SkillToolInvocationBudgetPort,
    SkillToolModuleHostCallPort, SkillToolPrincipal,
};
use crate::contexts::tooling::skill_tools::domain::{SkillToolCapability, SkillToolLimits};
use serde::Deserialize;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SkillToolModuleHostRequest {
    capability: String,
    arguments: Value,
}

pub(crate) struct SkillToolModuleHostDispatcher {
    principal: SkillToolPrincipal,
    mode: SkillToolExecutionMode,
    declared: Vec<SkillToolCapability>,
    limits: SkillToolLimits,
    host: Arc<dyn SkillToolHostDispatchPort>,
    modes: Arc<dyn SkillToolCapabilityModePort>,
    budget: Arc<dyn SkillToolInvocationBudgetPort>,
    cancelled: Arc<AtomicBool>,
}

impl SkillToolModuleHostDispatcher {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        principal: SkillToolPrincipal,
        mode: SkillToolExecutionMode,
        declared: Vec<SkillToolCapability>,
        limits: SkillToolLimits,
        host: Arc<dyn SkillToolHostDispatchPort>,
        modes: Arc<dyn SkillToolCapabilityModePort>,
        budget: Arc<dyn SkillToolInvocationBudgetPort>,
        cancelled: Arc<AtomicBool>,
    ) -> Self {
        Self {
            principal,
            mode,
            declared,
            limits,
            host,
            modes,
            budget,
            cancelled,
        }
    }
}

impl SkillToolModuleHostCallPort for SkillToolModuleHostDispatcher {
    fn call(&self, request: &Value) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError> {
        if self.cancelled.load(Ordering::Acquire) {
            return Ok(SkillToolDispatchOutcome::Cancelled);
        }
        let request: SkillToolModuleHostRequest = serde_json::from_value(request.clone())
            .map_err(|_| SkillToolApplicationError::HostDenied("host-call-request".to_string()))?;
        let capability = SkillToolCapability::parse(&request.capability)?;
        if !self.declared.contains(&capability) {
            return Ok(denied("capability-not-declared"));
        }
        if !self.modes.allows(self.mode, &capability) {
            return Ok(denied("execution-mode"));
        }
        let marker = capability.as_declaration();
        if self
            .principal
            .delegation_chain
            .iter()
            .any(|ancestor| ancestor == &marker)
        {
            return Ok(denied("delegation-cycle"));
        }
        if self.principal.delegation_chain.len() as u32 >= self.limits.delegation_depth {
            return Ok(denied("delegation-depth"));
        }
        self.budget.reserve_host_call()?;
        self.host
            .dispatch(&self.principal, &capability, &request.arguments)
    }
}

fn denied(reason: &str) -> SkillToolDispatchOutcome {
    SkillToolDispatchOutcome::Denied {
        reason: reason.to_string(),
    }
}

#[cfg(test)]
#[path = "module_host_tests.rs"]
mod tests;
