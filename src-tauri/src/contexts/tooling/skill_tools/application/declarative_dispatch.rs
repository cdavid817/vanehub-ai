use super::{
    SkillToolApplicationError, SkillToolDispatchOutcome, SkillToolHostDispatchPort,
    SkillToolInvocationBudgetPort, SkillToolPayloadValidator, SkillToolPrincipal,
    ValidatedDeclarativeTemplate,
};
use crate::contexts::tooling::skill_tools::domain::{
    BoundedJsonSchema, SkillToolCapability, SkillToolLimits,
};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolExecutionMode {
    Plan,
    Execute,
}

pub(crate) trait SkillToolCapabilityModePort: Send + Sync {
    fn allows(&self, mode: SkillToolExecutionMode, capability: &SkillToolCapability) -> bool;
}

pub(crate) struct SkillToolDeclarativeDispatcher<'a> {
    host: &'a dyn SkillToolHostDispatchPort,
    modes: &'a dyn SkillToolCapabilityModePort,
    payloads: SkillToolPayloadValidator<'a>,
}

impl<'a> SkillToolDeclarativeDispatcher<'a> {
    pub(crate) fn new(
        host: &'a dyn SkillToolHostDispatchPort,
        modes: &'a dyn SkillToolCapabilityModePort,
        payloads: SkillToolPayloadValidator<'a>,
    ) -> Self {
        Self {
            host,
            modes,
            payloads,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch(
        &self,
        principal: &SkillToolPrincipal,
        mode: SkillToolExecutionMode,
        declared_capabilities: &[SkillToolCapability],
        template: &ValidatedDeclarativeTemplate,
        input_schema: &BoundedJsonSchema,
        output_schema: &BoundedJsonSchema,
        input: &Value,
        limits: SkillToolLimits,
        budget: &dyn SkillToolInvocationBudgetPort,
    ) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError> {
        if let Err(outcome) = self
            .payloads
            .validate_input(input_schema, input, limits.input_bytes)
        {
            return Ok(outcome);
        }
        let target = template.target();
        if !declared_capabilities.contains(target) {
            return Ok(denied("capability-not-declared"));
        }
        if !self.modes.allows(mode, target) {
            return Ok(denied("execution-mode"));
        }
        let marker = target.as_declaration();
        if principal
            .delegation_chain
            .iter()
            .any(|item| item == &marker)
        {
            return Ok(denied("delegation-cycle"));
        }
        if principal.delegation_chain.len() as u32 >= limits.delegation_depth {
            return Ok(denied("delegation-depth"));
        }
        budget.reserve_host_call()?;
        let arguments = template.project(input)?;
        let outcome = self.host.dispatch(principal, target, &arguments)?;
        let SkillToolDispatchOutcome::Completed(output) = outcome else {
            return Ok(outcome);
        };
        let output_bytes =
            match self
                .payloads
                .validate_output(output_schema, &output, limits.output_bytes)
            {
                Ok(bytes) => bytes,
                Err(outcome) => return Ok(outcome),
            };
        budget.consume_output(output_bytes)?;
        Ok(SkillToolDispatchOutcome::Completed(output))
    }
}

fn denied(reason: &str) -> SkillToolDispatchOutcome {
    SkillToolDispatchOutcome::Denied {
        reason: reason.to_string(),
    }
}

#[cfg(test)]
#[path = "declarative_dispatch_tests.rs"]
mod tests;
