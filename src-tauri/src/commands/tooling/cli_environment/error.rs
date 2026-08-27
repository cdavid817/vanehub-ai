//! The one place a CLI environment failure becomes something the frontend can act on.
//!
//! The frontend switches on `category` and localizes from it. It never parses `message`, so wording
//! can change without breaking a UI, and a message can stay terse without hiding anything the user
//! needs.
//!
//! `diagnosticId` carries the operation id when one exists. It is how a support conversation gets
//! from "it failed" to the unified log entry that says why, without putting a path or a process
//! fragment on screen.

use serde::Serialize;

use crate::contexts::tooling::cli::api::CliEnvironmentError;
use crate::platform::logging::redact_text;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliEnvironmentCommandError {
    /// Stable across releases. The frontend's localization keys off exactly this.
    pub(crate) category: String,
    /// Human-readable, already redacted, and never parsed by a caller.
    pub(crate) message: String,
    /// Whether preparing a fresh plan is the sensible next step, so the UI can offer "retry"
    /// instead of asking the user to guess.
    pub(crate) retryable_with_a_new_plan: bool,
    /// The operation whose log explains this, when the failure happened inside one.
    pub(crate) diagnostic_id: Option<String>,
}

impl std::fmt::Display for CliEnvironmentCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CliEnvironmentCommandError {}

impl From<CliEnvironmentError> for CliEnvironmentCommandError {
    fn from(error: CliEnvironmentError) -> Self {
        Self::of(error, None)
    }
}

impl CliEnvironmentCommandError {
    /// Maps a domain failure, attaching the operation it happened inside when there is one.
    pub(crate) fn of(error: CliEnvironmentError, diagnostic_id: Option<String>) -> Self {
        Self {
            category: error.category().to_string(),
            retryable_with_a_new_plan: error.is_retryable_with_a_new_plan(),
            // Redacted here rather than at serialization: the category is a structured value the
            // frontend matches on, and `redact_text`'s key heuristics would mangle it.
            message: redact_text(&error.to_string()),
            diagnostic_id,
        }
    }
}

/// Maps a failure that happened before any operation existed.
pub(crate) fn command_error(error: CliEnvironmentError) -> CliEnvironmentCommandError {
    CliEnvironmentCommandError::of(error, None)
}

#[cfg(test)]
#[path = "error_tests.rs"]
mod tests;
