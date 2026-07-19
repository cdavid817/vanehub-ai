use crate::contexts::tooling::plugin_integrations::application::PluginIntegrationToolPort;
use crate::contexts::tooling::plugin_integrations::domain::{
    PluginIntegrationId, PluginIntegrationToolOutcome, PluginIntegrationToolPlan,
};
use crate::platform::process::{ProcessAdapter, ProcessError, ProcessOutput, ProcessRequest};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct GitHubCliToolAdapter;

impl PluginIntegrationToolPort for GitHubCliToolAdapter {
    fn execute(&self, plan: PluginIntegrationToolPlan) -> PluginIntegrationToolOutcome {
        crate::platform::process::audit_command(
            audit_category(plan.integration_id),
            plan.executable,
            &plan
                .arguments
                .iter()
                .map(|argument| (*argument).to_string())
                .collect::<Vec<_>>(),
        );
        let request = process_request(plan);
        match ProcessAdapter.execute(&request) {
            Ok(output) => completed_outcome(output),
            Err(error) => process_failure_outcome(error),
        }
    }
}

fn process_request(plan: PluginIntegrationToolPlan) -> ProcessRequest {
    ProcessRequest::new(plan.executable)
        .args(plan.arguments.iter().copied())
        .timeout(Duration::from_secs(plan.timeout_seconds))
}

fn completed_outcome(output: ProcessOutput) -> PluginIntegrationToolOutcome {
    PluginIntegrationToolOutcome::Completed {
        success: output.success(),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

fn process_failure_outcome(error: ProcessError) -> PluginIntegrationToolOutcome {
    match error {
        ProcessError::TimedOut { .. } => PluginIntegrationToolOutcome::TimedOut,
        ProcessError::Spawn(message) if indicates_missing_executable(&message) => {
            PluginIntegrationToolOutcome::MissingExecutable
        }
        ProcessError::InvalidExecutable(_) | ProcessError::Spawn(_) | ProcessError::Wait(_) => {
            PluginIntegrationToolOutcome::LaunchFailed
        }
    }
}

fn indicates_missing_executable(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("failed to resolve executable")
        || lower.contains("not found")
        || lower.contains("cannot find the file")
        || lower.contains("no such file")
        || lower.contains("os error 2")
}

fn audit_category(integration_id: PluginIntegrationId) -> &'static str {
    match integration_id {
        PluginIntegrationId::Github => "plugin-integration.github",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::plugin_integrations::domain::readiness_plan;
    use std::ffi::OsStr;

    #[test]
    fn github_plan_builds_an_explicit_bounded_command_without_a_shell() {
        let plan = readiness_plan(PluginIntegrationId::Github);
        let command = process_request(plan).command().expect("command");

        assert_eq!(command.get_program(), OsStr::new("gh"));
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![OsStr::new("auth"), OsStr::new("status")]
        );
        assert_eq!(
            audit_category(plan.integration_id),
            "plugin-integration.github"
        );
    }

    #[test]
    fn process_failures_are_reduced_to_safe_domain_classifications() {
        assert_eq!(
            process_failure_outcome(ProcessError::Spawn(
                "The system cannot find the file specified. (os error 2)".to_string()
            )),
            PluginIntegrationToolOutcome::MissingExecutable
        );
        assert_eq!(
            process_failure_outcome(ProcessError::Spawn(
                "permission denied at C:\\private\\gh.exe".to_string()
            )),
            PluginIntegrationToolOutcome::LaunchFailed
        );
        assert_eq!(
            process_failure_outcome(ProcessError::TimedOut {
                timeout_seconds: 10,
                stdout: "token=secret".to_string(),
                stderr: "private path".to_string(),
            }),
            PluginIntegrationToolOutcome::TimedOut
        );
    }
}
