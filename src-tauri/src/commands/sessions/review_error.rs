use crate::commands::error::CommandError;
use crate::contexts::sessions::application::ReviewApplicationError;

pub(crate) fn map_review_error(error: ReviewApplicationError) -> CommandError {
    match error {
        ReviewApplicationError::NotFound(_) => CommandError::validation("review not found"),
        ReviewApplicationError::CommentNotFound(_) => {
            CommandError::validation("review comment not found")
        }
        ReviewApplicationError::NoSelectedComments => {
            CommandError::validation("no review comments selected")
        }
        ReviewApplicationError::StaleAcknowledgementRequired => {
            CommandError::validation("stale review acknowledgement required")
        }
        // The stable reason code from the change's error taxonomy, verbatim, because the
        // frontend matches on it. Which of the three witnesses failed does not cross: the caller's
        // next move is the same for all three, and the distinction is for the message the Review
        // Center renders from state it already has.
        ReviewApplicationError::StaleWitness(_) => CommandError::conflict("stale_witness"),
        ReviewApplicationError::InvalidActionOutput => {
            CommandError::validation("invalid review action output")
        }
        ReviewApplicationError::Domain(_) => CommandError::validation("invalid review input"),
        ReviewApplicationError::Repository(_) | ReviewApplicationError::Feedback(_) => {
            CommandError::storage("review operation failed")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reason code the frontend matches on, as the whole value rather than as a substring of
    /// a sentence. `stale_witness` is one of the change's stable codes; a message that merely
    /// contained it would make every caller parse prose to find it.
    #[test]
    fn a_stale_witness_crosses_as_its_reason_code() {
        use crate::commands::error::CommandErrorCategory;
        use crate::contexts::sessions::application::StaleReviewWitness;

        for witness in [
            StaleReviewWitness::Snapshot,
            StaleReviewWitness::File,
            StaleReviewWitness::Hunk,
        ] {
            let error = map_review_error(ReviewApplicationError::StaleWitness(witness));
            assert_eq!(serde_json::to_string(&error).unwrap(), "\"stale_witness\"");
            // Conflict, not validation: the request was well formed and the state moved under it.
            assert_eq!(error.category(), CommandErrorCategory::Conflict);
        }
    }

    #[test]
    fn repository_details_do_not_cross_the_command_boundary() {
        let error = map_review_error(ReviewApplicationError::Repository(
            "/private/workspace/token=secret".into(),
        ));
        let serialized = serde_json::to_string(&error).unwrap();
        assert!(!serialized.contains("private"));
        assert!(!serialized.contains("secret"));
    }
}
