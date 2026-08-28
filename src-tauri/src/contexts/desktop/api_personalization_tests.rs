//! The compatibility window: the pre-governance settings page against the dedicated policy.
//!
//! The bridge is faked here on purpose. What is under test is the routing — which keys leave the
//! settings table, what the overlay does to a read, and whether a stale save is refused — and the
//! policy's own behaviour is tested where the policy lives.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tempfile::TempDir;

use super::api::{
    DesktopSettingsApi, PersonalizationSaveRejection, PersonalizationSettingsBridge,
    PersonalizationSettingsSnapshot,
};
use super::application::{
    DesktopClockPort, DesktopEnvironmentApplicationService, DesktopLocalePort,
    DesktopLogDirectoryPort, DesktopNetworkProxyPort, DesktopSettingsApplicationError,
    DesktopSettingsApplicationService, DesktopStartupPort,
};
use super::domain::{ApplicationLanguage, NetworkProxyPreferences, StartupPreference};
use super::infrastructure::{
    DesktopDirectoryAdapter, FolderOpenerService, PlatformNodeInfoAdapter,
    RuntimeNetworkProxyActionsAdapter, SqliteDesktopSettingsRepository,
    UnifiedClientLoggingAdapter,
};
use crate::platform::database::NativeDatabase;

/// The four side-effecting ports, stubbed. None of them is what these tests are about, and every
/// real one needs a live desktop shell.
struct NoopPorts;

impl DesktopClockPort for NoopPorts {
    fn now(&self) -> String {
        "2026-08-25T09:00:00.000Z".to_string()
    }
}

impl DesktopNetworkProxyPort for NoopPorts {
    fn apply(
        &self,
        _preferences: &NetworkProxyPreferences,
    ) -> Result<(), DesktopSettingsApplicationError> {
        Ok(())
    }
}

impl DesktopLogDirectoryPort for NoopPorts {
    fn validate(&self, _path: &str) -> Result<(), DesktopSettingsApplicationError> {
        Ok(())
    }
    fn activate(&self, _path: &str) -> Result<(), DesktopSettingsApplicationError> {
        Ok(())
    }
}

impl DesktopStartupPort for NoopPorts {
    fn apply(&self, _preference: StartupPreference) -> Result<(), DesktopSettingsApplicationError> {
        Ok(())
    }
}

impl DesktopLocalePort for NoopPorts {
    fn apply(&self, _language: ApplicationLanguage) -> Result<(), DesktopSettingsApplicationError> {
        Ok(())
    }
}

/// The five keys the policy owns.
const PERSONALIZATION_KEYS: &[&str] = &[
    "customInstructionsAboutUser",
    "customInstructionsStyleRules",
    "customInstructionsEnabled",
    "memoryEnabled",
    "memoryToolAssistedChatsEnabled",
];

#[derive(Default)]
struct FakePolicy {
    snapshot: Mutex<PersonalizationSettingsSnapshot>,
    saves: AtomicUsize,
    unavailable: Mutex<Option<String>>,
}

impl Default for PersonalizationSettingsSnapshot {
    fn default() -> Self {
        Self {
            about_user: "from the policy".to_string(),
            style_rules: "policy rules".to_string(),
            custom_instructions_enabled: true,
            memory_enabled: false,
            tool_assisted_extraction_enabled: false,
            revision: 3,
        }
    }
}

impl PersonalizationSettingsBridge for FakePolicy {
    fn view(&self) -> Result<PersonalizationSettingsSnapshot, String> {
        if let Some(reason) = self.unavailable.lock().expect("unavailable").clone() {
            return Err(reason);
        }
        Ok(self.snapshot.lock().expect("snapshot").clone())
    }

    fn save(
        &self,
        key: &str,
        value: &str,
        expected_revision: u64,
    ) -> Result<PersonalizationSettingsSnapshot, PersonalizationSaveRejection> {
        self.saves.fetch_add(1, Ordering::SeqCst);
        if let Some(reason) = self.unavailable.lock().expect("unavailable").clone() {
            return Err(PersonalizationSaveRejection::Unavailable(reason));
        }
        let mut snapshot = self.snapshot.lock().expect("snapshot");
        if expected_revision != snapshot.revision {
            return Err(PersonalizationSaveRejection::Conflict {
                expected: expected_revision,
                current: snapshot.revision,
            });
        }
        match key {
            "customInstructionsAboutUser" => snapshot.about_user = value.to_string(),
            "memoryEnabled" => snapshot.memory_enabled = value == "true",
            _ => {}
        }
        snapshot.revision += 1;
        Ok(snapshot.clone())
    }

    fn owns(&self, key: &str) -> bool {
        PERSONALIZATION_KEYS.contains(&key)
    }
}

struct Fixture {
    _directory: TempDir,
    api: DesktopSettingsApi,
    policy: Arc<FakePolicy>,
}

fn fixture(label: &str) -> Fixture {
    let directory =
        TempDir::with_prefix(format!("desktop-personalization-{label}-")).expect("directory");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteDesktopSettingsRepository::new(database.clone());
    let settings = DesktopSettingsApplicationService::new(
        Arc::new(repository.clone()),
        Arc::new(NoopPorts),
        Arc::new(NoopPorts),
        Arc::new(NoopPorts),
        Arc::new(NoopPorts),
        Arc::new(NoopPorts),
        directory.path().to_string_lossy().to_string(),
    );
    let environment = DesktopEnvironmentApplicationService::new(
        Arc::new(DesktopDirectoryAdapter::new(database.clone())),
        Arc::new(PlatformNodeInfoAdapter),
        Arc::new(RuntimeNetworkProxyActionsAdapter),
        Arc::new(UnifiedClientLoggingAdapter),
    );
    let api = DesktopSettingsApi::new(settings, environment, FolderOpenerService::new(repository));
    Fixture {
        _directory: directory,
        api,
        policy: Arc::new(FakePolicy::default()),
    }
}

impl Fixture {
    fn bind(&self) {
        self.api.bind_personalization(self.policy.clone());
    }
}

#[test]
fn an_unbound_api_still_answers_from_the_legacy_rows() {
    // The window before personalization is assembled. The page has to keep working, and it has to
    // report no revision, so nothing can be written back against a policy that is not there yet.
    let fixture = fixture("unbound");

    let view = fixture.api.get_settings().expect("settings");

    assert_eq!(view.personalization_revision, None);
    assert_eq!(view.settings.custom_instructions_about_user(), "");
}

#[test]
fn a_bound_policy_becomes_the_source_of_truth_for_a_read() {
    let fixture = fixture("read-through");
    // Written into the legacy rows first, so the overlay has something to disagree with.
    fixture
        .api
        .save_setting("customInstructionsAboutUser", "from the rows", None)
        .expect("legacy write");
    fixture.bind();

    let view = fixture.api.get_settings().expect("settings");

    assert_eq!(
        view.settings.custom_instructions_about_user(),
        "from the policy",
        "the policy wins over the stale row"
    );
    assert_eq!(view.personalization_revision, Some(3));
}

#[test]
fn a_personalization_save_goes_to_the_policy_and_never_to_the_rows() {
    let fixture = fixture("write-through");
    fixture.bind();

    let view = fixture
        .api
        .save_setting("customInstructionsAboutUser", "edited", Some(3))
        .expect("write-through");

    assert_eq!(view.settings.custom_instructions_about_user(), "edited");
    assert_eq!(view.personalization_revision, Some(4));
    assert_eq!(fixture.policy.saves.load(Ordering::SeqCst), 1);
}

#[test]
fn a_save_with_a_stale_revision_is_refused_with_a_typed_conflict() {
    let fixture = fixture("conflict");
    fixture.bind();
    fixture
        .api
        .save_setting("memoryEnabled", "true", Some(3))
        .expect("first edit");

    let refused = fixture.api.save_setting("memoryEnabled", "false", Some(3));

    match refused {
        Err(DesktopSettingsApplicationError::PersonalizationConflict { expected, current }) => {
            assert_eq!(expected, 3);
            assert_eq!(current, 4);
        }
        other => panic!("expected a typed conflict, got {other:?}"),
    }
}

#[test]
fn a_personalization_save_without_a_revision_is_refused_rather_than_guessed() {
    // Re-reading the current revision here and writing anyway would accept every save and make the
    // check theatre. A caller that cannot say what it was looking at is not making an informed edit.
    let fixture = fixture("no-revision");
    fixture.bind();

    let refused = fixture
        .api
        .save_setting("customInstructionsAboutUser", "edited", None);

    assert!(matches!(
        refused,
        Err(DesktopSettingsApplicationError::Personalization(_))
    ));
    assert_eq!(fixture.policy.saves.load(Ordering::SeqCst), 0);
}

#[test]
fn a_non_personalization_save_keeps_its_existing_path() {
    let fixture = fixture("unrelated-key");
    fixture.bind();

    let view = fixture
        .api
        .save_setting("applicationLanguage", "ko", None)
        .expect("ordinary setting");

    assert_eq!(view.settings.application_language().as_str(), "ko");
    assert_eq!(
        fixture.policy.saves.load(Ordering::SeqCst),
        0,
        "the policy is not consulted for a key it does not own"
    );
}

#[test]
fn an_invalid_personalization_value_is_refused_before_it_reaches_the_policy() {
    let fixture = fixture("invalid-value");
    fixture.bind();
    let too_long = "x".repeat(3_001);

    let refused = fixture
        .api
        .save_setting("customInstructionsAboutUser", &too_long, Some(3));

    assert!(matches!(
        refused,
        Err(DesktopSettingsApplicationError::Domain(_))
    ));
    assert_eq!(fixture.policy.saves.load(Ordering::SeqCst), 0);
}

#[test]
fn an_unreadable_policy_shows_the_legacy_values_and_accepts_no_write() {
    // Fail-closed in the direction that matters: displaying something is harmless, writing on top
    // of a policy nobody could read is not.
    let fixture = fixture("policy-unavailable");
    fixture
        .api
        .save_setting("customInstructionsAboutUser", "from the rows", None)
        .expect("legacy write");
    fixture.bind();
    *fixture.policy.unavailable.lock().expect("unavailable") = Some("unreachable".to_string());

    let view = fixture.api.get_settings().expect("settings");
    assert_eq!(
        view.settings.custom_instructions_about_user(),
        "from the rows"
    );
    assert_eq!(view.personalization_revision, None);

    let refused = fixture
        .api
        .save_setting("customInstructionsAboutUser", "edited", Some(3));
    assert!(matches!(
        refused,
        Err(DesktopSettingsApplicationError::Personalization(_))
    ));
}

#[test]
fn every_personalization_key_is_claimed_by_the_policy_and_no_other_key_is() {
    let fixture = fixture("ownership");
    fixture.bind();

    for key in PERSONALIZATION_KEYS {
        assert!(
            fixture
                .api
                .save_setting(key, "true", None)
                .is_err_and(|error| matches!(
                    error,
                    DesktopSettingsApplicationError::Personalization(_)
                        | DesktopSettingsApplicationError::Domain(_)
                )),
            "{key} must route to the policy, which refuses a save with no revision"
        );
    }
    // A neighbouring key that looks similar but was never personalization.
    assert!(fixture
        .api
        .save_setting("automaticContextCompactionEnabled", "false", None)
        .is_ok());
}
