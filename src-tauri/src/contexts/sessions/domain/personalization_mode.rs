use super::SessionsDomainError;

/// How much long-term personalization a session participates in.
///
/// The sessions context owns its own copy of this taxonomy rather than importing the
/// personalization context's. The two are deliberately separate: a session records what the user
/// chose, and personalization decides what that choice means, so translating at the composition
/// boundary is what keeps one context's vocabulary from becoming a dependency of the other's.
///
/// A hard restriction rather than another policy layer. It is applied last, after every scope has
/// resolved, and it can only narrow — no policy, override or default can widen a temporary session
/// back into long-term memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SessionPersonalizationMode {
    /// Everything the resolved policy allows.
    #[default]
    Standard,
    /// Workspace memories only. Global memories are excluded even where policy permits them.
    ProjectOnly,
    /// No long-term memory in any direction — reading, saving, extracting, proposing, or writing
    /// the retrieval index. Custom instructions still apply: a session the user asked not to
    /// retain is still their session, and how they want to be answered is not what the mode was
    /// meant to discard.
    Temporary,
}

impl SessionPersonalizationMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::ProjectOnly => "project-only",
            Self::Temporary => "temporary",
        }
    }

    /// Parses a stored or requested value.
    ///
    /// An empty value reads as `Standard` because that is what every session written before this
    /// column existed is: the migration backfills it, and a row that somehow escaped the backfill
    /// must still open. An unrecognised value is rejected rather than defaulted, because a mode
    /// this build does not understand is not safely interpretable as the most permissive one.
    pub(crate) fn parse(value: &str) -> Result<Self, SessionsDomainError> {
        match value.trim() {
            "" | "standard" => Ok(Self::Standard),
            "project-only" => Ok(Self::ProjectOnly),
            "temporary" => Ok(Self::Temporary),
            other => Err(SessionsDomainError::UnknownPersonalizationMode(
                other.to_string(),
            )),
        }
    }

    /// Project-only has nothing to scope to without a workspace, so creation must refuse rather
    /// than silently degrade. "Read everything global" is the one interpretation a project-isolated
    /// session must never become.
    pub(crate) fn requires_workspace(self) -> bool {
        matches!(self, Self::ProjectOnly)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_written_before_this_column_existed_reads_as_standard() {
        assert_eq!(
            SessionPersonalizationMode::parse("").expect("mode"),
            SessionPersonalizationMode::Standard
        );
    }

    /// Defaulting an unknown mode to `Standard` would silently widen a session a newer build
    /// created as temporary, which is the one direction this must never move.
    #[test]
    fn a_mode_this_build_does_not_know_is_refused_rather_than_widened() {
        assert!(SessionPersonalizationMode::parse("incognito").is_err());
    }

    #[test]
    fn every_mode_round_trips_through_its_stored_form() {
        for mode in [
            SessionPersonalizationMode::Standard,
            SessionPersonalizationMode::ProjectOnly,
            SessionPersonalizationMode::Temporary,
        ] {
            assert_eq!(
                SessionPersonalizationMode::parse(mode.as_str()).expect("mode"),
                mode
            );
        }
    }

    #[test]
    fn only_project_only_needs_a_workspace_to_mean_anything() {
        assert!(SessionPersonalizationMode::ProjectOnly.requires_workspace());
        assert!(!SessionPersonalizationMode::Standard.requires_workspace());
        assert!(!SessionPersonalizationMode::Temporary.requires_workspace());
    }
}
