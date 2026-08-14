use super::*;
use crate::platform::process::std_command;

fn git(root: &Path, arguments: &[&str]) -> String {
    let output = std_command("git")
        .expect("command")
        .args(arguments)
        .current_dir(root)
        .output()
        .expect("git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf8")
        .trim()
        .to_owned()
}

fn source_repository() -> (tempfile::TempDir, String) {
    let root = tempfile::tempdir().expect("source");
    git(root.path(), &["init"]);
    git(root.path(), &["config", "user.name", "Fixture"]);
    git(
        root.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    std::fs::write(root.path().join("tracked.txt"), "baseline").expect("file");
    git(root.path(), &["add", "tracked.txt"]);
    git(root.path(), &["commit", "-m", "fixture"]);
    let commit = git(root.path(), &["rev-parse", "HEAD"]);
    (root, commit)
}

#[test]
fn exact_commit_is_cloned_into_independent_detached_no_remote_repository() {
    let (source, commit) = source_repository();
    let operations = tempfile::tempdir().expect("operations");
    let adapter = IndependentGitWorkspaceAdapter::new(operations.path().to_path_buf());
    let workspace = adapter.create(source.path(), &commit).expect("workspace");

    assert_eq!(git(&workspace.workspace, &["rev-parse", "HEAD"]), commit);
    let symbolic = std_command("git")
        .expect("command")
        .args(["symbolic-ref", "-q", "HEAD"])
        .current_dir(&workspace.workspace)
        .output()
        .expect("symbolic ref");
    assert!(!symbolic.status.success());
    assert!(git(&workspace.workspace, &["remote"]).is_empty());
    assert!(!workspace
        .workspace
        .join(".git/objects/info/alternates")
        .exists());
    assert!(workspace.inputs.is_dir());
    assert!(workspace.output.is_dir());
    assert!(workspace.control.is_dir());
    assert!(workspace.recovery.is_dir());

    adapter.cleanup(&workspace).expect("cleanup");
    assert!(!workspace.attempt_root.exists());
}

#[test]
fn invalid_commit_is_rejected_before_creating_an_attempt_directory() {
    let (source, _) = source_repository();
    let operations = tempfile::tempdir().expect("operations");
    let adapter = IndependentGitWorkspaceAdapter::new(operations.path().to_path_buf());
    assert_eq!(
        adapter.create(source.path(), "HEAD"),
        Err(DelegationWorkspaceError::InvalidRequest)
    );
    assert_eq!(
        std::fs::read_dir(operations.path())
            .expect("entries")
            .count(),
        0
    );
}

#[test]
fn baseline_requires_exact_clean_head_and_rejects_in_progress_git_state() {
    let (source, commit) = source_repository();
    let operations = tempfile::tempdir().expect("operations");
    let adapter = IndependentGitWorkspaceAdapter::new(operations.path().to_path_buf());
    let baseline = adapter
        .inspect_baseline(source.path(), &commit)
        .expect("baseline");
    assert_eq!(baseline.head_commit, commit);
    assert_eq!(baseline.tracked_files, 1);
    assert!(baseline.repository_identity.starts_with("git:"));

    std::fs::write(source.path().join("tracked.txt"), "dirty").expect("dirty");
    assert_eq!(
        adapter.inspect_baseline(source.path(), &commit),
        Err(DelegationWorkspaceError::VerificationFailure)
    );
    git(source.path(), &["checkout", "--", "tracked.txt"]);
    std::fs::write(source.path().join(".git/MERGE_HEAD"), &commit).expect("merge state");
    assert_eq!(
        adapter.inspect_baseline(source.path(), &commit),
        Err(DelegationWorkspaceError::VerificationFailure)
    );
}
