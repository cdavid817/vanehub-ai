// The startup hook that runs this lands with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Walking the four roots at startup and applying the domain's verdict to each entry.
//!
//! The walk is deliberately shallow and shaped: it descends exactly as far as each root's layout
//! goes and no further, so a deep tree of unexpected directories cannot turn a cleanup into an
//! unbounded traversal. Anything whose shape does not match is reported and left alone.
//!
//! Nothing here decides what may go. That is `judge_entry`, and keeping the decision out of the
//! walk is what makes it testable without a filesystem.

use super::roots::ExtensionRoots;
use crate::contexts::tooling::extension_platform::domain::{
    judge_entry, ExtensionRootScope, ReconciliationSummary, ReconciliationVerdict,
    ALL_EXTENSION_ROOT_SCOPES,
};
use crate::platform::database::NativeDatabase;
use rusqlite::params;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// How deep each root's layout goes. A package is `sha256/<hash>`; everything else is one or two
/// identifier segments.
const fn depth(scope: ExtensionRootScope) -> usize {
    match scope {
        ExtensionRootScope::Quarantine => 1,
        ExtensionRootScope::Packages
        | ExtensionRootScope::Scratch
        | ExtensionRootScope::Sidecars => 2,
    }
}

/// Every package hash a snapshot row names.
///
/// Rows rather than pointers: a snapshot that is no longer active is still the rollback target and
/// still the record of what an installation ran. Collecting its bytes because nothing points at it
/// *right now* would delete the thing a rollback needs.
pub(crate) fn referenced_package_hashes(
    database: &NativeDatabase,
) -> Result<BTreeSet<String>, String> {
    let connection = database.connection().map_err(|error| error.to_string())?;
    let mut statement = connection
        .prepare("SELECT package_hash FROM extension_platform_snapshots")
        .map_err(|error| error.to_string())?;
    let hashes = statement
        .query_map(params![], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(hashes)
}

/// Cleans up what a previous run left behind, and reports what it would not touch.
///
/// Never returns an error for an entry it could not remove. The entry is unreferenced, the next
/// start will try again, and refusing to finish startup because one directory is locked would be a
/// worse failure than leaving it.
pub(crate) fn reconcile(
    roots: &ExtensionRoots,
    referenced_hashes: &BTreeSet<String>,
) -> ReconciliationSummary {
    let mut summary = ReconciliationSummary::default();
    for scope in ALL_EXTENSION_ROOT_SCOPES {
        let root = roots.root(scope);
        for (path, segments) in enumerate(&root, depth(scope)) {
            let borrowed: Vec<&str> = segments.iter().map(String::as_str).collect();
            let label = display(&root, &path);
            match judge_entry(scope, &borrowed, referenced_hashes) {
                ReconciliationVerdict::Collect(_) => {
                    if roots.discard(&path).is_ok() {
                        summary.collected.push(label);
                    } else {
                        summary.uncollectable.push(label);
                    }
                }
                ReconciliationVerdict::RetainReferencedPackage => summary.retained.push(label),
                ReconciliationVerdict::Unrecognised => summary.unrecognised.push(label),
            }
        }
    }
    summary.collected.sort();
    summary.retained.sort();
    summary.unrecognised.sort();
    summary.uncollectable.sort();
    summary
}

/// Every entry at exactly `depth` below `root`, with the segments that lead to it.
///
/// A shallower entry that should have had children — a file where a directory belongs — is yielded
/// at the depth it was found, so the domain sees a shape that does not match and says
/// "unrecognised" rather than the walk silently skipping it.
fn enumerate(root: &Path, depth: usize) -> Vec<(PathBuf, Vec<String>)> {
    let mut found = Vec::new();
    let mut frontier = vec![(root.to_path_buf(), Vec::new())];
    for _ in 0..depth {
        let mut next = Vec::new();
        for (path, segments) in frontier {
            let Ok(entries) = std::fs::read_dir(&path) else {
                // A leaf that cannot be listed is a leaf: yielded as-is so it is judged rather than
                // skipped.
                if !segments.is_empty() {
                    found.push((path, segments));
                }
                continue;
            };
            for entry in entries.flatten() {
                let mut deeper = segments.clone();
                deeper.push(entry.file_name().to_string_lossy().to_string());
                next.push((entry.path(), deeper));
            }
        }
        frontier = next;
    }
    found.extend(
        frontier
            .into_iter()
            .filter(|(_, segments)| !segments.is_empty()),
    );
    found
}

/// A root-relative label, so a diagnostic names the entry without naming the machine.
fn display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
