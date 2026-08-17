use crate::contexts::agent_runtime::application::{
    NativeToolExecutionContext, NativeToolExecutionMode, NativeToolProgress,
    NativeToolProgressSink, NativeToolRegistry, NativeToolResultStatus, ToolEligibility,
    ToolEligibilityContext,
};
use crate::contexts::tooling::skill_tools::application::{
    SkillToolApplicationError, SkillToolApprovalPort, SkillToolCapabilityModePort,
    SkillToolDeclarativeDispatcher, SkillToolDeclarativeValidator, SkillToolDispatchOutcome,
    SkillToolEffectiveDiscoveryPort, SkillToolExecutionLifecyclePhase, SkillToolExecutionMode,
    SkillToolExecutionPort, SkillToolExecutionRequest, SkillToolHostDispatchPort,
    SkillToolLogAction, SkillToolLogEvent, SkillToolLogLevel, SkillToolLoggingPort,
    SkillToolModuleHostDispatcher, SkillToolModuleOutcome, SkillToolModuleRuntime,
    SkillToolPayloadValidator, SkillToolPermissionDecision, SkillToolPermissionPort,
    SkillToolPrincipal, SkillToolRegistry, SkillToolStateRepository, SkillToolTargetCatalogPort,
};
use crate::contexts::tooling::skill_tools::domain::{SkillToolCapability, SkillToolImplementation};
use crate::contexts::tooling::skill_tools::infrastructure::{
    BoundedSkillToolSchemaValidator, SkillToolInvocationBudget,
};
use crate::contexts::tooling::skills::api::SkillApi;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) struct NativeSkillToolExecutionAdapter {
    registry: Arc<SkillToolRegistry>,
    discovery: Arc<dyn SkillToolEffectiveDiscoveryPort>,
    states: Arc<dyn SkillToolStateRepository>,
    native: NativeToolRegistry,
    permissions: Arc<dyn SkillToolPermissionPort>,
    approvals: Arc<dyn SkillToolApprovalPort>,
    logging: Arc<dyn SkillToolLoggingPort>,
    module_runtime: Arc<dyn SkillToolModuleRuntime>,
    skills: SkillApi,
}

pub(crate) struct NativeSkillToolExecutionDependencies {
    pub(crate) registry: Arc<SkillToolRegistry>,
    pub(crate) discovery: Arc<dyn SkillToolEffectiveDiscoveryPort>,
    pub(crate) states: Arc<dyn SkillToolStateRepository>,
    pub(crate) native: NativeToolRegistry,
    pub(crate) permissions: Arc<dyn SkillToolPermissionPort>,
    pub(crate) approvals: Arc<dyn SkillToolApprovalPort>,
    pub(crate) logging: Arc<dyn SkillToolLoggingPort>,
    pub(crate) module_runtime: Arc<dyn SkillToolModuleRuntime>,
    pub(crate) skills: SkillApi,
}

impl NativeSkillToolExecutionAdapter {
    pub(crate) fn new(dependencies: NativeSkillToolExecutionDependencies) -> Self {
        Self {
            registry: dependencies.registry,
            discovery: dependencies.discovery,
            states: dependencies.states,
            native: dependencies.native,
            permissions: dependencies.permissions,
            approvals: dependencies.approvals,
            logging: dependencies.logging,
            module_runtime: dependencies.module_runtime,
            skills: dependencies.skills,
        }
    }
}

impl SkillToolExecutionPort for NativeSkillToolExecutionAdapter {
    fn execute(
        &self,
        request: SkillToolExecutionRequest<'_>,
    ) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError> {
        let pin = self.registry.pin_invocation(request.key)?;
        self.log(request.key, SkillToolLogAction::Invocation, "started");
        if request.cancelled.load(Ordering::Acquire) || pin.cancelled.load(Ordering::Acquire) {
            self.log(request.key, SkillToolLogAction::Cancellation, "cancelled");
            return Ok(SkillToolDispatchOutcome::Cancelled);
        }
        let state = self
            .states
            .revision_state(&request.key.revision)?
            .filter(|state| state.key == *request.key)
            .ok_or(SkillToolApplicationError::StaleRevision)?;
        if !state
            .lifecycle
            .availability(
                false,
                crate::contexts::tooling::skill_tools::MODULE_RUNTIME_ENABLED,
                state.implementation_kind == "wasm",
            )
            .is_available()
        {
            return Ok(SkillToolDispatchOutcome::Denied {
                reason: "tool-unavailable".into(),
            });
        }
        let owner = crate::contexts::tooling::skill_tools::application::SkillToolPackageRef {
            owner: request.key.owner.clone(),
            source: request.key.source.clone(),
            base_revision: state.integrity.base_revision.clone(),
            root_path: String::new(),
        };
        let (package, discovered, _) = self.discovery.discover_effective(&owner)?;
        let tool = discovered
            .discovered
            .into_iter()
            .find(|tool| tool.key == *request.key && tool.integrity == state.integrity)
            .ok_or(SkillToolApplicationError::StaleRevision)?;
        let principal = SkillToolPrincipal::new(
            request.parent_agent_id,
            request.key.clone(),
            request.workspace_path,
            request.session_id,
            request.generation_id,
            Vec::new(),
        )?;
        let host = Arc::new(NativeHostDispatch {
            native: self.native.clone(),
            permissions: self.permissions.clone(),
            approvals: self.approvals.clone(),
            call_id: request.call_id.to_string(),
            mode: request.mode,
            logging: self.logging.clone(),
            key: request.key.clone(),
        });
        let schemas = BoundedSkillToolSchemaValidator;
        let budget = SkillToolInvocationBudget::new(tool.declaration.limits);
        let outcome = match &tool.declaration.implementation {
            SkillToolImplementation::Declarative(implementation) => {
                let template =
                    SkillToolDeclarativeValidator::new(host.as_ref()).validate(implementation)?;
                let dispatcher = SkillToolDeclarativeDispatcher::new(
                    host.as_ref(),
                    host.as_ref(),
                    SkillToolPayloadValidator::new(&schemas),
                );
                dispatcher.dispatch(
                    &principal,
                    execution_mode(request.mode),
                    &tool.declaration.capabilities,
                    &template,
                    &tool.declaration.input,
                    &tool.declaration.output,
                    request.input,
                    tool.declaration.limits,
                    &budget,
                )?
            }
            SkillToolImplementation::Module(module) => {
                let payloads = SkillToolPayloadValidator::new(&schemas);
                if let Err(outcome) = payloads.validate_input(
                    &tool.declaration.input,
                    request.input,
                    tool.declaration.limits.input_bytes,
                ) {
                    outcome
                } else {
                    let host_calls = Arc::new(SkillToolModuleHostDispatcher::new(
                        principal.clone(),
                        execution_mode(request.mode),
                        tool.declaration.capabilities.clone(),
                        tool.declaration.limits,
                        host.clone(),
                        host.clone(),
                        Arc::new(budget.clone()),
                        pin.cancelled.clone(),
                    ));
                    match self.module_runtime.invoke(
                        &package.effective,
                        request.key,
                        module,
                        &module.export,
                        request.input,
                        &tool.declaration.limits,
                        pin.cancelled.as_ref(),
                        host_calls,
                    )? {
                        SkillToolModuleOutcome::Completed(value) => payloads
                            .validate_output(
                                &tool.declaration.output,
                                &value,
                                tool.declaration.limits.output_bytes,
                            )
                            .map(|_| SkillToolDispatchOutcome::Completed(value))
                            .unwrap_or_else(|outcome| outcome),
                        SkillToolModuleOutcome::LimitBreached { .. } => {
                            SkillToolDispatchOutcome::Failed {
                                code: "module-limit-breached".into(),
                            }
                        }
                        SkillToolModuleOutcome::Trapped { .. } => {
                            SkillToolDispatchOutcome::Failed {
                                code: "module-trapped".into(),
                            }
                        }
                        SkillToolModuleOutcome::Cancelled => SkillToolDispatchOutcome::Cancelled,
                    }
                }
            }
        };
        let label = match &outcome {
            SkillToolDispatchOutcome::Completed(_) => "completed",
            SkillToolDispatchOutcome::Denied { .. } => "denied",
            SkillToolDispatchOutcome::Failed { .. } => "failed",
            SkillToolDispatchOutcome::Cancelled => "cancelled",
        };
        if matches!(
            &outcome,
            SkillToolDispatchOutcome::Denied { reason } if reason == "approval-pending"
        ) {
            request
                .lifecycle
                .transition(SkillToolExecutionLifecyclePhase::AwaitingApproval);
        }
        self.log(request.key, SkillToolLogAction::Invocation, label);
        if matches!(outcome, SkillToolDispatchOutcome::Completed(_)) {
            let _ = self.skills.record_tool_use(
                request.key.owner.as_str(),
                request.key.source.workspace_path.as_deref(),
                &state.integrity.base_revision,
            );
        }
        Ok(outcome)
    }
}

impl NativeSkillToolExecutionAdapter {
    fn log(
        &self,
        key: &crate::contexts::tooling::skill_tools::domain::SkillToolKey,
        action: SkillToolLogAction,
        outcome: &str,
    ) {
        let _ = self.logging.record(&SkillToolLogEvent {
            action,
            level: SkillToolLogLevel::Info,
            skill_id: Some(key.owner.as_str().into()),
            tool_id: Some(key.tool.as_str().into()),
            revision: Some(key.revision.as_str().into()),
            message: action.as_str().into(),
            context: [("outcome".into(), outcome.into())].into_iter().collect(),
        });
    }
}

struct NativeHostDispatch {
    native: NativeToolRegistry,
    permissions: Arc<dyn SkillToolPermissionPort>,
    approvals: Arc<dyn SkillToolApprovalPort>,
    call_id: String,
    mode: crate::contexts::tooling::skill_tools::application::SkillToolCatalogMode,
    logging: Arc<dyn SkillToolLoggingPort>,
    key: crate::contexts::tooling::skill_tools::domain::SkillToolKey,
}

impl SkillToolTargetCatalogPort for NativeHostDispatch {
    fn contains_operation(&self, operation: &str) -> bool {
        self.native.handler(operation).is_some()
    }
}

impl SkillToolCapabilityModePort for NativeHostDispatch {
    fn allows(&self, mode: SkillToolExecutionMode, capability: &SkillToolCapability) -> bool {
        let context = eligibility_context("skill-tool", "skill-tool", "skill-tool", None, mode);
        self.native.eligibility(capability.operation(), &context) == ToolEligibility::Eligible
    }
}

impl SkillToolHostDispatchPort for NativeHostDispatch {
    fn dispatch(
        &self,
        principal: &SkillToolPrincipal,
        capability: &SkillToolCapability,
        arguments: &Value,
    ) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError> {
        let decision = self.permissions.evaluate(principal, capability, arguments);
        self.log(
            SkillToolLogAction::Permission,
            match decision {
                SkillToolPermissionDecision::Allow => "allow",
                SkillToolPermissionDecision::Ask => "ask",
                SkillToolPermissionDecision::Deny => "deny",
            },
        );
        match decision {
            SkillToolPermissionDecision::Deny => {
                return Ok(SkillToolDispatchOutcome::Denied {
                    reason: "permission-denied".into(),
                })
            }
            SkillToolPermissionDecision::Ask => {
                self.approvals
                    .create_pending(principal, capability, arguments, &self.call_id)?;
                self.log(SkillToolLogAction::Approval, "pending");
                return Ok(SkillToolDispatchOutcome::Denied {
                    reason: "approval-pending".into(),
                });
            }
            SkillToolPermissionDecision::Allow => {}
        }
        let handler = self.native.handler(capability.operation()).ok_or_else(|| {
            SkillToolApplicationError::HostDenied("unknown-host-operation".into())
        })?;
        let mode = execution_mode(self.mode);
        let context = eligibility_context(
            &principal.parent_agent_id,
            principal.session_id.as_deref().unwrap_or(""),
            &principal.generation_id,
            principal.workspace_path.as_deref(),
            mode,
        );
        if self.native.eligibility(capability.operation(), &context) != ToolEligibility::Eligible {
            return Ok(SkillToolDispatchOutcome::Denied {
                reason: "execution-mode".into(),
            });
        }
        let input = handler
            .validate(arguments)
            .map_err(|error| SkillToolApplicationError::HostDenied(error.code.as_str().into()))?;
        let result = handler.execute(
            input,
            NativeToolExecutionContext {
                call_id: self.call_id.clone(),
                session_id: context.session_id,
                generation_id: context.generation_id,
                agent_id: context.agent_id,
                canonical_workspace: context.canonical_workspace,
                deadline: Instant::now() + Duration::from_secs(10),
                cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                progress: Arc::new(NoopProgress),
            },
        );
        self.log(
            SkillToolLogAction::HostCall,
            match result.status {
                NativeToolResultStatus::Succeeded => "completed",
                NativeToolResultStatus::Cancelled => "cancelled",
                NativeToolResultStatus::LimitExceeded => "limit",
                _ => "failed",
            },
        );
        Ok(match result.status {
            NativeToolResultStatus::Succeeded => {
                SkillToolDispatchOutcome::Completed(result.output.unwrap_or(Value::Null))
            }
            NativeToolResultStatus::Cancelled => SkillToolDispatchOutcome::Cancelled,
            NativeToolResultStatus::Denied | NativeToolResultStatus::Unavailable => {
                SkillToolDispatchOutcome::Denied {
                    reason: result
                        .error_code
                        .map_or_else(|| "host-denied".into(), |code| code.as_str().into()),
                }
            }
            _ => SkillToolDispatchOutcome::Failed {
                code: result
                    .error_code
                    .map_or_else(|| "host-failed".into(), |code| code.as_str().into()),
            },
        })
    }
}

impl NativeHostDispatch {
    fn log(&self, action: SkillToolLogAction, outcome: &str) {
        let _ = self.logging.record(&SkillToolLogEvent {
            action,
            level: SkillToolLogLevel::Info,
            skill_id: Some(self.key.owner.as_str().into()),
            tool_id: Some(self.key.tool.as_str().into()),
            revision: Some(self.key.revision.as_str().into()),
            message: action.as_str().into(),
            context: [("outcome".into(), outcome.into())].into_iter().collect(),
        });
    }
}

fn execution_mode(
    mode: crate::contexts::tooling::skill_tools::application::SkillToolCatalogMode,
) -> SkillToolExecutionMode {
    match mode {
        crate::contexts::tooling::skill_tools::application::SkillToolCatalogMode::Plan => {
            SkillToolExecutionMode::Plan
        }
        crate::contexts::tooling::skill_tools::application::SkillToolCatalogMode::Execute => {
            SkillToolExecutionMode::Execute
        }
    }
}

fn eligibility_context(
    agent: &str,
    session: &str,
    generation: &str,
    workspace: Option<&str>,
    mode: SkillToolExecutionMode,
) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: agent.into(),
        session_id: session.into(),
        generation_id: generation.into(),
        canonical_workspace: workspace.map(PathBuf::from),
        execution_mode: match mode {
            SkillToolExecutionMode::Plan => NativeToolExecutionMode::Plan,
            SkillToolExecutionMode::Execute => NativeToolExecutionMode::Execute,
        },
        readiness: BTreeMap::new(),
    }
}

#[derive(Debug)]
struct NoopProgress;
impl NativeToolProgressSink for NoopProgress {
    fn publish(&self, _progress: NativeToolProgress) {}
}
