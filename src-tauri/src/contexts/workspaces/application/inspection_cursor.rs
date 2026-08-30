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
//!
//! It carries four more things for the same reason, and they are the difference between v1 and v2:
//! which workspace it came from, which ordering produced it, which navigation policy decided what
//! the page contains, and what the directory looked like when the page was issued. Each of those can
//! change between two pages, and every one of them changes what "resume after this entry" means. A
//! cursor that omitted them would still decode, still compare, and still return a page — a page
//! assembled under rules that no longer hold, with nothing in it to say so.
//!
//! The two ways a cursor can fail are kept apart. `Invalid` is "this token is not for this listing",
//! and the only sensible response is to start again. `Stale` is "it was for this listing, and the
//! listing has moved on" — also a restart, but one a reader can be told the reason for, because it
//! means somebody or something changed the folder while they were reading it.

use super::inspection::WorkspaceInspectionError;
use base64::Engine;
use sha2::{Digest, Sha256};

/// How many entries a page holds when the caller does not say.
pub(crate) const DEFAULT_DIRECTORY_PAGE_SIZE: usize = 500;

/// The most a caller may ask for.
///
/// A ceiling rather than a suggestion: the limit arrives from a client, and a listing is built by
/// enumerating and sorting a whole directory before it is cut. An unbounded request is an unbounded
/// response held in memory on both sides of the wire.
pub(crate) const MAX_DIRECTORY_PAGE_SIZE: usize = 1000;

/// The cursor format this build issues.
///
/// Named in the token rather than inferred from its shape. A build that changed what a cursor
/// carries and left the version alone would decode an old one into the wrong fields and page from
/// wherever the misreading landed — a wrong answer that looks exactly like a right one.
const DIRECTORY_CURSOR_VERSION: &str = "v2";

/// The separator between a cursor's fields. A unit separator rather than a printable character,
/// because every printable one is a character a file can legally be called.
const FIELD: char = '\u{1f}';

/// How many fields a v2 directory cursor has. The last is the name key, which is taken whole.
const DIRECTORY_CURSOR_FIELDS: usize = 8;

/// Why a cursor was not used.
///
/// Two answers rather than one, because the difference is actionable. `Invalid` means the token does
/// not belong to this listing at all — forged, from another folder, or from a build that wrote it
/// differently. `Stale` means it did belong here and the directory has changed since, which is worth
/// saying out loud: the page about to be shown is not a continuation of the one being read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CursorRefusal {
    Invalid,
    Stale,
}

impl CursorRefusal {
    /// The stable token the frontend translates.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Invalid => "invalid_cursor",
            Self::Stale => "stale_cursor",
        }
    }
}

impl From<CursorRefusal> for WorkspaceInspectionError {
    /// Both collapse where a caller has no way to receive a page instead of an error.
    ///
    /// Lossy on purpose and only at that boundary: what such a caller can act on is "start again",
    /// which is right for either. A caller that can receive a page gets the refusal itself.
    fn from(_: CursorRefusal) -> Self {
        Self::InvalidCursor
    }
}

/// How a listing is ordered.
///
/// One mode today, named anyway. A cursor resumes at a position in an ordering, so a build that
/// added "by size" and resumed a `kind-name` cursor into it would page from a place that ordering
/// never produced. Naming it means such a cursor is refused instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectoryOrder {
    /// Directories first, then case-insensitively by name.
    KindThenName,
}

impl DirectoryOrder {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::KindThenName => "kind-name",
        }
    }

    fn parse(token: &str) -> Option<Self> {
        match token {
            "kind-name" => Some(Self::KindThenName),
            _ => None,
        }
    }
}

/// Everything a cursor is only valid within.
///
/// Built by the listing that issues a page and rebuilt by the listing that resumes it. Comparing two
/// of these field by field is what makes each mismatch its own answer, rather than a single "cursor
/// rejected" that leaves a reader guessing which rule they broke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectoryPageScope {
    /// Which workspace, as an opaque identity rather than a path.
    pub(crate) workspace: String,
    /// The directory this page is of, relative to the root.
    pub(crate) path: String,
    pub(crate) order: DirectoryOrder,
    /// Which navigation policy decided what the page contains.
    pub(crate) policy: String,
    /// What the directory looked like when the page was issued.
    ///
    /// `None` where the provider cannot detect a change at all — a remote host this build cannot
    /// ask, a volume with no directory mtime. Carried as an absence rather than a constant, because
    /// a constant compares equal forever and would report "unchanged" about a directory nobody
    /// looked at.
    pub(crate) fingerprint: Option<String>,
}

impl DirectoryPageScope {
    /// Whether a cursor issued under `self` names the same listing as `other`.
    fn matches_identity(&self, other: &Self) -> bool {
        self.workspace == other.workspace
            && self.path == other.path
            && self.order == other.order
            && self.policy == other.policy
            // A provider that gained or lost change detection between two pages is a different
            // provider answering, not a changed directory.
            && self.fingerprint.is_some() == other.fingerprint.is_some()
    }
}

/// An opaque, stable name for one workspace.
///
/// Hashed rather than carried. The cursor travels to a client, and a local root or an SSH target in
/// it would put this machine's directory layout — or a hostname and a username — somewhere a reader
/// can copy it out of, for a field whose only use is equality.
///
/// Takes the location as text so a local root and a remote target reach it the same way. The two
/// must not collide, which they cannot: a remote name carries its scheme.
pub(crate) fn workspace_identity(location: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut digest = Sha256::new();
    digest.update(location.as_bytes());
    // Sixteen hex characters. This is compared against another identity computed the same way in
    // the same process, so it is a name rather than a security boundary.
    digest
        .finalize()
        .iter()
        .take(8)
        .flat_map(|byte| {
            [
                HEX[usize::from(byte >> 4)] as char,
                HEX[usize::from(byte & 0x0f)] as char,
            ]
        })
        .collect()
}

/// Where a listing resumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectoryCursor {
    pub(crate) scope: DirectoryPageScope,
    /// 0 for a directory, 1 for a file — the first half of the ordering key.
    pub(crate) kind_rank: u8,
    /// The lowercased name, which is what the ordering compares.
    pub(crate) name_key: String,
}

impl DirectoryCursor {
    pub(crate) fn after(scope: DirectoryPageScope, kind: &str, name: &str) -> Self {
        Self {
            scope,
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
        self.precedes_key(kind_rank(kind), &name.to_lowercase())
    }

    /// The same question asked with the key already built.
    ///
    /// A listing that resumes before it stats an entry has the rank and the lowered name in hand
    /// and nothing else; making it construct a `&str` kind just to have this function take it apart
    /// again would be ceremony around a `u8`.
    pub(crate) fn precedes_key(&self, kind_rank: u8, name_key: &str) -> bool {
        (kind_rank, name_key) > (self.kind_rank, self.name_key.as_str())
    }

    /// Opaque on the wire.
    ///
    /// Encoded rather than readable so a caller cannot hand-craft one: a cursor is a resume point
    /// this side issued, and a constructed one is a request to start reading from somewhere nobody
    /// paged to.
    pub(crate) fn encode(&self) -> String {
        // The name key goes last so it can contain anything the decoder does not have to split on.
        let payload = [
            DIRECTORY_CURSOR_VERSION,
            &self.scope.workspace,
            &self.scope.path,
            self.scope.order.token(),
            &self.scope.policy,
            self.scope.fingerprint.as_deref().unwrap_or(""),
            &self.kind_rank.to_string(),
            &self.name_key,
        ]
        .join(&FIELD.to_string());
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload)
    }

    /// Decodes a cursor, or says which way it does not apply.
    ///
    /// The expected scope is supplied rather than read out of the cursor, so a mismatch is a refusal
    /// instead of a silent redirection to whatever the cursor names.
    pub(crate) fn decode(
        encoded: &str,
        expected: &DirectoryPageScope,
    ) -> Result<Self, CursorRefusal> {
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| CursorRefusal::Invalid)?;
        let text = String::from_utf8(raw).map_err(|_| CursorRefusal::Invalid)?;
        let parts: Vec<&str> = text.splitn(DIRECTORY_CURSOR_FIELDS, FIELD).collect();
        if parts.len() != DIRECTORY_CURSOR_FIELDS || parts[0] != DIRECTORY_CURSOR_VERSION {
            // Every v1 cursor lands here. Refused rather than migrated: a v1 token names no
            // workspace, no ordering and no policy, so migrating it would mean assuming all three,
            // and the assumption is wrong exactly when it matters.
            return Err(CursorRefusal::Invalid);
        }
        let order = DirectoryOrder::parse(parts[3]).ok_or(CursorRefusal::Invalid)?;
        let kind_rank = parts[6].parse::<u8>().map_err(|_| CursorRefusal::Invalid)?;
        let scope = DirectoryPageScope {
            workspace: parts[1].to_string(),
            path: parts[2].to_string(),
            order,
            policy: parts[4].to_string(),
            fingerprint: (!parts[5].is_empty()).then(|| parts[5].to_string()),
        };
        if !scope.matches_identity(expected) {
            return Err(CursorRefusal::Invalid);
        }
        if scope.fingerprint != expected.fingerprint {
            // It was issued for this listing and the listing has moved on. Appending a page built
            // from a different set of entries would drop or repeat rows silently; the reader is told
            // to start again instead.
            return Err(CursorRefusal::Stale);
        }
        Ok(Self {
            scope,
            kind_rank,
            name_key: parts[7].to_string(),
        })
    }
}

/// How many matches one page of a path search holds when the caller does not say.
pub(crate) const DEFAULT_PATH_SEARCH_RESULTS: usize = 25;

/// The most a path search will return in one page.
pub(crate) const MAX_PATH_SEARCH_RESULTS: usize = 50;

/// Where a ranked search resumes.
///
/// Bound to the query, not just to a position. A search's ordering is a function of what was
/// typed — the same file scores differently for `main` and for `ma` — so a cursor applied to a
/// different query would resume at a rank that ordering never produced, and hand back a page from
/// the middle of nowhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathSearchCursor {
    pub(crate) query: String,
    pub(crate) score: u32,
    pub(crate) depth: u32,
    pub(crate) path_key: String,
}

impl PathSearchCursor {
    pub(crate) fn after(query: &str, score: u32, depth: u32, path: &str) -> Self {
        Self {
            query: query.to_string(),
            score,
            depth,
            path_key: path.to_lowercase(),
        }
    }

    /// Whether a candidate belongs to the page after this cursor.
    ///
    /// Strictly after, by the same comparison the ranking uses: better score first, then shallower,
    /// then by path. An entry equal to the cursor ended the previous page, and including it would
    /// repeat a row — which in a result list reads as a duplicate file rather than a paging bug.
    pub(crate) fn precedes(&self, score: u32, depth: u32, path: &str) -> bool {
        rank_key(score, depth, &path.to_lowercase())
            > rank_key(self.score, self.depth, &self.path_key)
    }

    pub(crate) fn encode(&self) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!(
            "{}\u{1f}{}\u{1f}{}\u{1f}{}",
            self.score, self.depth, self.path_key, self.query
        ))
    }

    pub(crate) fn decode(
        encoded: &str,
        expected_query: &str,
    ) -> Result<Self, WorkspaceInspectionError> {
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| WorkspaceInspectionError::InvalidCursor)?;
        let text = String::from_utf8(raw).map_err(|_| WorkspaceInspectionError::InvalidCursor)?;
        let mut parts = text.splitn(4, '\u{1f}');
        let score = parts
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or(WorkspaceInspectionError::InvalidCursor)?;
        let depth = parts
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or(WorkspaceInspectionError::InvalidCursor)?;
        let path_key = parts
            .next()
            .ok_or(WorkspaceInspectionError::InvalidCursor)?
            .to_string();
        let query = parts
            .next()
            .ok_or(WorkspaceInspectionError::InvalidCursor)?
            .to_string();
        if query != expected_query {
            return Err(WorkspaceInspectionError::InvalidCursor);
        }
        Ok(Self {
            query,
            score,
            depth,
            path_key,
        })
    }
}

/// The ordering, as one comparable value.
///
/// Score is negated rather than the comparison being reversed, so "after the cursor" is a plain
/// `>` everywhere. A ranking expressed as a mix of ascending and descending comparisons is one
/// somebody eventually gets backwards in exactly one of the places it appears.
fn rank_key(score: u32, depth: u32, path_key: &str) -> (i64, u32, String) {
    (-(i64::from(score)), depth, path_key.to_string())
}

/// The page size a path search actually uses.
pub(crate) fn bounded_search_page(limit: Option<usize>) -> usize {
    match limit {
        None | Some(0) => DEFAULT_PATH_SEARCH_RESULTS,
        Some(value) => value.min(MAX_PATH_SEARCH_RESULTS),
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

    fn scope(path: &str) -> DirectoryPageScope {
        DirectoryPageScope {
            workspace: "workspace-a".to_string(),
            path: path.to_string(),
            order: DirectoryOrder::KindThenName,
            policy: "v1:direct".to_string(),
            fingerprint: Some("1000".to_string()),
        }
    }

    #[test]
    fn a_cursor_round_trips_through_its_encoding() {
        let cursor = DirectoryCursor::after(scope("src"), "file", "Main.rs");
        let decoded = DirectoryCursor::decode(&cursor.encode(), &scope("src")).expect("decode");

        assert_eq!(decoded, cursor);
        // Lowercased, because that is what the ordering compares. Storing the original name would
        // resume at a key the listing never sorts by.
        assert_eq!(decoded.name_key, "main.rs");
    }

    #[test]
    fn a_name_containing_the_field_separator_survives_the_round_trip() {
        // The separator is a control character, so this is a file nobody has. It is here because the
        // decoder splits on a fixed count and the name key is taken whole — a name that ate the
        // split would resume the listing at a key that is not the one the page ended on, which is a
        // wrong page rather than a rejected one.
        let cursor = DirectoryCursor::after(scope("src"), "file", "od\u{1f}d.rs");

        let decoded = DirectoryCursor::decode(&cursor.encode(), &scope("src")).expect("decode");

        assert_eq!(decoded.name_key, "od\u{1f}d.rs");
    }

    #[test]
    fn a_cursor_from_another_directory_is_refused() {
        let cursor = DirectoryCursor::after(scope("src"), "file", "main.rs").encode();

        // The name compares perfectly well against another directory's entries, which is exactly
        // why the directory has to be checked rather than trusted.
        assert_eq!(
            DirectoryCursor::decode(&cursor, &scope("docs")),
            Err(CursorRefusal::Invalid)
        );
        assert_eq!(
            DirectoryCursor::decode(&cursor, &scope("")),
            Err(CursorRefusal::Invalid)
        );
    }

    #[test]
    fn a_cursor_from_another_workspace_is_refused() {
        let cursor = DirectoryCursor::after(scope("src"), "file", "main.rs").encode();
        let elsewhere = DirectoryPageScope {
            workspace: "workspace-b".to_string(),
            ..scope("src")
        };

        // Two sessions can both have a `src`, and a relative path does not name which machine or
        // which project it is on. Without this the page would come from the other one.
        assert_eq!(
            DirectoryCursor::decode(&cursor, &elsewhere),
            Err(CursorRefusal::Invalid)
        );
    }

    #[test]
    fn a_cursor_from_another_policy_is_refused() {
        let cursor = DirectoryCursor::after(scope("src"), "file", "main.rs").encode();
        let other_policy = DirectoryPageScope {
            policy: "v1:recursive".to_string(),
            ..scope("src")
        };

        // A page assembled while dependency directories were hidden cannot be continued by one that
        // shows them: the resume key names a position in a listing the second page does not have.
        assert_eq!(
            DirectoryCursor::decode(&cursor, &other_policy),
            Err(CursorRefusal::Invalid)
        );
    }

    #[test]
    fn a_directory_that_changed_between_pages_is_stale_rather_than_invalid() {
        let cursor = DirectoryCursor::after(scope("src"), "file", "main.rs").encode();
        let changed = DirectoryPageScope {
            fingerprint: Some("2000".to_string()),
            ..scope("src")
        };

        // Its own answer, because the reader can be told why: somebody changed the folder while
        // they were reading it. Reported as `Invalid` it would look like a bug in the application.
        assert_eq!(
            DirectoryCursor::decode(&cursor, &changed),
            Err(CursorRefusal::Stale)
        );
        assert_eq!(CursorRefusal::Stale.code(), "stale_cursor");
        assert_eq!(CursorRefusal::Invalid.code(), "invalid_cursor");
    }

    #[test]
    fn a_provider_that_cannot_detect_change_still_pages() {
        let blind = DirectoryPageScope {
            fingerprint: None,
            ..scope("src")
        };
        let cursor = DirectoryCursor::after(blind.clone(), "file", "main.rs").encode();

        // A real limit rather than a failure: a host that cannot report a directory's fingerprint
        // would otherwise be unable to page at all. Refusing every cursor there would be safer only
        // in the sense that nothing works.
        assert!(DirectoryCursor::decode(&cursor, &blind).is_ok());
        // But it must not be readable as "unchanged" by a listing that can see the fingerprint.
        assert_eq!(
            DirectoryCursor::decode(&cursor, &scope("src")),
            Err(CursorRefusal::Invalid)
        );
    }

    #[test]
    fn a_cursor_nobody_issued_is_refused_rather_than_guessed_at() {
        for forged in ["", "not-base64!", "YWJj"] {
            assert_eq!(
                DirectoryCursor::decode(forged, &scope("src")),
                Err(CursorRefusal::Invalid),
                "{forged}"
            );
        }
    }

    #[test]
    fn a_cursor_from_an_ordering_this_build_does_not_have_is_refused() {
        // Hand-built, because there is one ordering today and no way to encode another through the
        // public constructor. That is exactly why the field exists: a build that adds "by size"
        // would otherwise resume a `kind-name` cursor into an ordering that never produced it, and
        // page from a position nothing in the new listing occupies.
        let payload = [
            "v2",
            "workspace-a",
            "src",
            "by-size",
            "v1:direct",
            "1000",
            "1",
            "main.rs",
        ]
        .join("\u{1f}");
        let forged = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);

        assert_eq!(
            DirectoryCursor::decode(&forged, &scope("src")),
            Err(CursorRefusal::Invalid)
        );
    }

    #[test]
    fn a_cursor_from_the_previous_format_is_refused_rather_than_reinterpreted() {
        // What v1 encoded: rank, name key, path — and nothing about the workspace, the ordering or
        // the policy. Reading it as a v2 cursor would fill those three from whatever happened to be
        // in the adjacent fields, and page from wherever that landed.
        let v1 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("1\u{1f}main.rs\u{1f}src");

        assert_eq!(
            DirectoryCursor::decode(&v1, &scope("src")),
            Err(CursorRefusal::Invalid)
        );
    }

    #[test]
    fn a_directory_sorts_before_a_file_with_an_earlier_name() {
        let after_directory = DirectoryCursor::after(scope(""), "directory", "zebra");

        // The whole reason the rank is in the key: resuming after a name alone would put a file
        // called `alpha` before a directory called `zebra`, which is not the order the listing has.
        assert!(after_directory.precedes("file", "alpha"));
        assert!(!after_directory.precedes("directory", "alpha"));
    }

    #[test]
    fn an_entry_equal_to_the_cursor_belongs_to_the_page_before() {
        let cursor = DirectoryCursor::after(scope(""), "file", "readme.md");

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
