use super::{
    definition, ExtensionCapabilityId, ExtensionEnvironment, ExtensionFrameworkDefinition,
    ExtensionFrameworkId, PythonRuntime,
};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtensionLifecycleStatus {
    NotInstalled,
    Installing,
    Installed,
    Starting,
    Running,
    Stopping,
    Uninstalling,
    Error,
    Unsupported,
}

impl ExtensionLifecycleStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotInstalled => "not-installed",
            Self::Installing => "installing",
            Self::Installed => "installed",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Uninstalling => "uninstalling",
            Self::Error => "error",
            Self::Unsupported => "unsupported",
        }
    }

    pub(crate) fn parse(value: &str) -> Self {
        match value {
            "installing" => Self::Installing,
            "installed" => Self::Installed,
            "starting" => Self::Starting,
            "running" => Self::Running,
            "stopping" => Self::Stopping,
            "uninstalling" => Self::Uninstalling,
            "error" => Self::Error,
            "unsupported" => Self::Unsupported,
            _ => Self::NotInstalled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionFrameworkState {
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) capability_id: ExtensionCapabilityId,
    pub(crate) status: ExtensionLifecycleStatus,
    pub(crate) installed: bool,
    pub(crate) enabled: bool,
    pub(crate) port: u16,
    pub(crate) install_path: Option<String>,
    pub(crate) installed_version: Option<String>,
    pub(crate) last_health_check: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) last_operation_id: Option<String>,
}

impl ExtensionFrameworkState {
    pub(crate) fn seeded(definition: ExtensionFrameworkDefinition) -> Self {
        Self {
            framework_id: definition.id,
            capability_id: definition.capability_id,
            status: ExtensionLifecycleStatus::NotInstalled,
            installed: false,
            enabled: false,
            port: definition.default_port,
            install_path: None,
            installed_version: None,
            last_health_check: None,
            last_error: None,
            last_operation_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InstallationVerification {
    NotChecked,
    Passed { installed_version: Option<String> },
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionInstallationObservation {
    pub(crate) managed_directory_exists: bool,
    pub(crate) interpreter_exists: bool,
    pub(crate) marker_version: Option<String>,
    pub(crate) verification: InstallationVerification,
}

impl ExtensionInstallationObservation {
    pub(crate) fn absent() -> Self {
        Self {
            managed_directory_exists: false,
            interpreter_exists: false,
            marker_version: None,
            verification: InstallationVerification::NotChecked,
        }
    }

    pub(crate) fn installation_ready(&self) -> bool {
        self.managed_directory_exists && self.interpreter_exists && self.marker_version.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionRuntimeObservation {
    pub(crate) owned_process_running: bool,
    pub(crate) healthy: bool,
    pub(crate) error: Option<String>,
}

impl ExtensionRuntimeObservation {
    pub(crate) fn stopped() -> Self {
        Self {
            owned_process_running: false,
            healthy: false,
            error: None,
        }
    }

    pub(crate) fn healthy() -> Self {
        Self {
            owned_process_running: true,
            healthy: true,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExtensionInstallationDrift {
    ManagedFilesWithoutRegistry,
    MissingManagedDirectory,
    MissingInterpreter,
    MissingInstallationMarker,
    VersionMismatch { recorded: String, observed: String },
    VerificationFailed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionHealth {
    pub(crate) installation_ready: bool,
    pub(crate) installation_drift: Vec<ExtensionInstallationDrift>,
    pub(crate) runtime_healthy: bool,
    pub(crate) runtime_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionFrameworkStatus {
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) capability_id: ExtensionCapabilityId,
    pub(crate) status: ExtensionLifecycleStatus,
    pub(crate) installed: bool,
    pub(crate) enabled: bool,
    pub(crate) running: bool,
    pub(crate) port: u16,
    pub(crate) install_path: Option<String>,
    pub(crate) installed_version: Option<String>,
    pub(crate) last_health_check: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) last_operation_id: Option<String>,
    pub(crate) health: ExtensionHealth,
}

pub(crate) fn observe_status(
    state: ExtensionFrameworkState,
    environment: &ExtensionEnvironment,
    installation: &ExtensionInstallationObservation,
    runtime: &ExtensionRuntimeObservation,
) -> ExtensionFrameworkStatus {
    let installation_drift = installation_drift(&state, installation);
    let running = runtime.owned_process_running;
    let status = if running {
        ExtensionLifecycleStatus::Running
    } else if !environment.supported && !state.installed {
        ExtensionLifecycleStatus::Unsupported
    } else if state.status == ExtensionLifecycleStatus::Running {
        ExtensionLifecycleStatus::Installed
    } else {
        state.status
    };
    let last_error = if status == ExtensionLifecycleStatus::Unsupported {
        environment
            .reason_key()
            .map(str::to_string)
            .or(state.last_error)
    } else {
        state.last_error
    };
    ExtensionFrameworkStatus {
        framework_id: state.framework_id,
        capability_id: state.capability_id,
        status,
        installed: state.installed,
        enabled: state.enabled,
        running,
        port: state.port,
        install_path: state.install_path,
        installed_version: state.installed_version,
        last_health_check: state.last_health_check,
        last_error,
        last_operation_id: state.last_operation_id,
        health: ExtensionHealth {
            installation_ready: installation.installation_ready(),
            installation_drift,
            runtime_healthy: runtime.healthy,
            runtime_error: runtime.error.clone(),
        },
    }
}

fn installation_drift(
    state: &ExtensionFrameworkState,
    observation: &ExtensionInstallationObservation,
) -> Vec<ExtensionInstallationDrift> {
    if !state.installed {
        return observation
            .managed_directory_exists
            .then_some(ExtensionInstallationDrift::ManagedFilesWithoutRegistry)
            .into_iter()
            .collect();
    }
    if !observation.managed_directory_exists {
        return vec![ExtensionInstallationDrift::MissingManagedDirectory];
    }

    let mut drift = Vec::new();
    if !observation.interpreter_exists {
        drift.push(ExtensionInstallationDrift::MissingInterpreter);
    }
    match observation.marker_version.as_deref() {
        None => drift.push(ExtensionInstallationDrift::MissingInstallationMarker),
        Some(observed) => push_version_drift(&mut drift, state, observed),
    }
    match &observation.verification {
        InstallationVerification::NotChecked => {}
        InstallationVerification::Passed {
            installed_version: Some(observed),
        } => push_version_drift(&mut drift, state, observed),
        InstallationVerification::Passed {
            installed_version: None,
        } => {}
        InstallationVerification::Failed(error) => {
            drift.push(ExtensionInstallationDrift::VerificationFailed(
                error.clone(),
            ));
        }
    }
    drift
}

fn push_version_drift(
    drift: &mut Vec<ExtensionInstallationDrift>,
    state: &ExtensionFrameworkState,
    observed: &str,
) {
    let Some(recorded) = state.installed_version.as_deref() else {
        return;
    };
    if recorded != observed
        && !drift.iter().any(|item| {
            matches!(
                item,
                ExtensionInstallationDrift::VersionMismatch {
                    recorded: existing_recorded,
                    observed: existing_observed,
                } if existing_recorded == recorded && existing_observed == observed
            )
        })
    {
        drift.push(ExtensionInstallationDrift::VersionMismatch {
            recorded: recorded.to_string(),
            observed: observed.to_string(),
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtensionAction {
    Install,
    Uninstall,
    Enable,
    Disable,
    Start,
    Stop,
    SelfTest,
}

impl ExtensionAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Uninstall => "uninstall",
            Self::Enable => "enable",
            Self::Disable => "disable",
            Self::Start => "start",
            Self::Stop => "stop",
            Self::SelfTest => "self-test",
        }
    }

    pub(crate) fn task_message(self) -> &'static str {
        match self {
            Self::Install => "Install local extension",
            Self::Uninstall => "Uninstall local extension",
            Self::Enable => "Enable local extension",
            Self::Disable => "Disable local extension",
            Self::Start => "Start local extension",
            Self::Stop => "Stop local extension",
            Self::SelfTest => "SelfTest local extension",
        }
    }

    pub(crate) fn transition(self) -> ExtensionLifecycleStatus {
        match self {
            Self::Install => ExtensionLifecycleStatus::Installing,
            Self::Uninstall => ExtensionLifecycleStatus::Uninstalling,
            Self::Start => ExtensionLifecycleStatus::Starting,
            Self::Stop => ExtensionLifecycleStatus::Stopping,
            Self::Enable | Self::Disable | Self::SelfTest => ExtensionLifecycleStatus::Installed,
        }
    }

    pub(crate) fn success_message(self) -> &'static str {
        match self {
            Self::Install => "Framework installed",
            Self::Uninstall => "Framework uninstalled",
            Self::Enable => "Framework enabled",
            Self::Disable => "Framework disabled",
            Self::Start => "Framework started",
            Self::Stop => "Framework stopped",
            Self::SelfTest => "Runtime self-test passed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstallPlan {
    pub(crate) definition: ExtensionFrameworkDefinition,
    pub(crate) python: PythonRuntime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RemovalPlan {
    pub(crate) framework_id: ExtensionFrameworkId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EnablementPlan {
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) capability_id: ExtensionCapabilityId,
    pub(crate) enabled: bool,
    pub(crate) disable_capability_peers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimePlan {
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelfTestPlan {
    pub(crate) framework_id: ExtensionFrameworkId,
    pub(crate) import_module: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExtensionOperationPlan {
    Install(InstallPlan),
    Remove(RemovalPlan),
    Enablement(EnablementPlan),
    Start(RuntimePlan),
    Stop(RuntimePlan),
    SelfTest(SelfTestPlan),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExtensionDomainError {
    UnsupportedEnvironment(&'static str),
    EnableRequiresInstallation,
    StartRequiresInstallation,
    SelfTestRequiresInstallation,
    RemovalRequiresStoppedRuntime,
}

impl fmt::Display for ExtensionDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedEnvironment(reason) => formatter.write_str(reason),
            Self::EnableRequiresInstallation => {
                formatter.write_str("Framework must be installed before it can be enabled")
            }
            Self::StartRequiresInstallation => {
                formatter.write_str("Framework must be installed before starting")
            }
            Self::SelfTestRequiresInstallation => {
                formatter.write_str("Managed framework environment is not installed")
            }
            Self::RemovalRequiresStoppedRuntime => {
                formatter.write_str("Stop the framework before uninstalling")
            }
        }
    }
}

impl std::error::Error for ExtensionDomainError {}

pub(crate) fn plan_operation(
    action: ExtensionAction,
    status: &ExtensionFrameworkStatus,
    environment: &ExtensionEnvironment,
) -> Result<ExtensionOperationPlan, ExtensionDomainError> {
    let framework = definition(status.framework_id);
    match action {
        ExtensionAction::Install => {
            let python = supported_python(environment)?;
            Ok(ExtensionOperationPlan::Install(InstallPlan {
                definition: framework,
                python,
            }))
        }
        ExtensionAction::Uninstall if status.running => {
            Err(ExtensionDomainError::RemovalRequiresStoppedRuntime)
        }
        ExtensionAction::Uninstall => Ok(ExtensionOperationPlan::Remove(RemovalPlan {
            framework_id: status.framework_id,
        })),
        ExtensionAction::Enable if !status.installed => {
            Err(ExtensionDomainError::EnableRequiresInstallation)
        }
        ExtensionAction::Enable | ExtensionAction::Disable => {
            Ok(ExtensionOperationPlan::Enablement(EnablementPlan {
                framework_id: status.framework_id,
                capability_id: status.capability_id,
                enabled: action == ExtensionAction::Enable,
                disable_capability_peers: action == ExtensionAction::Enable,
            }))
        }
        ExtensionAction::Start if !status.installed => {
            Err(ExtensionDomainError::StartRequiresInstallation)
        }
        ExtensionAction::Start => {
            supported_python(environment)?;
            Ok(ExtensionOperationPlan::Start(RuntimePlan {
                framework_id: status.framework_id,
                port: status.port,
            }))
        }
        ExtensionAction::Stop => Ok(ExtensionOperationPlan::Stop(RuntimePlan {
            framework_id: status.framework_id,
            port: status.port,
        })),
        ExtensionAction::SelfTest if !status.health.installation_ready => {
            Err(ExtensionDomainError::SelfTestRequiresInstallation)
        }
        ExtensionAction::SelfTest => Ok(ExtensionOperationPlan::SelfTest(SelfTestPlan {
            framework_id: status.framework_id,
            import_module: framework.requirement.import_module,
        })),
    }
}

fn supported_python(
    environment: &ExtensionEnvironment,
) -> Result<PythonRuntime, ExtensionDomainError> {
    if !environment.supported {
        return Err(ExtensionDomainError::UnsupportedEnvironment(
            environment
                .reason_key()
                .unwrap_or("Unsupported environment"),
        ));
    }
    environment
        .python
        .clone()
        .ok_or(ExtensionDomainError::UnsupportedEnvironment(
            "extensions.environment.pythonMissing",
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::extensions::domain::HostEnvironment;

    fn supported_environment() -> ExtensionEnvironment {
        ExtensionEnvironment::evaluate(HostEnvironment {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            python: Some(PythonRuntime {
                path: "python".to_string(),
                version: "3.12.4".to_string(),
            }),
        })
    }

    fn status(
        installed: bool,
        enabled: bool,
        runtime: ExtensionRuntimeObservation,
        installation: ExtensionInstallationObservation,
    ) -> ExtensionFrameworkStatus {
        let mut state =
            ExtensionFrameworkState::seeded(definition(ExtensionFrameworkId::Paddleocr));
        state.installed = installed;
        state.enabled = enabled;
        state.status = if installed {
            ExtensionLifecycleStatus::Installed
        } else {
            ExtensionLifecycleStatus::NotInstalled
        };
        observe_status(state, &supported_environment(), &installation, &runtime)
    }

    fn installed_observation(version: &str) -> ExtensionInstallationObservation {
        ExtensionInstallationObservation {
            managed_directory_exists: true,
            interpreter_exists: true,
            marker_version: Some(version.to_string()),
            verification: InstallationVerification::Passed {
                installed_version: Some(version.to_string()),
            },
        }
    }

    #[test]
    fn observation_reconciles_runtime_and_unsupported_state_without_mutating_registry_facts() {
        let mut state =
            ExtensionFrameworkState::seeded(definition(ExtensionFrameworkId::Paddleocr));
        state.status = ExtensionLifecycleStatus::Running;
        state.installed = true;
        let stopped = observe_status(
            state,
            &supported_environment(),
            &installed_observation("3.2.0"),
            &ExtensionRuntimeObservation::stopped(),
        );
        assert_eq!(stopped.status, ExtensionLifecycleStatus::Installed);
        assert!(!stopped.running);
        assert!(stopped.installed);

        let unsupported = ExtensionEnvironment::evaluate(HostEnvironment {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            python: None,
        });
        let status = observe_status(
            ExtensionFrameworkState::seeded(definition(ExtensionFrameworkId::Paddleocr)),
            &unsupported,
            &ExtensionInstallationObservation::absent(),
            &ExtensionRuntimeObservation::stopped(),
        );
        assert_eq!(status.status, ExtensionLifecycleStatus::Unsupported);
        assert_eq!(
            status.last_error.as_deref(),
            Some("extensions.environment.windowsX64Only")
        );
    }

    #[test]
    fn installation_drift_distinguishes_orphans_missing_files_and_version_changes() {
        let orphan = status(
            false,
            false,
            ExtensionRuntimeObservation::stopped(),
            installed_observation("3.2.0"),
        );
        assert_eq!(
            orphan.health.installation_drift,
            vec![ExtensionInstallationDrift::ManagedFilesWithoutRegistry]
        );

        let mut state =
            ExtensionFrameworkState::seeded(definition(ExtensionFrameworkId::Paddleocr));
        state.installed = true;
        state.installed_version = Some("3.1.0".to_string());
        let changed = observe_status(
            state,
            &supported_environment(),
            &installed_observation("3.2.0"),
            &ExtensionRuntimeObservation::stopped(),
        );
        assert_eq!(
            changed.health.installation_drift,
            vec![ExtensionInstallationDrift::VersionMismatch {
                recorded: "3.1.0".to_string(),
                observed: "3.2.0".to_string()
            }]
        );

        let missing = status(
            true,
            false,
            ExtensionRuntimeObservation::stopped(),
            ExtensionInstallationObservation::absent(),
        );
        assert_eq!(
            missing.health.installation_drift,
            vec![ExtensionInstallationDrift::MissingManagedDirectory]
        );
    }

    #[test]
    fn enablement_is_installation_gated_and_exclusive_within_capability() {
        let not_installed = status(
            false,
            false,
            ExtensionRuntimeObservation::stopped(),
            ExtensionInstallationObservation::absent(),
        );
        assert_eq!(
            plan_operation(
                ExtensionAction::Enable,
                &not_installed,
                &supported_environment()
            ),
            Err(ExtensionDomainError::EnableRequiresInstallation)
        );

        let installed = status(
            true,
            false,
            ExtensionRuntimeObservation::stopped(),
            installed_observation("3.2.0"),
        );
        let ExtensionOperationPlan::Enablement(plan) = plan_operation(
            ExtensionAction::Enable,
            &installed,
            &supported_environment(),
        )
        .expect("enablement plan") else {
            panic!("expected enablement plan");
        };
        assert!(plan.enabled);
        assert!(plan.disable_capability_peers);
        assert_eq!(plan.capability_id, ExtensionCapabilityId::Ocr);
    }

    #[test]
    fn install_start_self_test_and_removal_rules_prevent_unsafe_execution() {
        let not_installed = status(
            false,
            false,
            ExtensionRuntimeObservation::stopped(),
            ExtensionInstallationObservation::absent(),
        );
        assert_eq!(
            plan_operation(
                ExtensionAction::Start,
                &not_installed,
                &supported_environment()
            ),
            Err(ExtensionDomainError::StartRequiresInstallation)
        );
        assert_eq!(
            plan_operation(
                ExtensionAction::SelfTest,
                &not_installed,
                &supported_environment()
            ),
            Err(ExtensionDomainError::SelfTestRequiresInstallation)
        );

        let running = status(
            true,
            true,
            ExtensionRuntimeObservation::healthy(),
            installed_observation("3.2.0"),
        );
        assert_eq!(
            plan_operation(
                ExtensionAction::Uninstall,
                &running,
                &supported_environment()
            ),
            Err(ExtensionDomainError::RemovalRequiresStoppedRuntime)
        );

        let unsupported = ExtensionEnvironment::evaluate(HostEnvironment {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            python: None,
        });
        assert_eq!(
            plan_operation(ExtensionAction::Install, &not_installed, &unsupported),
            Err(ExtensionDomainError::UnsupportedEnvironment(
                "extensions.environment.windowsX64Only"
            ))
        );
    }
}
