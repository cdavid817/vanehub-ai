#![cfg_attr(not(test), allow(dead_code))]

use super::configuration_secrets::SecretRecovery;
use super::configuration_service::SkillConfigurationError;
use crate::contexts::tooling::skills::application::{
    SkillLogAction, SkillLogEvent, SkillLogLevel, SkillLoggingPort,
};
use crate::contexts::tooling::skills::domain::{SkillConfigDrift, SkillConfigScope};
use crate::platform::clock::SystemClock;
use std::collections::BTreeMap;

/// Every event is assembled from a fixed vocabulary rather than from caller-supplied text. A
/// configuration value or a credential alias cannot reach a log line if no code path can put one
/// into an event in the first place, which is a stronger guarantee than redacting on the way out.
pub(crate) fn configuration_event(
    action: SkillLogAction,
    level: SkillLogLevel,
    skill_id: &str,
    context: BTreeMap<String, String>,
) -> SkillLogEvent {
    SkillLogEvent {
        action,
        level,
        skill_id: Some(skill_id.to_string()),
        message: action.as_str().to_string(),
        timestamp: SystemClock.rfc3339(),
        context,
    }
}

fn scoped(scope: SkillConfigScope, workspace_identity: &str) -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    context.insert("scope".to_string(), scope.as_str().to_string());
    // Presence, not the path: a workspace path can name a customer or a private repository.
    context.insert(
        "workspace".to_string(),
        if workspace_identity.is_empty() {
            "none".to_string()
        } else {
            "present".to_string()
        },
    );
    context
}

pub(crate) fn record_save(
    logging: &dyn SkillLoggingPort,
    skill_id: &str,
    scope: SkillConfigScope,
    workspace_identity: &str,
    stored_revision: u64,
    changed_keys: &[String],
    secret_keys: &[String],
) {
    let mut context = scoped(scope, workspace_identity);
    context.insert("revision".to_string(), stored_revision.to_string());
    // Property names are declared in the Skill package and are already visible to anyone who can
    // read it, so naming which ones changed is safe and is what makes an audit trail useful.
    context.insert("properties".to_string(), joined(changed_keys));
    context.insert("secrets".to_string(), joined(secret_keys));
    emit(
        logging,
        configuration_event(
            SkillLogAction::ConfigurationSave,
            SkillLogLevel::Info,
            skill_id,
            context,
        ),
    );
}

pub(crate) fn record_validation_failure(
    logging: &dyn SkillLoggingPort,
    skill_id: &str,
    error: &SkillConfigurationError,
) {
    let mut context = BTreeMap::new();
    context.insert("outcome".to_string(), error_code(error).to_string());
    if let Some(key) = error_property(error) {
        context.insert("property".to_string(), key);
    }
    emit(
        logging,
        configuration_event(
            SkillLogAction::ConfigurationValidate,
            SkillLogLevel::Warn,
            skill_id,
            context,
        ),
    );
}

pub(crate) fn record_secret_mutation(
    logging: &dyn SkillLoggingPort,
    skill_id: &str,
    scope: SkillConfigScope,
    workspace_identity: &str,
    intent: &'static str,
    property_key: &str,
) {
    let mut context = scoped(scope, workspace_identity);
    context.insert("intent".to_string(), intent.to_string());
    context.insert("property".to_string(), property_key.to_string());
    emit(
        logging,
        configuration_event(
            SkillLogAction::ConfigurationSecretMutation,
            SkillLogLevel::Info,
            skill_id,
            context,
        ),
    );
}

pub(crate) fn record_drift(
    logging: &dyn SkillLoggingPort,
    skill_id: &str,
    drift: SkillConfigDrift,
    schema_hash: &str,
) {
    let mut context = BTreeMap::new();
    context.insert("drift".to_string(), drift.as_str().to_string());
    context.insert("schema".to_string(), short_hash(schema_hash));
    emit(
        logging,
        configuration_event(
            SkillLogAction::ConfigurationDrift,
            match drift {
                SkillConfigDrift::Compatible => SkillLogLevel::Info,
                _ => SkillLogLevel::Warn,
            },
            skill_id,
            context,
        ),
    );
}

pub(crate) fn record_lifecycle(
    logging: &dyn SkillLoggingPort,
    action: SkillLogAction,
    skill_id: &str,
    outcome: &'static str,
) {
    let mut context = BTreeMap::new();
    context.insert("outcome".to_string(), outcome.to_string());
    emit(
        logging,
        configuration_event(action, SkillLogLevel::Info, skill_id, context),
    );
}

pub(crate) fn record_cleanup(
    logging: &dyn SkillLoggingPort,
    skill_id: &str,
    recovery: &SecretRecovery,
) {
    let mut context = BTreeMap::new();
    match recovery {
        SecretRecovery::Clean => {
            context.insert("outcome".to_string(), "clean".to_string());
        }
        SecretRecovery::Incomplete { properties } => {
            context.insert("outcome".to_string(), "incomplete".to_string());
            context.insert("properties".to_string(), joined(properties));
        }
    }
    emit(
        logging,
        configuration_event(
            SkillLogAction::ConfigurationCleanup,
            match recovery {
                SecretRecovery::Clean => SkillLogLevel::Info,
                SecretRecovery::Incomplete { .. } => SkillLogLevel::Error,
            },
            skill_id,
            context,
        ),
    );
}

/// A hash is not sensitive, but a full one makes log lines unreadable and invites treating it as
/// an identifier to look up rather than to compare.
fn short_hash(hash: &str) -> String {
    hash.chars().take(12).collect()
}

fn joined(keys: &[String]) -> String {
    let mut sorted = keys.to_vec();
    sorted.sort();
    sorted.join(",")
}

fn error_code(error: &SkillConfigurationError) -> &'static str {
    match error {
        SkillConfigurationError::UnknownProperty { .. } => "unknown-property",
        SkillConfigurationError::NotConfigurable { .. } => "not-configurable",
        SkillConfigurationError::InvalidValue { .. } => "invalid-value",
        SkillConfigurationError::PayloadTooLarge { .. } => "payload-too-large",
        SkillConfigurationError::SchemaChanged { .. } => "schema-changed",
        SkillConfigurationError::BaseRevisionChanged { .. } => "base-revision-changed",
        SkillConfigurationError::Stale { .. } => "stale",
        SkillConfigurationError::InvalidWorkspace => "invalid-workspace",
        SkillConfigurationError::CredentialFailure { .. } => "credential-failure",
        SkillConfigurationError::RepositoryFailure { .. } => "repository-failure",
        SkillConfigurationError::RecoveryRequired { .. } => "recovery-required",
        SkillConfigurationError::ReconciliationIncomplete { .. } => "reconciliation-incomplete",
    }
}

/// Deliberately omits `InvalidValue`'s reason and `RepositoryFailure`'s reason: both are built
/// from lower-level messages that can quote the offending input.
fn error_property(error: &SkillConfigurationError) -> Option<String> {
    match error {
        SkillConfigurationError::UnknownProperty { key }
        | SkillConfigurationError::NotConfigurable { key }
        | SkillConfigurationError::InvalidValue { key, .. }
        | SkillConfigurationError::CredentialFailure { key, .. }
        | SkillConfigurationError::ReconciliationIncomplete { key } => Some(key.clone()),
        _ => None,
    }
}

fn emit(logging: &dyn SkillLoggingPort, event: SkillLogEvent) {
    // A failed log must not fail the operation it describes; the unified log service already
    // reports its own write failures.
    let _ = logging.record(&event);
}

#[cfg(test)]
#[path = "configuration_logging_tests.rs"]
mod tests;
