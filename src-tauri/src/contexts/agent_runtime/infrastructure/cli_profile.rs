use super::providers::{
    message_override_selections, opencode_standard_permission_env_var, policy_override_selections,
    POLICY_TEMPLATE_GOVERNED_AGENT_IDS,
};
use crate::contexts::agent_runtime::application::{
    resolve_effective_execution_policy, AgentChatConfiguration, AgentCliProfileGateway,
    AgentRuntimeApplicationError, CliProfileSnapshot, SessionExecutionMode,
};
use crate::contexts::permissions::api::{PermissionsApi, PolicyTemplateName};
use crate::contexts::tooling::api::{
    CliLaunchExecutionContext, CliLaunchScope, CliParameterRuntimeApi, CliParameterSelectionMap,
    ResolveCliLaunchParametersInput,
};
use crate::contexts::tooling::cli::api::CliApi;
use std::collections::BTreeMap;

/// Builds the launch profile for every managed CLI process — chat, resume, and Agent Terminal.
///
/// Ordinary parameters are resolved by the Tooling CLI-parameter API, which dual-reads legacy and
/// v2 rows. Policy-governed parameters never travel that path: they are projected here from the
/// agent principal's template and handed to the resolver on a separate input, which refuses a
/// user-editable id on it. Tooling therefore never depends on Permissions.
#[derive(Clone)]
pub(crate) struct RuntimeAgentCliProfileAdapter {
    parameters: CliParameterRuntimeApi,
    cli: CliApi,
    permissions: PermissionsApi,
}

impl RuntimeAgentCliProfileAdapter {
    pub(crate) fn new(
        parameters: CliParameterRuntimeApi,
        cli: CliApi,
        permissions: PermissionsApi,
    ) -> Self {
        Self {
            parameters,
            cli,
            permissions,
        }
    }

    fn snapshot(
        &self,
        agent_id: &str,
        scope: CliLaunchScope,
        message_overrides: CliParameterSelectionMap,
        mode: SessionExecutionMode,
        operation_id: Option<&str>,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        let launch = resolve_launch(
            &self.parameters,
            &self.permissions,
            agent_id,
            scope,
            message_overrides,
            mode,
            operation_id,
        )?;
        let executable = self
            .cli
            .resolve_executable(agent_id)
            .map_err(cli_profile_error)?
            .ok_or_else(|| {
                AgentRuntimeApplicationError::CliProfile(format!(
                    "Agent executable could not be resolved for {agent_id}."
                ))
            })?;
        Ok(CliProfileSnapshot {
            executable,
            global_args: launch.global_args,
            invocation_args: launch.invocation_args,
            env: launch.env,
        })
    }
}

impl AgentCliProfileGateway for RuntimeAgentCliProfileAdapter {
    fn load(
        &self,
        agent_id: &str,
        configuration: &AgentChatConfiguration,
        operation_id: Option<&str>,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        let mode = SessionExecutionMode::parse(&configuration.execution_mode).ok_or_else(|| {
            AgentRuntimeApplicationError::CliProfile(format!(
                "Unsupported session execution mode: {}.",
                configuration.execution_mode
            ))
        })?;
        self.snapshot(
            agent_id,
            CliLaunchScope::Chat,
            message_override_selections(agent_id, configuration),
            mode,
            operation_id,
        )
    }

    /// Interactive terminals inherit the Agent policy directly. Session execution intent applies
    /// only to chat generations, and a terminal carries no per-message override.
    fn load_interactive(
        &self,
        agent_id: &str,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        self.snapshot(
            agent_id,
            CliLaunchScope::Interactive,
            CliParameterSelectionMap::new(),
            SessionExecutionMode::Inherit,
            // An Agent Terminal launch is not an observable operation.
            None,
        )
    }
}

pub(super) struct ResolvedLaunch {
    pub(super) global_args: Vec<String>,
    pub(super) invocation_args: Vec<String>,
    pub(super) env: BTreeMap<String, String>,
}

/// The policy projection plus resolver call, factored out of the gateway so it can be tested
/// against real `CliParameterRuntimeApi`/`PermissionsApi` instances without also needing a fully
/// wired `CliApi` — executable resolution has its own unrelated dependency graph, and pulling it
/// in would make policy-projection tests fragile for reasons unrelated to the logic under test.
#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_launch(
    parameters: &CliParameterRuntimeApi,
    permissions: &PermissionsApi,
    agent_id: &str,
    scope: CliLaunchScope,
    message_overrides: CliParameterSelectionMap,
    mode: SessionExecutionMode,
    operation_id: Option<&str>,
) -> Result<ResolvedLaunch, AgentRuntimeApplicationError> {
    let template = launch_template(permissions, agent_id, mode)?;
    let resolved = parameters
        .resolve_cli_launch_segments(&ResolveCliLaunchParametersInput {
            agent_id: agent_id.to_string(),
            scope,
            message_overrides,
            policy_overrides: policy_override_selections(agent_id, template),
            execution_context: CliLaunchExecutionContext {
                operation_id: operation_id.map(str::to_string),
            },
        })
        .map_err(|error| {
            AgentRuntimeApplicationError::CliProfile(error.code().as_str().to_string())
        })?;
    Ok(ResolvedLaunch {
        global_args: resolved.global_tokens,
        invocation_args: resolved.invocation_tokens,
        env: launch_env(agent_id, template),
    })
}

/// Resolves the policy template that governs this launch. A lookup failure fails the launch rather
/// than guessing a permissive default.
fn launch_template(
    permissions: &PermissionsApi,
    agent_id: &str,
    mode: SessionExecutionMode,
) -> Result<PolicyTemplateName, AgentRuntimeApplicationError> {
    if !POLICY_TEMPLATE_GOVERNED_AGENT_IDS.contains(&agent_id) {
        return Err(AgentRuntimeApplicationError::CliProfile(format!(
            "No execution-policy mapping exists for {agent_id}."
        )));
    }
    let (principal, _has_explicit_assignment) = permissions
        .find_principal(agent_id)
        .map_err(cli_profile_error)?;
    Ok(resolve_effective_execution_policy(principal.template(), mode).launch_template())
}

/// Environment carried alongside argv. Both entries are policy/runtime concerns with no expressible
/// registry parameter, so neither is a catalog entry and neither reaches the settings page.
fn launch_env(agent_id: &str, template: PolicyTemplateName) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    if let Some((key, value)) = opencode_standard_permission_env_var(agent_id, template) {
        env.insert(key.to_string(), value.to_string());
    }
    if agent_id == "claude-code" {
        env.insert("VANEHUB_PERMISSION_HOOK_SCOPE".into(), "managed".into());
    }
    env
}

fn cli_profile_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::CliProfile(error.to_string())
}
