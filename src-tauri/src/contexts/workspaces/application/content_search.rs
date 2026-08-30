//! Searching inside files, and being able to stop.
//!
//! Two things separate this from Quick Open. A path search reads directory entries; this one reads
//! file contents, which is orders of magnitude more work and the reason cancellation is a real
//! mechanism here rather than a frontend convention. And a match is a position rather than a
//! result: a reader wants the line they can go to, not the file it happens to be in.
//!
//! Matching is fixed-string and case-insensitive on both providers. Not because regular expressions
//! would be less useful, but because the two implementations are different engines — an in-process
//! scan here, ripgrep on a remote host — and a pattern language is the one thing two engines cannot
//! be made to agree about. A reader who gets different matches from the same query depending on
//! which machine the workspace is on has been handed a puzzle rather than a feature.

/// How many matches one search returns.
pub(crate) const MAX_CONTENT_MATCHES: usize = 200;

/// How many bytes of any one file are examined.
///
/// The same bound the preview uses, so a file too large to read is also a file too large to search.
/// A search that found a match a reader cannot then open would be worse than not finding it.
pub(crate) const MAX_SEARCHED_FILE_BYTES: u64 = 1024 * 1024;

/// How much of a matching line travels back.
///
/// Bounded around the match rather than the whole line: a minified bundle has lines megabytes long,
/// and one of them would be the entire response.
pub(crate) const MAX_SNIPPET_CHARS: usize = 200;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorkspaceContentSearchRequest {
    pub(crate) query: String,
    /// Issued by the caller.
    ///
    /// A caller-chosen id rather than one this side returns: an id that arrived with the answer
    /// would be useless for cancelling the search that produced it, which is the only thing anybody
    /// wants to cancel.
    pub(crate) search_id: String,
    pub(crate) limit: Option<usize>,
}

/// One position in one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceContentMatch {
    pub(crate) path: String,
    /// 1-based, because that is what every editor and every error message uses.
    pub(crate) line: u32,
    /// 1-based, counted in characters rather than bytes — a byte column is meaningless to a reader
    /// looking at a line with an accented character in it.
    pub(crate) column: u32,
    /// A bounded, control-free slice of the matching line.
    pub(crate) snippet: String,
    /// Whether the line was cut to fit. Set independently of the search's own bound, because a
    /// complete result made of trimmed lines is still complete.
    pub(crate) snippet_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceContentSearchResult {
    pub(crate) coverage: super::inspection::WorkspaceSearchCoverage,
    pub(crate) matches: Vec<WorkspaceContentMatch>,
}

/// A content search answer together with the registration that produced it.
///
/// Separate from the result because a provider cannot know this. A provider is handed a query and a
/// token and returns what it found; whether that answer is still the one anybody is waiting for is a
/// fact about the registry, and it is only knowable at the moment the answer comes back.
///
/// The generation travels all the way to the frontend rather than being consumed here. A caller that
/// only saw "superseded" would still have to guess whether the delivery it is holding is older or
/// newer than the one already on screen — two answers can be in flight, and arrival order is not
/// issue order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceContentSearchDelivery {
    /// Which registration under the search id produced this.
    pub(crate) generation: u64,
    pub(crate) result: WorkspaceContentSearchResult,
}

/// What a finished search is allowed to hand back, given whether it is still the current one.
///
/// Asked here rather than at the call site so the rule has one statement. A superseded generation's
/// matches are matches for a query the reader has already replaced; passing them on would invite
/// whoever receives them to render text nobody searched for, and the coverage would say `complete`
/// while describing a different question.
///
/// The generation goes out either way. A receiver holding two answers needs to know which is newer,
/// and "this one was stale" does not tell it whether what is already on screen was staler still.
pub(crate) fn deliver(
    registration: &super::search_cancellation::SearchRegistration,
    result: WorkspaceContentSearchResult,
) -> WorkspaceContentSearchDelivery {
    WorkspaceContentSearchDelivery {
        generation: registration.generation().value(),
        result: if registration.is_current() {
            result
        } else {
            WorkspaceContentSearchResult {
                coverage: super::inspection::WorkspaceSearchCoverage::stopped(
                    super::inspection_budget::WorkspaceInspectionReason::Superseded,
                ),
                matches: Vec::new(),
            }
        },
    }
}

/// A bounded, control-free slice of a line, and where the match starts in it.
///
/// Control characters are removed rather than escaped. They are not content a reader is searching
/// for, and an ANSI escape reaching a terminal-styled panel would be a match that repaints the
/// surrounding interface.
///
/// Nothing here attempts to detect and blank secrets, and that is deliberate. A search is a request
/// to be shown what a file contains; one that silently replaced the matched text would answer a
/// question the reader did not ask, and they can already open the same file in the preview beside
/// it. The bound and the control-character strip are about safety of *rendering*, not about hiding
/// what was found.
pub(crate) fn safe_snippet(line: &str, match_start_chars: usize) -> (String, bool, u32) {
    let cleaned: Vec<char> = line
        .chars()
        .map(|character| if character == '\t' { ' ' } else { character })
        .filter(|character| !character.is_control())
        .collect();

    if cleaned.len() <= MAX_SNIPPET_CHARS {
        return (
            cleaned.into_iter().collect(),
            false,
            match_start_chars as u32 + 1,
        );
    }

    // Centred on the match, so a hit near the end of a long line is still visible. A snippet that
    // always started at column one would show the first two hundred characters of a minified
    // bundle and never the thing that matched.
    let half = MAX_SNIPPET_CHARS / 2;
    let start = match_start_chars
        .saturating_sub(half)
        .min(cleaned.len().saturating_sub(MAX_SNIPPET_CHARS));
    let snippet: String = cleaned.iter().skip(start).take(MAX_SNIPPET_CHARS).collect();
    (snippet, true, match_start_chars as u32 + 1)
}

#[cfg(test)]
mod tests {
    use super::super::inspection::{WorkspaceSearchCoverage, WorkspaceSearchCoverageState};
    use super::super::search_cancellation::WorkspaceSearchCancellation;
    use super::*;
    use std::sync::Arc;

    fn one_match() -> WorkspaceContentSearchResult {
        WorkspaceContentSearchResult {
            coverage: WorkspaceSearchCoverage::complete(),
            matches: vec![WorkspaceContentMatch {
                path: "src/main.rs".to_string(),
                line: 1,
                column: 1,
                snippet: "let needle = 1;".to_string(),
                snippet_truncated: false,
            }],
        }
    }

    #[test]
    fn a_current_search_delivers_what_it_found() {
        let registry = Arc::new(WorkspaceSearchCancellation::default());
        let registration = registry.begin("content-1");

        let delivery = deliver(&registration, one_match());

        assert_eq!(delivery.generation, registration.generation().value());
        assert_eq!(delivery.result, one_match());
    }

    /// The defect this rule exists for.
    ///
    /// A finishes after B replaced it under the same id. Its matches are matches for a query the
    /// reader has already retyped, and delivering them puts text nobody searched for on screen under
    /// a `complete` coverage — the one combination that reads as an authoritative answer.
    #[test]
    fn a_superseded_search_delivers_no_matches_and_says_why() {
        let registry = Arc::new(WorkspaceSearchCancellation::default());
        let stale = registry.begin("content-1");
        let _current = registry.begin("content-1");

        let delivery = deliver(&stale, one_match());

        assert!(delivery.result.matches.is_empty());
        assert_eq!(
            delivery.result.coverage.state,
            WorkspaceSearchCoverageState::Partial
        );
        assert_eq!(delivery.result.coverage.reason_code, Some("superseded"));
    }

    /// Arrival order is not issue order, so a receiver cannot use "most recent response wins".
    #[test]
    fn both_deliveries_carry_the_generation_that_produced_them() {
        let registry = Arc::new(WorkspaceSearchCancellation::default());
        let stale = registry.begin("content-1");
        let current = registry.begin("content-1");

        let late = deliver(&stale, one_match());
        let fresh = deliver(&current, one_match());

        // The stale answer is not merely flagged: it is comparable, so a receiver holding the newer
        // one already can tell that this is the older without having to remember what it asked.
        assert!(late.generation < fresh.generation);
    }

    #[test]
    fn a_short_line_is_returned_whole_with_a_one_based_column() {
        let (snippet, truncated, column) = safe_snippet("let needle = 1;", 4);

        assert_eq!(snippet, "let needle = 1;");
        assert!(!truncated);
        assert_eq!(column, 5);
    }

    #[test]
    fn a_long_line_is_cut_around_the_match_rather_than_from_the_start() {
        let padding = "x".repeat(1_000);
        let line = format!("{padding}needle{padding}");

        let (snippet, truncated, _) = safe_snippet(&line, 1_000);

        assert!(truncated);
        assert_eq!(snippet.chars().count(), MAX_SNIPPET_CHARS);
        // A snippet that always started at column one would show the first two hundred characters
        // of a minified bundle and never the thing that matched.
        assert!(snippet.contains("needle"));
    }

    #[test]
    fn a_match_at_the_very_end_of_a_long_line_is_still_visible() {
        let line = format!("{}needle", "x".repeat(1_000));

        let (snippet, truncated, _) = safe_snippet(&line, 1_000);

        assert!(truncated);
        // The window is clamped to the end of the line rather than running past it, which would
        // otherwise return a short snippet that stops just before the match.
        assert!(snippet.ends_with("needle"));
    }

    #[test]
    fn control_characters_are_removed_and_tabs_become_spaces() {
        let (snippet, _, _) = safe_snippet("a\tb\u{1b}[31mc\u{7}", 0);

        // Tabs are content a reader recognises as spacing; the rest are not content at all, and an
        // ANSI escape reaching a styled panel would repaint the interface around the match.
        assert_eq!(snippet, "a b[31mc");
    }
}
