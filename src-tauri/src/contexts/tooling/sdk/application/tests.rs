use super::*;
use crate::contexts::tooling::sdk::domain::{
    SdkDefinition, SdkId, SdkLifecycleAction, SdkLifecyclePlan, SdkOperationType, SdkVersionSource,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeRepository {
    installed: Mutex<BTreeMap<SdkId, String>>,
    paths: Mutex<BTreeMap<SdkId, String>>,
    logs: Mutex<Vec<SdkOperationLog>>,
    failures: Mutex<BTreeSet<SdkId>>,
}

impl SdkRepository for FakeRepository {
    fn installed_version(
        &self,
        definition: SdkDefinition,
    ) -> Result<Option<String>, SdkApplicationError> {
        if self
            .failures
            .lock()
            .expect("failures")
            .contains(&definition.id)
        {
            return Err(SdkApplicationError::Repository(
                "manifest read failed".to_string(),
            ));
        }
        Ok(self
            .installed
            .lock()
            .expect("installed")
            .get(&definition.id)
            .cloned())
    }

    fn install_path(&self, sdk_id: SdkId) -> Result<String, SdkApplicationError> {
        Ok(self
            .paths
            .lock()
            .expect("paths")
            .get(&sdk_id)
            .cloned()
            .unwrap_or_else(|| format!("/dependencies/{}", sdk_id.as_str())))
    }

    fn operation_logs(
        &self,
        sdk_id: Option<SdkId>,
    ) -> Result<Vec<SdkOperationLog>, SdkApplicationError> {
        Ok(self
            .logs
            .lock()
            .expect("logs")
            .iter()
            .filter(|log| sdk_id.is_none_or(|sdk_id| sdk_id == log.sdk_id))
            .cloned()
            .collect())
    }

    fn append_operation_log(&self, event: &SdkLogEvent) -> Result<(), SdkApplicationError> {
        self.logs
            .lock()
            .expect("logs")
            .push(SdkOperationLog::from(event));
        Ok(())
    }
}

struct FakePackages {
    environment: Mutex<SdkEnvironmentStatus>,
    versions: Mutex<BTreeMap<SdkId, Result<Vec<String>, String>>>,
    latest: Mutex<BTreeMap<SdkId, Result<String, String>>>,
    executions: Mutex<Vec<SdkLifecyclePlan>>,
    execution_error: Mutex<Option<String>>,
}

impl Default for FakePackages {
    fn default() -> Self {
        Self {
            environment: Mutex::new(SdkEnvironmentStatus {
                available: true,
                node_path: Some("/bin/node".to_string()),
                node_version: Some("v22.0.0".to_string()),
                npm_path: Some("/bin/npm".to_string()),
                npm_version: Some("10.0.0".to_string()),
                error: None,
            }),
            versions: Mutex::new(BTreeMap::new()),
            latest: Mutex::new(BTreeMap::new()),
            executions: Mutex::new(Vec::new()),
            execution_error: Mutex::new(None),
        }
    }
}

impl SdkPackageExecutionPort for FakePackages {
    fn environment(&self) -> Result<SdkEnvironmentStatus, SdkApplicationError> {
        Ok(self.environment.lock().expect("environment").clone())
    }

    fn available_versions(
        &self,
        definition: SdkDefinition,
    ) -> Result<Vec<String>, SdkApplicationError> {
        match self
            .versions
            .lock()
            .expect("versions")
            .get(&definition.id)
            .cloned()
            .unwrap_or_else(|| Ok(Vec::new()))
        {
            Ok(versions) => Ok(versions),
            Err(error) => Err(SdkApplicationError::Package(error)),
        }
    }

    fn latest_version(&self, definition: SdkDefinition) -> Result<String, SdkApplicationError> {
        match self
            .latest
            .lock()
            .expect("latest")
            .get(&definition.id)
            .cloned()
            .unwrap_or_else(|| Ok(definition.default_version.to_string()))
        {
            Ok(version) => Ok(version),
            Err(error) => Err(SdkApplicationError::Package(error)),
        }
    }

    fn execute(
        &self,
        _operation_id: &str,
        plan: &SdkLifecyclePlan,
        emit: &mut dyn FnMut(SdkPackageLog),
    ) -> Result<Option<String>, SdkApplicationError> {
        self.executions
            .lock()
            .expect("executions")
            .push(plan.clone());
        emit(SdkPackageLog {
            level: SdkLogLevel::Info,
            line: "package adapter output".to_string(),
            context: BTreeMap::from([("executable".to_string(), "npm".to_string())]),
        });
        if let Some(error) = self.execution_error.lock().expect("error").clone() {
            return Err(SdkApplicationError::Package(error));
        }
        Ok(plan.requested_version.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OperationEvent {
    Started(SdkId, SdkOperationType, String),
    Logged(SdkLogEvent),
    Completed(SdkOperationResult),
    Failed(String, String),
}

#[derive(Default)]
struct FakeOperations {
    next_id: AtomicUsize,
    events: Mutex<Vec<OperationEvent>>,
}

impl SdkOperationPort for FakeOperations {
    fn start(
        &self,
        sdk_id: SdkId,
        operation: SdkOperationType,
        message: String,
    ) -> Result<StartedSdkOperation, SdkApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Started(sdk_id, operation, message.clone()));
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(StartedSdkOperation {
            id: format!("sdk-op-{id}"),
            related_entity_id: Some(sdk_id.as_str().to_string()),
            message: Some(message),
            created_at: "created".to_string(),
            updated_at: "created".to_string(),
        })
    }

    fn append_log(&self, event: &SdkLogEvent) -> Result<(), SdkApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Logged(event.clone()));
        Ok(())
    }

    fn complete(&self, result: &SdkOperationResult) -> Result<(), SdkApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Completed(result.clone()));
        Ok(())
    }

    fn fail(&self, operation_id: &str, error: String) -> Result<(), SdkApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push(OperationEvent::Failed(operation_id.to_string(), error));
        Ok(())
    }
}

#[derive(Default)]
struct FakeLogging {
    events: Mutex<Vec<SdkLogEvent>>,
}

impl SdkLoggingPort for FakeLogging {
    fn record(&self, event: &SdkLogEvent) -> Result<(), SdkApplicationError> {
        self.events.lock().expect("events").push(event.clone());
        Ok(())
    }
}

struct FakeClock;

impl SdkClockPort for FakeClock {
    fn now(&self) -> String {
        "2026-07-18T12:00:00Z".to_string()
    }
}

struct Fixture {
    service: SdkApplicationService,
    repository: Arc<FakeRepository>,
    packages: Arc<FakePackages>,
    operations: Arc<FakeOperations>,
    logging: Arc<FakeLogging>,
}

impl Fixture {
    fn new() -> Self {
        let repository = Arc::new(FakeRepository::default());
        let packages = Arc::new(FakePackages::default());
        let operations = Arc::new(FakeOperations::default());
        let logging = Arc::new(FakeLogging::default());
        let service = SdkApplicationService::new(
            repository.clone(),
            packages.clone(),
            operations.clone(),
            logging.clone(),
            Arc::new(FakeClock),
        );
        Self {
            service,
            repository,
            packages,
            operations,
            logging,
        }
    }
}

#[test]
fn definition_status_environment_and_installed_queries_use_ports() {
    let fixture = Fixture::new();
    fixture
        .repository
        .installed
        .lock()
        .expect("installed")
        .insert(SdkId::ClaudeSdk, "0.2.81".to_string());

    assert_eq!(fixture.service.list_definitions().len(), 2);
    let statuses = fixture.service.list_statuses().expect("statuses");
    assert_eq!(statuses.len(), 2);
    assert!(statuses[0].has_update);
    assert_eq!(
        statuses[0].last_checked.as_deref(),
        Some("2026-07-18T12:00:00Z")
    );
    assert!(fixture
        .service
        .is_installed(SdkId::ClaudeSdk)
        .expect("installed query"));
    assert!(
        fixture
            .service
            .check_environment()
            .expect("environment")
            .available
    );
}

#[test]
fn version_and_update_queries_apply_remote_and_fallback_rules() {
    let fixture = Fixture::new();
    fixture.packages.versions.lock().expect("versions").insert(
        SdkId::ClaudeSdk,
        Ok(vec![
            "0.2.81".to_string(),
            "0.2.90".to_string(),
            "0.2.90-beta.1".to_string(),
        ]),
    );
    fixture
        .packages
        .versions
        .lock()
        .expect("versions")
        .insert(SdkId::CodexSdk, Err("registry unavailable".to_string()));

    let versions = fixture.service.get_versions(None);
    assert_eq!(
        versions[&SdkId::ClaudeSdk].latest_version.as_deref(),
        Some("0.2.90")
    );
    assert_eq!(
        versions[&SdkId::CodexSdk].source,
        SdkVersionSource::Fallback
    );

    fixture
        .repository
        .installed
        .lock()
        .expect("installed")
        .insert(SdkId::ClaudeSdk, "0.2.81".to_string());
    fixture
        .packages
        .latest
        .lock()
        .expect("latest")
        .insert(SdkId::ClaudeSdk, Err("latest lookup failed".to_string()));
    let updates = fixture
        .service
        .check_updates(Some(SdkId::ClaudeSdk))
        .expect("updates");
    assert!(updates[&SdkId::ClaudeSdk].has_update);
    assert_eq!(
        updates[&SdkId::ClaudeSdk].error_message.as_deref(),
        Some("latest lookup failed")
    );
}

#[test]
fn rollback_operation_coordinates_package_logs_and_terminal_result() {
    let fixture = Fixture::new();
    let prepared = fixture
        .service
        .prepare_operation(SdkOperationRequest {
            sdk_id: SdkId::ClaudeSdk,
            operation: SdkOperationType::Rollback,
            version: Some("0.2.58".to_string()),
        })
        .expect("prepare");
    assert_eq!(prepared.operation.id, "sdk-op-1");

    fixture
        .service
        .execute_operation(prepared)
        .expect("execute");

    let executions = fixture.packages.executions.lock().expect("executions");
    assert_eq!(executions[0].requested_version.as_deref(), Some("0.2.58"));
    assert_eq!(executions[0].action, SdkLifecycleAction::InstallPackages);
    let events = fixture.operations.events.lock().expect("events");
    let completed = events
        .iter()
        .find_map(|event| match event {
            OperationEvent::Completed(result) => Some(result),
            _ => None,
        })
        .expect("completed result");
    assert_eq!(completed.operation_id, "sdk-op-1");
    assert_eq!(completed.installed_version.as_deref(), Some("0.2.58"));
    assert_eq!(completed.logs.len(), 3);
    assert_eq!(fixture.repository.logs.lock().expect("logs").len(), 3);
    assert_eq!(fixture.logging.events.lock().expect("events").len(), 3);
}

#[test]
fn package_failure_emits_error_and_fails_the_same_operation() {
    let fixture = Fixture::new();
    *fixture
        .packages
        .execution_error
        .lock()
        .expect("execution error") = Some("npm failed".to_string());
    let prepared = fixture
        .service
        .prepare_operation(SdkOperationRequest {
            sdk_id: SdkId::CodexSdk,
            operation: SdkOperationType::Update,
            version: Some("0.116.0".to_string()),
        })
        .expect("prepare");

    fixture
        .service
        .execute_operation(prepared)
        .expect("terminal failure recorded");

    let events = fixture.operations.events.lock().expect("events");
    assert!(events.iter().any(|event| {
        matches!(event, OperationEvent::Failed(id, error) if id == "sdk-op-1" && error == "npm failed")
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, OperationEvent::Completed(_))));
    assert!(fixture
        .logging
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| event.level == SdkLogLevel::Error && event.line == "npm failed"));
}

#[test]
fn uninstall_operation_uses_remove_plan_and_scoped_log_query() {
    let fixture = Fixture::new();
    let prepared = fixture
        .service
        .prepare_operation(SdkOperationRequest {
            sdk_id: SdkId::ClaudeSdk,
            operation: SdkOperationType::Uninstall,
            version: Some("ignored".to_string()),
        })
        .expect("prepare");
    fixture
        .service
        .execute_operation(prepared)
        .expect("execute");

    let executions = fixture.packages.executions.lock().expect("executions");
    assert_eq!(executions[0].action, SdkLifecycleAction::RemoveInstallation);
    assert!(executions[0].requested_version.is_none());
    let logs = fixture
        .service
        .operation_logs(Some(SdkId::ClaudeSdk))
        .expect("logs");
    assert_eq!(logs.len(), 3);
    assert!(fixture
        .service
        .operation_logs(Some(SdkId::CodexSdk))
        .expect("logs")
        .is_empty());
}
