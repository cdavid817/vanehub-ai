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
