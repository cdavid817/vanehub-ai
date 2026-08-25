//! Continuing a directory listing without losing your place.
//!
//! A keyset cursor rather than an offset. An offset is a promise about how many entries came
//! before, and that promise breaks the moment a file is created: the next page skips an entry or
//! repeats one, and nothing in the answer says which happened. A key says "resume after this
//! entry", which stays true whatever else the directory does.
//!
//! The key is the ordering key, not the name. Entries sort directories first and then
//! case-insensitively, so resuming after a name alone would resume in the wrong half of the
//! listing — a directory called `zebra` sorts before a file called `alpha`.
//!
//! A cursor also carries the directory it was issued for and is refused anywhere else. Without
//! that, a cursor from one folder silently continues another: the resume key is just a name, it
//! compares fine, and the reader gets a page of entries from a directory they are not looking at.

use super::inspection::WorkspaceInspectionError;
use base64::Engine;

/// How many entries a page holds when the caller does not say.
pub(crate) const DEFAULT_DIRECTORY_PAGE_SIZE: usize = 500;

/// The most a caller may ask for.
///
/// A ceiling rather than a suggestion: the limit arrives from a client, and a listing is built by
/// enumerating and sorting a whole directory before it is cut. An unbounded request is an unbounded
/// response held in memory on both sides of the wire.
pub(crate) const MAX_DIRECTORY_PAGE_SIZE: usize = 1000;

/// Where a listing resumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectoryCursor {
    /// The directory this cursor was issued for, relative to the root.
    pub(crate) path: String,
    /// 0 for a directory, 1 for a file — the first half of the ordering key.
    pub(crate) kind_rank: u8,
    /// The lowercased name, which is what the ordering compares.
    pub(crate) name_key: String,
}

impl DirectoryCursor {
    pub(crate) fn after(path: &str, kind: &str, name: &str) -> Self {
        Self {
            path: path.to_string(),
            kind_rank: kind_rank(kind),
            name_key: name.to_lowercase(),
        }
    }

    /// Whether an entry belongs to the page that follows this cursor.
    ///
    /// Strictly after, by the same comparison the ordering uses. An entry equal to the cursor was
    /// the last one on the previous page; including it would repeat a row, which reads to a user as
    /// a duplicate file rather than as a paging bug.
    pub(crate) fn precedes(&self, kind: &str, name: &str) -> bool {
        let candidate = (kind_rank(kind), name.to_lowercase());
        candidate > (self.kind_rank, self.name_key.clone())
    }

    /// Opaque on the wire.
    ///
    /// Encoded rather than readable so a caller cannot hand-craft one: a cursor is a resume point
    /// this side issued, and a constructed one is a request to start reading from somewhere nobody
    /// paged to.
    pub(crate) fn encode(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(
            "{}\u{1f}{}\u{1f}{}",
            self.kind_rank, self.name_key, self.path
        ))
    }

    /// Decodes a cursor, or refuses it.
    ///
    /// The expected directory is required rather than read out of the cursor, so a mismatch is a
    /// refusal instead of a silent redirection to whatever the cursor names.
    pub(crate) fn decode(
        encoded: &str,
        expected_path: &str,
    ) -> Result<Self, WorkspaceInspectionError> {
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| WorkspaceInspectionError::InvalidCursor)?;
        let text = String::from_utf8(raw).map_err(|_| WorkspaceInspectionError::InvalidCursor)?;
        let mut parts = text.splitn(3, '\u{1f}');
        let kind_rank = parts
            .next()
            .and_then(|value| value.parse::<u8>().ok())
            .ok_or(WorkspaceInspectionError::InvalidCursor)?;
        let name_key = parts
            .next()
            .ok_or(WorkspaceInspectionError::InvalidCursor)?
            .to_string();
        let path = parts
            .next()
            .ok_or(WorkspaceInspectionError::InvalidCursor)?
            .to_string();
        if path != expected_path {
            // A cursor from another directory would resume this one at a name that happens to
            // compare, and hand back a page of entries from a folder the reader is not looking at.
            return Err(WorkspaceInspectionError::InvalidCursor);
        }
        Ok(Self {
            path,
            kind_rank,
            name_key,
        })
    }
}

/// Directories first. The same rank the local listing sorts by, named once so the cursor and the
/// ordering cannot drift apart.
pub(crate) fn kind_rank(kind: &str) -> u8 {
    if kind == "directory" {
        0
    } else {
        1
    }
}

/// The page size to actually use.
///
/// Clamped rather than refused: unlike a scope, a page size has an obviously correct nearby answer,
/// and refusing a request for 5000 entries would fail a reader who simply asked for a lot. The
/// answer still says `truncated`, so nobody mistakes the clamp for the end of the directory.
pub(crate) fn bounded_page_size(limit: Option<usize>) -> usize {
    match limit {
        None | Some(0) => DEFAULT_DIRECTORY_PAGE_SIZE,
        Some(value) => value.min(MAX_DIRECTORY_PAGE_SIZE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cursor_round_trips_through_its_encoding() {
        let cursor = DirectoryCursor::after("src", "file", "Main.rs");
        let decoded = DirectoryCursor::decode(&cursor.encode(), "src").expect("decode");

        assert_eq!(decoded, cursor);
        // Lowercased, because that is what the ordering compares. Storing the original name would
        // resume at a key the listing never sorts by.
        assert_eq!(decoded.name_key, "main.rs");
    }

    #[test]
    fn a_cursor_from_another_directory_is_refused() {
        let cursor = DirectoryCursor::after("src", "file", "main.rs").encode();

        // The name compares perfectly well against another directory's entries, which is exactly
        // why the directory has to be checked rather than trusted.
        assert!(DirectoryCursor::decode(&cursor, "docs").is_err());
        assert!(DirectoryCursor::decode(&cursor, "").is_err());
    }

    #[test]
    fn a_cursor_nobody_issued_is_refused_rather_than_guessed_at() {
        for forged in ["", "not-base64!", "YWJj"] {
            assert!(DirectoryCursor::decode(forged, "src").is_err(), "{forged}");
        }
    }

    #[test]
    fn a_directory_sorts_before_a_file_with_an_earlier_name() {
        let after_directory = DirectoryCursor::after("", "directory", "zebra");

        // The whole reason the rank is in the key: resuming after a name alone would put a file
        // called `alpha` before a directory called `zebra`, which is not the order the listing has.
        assert!(after_directory.precedes("file", "alpha"));
        assert!(!after_directory.precedes("directory", "alpha"));
    }

    #[test]
    fn an_entry_equal_to_the_cursor_belongs_to_the_page_before() {
        let cursor = DirectoryCursor::after("", "file", "readme.md");

        // Including it would repeat a row, which reads to a user as a duplicate file rather than
        // as a paging bug.
        assert!(!cursor.precedes("file", "readme.md"));
        assert!(!cursor.precedes("file", "README.MD"));
        assert!(cursor.precedes("file", "readme.mdx"));
    }

    #[test]
    fn a_page_size_is_clamped_rather_than_refused() {
        assert_eq!(bounded_page_size(None), DEFAULT_DIRECTORY_PAGE_SIZE);
        // Zero is a caller who did not choose, not a caller who wants nothing.
        assert_eq!(bounded_page_size(Some(0)), DEFAULT_DIRECTORY_PAGE_SIZE);
        assert_eq!(bounded_page_size(Some(10)), 10);
        // Clamped, because a page size has an obviously correct nearby answer and refusing would
        // fail a reader who simply asked for a lot. The answer still says `truncated`.
        assert_eq!(bounded_page_size(Some(100_000)), MAX_DIRECTORY_PAGE_SIZE);
    }
}
