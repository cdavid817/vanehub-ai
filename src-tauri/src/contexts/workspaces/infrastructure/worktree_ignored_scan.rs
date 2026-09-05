//! A bounded metadata inventory of the ignored paths in a worktree.
//!
//! Ignored is not the same as disposable: `.env`, local databases and editor state are all
//! ignored and all irreplaceable. The inventory exists so the user can be shown what removal
//! would take with it, and so an acknowledgement can be bound to exactly that set. It reads
//! names, types, sizes and timestamps — never file contents — and never follows a symlink.

use crate::contexts::workspaces::application::{CheckCompleteness, IgnoredEntry, IgnoredInventory};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub(crate) struct IgnoredScanLimits {
    pub(crate) max_entries: usize,
    pub(crate) max_bytes: u64,
    pub(crate) max_samples: usize,
}

pub(crate) struct IgnoredScan {
    pub(crate) inventory: IgnoredInventory,
    /// A `.git` directory or file was found inside an ignored directory: a nested repository
    /// whose own state this walk cannot vouch for.
    pub(crate) nested_repository: bool,
    /// A mount point, reparse point or unreadable directory was met.
    pub(crate) unreadable: bool,
}

/// `ignored` holds paths relative to `root` as Git printed them; a trailing slash marks a
/// directory Git chose not to descend into.
pub(crate) fn scan_ignored(
    root: &Path,
    ignored: &[Vec<u8>],
    limits: &IgnoredScanLimits,
) -> IgnoredScan {
    let mut state = ScanState {
        digest: Sha256::new(),
        entries: 0,
        bytes: 0,
        samples: Vec::new(),
        samples_truncated: false,
        complete: true,
        nested_repository: false,
        unreadable: false,
        limits,
    };
    for raw in ignored {
        let relative = super::worktree_git_parsing::bytes_to_path(raw);
        let absolute = root.join(&relative);
        state.visit(root, &absolute);
        if !state.complete {
            break;
        }
    }
    let fingerprint = crate::platform::hashing::hex(&state.digest.finalize());
    IgnoredScan {
        inventory: IgnoredInventory {
            total_entries: state.entries,
            samples: state.samples,
            samples_truncated: state.samples_truncated,
            completeness: if state.complete && !state.nested_repository && !state.unreadable {
                CheckCompleteness::Complete
            } else {
                CheckCompleteness::Incomplete
            },
            fingerprint,
        },
        nested_repository: state.nested_repository,
        unreadable: state.unreadable,
    }
}

struct ScanState<'a> {
    digest: Sha256,
    entries: usize,
    bytes: u64,
    samples: Vec<IgnoredEntry>,
    samples_truncated: bool,
    complete: bool,
    nested_repository: bool,
    unreadable: bool,
    limits: &'a IgnoredScanLimits,
}

impl ScanState<'_> {
    fn visit(&mut self, root: &Path, absolute: &Path) {
        if !self.complete {
            return;
        }
        let Ok(metadata) = fs::symlink_metadata(absolute) else {
            // Deleted between `git status` and now, or unreadable. Either way the inventory is
            // not a faithful account of the directory any more.
            self.unreadable = true;
            return;
        };
        let file_type = metadata.file_type();
        let kind = if file_type.is_symlink() {
            "symlink"
        } else if file_type.is_dir() {
            "dir"
        } else if file_type.is_file() {
            "file"
        } else {
            "other"
        };
        let relative = absolute.strip_prefix(root).unwrap_or(absolute);
        if relative.file_name().is_some_and(|name| name == ".git") && kind != "symlink" {
            self.nested_repository = true;
        }
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX))
            .unwrap_or(0);
        let size = if kind == "file" { metadata.len() } else { 0 };
        self.record(relative, kind, size, modified);
        if !self.complete || kind != "dir" {
            return;
        }
        let Ok(children) = fs::read_dir(absolute) else {
            self.unreadable = true;
            return;
        };
        let mut paths: Vec<PathBuf> = Vec::new();
        for child in children {
            match child {
                Ok(child) => paths.push(child.path()),
                Err(_) => {
                    self.unreadable = true;
                    return;
                }
            }
        }
        // Deterministic order, so the fingerprint is a property of the directory and not of the
        // filesystem's enumeration order.
        paths.sort();
        for path in paths {
            self.visit(root, &path);
            if !self.complete {
                return;
            }
        }
    }

    fn record(&mut self, relative: &Path, kind: &'static str, size: u64, modified: i64) {
        let raw = path_bytes(relative);
        self.entries += 1;
        self.bytes += raw.len() as u64 + 32;
        if self.entries > self.limits.max_entries || self.bytes > self.limits.max_bytes {
            self.complete = false;
            return;
        }
        self.digest.update(&raw);
        self.digest.update([0]);
        self.digest.update(kind.as_bytes());
        self.digest.update([0]);
        self.digest.update(size.to_le_bytes());
        self.digest.update(modified.to_le_bytes());
        self.digest.update(b"\n");
        if self.samples.len() < self.limits.max_samples {
            self.samples.push(IgnoredEntry {
                path: relative.to_string_lossy().into_owned(),
                kind,
                size,
                modified_unix: modified,
            });
        } else {
            self.samples_truncated = true;
        }
    }
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().into_owned().into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn limits() -> IgnoredScanLimits {
        IgnoredScanLimits {
            max_entries: 100,
            max_bytes: 1024 * 1024,
            max_samples: 3,
        }
    }

    #[test]
    fn inventories_ignored_files_and_directories_without_reading_contents() {
        let directory = TempDirectory::new("ignored-scan");
        directory.write(".env", "SECRET=do-not-read");
        directory.write("node_modules/a/index.js", "x");
        directory.write("node_modules/b.js", "y");
        let scan = scan_ignored(
            directory.path(),
            &[b".env".to_vec(), b"node_modules/".to_vec()],
            &limits(),
        );
        assert_eq!(scan.inventory.completeness, CheckCompleteness::Complete);
        assert_eq!(scan.inventory.total_entries, 5);
        assert!(scan.inventory.samples_truncated);
        assert_eq!(scan.inventory.samples.len(), 3);
        assert!(scan
            .inventory
            .samples
            .iter()
            .all(|entry| !entry.path.contains("SECRET")));
        assert!(!scan.nested_repository);
    }

    #[test]
    fn fingerprint_changes_when_ignored_metadata_changes() {
        let directory = TempDirectory::new("ignored-fingerprint");
        directory.write(".env", "a");
        let before = scan_ignored(directory.path(), &[b".env".to_vec()], &limits());
        directory.write(".env", "a longer value");
        let after = scan_ignored(directory.path(), &[b".env".to_vec()], &limits());
        assert_ne!(before.inventory.fingerprint, after.inventory.fingerprint);
        let again = scan_ignored(directory.path(), &[b".env".to_vec()], &limits());
        assert_eq!(after.inventory.fingerprint, again.inventory.fingerprint);
    }

    #[test]
    fn entry_budget_marks_the_inventory_incomplete() {
        let directory = TempDirectory::new("ignored-budget");
        for index in 0..10 {
            directory.write(&format!("build/{index}.o"), "o");
        }
        let scan = scan_ignored(
            directory.path(),
            &[b"build/".to_vec()],
            &IgnoredScanLimits {
                max_entries: 4,
                max_bytes: 1024 * 1024,
                max_samples: 10,
            },
        );
        assert_eq!(scan.inventory.completeness, CheckCompleteness::Incomplete);
    }

    #[test]
    fn nested_repositories_and_missing_paths_are_not_reported_as_complete() {
        let directory = TempDirectory::new("ignored-nested");
        directory.write("vendor/lib/.git/HEAD", "ref: refs/heads/main");
        let scan = scan_ignored(directory.path(), &[b"vendor/".to_vec()], &limits());
        assert!(scan.nested_repository);
        assert_eq!(scan.inventory.completeness, CheckCompleteness::Incomplete);

        let scan = scan_ignored(directory.path(), &[b"missing".to_vec()], &limits());
        assert!(scan.unreadable);
        assert_eq!(scan.inventory.completeness, CheckCompleteness::Incomplete);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_recorded_and_never_followed() {
        let directory = TempDirectory::new("ignored-symlink");
        let outside = TempDirectory::new("ignored-symlink-target");
        outside.write("secret.txt", "outside");
        std::os::unix::fs::symlink(outside.path(), directory.path().join("link")).expect("symlink");
        let scan = scan_ignored(directory.path(), &[b"link".to_vec()], &limits());
        assert_eq!(scan.inventory.total_entries, 1);
        assert_eq!(scan.inventory.samples[0].kind, "symlink");
        assert_eq!(scan.inventory.completeness, CheckCompleteness::Complete);
    }
}
