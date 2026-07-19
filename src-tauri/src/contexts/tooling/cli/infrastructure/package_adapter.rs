use super::candidates::{CliCandidateSource, SystemCliCandidateSource};
use super::process_adapter::{
    platform_process_runner, CliProcessOutput, CliProcessRequest, CliProcessRunner,
};
use super::support::{install_source, npm_executable};
use crate::contexts::tooling::cli::application::{
    CliApplicationError, CliLogCategory, CliLogEvent, CliLogLevel, CliPackagePort, CliToolStatus,
};
use crate::contexts::tooling::cli::domain::{
    winget_package_id, InstallSource, LifecycleEligibility, ToolDefinition,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

const PACKAGE_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone)]
pub(crate) struct CliPackageAdapter {
    process: Arc<dyn CliProcessRunner>,
    candidates: Arc<dyn CliCandidateSource>,
}

impl CliPackageAdapter {
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

impl CliPackagePort for CliPackageAdapter {
    fn validate(
        &self,
        definition: ToolDefinition,
        status: &CliToolStatus,
        confirmed_active_path: Option<&str>,
    ) -> Result<(), CliApplicationError> {
        let fresh_candidates = self.candidates.candidates(definition);
        match status.lifecycle_eligibility {
            LifecycleEligibility::Npm => {
                if let Some(active_path) = fresh_candidates.first() {
                    if install_source(active_path) != InstallSource::Npm {
                        return validation_error(
                            "the active CLI path changed and is not npm-managed",
                        );
                    }
                }
            }
            LifecycleEligibility::Wget => {
                if definition.script_install_url.is_none() {
                    return validation_error("the CLI does not have a verified wget installer");
                }
                if status.installed == Some(true) {
                    let active_path = fresh_candidates.first().ok_or_else(|| {
                        CliApplicationError::Validation(
                            "the active CLI installation could not be resolved".to_string(),
                        )
                    })?;
                    if install_source(active_path) != InstallSource::Vendor {
                        return validation_error(
                            "the active CLI path changed and is not a recognized script installation",
                        );
                    }
                }
            }
            LifecycleEligibility::Winget => {
                let active_path = fresh_candidates.first().ok_or_else(|| {
                    CliApplicationError::Validation(
                        "the active CLI installation could not be resolved".to_string(),
                    )
                })?;
                if install_source(active_path) != InstallSource::Winget {
                    return validation_error(
                        "the active CLI path changed and is not WinGet-managed",
                    );
                }
                if definition.winget_package_id.is_none()
                    && winget_package_id(&active_path.to_string_lossy()).is_none()
                {
                    return validation_error("the active WinGet package id could not be resolved");
                }
            }
            LifecycleEligibility::Manual => {
                return validation_error(
                    "the active CLI installation must be updated by its source-native installer",
                );
            }
            LifecycleEligibility::Unavailable => {
                return validation_error("the CLI lifecycle method is unavailable");
            }
        }
        if fresh_candidates.len() > 1 {
            let active_path = fresh_candidates
                .first()
                .map(|path| path.to_string_lossy().to_string())
                .ok_or_else(|| {
                    CliApplicationError::Validation(
                        "the active CLI installation could not be resolved".to_string(),
                    )
                })?;
            if confirmed_active_path != Some(active_path.as_str()) {
                return validation_error(
                    "multiple CLI installations require confirmation of the active path",
                );
            }
        }
        Ok(())
    }

    fn execute(
        &self,
        operation_id: &str,
        definition: ToolDefinition,
        status: &CliToolStatus,
        target_version: &str,
        emit: &mut dyn FnMut(CliLogEvent),
    ) -> Result<(), CliApplicationError> {
        let plan = lifecycle_plan(definition, status, target_version)?;
        let first_result = self.run_plan(operation_id, definition, target_version, &plan, emit);
        if first_result.is_err() && plan.fallback_npm_on_failure {
            emit(operation_event(
                operation_id,
                definition.agent_id,
                CliLogLevel::Warn,
                "wget installer failed; falling back to npm for first install.",
            ));
            let fallback = LifecyclePlan {
                method: LifecycleMethod::Npm,
                executable: npm_executable().to_string(),
                args: npm_install_args(definition, target_version),
                fallback_npm_on_failure: false,
            };
            self.run_plan(operation_id, definition, target_version, &fallback, emit)
        } else {
            first_result
        }
    }
}

impl CliPackageAdapter {
    fn run_plan(
        &self,
        operation_id: &str,
        definition: ToolDefinition,
        target_version: &str,
        plan: &LifecyclePlan,
        emit: &mut dyn FnMut(CliLogEvent),
    ) -> Result<(), CliApplicationError> {
        emit(operation_event(
            operation_id,
            definition.agent_id,
            CliLogLevel::Info,
            format!(
                "Running {} lifecycle operation for {} version {}.",
                plan.method.as_str(),
                definition.display_name,
                target_version
            ),
        ));
        emit(operation_event(
            operation_id,
            definition.agent_id,
            CliLogLevel::Info,
            format!(
                "{} executable: {}; args: {}",
                plan.method.as_str(),
                plan.executable,
                plan.args.join(" ")
            ),
        ));
        let output = self.process.execute(CliProcessRequest {
            executable: plan.executable.clone(),
            args: plan.args.clone(),
            timeout: PACKAGE_TIMEOUT,
            audit_category: Some(plan.method.audit_category()),
        });
        match output {
            Ok(output) => {
                emit_command_output(operation_id, definition.agent_id, &output, emit);
                if output.success {
                    emit(operation_event(
                        operation_id,
                        definition.agent_id,
                        CliLogLevel::Info,
                        format!(
                            "{} lifecycle operation completed for {}.",
                            plan.method.as_str(),
                            definition.display_name
                        ),
                    ));
                    Ok(())
                } else {
                    let error = first_output_line(&output)
                        .unwrap_or_else(|| format!("{} install failed", plan.method.as_str()));
                    emit(package_diagnostic(
                        operation_id,
                        definition,
                        target_version,
                        plan,
                        &error,
                        Some(&output),
                    ));
                    Err(CliApplicationError::Package(error))
                }
            }
            Err(error) => {
                emit(package_diagnostic(
                    operation_id,
                    definition,
                    target_version,
                    plan,
                    &error,
                    None,
                ));
                Err(CliApplicationError::Package(error))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleMethod {
    Npm,
    Wget,
    Winget,
}

impl LifecycleMethod {
    fn as_str(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Wget => "wget",
            Self::Winget => "winget",
        }
    }

    fn audit_category(self) -> &'static str {
        match self {
            Self::Npm => "cli.npm.install",
            Self::Wget => "cli.wget.install",
            Self::Winget => "cli.winget.upgrade",
        }
    }
}

struct LifecyclePlan {
    method: LifecycleMethod,
    executable: String,
    args: Vec<String>,
    fallback_npm_on_failure: bool,
}

fn lifecycle_plan(
    definition: ToolDefinition,
    status: &CliToolStatus,
    target_version: &str,
) -> Result<LifecyclePlan, CliApplicationError> {
    match status.lifecycle_eligibility {
        LifecycleEligibility::Npm => Ok(LifecyclePlan {
            method: LifecycleMethod::Npm,
            executable: npm_executable().to_string(),
            args: npm_install_args(definition, target_version),
            fallback_npm_on_failure: false,
        }),
        LifecycleEligibility::Wget => Ok(LifecyclePlan {
            method: LifecycleMethod::Wget,
            executable: "bash".to_string(),
            args: wget_script_args(definition).ok_or_else(|| {
                CliApplicationError::Validation(format!(
                    "{} does not have a verified wget installer",
                    definition.display_name
                ))
            })?,
            fallback_npm_on_failure: status.installed != Some(true),
        }),
        LifecycleEligibility::Winget => Ok(LifecyclePlan {
            method: LifecycleMethod::Winget,
            executable: "winget".to_string(),
            args: vec![
                "upgrade".to_string(),
                "--id".to_string(),
                winget_id(definition, status).ok_or_else(|| {
                    CliApplicationError::Validation(format!(
                        "{} does not have a verified WinGet package id",
                        definition.display_name
                    ))
                })?,
                "--exact".to_string(),
                "--accept-package-agreements".to_string(),
                "--accept-source-agreements".to_string(),
            ],
            fallback_npm_on_failure: false,
        }),
        LifecycleEligibility::Manual => validation_error(
            "the active CLI installation must be updated by its source-native installer",
        ),
        LifecycleEligibility::Unavailable => {
            validation_error("the CLI lifecycle method is unavailable")
        }
    }
}

fn npm_install_args(definition: ToolDefinition, target_version: &str) -> Vec<String> {
    vec![
        "install".to_string(),
        "-g".to_string(),
        format!("{}@{}", definition.package_name, target_version),
    ]
}

fn wget_script_args(definition: ToolDefinition) -> Option<Vec<String>> {
    let url = definition.script_install_url?;
    let script = format!(
        "tmp=$(mktemp) && \
         (if command -v wget >/dev/null 2>&1; then wget -qO \"$tmp\" {url}; \
         elif command -v curl >/dev/null 2>&1; then curl -fsSL {url} -o \"$tmp\"; \
         else echo \"wget or curl is required\" >&2; exit 127; fi) && \
         bash \"$tmp\"; status=$?; rm -f \"$tmp\"; exit $status"
    );
    Some(vec!["-lc".to_string(), script])
}

fn winget_id(definition: ToolDefinition, status: &CliToolStatus) -> Option<String> {
    status
        .installations
        .iter()
        .find(|installation| installation.is_active && installation.source == InstallSource::Winget)
        .and_then(|installation| winget_package_id(&installation.path))
        .or_else(|| definition.winget_package_id.map(str::to_string))
}

fn emit_command_output(
    operation_id: &str,
    agent_id: &str,
    output: &CliProcessOutput,
    emit: &mut dyn FnMut(CliLogEvent),
) {
    for line in output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        emit(operation_event(
            operation_id,
            agent_id,
            CliLogLevel::Info,
            line,
        ));
    }
    for line in output
        .stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        emit(operation_event(
            operation_id,
            agent_id,
            CliLogLevel::Warn,
            line,
        ));
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

fn operation_event(
    operation_id: &str,
    agent_id: &str,
    level: CliLogLevel,
    message: impl Into<String>,
) -> CliLogEvent {
    CliLogEvent {
        operation_id: operation_id.to_string(),
        agent_id: Some(agent_id.to_string()),
        level,
        category: CliLogCategory::Operation,
        message: message.into(),
        context: BTreeMap::new(),
    }
}

fn package_diagnostic(
    operation_id: &str,
    definition: ToolDefinition,
    target_version: &str,
    plan: &LifecyclePlan,
    error: &str,
    output: Option<&CliProcessOutput>,
) -> CliLogEvent {
    let mut context = definition_context(definition, operation_id);
    context.insert("targetVersion".to_string(), target_version.to_string());
    context.insert(
        "lifecycleMethod".to_string(),
        plan.method.as_str().to_string(),
    );
    context.insert("executable".to_string(), plan.executable.clone());
    context.insert("arguments".to_string(), plan.args.join(" "));
    if plan.method == LifecycleMethod::Npm {
        context.insert("npmExecutable".to_string(), plan.executable.clone());
        context.insert("npmArguments".to_string(), plan.args.join(" "));
    }
    context.insert("error".to_string(), error.to_string());
    if let Some(output) = output {
        context.insert("stdout".to_string(), output.stdout.clone());
        context.insert("stderr".to_string(), output.stderr.clone());
        context.insert("exitStatus".to_string(), output.status.clone());
    }
    if error.to_ascii_lowercase().contains("timed out") {
        context.insert("timeoutReason".to_string(), error.to_string());
    }
    context.extend(sanitized_environment_context());
    CliLogEvent {
        operation_id: operation_id.to_string(),
        agent_id: Some(definition.agent_id.to_string()),
        level: CliLogLevel::Error,
        category: CliLogCategory::Diagnostic,
        message: "CLI package operation failed.".to_string(),
        context,
    }
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

fn sanitized_environment_context() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("os".to_string(), std::env::consts::OS.to_string()),
        ("arch".to_string(), std::env::consts::ARCH.to_string()),
        (
            "pathConfigured".to_string(),
            std::env::var_os("PATH").is_some().to_string(),
        ),
        (
            "npmConfigUserconfigConfigured".to_string(),
            std::env::var_os("NPM_CONFIG_USERCONFIG")
                .is_some()
                .to_string(),
        ),
    ])
}

fn validation_error<T>(message: &str) -> Result<T, CliApplicationError> {
    Err(CliApplicationError::Validation(message.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli::domain::{
        definition, EnvironmentType, Installation, VersionCheckStatus,
    };
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

    fn status(agent_id: &str, eligibility: LifecycleEligibility) -> CliToolStatus {
        let definition = definition(agent_id).expect("definition");
        let mut status = CliToolStatus::unavailable(
            definition,
            EnvironmentType::Linux,
            format!("npm install -g {}@latest", definition.package_name),
        );
        status.lifecycle_eligibility = eligibility;
        status.version_check_status = VersionCheckStatus::Succeeded;
        status
    }

    #[test]
    fn validation_rechecks_source_and_multiple_path_confirmation() {
        let process = Arc::new(FakeProcess::new(Vec::new()));
        let definition = definition("codex-cli").expect("definition");
        let changed = CliPackageAdapter::with_dependencies(
            process.clone(),
            Arc::new(FakeCandidates(vec![PathBuf::from("/usr/bin/codex")])),
        );
        let status = status("codex-cli", LifecycleEligibility::Npm);

        let error = changed
            .validate(definition, &status, None)
            .expect_err("source change");

        assert_eq!(
            error.to_string(),
            "the active CLI path changed and is not npm-managed"
        );

        let first = "/fixture/.npm/bin/codex";
        let multiple = CliPackageAdapter::with_dependencies(
            process,
            Arc::new(FakeCandidates(vec![
                PathBuf::from(first),
                PathBuf::from("/other/.npm/bin/codex"),
            ])),
        );
        assert_eq!(
            multiple
                .validate(definition, &status, None)
                .expect_err("confirmation")
                .to_string(),
            "multiple CLI installations require confirmation of the active path"
        );
        multiple
            .validate(definition, &status, Some(first))
            .expect("confirmed path");
    }

    #[test]
    fn npm_execution_uses_explicit_arguments_timeout_and_structured_output() {
        let process = Arc::new(FakeProcess::new(vec![Ok(output(
            true,
            "installed\ncomplete",
            "warning",
        ))]));
        let adapter = CliPackageAdapter::with_dependencies(
            process.clone(),
            Arc::new(FakeCandidates(Vec::new())),
        );
        let definition = definition("codex-cli").expect("definition");
        let status = status("codex-cli", LifecycleEligibility::Npm);
        let mut events = Vec::new();

        adapter
            .execute("op-1", definition, &status, "1.2.3", &mut |event| {
                events.push(event)
            })
            .expect("execute");

        let requests = process.requests.lock().expect("requests");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].executable, npm_executable());
        assert_eq!(requests[0].args, ["install", "-g", "@openai/codex@1.2.3"]);
        assert_eq!(requests[0].timeout, PACKAGE_TIMEOUT);
        assert_eq!(requests[0].audit_category, Some("cli.npm.install"));
        assert!(events.iter().any(|event| event.message == "installed"));
        assert!(events
            .iter()
            .any(|event| event.level == CliLogLevel::Warn && event.message == "warning"));
        assert!(events
            .iter()
            .all(|event| event.category == CliLogCategory::Operation));
    }

    #[test]
    fn first_script_install_falls_back_to_explicit_npm_arguments() {
        let process = Arc::new(FakeProcess::new(vec![
            Err("command timed out".to_string()),
            Ok(output(true, "installed", "")),
        ]));
        let adapter = CliPackageAdapter::with_dependencies(
            process.clone(),
            Arc::new(FakeCandidates(Vec::new())),
        );
        let definition = definition("claude-code").expect("definition");
        let status = status("claude-code", LifecycleEligibility::Wget);
        let mut events = Vec::new();

        adapter
            .execute("op-2", definition, &status, "9.9.9", &mut |event| {
                events.push(event)
            })
            .expect("fallback succeeds");

        let requests = process.requests.lock().expect("requests");
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].executable, "bash");
        assert_eq!(requests[0].args[0], "-lc");
        assert!(!requests[0].args[1].contains("9.9.9"));
        assert_eq!(
            requests[1].args,
            ["install", "-g", "@anthropic-ai/claude-code@9.9.9"]
        );
        assert!(events.iter().any(|event| {
            event.message == "wget installer failed; falling back to npm for first install."
        }));
        assert!(events.iter().any(|event| {
            event.category == CliLogCategory::Diagnostic
                && event.context.get("timeoutReason").map(String::as_str)
                    == Some("command timed out")
        }));
    }

    #[test]
    fn winget_execution_uses_verified_id_and_fixed_flags() {
        let process = Arc::new(FakeProcess::new(vec![Ok(output(true, "upgraded", ""))]));
        let adapter = CliPackageAdapter::with_dependencies(
            process.clone(),
            Arc::new(FakeCandidates(Vec::new())),
        );
        let definition = definition("claude-code").expect("definition");
        let mut status = status("claude-code", LifecycleEligibility::Winget);
        status.installations = vec![Installation {
            path: "C:\\Users\\dev\\Microsoft\\WinGet\\Packages\\Anthropic.ClaudeCode_Microsoft.Winget.Source_8wekyb3d8bbwe\\claude.exe".to_string(),
            version: Some("1.0.0".to_string()),
            runnable: true,
            error: None,
            source: InstallSource::Winget,
            environment_type: EnvironmentType::Windows,
            is_active: true,
        }];

        adapter
            .execute("op-3", definition, &status, "2.0.0", &mut |_| {})
            .expect("execute");

        let requests = process.requests.lock().expect("requests");
        assert_eq!(requests[0].executable, "winget");
        assert_eq!(
            requests[0].args,
            [
                "upgrade",
                "--id",
                "Anthropic.ClaudeCode",
                "--exact",
                "--accept-package-agreements",
                "--accept-source-agreements",
            ]
        );
        assert_eq!(requests[0].timeout, PACKAGE_TIMEOUT);
    }
}
