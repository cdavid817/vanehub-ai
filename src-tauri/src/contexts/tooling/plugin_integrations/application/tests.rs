use super::*;
use crate::contexts::tooling::plugin_integrations::domain::{
    PluginIntegrationId, PluginIntegrationStatus, PluginIntegrationToolOutcome,
    PluginIntegrationToolPlan,
};
use std::sync::{Arc, Mutex};

struct FakeTool {
    outcome: Mutex<PluginIntegrationToolOutcome>,
    plans: Mutex<Vec<PluginIntegrationToolPlan>>,
}

impl FakeTool {
    fn new(outcome: PluginIntegrationToolOutcome) -> Self {
        Self {
            outcome: Mutex::new(outcome),
            plans: Mutex::new(Vec::new()),
        }
    }
}

impl PluginIntegrationToolPort for FakeTool {
    fn execute(&self, plan: PluginIntegrationToolPlan) -> PluginIntegrationToolOutcome {
        self.plans.lock().expect("plans").push(plan);
        self.outcome.lock().expect("outcome").clone()
    }
}

#[derive(Default)]
struct FakeLogging {
    diagnostics: Mutex<Vec<PluginIntegrationDiagnostic>>,
    fail: Mutex<bool>,
}

impl PluginIntegrationLoggingPort for FakeLogging {
    fn record(
        &self,
        diagnostic: &PluginIntegrationDiagnostic,
    ) -> Result<(), PluginIntegrationApplicationError> {
        self.diagnostics
            .lock()
            .expect("diagnostics")
            .push(diagnostic.clone());
        if *self.fail.lock().expect("fail") {
            Err(PluginIntegrationApplicationError::Logging(
                "log unavailable".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

struct FixedClock;

impl PluginIntegrationClockPort for FixedClock {
    fn now(&self) -> String {
        "2026-07-18T00:00:00Z".to_string()
    }
}

struct Fixture {
    service: PluginIntegrationApplicationService,
    tool: Arc<FakeTool>,
    logging: Arc<FakeLogging>,
}

impl Fixture {
    fn new(outcome: PluginIntegrationToolOutcome) -> Self {
        let tool = Arc::new(FakeTool::new(outcome));
        let logging = Arc::new(FakeLogging::default());
        Self {
            service: PluginIntegrationApplicationService::new(
                tool.clone(),
                logging.clone(),
                Arc::new(FixedClock),
            ),
            tool,
            logging,
        }
    }
}

#[test]
fn overview_and_refresh_expose_defaults_without_executing_external_tools() {
    let fixture = Fixture::new(PluginIntegrationToolOutcome::LaunchFailed);

    for overview in [fixture.service.overview(), fixture.service.refresh()] {
        assert_eq!(overview.definitions.len(), 1);
        assert_eq!(overview.definitions[0].id, PluginIntegrationId::Github);
        assert_eq!(overview.states.len(), 1);
        assert_eq!(
            overview.states[0].status_reason_key.as_deref(),
            Some("plugins.statusReason.notChecked")
        );
        assert_eq!(overview.environment.runtime, "tauri");
        assert!(overview.environment.native_checks_available);
    }

    assert!(fixture.tool.plans.lock().expect("plans").is_empty());
    assert!(fixture
        .logging
        .diagnostics
        .lock()
        .expect("diagnostics")
        .is_empty());
}

#[test]
fn readiness_coordinates_the_allowlisted_tool_clock_and_safe_diagnostic() {
    let fixture = Fixture::new(PluginIntegrationToolOutcome::Completed {
        success: true,
        stdout: "Logged in".to_string(),
        stderr: String::new(),
    });

    let result = fixture.service.test_readiness("github").expect("readiness");
    assert_eq!(result.status, PluginIntegrationStatus::Configured);
    assert!(result.configured);
    assert_eq!(result.message, "plugins.statusReason.configured");
    assert_eq!(result.checked_at, "2026-07-18T00:00:00Z");

    let plans = fixture.tool.plans.lock().expect("plans");
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].executable, "gh");
    assert_eq!(plans[0].arguments, &["auth", "status"]);
    assert_eq!(plans[0].timeout_seconds, 10);
    drop(plans);

    let diagnostics = fixture.logging.diagnostics.lock().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].integration_id, PluginIntegrationId::Github);
    assert_eq!(diagnostics[0].operation, "readiness-check");
    assert_eq!(diagnostics[0].level, PluginIntegrationDiagnosticLevel::Info);
    assert_eq!(diagnostics[0].status, PluginIntegrationStatus::Configured);
}

#[test]
fn expected_external_failures_return_stable_results_and_warn_diagnostics() {
    let fixture = Fixture::new(PluginIntegrationToolOutcome::MissingExecutable);

    let result = fixture
        .service
        .test_readiness("github")
        .expect("classified result");
    assert_eq!(result.status, PluginIntegrationStatus::MissingCli);
    assert!(!result.configured);
    assert_eq!(result.message, "plugins.statusReason.missingCli");
    assert_eq!(
        fixture.logging.diagnostics.lock().expect("diagnostics")[0].level,
        PluginIntegrationDiagnosticLevel::Warn
    );
}

#[test]
fn unknown_integration_is_rejected_before_clock_tool_or_logging_ports() {
    let fixture = Fixture::new(PluginIntegrationToolOutcome::Completed {
        success: true,
        stdout: String::new(),
        stderr: String::new(),
    });

    let error = fixture
        .service
        .test_readiness("gitlab")
        .expect_err("unknown id");
    assert_eq!(error.to_string(), "Unknown plugin integration: gitlab");
    assert!(fixture.tool.plans.lock().expect("plans").is_empty());
    assert!(fixture
        .logging
        .diagnostics
        .lock()
        .expect("diagnostics")
        .is_empty());
}

#[test]
fn diagnostic_storage_failure_does_not_hide_the_readiness_result() {
    let fixture = Fixture::new(PluginIntegrationToolOutcome::TimedOut);
    *fixture.logging.fail.lock().expect("fail") = true;

    let result = fixture
        .service
        .test_readiness("github")
        .expect("readiness result");
    assert_eq!(result.status, PluginIntegrationStatus::Error);
    assert_eq!(
        fixture
            .logging
            .diagnostics
            .lock()
            .expect("diagnostics")
            .len(),
        1
    );
}
