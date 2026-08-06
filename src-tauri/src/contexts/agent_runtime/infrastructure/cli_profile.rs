use super::providers::{
    apply_configuration_overrides, apply_policy_template_overrides,
    force_gemini_standard_approval_flag, opencode_standard_permission_env_var,
    POLICY_TEMPLATE_GOVERNED_AGENT_IDS,
};
use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, AgentCliProfileGateway, AgentRuntimeApplicationError,
    CliProfileSnapshot,
};
use crate::contexts::permissions::api::PermissionsApi;
use crate::contexts::tooling::cli::api::CliApi;
use crate::contexts::tooling::cli_parameters::{CliParameterLaunchScope, CliParametersApi};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) struct RuntimeAgentCliProfileAdapter {
    parameters: CliParametersApi,
    cli: CliApi,
    permissions: PermissionsApi,
}

impl RuntimeAgentCliProfileAdapter {
    pub(crate) fn new(parameters: CliParametersApi, cli: CliApi, permissions: PermissionsApi) -> Self {
        Self {
            parameters,
            cli,
            permissions,
        }
    }
}

impl AgentCliProfileGateway for RuntimeAgentCliProfileAdapter {
    fn load(
        &self,
        agent_id: &str,
        configuration: &AgentChatConfiguration,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        let selections = self
            .parameters
            .load_selections(agent_id)
            .map_err(cli_profile_error)?;
        let selections = apply_configuration_overrides(agent_id, selections, configuration);
        let selections = self
            .parameters
            .normalize_selections(agent_id, &selections)
            .map_err(cli_profile_error)?;
        let managed_args = self
            .parameters
            .preview_args(agent_id, &selections, CliParameterLaunchScope::Chat)
            .map_err(cli_profile_error)?;
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
            selections,
            managed_args,
            env: BTreeMap::new(),
        })
    }

    /// For `codex-cli`/`gemini-cli`/`opencode`, the agent's assigned policy template overrides
    /// the security-relevant parameters it governs — see `apply_policy_template_overrides` and
    /// `add-cli-agent-permission-launch-flags` design.md. `claude-code` is excluded: its template
    /// is already enforced dynamically through `claude-code-permission-hook`'s per-call hook, so
    /// no template lookup happens for it here, and a lookup failure for it is impossible by
    /// construction rather than silently ignored.
    fn load_interactive(
        &self,
        agent_id: &str,
    ) -> Result<CliProfileSnapshot, AgentRuntimeApplicationError> {
        let (selections, managed_args, env) =
            interactive_selections_and_args(&self.parameters, &self.permissions, agent_id)?;

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
            selections,
            managed_args,
            env,
        })
    }
}

/// The interactive-scope selections/args computation, factored out of `load_interactive` so it's
/// testable against real `CliParametersApi`/`PermissionsApi` instances without also needing a
/// fully-wired `CliApi` (executable resolution has its own, unrelated dependency graph — pulling
/// it in here would make policy-template-override tests fragile for reasons that have nothing to
/// do with the logic being tested).
#[allow(clippy::type_complexity)]
fn interactive_selections_and_args(
    parameters: &CliParametersApi,
    permissions: &PermissionsApi,
    agent_id: &str,
) -> Result<(BTreeMap<String, Value>, Vec<String>, BTreeMap<String, String>), AgentRuntimeApplicationError>
{
    let selections = parameters
        .load_selections(agent_id)
        .map_err(cli_profile_error)?;
    let selections = parameters
        .normalize_selections(agent_id, &selections)
        .map_err(cli_profile_error)?;

    let template = if POLICY_TEMPLATE_GOVERNED_AGENT_IDS.contains(&agent_id) {
        let (principal, _has_explicit_assignment) = permissions
            .find_principal(agent_id)
            .map_err(cli_profile_error)?;
        Some(principal.template())
    } else {
        None
    };
    let selections = match template {
        Some(template) => apply_policy_template_overrides(agent_id, selections, template),
        None => selections,
    };

    let mut managed_args = parameters
        .preview_args(agent_id, &selections, CliParameterLaunchScope::Interactive)
        .map_err(cli_profile_error)?;
    let mut env = BTreeMap::new();
    if let Some(template) = template {
        managed_args = force_gemini_standard_approval_flag(agent_id, template, managed_args);
        if let Some((key, value)) = opencode_standard_permission_env_var(agent_id, template) {
            env.insert(key.to_string(), value.to_string());
        }
    }

    Ok((selections, managed_args, env))
}

fn cli_profile_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::CliProfile(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, OperationsError};
    use crate::contexts::permissions::application::{
        ApprovalBroker, ClaudeCodeHookPort, DefaultTemplatePort, EvaluationService,
        PendingApprovalEventPort, PermissionsApplicationError,
    };
    use crate::contexts::permissions::domain::{ApprovalRequest, PolicyTemplateName};
    use crate::contexts::permissions::infrastructure::{
        HookWaitRegistry, PermissionsSystemClock, PermissionsUuidIdGenerator,
        SqliteAuditRepository, SqliteGrantRepository, SqlitePrincipalRepository,
    };
    use crate::contexts::tooling::cli_parameters::SaveCliParameterProfileInput;
    use crate::platform::database::NativeDatabase;
    use crate::test_support::TempDirectory;
    use std::sync::Arc;

    struct NoopDiagnosticLog;
    impl DiagnosticLogPort for NoopDiagnosticLog {
        fn write_diagnostic(&self, _log: DiagnosticLog) -> Result<(), OperationsError> {
            Ok(())
        }
    }

    struct FixedDefaultTemplate(PolicyTemplateName);
    impl DefaultTemplatePort for FixedDefaultTemplate {
        fn default_template(&self) -> PolicyTemplateName {
            self.0
        }
    }

    struct NoopClaudeCodeHook;
    impl ClaudeCodeHookPort for NoopClaudeCodeHook {
        fn install(&self) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
        fn remove(&self) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
    }

    struct NoopEvents;
    impl PendingApprovalEventPort for NoopEvents {
        fn publish(&self, _request: &ApprovalRequest) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
    }

    /// Real, SQLite-backed `CliParametersApi` and `PermissionsApi` sharing one temp database —
    /// exercises the actual wiring `load_interactive` depends on, not fakes standing in for it.
    fn test_apis(
        temp_label: &str,
        default_template: PolicyTemplateName,
    ) -> (CliParametersApi, PermissionsApi) {
        let directory = TempDirectory::new(temp_label);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");

        let parameters = CliParametersApi::new(database.clone(), Arc::new(NoopDiagnosticLog));

        let principals = Arc::new(SqlitePrincipalRepository::new(database.clone()));
        let grants = Arc::new(SqliteGrantRepository::new(database.clone()));
        let audit = Arc::new(SqliteAuditRepository::new(database));
        let clock = Arc::new(PermissionsSystemClock);
        let ids = Arc::new(PermissionsUuidIdGenerator);
        let evaluation = EvaluationService::new(
            principals.clone(),
            grants.clone(),
            audit.clone(),
            clock.clone(),
            ids.clone(),
            Arc::new(FixedDefaultTemplate(default_template)),
        );
        let approvals = ApprovalBroker::new(
            principals,
            grants,
            audit,
            clock,
            ids,
            Arc::new(NoopEvents),
            300,
        );
        let permissions = PermissionsApi::new(
            evaluation,
            approvals,
            Arc::new(HookWaitRegistry::new()),
            Arc::new(NoopClaudeCodeHook),
        );
        (parameters, permissions)
    }

    #[test]
    fn readonly_template_overrides_a_conflicting_saved_codex_selection() {
        let (parameters, permissions) =
            test_apis("cli-profile-readonly", PolicyTemplateName::Standard);
        let seed: SaveCliParameterProfileInput = serde_json::from_value(serde_json::json!({
            "agentId": "codex-cli",
            "selections": {"sandbox": "workspace-write"},
        }))
        .expect("deserialize seed input");
        parameters
            .save_profile(&seed)
            .expect("seed a conflicting saved selection");
        permissions
            .assign_template("codex-cli", PolicyTemplateName::Readonly)
            .expect("assign readonly");

        let (selections, managed_args, _env) =
            interactive_selections_and_args(&parameters, &permissions, "codex-cli")
                .expect("interactive selections");

        assert_eq!(selections["sandbox"], "read-only");
        assert_eq!(selections["approvalPolicy"], "never");
        assert!(managed_args
            .windows(2)
            .any(|pair| pair == ["--sandbox", "read-only"]));
        assert!(!managed_args.iter().any(|arg| arg == "workspace-write"));
    }

    #[test]
    fn unassigned_agent_resolves_the_configured_default_template() {
        let (parameters, permissions) =
            test_apis("cli-profile-default", PolicyTemplateName::Trusted);

        let (selections, managed_args, env) =
            interactive_selections_and_args(&parameters, &permissions, "opencode")
                .expect("interactive selections");

        assert_eq!(selections["autoApprove"], true);
        assert!(managed_args.iter().any(|arg| arg == "--auto"));
        assert!(env.is_empty(), "trusted must not set OPENCODE_PERMISSION");
    }

    #[test]
    fn claude_code_is_never_looked_up() {
        let (parameters, permissions) =
            test_apis("cli-profile-claude-code", PolicyTemplateName::Standard);

        let (selections, _managed_args, env) =
            interactive_selections_and_args(&parameters, &permissions, "claude-code")
                .expect("interactive selections");

        assert!(!selections.contains_key("sandbox"));
        assert!(!selections.contains_key("approvalMode"));
        assert!(!selections.contains_key("autoApprove"));
        assert!(env.is_empty());
    }

    #[test]
    fn opencode_standard_injects_the_permission_env_var() {
        let (parameters, permissions) =
            test_apis("cli-profile-opencode-standard", PolicyTemplateName::Standard);

        let (_selections, _managed_args, env) =
            interactive_selections_and_args(&parameters, &permissions, "opencode")
                .expect("interactive selections");

        assert_eq!(
            env.get("OPENCODE_PERMISSION").map(String::as_str),
            Some(r#"{"edit":"ask","bash":"ask"}"#)
        );
    }

    #[test]
    fn opencode_readonly_does_not_inject_the_permission_env_var() {
        let (parameters, permissions) =
            test_apis("cli-profile-opencode-readonly", PolicyTemplateName::Readonly);

        let (_selections, _managed_args, env) =
            interactive_selections_and_args(&parameters, &permissions, "opencode")
                .expect("interactive selections");

        assert!(env.is_empty());
    }
}
