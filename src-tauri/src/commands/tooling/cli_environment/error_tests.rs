// Included through `#[path]` from error.rs.
use super::*;

use crate::contexts::tooling::cli::domain::catalog::CliCatalogUnavailableReason;

fn every_variant() -> Vec<CliEnvironmentError> {
    vec![
        CliEnvironmentError::UnknownTool {
            agent_id: "nope".to_string(),
        },
        CliEnvironmentError::UnsupportedSource {
            agent_id: "codex-cli".to_string(),
            source_id: "winget".to_string(),
        },
        CliEnvironmentError::UnsupportedAction {
            agent_id: "claude-code".to_string(),
            source_id: "winget".to_string(),
            action: "downgrade",
        },
        CliEnvironmentError::InvalidVersion {
            source_id: "npm".to_string(),
            value: "9.9.9".to_string(),
        },
        CliEnvironmentError::CatalogUnavailable {
            source_id: "npm".to_string(),
            reason: CliCatalogUnavailableReason::QueryFailed,
        },
        CliEnvironmentError::PlanExpired,
        CliEnvironmentError::PlanStale,
        CliEnvironmentError::PlanConsumed,
        CliEnvironmentError::PlanRevisionMismatch {
            expected: 1,
            actual: 2,
        },
        CliEnvironmentError::PlanNotFound,
        CliEnvironmentError::MissingDependency {
            dependency: "npm".to_string(),
        },
        CliEnvironmentError::ElevationRequired,
        CliEnvironmentError::OperationConflict {
            agent_id: "claude-code".to_string(),
        },
        CliEnvironmentError::RuntimeUnsupported,
        CliEnvironmentError::SourceUnavailable {
            source_id: "winget".to_string(),
        },
        CliEnvironmentError::Validation("bad input".to_string()),
        CliEnvironmentError::Storage("write failed".to_string()),
        CliEnvironmentError::Process("exit 1".to_string()),
    ]
}

#[test]
fn every_failure_crosses_with_a_stable_category() {
    for error in every_variant() {
        let expected = error.category().to_string();
        let mapped = command_error(error);
        assert_eq!(mapped.category, expected);
        assert!(!mapped.message.is_empty());
    }
}

#[test]
fn the_serialized_shape_is_what_the_frontend_switches_on() {
    let value =
        serde_json::to_value(command_error(CliEnvironmentError::PlanStale)).expect("serializes");

    assert_eq!(value["category"], "plan-stale");
    assert_eq!(value["retryableWithANewPlan"], true);
    assert_eq!(value["diagnosticId"], serde_json::Value::Null);
    // The message is present for a human reading a log, and is not what the UI matches on.
    assert!(value["message"]
        .as_str()
        .expect("message")
        .contains("environment changed"));
}

#[test]
fn only_plan_failures_advertise_a_retry_with_a_new_plan() {
    for error in every_variant() {
        let category = error.category().to_string();
        let mapped = command_error(error);
        let expected = category.starts_with("plan-") && category != "plan-not-found";
        assert_eq!(mapped.retryable_with_a_new_plan, expected, "{category}");
    }
}

#[test]
fn a_diagnostic_id_points_at_the_operation_whose_log_explains_it() {
    let mapped = CliEnvironmentCommandError::of(
        CliEnvironmentError::Storage("write failed".to_string()),
        Some("op-42".to_string()),
    );
    let value = serde_json::to_value(&mapped).expect("serializes");

    assert_eq!(value["diagnosticId"], "op-42");
    assert_eq!(value["category"], "storage");
}

#[test]
fn no_mapped_message_carries_a_host_path_or_a_secret() {
    // The domain error type is command-safe by construction; this asserts the mapper does not
    // reintroduce anything, and that redaction runs on the free-text variants.
    for error in every_variant() {
        let message = command_error(error).message;
        assert!(!message.contains('\\'), "{message}");
        assert!(!message.contains("/home/"), "{message}");
        assert!(!message.contains("C:"), "{message}");
    }

    let leaky = command_error(CliEnvironmentError::Process(
        "Authorization: Bearer sk-ant-secret-value".to_string(),
    ));
    assert!(!leaky.message.contains("sk-ant-secret-value"));
}

#[test]
fn a_category_survives_redaction_unmangled() {
    // Redaction runs on the message only. A category is a structured value the frontend matches
    // exactly, and `redact_text`'s key heuristics would happily rewrite one.
    let mapped = command_error(CliEnvironmentError::MissingDependency {
        dependency: "npm".to_string(),
    });
    assert_eq!(mapped.category, "missing-dependency");
}
