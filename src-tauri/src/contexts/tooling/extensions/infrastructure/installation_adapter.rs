use super::process_adapter::{
    platform_process_runner, ExtensionProcessOutput, ExtensionProcessRequest,
    ExtensionProcessRunner,
};
use crate::contexts::tooling::extensions::application::{
    ExtensionApplicationError, ExtensionEnvironmentPort, ExtensionExecutionLog,
    ExtensionInstallationPort, ExtensionLogLevel, InstallationInspection, InstalledExtension,
};
use crate::contexts::tooling::extensions::domain::{
    definition, ExtensionFrameworkId, ExtensionInstallationObservation, HostEnvironment,
    InstallPlan, InstallationVerification, PythonRuntime, RemovalPlan, SelfTestPlan,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const INSTALLED_MARKER: &str = ".vanehub-installed";
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(2);
const VENV_TIMEOUT: Duration = Duration::from_secs(120);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(1_800);
const VERIFY_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub(crate) struct SystemExtensionEnvironment {
    process: Arc<dyn ExtensionProcessRunner>,
}

impl SystemExtensionEnvironment {
    pub(crate) fn new() -> Self {
        Self {
            process: platform_process_runner(),
        }
    }

    #[cfg(test)]
    fn with_process(process: Arc<dyn ExtensionProcessRunner>) -> Self {
        Self { process }
    }
}

impl ExtensionEnvironmentPort for SystemExtensionEnvironment {
    fn observe_host(&self) -> Result<HostEnvironment, ExtensionApplicationError> {
        Ok(HostEnvironment {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            python: self.resolve_python(),
        })
    }
}

impl SystemExtensionEnvironment {
    fn resolve_python(&self) -> Option<PythonRuntime> {
        ["python", "python3"].into_iter().find_map(|candidate| {
            let output = self
                .process
                .execute(ExtensionProcessRequest {
                    executable: candidate.to_string(),
                    args: vec!["--version".to_string()],
                    current_dir: None,
                    timeout: RESOLVE_TIMEOUT,
                    audit_category: "extension.environment",
                })
                .ok()?;
            if !output.success {
                return None;
            }
            let raw = if output.stdout.trim().is_empty() {
                output.stderr.trim()
            } else {
                output.stdout.trim()
            };
            Some(PythonRuntime {
                path: candidate.to_string(),
                version: raw.strip_prefix("Python ").unwrap_or(raw).to_string(),
            })
        })
    }
}

#[derive(Clone)]
pub(crate) struct ManagedExtensionInstallation {
    root: PathBuf,
    process: Arc<dyn ExtensionProcessRunner>,
}

impl ManagedExtensionInstallation {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            process: platform_process_runner(),
        }
    }

    #[cfg(test)]
    fn with_dependencies(root: PathBuf, process: Arc<dyn ExtensionProcessRunner>) -> Self {
        Self { root, process }
    }

    fn target(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<PathBuf, ExtensionApplicationError> {
        let target = self.root.join(framework_id.as_str());
        ensure_owned_target(&self.root, &target, framework_id)?;
        Ok(target)
    }
}

impl ExtensionInstallationPort for ManagedExtensionInstallation {
    fn managed_path(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<String, ExtensionApplicationError> {
        Ok(self.target(framework_id)?.to_string_lossy().to_string())
    }

    fn inspect(
        &self,
        framework_id: ExtensionFrameworkId,
        inspection: InstallationInspection,
    ) -> Result<ExtensionInstallationObservation, ExtensionApplicationError> {
        let target = self.target(framework_id)?;
        if !target.exists() {
            return Ok(ExtensionInstallationObservation::absent());
        }
        let interpreter = venv_python(&target);
        let marker_version = read_marker(&target)?;
        let verification = if inspection == InstallationInspection::VerifyImport
            && interpreter.is_file()
            && marker_version.is_some()
        {
            self.verify_observation(framework_id, &interpreter)
        } else {
            InstallationVerification::NotChecked
        };
        Ok(ExtensionInstallationObservation {
            managed_directory_exists: target.is_dir(),
            interpreter_exists: interpreter.is_file(),
            marker_version,
            verification,
        })
    }

    fn install(
        &self,
        _operation_id: &str,
        plan: &InstallPlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<InstalledExtension, ExtensionApplicationError> {
        std::fs::create_dir_all(&self.root).map_err(installation_error)?;
        let target = self.target(plan.definition.id)?;
        std::fs::create_dir_all(&target).map_err(installation_error)?;
        emit(info(format!(
            "Preparing {} managed environment",
            plan.definition.id.as_str()
        )));
        emit(info(format!(
            "Creating virtual environment at {}",
            target.display()
        )));
        self.run_checked(
            ExtensionProcessRequest {
                executable: plan.python.path.clone(),
                args: vec![
                    "-m".to_string(),
                    "venv".to_string(),
                    target.to_string_lossy().to_string(),
                ],
                current_dir: None,
                timeout: VENV_TIMEOUT,
                audit_category: "extension.operation",
            },
            emit,
        )?;

        let interpreter = venv_python(&target);
        let mut args = vec![
            "-m".to_string(),
            "pip".to_string(),
            "install".to_string(),
            "--disable-pip-version-check".to_string(),
        ];
        args.extend(
            plan.definition
                .requirement
                .packages
                .iter()
                .map(|package| (*package).to_string()),
        );
        emit(info(format!(
            "Installing allowlisted packages: {}",
            plan.definition.requirement.packages.join(", ")
        )));
        self.run_checked(
            ExtensionProcessRequest {
                executable: interpreter.to_string_lossy().to_string(),
                args,
                current_dir: None,
                timeout: INSTALL_TIMEOUT,
                audit_category: "extension.operation",
            },
            emit,
        )?;

        let version = self.installed_version(plan.definition.id, &interpreter, None)?;
        emit(info(format!(
            "Verifying {} import",
            plan.definition.requirement.import_module
        )));
        self.verify_import(plan.definition.id, &interpreter, emit)?;
        std::fs::write(target.join(INSTALLED_MARKER), &version).map_err(installation_error)?;
        emit(info("Framework installation verified"));
        Ok(InstalledExtension {
            install_path: target.to_string_lossy().to_string(),
            installed_version: version,
        })
    }

    fn rollback_installation(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<(), ExtensionApplicationError> {
        let marker = self.target(framework_id)?.join(INSTALLED_MARKER);
        match std::fs::remove_file(marker) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(installation_error(error)),
        }
    }

    fn remove(
        &self,
        _operation_id: &str,
        plan: &RemovalPlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<(), ExtensionApplicationError> {
        let target = self.target(plan.framework_id)?;
        emit(info(format!(
            "Removing managed framework directory {}",
            target.display()
        )));
        if target.exists() {
            std::fs::remove_dir_all(target).map_err(installation_error)?;
        }
        Ok(())
    }

    fn self_test(
        &self,
        _operation_id: &str,
        plan: &SelfTestPlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<(), ExtensionApplicationError> {
        let target = self.target(plan.framework_id)?;
        let interpreter = venv_python(&target);
        if !interpreter.is_file() || !target.join(INSTALLED_MARKER).is_file() {
            return Err(ExtensionApplicationError::Installation(
                "Managed framework environment is not installed".to_string(),
            ));
        }
        emit(info(format!(
            "Loading {} from the managed environment",
            plan.import_module
        )));
        self.verify_import(plan.framework_id, &interpreter, emit)
    }
}

impl ManagedExtensionInstallation {
    fn verify_observation(
        &self,
        framework_id: ExtensionFrameworkId,
        interpreter: &Path,
    ) -> InstallationVerification {
        let definition = definition(framework_id);
        let code = format!(
            "import importlib.metadata as m; import {}; print(m.version('{}'))",
            definition.requirement.import_module, definition.requirement.version_package
        );
        match self.process.execute(ExtensionProcessRequest {
            executable: interpreter.to_string_lossy().to_string(),
            args: vec!["-c".to_string(), code],
            current_dir: None,
            timeout: VERIFY_TIMEOUT,
            audit_category: "extension.health",
        }) {
            Ok(output) if output.success => InstallationVerification::Passed {
                installed_version: nonempty_output(&output),
            },
            Ok(output) => InstallationVerification::Failed(output_error(
                &output,
                "managed framework verification failed",
            )),
            Err(error) => InstallationVerification::Failed(error),
        }
    }

    fn installed_version(
        &self,
        framework_id: ExtensionFrameworkId,
        interpreter: &Path,
        emit: Option<&mut dyn FnMut(ExtensionExecutionLog)>,
    ) -> Result<String, ExtensionApplicationError> {
        let definition = definition(framework_id);
        let code = format!(
            "import importlib.metadata as m; print(m.version('{}'))",
            definition.requirement.version_package
        );
        let request = ExtensionProcessRequest {
            executable: interpreter.to_string_lossy().to_string(),
            args: vec!["-c".to_string(), code],
            current_dir: None,
            timeout: VERIFY_TIMEOUT,
            audit_category: "extension.operation",
        };
        let output = match emit {
            Some(emit) => self.run_checked(request, emit)?,
            None => self.execute_checked(request)?,
        };
        nonempty_output(&output).ok_or_else(|| {
            ExtensionApplicationError::Installation(
                "managed framework version is unavailable".to_string(),
            )
        })
    }

    fn verify_import(
        &self,
        framework_id: ExtensionFrameworkId,
        interpreter: &Path,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<(), ExtensionApplicationError> {
        let module = definition(framework_id).requirement.import_module;
        let code = format!("import {module}; print('self-test-ok')");
        self.run_checked(
            ExtensionProcessRequest {
                executable: interpreter.to_string_lossy().to_string(),
                args: vec!["-c".to_string(), code],
                current_dir: None,
                timeout: VERIFY_TIMEOUT,
                audit_category: "extension.operation",
            },
            emit,
        )?;
        Ok(())
    }

    fn run_checked(
        &self,
        request: ExtensionProcessRequest,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<ExtensionProcessOutput, ExtensionApplicationError> {
        let output = self
            .process
            .execute(request)
            .map_err(ExtensionApplicationError::Installation)?;
        emit_output(&output, emit);
        if output.success {
            Ok(output)
        } else {
            Err(ExtensionApplicationError::Installation(format!(
                "allowlisted extension command failed with {}",
                output.status
            )))
        }
    }

    fn execute_checked(
        &self,
        request: ExtensionProcessRequest,
    ) -> Result<ExtensionProcessOutput, ExtensionApplicationError> {
        let output = self
            .process
            .execute(request)
            .map_err(ExtensionApplicationError::Installation)?;
        if output.success {
            Ok(output)
        } else {
            Err(ExtensionApplicationError::Installation(format!(
                "allowlisted extension command failed with {}",
                output.status
            )))
        }
    }
}

pub(super) fn ensure_owned_target(
    root: &Path,
    target: &Path,
    framework_id: ExtensionFrameworkId,
) -> Result<(), ExtensionApplicationError> {
    if target == root || target != root.join(framework_id.as_str()) {
        return Err(ExtensionApplicationError::Installation(
            "extension path is outside its managed directory".to_string(),
        ));
    }
    if target.exists() {
        let canonical_root = root.canonicalize().map_err(installation_error)?;
        let canonical_target = target.canonicalize().map_err(installation_error)?;
        if canonical_target.parent() != Some(canonical_root.as_path()) {
            return Err(ExtensionApplicationError::Installation(
                "extension path is outside its managed directory".to_string(),
            ));
        }
    }
    Ok(())
}

pub(super) fn venv_python(target: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        target.join("Scripts").join("python.exe")
    } else {
        target.join("bin").join("python")
    }
}

fn read_marker(target: &Path) -> Result<Option<String>, ExtensionApplicationError> {
    match std::fs::read_to_string(target.join(INSTALLED_MARKER)) {
        Ok(version) => Ok(Some(version.trim().to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(installation_error(error)),
    }
}

fn emit_output(output: &ExtensionProcessOutput, emit: &mut dyn FnMut(ExtensionExecutionLog)) {
    for line in output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        emit(log(ExtensionLogLevel::Info, line));
    }
    for line in output
        .stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        emit(log(ExtensionLogLevel::Warn, line));
    }
}

fn info(line: impl Into<String>) -> ExtensionExecutionLog {
    ExtensionExecutionLog::info(line)
}

fn log(level: ExtensionLogLevel, line: impl Into<String>) -> ExtensionExecutionLog {
    ExtensionExecutionLog {
        level,
        line: line.into(),
        context: BTreeMap::new(),
    }
}

fn nonempty_output(output: &ExtensionProcessOutput) -> Option<String> {
    output
        .stdout
        .lines()
        .chain(output.stderr.lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn output_error(output: &ExtensionProcessOutput, fallback: &str) -> String {
    nonempty_output(output).unwrap_or_else(|| format!("{fallback} with status {}", output.status))
}

fn installation_error(error: impl std::fmt::Display) -> ExtensionApplicationError {
    ExtensionApplicationError::Installation(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::extensions::domain::{
        definition, ExtensionFrameworkId, InstallPlan, PythonRuntime,
    };
    use crate::test_support::TempDirectory;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    struct FakeProcess {
        responses: Mutex<VecDeque<Result<ExtensionProcessOutput, String>>>,
        requests: Mutex<Vec<ExtensionProcessRequest>>,
    }

    impl FakeProcess {
        fn new(responses: Vec<Result<ExtensionProcessOutput, String>>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
                requests: Mutex::new(Vec::new()),
            }
        }
    }

    impl ExtensionProcessRunner for FakeProcess {
        fn execute(
            &self,
            request: ExtensionProcessRequest,
        ) -> Result<ExtensionProcessOutput, String> {
            self.requests.lock().expect("requests").push(request);
            self.responses
                .lock()
                .expect("responses")
                .pop_front()
                .expect("response")
        }
    }

    fn output(stdout: &str, stderr: &str) -> Result<ExtensionProcessOutput, String> {
        Ok(ExtensionProcessOutput {
            success: true,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            status: "0".to_string(),
        })
    }

    #[test]
    fn environment_detection_uses_bounded_python_version_requests() {
        let process = Arc::new(FakeProcess::new(vec![output("Python 3.12.4", "")]));
        let environment = SystemExtensionEnvironment::with_process(process.clone())
            .observe_host()
            .expect("environment");

        assert_eq!(environment.python.expect("python").version, "3.12.4");
        let requests = process.requests.lock().expect("requests");
        assert_eq!(requests[0].executable, "python");
        assert_eq!(requests[0].args, ["--version"]);
        assert_eq!(requests[0].timeout, RESOLVE_TIMEOUT);
    }

    #[test]
    fn install_uses_fixed_allowlisted_arguments_and_writes_marker_after_verification() {
        let root = TempDirectory::new("extension-install-adapter");
        let process = Arc::new(FakeProcess::new(vec![
            output("", ""),
            output("pip installed", "warning"),
            output("3.2.0", ""),
            output("self-test-ok", ""),
        ]));
        let adapter = ManagedExtensionInstallation::with_dependencies(
            root.path().to_path_buf(),
            process.clone(),
        );
        let mut logs = Vec::new();
        let installed = adapter
            .install(
                "extension-op-1",
                &InstallPlan {
                    definition: definition(ExtensionFrameworkId::Paddleocr),
                    python: PythonRuntime {
                        path: "python".to_string(),
                        version: "3.12.4".to_string(),
                    },
                },
                &mut |log| logs.push(log),
            )
            .expect("install");

        assert_eq!(installed.installed_version, "3.2.0");
        assert_eq!(
            std::fs::read_to_string(root.path().join("paddleocr").join(INSTALLED_MARKER))
                .expect("marker"),
            "3.2.0"
        );
        let requests = process.requests.lock().expect("requests");
        assert_eq!(requests[0].args[0..2], ["-m", "venv"]);
        assert_eq!(
            requests[1].args[0..4],
            ["-m", "pip", "install", "--disable-pip-version-check"]
        );
        assert!(requests[1].args.contains(&"paddleocr>=3,<4".to_string()));
        assert!(requests[1].args.contains(&"paddlepaddle>=3,<4".to_string()));
        assert!(logs.iter().any(|log| log.level == ExtensionLogLevel::Warn));
    }

    #[test]
    fn removal_and_marker_rollback_are_exactly_scoped_to_allowlisted_target() {
        let root = TempDirectory::new("extension-removal-adapter");
        let target = root.path().join("sherpa-onnx");
        std::fs::create_dir_all(&target).expect("target");
        std::fs::write(target.join(INSTALLED_MARKER), "1.12.0").expect("marker");
        let adapter = ManagedExtensionInstallation::with_dependencies(
            root.path().to_path_buf(),
            Arc::new(FakeProcess::new(Vec::new())),
        );

        adapter
            .rollback_installation(ExtensionFrameworkId::SherpaOnnx)
            .expect("rollback");
        assert!(!target.join(INSTALLED_MARKER).exists());
        adapter
            .remove(
                "extension-op-2",
                &RemovalPlan {
                    framework_id: ExtensionFrameworkId::SherpaOnnx,
                },
                &mut |_| {},
            )
            .expect("remove");
        assert!(!target.exists());
        assert!(
            ensure_owned_target(root.path(), root.path(), ExtensionFrameworkId::SherpaOnnx)
                .is_err()
        );
        assert!(ensure_owned_target(
            root.path(),
            &root.path().join("paddleocr"),
            ExtensionFrameworkId::SherpaOnnx
        )
        .is_err());
    }
}
