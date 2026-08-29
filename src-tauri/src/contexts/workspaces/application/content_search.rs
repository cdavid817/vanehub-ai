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

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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

/// Which searches have been asked to stop.
///
/// A registry rather than a token passed down, because the thing that cancels a search is a
/// different command from the one running it: by the time a reader presses Escape, the search is
/// already inside a walk on the blocking pool, and the only way to reach it is a flag it polls.
///
/// Entries are removed when their search ends, and a cancel for a search that already finished is
/// silently accepted — the caller cannot know which happened first, and refusing would make an
/// ordinary race look like an error.
#[derive(Default)]
pub(crate) struct WorkspaceSearchCancellation {
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl WorkspaceSearchCancellation {
    /// Registers a search and hands back the flag it should poll.
    ///
    /// Registering *before* the work starts is what makes a cancel that arrives immediately still
    /// land: a flag created when the walk begins would miss every cancel sent in the window
    /// between the request leaving the frontend and the first directory being read.
    pub(crate) fn begin(&self, search_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut running) = self.running.lock() {
            // Replacing an id already in flight cancels the old one. A caller reusing an id has
            // superseded its own search, and leaving both running would spend a machine's effort on
            // an answer nobody will read.
            if let Some(previous) = running.insert(search_id.to_string(), flag.clone()) {
                previous.store(true, Ordering::Relaxed);
            }
        }
        flag
    }

    pub(crate) fn finish(&self, search_id: &str) {
        if let Ok(mut running) = self.running.lock() {
            running.remove(search_id);
        }
    }

    /// Asks a search to stop. Returns whether one was running under that id.
    pub(crate) fn cancel(&self, search_id: &str) -> bool {
        let Ok(running) = self.running.lock() else {
            return false;
        };
        match running.get(search_id) {
            Some(flag) => {
                flag.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
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
    use super::*;

    #[test]
    fn a_cancel_reaches_a_search_that_registered_first() {
        let registry = WorkspaceSearchCancellation::default();
        let flag = registry.begin("search-1");

        assert!(registry.cancel("search-1"));
        assert!(flag.load(Ordering::Relaxed));
    }

    #[test]
    fn a_cancel_for_a_search_that_already_ended_is_accepted_quietly() {
        let registry = WorkspaceSearchCancellation::default();
        registry.begin("search-1");
        registry.finish("search-1");

        // False means "there was nothing to stop", not "you did something wrong". A caller cannot
        // know whether their cancel beat the search's own completion, and turning that ordinary
        // race into an error would put a failure on screen for a keystroke that worked.
        assert!(!registry.cancel("search-1"));
    }

    #[test]
    fn reusing_an_id_cancels_the_search_it_replaces() {
        let registry = WorkspaceSearchCancellation::default();
        let first = registry.begin("search-1");
        let second = registry.begin("search-1");

        // A caller reusing an id has superseded its own search. Leaving both running would spend a
        // machine's effort producing an answer nobody will read.
        assert!(first.load(Ordering::Relaxed));
        assert!(!second.load(Ordering::Relaxed));
    }

    /// The A/B defect, written down before it is repaired.
    ///
    /// A registry keyed by id alone cannot tell whose registration it is removing. A finishes after
    /// B replaced it, `finish` removes the id unconditionally, and B — which is still running — is
    /// no longer reachable by a cancel. This test asserts the *current* behaviour so the repair has
    /// something to invert rather than something to write from scratch.
    #[test]
    fn characterizes_an_older_search_removing_its_successors_registration() {
        let registry = WorkspaceSearchCancellation::default();

        let a = registry.begin("search-1");
        let b = registry.begin("search-1");
        assert!(a.load(Ordering::Relaxed), "B supersedes A");

        // A's owning future reaches its own cleanup after B has already registered.
        registry.finish("search-1");

        // The defect: B is still running and can no longer be stopped.
        assert!(
            !registry.cancel("search-1"),
            "the registry no longer knows about B"
        );
        assert!(
            !b.load(Ordering::Relaxed),
            "B keeps running with no way left to reach it"
        );
    }

    /// The drop half of the same defect.
    ///
    /// Nothing signals A's flag when A's owning future is aborted rather than completed: the flag
    /// only moves when some later `begin` happens to replace it, so an aborted search runs to its
    /// natural end on the blocking pool.
    #[test]
    fn characterizes_an_aborted_owner_leaving_its_worker_running() {
        let registry = WorkspaceSearchCancellation::default();

        let flag = registry.begin("search-1");
        // The owning future is dropped. There is no guard, so nothing happens.
        drop(registry);

        assert!(
            !flag.load(Ordering::Relaxed),
            "an aborted owner never signals its worker"
        );
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
