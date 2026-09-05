//! NUL-terminated Git output, parsed as bytes.
//!
//! `-z` output is the only form that survives a newline, a quote or a non-UTF-8 byte in a path,
//! and those paths are exactly the ones a cleanup must not misidentify. Nothing here splits on a
//! newline or decodes a path before comparing it.

use std::path::PathBuf;

/// One record of `git worktree list --porcelain -z`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WorktreeListEntry {
    pub(crate) path: PathBuf,
    pub(crate) head: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) bare: bool,
    pub(crate) detached: bool,
    pub(crate) locked: bool,
    pub(crate) prunable: bool,
}

/// Records are attribute lines each terminated by NUL, and records are separated by one more
/// NUL, so the stream is `attr\0attr\0\0attr\0\0`.
pub(crate) fn parse_worktree_list(raw: &[u8]) -> Vec<WorktreeListEntry> {
    let mut entries = Vec::new();
    let mut current: Option<WorktreeListEntry> = None;
    for field in raw.split(|byte| *byte == 0) {
        if field.is_empty() {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            continue;
        }
        if let Some(rest) = field.strip_prefix(b"worktree ") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(WorktreeListEntry {
                path: bytes_to_path(rest),
                ..WorktreeListEntry::default()
            });
            continue;
        }
        let Some(entry) = current.as_mut() else {
            continue;
        };
        if let Some(rest) = field.strip_prefix(b"HEAD ") {
            entry.head = Some(String::from_utf8_lossy(rest).into_owned());
        } else if let Some(rest) = field.strip_prefix(b"branch ") {
            entry.branch = Some(String::from_utf8_lossy(rest).into_owned());
        } else if field == b"bare" {
            entry.bare = true;
        } else if field == b"detached" {
            entry.detached = true;
        } else if field == b"locked" || field.starts_with(b"locked ") {
            entry.locked = true;
        } else if field == b"prunable" || field.starts_with(b"prunable ") {
            entry.prunable = true;
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    entries
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct StatusCounts {
    pub(crate) tracked_modified: usize,
    pub(crate) staged: usize,
    pub(crate) conflicted: usize,
    pub(crate) untracked: usize,
    /// Raw paths of ignored entries, for the inventory walk. Kept as bytes.
    pub(crate) ignored: Vec<Vec<u8>>,
    /// The stream ended without a terminating NUL or had a malformed record.
    pub(crate) malformed: bool,
    /// `max_entries` was reached before the stream ended.
    pub(crate) truncated: bool,
}

/// `git status --porcelain=v1 -z`: `XY<space><path>\0`, and for renames/copies a second
/// `<original>\0` follows. Counts are per entry, never per byte of prose.
pub(crate) fn parse_status_z(raw: &[u8], max_entries: usize) -> StatusCounts {
    let mut counts = StatusCounts::default();
    if raw.is_empty() {
        return counts;
    }
    if raw.last() != Some(&0) {
        counts.malformed = true;
    }
    let mut fields = raw.split(|byte| *byte == 0).peekable();
    let mut seen = 0usize;
    while let Some(field) = fields.next() {
        if field.is_empty() {
            continue;
        }
        if field.len() < 3 || field[2] != b' ' {
            counts.malformed = true;
            break;
        }
        seen += 1;
        if seen > max_entries {
            counts.truncated = true;
            break;
        }
        let (x, y) = (field[0], field[1]);
        let path = &field[3..];
        match (x, y) {
            (b'?', b'?') => counts.untracked += 1,
            (b'!', b'!') => counts.ignored.push(path.to_vec()),
            _ => {
                let unmerged =
                    matches!((x, y), (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D'));
                if unmerged {
                    counts.conflicted += 1;
                } else {
                    if matches!(x, b'M' | b'A' | b'D' | b'R' | b'C' | b'T') {
                        counts.staged += 1;
                    }
                    if matches!(y, b'M' | b'D' | b'T') {
                        counts.tracked_modified += 1;
                    }
                }
                if matches!(x, b'R' | b'C') {
                    // The original path follows as its own field and is not an entry.
                    if !fields.next().is_some_and(|original| !original.is_empty()) {
                        counts.malformed = true;
                    }
                }
            }
        }
    }
    counts
}

#[cfg(unix)]
pub(crate) fn bytes_to_path(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
pub(crate) fn bytes_to_path(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_list_records_are_split_on_double_nul_and_carry_flags() {
        let raw = b"worktree /repo\0HEAD aaa\0branch refs/heads/main\0\0worktree /repo-feat\0HEAD bbb\0branch refs/heads/vanehub/feat\0locked reason with\nnewline\0\0worktree /repo-gone\0HEAD ccc\0detached\0prunable gitdir file points to non-existent location\0\0";
        let entries = parse_worktree_list(raw);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].path, PathBuf::from("/repo"));
        assert_eq!(entries[0].branch.as_deref(), Some("refs/heads/main"));
        assert!(entries[1].locked);
        assert_eq!(
            entries[1].branch.as_deref(),
            Some("refs/heads/vanehub/feat")
        );
        assert!(entries[2].detached && entries[2].prunable);
        assert!(!entries[2].locked);
    }

    #[test]
    fn worktree_paths_with_spaces_quotes_and_newlines_survive() {
        let raw = b"worktree /tmp/odd \"name\"\nwith newline\0HEAD aaa\0branch refs/heads/x\0\0";
        let entries = parse_worktree_list(raw);
        assert_eq!(
            entries[0].path,
            PathBuf::from("/tmp/odd \"name\"\nwith newline")
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_keep_their_bytes() {
        use std::os::unix::ffi::OsStrExt;
        let raw = b"worktree /tmp/caf\xe9\0HEAD aaa\0\0";
        let entries = parse_worktree_list(raw);
        assert_eq!(entries[0].path.as_os_str().as_bytes(), b"/tmp/caf\xe9");
    }

    #[test]
    fn status_counts_each_category_independently() {
        let raw = b" M tracked.txt\0M  staged.txt\0MM both.txt\0?? new file.txt\0!! .env\0!! node_modules/\0UU conflict.txt\0R  new-name.txt\0old-name.txt\0";
        let counts = parse_status_z(raw, 100);
        assert_eq!(counts.tracked_modified, 2);
        assert_eq!(counts.staged, 3);
        assert_eq!(counts.conflicted, 1);
        assert_eq!(counts.untracked, 1);
        assert_eq!(
            counts.ignored,
            vec![b".env".to_vec(), b"node_modules/".to_vec()]
        );
        assert!(!counts.malformed);
        assert!(!counts.truncated);
    }

    #[test]
    fn status_reports_truncation_and_malformed_streams_instead_of_guessing() {
        let raw = b" M a\0 M b\0 M c\0";
        let counts = parse_status_z(raw, 2);
        assert!(counts.truncated);

        let counts = parse_status_z(b" M a\0 M b", 10);
        assert!(counts.malformed);

        let counts = parse_status_z(b"garbage\0", 10);
        assert!(counts.malformed);

        let counts = parse_status_z(b"R  new\0", 10);
        assert!(
            counts.malformed,
            "a rename without its original is malformed"
        );
    }

    #[test]
    fn status_paths_with_leading_dashes_and_newlines_are_entries_not_flags() {
        let raw = b"?? -rf\0?? line\nbreak\0";
        let counts = parse_status_z(raw, 10);
        assert_eq!(counts.untracked, 2);
    }
}
