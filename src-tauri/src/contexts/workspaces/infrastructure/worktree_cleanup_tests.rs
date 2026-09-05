//! The cleanup stack against real temporary Git repositories.
//!
//! Every repository here is created under the system temp directory by the test itself; nothing
//! reads a user project, credential or database. Each destructive test records the base HEAD,
//! the branch refs, the worktree registry and a sentinel outside the target before it acts, and
//! checks all of them afterwards: the branch survives, the main worktree survives, the sentinel
//! survives, and only the one directory that was authorized is gone.

use super::managed_worktree_repository::{
    SqliteManagedWorktreeRepository, SqliteWorkspaceUseGate, SystemWorktreeCleanupClock,
    UuidWorktreeIds,
};
use super::worktree_probe::GitWorktreeProbe;
use crate::contexts::operations::application::{
    ApplicationError, DiagnosticLog, DiagnosticLogPort,
};
use crate::contexts::workspaces::application::{
    evaluate_cleanup, reason, GateOwner, ProbeBudget, ReferenceSummary, WorktreeCleanupService,
    WorktreeProbePort, WorktreeRemovalOutcome, WorktreeSessionView,
};
use crate::contexts::workspaces::domain::{
    ManagedWorktreeStatus, WorktreeOrigin, WorktreeProvenance,
};
use crate::platform::database::NativeDatabase;
use crate::platform::instance_lease::InstanceLease;
use crate::test_support::TempDirectory;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

struct NoopLogs;

impl DiagnosticLogPort for NoopLogs {
    fn write_diagnostic(&self, _log: DiagnosticLog) -> Result<(), ApplicationError> {
        Ok(())
    }
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} in {} failed: {}",
        cwd.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn git_status(cwd: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .current_dir(cwd)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .expect("run git")
        .status
        .success()
}

/// A main repository with two commits and a `.gitignore`, plus a sentinel file next to it that
/// no cleanup may touch.
struct RepoFixture {
    _directory: TempDirectory,
    root: PathBuf,
    repo: PathBuf,
    sentinel: PathBuf,
    base_head: String,
}

impl RepoFixture {
    fn new(label: &str) -> Self {
        let directory = TempDirectory::new(label);
        let root = directory.path().to_path_buf();
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).expect("repo dir");
        git(&repo, &["init", "--initial-branch=main"]);
        git(&repo, &["config", "user.email", "tests@example.invalid"]);
        git(&repo, &["config", "user.name", "VaneHub Tests"]);
        git(&repo, &["config", "commit.gpgsign", "false"]);
        std::fs::write(repo.join("README.md"), "base\n").expect("readme");
        std::fs::write(repo.join(".gitignore"), ".env\nbuild/\n").expect("gitignore");
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "first"]);
        std::fs::write(repo.join("src.txt"), "second\n").expect("src");
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-m", "second"]);
        let base_head = git(&repo, &["rev-parse", "HEAD"]);
        let sentinel = root.join("sentinel.txt");
        std::fs::write(&sentinel, "untouched").expect("sentinel");
        Self {
            _directory: directory,
            root,
            repo,
            sentinel,
            base_head,
        }
    }

    fn add_worktree(&self, name: &str) -> PathBuf {
        let target = self.root.join(format!("repo-{name}"));
        git(
            &self.repo,
            &[
                "worktree",
                "add",
                target.to_str().expect("utf8 path"),
                "-b",
                &format!("vanehub/{name}"),
            ],
        );
        target
    }

    fn registered(&self, target: &Path) -> bool {
        let canonical = target.canonicalize().ok();
        git(&self.repo, &["worktree", "list", "--porcelain"])
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .any(|path| {
                Path::new(path) == target
                    || Path::new(path).canonicalize().ok() == canonical.clone()
            })
    }

    fn branch_head(&self, name: &str) -> Option<String> {
        let output = Command::new("git")
            .current_dir(&self.repo)
            .args([
                "rev-parse",
                "--verify",
                &format!("refs/heads/vanehub/{name}"),
            ])
            .output()
            .expect("run git");
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn assert_untouched(&self) {
        assert_eq!(git(&self.repo, &["rev-parse", "HEAD"]), self.base_head);
        assert_eq!(
            std::fs::read_to_string(&self.sentinel).expect("sentinel"),
            "untouched"
        );
        assert_eq!(
            std::fs::read_to_string(self.repo.join("README.md")).expect("readme"),
            "base\n"
        );
        assert!(git_status(&self.repo, &["diff", "--quiet"]));
    }
}

struct Stack {
    _database_dir: TempDirectory,
    service: WorktreeCleanupService,
    probe: Arc<GitWorktreeProbe>,
    lease: InstanceLease,
}

fn stack(label: &str) -> Stack {
    let database_dir = TempDirectory::new(&format!("{label}-db"));
    let database = NativeDatabase::new(database_dir.path().to_path_buf()).expect("database");
    let lease = InstanceLease::acquire(database_dir.path()).expect("lease");
    let clock = Arc::new(SystemWorktreeCleanupClock);
    let probe = Arc::new(GitWorktreeProbe::new(Arc::new(NoopLogs)));
    let service = WorktreeCleanupService::new(
        Arc::new(SqliteManagedWorktreeRepository::new(
            database.clone(),
            clock.clone(),
        )),
        probe.clone(),
        probe.clone(),
        Arc::new(SqliteWorkspaceUseGate::new(
            database,
            lease.clone(),
            clock.clone(),
        )),
        Arc::new(UuidWorktreeIds),
        clock,
    );
    Stack {
        _database_dir: database_dir,
        service,
        probe,
        lease,
    }
}

fn owner(stack: &Stack, operation: &str) -> GateOwner {
    GateOwner {
        instance_id: stack.lease.id().to_string(),
        epoch: stack.lease.epoch(),
        operation_id: operation.to_string(),
    }
}

fn complete_references() -> ReferenceSummary {
    ReferenceSummary {
        external_count: 0,
        completeness: Some(crate::contexts::workspaces::application::CheckCompleteness::Complete),
    }
}

/// Registers intent, creates the worktree, and binds it — the ordinary session creation path.
fn provisioned(stack: &Stack, fixture: &RepoFixture, name: &str) -> (String, PathBuf) {
    let target = fixture.root.join(format!("repo-{name}"));
    let intent = stack
        .service
        .register_intent(
            WorktreeOrigin::OrdinarySession,
            fixture.repo.to_str().expect("utf8"),
            target.to_str().expect("utf8"),
            Some("op-create"),
        )
        .expect("intent");
    assert_eq!(intent.status, ManagedWorktreeStatus::Provisioning);
    fixture.add_worktree(name);
    let record = stack
        .service
        .confirm_created(&intent.id, &format!("session-{name}"))
        .expect("confirm");
    assert_eq!(record.status, ManagedWorktreeStatus::Attached);
    assert_eq!(record.provenance, WorktreeProvenance::Verified);
    assert!(record.cleanup_eligible());
    (intent.id, target)
}

#[test]
fn a_clean_verified_worktree_is_removed_without_force_and_the_branch_survives() {
    let fixture = RepoFixture::new("cleanup-remove");
    let stack = stack("cleanup-remove");
    let (worktree_id, target) = provisioned(&stack, &fixture, "feature");
    let branch_head_before = fixture.branch_head("feature").expect("branch");
    assert!(fixture.registered(&target));

    let inspection = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    assert!(inspection.identity_matches);
    let probe = &inspection.probe;
    assert!(probe.is_linked && probe.registered && probe.branch_resolves_to_head);
    assert!(!probe.detached && !probe.locked && !probe.nested_layout);
    assert_eq!(
        probe
            .anchor
            .as_deref()
            .map(|a| Path::new(a).canonicalize().ok()),
        Some(fixture.repo.canonicalize().ok())
    );
    assert_eq!(
        probe.changes.as_ref().map(|c| c.has_non_ignored_changes()),
        Some(false)
    );
    assert_eq!(probe.ignored.as_ref().map(|i| i.total_entries), Some(0));
    let evaluation = evaluate_cleanup(&inspection, complete_references(), false);
    assert!(evaluation.allows_removal(), "{:?}", evaluation.blockers);
    assert!(!evaluation.requires_ignored_acknowledgement);

    let claim = stack
        .service
        .claim_gate(&worktree_id, &owner(&stack, "op-1"))
        .expect("gate");
    assert!(stack
        .service
        .foreign_gate_holder(
            &inspection.record.identity.as_ref().unwrap().canonical_root,
            Some(&claim.owner)
        )
        .expect("holder")
        .is_none());
    let record = stack
        .service
        .begin_removal(&worktree_id, inspection.record.revision)
        .expect("removing");
    assert_eq!(record.status, ManagedWorktreeStatus::Removing);
    let identity = inspection.record.identity.clone().expect("identity");
    let report = stack
        .service
        .remove_safely(&worktree_id, &identity, &claim)
        .expect("remove");
    assert_eq!(report.outcome, WorktreeRemovalOutcome::Succeeded);
    assert!(report.observation.confirmed_removed());

    assert!(!target.exists(), "target directory removed");
    assert!(!fixture.registered(&target), "registration removed");
    assert_eq!(
        fixture.branch_head("feature"),
        Some(branch_head_before),
        "branch preserved"
    );
    fixture.assert_untouched();
    stack
        .service
        .finalize_removed(&worktree_id, &["session-feature".to_string()])
        .expect("finalize");
    let observed = stack
        .service
        .observe(&worktree_id)
        .expect("observe")
        .expect("identity");
    assert!(observed.confirmed_removed());
    stack.service.release_gate(&claim).expect("release");
}

#[test]
fn keep_leaves_directory_registration_branch_and_record() {
    let fixture = RepoFixture::new("cleanup-keep");
    let stack = stack("cleanup-keep");
    let (worktree_id, target) = provisioned(&stack, &fixture, "kept");
    stack
        .service
        .finalize_retained(&worktree_id, &["session-kept".to_string()])
        .expect("retain");
    assert!(target.join("README.md").exists());
    assert!(fixture.registered(&target));
    assert!(fixture.branch_head("kept").is_some());
    assert!(stack
        .service
        .bound_sessions(&worktree_id)
        .expect("bound")
        .is_empty());
    let inspection = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    assert_eq!(inspection.record.status, ManagedWorktreeStatus::Retained);
    assert!(
        inspection.record.cleanup_eligible(),
        "retained records can still be cleaned later"
    );
    fixture.assert_untouched();
}

#[test]
fn uncommitted_content_of_every_kind_blocks_and_git_itself_refuses_a_non_forced_remove() {
    let fixture = RepoFixture::new("cleanup-dirty");
    let stack = stack("cleanup-dirty");
    let (worktree_id, target) = provisioned(&stack, &fixture, "dirty");

    std::fs::write(target.join("README.md"), "changed\n").expect("modify");
    let inspection = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    let evaluation = evaluate_cleanup(&inspection, complete_references(), false);
    assert!(evaluation.blockers.contains(&reason::TRACKED_CHANGES));
    assert!(!evaluation.allows_removal());

    git(&target, &["add", "README.md"]);
    let inspection = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    assert!(evaluate_cleanup(&inspection, complete_references(), false)
        .blockers
        .contains(&reason::STAGED_CHANGES));

    git(&target, &["checkout", "--", "README.md"]);
    git(&target, &["reset", "-q"]);
    std::fs::write(target.join("notes.txt"), "new\n").expect("untracked");
    let inspection = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    assert!(evaluate_cleanup(&inspection, complete_references(), false)
        .blockers
        .contains(&reason::UNTRACKED_FILES));

    // Even if a caller got past the policy, the command itself carries no force flag.
    let claim = stack
        .service
        .claim_gate(&worktree_id, &owner(&stack, "op-dirty"))
        .expect("gate");
    let record = stack
        .service
        .begin_removal(&worktree_id, inspection.record.revision)
        .expect("removing");
    let identity = record.identity.clone().expect("identity");
    let report = stack
        .service
        .remove_safely(&worktree_id, &identity, &claim)
        .expect("remove");
    assert!(
        matches!(report.outcome, WorktreeRemovalOutcome::Refused { .. }),
        "{:?}",
        report.outcome
    );
    assert!(report.observation.confirmed_intact());
    assert!(target.join("notes.txt").exists());
    assert!(fixture.registered(&target));
    stack
        .service
        .removal_refused(&worktree_id)
        .expect("refused");
    assert_eq!(
        stack
            .service
            .inspect(&worktree_id, &ProbeBudget::DEFAULT)
            .expect("inspect")
            .record
            .status,
        ManagedWorktreeStatus::Attached
    );
    fixture.assert_untouched();
}

#[test]
fn ignored_files_require_an_acknowledgement_bound_to_a_fingerprint_that_tracks_the_files() {
    let fixture = RepoFixture::new("cleanup-ignored");
    let stack = stack("cleanup-ignored");
    let (worktree_id, target) = provisioned(&stack, &fixture, "ignored");
    std::fs::write(target.join(".env"), "SECRET=1\n").expect("env");
    std::fs::create_dir_all(target.join("build")).expect("build");
    std::fs::write(target.join("build/out.o"), "o").expect("out");

    let inspection = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    let evaluation = evaluate_cleanup(&inspection, complete_references(), false);
    assert!(evaluation.allows_removal(), "{:?}", evaluation.blockers);
    assert!(evaluation.requires_ignored_acknowledgement);
    let inventory = inspection.probe.ignored.clone().expect("inventory");
    assert_eq!(inventory.total_entries, 2, "{inventory:?}");
    assert!(inventory.samples.iter().any(|entry| entry.path == ".env"));
    assert!(!inventory
        .samples
        .iter()
        .any(|entry| entry.path.contains("SECRET")));
    let fingerprint = inventory.fingerprint.clone();

    std::fs::write(target.join(".env"), "SECRET=1\nMORE=2\n").expect("env changed");
    let again = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    assert_ne!(
        again.probe.ignored.expect("inventory").fingerprint,
        fingerprint
    );
}

#[test]
fn main_workspaces_plain_directories_nested_layouts_and_links_are_refused() {
    let fixture = RepoFixture::new("cleanup-topology");
    let stack = stack("cleanup-topology");

    let main = stack.probe.probe(&fixture.repo, &ProbeBudget::DEFAULT);
    assert!(main.identity.is_some());
    assert!(!main.is_linked, "the main worktree is never linked");

    let plain = fixture.root.join("plain");
    std::fs::create_dir_all(&plain).expect("plain");
    let plain_probe = stack.probe.probe(&plain, &ProbeBudget::DEFAULT);
    assert!(plain_probe.root_exists && plain_probe.identity.is_none());

    let missing = stack
        .probe
        .probe(&fixture.root.join("missing"), &ProbeBudget::DEFAULT);
    assert!(!missing.root_exists);

    // A worktree created inside the main worktree: the historical `repo/src-feature` layout.
    let nested = fixture.repo.join("src-nested");
    git(
        &fixture.repo,
        &[
            "worktree",
            "add",
            nested.to_str().unwrap(),
            "-b",
            "vanehub/nested",
        ],
    );
    let nested_probe = stack.probe.probe(&nested, &ProbeBudget::DEFAULT);
    assert!(nested_probe.nested_layout, "{nested_probe:?}");

    #[cfg(unix)]
    {
        let real = fixture.add_worktree("linked");
        let link = fixture.root.join("repo-link");
        std::os::unix::fs::symlink(&real, &link).expect("symlink");
        let link_probe = stack.probe.probe(&link, &ProbeBudget::DEFAULT);
        assert_eq!(link_probe.unsupported_layout, Some("symlink_root"));
        assert!(
            link_probe.identity.is_none(),
            "a link is never resolved to its target"
        );
    }
    fixture.assert_untouched();
}

#[test]
fn detached_heads_locked_worktrees_and_in_progress_operations_block() {
    let fixture = RepoFixture::new("cleanup-state");
    let stack = stack("cleanup-state");
    let (worktree_id, target) = provisioned(&stack, &fixture, "state");

    git(&target, &["checkout", "--detach", "-q"]);
    let inspection = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    let evaluation = evaluate_cleanup(&inspection, complete_references(), false);
    assert!(
        evaluation.blockers.contains(&reason::DETACHED_HEAD),
        "{:?}",
        evaluation.blockers
    );
    git(&target, &["checkout", "-q", "vanehub/state"]);

    git(
        &fixture.repo,
        &["worktree", "lock", target.to_str().unwrap()],
    );
    let inspection = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    assert!(evaluate_cleanup(&inspection, complete_references(), false)
        .blockers
        .contains(&reason::LOCKED));
    git(
        &fixture.repo,
        &["worktree", "unlock", target.to_str().unwrap()],
    );

    std::fs::write(
        fixture.repo.join(".git/worktrees/repo-state/MERGE_HEAD"),
        &fixture.base_head,
    )
    .expect("merge marker");
    let inspection = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    assert!(evaluate_cleanup(&inspection, complete_references(), false)
        .blockers
        .contains(&reason::IN_PROGRESS_OPERATION));
    fixture.assert_untouched();
}

#[test]
fn a_replaced_root_never_matches_the_recorded_identity_and_is_not_removed() {
    let fixture = RepoFixture::new("cleanup-replaced");
    let stack = stack("cleanup-replaced");
    let (worktree_id, target) = provisioned(&stack, &fixture, "replaced");
    let recorded = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect")
        .record
        .identity
        .expect("identity");

    // Another actor swaps the directory for a plain one with the same name.
    std::fs::remove_dir_all(&target).expect("remove");
    std::fs::create_dir_all(&target).expect("recreate");
    std::fs::write(target.join("important.txt"), "someone else's data").expect("write");

    let inspection = stack
        .service
        .inspect(&worktree_id, &ProbeBudget::DEFAULT)
        .expect("inspect");
    assert!(!inspection.identity_matches);
    let evaluation = evaluate_cleanup(&inspection, complete_references(), false);
    assert!(!evaluation.allows_removal());
    assert!(evaluation.blockers.contains(&reason::IDENTITY_MISMATCH));

    let claim = stack
        .service
        .claim_gate(&worktree_id, &owner(&stack, "op-replaced"))
        .expect("gate");
    // Even with a forged transition, the removal step re-checks identity and refuses.
    let record = stack
        .service
        .begin_removal(&worktree_id, inspection.record.revision);
    assert!(
        record.is_err(),
        "a record that is no longer eligible cannot begin removal"
    );
    let observation = stack.probe.observe(&recorded, Some(&fixture.repo));
    assert!(!observation.confirmed_removed());
    assert!(!observation.confirmed_intact());
    assert_eq!(observation.identity_matches, Some(false));
    assert!(
        target.join("important.txt").exists(),
        "the new object is untouched"
    );
    stack.service.release_gate(&claim).expect("release");
}

#[test]
fn legacy_sessions_are_verified_only_with_complete_evidence() {
    let fixture = RepoFixture::new("cleanup-legacy");
    let stack = stack("cleanup-legacy");
    let target = fixture.add_worktree("legacy");
    let view = |evidence: bool, branch: &str| WorktreeSessionView {
        session_id: "session-legacy".to_string(),
        worktree_path: Some(target.to_string_lossy().to_string()),
        worktree_branch: Some(branch.to_string()),
        project_path: Some(fixture.repo.to_string_lossy().to_string()),
        remote: false,
        loop_owned: false,
        creation_evidence: evidence,
    };
    let unverified = stack
        .service
        .resolve_for_session(&view(false, "vanehub/legacy"))
        .expect("resolve")
        .expect("resolution");
    assert!(unverified.record.is_none());
    assert_eq!(unverified.provenance_reason, reason::PROVENANCE_UNVERIFIED);

    let wrong_branch = stack
        .service
        .resolve_for_session(&view(true, "vanehub/other"))
        .expect("resolve")
        .expect("resolution");
    assert!(
        wrong_branch.record.is_none(),
        "a branch that does not match is not evidence"
    );

    let verified = stack
        .service
        .resolve_for_session(&view(true, "vanehub/legacy"))
        .expect("resolve")
        .expect("resolution");
    let record = verified.record.expect("legacy record");
    assert_eq!(record.provenance, WorktreeProvenance::LegacyVerified);
    assert!(record.cleanup_eligible());
    assert_eq!(
        stack.service.bound_sessions(&record.id).expect("bound"),
        vec!["session-legacy"]
    );
    // Resolving again finds the same record rather than creating a second one.
    let again = stack
        .service
        .resolve_for_session(&view(true, "vanehub/legacy"))
        .expect("resolve")
        .expect("resolution");
    assert_eq!(again.record.map(|r| r.id), Some(record.id));

    let loop_view = WorktreeSessionView {
        loop_owned: true,
        session_id: "loop-session".to_string(),
        ..view(true, "vanehub/legacy")
    };
    let loop_resolution = stack
        .service
        .resolve_for_session(&loop_view)
        .expect("resolve")
        .expect("resolution");
    // The record exists now and is found by root; provenance still comes from the record, not the Loop flag.
    assert!(loop_resolution.record.is_some());
}

#[test]
fn gates_are_exclusive_across_operations_and_paths_under_a_gated_root_are_refused() {
    let fixture = RepoFixture::new("cleanup-gate");
    let stack = stack("cleanup-gate");
    let (worktree_id, target) = provisioned(&stack, &fixture, "gated");
    let claim = stack
        .service
        .claim_gate(&worktree_id, &owner(&stack, "op-a"))
        .expect("gate");
    assert!(stack
        .service
        .claim_gate(&worktree_id, &owner(&stack, "op-b"))
        .is_err());
    assert!(stack
        .service
        .is_path_gated(target.join("src").to_str().unwrap())
        .expect("gated"));
    assert!(stack
        .service
        .is_path_gated(target.to_str().unwrap())
        .expect("gated"));
    assert!(!stack
        .service
        .is_path_gated(fixture.repo.to_str().unwrap())
        .expect("gated"));
    stack.service.release_gate(&claim).expect("release");
    assert!(!stack
        .service
        .is_path_gated(target.to_str().unwrap())
        .expect("gated"));
}

#[test]
fn a_failed_creation_marks_the_intent_for_attention_instead_of_deleting_anything() {
    let fixture = RepoFixture::new("cleanup-intent");
    let stack = stack("cleanup-intent");
    let target = fixture.root.join("repo-never");
    let intent = stack
        .service
        .register_intent(
            WorktreeOrigin::OrdinarySession,
            fixture.repo.to_str().unwrap(),
            target.to_str().unwrap(),
            None,
        )
        .expect("intent");
    // Git never ran: confirming finds no worktree and parks the record.
    let record = stack
        .service
        .confirm_created(&intent.id, "session-never")
        .expect("confirm");
    assert_eq!(record.status, ManagedWorktreeStatus::NeedsAttention);
    assert!(!record.cleanup_eligible());
    assert!(!target.exists());
    fixture.assert_untouched();
}
