use super::providers::{
    apply_configuration_overrides, apply_policy_template_overrides,
    force_gemini_standard_approval_flag, opencode_standard_permission_env_var,
    POLICY_TEMPLATE_GOVERNED_AGENT_IDS,
};
use crate::contexts::agent_runtime::application::{
    resolve_effective_execution_policy, AgentChatConfiguration, AgentCliProfileGateway,
    AgentRuntimeApplicationError, CliProfileSnapshot, SessionExecutionMode,
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
    pub(crate) fn new(
        parameters: CliParametersApi,
        cli: CliApi,
        permissions: PermissionsApi,
    ) -> Self {
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
        let mode = SessionExecutionMode::parse(&configuration.execution_mode).ok_or_else(|| {
            AgentRuntimeApplicationError::CliProfile(format!(
                "Unsupported session execution mode: {}.",
                configuration.execution_mode
            ))
        })?;
        let (selections, managed_args, env) = project_policy_and_build_args(
            &self.parameters,
            &self.permissions,
            agent_id,
            selections,
            CliParameterLaunchScope::Chat,
            mode,
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
            selections,
            managed_args,
            env,
        })
    }

    /// Interactive terminals inherit the Agent policy directly. Session execution intent applies
    /// only to chat generations, so both launch scopes share the same final policy projection.
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
) -> Result<
    (
        BTreeMap<String, Value>,
        Vec<String>,
        BTreeMap<String, String>,
    ),
    AgentRuntimeApplicationError,
> {
    let selections = parameters
        .load_selections(agent_id)
        .map_err(cli_profile_error)?;
    let selections = parameters
        .normalize_selections(agent_id, &selections)
        .map_err(cli_profile_error)?;

    project_policy_and_build_args(
        parameters,
        permissions,
        agent_id,
        selections,
        CliParameterLaunchScope::Interactive,
        SessionExecutionMode::Inherit,
    )
}

#[allow(clippy::type_complexity)]
fn project_policy_and_build_args(
    parameters: &CliParametersApi,
    permissions: &PermissionsApi,
    agent_id: &str,
    selections: BTreeMap<String, Value>,
    scope: CliParameterLaunchScope,
    mode: SessionExecutionMode,
) -> Result<
    (
        BTreeMap<String, Value>,
        Vec<String>,
        BTreeMap<String, String>,
    ),
    AgentRuntimeApplicationError,
> {
    if !POLICY_TEMPLATE_GOVERNED_AGENT_IDS.contains(&agent_id) {
        return Err(AgentRuntimeApplicationError::CliProfile(format!(
            "No execution-policy mapping exists for {agent_id}."
        )));
    }
    let (principal, _has_explicit_assignment) = permissions
        .find_principal(agent_id)
        .map_err(cli_profile_error)?;
    let effective = resolve_effective_execution_policy(principal.template(), mode);
    let launch_template = effective.launch_template();
    let selections = apply_policy_template_overrides(agent_id, selections, launch_template);
    let mut managed_args = parameters
        .preview_args(agent_id, &selections, scope)
        .map_err(cli_profile_error)?;
    managed_args = force_gemini_standard_approval_flag(agent_id, launch_template, managed_args);
    let mut env = BTreeMap::new();
    if let Some((key, value)) = opencode_standard_permission_env_var(agent_id, launch_template) {
        env.insert(key.to_string(), value.to_string());
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
    /// Returns the shared `NativeDatabase` too, so a test can reach in and break it deliberately
    /// (see `template_lookup_failure_fails_the_launch_instead_of_guessing_a_default`).
    fn test_apis(
        temp_label: &str,
        default_template: PolicyTemplateName,
    ) -> (CliParametersApi, PermissionsApi, NativeDatabase) {
        let directory = TempDirectory::new(temp_label);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");

        let parameters = CliParametersApi::new(database.clone(), Arc::new(NoopDiagnosticLog));

        let principals = Arc::new(SqlitePrincipalRepository::new(database.clone()));
        let grants = Arc::new(SqliteGrantRepository::new(database.clone()));
        let audit = Arc::new(SqliteAuditRepository::new(database.clone()));
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
        (parameters, permissions, database)
    }

    fn assert_policy_projection(
        agent_id: &str,
        selections: &BTreeMap<String, Value>,
        template: PolicyTemplateName,
    ) {
        match agent_id {
            "claude-code" => assert_eq!(
                selections["permissionMode"],
                match template {
                    PolicyTemplateName::Readonly => "plan",
                    PolicyTemplateName::Standard => "default",
                    PolicyTemplateName::Trusted | PolicyTemplateName::Yolo => "acceptEdits",
                }
            ),
            "codex-cli" => {
                let expected = match template {
                    PolicyTemplateName::Readonly => ("read-only", "never"),
                    PolicyTemplateName::Standard => ("workspace-write", "on-request"),
                    PolicyTemplateName::Trusted | PolicyTemplateName::Yolo => {
                        ("workspace-write", "never")
                    }
                };
                assert_eq!(selections["sandbox"], expected.0);
                assert_eq!(selections["approvalPolicy"], expected.1);
            }
            "gemini-cli" => assert_eq!(
                selections["approvalMode"],
                match template {
                    PolicyTemplateName::Readonly => "plan",
                    PolicyTemplateName::Standard => "default",
                    PolicyTemplateName::Trusted | PolicyTemplateName::Yolo => "yolo",
                }
            ),
            "opencode" => match template {
                PolicyTemplateName::Readonly => assert_eq!(selections["agent"], "plan"),
                PolicyTemplateName::Standard => {
                    assert!(matches!(
                        selections.get("agent").and_then(Value::as_str),
                        None | Some("default")
                    ));
                    assert!(matches!(
                        selections.get("autoApprove").and_then(Value::as_bool),
                        None | Some(false)
                    ));
                }
                PolicyTemplateName::Trusted | PolicyTemplateName::Yolo => {
                    assert_eq!(selections["autoApprove"], true)
                }
            },
            "antigravity-cli" => {
                let expected = match template {
                    PolicyTemplateName::Readonly => ("plan", true),
                    PolicyTemplateName::Standard => ("default", false),
                    PolicyTemplateName::Trusted | PolicyTemplateName::Yolo => {
                        ("accept-edits", false)
                    }
                };
                assert_eq!(selections["mode"], expected.0);
                assert_eq!(selections["sandbox"], expected.1);
            }
            other => panic!("unexpected managed CLI: {other}"),
        }
    }

    #[test]
    fn every_managed_cli_projects_every_policy_for_chat_and_terminal() {
        let (parameters, permissions, _database) =
            test_apis("cli-profile-matrix", PolicyTemplateName::Standard);
        let templates = [
            PolicyTemplateName::Readonly,
            PolicyTemplateName::Standard,
            PolicyTemplateName::Trusted,
            PolicyTemplateName::Yolo,
        ];
        let modes = [
            SessionExecutionMode::Inherit,
            SessionExecutionMode::Plan,
            SessionExecutionMode::Execute,
        ];

        for template in templates {
            for agent_id in POLICY_TEMPLATE_GOVERNED_AGENT_IDS {
                permissions
                    .assign_template(agent_id, template)
                    .expect("assign template");
                for mode in modes {
                    let (selections, _, _) = project_policy_and_build_args(
                        &parameters,
                        &permissions,
                        agent_id,
                        BTreeMap::new(),
                        CliParameterLaunchScope::Chat,
                        mode,
                    )
                    .expect("chat policy projection");
                    let effective = resolve_effective_execution_policy(template, mode);
                    assert_policy_projection(agent_id, &selections, effective.launch_template());
                }

                let (selections, _, _) =
                    interactive_selections_and_args(&parameters, &permissions, agent_id)
                        .expect("terminal policy projection");
                assert_policy_projection(agent_id, &selections, template);
            }
        }
    }

    #[test]
    fn readonly_template_projects_codex_security_arguments() {
        let (parameters, permissions, _database) =
            test_apis("cli-profile-readonly", PolicyTemplateName::Standard);
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
    fn editable_profiles_reject_policy_governed_parameters() {
        let (parameters, _permissions, _database) = test_apis(
            "cli-profile-governed-rejected",
            PolicyTemplateName::Standard,
        );
        let input: SaveCliParameterProfileInput = serde_json::from_value(serde_json::json!({
            "agentId": "codex-cli",
            "selections": {"sandbox": "read-only"},
        }))
        .expect("deserialize input");

        let error = parameters
            .save_profile(&input)
            .expect_err("governed parameter must be rejected");

        assert!(error
            .to_string()
            .contains("unknown CLI parameter 'sandbox'"));
    }

    #[test]
    fn unassigned_agent_resolves_the_configured_default_template() {
        let (parameters, permissions, _database) =
            test_apis("cli-profile-default", PolicyTemplateName::Trusted);

        let (selections, managed_args, env) =
            interactive_selections_and_args(&parameters, &permissions, "opencode")
                .expect("interactive selections");

        assert_eq!(selections["autoApprove"], true);
        assert!(managed_args.iter().any(|arg| arg == "--auto"));
        assert!(env.is_empty(), "trusted must not set OPENCODE_PERMISSION");
    }

    #[test]
    fn standard_template_projects_claude_code_approval_arguments() {
        let (parameters, permissions, _database) =
            test_apis("cli-profile-claude-code", PolicyTemplateName::Standard);

        let (selections, managed_args, env) =
            interactive_selections_and_args(&parameters, &permissions, "claude-code")
                .expect("interactive selections");

        assert_eq!(selections["permissionMode"], "default");
        assert!(!managed_args.iter().any(|arg| arg == "--permission-mode"));
        assert!(env.is_empty());
    }

    #[test]
    fn opencode_standard_injects_the_permission_env_var() {
        let (parameters, permissions, _database) = test_apis(
            "cli-profile-opencode-standard",
            PolicyTemplateName::Standard,
        );

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
        let (parameters, permissions, _database) = test_apis(
            "cli-profile-opencode-readonly",
            PolicyTemplateName::Readonly,
        );

        let (_selections, _managed_args, env) =
            interactive_selections_and_args(&parameters, &permissions, "opencode")
                .expect("interactive selections");

        assert!(env.is_empty());
    }

    /// A genuine SQLite-level failure — dropping the table `find_principal` reads from, through
    /// the same connection pool it will use — not a filesystem trick (fragile/platform-dependent)
    /// and not a fake standing in for the failure.
    #[test]
    fn template_lookup_failure_fails_the_launch_instead_of_guessing_a_default() {
        let (parameters, permissions, database) =
            test_apis("cli-profile-lookup-failure", PolicyTemplateName::Standard);
        database
            .connection()
            .expect("connection")
            .execute("DROP TABLE agent_principals", [])
            .expect("drop table");

        let result = interactive_selections_and_args(&parameters, &permissions, "codex-cli");

        assert!(
            matches!(result, Err(AgentRuntimeApplicationError::CliProfile(_))),
            "expected a CliProfile error, got {result:?}"
        );
    }
}
