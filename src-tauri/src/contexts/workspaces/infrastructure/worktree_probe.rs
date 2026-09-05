//! Read-only Git and filesystem probes for one worktree, and the one removal command.
//!
//! Every probe runs through `GitAdapter::execute_isolated` so an inherited `GIT_DIR` cannot
//! redirect it, with argument arrays rather than shell strings, and reads `-z` output as bytes.
//! Paths Git reports are used as Git reports them; nothing is assembled from a basename.

use super::worktree_git_parsing::{parse_status_z, parse_worktree_list, WorktreeListEntry};
use super::worktree_ignored_scan::{scan_ignored, IgnoredScanLimits};
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::workspaces::application::{
    reason, CheckCompleteness, Presence, ProbeBudget, WorktreeChangeSummary, WorktreeObservation,
    WorktreeProbe, WorktreeProbePort, WorktreeRemovalOutcome, WorktreeRemovalPort,
};
use crate::contexts::workspaces::domain::WorktreeIdentity;
use crate::platform::git::{GitAdapter, GitOutput};
use crate::platform::process::ProcessError;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const LOG_CATEGORY: &str = "git.worktree.cleanup";
/// Output cap for every probe. Reaching it makes the answer `Incomplete`, never longer.
const MAX_LS_FILES_ENTRIES: usize = 50_000;

#[derive(Clone)]
pub(crate) struct GitWorktreeProbe {
    git: GitAdapter,
    logging: Arc<dyn DiagnosticLogPort>,
}

impl GitWorktreeProbe {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self {
            git: GitAdapter::default(),
            logging,
        }
    }

    fn run(&self, root: &Path, args: &[&str], timeout: Duration) -> Result<GitOutput, ProbeFault> {
        let args: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        self.git
            .execute_isolated(root, &args, timeout)
            .map_err(|error| match error {
                ProcessError::TimedOut { .. } => ProbeFault::TimedOut,
                ProcessError::InvalidExecutable(_) | ProcessError::Spawn(_) => {
                    ProbeFault::Unavailable
                }
                _ => ProbeFault::Failed,
            })
    }

    fn stdout_text(
        &self,
        root: &Path,
        args: &[&str],
        timeout: Duration,
    ) -> Result<Option<String>, ProbeFault> {
        let output = self.run(root, args, timeout)?;
        if !output.status.success() {
            return Ok(None);
        }
        Ok(Some(
            String::from_utf8_lossy(&output.stdout)
                .trim_end_matches(['\n', '\r'])
                .to_string(),
        ))
    }

    fn record(&self, severity: LogSeverity, message: String, context: BTreeMap<String, String>) {
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity,
            category: LOG_CATEGORY.to_string(),
            message,
            context,
        });
    }

    fn probe_inner(
        &self,
        root: &Path,
        budget: &ProbeBudget,
        with_contents: bool,
    ) -> Result<WorktreeProbe, ProbeFault> {
        let timeout = budget.git_timeout;
        let mut probe = WorktreeProbe::default();
        let Ok(root_metadata) = fs::symlink_metadata(root) else {
            return Ok(probe);
        };
        probe.root_exists = root_metadata.is_dir() || root_metadata.file_type().is_symlink();
        if root_metadata.file_type().is_symlink() {
            // A link is refused outright: removing "the directory" would remove its target.
            probe.unsupported_layout = Some("symlink_root");
            return Ok(probe);
        }
        if !root_metadata.is_dir() {
            probe.root_exists = false;
            return Ok(probe);
        }
        let Some(toplevel) = self.stdout_text(root, &["rev-parse", "--show-toplevel"], timeout)?
        else {
            return Ok(probe);
        };
        let canonical_root = canonical(root);
        let canonical_toplevel = canonical(Path::new(&toplevel));
        let (Some(canonical_root), Some(canonical_toplevel)) = (canonical_root, canonical_toplevel)
        else {
            return Ok(probe);
        };
        if canonical_root != canonical_toplevel {
            // The directory sits inside a worktree rooted elsewhere.
            probe.nested_layout = true;
            return Ok(probe);
        }
        let bare = self
            .stdout_text(root, &["rev-parse", "--is-bare-repository"], timeout)?
            .is_some_and(|value| value == "true");
        let Some(git_dir) =
            self.stdout_text(root, &["rev-parse", "--absolute-git-dir"], timeout)?
        else {
            return Ok(probe);
        };
        let Some(common_dir) = self.stdout_text(
            root,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
            timeout,
        )?
        else {
            return Ok(probe);
        };
        let git_dir = canonical(Path::new(&git_dir)).unwrap_or(git_dir);
        let common_dir = canonical(Path::new(&common_dir)).unwrap_or(common_dir);
        probe.is_linked = !bare && git_dir != common_dir;

        let symbolic = self.run(root, &["symbolic-ref", "-q", "HEAD"], timeout)?;
        let branch = if symbolic.status.success() {
            String::from_utf8_lossy(&symbolic.stdout)
                .trim()
                .strip_prefix("refs/heads/")
                .map(str::to_string)
        } else {
            probe.detached = true;
            None
        };
        let head = self.stdout_text(root, &["rev-parse", "--verify", "HEAD"], timeout)?;
        probe.branch_resolves_to_head = match (&branch, &head) {
            (Some(branch), Some(head)) => {
                let reference = format!("refs/heads/{branch}");
                self.stdout_text(
                    root,
                    &["rev-parse", "--verify", reference.as_str()],
                    timeout,
                )?
                .as_deref()
                    == Some(head.as_str())
            }
            _ => false,
        };
        probe.identity = Some(WorktreeIdentity {
            canonical_root: canonical_root.clone(),
            git_dir: git_dir.clone(),
            common_dir: common_dir.clone(),
            branch,
            head,
            fs_identity: fs_identity(root),
        });

        let entries = self.list_worktrees(root, timeout)?;
        self.apply_registration(&mut probe, &entries, &canonical_root);
        probe.in_progress_operation = in_progress_operation(Path::new(&git_dir));
        if !with_contents {
            return Ok(probe);
        }
        probe.unsupported_layout = probe
            .unsupported_layout
            .or(self.unsupported_layout(root, timeout)?);

        let status = self.run(
            root,
            &[
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--ignored=traditional",
                "--ignore-submodules=none",
            ],
            timeout,
        )?;
        if !status.status.success() {
            return Ok(probe);
        }
        let counts = parse_status_z(&status.stdout, budget.max_status_entries);
        let complete = !counts.malformed && !counts.truncated;
        probe.changes = Some(WorktreeChangeSummary {
            tracked_modified: counts.tracked_modified,
            staged: counts.staged,
            conflicted: counts.conflicted,
            untracked: counts.untracked,
            ignored_paths: counts.ignored.len(),
            completeness: Some(if complete {
                CheckCompleteness::Complete
            } else {
                CheckCompleteness::Incomplete
            }),
        });
        if complete {
            let scan = scan_ignored(
                Path::new(&canonical_root),
                &counts.ignored,
                &IgnoredScanLimits {
                    max_entries: budget.max_ignored_entries,
                    max_bytes: budget.max_ignored_bytes,
                    max_samples: budget.max_ignored_samples,
                },
            );
            if scan.nested_repository {
                probe.unsupported_layout = probe.unsupported_layout.or(Some("nested_repository"));
            }
            probe.ignored = Some(scan.inventory);
        }
        Ok(probe)
    }

    fn list_worktrees(
        &self,
        root: &Path,
        timeout: Duration,
    ) -> Result<Vec<WorktreeListEntry>, ProbeFault> {
        let output = self.run(root, &["worktree", "list", "--porcelain", "-z"], timeout)?;
        if !output.status.success() {
            return Ok(Vec::new());
        }
        Ok(parse_worktree_list(&output.stdout))
    }

    fn apply_registration(
        &self,
        probe: &mut WorktreeProbe,
        entries: &[WorktreeListEntry],
        canonical_root: &str,
    ) {
        let target = Path::new(canonical_root);
        let mut main: Option<PathBuf> = None;
        for (index, entry) in entries.iter().enumerate() {
            let entry_path = canonical(&entry.path)
                .map(PathBuf::from)
                .unwrap_or(entry.path.clone());
            if index == 0 && !entry.bare {
                main = Some(entry_path.clone());
            }
            if entry_path == target {
                probe.registered = true;
                probe.locked = entry.locked;
                probe.prunable = entry.prunable;
                continue;
            }
            if entry_path.starts_with(target) || target.starts_with(&entry_path) {
                // Another registered worktree above or below this one. The main worktree is
                // always "above" only when the target is inside it, which is the nested case.
                probe.nested_layout = true;
            }
        }
        probe.anchor = main
            .filter(|main| main != target && !main.starts_with(target))
            .map(|main| main.to_string_lossy().into_owned());
    }

    fn unsupported_layout(
        &self,
        root: &Path,
        timeout: Duration,
    ) -> Result<Option<&'static str>, ProbeFault> {
        if root.join(".gitmodules").exists() {
            return Ok(Some("submodule"));
        }
        if self
            .stdout_text(root, &["config", "--get", "core.sparseCheckout"], timeout)?
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
        {
            return Ok(Some("sparse_checkout"));
        }
        let listing = self.run(root, &["ls-files", "-v", "-z"], timeout)?;
        if !listing.status.success() {
            return Ok(Some("index_unreadable"));
        }
        let mut seen = 0usize;
        for field in listing.stdout.split(|byte| *byte == 0) {
            if field.is_empty() {
                continue;
            }
            seen += 1;
            if seen > MAX_LS_FILES_ENTRIES {
                return Ok(Some("index_too_large"));
            }
            // Lowercase tags mark assume-unchanged; `S`/`s` mark skip-worktree. Either hides a
            // modification from `status`, so "clean" would not mean clean.
            match field[0] {
                b'h' | b's' | b'S' | b'm' | b'r' | b'c' | b'k' => return Ok(Some("index_flags")),
                _ => {}
            }
        }
        Ok(None)
    }
}

enum ProbeFault {
    Unavailable,
    TimedOut,
    Failed,
}

impl GitWorktreeProbe {
    fn probe_or_failure(
        &self,
        root: &Path,
        budget: &ProbeBudget,
        with_contents: bool,
    ) -> WorktreeProbe {
        match self.probe_inner(root, budget, with_contents) {
            Ok(probe) => probe,
            Err(fault) => {
                let reason = match fault {
                    ProbeFault::Unavailable => reason::GIT_UNAVAILABLE,
                    ProbeFault::TimedOut | ProbeFault::Failed => reason::PROBE_FAILED,
                };
                self.record(
                    LogSeverity::Warn,
                    "Worktree probe failed".to_string(),
                    BTreeMap::from([("reason".to_string(), reason.to_string())]),
                );
                WorktreeProbe::failed(reason)
            }
        }
    }
}

impl WorktreeProbePort for GitWorktreeProbe {
    fn probe(&self, root: &Path, budget: &ProbeBudget) -> WorktreeProbe {
        self.probe_or_failure(root, budget, true)
    }

    fn probe_identity(&self, root: &Path, budget: &ProbeBudget) -> WorktreeProbe {
        self.probe_or_failure(root, budget, false)
    }

    fn canonical_root(&self, path: &str) -> Option<String> {
        let candidate = Path::new(path);
        if !candidate.is_dir() {
            return None;
        }
        canonical(candidate)
    }

    fn observe(&self, expected: &WorktreeIdentity, anchor: Option<&Path>) -> WorktreeObservation {
        let root = Path::new(&expected.canonical_root);
        let root_present = match fs::symlink_metadata(root) {
            Ok(_) => Presence::Present,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Presence::Absent,
            Err(_) => Presence::Unknown,
        };
        let anchor = anchor
            .filter(|anchor| anchor.is_dir() && !anchor.starts_with(root))
            .or_else(|| {
                // Without a caller-supplied anchor the common directory's parent is the main
                // worktree for an ordinary layout.
                Path::new(&expected.common_dir).parent()
            })
            .filter(|anchor| anchor.is_dir() && !anchor.starts_with(root));
        let (registered, anchor_available) = match anchor {
            Some(anchor) => match self.list_worktrees(anchor, ProbeBudget::DEFAULT.git_timeout) {
                Ok(entries) => {
                    let registered = entries.iter().any(|entry| {
                        canonical(&entry.path)
                            .map(PathBuf::from)
                            .unwrap_or(entry.path.clone())
                            == root
                            || entry.path == root
                    });
                    (
                        if registered {
                            Presence::Present
                        } else {
                            Presence::Absent
                        },
                        true,
                    )
                }
                Err(_) => (Presence::Unknown, false),
            },
            None => (Presence::Unknown, false),
        };
        let identity_matches = if root_present == Presence::Present {
            let probe = self.probe_identity(root, &ProbeBudget::DEFAULT);
            Some(probe.identity.as_ref().is_some_and(|current| {
                current.git_dir == expected.git_dir
                    && current.common_dir == expected.common_dir
                    && match (&current.fs_identity, &expected.fs_identity) {
                        (Some(left), Some(right)) => left == right,
                        _ => true,
                    }
            }))
        } else {
            None
        };
        WorktreeObservation {
            root_present,
            registered,
            identity_matches,
            anchor_available,
        }
    }
}

impl WorktreeRemovalPort for GitWorktreeProbe {
    fn remove(&self, anchor: &Path, target: &Path, timeout: Duration) -> WorktreeRemovalOutcome {
        let target_text = target.to_string_lossy().into_owned();
        // Exactly `worktree remove <absolute target>`: no `--force`, and the target is the
        // verified absolute path rather than anything a caller typed.
        let args = ["worktree", "remove", target_text.as_str()];
        let started = std::time::Instant::now();
        let outcome = match self.run(anchor, &args, timeout) {
            Ok(output) if output.status.success() => WorktreeRemovalOutcome::Succeeded,
            Ok(output) => WorktreeRemovalOutcome::Refused {
                exit_code: output.status.code(),
                diagnostic: GitAdapter::redacted_diagnostic("worktree-remove", anchor, &output),
            },
            Err(ProbeFault::TimedOut) => WorktreeRemovalOutcome::TimedOut {
                // The process adapter kills and waits for the child on timeout.
                exit_confirmed: true,
            },
            Err(ProbeFault::Unavailable) => {
                WorktreeRemovalOutcome::Unavailable(reason::GIT_UNAVAILABLE.to_string())
            }
            Err(ProbeFault::Failed) => {
                WorktreeRemovalOutcome::Unavailable(reason::PROBE_FAILED.to_string())
            }
        };
        let mut context = BTreeMap::new();
        context.insert(
            "elapsedMs".to_string(),
            started.elapsed().as_millis().to_string(),
        );
        context.insert(
            "outcome".to_string(),
            match &outcome {
                WorktreeRemovalOutcome::Succeeded => "succeeded".to_string(),
                WorktreeRemovalOutcome::Refused { exit_code, .. } => {
                    format!(
                        "refused:{}",
                        exit_code.map_or("none".to_string(), |c| c.to_string())
                    )
                }
                WorktreeRemovalOutcome::TimedOut { .. } => "timed_out".to_string(),
                WorktreeRemovalOutcome::Unavailable(reason) => format!("unavailable:{reason}"),
            },
        );
        let severity = if matches!(outcome, WorktreeRemovalOutcome::Succeeded) {
            LogSeverity::Info
        } else {
            LogSeverity::Error
        };
        let message = match &outcome {
            WorktreeRemovalOutcome::Refused { diagnostic, .. } => {
                format!("Git worktree remove refused: {diagnostic}")
            }
            _ => "Git worktree remove finished".to_string(),
        };
        self.record(severity, message, context);
        outcome
    }
}

fn canonical(path: &Path) -> Option<String> {
    path.canonicalize()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

#[cfg(unix)]
fn fs_identity(root: &Path) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(root)
        .ok()
        .map(|metadata| format!("{}:{}", metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn fs_identity(_root: &Path) -> Option<String> {
    None
}

fn in_progress_operation(git_dir: &Path) -> bool {
    [
        "MERGE_HEAD",
        "REBASE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "BISECT_LOG",
        "rebase-merge",
        "rebase-apply",
    ]
    .iter()
    .any(|marker| git_dir.join(marker).exists())
}
