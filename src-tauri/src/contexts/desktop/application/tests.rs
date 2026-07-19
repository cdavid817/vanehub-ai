use super::*;
use crate::contexts::desktop::domain::{
    AutomaticArchivalSettings, DesktopSettingKey, DesktopSettingMutation, DesktopSettings,
    NetworkProxyPreferences, StartupPreference,
};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeRepositoryState {
    values: BTreeMap<DesktopSettingKey, String>,
    fail_save: Option<String>,
}

#[derive(Clone)]
struct FakeRepository {
    state: Arc<Mutex<FakeRepositoryState>>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl DesktopSettingsRepository for FakeRepository {
    fn load_settings(&self) -> Result<Vec<StoredDesktopSetting>, DesktopSettingsApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("repository:load".to_string());
        Ok(self
            .state
            .lock()
            .expect("repository state")
            .values
            .iter()
            .map(|(key, value)| StoredDesktopSetting {
                key: *key,
                value: value.clone(),
            })
            .collect())
    }

    fn save_setting(
        &self,
        mutation: &DesktopSettingMutation,
        updated_at: &str,
    ) -> Result<(), DesktopSettingsApplicationError> {
        self.calls.lock().expect("calls").push(format!(
            "repository:save:{}:{updated_at}",
            mutation.key().as_str()
        ));
        let mut state = self.state.lock().expect("repository state");
        if let Some(message) = state.fail_save.take() {
            return Err(DesktopSettingsApplicationError::Repository(message));
        }
        state
            .values
            .insert(mutation.key(), mutation.persisted_value());
        Ok(())
    }

    fn save_automatic_archival(
        &self,
        settings: AutomaticArchivalSettings,
        updated_at: &str,
    ) -> Result<(), DesktopSettingsApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("repository:save-archival:{updated_at}"));
        let mut state = self.state.lock().expect("repository state");
        if let Some(message) = state.fail_save.take() {
            return Err(DesktopSettingsApplicationError::Repository(message));
        }
        state.values.insert(
            DesktopSettingKey::AutomaticArchivalEnabled,
            settings.enabled().to_string(),
        );
        state.values.insert(
            DesktopSettingKey::AutomaticArchivalInactiveDays,
            settings.inactive_days().to_string(),
        );
        Ok(())
    }
}

#[derive(Clone)]
struct FixedClock;

impl DesktopClockPort for FixedClock {
    fn now(&self) -> String {
        "2026-07-18T12:00:00Z".to_string()
    }
}

#[derive(Clone)]
struct FakeNetworkProxy {
    calls: Arc<Mutex<Vec<String>>>,
}

impl DesktopNetworkProxyPort for FakeNetworkProxy {
    fn apply(
        &self,
        preferences: &NetworkProxyPreferences,
    ) -> Result<(), DesktopSettingsApplicationError> {
        self.calls.lock().expect("calls").push(format!(
            "network:{}:{}",
            preferences.url(),
            preferences.bypass()
        ));
        Ok(())
    }
}

#[derive(Clone)]
struct FakeLogDirectory {
    calls: Arc<Mutex<Vec<String>>>,
    fail_validation: Arc<Mutex<Option<String>>>,
}

impl DesktopLogDirectoryPort for FakeLogDirectory {
    fn validate(&self, path: &str) -> Result<(), DesktopSettingsApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("log:validate:{path}"));
        if let Some(message) = self.fail_validation.lock().expect("failure").take() {
            return Err(DesktopSettingsApplicationError::LogDirectory(message));
        }
        Ok(())
    }

    fn activate(&self, path: &str) -> Result<(), DesktopSettingsApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("log:activate:{path}"));
        Ok(())
    }
}

#[derive(Clone)]
struct FakeStartup {
    calls: Arc<Mutex<Vec<String>>>,
    failure: Arc<Mutex<Option<String>>>,
}

impl DesktopStartupPort for FakeStartup {
    fn apply(&self, preference: StartupPreference) -> Result<(), DesktopSettingsApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("startup:{}", preference.enabled()));
        if let Some(message) = self.failure.lock().expect("failure").take() {
            return Err(DesktopSettingsApplicationError::Startup(message));
        }
        Ok(())
    }
}

struct Fixture {
    service: DesktopSettingsApplicationService,
    repository: FakeRepository,
    calls: Arc<Mutex<Vec<String>>>,
    log_validation_failure: Arc<Mutex<Option<String>>>,
    startup_failure: Arc<Mutex<Option<String>>>,
}

impl Fixture {
    fn new() -> Self {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let repository = FakeRepository {
            state: Arc::new(Mutex::new(FakeRepositoryState::default())),
            calls: calls.clone(),
        };
        let log_validation_failure = Arc::new(Mutex::new(None));
        let startup_failure = Arc::new(Mutex::new(None));
        let service = DesktopSettingsApplicationService::new(
            Arc::new(repository.clone()),
            Arc::new(FixedClock),
            Arc::new(FakeNetworkProxy {
                calls: calls.clone(),
            }),
            Arc::new(FakeLogDirectory {
                calls: calls.clone(),
                fail_validation: log_validation_failure.clone(),
            }),
            Arc::new(FakeStartup {
                calls: calls.clone(),
                failure: startup_failure.clone(),
            }),
            "D:/data/logs",
        );
        Self {
            service,
            repository,
            calls,
            log_validation_failure,
            startup_failure,
        }
    }

    fn set_stored(&self, key: DesktopSettingKey, value: &str) {
        self.repository
            .state
            .lock()
            .expect("repository state")
            .values
            .insert(key, value.to_string());
    }

    fn settings(&self) -> DesktopSettings {
        self.service.get_settings().expect("settings")
    }
}

#[test]
fn query_merges_valid_stored_values_and_falls_back_for_invalid_legacy_values() {
    let fixture = Fixture::new();
    fixture.set_stored(DesktopSettingKey::ApplicationLanguage, "en");
    fixture.set_stored(DesktopSettingKey::FontSize, "20px");
    fixture.set_stored(
        DesktopSettingKey::NetworkProxyBypass,
        " localhost 127.0.0.1 ",
    );

    let settings = fixture.settings();

    assert_eq!(settings.application_language().as_str(), "en");
    assert_eq!(settings.font_size().as_str(), "14px");
    assert_eq!(settings.network_proxy().bypass(), "localhost,127.0.0.1");
    assert_eq!(
        fixture.calls.lock().expect("calls").as_slice(),
        ["repository:load", "network::localhost,127.0.0.1"]
    );
}

#[test]
fn log_directory_save_validates_before_persistence_and_activates_after_reload() {
    let fixture = Fixture::new();

    let settings = fixture
        .service
        .save_setting(
            DesktopSettingMutation::parse("logDirectory", "D:/custom/logs").expect("mutation"),
        )
        .expect("save");

    assert_eq!(settings.log_directory(), "D:/custom/logs");
    assert_eq!(
        fixture.calls.lock().expect("calls").as_slice(),
        [
            "log:validate:D:/custom/logs",
            "repository:save:logDirectory:2026-07-18T12:00:00Z",
            "repository:load",
            "network::localhost,127.0.0.1,::1",
            "log:activate:D:/custom/logs",
        ]
    );
}

#[test]
fn failed_log_directory_validation_prevents_the_repository_write() {
    let fixture = Fixture::new();
    *fixture.log_validation_failure.lock().expect("failure") = Some("not a directory".to_string());

    let error = fixture
        .service
        .save_setting(DesktopSettingMutation::parse("logDirectory", "D:/file").expect("mutation"))
        .expect_err("validation failure");

    assert_eq!(
        error,
        DesktopSettingsApplicationError::LogDirectory("not a directory".to_string())
    );
    assert_eq!(
        fixture.calls.lock().expect("calls").as_slice(),
        ["log:validate:D:/file"]
    );
}

#[test]
fn archival_use_case_validates_before_one_atomic_repository_behavior() {
    let fixture = Fixture::new();

    let invalid = fixture
        .service
        .save_automatic_archival_settings(true, 0)
        .expect_err("threshold");
    assert!(matches!(
        invalid,
        DesktopSettingsApplicationError::Domain(_)
    ));
    assert!(fixture.calls.lock().expect("calls").is_empty());

    let saved = fixture
        .service
        .save_automatic_archival_settings(false, 30)
        .expect("archival save");
    assert_eq!(
        saved,
        AutomaticArchivalSettings::new(false, 30).expect("archival")
    );
    assert_eq!(
        fixture.calls.lock().expect("calls").as_slice(),
        [
            "repository:save-archival:2026-07-18T12:00:00Z",
            "repository:load",
            "network::localhost,127.0.0.1,::1",
        ]
    );
}

#[test]
fn startup_sync_occurs_after_persistence_and_keeps_the_saved_preference_on_failure() {
    let fixture = Fixture::new();
    *fixture.startup_failure.lock().expect("failure") = Some("OS denied".to_string());

    let error = fixture
        .service
        .set_launch_on_startup(true)
        .expect_err("startup failure");

    assert_eq!(
        error,
        DesktopSettingsApplicationError::Startup("OS denied".to_string())
    );
    assert_eq!(
        fixture
            .repository
            .state
            .lock()
            .expect("repository state")
            .values
            .get(&DesktopSettingKey::LaunchOnStartup)
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        fixture.calls.lock().expect("calls").as_slice(),
        [
            "repository:save:launchOnStartup:2026-07-18T12:00:00Z",
            "repository:load",
            "network::localhost,127.0.0.1,::1",
            "startup:true",
        ]
    );
}

#[test]
fn repository_failure_stops_external_side_effects() {
    let fixture = Fixture::new();
    fixture
        .repository
        .state
        .lock()
        .expect("repository state")
        .fail_save = Some("database unavailable".to_string());

    let error = fixture
        .service
        .set_launch_on_startup(true)
        .expect_err("repository failure");

    assert_eq!(
        error,
        DesktopSettingsApplicationError::Repository("database unavailable".to_string())
    );
    assert_eq!(
        fixture.calls.lock().expect("calls").as_slice(),
        ["repository:save:launchOnStartup:2026-07-18T12:00:00Z"]
    );
}

#[derive(Clone)]
struct FakeDirectories {
    calls: Arc<Mutex<Vec<String>>>,
}

impl DesktopDirectoryPort for FakeDirectories {
    fn data_management_info(
        &self,
    ) -> Result<DataManagementInformation, DesktopSettingsApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("directories:info".to_string());
        Ok(DataManagementInformation {
            database_path: "D:/data/vanehub.sqlite".to_string(),
            database_directory: "D:/data".to_string(),
            can_open_directory: true,
        })
    }

    fn open_database_directory(&self) -> Result<(), DesktopSettingsApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push("directories:open-database".to_string());
        Ok(())
    }

    fn open_log_directory(&self, path: &str) -> Result<(), DesktopSettingsApplicationError> {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("directories:open-log:{path}"));
        Ok(())
    }
}

#[derive(Clone)]
struct FakeNode;

impl DesktopNodeInfoPort for FakeNode {
    fn inspect(&self) -> NodeInformation {
        NodeInformation {
            available: true,
            path: Some("C:/node.exe".to_string()),
            version: Some("v22.0.0".to_string()),
            reason: None,
        }
    }
}

#[derive(Clone)]
struct FakeProxyActions {
    calls: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl DesktopNetworkProxyActionsPort for FakeProxyActions {
    async fn test(
        &self,
        preferences: &NetworkProxyPreferences,
    ) -> Result<NetworkProxyTestResult, DesktopSettingsApplicationError> {
        self.calls.lock().expect("calls").push(format!(
            "proxy:test:{}:{}",
            preferences.url(),
            preferences.bypass()
        ));
        Ok(NetworkProxyTestResult {
            success: true,
            latency_ms: 25,
            error: None,
        })
    }

    async fn scan(&self) -> Vec<DetectedNetworkProxy> {
        self.calls
            .lock()
            .expect("calls")
            .push("proxy:scan".to_string());
        vec![DetectedNetworkProxy {
            url: "http://127.0.0.1:7890".to_string(),
            proxy_type: "http".to_string(),
            port: 7890,
        }]
    }
}

#[derive(Clone)]
struct FakeClientLogging {
    events: ClientLogEvents,
}

type RecordedCalls = Arc<Mutex<Vec<String>>>;
type ClientLogEvents = Arc<Mutex<Vec<(String, ClientLogEvent)>>>;

impl DesktopClientLoggingPort for FakeClientLogging {
    fn record(
        &self,
        log_directory: &str,
        event: ClientLogEvent,
    ) -> Result<(), DesktopSettingsApplicationError> {
        self.events
            .lock()
            .expect("events")
            .push((log_directory.to_string(), event));
        Ok(())
    }
}

fn environment_fixture() -> (
    DesktopEnvironmentApplicationService,
    RecordedCalls,
    ClientLogEvents,
) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let events = Arc::new(Mutex::new(Vec::new()));
    (
        DesktopEnvironmentApplicationService::new(
            Arc::new(FakeDirectories {
                calls: calls.clone(),
            }),
            Arc::new(FakeNode),
            Arc::new(FakeProxyActions {
                calls: calls.clone(),
            }),
            Arc::new(FakeClientLogging {
                events: events.clone(),
            }),
        ),
        calls,
        events,
    )
}

#[tokio::test]
async fn proxy_actions_validate_and_normalize_before_calling_the_external_port() {
    let (service, calls, _events) = environment_fixture();

    let invalid = service
        .test_network_proxy("ftp://127.0.0.1:21".to_string(), "localhost".to_string())
        .await
        .expect_err("invalid proxy");
    assert!(matches!(
        invalid,
        DesktopSettingsApplicationError::Domain(_)
    ));
    assert!(calls.lock().expect("calls").is_empty());

    let result = service
        .test_network_proxy(
            "http://127.0.0.1:7890".to_string(),
            " localhost 127.0.0.1 ".to_string(),
        )
        .await
        .expect("proxy test");
    let detected = service.scan_network_proxies().await;

    assert!(result.success);
    assert_eq!(detected[0].port, 7890);
    assert_eq!(
        calls.lock().expect("calls").as_slice(),
        [
            "proxy:test:http://127.0.0.1:7890:localhost,127.0.0.1",
            "proxy:scan",
        ]
    );
}

#[test]
fn environment_queries_and_client_logs_delegate_without_runtime_dependencies() {
    let (service, calls, events) = environment_fixture();
    let information = service.data_management_info().expect("data information");
    service
        .open_database_directory()
        .expect("open database directory");
    service
        .open_log_directory("D:/data/logs")
        .expect("open log directory");
    let node = service.node_information();
    service
        .report_client_log(
            "D:/data/logs",
            ClientLogEvent {
                level: DesktopLogLevel::Warn,
                kind: ClientLogEventKind::ErrorBoundary,
                message: "fixture".to_string(),
                source: "desktop-test".to_string(),
                details: None,
                stack: None,
            },
        )
        .expect("client log");

    assert_eq!(information.database_directory, "D:/data");
    assert!(node.available);
    assert_eq!(
        calls.lock().expect("calls").as_slice(),
        [
            "directories:info",
            "directories:open-database",
            "directories:open-log:D:/data/logs",
        ]
    );
    let events = events.lock().expect("events");
    assert_eq!(events[0].0, "D:/data/logs");
    assert_eq!(events[0].1.level, DesktopLogLevel::Warn);
}
