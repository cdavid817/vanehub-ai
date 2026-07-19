use super::*;
use crate::contexts::tooling::cli::domain::{
    definition, EnvironmentType, InstallSource, Installation, LifecycleEligibility, MutationClaims,
    ToolDefinition, VersionCheckStatus,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeRepository {
    statuses: Mutex<BTreeMap<String, CliToolStatus>>,
    saved: Mutex<Vec<CliToolStatus>>,
    has_cached_statuses: AtomicBool,
    save_failures: Mutex<BTreeSet<String>>,
}

impl CliStatusRepository for FakeRepository {
    fn load(&self, definition: ToolDefinition) -> Result<CliToolStatus, CliApplicationError> {
        Ok(self
            .statuses
            .lock()
            .expect("statuses")
            .get(definition.agent_id)
            .cloned()
            .unwrap_or_else(|| unknown_status(definition)))
    }

    fn save(&self, status: &CliToolStatus) -> Result<(), CliApplicationError> {
        if self
            .save_failures
            .lock()
            .expect("save failures")
            .contains(&status.agent_id)
        {
            return Err(CliApplicationError::Database(format!(
                "cannot save {}",
                status.agent_id
            )));
        }
        self.saved.lock().expect("saved").push(status.clone());
        self.statuses
            .lock()
            .expect("statuses")
            .insert(status.agent_id.clone(), status.clone());
        Ok(())
    }

    fn has_cached_statuses(&self) -> Result<bool, CliApplicationError> {
        Ok(self.has_cached_statuses.load(Ordering::SeqCst))
    }
}

#[derive(Default)]
struct FakeDetection {
    failures: Mutex<BTreeSet<String>>,
    warnings: Mutex<BTreeMap<String, Vec<String>>>,
}

impl CliDetectionPort for FakeDetection {
    fn detect(
        &self,
        definition: ToolDefinition,
        _operation_id: &str,
    ) -> Result<CliDetectionResult, CliApplicationError> {
        if self
            .failures
            .lock()
            .expect("failures")
            .contains(definition.agent_id)
        {
            return Err(CliApplicationError::Detection("probe failed".to_string()));
        }
        let mut status = eligible_status(definition.agent_id, "1.0.0", "2.0.0");
        status.last_checked_at = None;
        status.last_operation_id = None;
        Ok(CliDetectionResult {
            status,
            warnings: self
                .warnings
                .lock()
                .expect("warnings")
                .get(definition.agent_id)
                .cloned()
                .unwrap_or_default(),
            events: Vec::new(),
        })
    }
}

#[derive(Default)]
struct FakePackages {
    validation_failures: Mutex<BTreeSet<String>>,
    execution_failures: Mutex<BTreeSet<String>>,
    validations: Mutex<Vec<String>>,
    executions: Mutex<Vec<(String, String)>>,
}

impl CliPackagePort for FakePackages {
    fn validate(
        &self,
        definition: ToolDefinition,
        _status: &CliToolStatus,
        _confirmed_active_path: Option<&str>,
    ) -> Result<(), CliApplicationError> {
        self.validations
            .lock()
            .expect("validations")
            .push(definition.agent_id.to_string());
        if self
            .validation_failures
            .lock()
            .expect("validation failures")
            .contains(definition.agent_id)
        {
            return Err(CliApplicationError::Validation(
                "the active CLI path changed".to_string(),
            ));
        }
        Ok(())
    }

    fn execute(
        &self,
        _operation_id: &str,
        definition: ToolDefinition,
        _status: &CliToolStatus,
        target_version: &str,
        _emit: &mut dyn FnMut(CliLogEvent),
    ) -> Result<(), CliApplicationError> {
        self.executions
            .lock()
            .expect("executions")
            .push((definition.agent_id.to_string(), target_version.to_string()));
        if self
            .execution_failures
            .lock()
            .expect("execution failures")
            .contains(definition.agent_id)
        {
            return Err(CliApplicationError::Package(
                "package manager failed".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OperationEvent {
    Started(CliOperationRequest),
    Logged(CliLogEvent),
    Completed(CliOperationResult),
    Failed(String),
}

#[derive(Default)]
struct FakeOperations {
    next_id: AtomicUsize,
    fail_start: AtomicBool,
    events: Mutex<Vec<OperationEvent>>,
}

impl CliOperationPort for FakeOperations {
    fn start(
        &self,
        request: &CliOperationRequest,
    ) -> Result<StartedCliOperation, CliApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Started(request.clone()));
        if self.fail_start.load(Ordering::SeqCst) {
            return Err(CliApplicationError::Operation(
                "operation start failed".to_string(),
            ));
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(StartedCliOperation {
            id: format!("op-{id}"),
            related_entity_id: request.related_agent_id.clone(),
            message: Some(request.message.clone()),
            created_at: "created".to_string(),
            updated_at: "created".to_string(),
        })
    }

    fn append_log(&self, event: &CliLogEvent) -> Result<(), CliApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Logged(event.clone()));
        Ok(())
    }

    fn complete(
        &self,
        _operation_id: &str,
        result: &CliOperationResult,
    ) -> Result<(), CliApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Completed(result.clone()));
        Ok(())
    }

    fn fail(&self, _operation_id: &str, error: String) -> Result<(), CliApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Failed(error));
        Ok(())
    }
}

#[derive(Default)]
struct FakeLogging {
    events: Mutex<Vec<CliLogEvent>>,
}

impl CliLoggingPort for FakeLogging {
    fn record(&self, event: &CliLogEvent) -> Result<(), CliApplicationError> {
        self.events.lock().expect("events").push(event.clone());
        Ok(())
    }
}

struct FakeClock;

impl CliClockPort for FakeClock {
    fn now(&self) -> String {
        "2026-07-18T12:00:00Z".to_string()
    }
}

#[derive(Default)]
struct FakeExecutableLocator {
    resolved: Mutex<Option<String>>,
    requests: Mutex<Vec<(String, Option<String>)>>,
}

impl CliExecutableLocatorPort for FakeExecutableLocator {
    fn resolve(&self, definition: ToolDefinition, cached_path: Option<&str>) -> Option<String> {
        self.requests.lock().expect("requests").push((
            definition.agent_id.to_string(),
            cached_path.map(str::to_string),
        ));
        self.resolved.lock().expect("resolved").clone()
    }
}

#[derive(Default)]
struct FakeMutations {
    claims: Mutex<MutationClaims>,
    released: Mutex<Vec<String>>,
}

impl CliMutationPort for FakeMutations {
    fn try_acquire(&self, agent_id: &str) -> Result<bool, CliApplicationError> {
        Ok(self.claims.lock().expect("claims").try_acquire(agent_id))
    }

    fn release(&self, agent_id: &str) -> Result<(), CliApplicationError> {
        self.claims.lock().expect("claims").release(agent_id);
        self.released
            .lock()
            .expect("released")
            .push(agent_id.to_string());
        Ok(())
    }

    fn try_acquire_many(&self, agent_ids: &[String]) -> Result<Vec<String>, CliApplicationError> {
        Ok(self
            .claims
            .lock()
            .expect("claims")
            .try_acquire_many(agent_ids.iter().map(String::as_str)))
    }

    fn release_many(&self, agent_ids: &[String]) -> Result<(), CliApplicationError> {
        self.claims
            .lock()
            .expect("claims")
            .release_many(agent_ids.iter().map(String::as_str));
        self.released
            .lock()
            .expect("released")
            .extend(agent_ids.iter().cloned());
        Ok(())
    }
}

struct Fixture {
    service: CliApplicationService,
    repository: Arc<FakeRepository>,
    detection: Arc<FakeDetection>,
    executable_locator: Arc<FakeExecutableLocator>,
    packages: Arc<FakePackages>,
    operations: Arc<FakeOperations>,
    logging: Arc<FakeLogging>,
    mutations: Arc<FakeMutations>,
}

impl Fixture {
    fn new() -> Self {
        let repository = Arc::new(FakeRepository::default());
        let detection = Arc::new(FakeDetection::default());
        let executable_locator = Arc::new(FakeExecutableLocator::default());
        let packages = Arc::new(FakePackages::default());
        let operations = Arc::new(FakeOperations::default());
        let logging = Arc::new(FakeLogging::default());
        let mutations = Arc::new(FakeMutations::default());
        let service = CliApplicationService::new(CliApplicationPorts {
            repository: repository.clone(),
            detection: detection.clone(),
            executable_locator: executable_locator.clone(),
            packages: packages.clone(),
            operations: operations.clone(),
            logging: logging.clone(),
            clock: Arc::new(FakeClock),
            mutations: mutations.clone(),
        });
        Self {
            service,
            repository,
            detection,
            executable_locator,
            packages,
            operations,
            logging,
            mutations,
        }
    }

    fn insert_status(&self, status: CliToolStatus) {
        self.repository
            .statuses
            .lock()
            .expect("statuses")
            .insert(status.agent_id.clone(), status);
    }
}

fn unknown_status(definition: ToolDefinition) -> CliToolStatus {
    CliToolStatus::unavailable(
        definition,
        EnvironmentType::Linux,
        format!("npm install -g {}@latest", definition.package_name),
    )
}

fn eligible_status(agent_id: &str, current: &str, latest: &str) -> CliToolStatus {
    let definition = definition(agent_id).expect("definition");
    let path = format!("/fixture/npm/{}", definition.executable_name);
    let mut status = unknown_status(definition);
    status.installed = Some(true);
    status.current_version = Some(current.to_string());
    status.latest_version = Some(latest.to_string());
    status.detected_path = Some(path.clone());
    status.active_installation_path = Some(path.clone());
    status.version_check_status = VersionCheckStatus::Succeeded;
    status.lifecycle_eligibility = LifecycleEligibility::Npm;
    status.installations = vec![Installation {
        path,
        version: Some(current.to_string()),
        runnable: true,
        error: None,
        source: InstallSource::Npm,
        environment_type: EnvironmentType::Linux,
        is_active: true,
    }];
    status
}

#[test]
fn list_uses_catalog_order_and_cached_startup_state() {
    let fixture = Fixture::new();

    let statuses = fixture.service.list_tools().expect("list tools");

    assert_eq!(
        statuses
            .iter()
            .map(|status| status.agent_id.as_str())
            .collect::<Vec<_>>(),
        ["claude-code", "codex-cli", "gemini-cli", "opencode"]
    );
    assert!(fixture
        .service
        .needs_initial_refresh()
        .expect("initial refresh"));
    fixture
        .repository
        .has_cached_statuses
        .store(true, Ordering::SeqCst);
    assert!(!fixture
        .service
        .needs_initial_refresh()
        .expect("cached refresh"));
}

#[test]
fn executable_resolution_uses_cached_status_through_the_locator_port() {
    let fixture = Fixture::new();
    let mut status = unknown_status(definition("codex-cli").expect("definition"));
    status.detected_path = Some("/cached/codex".to_string());
    fixture.insert_status(status);
    *fixture
        .executable_locator
        .resolved
        .lock()
        .expect("resolved") = Some("/resolved/codex".to_string());

    let resolved = fixture
        .service
        .resolve_executable("codex-cli")
        .expect("resolve executable");

    assert_eq!(resolved.as_deref(), Some("/resolved/codex"));
    assert_eq!(
        fixture
            .executable_locator
            .requests
            .lock()
            .expect("requests")
            .as_slice(),
        &[("codex-cli".to_string(), Some("/cached/codex".to_string()))]
    );
}

#[test]
fn refresh_associates_clock_operation_and_both_log_channels() {
    let fixture = Fixture::new();
    fixture.detection.warnings.lock().expect("warnings").insert(
        "codex-cli".to_string(),
        vec!["registry unavailable".to_string()],
    );
    let prepared = fixture
        .service
        .prepare_refresh(
            Some("codex-cli".to_string()),
            "Refreshing CLI detections".to_string(),
        )
        .expect("prepare refresh");

    fixture
        .service
        .execute_refresh(prepared)
        .expect("execute refresh");

    let saved = fixture.repository.saved.lock().expect("saved");
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].last_operation_id.as_deref(), Some("op-1"));
    assert_eq!(
        saved[0].last_checked_at.as_deref(),
        Some("2026-07-18T12:00:00Z")
    );
    let operation_events = fixture.operations.events.lock().expect("events");
    assert!(
        operation_events.contains(&OperationEvent::Completed(CliOperationResult::Refresh {
            agent_ids: vec!["codex-cli".to_string()],
            failed: Vec::new(),
        }))
    );
    let operation_logs = operation_events
        .iter()
        .filter(|event| matches!(event, OperationEvent::Logged(_)))
        .count();
    assert_eq!(
        operation_logs,
        fixture.logging.events.lock().expect("logs").len()
    );
}

#[test]
fn install_start_failure_releases_the_agent_claim() {
    let fixture = Fixture::new();
    fixture.insert_status(eligible_status("codex-cli", "1.0.0", "2.0.0"));
    fixture.operations.fail_start.store(true, Ordering::SeqCst);

    let error = fixture
        .service
        .prepare_install("codex-cli".to_string(), "latest".to_string(), None)
        .expect_err("start must fail");

    assert_eq!(error.to_string(), "operation start failed");
    assert!(fixture
        .mutations
        .try_acquire("codex-cli")
        .expect("claim after failure"));
}

#[test]
fn package_failure_is_persisted_terminal_and_releases_the_claim() {
    let fixture = Fixture::new();
    fixture.insert_status(eligible_status("codex-cli", "1.0.0", "2.0.0"));
    fixture
        .packages
        .execution_failures
        .lock()
        .expect("execution failures")
        .insert("codex-cli".to_string());
    let prepared = fixture
        .service
        .prepare_install("codex-cli".to_string(), "2.0.0".to_string(), None)
        .expect("prepare install");

    fixture
        .service
        .execute_install(prepared)
        .expect("terminal failure recorded");

    assert!(fixture
        .operations
        .events
        .lock()
        .expect("events")
        .contains(&OperationEvent::Failed(
            "package manager failed".to_string()
        )));
    let failed = fixture.repository.saved.lock().expect("saved");
    assert_eq!(
        failed
            .last()
            .and_then(|status| status.last_error.as_deref()),
        Some("package manager failed")
    );
    assert_eq!(
        failed.last().map(|status| status.version_check_status),
        Some(VersionCheckStatus::Failed)
    );
    assert!(fixture
        .mutations
        .try_acquire("codex-cli")
        .expect("claim after execution"));
}

#[test]
fn successful_install_refreshes_and_persists_detection_before_completion() {
    let fixture = Fixture::new();
    fixture.insert_status(eligible_status("codex-cli", "1.0.0", "2.0.0"));
    let prepared = fixture
        .service
        .prepare_install("codex-cli".to_string(), "2.0.0".to_string(), None)
        .expect("prepare install");

    fixture
        .service
        .execute_install(prepared)
        .expect("execute install");

    assert_eq!(
        fixture
            .packages
            .executions
            .lock()
            .expect("executions")
            .as_slice(),
        [("codex-cli".to_string(), "2.0.0".to_string())]
    );
    let saved = fixture.repository.saved.lock().expect("saved");
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].last_operation_id.as_deref(), Some("op-1"));
    assert_eq!(
        saved[0].last_checked_at.as_deref(),
        Some("2026-07-18T12:00:00Z")
    );
    assert!(fixture.operations.events.lock().expect("events").contains(
        &OperationEvent::Completed(CliOperationResult::Install {
            agent_id: "codex-cli".to_string(),
            target_version: "2.0.0".to_string(),
        })
    ));
}

#[test]
fn bulk_upgrade_skips_busy_agents_and_releases_only_acquired_claims() {
    let fixture = Fixture::new();
    fixture.insert_status(eligible_status("codex-cli", "1.0.0", "2.0.0"));
    fixture.insert_status(eligible_status("gemini-cli", "1.0.0", "3.0.0"));
    assert!(fixture
        .mutations
        .try_acquire("gemini-cli")
        .expect("preclaim gemini"));
    let prepared = fixture
        .service
        .prepare_upgrade_all()
        .expect("prepare bulk upgrade");

    fixture
        .service
        .execute_upgrade_all(prepared)
        .expect("execute bulk upgrade");

    assert_eq!(
        fixture
            .packages
            .executions
            .lock()
            .expect("executions")
            .as_slice(),
        [("codex-cli".to_string(), "2.0.0".to_string())]
    );
    assert!(fixture.operations.events.lock().expect("events").contains(
        &OperationEvent::Completed(CliOperationResult::UpgradeAll {
            upgraded: vec!["codex-cli".to_string()],
            skipped: vec![
                "claude-code".to_string(),
                "gemini-cli".to_string(),
                "opencode".to_string(),
            ],
            failed: Vec::new(),
        })
    ));
    assert!(fixture
        .mutations
        .try_acquire("codex-cli")
        .expect("codex released"));
    assert!(!fixture
        .mutations
        .try_acquire("gemini-cli")
        .expect("gemini remains claimed"));
}
