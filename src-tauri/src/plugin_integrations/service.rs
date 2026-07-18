use crate::command_safety;
use crate::{logging, AppError};
use chrono::Utc;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::models::*;

const GITHUB_DOCS_URL: &str = "https://cli.github.com/manual/gh_auth_login";
const GITHUB_AUTH_STATUS_TIMEOUT: Duration = Duration::from_secs(10);

pub fn definitions() -> Vec<PluginIntegrationDefinition> {
    vec![PluginIntegrationDefinition {
        id: PluginIntegrationId::Github,
        name_key: "plugins.github.name".to_string(),
        description_key: "plugins.github.description".to_string(),
        version: "1.0.0".to_string(),
        provider: "GitHub".to_string(),
        icon: "github".to_string(),
        docs_url: GITHUB_DOCS_URL.to_string(),
        setup_steps: vec![
            PluginIntegrationSetupStep {
                id: "install".to_string(),
                label_key: "plugins.github.setup.install".to_string(),
            },
            PluginIntegrationSetupStep {
                id: "auth".to_string(),
                label_key: "plugins.github.setup.auth".to_string(),
            },
        ],
    }]
}

pub fn overview() -> PluginIntegrationOverview {
    PluginIntegrationOverview {
        definitions: definitions(),
        states: vec![default_state()],
        environment: PluginIntegrationEnvironment {
            runtime: "tauri".to_string(),
            native_checks_available: true,
            reason_key: None,
        },
    }
}

pub fn default_state() -> PluginIntegrationState {
    PluginIntegrationState {
        integration_id: PluginIntegrationId::Github,
        status: PluginIntegrationStatus::NotConfigured,
        configured: false,
        can_test: true,
        last_checked_at: None,
        status_reason_key: Some("plugins.statusReason.notChecked".to_string()),
        message: None,
    }
}

pub fn test_readiness(id: PluginIntegrationId, log_dir: PathBuf) -> Result<PluginIntegrationTestResult, AppError> {
    match id {
        PluginIntegrationId::Github => test_github_readiness(log_dir),
    }
}

fn test_github_readiness(log_dir: PathBuf) -> Result<PluginIntegrationTestResult, AppError> {
    let checked_at = Utc::now().to_rfc3339();
    let output = run_gh_auth_status();
    let result = match output {
        Ok(output) => github_result_from_output(output, checked_at),
        Err(error) => {
            let message = error.to_string();
            let status = launch_error_status(&message);
            PluginIntegrationTestResult {
                integration_id: PluginIntegrationId::Github,
                status,
                configured: false,
                message: status_message_key(status).to_string(),
                checked_at,
            }
        }
    };

    write_readiness_log(&log_dir, &result);
    Ok(result)
}

fn run_gh_auth_status() -> Result<Output, AppError> {
    let args = vec!["auth".to_string(), "status".to_string()];
    command_safety::audit_command("plugin-integration.github", "gh", &args);
    let mut command = command_safety::std_command("gh")?;
    command.args(["auth", "status"]);
    output_with_timeout(&mut command, GITHUB_AUTH_STATUS_TIMEOUT)
        .map_err(AppError::LaunchFailed)
}

fn output_with_timeout(command: &mut Command, timeout: Duration) -> Result<Output, String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => return child.wait_with_output().map_err(|error| error.to_string()),
            Ok(None) if start.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("command timed out".to_string());
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn launch_error_status(message: &str) -> PluginIntegrationStatus {
    let lower = message.to_ascii_lowercase();
    if lower.contains("failed to resolve executable")
        || lower.contains("not found")
        || lower.contains("cannot find the file")
        || lower.contains("no such file")
        || lower.contains("os error 2")
    {
        PluginIntegrationStatus::MissingCli
    } else {
        PluginIntegrationStatus::Error
    }
}

fn github_result_from_output(output: Output, checked_at: String) -> PluginIntegrationTestResult {
    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    let stdout = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    let combined = format!("{stdout}\n{stderr}");
    let status = if output.status.success() {
        PluginIntegrationStatus::Configured
    } else if combined.contains("not logged") || combined.contains("not authenticated") || combined.contains("authentication") {
        PluginIntegrationStatus::NotConfigured
    } else {
        PluginIntegrationStatus::Error
    };

    PluginIntegrationTestResult {
        integration_id: PluginIntegrationId::Github,
        status,
        configured: status == PluginIntegrationStatus::Configured,
        message: status_message_key(status).to_string(),
        checked_at,
    }
}

fn status_message_key(status: PluginIntegrationStatus) -> &'static str {
    match status {
        PluginIntegrationStatus::Configured => "plugins.statusReason.configured",
        PluginIntegrationStatus::NotConfigured => "plugins.statusReason.notConfigured",
        PluginIntegrationStatus::MissingCli => "plugins.statusReason.missingCli",
        PluginIntegrationStatus::Unavailable => "plugins.statusReason.unavailable",
        PluginIntegrationStatus::Error => "plugins.statusReason.error",
    }
}

fn write_readiness_log(log_dir: &std::path::Path, result: &PluginIntegrationTestResult) {
    let mut context = BTreeMap::new();
    context.insert("integrationId".to_string(), result.integration_id.as_str().to_string());
    context.insert("operation".to_string(), "readiness-check".to_string());
    context.insert("safeStatus".to_string(), result.status.as_str().to_string());
    let level = if result.configured {
        logging::LogLevel::Info
    } else {
        logging::LogLevel::Warn
    };
    let _ = logging::write_message(
        log_dir,
        level,
        "plugin-integration.github",
        &result.message,
        context,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::ExitStatus;

    #[cfg(unix)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    #[cfg(windows)]
    fn exit_status(code: u32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code)
    }

    #[test]
    fn catalog_contains_only_builtin_github() {
        let definitions = definitions();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].id.as_str(), "github");
        assert_eq!(definitions[0].docs_url, GITHUB_DOCS_URL);
    }

    #[test]
    fn successful_gh_status_marks_configured() {
        let result = github_result_from_output(
            Output {
                status: exit_status(0),
                stdout: b"Logged in to github.com".to_vec(),
                stderr: Vec::new(),
            },
            "now".to_string(),
        );

        assert_eq!(result.status, PluginIntegrationStatus::Configured);
        assert!(result.configured);
    }

    #[test]
    fn unauthenticated_gh_status_marks_not_configured() {
        let result = github_result_from_output(
            Output {
                status: exit_status(1),
                stdout: Vec::new(),
                stderr: b"You are not logged into any GitHub hosts".to_vec(),
            },
            "now".to_string(),
        );

        assert_eq!(result.status, PluginIntegrationStatus::NotConfigured);
        assert!(!result.configured);
    }

    #[test]
    fn launch_errors_classify_missing_cli() {
        assert_eq!(
            launch_error_status("The system cannot find the file specified. (os error 2)"),
            PluginIntegrationStatus::MissingCli
        );
        assert_eq!(
            launch_error_status("permission denied"),
            PluginIntegrationStatus::Error
        );
        assert_eq!(
            launch_error_status("command timed out"),
            PluginIntegrationStatus::Error
        );
    }

    #[test]
    fn readiness_log_uses_unified_redaction() {
        let dir = std::env::temp_dir().join(format!(
            "vanehub-plugin-integration-log-test-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let result = PluginIntegrationTestResult {
            integration_id: PluginIntegrationId::Github,
            status: PluginIntegrationStatus::Error,
            configured: false,
            message: "token=ghp_secret".to_string(),
            checked_at: "now".to_string(),
        };

        write_readiness_log(&dir, &result);

        let raw = std::fs::read_to_string(dir.join(logging::LOG_FILE_NAME)).expect("log");
        assert!(raw.contains("plugin-integration.github"));
        assert!(raw.contains("[REDACTED]"));
        assert!(!raw.contains("ghp_secret"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
