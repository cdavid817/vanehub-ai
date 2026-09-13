//! The built-in `patch-whitespace` verification: an in-process rule over safe snapshots.
//!
//! It launches nothing. Baseline text comes from the run's sealed baseline objects, current text
//! is read through the scoped root with no link following, and the rule is fixed: a newly added
//! text line must not end in a space or tab. Anything it cannot inspect consistently is reported
//! as unverifiable rather than passed, and binary files are excluded from the text rule while
//! still being covered by the complete scope evidence.

use super::loop_artifact_scan::{diff_manifests, ChangeKind, WorkspaceManifest};
use super::loop_evidence_store::LoopEvidenceStore;
use super::loop_scope_fs::ScopedWorkspaceRoot;
use crate::contexts::agent_runtime::domain::LoopScopePath;
use std::collections::HashMap;

const MAX_FINDINGS: usize = 50;
const BINARY_PROBE_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeCheckStatus {
    Passed,
    Failed,
    Unverifiable,
}

impl NativeCheckStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Unverifiable => "unverifiable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeCheckOutcome {
    pub(crate) status: NativeCheckStatus,
    pub(crate) findings: Vec<String>,
    pub(crate) inspected_files: usize,
    pub(crate) binary_excluded: usize,
    pub(crate) detail: Option<String>,
}

pub(crate) fn patch_whitespace(
    run_id: &str,
    store: &LoopEvidenceStore,
    root: &ScopedWorkspaceRoot,
    baseline: &WorkspaceManifest,
    current: &WorkspaceManifest,
) -> NativeCheckOutcome {
    let mut findings = Vec::new();
    let mut inspected = 0usize;
    let mut binary_excluded = 0usize;
    for change in diff_manifests(baseline, current) {
        let Some(entry) = current.find(&change.path) else {
            continue;
        };
        if entry.kind != "file" || !matches!(change.kind, ChangeKind::Added | ChangeKind::Modified)
        {
            continue;
        }
        let path = match LoopScopePath::parse(&change.path) {
            Ok(path) => path,
            Err(error) => {
                return unverifiable(format!("{}: {error}", change.path));
            }
        };
        let current_bytes = match root.read_file(&path) {
            Ok(bytes) => bytes,
            Err(error) => return unverifiable(format!("{}: {error}", change.path)),
        };
        let expected_digest = entry.digest.as_deref().unwrap_or_default();
        if super::loop_artifact_scan::hex(&sha2::Sha256::digest(&current_bytes)) != expected_digest
        {
            return unverifiable(format!(
                "{} changed after the phase manifest was captured",
                change.path
            ));
        }
        let baseline_bytes = match change.kind {
            ChangeKind::Added => Vec::new(),
            _ => match baseline
                .find(&change.path)
                .and_then(|previous| previous.digest.as_deref())
            {
                Some(digest) => match store.read_object(run_id, digest) {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        return unverifiable(format!("{}: {}", change.path, error.message()))
                    }
                },
                None => Vec::new(),
            },
        };
        if is_binary(&current_bytes) || is_binary(&baseline_bytes) {
            binary_excluded += 1;
            continue;
        }
        let (Ok(current_text), Ok(baseline_text)) = (
            std::str::from_utf8(&current_bytes),
            std::str::from_utf8(&baseline_bytes),
        ) else {
            return unverifiable(format!("{} is not valid UTF-8 text", change.path));
        };
        inspected += 1;
        for (line_number, line) in added_lines(baseline_text, current_text) {
            if (line.ends_with(' ') || line.ends_with('\t')) && findings.len() < MAX_FINDINGS {
                findings.push(format!(
                    "{}:{line_number}: trailing whitespace",
                    change.path
                ));
            }
        }
    }
    NativeCheckOutcome {
        status: if findings.is_empty() {
            NativeCheckStatus::Passed
        } else {
            NativeCheckStatus::Failed
        },
        findings,
        inspected_files: inspected,
        binary_excluded,
        detail: None,
    }
}

/// Lines present in `current` beyond their multiplicity in `baseline`, with their 1-based
/// line numbers. A multiset difference is deterministic and needs no alignment heuristics.
fn added_lines<'a>(baseline: &str, current: &'a str) -> Vec<(usize, &'a str)> {
    let mut remaining: HashMap<&str, usize> = HashMap::new();
    for line in baseline.lines() {
        *remaining.entry(line).or_insert(0) += 1;
    }
    let mut added = Vec::new();
    for (index, line) in current.lines().enumerate() {
        match remaining.get_mut(line) {
            Some(count) if *count > 0 => *count -= 1,
            _ => added.push((index + 1, line)),
        }
    }
    added
}

fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(BINARY_PROBE_BYTES).any(|byte| *byte == 0)
}

fn unverifiable(detail: String) -> NativeCheckOutcome {
    NativeCheckOutcome {
        status: NativeCheckStatus::Unverifiable,
        findings: Vec::new(),
        inspected_files: 0,
        binary_excluded: 0,
        detail: Some(detail),
    }
}

use sha2::Digest;

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::infrastructure::loop_artifact_scan::{
        scan_workspace, ScanBudget,
    };
    use crate::contexts::agent_runtime::infrastructure::loop_evidence_store::DEFAULT_OBJECT_BUDGET_BYTES;
    use crate::test_support::TempDirectory;
    use std::sync::atomic::AtomicBool;

    fn snapshot(root: &std::path::Path) -> WorkspaceManifest {
        scan_workspace(root, &ScanBudget::default(), &AtomicBool::new(false)).expect("scan")
    }

    #[test]
    fn only_newly_added_lines_are_judged_and_binary_files_are_excluded() {
        let workspace = TempDirectory::new("native-check");
        let storage = TempDirectory::new("native-check-store");
        workspace.write("keep.txt", "old trailing \nclean\n");
        workspace.write("image.bin", "\u{0}\u{1}binary");
        let baseline = snapshot(workspace.path());
        let store = LoopEvidenceStore::new(storage.path().to_path_buf());
        store
            .capture_objects(
                "run",
                workspace.path(),
                &baseline,
                DEFAULT_OBJECT_BUDGET_BYTES,
            )
            .expect("baseline objects");
        let root = ScopedWorkspaceRoot::open(workspace.path()).expect("root");

        // Pre-existing trailing whitespace is not a finding; a new clean line passes.
        workspace.write("keep.txt", "old trailing \nclean\nnew clean\n");
        workspace.write("image.bin", "\u{0}\u{1}binary changed \t");
        let current = snapshot(workspace.path());
        let outcome = patch_whitespace("run", &store, &root, &baseline, &current);
        assert_eq!(outcome.status, NativeCheckStatus::Passed, "{outcome:?}");
        assert_eq!(outcome.inspected_files, 1);
        assert_eq!(outcome.binary_excluded, 1);

        workspace.write("keep.txt", "old trailing \nclean\nnew clean\nbad line\t\n");
        workspace.write("fresh.txt", "first \n");
        let current = snapshot(workspace.path());
        let outcome = patch_whitespace("run", &store, &root, &baseline, &current);
        assert_eq!(outcome.status, NativeCheckStatus::Failed);
        assert_eq!(
            outcome.findings,
            vec![
                "fresh.txt:1: trailing whitespace".to_string(),
                "keep.txt:4: trailing whitespace".to_string()
            ]
        );
    }

    #[test]
    fn an_inconsistent_snapshot_is_unverifiable_not_passed() {
        let workspace = TempDirectory::new("native-check-drift");
        let storage = TempDirectory::new("native-check-drift-store");
        workspace.write("a.txt", "one\n");
        let baseline = snapshot(workspace.path());
        let store = LoopEvidenceStore::new(storage.path().to_path_buf());
        store
            .capture_objects(
                "run",
                workspace.path(),
                &baseline,
                DEFAULT_OBJECT_BUDGET_BYTES,
            )
            .expect("objects");
        let root = ScopedWorkspaceRoot::open(workspace.path()).expect("root");
        workspace.write("a.txt", "one\ntwo\n");
        let current = snapshot(workspace.path());
        // The file changes again after the phase manifest was captured.
        workspace.write("a.txt", "one\nthree\n");
        let outcome = patch_whitespace("run", &store, &root, &baseline, &current);
        assert_eq!(outcome.status, NativeCheckStatus::Unverifiable);
        assert!(outcome
            .detail
            .as_deref()
            .unwrap_or_default()
            .contains("a.txt"));
    }
}
