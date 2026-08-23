use super::*;
use crate::contexts::tooling::cli::domain::{definition, EnvironmentType, ToolDefinition};
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
        // A detected tool, as the legacy detection path still reports one for Agent Runtime's
        // availability check. Nothing about versions matters here: this port no longer feeds a
        // lifecycle, only a resolved path and a timestamp.
        let mut status = unknown_status(definition);
        status.installed = Some(true);
        status.detected_path = Some(format!("/fixture/npm/{}", definition.executable_name));
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

/// What the fake operation port was asked to do, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OperationEvent {
    Started(CliOperationRequest),
    Logged(CliLogEvent),
    Completed(CliOperationResult),
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

struct Fixture {
    service: CliApplicationService,
    repository: Arc<FakeRepository>,
    detection: Arc<FakeDetection>,
    executable_locator: Arc<FakeExecutableLocator>,
    operations: Arc<FakeOperations>,
    logging: Arc<FakeLogging>,
}

impl Fixture {
    fn new() -> Self {
        let repository = Arc::new(FakeRepository::default());
        let detection = Arc::new(FakeDetection::default());
        let executable_locator = Arc::new(FakeExecutableLocator::default());
        let operations = Arc::new(FakeOperations::default());
        let logging = Arc::new(FakeLogging::default());
        let service = CliApplicationService::new(CliApplicationPorts {
            repository: repository.clone(),
            detection: detection.clone(),
            executable_locator: executable_locator.clone(),
            operations: operations.clone(),
            logging: logging.clone(),
            clock: Arc::new(FakeClock),
        });
        Self {
            service,
            repository,
            detection,
            executable_locator,
            operations,
            logging,
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
        format!(
            "npm install -g {}@latest",
            definition.package_name.unwrap_or_default()
        ),
    )
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
