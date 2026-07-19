use super::candidates::{CliCandidateSource, SystemCliCandidateSource};
use super::process_adapter::{
    platform_process_runner, CliProcessOutput, CliProcessRequest, CliProcessRunner,
};
use super::support::{
    current_environment_type, install_command_for, install_source, npm_executable,
};
use crate::contexts::tooling::cli::application::{
    CliApplicationError, CliDetectionPort, CliDetectionResult, CliLogCategory, CliLogEvent,
    CliLogLevel, CliToolStatus,
};
use crate::contexts::tooling::cli::domain::{
    derive_conflict_state, derive_lifecycle_eligibility, is_stable_version, Installation,
    ToolDefinition, VersionCheckStatus,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

const VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(15);
const NPM_VIEW_TIMEOUT: Duration = Duration::from_secs(10);
const AVAILABLE_VERSION_LIMIT: usize = 20;

#[derive(Clone)]
pub(crate) struct CliDetectionAdapter {
    process: Arc<dyn CliProcessRunner>,
    candidates: Arc<dyn CliCandidateSource>,
}

impl CliDetectionAdapter {
    pub(crate) fn new() -> Self {
        let process = platform_process_runner();
        let candidates = Arc::new(SystemCliCandidateSource::new(process.clone()));
        Self {
            process,
            candidates,
        }
    }

    #[cfg(test)]
    fn with_dependencies(
        process: Arc<dyn CliProcessRunner>,
        candidates: Arc<dyn CliCandidateSource>,
    ) -> Self {
        Self {
            process,
            candidates,
        }
    }
}

impl CliDetectionPort for CliDetectionAdapter {
    fn detect(
        &self,
        definition: ToolDefinition,
        operation_id: &str,
    ) -> Result<CliDetectionResult, CliApplicationError> {
        let environment_type = current_environment_type();
        let candidates = self.candidates.candidates(definition);
        let mut installations = Vec::new();
        let mut warnings = Vec::new();
        let mut events = Vec::new();

        for (index, path) in candidates.iter().enumerate() {
            let path_string = path.to_string_lossy().to_string();
            let source = install_source(path);
            let probe = self.process.execute(CliProcessRequest {
                executable: path_string.clone(),
                args: vec!["--version".to_string()],
                timeout: VERSION_PROBE_TIMEOUT,
                audit_category: Some("cli.executable.version"),
            });
            let (version, error) = match probe {
                Ok(output) if output.success => (first_output_line(&output), None),
                Ok(output) => {
                    let reason = first_output_line(&output).unwrap_or_else(|| {
                        format!("{} --version failed.", definition.executable_name)
                    });
                    events.extend(detection_failure_events(
                        definition,
                        operation_id,
                        "executable-version",
                        &reason,
                        Some(&output),
                    ));
                    warnings.push(reason.clone());
                    (None, Some(reason))
                }
                Err(reason) => {
                    events.extend(detection_failure_events(
                        definition,
                        operation_id,
                        "executable-version",
                        &reason,
                        None,
                    ));
                    warnings.push(reason.clone());
                    (None, Some(reason))
                }
            };
            installations.push(Installation {
                path: path_string,
                version,
                runnable: error.is_none(),
                error,
                source,
                environment_type,
                is_active: index == 0,
            });
        }

        let latest_version = match self.npm_view(definition, &["version"]) {
            Ok(version) => Some(version),
            Err((reason, output)) => {
                events.extend(detection_failure_events(
                    definition,
                    operation_id,
                    "npm-view-version",
                    &reason,
                    output.as_ref(),
                ));
                warnings.push(reason);
                None
            }
        };
        let available_versions = match self.npm_view(definition, &["versions", "--json"]) {
            Ok(raw) => stable_versions_from_npm_json(&raw, AVAILABLE_VERSION_LIMIT),
            Err((reason, output)) => {
                events.extend(detection_failure_events(
                    definition,
                    operation_id,
                    "npm-view-versions",
                    &reason,
                    output.as_ref(),
                ));
                warnings.push(reason);
                Vec::new()
            }
        };

        let installed = !installations.is_empty();
        let active = installations
            .iter()
            .find(|installation| installation.is_active);
        let detected_path = active.map(|installation| installation.path.clone());
        let status = CliToolStatus {
            agent_id: definition.agent_id.to_string(),
            display_name: definition.display_name.to_string(),
            provider: definition.provider.to_string(),
            executable_name: definition.executable_name.to_string(),
            package_name: definition.package_name.to_string(),
            installed: Some(installed),
            current_version: active.and_then(|installation| installation.version.clone()),
            latest_version,
            available_versions,
            detected_path: detected_path.clone(),
            install_command: install_command_for(definition),
            last_checked_at: None,
            last_error: None,
            last_operation_id: None,
            version_check_status: if installed {
                VersionCheckStatus::Succeeded
            } else {
                VersionCheckStatus::NotDetected
            },
            environment_type,
            conflict_state: derive_conflict_state(&installations),
            lifecycle_eligibility: derive_lifecycle_eligibility(definition, installed, active),
            installations,
            active_installation_path: detected_path,
        };
        Ok(CliDetectionResult {
            status,
            warnings,
            events,
        })
    }
}

impl CliDetectionAdapter {
    fn npm_view(
        &self,
        definition: ToolDefinition,
        view_args: &[&str],
    ) -> Result<String, (String, Option<CliProcessOutput>)> {
        let mut args = vec!["view".to_string(), definition.package_name.to_string()];
        args.extend(view_args.iter().map(|arg| (*arg).to_string()));
        match self.process.execute(CliProcessRequest {
            executable: npm_executable().to_string(),
            args,
            timeout: NPM_VIEW_TIMEOUT,
            audit_category: Some("cli.npm.view"),
        }) {
            Ok(output) if output.success => Ok(output.stdout.trim().to_string()),
            Ok(output) => Err((
                first_output_line(&output).unwrap_or_else(|| "npm view failed".to_string()),
                Some(output),
            )),
            Err(error) => Err((error, None)),
        }
    }
}

fn first_output_line(output: &CliProcessOutput) -> Option<String> {
    output
        .stdout
        .lines()
        .chain(output.stderr.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn stable_versions_from_npm_json(raw: &str, limit: usize) -> Vec<String> {
    let Ok(versions) = serde_json::from_str::<Vec<String>>(raw) else {
        return Vec::new();
    };
    versions
        .into_iter()
        .filter(|version| is_stable_version(version))
        .rev()
        .take(limit)
        .collect()
}

fn detection_failure_events(
    definition: ToolDefinition,
    operation_id: &str,
    attempted_operation: &str,
    reason: &str,
    output: Option<&CliProcessOutput>,
) -> [CliLogEvent; 2] {
    let operation = CliLogEvent {
        operation_id: operation_id.to_string(),
        agent_id: Some(definition.agent_id.to_string()),
        level: CliLogLevel::Warn,
        category: CliLogCategory::Operation,
        message: format!(
            "{} {} failed: {reason}",
            definition.display_name, attempted_operation
        ),
        context: BTreeMap::new(),
    };
    let mut context = definition_context(definition, operation_id);
    context.insert(
        "attemptedOperation".to_string(),
        attempted_operation.to_string(),
    );
    context.insert("reason".to_string(), reason.to_string());
    if let Some(output) = output {
        context.insert("stdout".to_string(), output.stdout.clone());
        context.insert("stderr".to_string(), output.stderr.clone());
        context.insert("exitStatus".to_string(), output.status.clone());
    }
    if reason.to_ascii_lowercase().contains("timed out") {
        context.insert("timeoutReason".to_string(), reason.to_string());
    }
    let diagnostic = CliLogEvent {
        operation_id: operation_id.to_string(),
        agent_id: Some(definition.agent_id.to_string()),
        level: CliLogLevel::Warn,
        category: CliLogCategory::Diagnostic,
        message: "CLI detection diagnostic failure.".to_string(),
        context,
    };
    [operation, diagnostic]
}

fn definition_context(definition: ToolDefinition, operation_id: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("operationId".to_string(), operation_id.to_string()),
        ("agentId".to_string(), definition.agent_id.to_string()),
        (
            "displayName".to_string(),
            definition.display_name.to_string(),
        ),
        ("provider".to_string(), definition.provider.to_string()),
        (
            "executableName".to_string(),
            definition.executable_name.to_string(),
        ),
        (
            "packageName".to_string(),
            definition.package_name.to_string(),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli::domain::{definition, InstallSource};
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use std::sync::Mutex;

    struct FakeCandidates(Vec<PathBuf>);

    impl CliCandidateSource for FakeCandidates {
        fn candidates(&self, _definition: ToolDefinition) -> Vec<PathBuf> {
            self.0.clone()
        }
    }

    struct FakeProcess {
        requests: Mutex<Vec<CliProcessRequest>>,
        outputs: Mutex<VecDeque<Result<CliProcessOutput, String>>>,
    }

    impl FakeProcess {
        fn new(outputs: Vec<Result<CliProcessOutput, String>>) -> Self {
            Self {
                requests: Mutex::new(Vec::new()),
                outputs: Mutex::new(outputs.into()),
            }
        }
    }

    impl CliProcessRunner for FakeProcess {
        fn execute(&self, request: CliProcessRequest) -> Result<CliProcessOutput, String> {
            self.requests.lock().expect("requests").push(request);
            self.outputs
                .lock()
                .expect("outputs")
                .pop_front()
                .expect("fixture output")
        }
    }

    fn output(success: bool, stdout: &str, stderr: &str) -> CliProcessOutput {
        CliProcessOutput {
            success,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            status: if success { "0" } else { "1" }.to_string(),
        }
    }

    #[test]
    fn detection_uses_explicit_arguments_and_bounded_timeouts() {
        let process = Arc::new(FakeProcess::new(vec![
            Ok(output(true, "codex-cli 1.2.3", "")),
            Ok(output(true, "2.0.0", "")),
            Ok(output(true, r#"["1.0.0","1.5.0-beta.1","2.0.0"]"#, "")),
        ]));
        let candidates = Arc::new(FakeCandidates(vec![PathBuf::from(
            "/fixture/.npm/bin/codex",
        )]));
        let adapter = CliDetectionAdapter::with_dependencies(process.clone(), candidates);

        let result = adapter
            .detect(definition("codex-cli").expect("definition"), "op-1")
            .expect("detect");

        assert_eq!(
            result.status.current_version.as_deref(),
            Some("codex-cli 1.2.3")
        );
        assert_eq!(result.status.latest_version.as_deref(), Some("2.0.0"));
        assert_eq!(result.status.available_versions, vec!["2.0.0", "1.0.0"]);
        assert_eq!(result.status.installations[0].source, InstallSource::Npm);
        assert!(result.events.is_empty());
        let requests = process.requests.lock().expect("requests");
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].args, ["--version"]);
        assert_eq!(requests[0].timeout, VERSION_PROBE_TIMEOUT);
        assert_eq!(requests[1].args, ["view", "@openai/codex", "version"]);
        assert_eq!(requests[1].timeout, NPM_VIEW_TIMEOUT);
        assert_eq!(
            requests[2].args,
            ["view", "@openai/codex", "versions", "--json"]
        );
        assert_eq!(requests[2].timeout, NPM_VIEW_TIMEOUT);
    }

    #[test]
    fn detection_failure_emits_page_and_diagnostic_events_without_status_error() {
        let process = Arc::new(FakeProcess::new(vec![
            Err("command timed out".to_string()),
            Ok(output(false, "", "registry unavailable")),
            Ok(output(true, "[]", "")),
        ]));
        let candidates = Arc::new(FakeCandidates(vec![PathBuf::from(
            "/fixture/.npm/bin/codex",
        )]));
        let adapter = CliDetectionAdapter::with_dependencies(process, candidates);

        let result = adapter
            .detect(definition("codex-cli").expect("definition"), "op-2")
            .expect("detect");

        assert_eq!(
            result.warnings,
            ["command timed out", "registry unavailable"]
        );
        assert!(result.status.last_error.is_none());
        assert_eq!(result.events.len(), 4);
        assert_eq!(result.events[0].category, CliLogCategory::Operation);
        assert_eq!(result.events[1].category, CliLogCategory::Diagnostic);
        assert_eq!(
            result.events[1]
                .context
                .get("timeoutReason")
                .map(String::as_str),
            Some("command timed out")
        );
    }
}
