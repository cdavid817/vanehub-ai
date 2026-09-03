use crate::commands::error::{CommandError, CommandErrorCategory};
use crate::contexts::personalization::application::PersonalizationApplicationError;
use crate::contexts::personalization::domain::ResetRefusal;

/// Maps one personalization failure onto the category the UI acts on.
///
/// Each category exists because the caller's correct response differs. A conflict means "keep the
/// user's draft and offer a reload"; unavailable means "try again once maintenance finishes"; a
/// validation error means "the input is wrong and the user can fix it". Folding any of them into a
/// generic infrastructure error would leave the screen guessing.
///
/// Storage messages are the only ones carrying lower-layer text, and they never reach the user: a
/// SQLite message or a filesystem path is a local diagnostic, and a memory directory sits inside a
/// user's home folder. What is returned in their place is a fixed sentence.
impl From<PersonalizationApplicationError> for CommandError {
    fn from(error: PersonalizationApplicationError) -> Self {
        match error {
            PersonalizationApplicationError::Domain(error) => Self::validation(error.to_string()),
            PersonalizationApplicationError::RevisionConflict(conflict) => Self::typed(
                CommandErrorCategory::Conflict,
                format!(
                    "personalization-revision-conflict: expected {}, stored {}",
                    conflict.expected, conflict.current
                ),
            ),
            PersonalizationApplicationError::NotFound => {
                Self::typed(CommandErrorCategory::NotFound, "personalization-not-found")
            }
            PersonalizationApplicationError::WorkspaceRequired => {
                Self::validation("This operation needs a workspace to be scoped to.".to_string())
            }
            PersonalizationApplicationError::ResetRefused(refusal) => Self::typed(
                CommandErrorCategory::Validation,
                format!(
                    "personalization-reset-refused: {}",
                    match refusal {
                        ResetRefusal::PhraseMismatch => "phrase-mismatch",
                        ResetRefusal::TokenExpired => "token-expired",
                        ResetRefusal::TokenScopeMismatch => "token-scope-mismatch",
                    }
                ),
            ),
            PersonalizationApplicationError::AmbiguousLegacyName { matches } => Self::typed(
                CommandErrorCategory::Conflict,
                format!("personalization-ambiguous-name: {matches} records share that name"),
            ),
            // Both mean "not now, try again", and the screen shows the same thing for each. They
            // stay distinct in the message so a report says which one happened.
            PersonalizationApplicationError::MaintenanceRequired => Self::typed(
                CommandErrorCategory::Unavailable,
                "personalization-maintenance-required",
            ),
            PersonalizationApplicationError::MaintenanceBusy => Self::typed(
                CommandErrorCategory::Unavailable,
                "personalization-maintenance-busy",
            ),
            // The message is deliberately dropped rather than redacted. Redaction removes what
            // looks like a secret; a path to a user's memory directory looks like neither a secret
            // nor a safe thing to display, and the screen has nothing to do with it either way.
            PersonalizationApplicationError::Storage(_) => Self::typed(
                CommandErrorCategory::Infrastructure,
                "personalization-storage-unavailable",
            ),
        }
    }
}
