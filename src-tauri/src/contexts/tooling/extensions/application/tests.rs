use super::*;
use crate::contexts::tooling::extensions::domain::{
    definitions, EnablementPlan, ExtensionAction, ExtensionFrameworkId, ExtensionFrameworkState,
    ExtensionInstallationDrift, ExtensionInstallationObservation, ExtensionLifecycleStatus,
    ExtensionRuntimeObservation, HostEnvironment, InstallPlan, InstallationVerification,
    PythonRuntime, RemovalPlan, RuntimePlan, SelfTestPlan,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum RepositoryEvent {
    Transition(ExtensionFrameworkId, ExtensionLifecycleStatus, String),
    Installed(ExtensionFrameworkId, String),
    Removed(ExtensionFrameworkId),
    Enablement(EnablementPlan),
    Runtime(ExtensionFrameworkId, bool),
    SelfTest(ExtensionFrameworkId),
    Failed(ExtensionFrameworkId, String),
}

struct FakeRepository {
    states: Mutex<BTreeMap<ExtensionFrameworkId, ExtensionFrameworkState>>,
    events: Mutex<Vec<RepositoryEvent>>,
    installation_failure: Mutex<Option<String>>,
}

impl Default for FakeRepository {
    fn default() -> Self {
        Self {
            states: Mutex::new(
                definitions()
                    .iter()
                    .map(|item| (item.id, ExtensionFrameworkState::seeded(*item)))
                    .collect(),
            ),
            events: Mutex::new(Vec::new()),
            installation_failure: Mutex::new(None),
        }
    }
}

impl ExtensionRepository for FakeRepository {
    fn list_states(&self) -> Result<Vec<ExtensionFrameworkState>, ExtensionApplicationError> {
        Ok(self
            .states
            .lock()
            .expect("states")
            .values()
            .cloned()
            .collect())
    }

    fn record_transition(
        &self,
        framework_id: ExtensionFrameworkId,
        status: ExtensionLifecycleStatus,
        operation_id: &str,
        _at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        let mut states = self.states.lock().expect("states");
        let state = states.get_mut(&framework_id).expect("state");
        state.status = status;
        state.last_operation_id = Some(operation_id.to_string());
        self.events
            .lock()
            .expect("events")
            .push(RepositoryEvent::Transition(
                framework_id,
                status,
                operation_id.to_string(),
            ));
        Ok(())
    }

    fn record_installation(
        &self,
        framework_id: ExtensionFrameworkId,
        installed: &InstalledExtension,
        _at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        if let Some(error) = self
            .installation_failure
            .lock()
            .expect("installation failure")
            .clone()
        {
            return Err(ExtensionApplicationError::Repository(error));
        }
        let mut states = self.states.lock().expect("states");
        let state = states.get_mut(&framework_id).expect("state");
        state.status = ExtensionLifecycleStatus::Installed;
        state.installed = true;
        state.enabled = false;
        state.install_path = Some(installed.install_path.clone());
        state.installed_version = Some(installed.installed_version.clone());
        state.last_error = None;
        self.events
            .lock()
            .expect("events")
            .push(RepositoryEvent::Installed(
                framework_id,
                installed.installed_version.clone(),
            ));
        Ok(())
    }

    fn record_removal(
        &self,
        framework_id: ExtensionFrameworkId,
        _at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        let mut states = self.states.lock().expect("states");
        let state = states.get_mut(&framework_id).expect("state");
        state.status = ExtensionLifecycleStatus::NotInstalled;
        state.installed = false;
        state.enabled = false;
        state.install_path = None;
        state.installed_version = None;
        state.last_health_check = None;
        state.last_error = None;
        self.events
            .lock()
            .expect("events")
            .push(RepositoryEvent::Removed(framework_id));
        Ok(())
    }

    fn apply_enablement(
        &self,
        plan: &EnablementPlan,
        _at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        let mut states = self.states.lock().expect("states");
        if plan.disable_capability_peers {
            for state in states.values_mut() {
                if state.capability_id == plan.capability_id {
                    state.enabled = false;
                }
            }
        }
        states.get_mut(&plan.framework_id).expect("state").enabled = plan.enabled;
        self.events
            .lock()
            .expect("events")
            .push(RepositoryEvent::Enablement(*plan));
        Ok(())
    }

    fn record_runtime_observation(
        &self,
        framework_id: ExtensionFrameworkId,
        observation: &ExtensionRuntimeObservation,
        checked_at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        let mut states = self.states.lock().expect("states");
        let state = states.get_mut(&framework_id).expect("state");
        state.status = if observation.owned_process_running {
            ExtensionLifecycleStatus::Running
        } else if state.installed {
            ExtensionLifecycleStatus::Installed
        } else {
            ExtensionLifecycleStatus::NotInstalled
        };
        state.last_health_check = Some(checked_at.to_string());
        state.last_error = observation.error.clone();
        self.events
            .lock()
            .expect("events")
            .push(RepositoryEvent::Runtime(
                framework_id,
                observation.owned_process_running,
            ));
        Ok(())
    }

    fn record_self_test(
        &self,
        framework_id: ExtensionFrameworkId,
        checked_at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        let mut states = self.states.lock().expect("states");
        let state = states.get_mut(&framework_id).expect("state");
        state.last_health_check = Some(checked_at.to_string());
        state.last_error = None;
        self.events
            .lock()
            .expect("events")
            .push(RepositoryEvent::SelfTest(framework_id));
        Ok(())
    }

    fn record_failure(
        &self,
        framework_id: ExtensionFrameworkId,
        error: &str,
        _at: &str,
    ) -> Result<(), ExtensionApplicationError> {
        let mut states = self.states.lock().expect("states");
        let state = states.get_mut(&framework_id).expect("state");
        state.status = ExtensionLifecycleStatus::Error;
        state.last_error = Some(error.to_string());
        self.events
            .lock()
            .expect("events")
            .push(RepositoryEvent::Failed(framework_id, error.to_string()));
        Ok(())
    }
}

struct FakeEnvironment;

impl ExtensionEnvironmentPort for FakeEnvironment {
    fn observe_host(&self) -> Result<HostEnvironment, ExtensionApplicationError> {
        Ok(HostEnvironment {
            os: "windows".to_string(),
            arch: "x86_64".to_string(),
            python: Some(PythonRuntime {
                path: "C:/Python312/python.exe".to_string(),
                version: "3.12.4".to_string(),
            }),
        })
    }
}

#[derive(Default)]
struct FakeInstallation {
    observations: Mutex<BTreeMap<ExtensionFrameworkId, ExtensionInstallationObservation>>,
    inspections: Mutex<Vec<(ExtensionFrameworkId, InstallationInspection)>>,
    executions: Mutex<Vec<String>>,
}

impl FakeInstallation {
    fn observation(&self, id: ExtensionFrameworkId, version: &str) {
        self.observations.lock().expect("observations").insert(
            id,
            ExtensionInstallationObservation {
                managed_directory_exists: true,
                interpreter_exists: true,
                marker_version: Some(version.to_string()),
                verification: InstallationVerification::NotChecked,
            },
        );
    }
}

impl ExtensionInstallationPort for FakeInstallation {
    fn managed_path(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<String, ExtensionApplicationError> {
        Ok(format!("C:/VaneHub/extensions/{}", framework_id.as_str()))
    }

    fn inspect(
        &self,
        framework_id: ExtensionFrameworkId,
        inspection: InstallationInspection,
    ) -> Result<ExtensionInstallationObservation, ExtensionApplicationError> {
        self.inspections
            .lock()
            .expect("inspections")
            .push((framework_id, inspection));
        Ok(self
            .observations
            .lock()
            .expect("observations")
            .get(&framework_id)
            .cloned()
            .unwrap_or_else(ExtensionInstallationObservation::absent))
    }

    fn install(
        &self,
        _operation_id: &str,
        plan: &InstallPlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<InstalledExtension, ExtensionApplicationError> {
        self.executions.lock().expect("executions").push(format!(
            "install:{}:{}",
            plan.definition.id.as_str(),
            plan.definition.requirement.packages.join(",")
        ));
        emit(ExtensionExecutionLog::info(
            "Framework installation verified",
        ));
        Ok(InstalledExtension {
            install_path: format!("C:/VaneHub/extensions/{}", plan.definition.id.as_str()),
            installed_version: "3.2.0".to_string(),
        })
    }

    fn rollback_installation(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<(), ExtensionApplicationError> {
        self.executions
            .lock()
            .expect("executions")
            .push(format!("rollback:{}", framework_id.as_str()));
        Ok(())
    }

    fn remove(
        &self,
        _operation_id: &str,
        plan: &RemovalPlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<(), ExtensionApplicationError> {
        self.executions
            .lock()
            .expect("executions")
            .push(format!("remove:{}", plan.framework_id.as_str()));
        emit(ExtensionExecutionLog::info("Managed directory removed"));
        Ok(())
    }

    fn self_test(
        &self,
        _operation_id: &str,
        plan: &SelfTestPlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<(), ExtensionApplicationError> {
        self.executions.lock().expect("executions").push(format!(
            "test:{}:{}",
            plan.framework_id.as_str(),
            plan.import_module
        ));
        emit(ExtensionExecutionLog::info("self-test-ok"));
        Ok(())
    }
}

#[derive(Default)]
struct FakeRuntime {
    observations: Mutex<BTreeMap<ExtensionFrameworkId, ExtensionRuntimeObservation>>,
    starts: Mutex<Vec<RuntimePlan>>,
    stops: Mutex<Vec<RuntimePlan>>,
}

impl ExtensionRuntimePort for FakeRuntime {
    fn observe(
        &self,
        framework_id: ExtensionFrameworkId,
        _port: u16,
    ) -> Result<ExtensionRuntimeObservation, ExtensionApplicationError> {
        Ok(self
            .observations
            .lock()
            .expect("observations")
            .get(&framework_id)
            .cloned()
            .unwrap_or_else(ExtensionRuntimeObservation::stopped))
    }

    fn start(
        &self,
        _operation_id: &str,
        plan: &RuntimePlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<ExtensionRuntimeObservation, ExtensionApplicationError> {
        self.starts.lock().expect("starts").push(*plan);
        emit(ExtensionExecutionLog::info("Started owned sidecar"));
        Ok(ExtensionRuntimeObservation::healthy())
    }

    fn stop(
        &self,
        _operation_id: &str,
        plan: &RuntimePlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<ExtensionRuntimeObservation, ExtensionApplicationError> {
        self.stops.lock().expect("stops").push(*plan);
        emit(ExtensionExecutionLog::info("Stopped owned sidecar"));
        Ok(ExtensionRuntimeObservation::stopped())
    }
}

#[derive(Default)]
struct FakeMutations {
    active: Mutex<BTreeSet<ExtensionFrameworkId>>,
    finished: Mutex<Vec<ExtensionFrameworkId>>,
}

impl ExtensionMutationPort for FakeMutations {
    fn begin(&self, framework_id: ExtensionFrameworkId) -> Result<(), ExtensionApplicationError> {
        if !self.active.lock().expect("active").insert(framework_id) {
            return Err(ExtensionApplicationError::ConcurrentMutation(format!(
                "an extension operation is already running for {}",
                framework_id.as_str()
            )));
        }
        Ok(())
    }

    fn finish(&self, framework_id: ExtensionFrameworkId) {
        self.active.lock().expect("active").remove(&framework_id);
        self.finished.lock().expect("finished").push(framework_id);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OperationEvent {
    Started(ExtensionFrameworkId, ExtensionAction),
    Logged(ExtensionLogEvent),
    Completed(ExtensionOperationResult),
    Failed(String, String),
}

#[derive(Default)]
struct FakeOperations {
    events: Mutex<Vec<OperationEvent>>,
}

impl ExtensionOperationPort for FakeOperations {
    fn start(
        &self,
        framework_id: ExtensionFrameworkId,
        action: ExtensionAction,
        message: String,
    ) -> Result<StartedExtensionOperation, ExtensionApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Started(framework_id, action));
        Ok(StartedExtensionOperation {
            id: format!("extension-op-{}", framework_id.as_str()),
            related_entity_id: Some(framework_id.as_str().to_string()),
            message: Some(message),
            created_at: "created".to_string(),
            updated_at: "created".to_string(),
        })
    }

    fn append_log(&self, event: &ExtensionLogEvent) -> Result<(), ExtensionApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Logged(event.clone()));
        Ok(())
    }

    fn complete(&self, result: &ExtensionOperationResult) -> Result<(), ExtensionApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Completed(result.clone()));
        Ok(())
    }

    fn fail(&self, operation_id: &str, error: String) -> Result<(), ExtensionApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Failed(operation_id.to_string(), error));
        Ok(())
    }
}

#[derive(Default)]
struct FakeLogging {
    events: Mutex<Vec<ExtensionLogEvent>>,
}

impl ExtensionLoggingPort for FakeLogging {
    fn record(&self, event: &ExtensionLogEvent) -> Result<(), ExtensionApplicationError> {
        self.events.lock().expect("events").push(event.clone());
        Ok(())
    }
}

struct FakeClock;

impl ExtensionClockPort for FakeClock {
    fn now(&self) -> String {
        "2026-07-18T12:00:00Z".to_string()
    }
}

struct Fixture {
    service: ExtensionApplicationService,
    repository: Arc<FakeRepository>,
    installation: Arc<FakeInstallation>,
    runtime: Arc<FakeRuntime>,
    mutations: Arc<FakeMutations>,
    operations: Arc<FakeOperations>,
    logging: Arc<FakeLogging>,
}

impl Fixture {
    fn new() -> Self {
        let repository = Arc::new(FakeRepository::default());
        let installation = Arc::new(FakeInstallation::default());
        let runtime = Arc::new(FakeRuntime::default());
        let mutations = Arc::new(FakeMutations::default());
        let operations = Arc::new(FakeOperations::default());
        let logging = Arc::new(FakeLogging::default());
        let service = ExtensionApplicationService::new(
            repository.clone(),
            Arc::new(FakeEnvironment),
            installation.clone(),
            runtime.clone(),
            mutations.clone(),
            operations.clone(),
            logging.clone(),
            Arc::new(FakeClock),
        );
        Self {
            service,
            repository,
            installation,
            runtime,
            mutations,
            operations,
            logging,
        }
    }

    fn mark_installed(&self, id: ExtensionFrameworkId, version: &str) {
        let mut states = self.repository.states.lock().expect("states");
        let state = states.get_mut(&id).expect("state");
        state.installed = true;
        state.status = ExtensionLifecycleStatus::Installed;
        state.installed_version = Some(version.to_string());
        state.install_path = Some(format!("C:/VaneHub/extensions/{}", id.as_str()));
        drop(states);
        self.installation.observation(id, version);
    }

    fn prepare(
        &self,
        id: ExtensionFrameworkId,
        action: ExtensionAction,
    ) -> PreparedExtensionOperation {
        self.service
            .prepare_operation(ExtensionOperationRequest {
                framework_id: id,
                action,
            })
            .expect("prepare")
    }
}

#[test]
fn overview_and_preview_use_catalog_environment_and_observation_ports() {
    let fixture = Fixture::new();
    fixture.mark_installed(ExtensionFrameworkId::Paddleocr, "3.1.0");
    fixture
        .installation
        .observation(ExtensionFrameworkId::Paddleocr, "3.2.0");
    fixture
        .repository
        .states
        .lock()
        .expect("states")
        .get_mut(&ExtensionFrameworkId::Paddleocr)
        .expect("state")
        .status = ExtensionLifecycleStatus::Running;

    let overview = fixture.service.overview().expect("overview");
    assert_eq!(overview.definitions.len(), 3);
    assert_eq!(overview.environment.runtime, "tauri");
    let paddleocr = &overview.statuses[0];
    assert_eq!(paddleocr.status, ExtensionLifecycleStatus::Installed);
    assert_eq!(
        paddleocr.health.installation_drift,
        vec![ExtensionInstallationDrift::VersionMismatch {
            recorded: "3.1.0".to_string(),
            observed: "3.2.0".to_string()
        }]
    );

    let preview = fixture
        .service
        .install_preview(ExtensionFrameworkId::Paddleocr)
        .expect("preview");
    assert!(preview.supported);
    assert_eq!(
        preview.packages,
        vec!["paddleocr>=3,<4", "paddlepaddle>=3,<4"]
    );
    assert_eq!(preview.install_path, "C:/VaneHub/extensions/paddleocr");
    assert!(fixture.operations.events.lock().expect("events").is_empty());
}

#[test]
fn invalid_enablement_fails_task_before_transition_or_external_execution() {
    let fixture = Fixture::new();
    let prepared = fixture.prepare(ExtensionFrameworkId::Paddleocr, ExtensionAction::Enable);
    fixture
        .service
        .execute_operation(prepared)
        .expect("terminal failure recorded");

    let repository_events = fixture.repository.events.lock().expect("events");
    assert!(!repository_events
        .iter()
        .any(|event| matches!(event, RepositoryEvent::Transition(..))));
    assert!(!repository_events
        .iter()
        .any(|event| matches!(event, RepositoryEvent::Enablement(_))));
    assert!(repository_events.iter().any(|event| {
        matches!(event, RepositoryEvent::Failed(ExtensionFrameworkId::Paddleocr, error)
            if error == "Framework must be installed before it can be enabled")
    }));
    assert!(fixture
        .installation
        .executions
        .lock()
        .expect("executions")
        .is_empty());
    assert!(fixture
        .operations
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| matches!(event, OperationEvent::Failed(_, error)
            if error == "Framework must be installed before it can be enabled")));
    assert!(fixture.mutations.active.lock().expect("active").is_empty());
}

#[test]
fn installation_uses_allowlisted_plan_and_persists_disabled_terminal_state() {
    let fixture = Fixture::new();
    let prepared = fixture.prepare(ExtensionFrameworkId::Paddleocr, ExtensionAction::Install);
    fixture
        .service
        .execute_operation(prepared)
        .expect("execute");

    assert_eq!(
        fixture
            .installation
            .executions
            .lock()
            .expect("executions")
            .as_slice(),
        ["install:paddleocr:paddleocr>=3,<4,paddlepaddle>=3,<4"]
    );
    let states = fixture.repository.states.lock().expect("states");
    let state = states.get(&ExtensionFrameworkId::Paddleocr).expect("state");
    assert!(state.installed);
    assert!(!state.enabled);
    assert_eq!(state.installed_version.as_deref(), Some("3.2.0"));
    drop(states);

    let operation_events = fixture.operations.events.lock().expect("events");
    let completed = operation_events
        .iter()
        .find_map(|event| match event {
            OperationEvent::Completed(result) => Some(result),
            _ => None,
        })
        .expect("completed");
    assert_eq!(completed.message, "Framework installed");
    assert_eq!(completed.logs, vec!["Framework installation verified"]);
    assert_eq!(fixture.logging.events.lock().expect("logs").len(), 1);
}

#[test]
fn installation_marker_is_rolled_back_when_registry_commit_fails() {
    let fixture = Fixture::new();
    *fixture
        .repository
        .installation_failure
        .lock()
        .expect("failure") = Some("sqlite commit failed".to_string());
    let prepared = fixture.prepare(ExtensionFrameworkId::Paddleocr, ExtensionAction::Install);
    fixture
        .service
        .execute_operation(prepared)
        .expect("terminal failure recorded");

    assert_eq!(
        fixture
            .installation
            .executions
            .lock()
            .expect("executions")
            .as_slice(),
        [
            "install:paddleocr:paddleocr>=3,<4,paddlepaddle>=3,<4",
            "rollback:paddleocr"
        ]
    );
    let state = fixture
        .repository
        .states
        .lock()
        .expect("states")
        .get(&ExtensionFrameworkId::Paddleocr)
        .expect("state")
        .clone();
    assert!(!state.installed);
    assert_eq!(state.status, ExtensionLifecycleStatus::Error);
}

#[test]
fn running_framework_cannot_be_removed_and_the_remove_adapter_is_not_called() {
    let fixture = Fixture::new();
    fixture.mark_installed(ExtensionFrameworkId::FasterWhisper, "1.1.0");
    fixture
        .runtime
        .observations
        .lock()
        .expect("runtime")
        .insert(
            ExtensionFrameworkId::FasterWhisper,
            ExtensionRuntimeObservation::healthy(),
        );
    let prepared = fixture.prepare(
        ExtensionFrameworkId::FasterWhisper,
        ExtensionAction::Uninstall,
    );
    fixture
        .service
        .execute_operation(prepared)
        .expect("terminal failure recorded");

    assert!(fixture
        .installation
        .executions
        .lock()
        .expect("executions")
        .is_empty());
    assert!(fixture
        .operations
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| matches!(event, OperationEvent::Failed(_, error)
            if error == "Stop the framework before uninstalling")));
}

#[test]
fn enablement_and_stopped_removal_apply_only_the_domain_plans() {
    let fixture = Fixture::new();
    fixture.mark_installed(ExtensionFrameworkId::Paddleocr, "3.2.0");
    let enable = fixture.prepare(ExtensionFrameworkId::Paddleocr, ExtensionAction::Enable);
    fixture.service.execute_operation(enable).expect("enable");
    assert!(
        fixture
            .repository
            .states
            .lock()
            .expect("states")
            .get(&ExtensionFrameworkId::Paddleocr)
            .expect("state")
            .enabled
    );
    assert!(fixture
        .repository
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| matches!(event, RepositoryEvent::Enablement(plan)
            if plan.framework_id == ExtensionFrameworkId::Paddleocr
                && plan.disable_capability_peers)));

    let remove = fixture.prepare(ExtensionFrameworkId::Paddleocr, ExtensionAction::Uninstall);
    fixture.service.execute_operation(remove).expect("remove");
    assert_eq!(
        fixture
            .installation
            .executions
            .lock()
            .expect("executions")
            .last()
            .map(String::as_str),
        Some("remove:paddleocr")
    );
    let states = fixture.repository.states.lock().expect("states");
    let removed = states.get(&ExtensionFrameworkId::Paddleocr).expect("state");
    assert!(!removed.installed);
    assert!(!removed.enabled);
    assert!(removed.install_path.is_none());
}

#[test]
fn health_refresh_verifies_installations_and_records_observations_without_starting_runtime() {
    let fixture = Fixture::new();
    fixture.mark_installed(ExtensionFrameworkId::SherpaOnnx, "1.12.0");
    fixture
        .runtime
        .observations
        .lock()
        .expect("runtime")
        .insert(
            ExtensionFrameworkId::SherpaOnnx,
            ExtensionRuntimeObservation::healthy(),
        );
    fixture.service.refresh_health().expect("health refresh");

    assert!(fixture
        .installation
        .inspections
        .lock()
        .expect("inspections")
        .iter()
        .all(|(_, inspection)| *inspection == InstallationInspection::VerifyImport));
    assert!(fixture.runtime.starts.lock().expect("starts").is_empty());
    assert!(fixture.runtime.stops.lock().expect("stops").is_empty());
    assert_eq!(
        fixture
            .repository
            .events
            .lock()
            .expect("events")
            .iter()
            .filter(|event| matches!(event, RepositoryEvent::Runtime(_, _)))
            .count(),
        1
    );
}

#[test]
fn concurrent_mutation_is_rejected_for_only_the_affected_framework() {
    let fixture = Fixture::new();
    let first = fixture.prepare(ExtensionFrameworkId::Paddleocr, ExtensionAction::Install);
    let error = fixture
        .service
        .prepare_operation(ExtensionOperationRequest {
            framework_id: ExtensionFrameworkId::Paddleocr,
            action: ExtensionAction::Install,
        })
        .expect_err("concurrent mutation");
    assert!(error
        .to_string()
        .contains("an extension operation is already running for paddleocr"));
    assert!(fixture
        .service
        .prepare_operation(ExtensionOperationRequest {
            framework_id: ExtensionFrameworkId::SherpaOnnx,
            action: ExtensionAction::Install,
        })
        .is_ok());

    fixture.mutations.finish(first.request.framework_id);
}
