use super::process_adapter::{
    platform_process_runner, SdkProcessOutput, SdkProcessRequest, SdkProcessRunner,
};
use super::sqlite_repository::{dependencies_root, package_dir, sdk_dir};
use crate::contexts::tooling::sdk::application::{
    SdkApplicationError, SdkEnvironmentStatus, SdkLogLevel, SdkPackageExecutionPort, SdkPackageLog,
};
use crate::contexts::tooling::sdk::domain::{
    definition, SdkDefinition, SdkLifecycleAction, SdkLifecyclePlan,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RESOLVE_TIMEOUT: Duration = Duration::from_secs(2);
const VERSION_TIMEOUT: Duration = Duration::from_secs(2);
const LATEST_VERSION_TIMEOUT: Duration = Duration::from_secs(5);
const AVAILABLE_VERSIONS_TIMEOUT: Duration = Duration::from_secs(30);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(300);
const MANIFEST_FILE: &str = "manifest.json";
const INSTALLED_MARKER: &str = ".installed";

#[derive(Clone)]
pub(crate) struct SdkPackageAdapter {
    process: Arc<dyn SdkProcessRunner>,
    dependencies_root: PathBuf,
}

impl SdkPackageAdapter {
    pub(crate) fn new() -> Self {
        Self {
            process: platform_process_runner(),
            dependencies_root: dependencies_root(),
        }
    }

    #[cfg(test)]
    fn with_dependencies(process: Arc<dyn SdkProcessRunner>, dependencies_root: PathBuf) -> Self {
        Self {
            process,
            dependencies_root,
        }
    }
}

impl SdkPackageExecutionPort for SdkPackageAdapter {
    fn environment(&self) -> Result<SdkEnvironmentStatus, SdkApplicationError> {
        let node_path = self.find_command("node");
        let npm_path = self
            .find_command(npm_executable())
            .or_else(|| self.find_command("npm"));
        let node_version = node_path
            .as_deref()
            .and_then(|path| self.command_version(path));
        let npm_version = npm_path
            .as_deref()
            .and_then(|path| self.command_version(path));
        let available = node_version.is_some() && npm_version.is_some();
        Ok(SdkEnvironmentStatus {
            available,
            node_path,
            node_version,
            npm_path,
            npm_version,
            error: (!available).then(|| "Node.js or npm was not found on PATH.".to_string()),
        })
    }

    fn available_versions(
        &self,
        definition: SdkDefinition,
    ) -> Result<Vec<String>, SdkApplicationError> {
        let output = self.npm_capture(
            vec![
                "view".to_string(),
                definition.npm_package.to_string(),
                "versions".to_string(),
                "--json".to_string(),
            ],
            AVAILABLE_VERSIONS_TIMEOUT,
        )?;
        let value = serde_json::from_str::<Value>(&output).map_err(package_error)?;
        Ok(value
            .as_array()
            .map(|versions| {
                versions
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default())
    }

    fn latest_version(&self, definition: SdkDefinition) -> Result<String, SdkApplicationError> {
        self.npm_capture(
            vec![
                "view".to_string(),
                definition.npm_package.to_string(),
                "version".to_string(),
            ],
            LATEST_VERSION_TIMEOUT,
        )
    }

    fn execute(
        &self,
        _operation_id: &str,
        plan: &SdkLifecyclePlan,
        emit: &mut dyn FnMut(SdkPackageLog),
    ) -> Result<Option<String>, SdkApplicationError> {
        match plan.action {
            SdkLifecycleAction::InstallPackages => self.install(plan, emit).map(Some),
            SdkLifecycleAction::RemoveInstallation => {
                self.uninstall(plan, emit)?;
                Ok(None)
            }
        }
    }
}

impl SdkPackageAdapter {
    fn install(
        &self,
        plan: &SdkLifecyclePlan,
        emit: &mut dyn FnMut(SdkPackageLog),
    ) -> Result<String, SdkApplicationError> {
        let definition = definition(plan.sdk_id);
        let environment = self.environment()?;
        if !environment.available {
            return Err(SdkApplicationError::Package(
                environment
                    .error
                    .unwrap_or_else(|| "Node.js or npm is unavailable".to_string()),
            ));
        }
        let target_version = plan.requested_version.as_deref().ok_or_else(|| {
            SdkApplicationError::Validation(
                "SDK install operation is missing a target version".to_string(),
            )
        })?;
        let target = sdk_dir(&self.dependencies_root, plan.sdk_id);
        ensure_child(&self.dependencies_root, &target)?;
        std::fs::create_dir_all(&target).map_err(package_error)?;
        create_package_json(&target, plan.sdk_id.as_str())?;

        let npm = environment
            .npm_path
            .unwrap_or_else(|| npm_executable().to_string());
        emit(package_log(
            SdkLogLevel::Info,
            format!("Using npm: {npm}"),
            BTreeMap::from([("executable".to_string(), npm.clone())]),
        ));
        emit(package_log(
            SdkLogLevel::Info,
            format!("Installing {}", plan.package_specs.join(" ")),
            BTreeMap::new(),
        ));
        let mut args = vec![
            "install".to_string(),
            "--include=optional".to_string(),
            "--ignore-scripts".to_string(),
            "--prefix".to_string(),
            target.to_string_lossy().to_string(),
        ];
        args.extend(plan.package_specs.iter().cloned());
        let output = self
            .process
            .execute(SdkProcessRequest {
                executable: npm,
                args,
                current_dir: None,
                timeout: INSTALL_TIMEOUT,
                audit_category: Some("sdk.npm.install"),
            })
            .map_err(SdkApplicationError::Package)?;
        emit_output(&output, emit);
        if !output.success {
            return Err(SdkApplicationError::Package(output_error(
                &output,
                "npm install failed",
            )));
        }

        let installed = installed_version(&self.dependencies_root, definition)
            .unwrap_or_else(|| target_version.to_string());
        std::fs::write(target.join(INSTALLED_MARKER), &installed).map_err(package_error)?;
        update_manifest(
            &self.dependencies_root,
            plan.sdk_id.as_str(),
            Some(&installed),
        )?;
        emit(package_log(
            SdkLogLevel::Info,
            format!("Installed version: {installed}"),
            BTreeMap::new(),
        ));
        Ok(installed)
    }

    fn uninstall(
        &self,
        plan: &SdkLifecyclePlan,
        emit: &mut dyn FnMut(SdkPackageLog),
    ) -> Result<(), SdkApplicationError> {
        let target = sdk_dir(&self.dependencies_root, plan.sdk_id);
        ensure_child(&self.dependencies_root, &target)?;
        emit(package_log(
            SdkLogLevel::Info,
            format!("Removing {}", target.display()),
            BTreeMap::new(),
        ));
        if target.exists() {
            std::fs::remove_dir_all(&target).map_err(package_error)?;
            emit(package_log(
                SdkLogLevel::Info,
                "SDK directory removed".to_string(),
                BTreeMap::new(),
            ));
        } else {
            emit(package_log(
                SdkLogLevel::Info,
                "SDK directory does not exist; nothing to remove".to_string(),
                BTreeMap::new(),
            ));
        }
        update_manifest(&self.dependencies_root, plan.sdk_id.as_str(), None)
    }

    fn npm_capture(
        &self,
        args: Vec<String>,
        timeout: Duration,
    ) -> Result<String, SdkApplicationError> {
        let environment = self.environment()?;
        if !environment.available {
            return Err(SdkApplicationError::Package(
                environment
                    .error
                    .unwrap_or_else(|| "Node.js or npm is unavailable".to_string()),
            ));
        }
        let npm = environment
            .npm_path
            .unwrap_or_else(|| npm_executable().to_string());
        let output = self
            .process
            .execute(SdkProcessRequest {
                executable: npm,
                args,
                current_dir: None,
                timeout,
                audit_category: Some("sdk.npm.capture"),
            })
            .map_err(SdkApplicationError::Package)?;
        if output.success {
            Ok(output.stdout.trim().to_string())
        } else {
            Err(SdkApplicationError::Package(output_error(
                &output,
                "npm view failed",
            )))
        }
    }

    fn find_command(&self, executable: &str) -> Option<String> {
        let resolver = if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        };
        let output = self
            .process
            .execute(SdkProcessRequest {
                executable: resolver.to_string(),
                args: vec![executable.to_string()],
                current_dir: None,
                timeout: RESOLVE_TIMEOUT,
                audit_category: None,
            })
            .ok()?;
        output.success.then_some(output.stdout).and_then(|stdout| {
            stdout
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_string)
        })
    }

    fn command_version(&self, executable: &str) -> Option<String> {
        let output = self
            .process
            .execute(SdkProcessRequest {
                executable: executable.to_string(),
                args: vec!["--version".to_string()],
                current_dir: None,
                timeout: VERSION_TIMEOUT,
                audit_category: None,
            })
            .ok()?;
        output
            .success
            .then(|| output.stdout.trim().to_string())
            .filter(|version| !version.is_empty())
    }
}

fn npm_executable() -> &'static str {
    if cfg!(target_os = "windows") {
        "npm.cmd"
    } else {
        "npm"
    }
}

fn ensure_child(root: &Path, target: &Path) -> Result<(), SdkApplicationError> {
    let root = normalize_path(root);
    let target = normalize_path(target);
    if target.starts_with(&root) && target != root {
        Ok(())
    } else {
        Err(SdkApplicationError::Validation(
            "SDK path is outside the VaneHub dependencies directory".to_string(),
        ))
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
        if let Ok(parent) = parent.canonicalize() {
            return parent.join(name);
        }
    }
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

fn create_package_json(target: &Path, sdk_id: &str) -> Result<(), SdkApplicationError> {
    let package_json = serde_json::json!({
        "name": format!("{sdk_id}-container"),
        "version": "1.0.0",
        "private": true
    });
    let content = serde_json::to_string_pretty(&package_json).map_err(package_error)?;
    std::fs::write(target.join("package.json"), content).map_err(package_error)
}

fn installed_version(root: &Path, definition: SdkDefinition) -> Option<String> {
    let raw = std::fs::read_to_string(package_dir(root, definition).join("package.json")).ok()?;
    serde_json::from_str::<Value>(&raw)
        .ok()?
        .get("version")?
        .as_str()
        .map(str::to_string)
}

fn update_manifest(
    root: &Path,
    sdk_id: &str,
    version: Option<&str>,
) -> Result<(), SdkApplicationError> {
    std::fs::create_dir_all(root).map_err(package_error)?;
    let path = root.join(MANIFEST_FILE);
    let mut manifest = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    match version {
        Some(version) => {
            manifest.insert(
                sdk_id.to_string(),
                serde_json::json!({ "version": version, "installedAt": now_string() }),
            );
        }
        None => {
            manifest.remove(sdk_id);
        }
    }
    let content = serde_json::to_string_pretty(&manifest).map_err(package_error)?;
    std::fs::write(path, content).map_err(package_error)
}

fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn emit_output(output: &SdkProcessOutput, emit: &mut dyn FnMut(SdkPackageLog)) {
    for line in output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        emit(package_log(
            SdkLogLevel::Info,
            line.to_string(),
            BTreeMap::new(),
        ));
    }
    for line in output
        .stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        emit(package_log(
            SdkLogLevel::Warn,
            line.to_string(),
            BTreeMap::new(),
        ));
    }
}

fn package_log(
    level: SdkLogLevel,
    line: String,
    context: BTreeMap<String, String>,
) -> SdkPackageLog {
    SdkPackageLog {
        level,
        line,
        context,
    }
}

fn output_error(output: &SdkProcessOutput, fallback: &str) -> String {
    output
        .stderr
        .lines()
        .chain(output.stdout.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{fallback} with status {}", output.status))
}

fn package_error(error: impl std::fmt::Display) -> SdkApplicationError {
    SdkApplicationError::Package(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::sdk::domain::{lifecycle_plan, SdkId, SdkOperationType};
    use crate::test_support::TempDirectory;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct FakeProcess {
        responses: Mutex<VecDeque<Result<SdkProcessOutput, String>>>,
        requests: Mutex<Vec<SdkProcessRequest>>,
    }

    impl FakeProcess {
        fn new(responses: Vec<Result<SdkProcessOutput, String>>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    impl SdkProcessRunner for FakeProcess {
        fn execute(&self, request: SdkProcessRequest) -> Result<SdkProcessOutput, String> {
            self.requests.lock().expect("requests").push(request);
            self.responses
                .lock()
                .expect("responses")
                .pop_front()
                .expect("fake response")
        }
    }

    fn output(success: bool, stdout: &str, stderr: &str) -> Result<SdkProcessOutput, String> {
        Ok(SdkProcessOutput {
            success,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            status: if success { "0" } else { "1" }.to_string(),
        })
    }

    fn environment_responses() -> Vec<Result<SdkProcessOutput, String>> {
        vec![
            output(true, "/bin/node", ""),
            output(true, "/bin/npm", ""),
            output(true, "v22.0.0", ""),
            output(true, "10.0.0", ""),
        ]
    }

    #[test]
    fn environment_and_version_lookup_use_bounded_explicit_process_requests() {
        let mut responses = environment_responses();
        responses.extend(environment_responses());
        responses.push(output(
            true,
            r#"["0.115.0","0.117.0","0.116.0-beta.1"]"#,
            "",
        ));
        responses.extend(environment_responses());
        responses.push(output(true, "0.117.0", ""));
        let process = Arc::new(FakeProcess::new(responses));
        let root = TempDirectory::new("sdk-package-version-root");
        let adapter =
            SdkPackageAdapter::with_dependencies(process.clone(), root.path().to_path_buf());

        assert!(adapter.environment().expect("environment").available);
        let versions = adapter
            .available_versions(definition(SdkId::CodexSdk))
            .expect("versions");
        assert_eq!(versions.len(), 3);
        assert_eq!(
            adapter
                .latest_version(definition(SdkId::CodexSdk))
                .expect("latest version"),
            "0.117.0"
        );

        let requests = process.requests.lock().expect("requests");
        assert_eq!(requests[0].timeout, RESOLVE_TIMEOUT);
        assert_eq!(requests[2].args, ["--version"]);
        let versions_view = &requests[8];
        assert_eq!(
            versions_view.args,
            ["view", "@openai/codex-sdk", "versions", "--json"]
        );
        assert_eq!(versions_view.timeout, AVAILABLE_VERSIONS_TIMEOUT);
        let latest_view = requests.last().expect("latest npm view");
        assert_eq!(latest_view.args, ["view", "@openai/codex-sdk", "version"]);
        assert_eq!(latest_view.timeout, LATEST_VERSION_TIMEOUT);
        assert_eq!(latest_view.audit_category, Some("sdk.npm.capture"));
    }

    #[test]
    fn rollback_install_uses_fixed_npm_arguments_and_updates_manifest() {
        let mut responses = environment_responses();
        responses.push(output(true, "installed", "warning line"));
        let process = Arc::new(FakeProcess::new(responses));
        let root = TempDirectory::new("sdk-package-rollback-root");
        let adapter =
            SdkPackageAdapter::with_dependencies(process.clone(), root.path().to_path_buf());
        let plan = lifecycle_plan(SdkId::ClaudeSdk, SdkOperationType::Rollback, Some("0.2.58"));
        let mut logs = Vec::new();

        let installed = adapter
            .execute("sdk-op-1", &plan, &mut |log| logs.push(log))
            .expect("install");

        assert_eq!(installed.as_deref(), Some("0.2.58"));
        let requests = process.requests.lock().expect("requests");
        let install = requests.last().expect("install request");
        assert_eq!(install.executable, "/bin/npm");
        assert_eq!(
            install.args[0..4],
            [
                "install",
                "--include=optional",
                "--ignore-scripts",
                "--prefix"
            ]
        );
        assert!(install
            .args
            .contains(&"@anthropic-ai/claude-agent-sdk@0.2.58".to_string()));
        assert!(install.args.contains(&"@anthropic-ai/sdk".to_string()));
        assert_eq!(install.timeout, INSTALL_TIMEOUT);
        assert_eq!(install.audit_category, Some("sdk.npm.install"));
        assert!(root
            .path()
            .join("claude-sdk")
            .join(INSTALLED_MARKER)
            .is_file());
        let manifest = std::fs::read_to_string(root.path().join(MANIFEST_FILE)).expect("manifest");
        assert!(manifest.contains("0.2.58"));
        assert!(logs.iter().any(|log| log.level == SdkLogLevel::Warn));
    }

    #[test]
    fn uninstall_is_bounded_and_removes_manifest_entry_without_processes() {
        let process = Arc::new(FakeProcess::new(Vec::new()));
        let root = TempDirectory::new("sdk-package-uninstall-root");
        let target = root.path().join("codex-sdk");
        std::fs::create_dir_all(&target).expect("target");
        std::fs::write(
            root.path().join(MANIFEST_FILE),
            r#"{"codex-sdk":{"version":"0.117.0"}}"#,
        )
        .expect("manifest");
        let adapter =
            SdkPackageAdapter::with_dependencies(process.clone(), root.path().to_path_buf());
        let plan = lifecycle_plan(SdkId::CodexSdk, SdkOperationType::Uninstall, None);

        adapter
            .execute("sdk-op-2", &plan, &mut |_| {})
            .expect("uninstall");

        assert!(!target.exists());
        let manifest = std::fs::read_to_string(root.path().join(MANIFEST_FILE)).expect("manifest");
        assert!(!manifest.contains("codex-sdk"));
        assert!(process.requests.lock().expect("requests").is_empty());
    }
}
