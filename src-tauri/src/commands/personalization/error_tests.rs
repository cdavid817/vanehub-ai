//! What a personalization failure looks like by the time a screen sees it.

use super::super::error::{CommandError, CommandErrorCategory};
use crate::contexts::personalization::application::PersonalizationApplicationError;
use crate::contexts::personalization::domain::{
    PersonalizationDomainError, ResetRefusal, RevisionConflict,
};

fn mapped(error: PersonalizationApplicationError) -> CommandError {
    CommandError::from(error)
}

/// The category is what the screen acts on, so each failure has to arrive as itself.
///
/// A conflict means "keep the user's draft and offer a reload"; unavailable means "try again once
/// maintenance finishes"; a validation error means "the input is wrong and can be fixed". Folding
/// any of them into one infrastructure error would leave the screen guessing which it was.
#[test]
fn every_failure_arrives_as_the_category_its_caller_must_act_on() {
    let cases = [
        (
            mapped(PersonalizationApplicationError::RevisionConflict(
                RevisionConflict {
                    expected: 3,
                    current: 5,
                },
            )),
            CommandErrorCategory::Conflict,
        ),
        (
            mapped(PersonalizationApplicationError::NotFound),
            CommandErrorCategory::NotFound,
        ),
        (
            mapped(PersonalizationApplicationError::MaintenanceRequired),
            CommandErrorCategory::Unavailable,
        ),
        (
            mapped(PersonalizationApplicationError::MaintenanceBusy),
            CommandErrorCategory::Unavailable,
        ),
        (
            mapped(PersonalizationApplicationError::WorkspaceRequired),
            CommandErrorCategory::Validation,
        ),
        (
            mapped(PersonalizationApplicationError::ResetRefused(
                ResetRefusal::TokenExpired,
            )),
            CommandErrorCategory::Validation,
        ),
        (
            mapped(PersonalizationApplicationError::Domain(
                PersonalizationDomainError::MemoryFieldEmpty { field: "content" },
            )),
            CommandErrorCategory::Validation,
        ),
        (
            mapped(PersonalizationApplicationError::AmbiguousLegacyName { matches: 2 }),
            CommandErrorCategory::Conflict,
        ),
        (
            mapped(PersonalizationApplicationError::Storage(
                "irrelevant".to_string(),
            )),
            CommandErrorCategory::Infrastructure,
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.category(), expected, "{}", error.message());
    }
}

/// A storage message is a local diagnostic, and it does not travel.
///
/// It carries SQLite text and filesystem paths, and a memory directory sits inside a user's home
/// folder — so the message is dropped rather than redacted. Redaction removes what looks like a
/// secret; a home directory path looks like neither a secret nor a safe thing to display.
#[test]
fn a_storage_failure_never_carries_the_underlying_message() {
    let error = mapped(PersonalizationApplicationError::Storage(
        r"C:\Users\alice\AppData\Roaming\ai.vanehub.app\memory: disk I/O error".to_string(),
    ));

    assert_eq!(error.message(), "personalization-storage-unavailable");
    assert!(!error.message().contains("alice"));
    assert!(!error.message().contains("disk I/O"));
}

/// A conflict says which revisions disagreed.
///
/// Both numbers, because the screen has to tell the user their copy is behind and by how much —
/// and because a conflict with no numbers is indistinguishable from a generic failure in a report.
#[test]
fn a_conflict_reports_both_revisions() {
    let error = mapped(PersonalizationApplicationError::RevisionConflict(
        RevisionConflict {
            expected: 3,
            current: 5,
        },
    ));

    assert!(error
        .message()
        .contains("personalization-revision-conflict"));
    assert!(error.message().contains('3'));
    assert!(error.message().contains('5'));
}

/// The two maintenance states stay distinguishable in the message.
///
/// A screen shows the same thing for each, but a bug report has to say which happened: "migration
/// has not finished" and "another holder owns the lock" lead to different investigations.
#[test]
fn the_two_maintenance_states_are_reported_separately() {
    assert_eq!(
        mapped(PersonalizationApplicationError::MaintenanceRequired).message(),
        "personalization-maintenance-required"
    );
    assert_eq!(
        mapped(PersonalizationApplicationError::MaintenanceBusy).message(),
        "personalization-maintenance-busy"
    );
}

/// Reset refusals arrive as codes, not prose.
///
/// The screen has to say something different for each — retype the phrase, start over, or preview
/// again — and matching on an English sentence would break the first time it was reworded.
#[test]
fn a_refused_reset_names_which_check_refused_it() {
    for (refusal, code) in [
        (ResetRefusal::PhraseMismatch, "phrase-mismatch"),
        (ResetRefusal::TokenExpired, "token-expired"),
        (ResetRefusal::TokenScopeMismatch, "token-scope-mismatch"),
    ] {
        assert_eq!(
            mapped(PersonalizationApplicationError::ResetRefused(refusal)).message(),
            format!("personalization-reset-refused: {code}")
        );
    }
}
