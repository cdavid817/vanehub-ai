//! Typed failures of the source-aware CLI use cases.
//!
//! Every variant carries a stable `category()`. The Tauri layer maps that category and nothing
//! else -- the frontend localizes from the category and never parses a message string, so wording
//! can change without breaking a UI.
//!
//! Messages are command-safe by construction: they name tools, sources, actions, and versions,
//! never paths outside a tool's own identity, credentials, or raw process output. Anything with
//! diagnostic value that cannot be shown lives in the unified log under a diagnostic id.

use thiserror::Error;

use crate::contexts::tooling::cli::domain::catalog::CliCatalogUnavailableReason;
use crate::contexts::tooling::cli::domain::plan::CliPlanRejection;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub(crate) enum CliEnvironmentError {
    #[error("unknown CLI tool: {agent_id}")]
    UnknownTool { agent_id: String },

    #[error("{agent_id} is not distributed through {source_id}")]
    UnsupportedSource { agent_id: String, source_id: String },

    #[error("{source_id} does not support {action} for {agent_id}")]
    UnsupportedAction {
        agent_id: String,
        source_id: String,
        action: &'static str,
    },

    #[error("{value} is not a version {source_id} offers")]
    InvalidVersion { source_id: String, value: String },

    #[error("no version catalog is available from {source_id}")]
    CatalogUnavailable {
        source_id: String,
        reason: CliCatalogUnavailableReason,
    },

    #[error("the action plan has expired; prepare a new one")]
    PlanExpired,

    #[error("the environment changed after this plan was prepared; prepare a new one")]
    PlanStale,

    #[error("the action plan has already been used; retrying requires a new plan")]
    PlanConsumed,

    #[error("the action plan was revised; reload it before executing")]
    PlanRevisionMismatch { expected: u32, actual: u32 },

    #[error("no action plan found")]
    PlanNotFound,

    // Declared because the contract enumerates them and the command layer already maps both to a
    // stable category. Nothing constructs them yet: the shipped adapters report elevation as a
    // plan precondition rather than a refusal, and no source declares a dependency to check.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "declared by the error contract; no adapter raises it yet"
        )
    )]
    #[error("{dependency} is required and was not found")]
    MissingDependency { dependency: String },

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "declared by the error contract; no adapter raises it yet"
        )
    )]
    #[error("this action requires elevated privileges")]
    ElevationRequired,

    #[error("another operation is already running for {agent_id}")]
    OperationConflict { agent_id: String },

    #[error("this action is not supported on the current platform")]
    RuntimeUnsupported,

    #[error("{source_id} is not available on this machine")]
    SourceUnavailable { source_id: String },

    #[error("{0}")]
    Validation(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("{0}")]
    Process(String),
}

impl CliEnvironmentError {
    /// The stable category the Tauri boundary serializes. Adding a variant without adding it here
    /// is a compile error, so a new failure mode cannot reach the frontend uncategorized.
    pub(crate) fn category(&self) -> &'static str {
        match self {
            Self::UnknownTool { .. } => "unknown-tool",
            Self::UnsupportedSource { .. } => "unsupported-source",
            Self::UnsupportedAction { .. } => "unsupported-action",
            Self::InvalidVersion { .. } => "invalid-version",
            Self::CatalogUnavailable { .. } => "catalog-unavailable",
            Self::PlanExpired => "plan-expired",
            Self::PlanStale => "plan-stale",
            Self::PlanConsumed => "plan-consumed",
            Self::PlanRevisionMismatch { .. } => "plan-revision-mismatch",
            Self::PlanNotFound => "plan-not-found",
            Self::MissingDependency { .. } => "missing-dependency",
            Self::ElevationRequired => "elevation-required",
            Self::OperationConflict { .. } => "operation-conflict",
            Self::RuntimeUnsupported => "runtime-unsupported",
            Self::SourceUnavailable { .. } => "source-unavailable",
            Self::Validation(_) => "validation",
            Self::Storage(_) => "storage",
            Self::Process(_) => "process",
        }
    }

    /// Whether preparing a fresh plan is the sensible next step. The UI turns this into a "Retry"
    /// affordance instead of asking the user to guess.
    pub(crate) fn is_retryable_with_a_new_plan(&self) -> bool {
        matches!(
            self,
            Self::PlanExpired
                | Self::PlanStale
                | Self::PlanConsumed
                | Self::PlanRevisionMismatch { .. }
        )
    }
}

impl From<CliPlanRejection> for CliEnvironmentError {
    fn from(rejection: CliPlanRejection) -> Self {
        match rejection {
            CliPlanRejection::Expired => Self::PlanExpired,
            CliPlanRejection::Stale => Self::PlanStale,
            CliPlanRejection::Consumed => Self::PlanConsumed,
            CliPlanRejection::RevisionMismatch { expected, actual } => {
                Self::PlanRevisionMismatch { expected, actual }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn every_variant_has_a_distinct_stable_category() {
        let mut categories = every_variant()
            .iter()
            .map(CliEnvironmentError::category)
            .collect::<Vec<_>>();
        let total = categories.len();
        categories.sort_unstable();
        categories.dedup();
        assert_eq!(
            categories.len(),
            total,
            "two variants share a category, so the frontend cannot tell them apart"
        );
    }

    #[test]
    fn categories_match_the_documented_command_safe_set() {
        let mut categories = every_variant()
            .iter()
            .map(CliEnvironmentError::category)
            .collect::<Vec<_>>();
        categories.sort_unstable();
        assert_eq!(
            categories,
            vec![
                "catalog-unavailable",
                "elevation-required",
                "invalid-version",
                "missing-dependency",
                "operation-conflict",
                "plan-consumed",
                "plan-expired",
                "plan-not-found",
                "plan-revision-mismatch",
                "plan-stale",
                "process",
                "runtime-unsupported",
                "source-unavailable",
                "storage",
                "unknown-tool",
                "unsupported-action",
                "unsupported-source",
                "validation",
            ]
        );
    }

    #[test]
    fn a_domain_plan_rejection_maps_onto_its_own_category() {
        assert_eq!(
            CliEnvironmentError::from(CliPlanRejection::Expired).category(),
            "plan-expired"
        );
        assert_eq!(
            CliEnvironmentError::from(CliPlanRejection::Stale).category(),
            "plan-stale"
        );
        assert_eq!(
            CliEnvironmentError::from(CliPlanRejection::Consumed).category(),
            "plan-consumed"
        );
        assert_eq!(
            CliEnvironmentError::from(CliPlanRejection::RevisionMismatch {
                expected: 3,
                actual: 4
            }),
            CliEnvironmentError::PlanRevisionMismatch {
                expected: 3,
                actual: 4
            }
        );
    }

    #[test]
    fn only_plan_failures_suggest_preparing_a_new_plan() {
        for error in every_variant() {
            let expected =
                error.category().starts_with("plan-") && error.category() != "plan-not-found";
            assert_eq!(
                error.is_retryable_with_a_new_plan(),
                expected,
                "{}",
                error.category()
            );
        }
    }

    #[test]
    fn messages_name_the_subject_without_leaking_host_detail() {
        let error = CliEnvironmentError::UnsupportedAction {
            agent_id: "claude-code".to_string(),
            source_id: "winget".to_string(),
            action: "downgrade",
        };
        let message = error.to_string();
        assert!(message.contains("winget"));
        assert!(message.contains("downgrade"));
        assert!(message.contains("claude-code"));

        // No message embeds a filesystem path, a home directory, or raw output.
        for error in every_variant() {
            let message = error.to_string();
            assert!(!message.contains('\\'), "{message}");
            assert!(!message.contains("/home/"), "{message}");
            assert!(!message.contains("C:"), "{message}");
        }
    }

    #[test]
    fn a_stale_plan_is_reported_as_an_environment_change_not_a_failure() {
        // The user did nothing wrong: something outside VaneHub moved. The message says so and
        // points at the fix.
        let message = CliEnvironmentError::PlanStale.to_string();
        assert!(message.contains("environment changed"));
        assert!(message.contains("prepare a new one"));
        assert!(CliEnvironmentError::PlanStale.is_retryable_with_a_new_plan());
    }
}
