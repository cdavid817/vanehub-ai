use super::PluginIntegrationId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginIntegrationStatus {
    Configured,
    NotConfigured,
    MissingCli,
    Error,
}

impl PluginIntegrationStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::NotConfigured => "not-configured",
            Self::MissingCli => "missing-cli",
            Self::Error => "error",
        }
    }

    pub(crate) fn reason_key(self) -> &'static str {
        match self {
            Self::Configured => "plugins.statusReason.configured",
            Self::NotConfigured => "plugins.statusReason.notConfigured",
            Self::MissingCli => "plugins.statusReason.missingCli",
            Self::Error => "plugins.statusReason.error",
        }
    }

    pub(crate) fn configured(self) -> bool {
        self == Self::Configured
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginIntegrationState {
    pub(crate) integration_id: PluginIntegrationId,
    pub(crate) status: PluginIntegrationStatus,
    pub(crate) configured: bool,
    pub(crate) can_test: bool,
    pub(crate) last_checked_at: Option<String>,
    pub(crate) status_reason_key: Option<String>,
    pub(crate) message: Option<String>,
}

impl PluginIntegrationState {
    pub(crate) fn initial(integration_id: PluginIntegrationId) -> Self {
        Self {
            integration_id,
            status: PluginIntegrationStatus::NotConfigured,
            configured: false,
            can_test: true,
            last_checked_at: None,
            status_reason_key: Some("plugins.statusReason.notChecked".to_string()),
            message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginIntegrationEnvironment {
    pub(crate) runtime: &'static str,
    pub(crate) native_checks_available: bool,
    pub(crate) reason_key: Option<&'static str>,
}

pub(crate) fn native_environment() -> PluginIntegrationEnvironment {
    PluginIntegrationEnvironment {
        runtime: "tauri",
        native_checks_available: true,
        reason_key: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PluginIntegrationToolOutcome {
    Completed {
        success: bool,
        stdout: String,
        stderr: String,
    },
    MissingExecutable,
    TimedOut,
    LaunchFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PluginIntegrationTestResult {
    pub(crate) integration_id: PluginIntegrationId,
    pub(crate) status: PluginIntegrationStatus,
    pub(crate) configured: bool,
    pub(crate) message: String,
    pub(crate) checked_at: String,
}

pub(crate) fn evaluate_readiness(
    integration_id: PluginIntegrationId,
    outcome: &PluginIntegrationToolOutcome,
    checked_at: String,
) -> PluginIntegrationTestResult {
    let status = match outcome {
        PluginIntegrationToolOutcome::Completed { success: true, .. } => {
            PluginIntegrationStatus::Configured
        }
        PluginIntegrationToolOutcome::Completed { stdout, stderr, .. }
            if indicates_missing_authentication(stdout, stderr) =>
        {
            PluginIntegrationStatus::NotConfigured
        }
        PluginIntegrationToolOutcome::Completed { .. }
        | PluginIntegrationToolOutcome::TimedOut
        | PluginIntegrationToolOutcome::LaunchFailed => PluginIntegrationStatus::Error,
        PluginIntegrationToolOutcome::MissingExecutable => PluginIntegrationStatus::MissingCli,
    };

    PluginIntegrationTestResult {
        integration_id,
        status,
        configured: status.configured(),
        message: status.reason_key().to_string(),
        checked_at,
    }
}

fn indicates_missing_authentication(stdout: &str, stderr: &str) -> bool {
    let combined = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    combined.contains("not logged")
        || combined.contains("not authenticated")
        || combined.contains("authentication")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn readiness(outcome: PluginIntegrationToolOutcome) -> PluginIntegrationTestResult {
        evaluate_readiness(PluginIntegrationId::Github, &outcome, "now".to_string())
    }

    #[test]
    fn initial_and_native_environment_state_preserve_existing_contract() {
        let state = PluginIntegrationState::initial(PluginIntegrationId::Github);
        assert_eq!(state.status, PluginIntegrationStatus::NotConfigured);
        assert!(!state.configured);
        assert!(state.can_test);
        assert_eq!(state.last_checked_at, None);
        assert_eq!(
            state.status_reason_key.as_deref(),
            Some("plugins.statusReason.notChecked")
        );
        assert_eq!(state.message, None);

        let environment = native_environment();
        assert_eq!(environment.runtime, "tauri");
        assert!(environment.native_checks_available);
        assert_eq!(environment.reason_key, None);
    }

    #[test]
    fn completed_tool_results_distinguish_authenticated_unauthenticated_and_other_failures() {
        let configured = readiness(PluginIntegrationToolOutcome::Completed {
            success: true,
            stdout: "Logged in to github.com".to_string(),
            stderr: String::new(),
        });
        assert_eq!(configured.status, PluginIntegrationStatus::Configured);
        assert!(configured.configured);

        let not_configured = readiness(PluginIntegrationToolOutcome::Completed {
            success: false,
            stdout: String::new(),
            stderr: "You are not logged into any GitHub hosts".to_string(),
        });
        assert_eq!(
            not_configured.status,
            PluginIntegrationStatus::NotConfigured
        );
        assert!(!not_configured.configured);

        let failed = readiness(PluginIntegrationToolOutcome::Completed {
            success: false,
            stdout: String::new(),
            stderr: "internal failure".to_string(),
        });
        assert_eq!(failed.status, PluginIntegrationStatus::Error);
    }

    #[test]
    fn tool_failures_have_stable_safe_status_and_message_classification() {
        let cases = [
            (
                PluginIntegrationToolOutcome::MissingExecutable,
                PluginIntegrationStatus::MissingCli,
                "plugins.statusReason.missingCli",
            ),
            (
                PluginIntegrationToolOutcome::TimedOut,
                PluginIntegrationStatus::Error,
                "plugins.statusReason.error",
            ),
            (
                PluginIntegrationToolOutcome::LaunchFailed,
                PluginIntegrationStatus::Error,
                "plugins.statusReason.error",
            ),
        ];

        for (outcome, status, message) in cases {
            let result = readiness(outcome);
            assert_eq!(result.status, status);
            assert_eq!(result.message, message);
            assert!(!result.configured);
            assert_eq!(result.checked_at, "now");
        }
    }
}
