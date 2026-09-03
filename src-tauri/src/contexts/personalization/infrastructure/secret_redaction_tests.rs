//! The redaction the preview actually applies.
//!
//! Asserted against the real adapter rather than through the preview, because what is under test is
//! the platform rule this context reuses — the preview only routes text through it, and that is
//! asserted where the preview lives.

use super::secret_redaction::PlatformSecretRedaction;
use crate::contexts::personalization::application::SecretRedactionPort;

#[test]
fn a_credential_a_user_pasted_into_their_own_instructions_does_not_survive() {
    let redaction = PlatformSecretRedaction;

    let rendered = redaction.redact("my api_key=sk-live-01234567890abcdef please remember");

    assert!(
        !rendered.contains("sk-live-01234567890abcdef"),
        "{rendered}"
    );
    assert!(
        rendered.contains("please remember"),
        "ordinary text survives"
    );
}

#[test]
fn a_private_path_a_user_mentioned_does_not_survive() {
    let redaction = PlatformSecretRedaction;

    let rendered = redaction.redact("my notes live in D:/cdavid/private/notes.md");

    assert!(
        !rendered.contains("D:/cdavid/private/notes.md"),
        "{rendered}"
    );
}

#[test]
fn ordinary_instructions_pass_through_unchanged() {
    // Over-redaction is its own failure: a screen that mangled a user's own sentence would make the
    // preview useless for the thing it exists to do.
    let redaction = PlatformSecretRedaction;
    let ordinary = "Prefers concise answers and no preamble.";

    assert_eq!(redaction.redact(ordinary), ordinary);
}
