use super::{
    record_cleanup, record_drift, record_lifecycle, record_save, record_secret_mutation,
    record_validation_failure,
};
use crate::contexts::tooling::skills::application::{
    SkillApplicationError, SkillLogAction, SkillLogEvent, SkillLogLevel, SkillLoggingPort,
};
use crate::contexts::tooling::skills::domain::{SkillConfigDrift, SkillConfigScope};
use crate::contexts::tooling::skills::infrastructure::{SecretRecovery, SkillConfigurationError};
use std::sync::Mutex;

const SECRET: &str = "sk-log-secret";
const WORKSPACE: &str = "/private/customer-alpha/checkout";

#[derive(Default)]
struct RecordingLogging {
    events: Mutex<Vec<SkillLogEvent>>,
}

impl RecordingLogging {
    fn rendered(&self) -> String {
        format!("{:?}", self.events.lock().expect("events"))
    }

    fn single(&self) -> SkillLogEvent {
        let events = self.events.lock().expect("events");
        assert_eq!(events.len(), 1, "expected exactly one event");
        events[0].clone()
    }
}

impl SkillLoggingPort for RecordingLogging {
    fn record(&self, event: &SkillLogEvent) -> Result<(), SkillApplicationError> {
        self.events.lock().expect("events").push(event.clone());
        Ok(())
    }
}

#[test]
fn a_save_event_names_the_properties_but_not_the_workspace_path() {
    let logging = RecordingLogging::default();

    record_save(
        &logging,
        "configured-skill",
        SkillConfigScope::Project,
        WORKSPACE,
        4,
        &["endpoint".to_string(), "retries".to_string()],
        &["api_key".to_string()],
    );

    let event = logging.single();
    assert_eq!(event.action, SkillLogAction::ConfigurationSave);
    assert_eq!(
        event.context.get("scope").map(String::as_str),
        Some("project")
    );
    assert_eq!(event.context.get("revision").map(String::as_str), Some("4"));
    assert_eq!(
        event.context.get("properties").map(String::as_str),
        Some("endpoint,retries")
    );
    // Presence only: a workspace path can name a customer or a private repository.
    assert_eq!(
        event.context.get("workspace").map(String::as_str),
        Some("present")
    );
    assert!(!logging.rendered().contains("customer-alpha"));
}

#[test]
fn a_validation_failure_records_a_code_and_a_property_not_the_offending_value() {
    let logging = RecordingLogging::default();

    record_validation_failure(
        &logging,
        "configured-skill",
        &SkillConfigurationError::InvalidValue {
            key: "retries".to_string(),
            reason: format!("expected an integer, found {SECRET}"),
        },
    );

    let event = logging.single();
    assert_eq!(event.level, SkillLogLevel::Warn);
    assert_eq!(
        event.context.get("outcome").map(String::as_str),
        Some("invalid-value")
    );
    assert_eq!(
        event.context.get("property").map(String::as_str),
        Some("retries")
    );
    // The reason is built from a lower-level message that quotes the input, so it is dropped.
    assert!(!logging.rendered().contains(SECRET));
}

#[test]
fn a_secret_mutation_records_the_intent_and_property_only() {
    let logging = RecordingLogging::default();

    record_secret_mutation(
        &logging,
        "configured-skill",
        SkillConfigScope::User,
        "",
        "replace",
        "api_key",
    );

    let event = logging.single();
    assert_eq!(event.action, SkillLogAction::ConfigurationSecretMutation);
    assert_eq!(
        event.context.get("intent").map(String::as_str),
        Some("replace")
    );
    assert_eq!(
        event.context.get("property").map(String::as_str),
        Some("api_key")
    );
    assert_eq!(
        event.context.get("workspace").map(String::as_str),
        Some("none")
    );
}

#[test]
fn drift_is_a_warning_and_carries_a_short_schema_witness() {
    let logging = RecordingLogging::default();
    let hash = "a".repeat(64);

    record_drift(
        &logging,
        "configured-skill",
        SkillConfigDrift::MigrationRequired,
        &hash,
    );

    let event = logging.single();
    assert_eq!(event.level, SkillLogLevel::Warn);
    assert_eq!(
        event.context.get("drift").map(String::as_str),
        Some("migration-required")
    );
    assert_eq!(
        event.context.get("schema").map(String::as_str),
        Some("aaaaaaaaaaaa")
    );
}

#[test]
fn an_incomplete_cleanup_is_an_error_naming_the_properties_left_behind() {
    let logging = RecordingLogging::default();

    record_cleanup(
        &logging,
        "configured-skill",
        &SecretRecovery::Incomplete {
            properties: vec!["api_key".to_string()],
        },
    );

    let event = logging.single();
    assert_eq!(event.level, SkillLogLevel::Error);
    assert_eq!(
        event.context.get("outcome").map(String::as_str),
        Some("incomplete")
    );
    assert_eq!(
        event.context.get("properties").map(String::as_str),
        Some("api_key")
    );
}

#[test]
fn lifecycle_events_cover_reset_reconcile_delete_and_retention() {
    let logging = RecordingLogging::default();

    for action in [
        SkillLogAction::ConfigurationReset,
        SkillLogAction::ConfigurationReconcile,
        SkillLogAction::ConfigurationDelete,
        SkillLogAction::ConfigurationRetention,
    ] {
        record_lifecycle(&logging, action, "configured-skill", "applied");
    }

    let events = logging.events.lock().expect("events");
    assert_eq!(events.len(), 4);
    assert!(events
        .iter()
        .all(|event| event.skill_id.as_deref() == Some("configured-skill")));
    assert!(events
        .iter()
        .all(|event| event.context.get("outcome").map(String::as_str) == Some("applied")));
    // Each action carries its own stable wire name rather than a shared generic one.
    let names = events
        .iter()
        .map(|event| event.action.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "configuration-reset",
            "configuration-reconcile",
            "configuration-delete",
            "configuration-retention",
        ]
    );
}

#[test]
fn a_logging_failure_does_not_fail_the_operation_it_describes() {
    struct FailingLogging;
    impl SkillLoggingPort for FailingLogging {
        fn record(&self, _event: &SkillLogEvent) -> Result<(), SkillApplicationError> {
            Err(SkillApplicationError::Repository(
                "log sink down".to_string(),
            ))
        }
    }

    // The call returns normally; a save must not be undone because its audit line could not be
    // written, and the unified log service reports its own write failures.
    record_lifecycle(
        &FailingLogging,
        SkillLogAction::ConfigurationDelete,
        "configured-skill",
        "applied",
    );
}
