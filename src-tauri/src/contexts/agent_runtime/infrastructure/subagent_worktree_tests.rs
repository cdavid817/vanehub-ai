use super::*;

/// Uses the same platform adapter production does, rather than spawning a process directly --
/// `tests/architecture.rs` enforces that no source file constructs an external process itself.
fn git(root: &Path, args: &[&str]) {
    let owned = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
    let output = GitAdapter::default()
        .execute(root, &owned, GIT_TIMEOUT)
        .expect("git");
    assert!(output.status.success(), "git {args:?} failed");
}

/// A repository with one commit, which is what `provision` requires: a base commit to bind to and
/// a clean tree.
fn repository(label: &str) -> crate::test_support::TempDirectory {
    let directory = crate::test_support::TempDirectory::new(label);
    let root = directory.path();
    git(root, &["init", "--quiet"]);
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    git(root, &["config", "commit.gpgsign", "false"]);
    std::fs::write(root.join("seed.txt"), "seed\n").expect("seed");
    git(root, &["add", "."]);
    git(root, &["commit", "--quiet", "-m", "seed"]);
    directory
}

#[test]
fn a_non_repository_workspace_is_refused() {
    let directory = crate::test_support::TempDirectory::new("subagent-worktree-not-repo");
    let operations = crate::test_support::TempDirectory::new("subagent-worktree-ops-1");

    assert_eq!(
        ChildWorktree::provision(directory.path(), operations.path()).err(),
        Some(WorktreeError::NotARepository)
    );
}

/// A ChangeSet captured against a dirty tree cannot state what it applies to, so this refuses
/// rather than capturing something unreviewable.
#[test]
fn a_dirty_workspace_is_refused() {
    let repository = repository("subagent-worktree-dirty");
    let operations = crate::test_support::TempDirectory::new("subagent-worktree-ops-2");
    std::fs::write(repository.path().join("seed.txt"), "modified\n").expect("dirty");

    assert_eq!(
        ChildWorktree::provision(repository.path(), operations.path()).err(),
        Some(WorktreeError::WorkspaceNotClean)
    );
}

#[test]
fn a_provisioned_worktree_is_isolated_and_carries_its_base_commit() {
    let repository = repository("subagent-worktree-isolated");
    let operations = crate::test_support::TempDirectory::new("subagent-worktree-ops-3");

    let worktree =
        ChildWorktree::provision(repository.path(), operations.path()).expect("provision");

    assert!(worktree.path().join("seed.txt").exists());
    assert_ne!(worktree.path(), repository.path());
    assert_eq!(worktree.base_commit().len(), 40);
    assert!(worktree.repository_identity().starts_with("repo-"));
}

#[test]
fn edits_stay_inside_the_worktree_and_are_captured() {
    let repository = repository("subagent-worktree-capture");
    let operations = crate::test_support::TempDirectory::new("subagent-worktree-ops-4");
    let worktree =
        ChildWorktree::provision(repository.path(), operations.path()).expect("provision");

    std::fs::write(worktree.path().join("seed.txt"), "changed\n").expect("edit");
    std::fs::write(worktree.path().join("added.txt"), "new file\n").expect("add");

    let captured = worktree.capture().expect("capture");
    let paths: Vec<&str> = captured.iter().map(|file| file.path.as_str()).collect();
    assert!(paths.contains(&"seed.txt"), "{paths:?}");
    assert!(
        paths.contains(&"added.txt"),
        "an untracked file must be captured, not skipped: {paths:?}"
    );
    for file in &captured {
        assert_eq!(file.new_hash.as_ref().map(String::len), Some(64));
        assert!(!file.binary);
    }

    // The parent's workspace is untouched until a ChangeSet is applied.
    assert_eq!(
        std::fs::read_to_string(repository.path().join("seed.txt")).expect("parent"),
        "seed\n"
    );
    assert!(!repository.path().join("added.txt").exists());
}

#[test]
fn the_diff_covers_modified_and_added_files() {
    let repository = repository("subagent-worktree-diff");
    let operations = crate::test_support::TempDirectory::new("subagent-worktree-ops-5");
    let worktree =
        ChildWorktree::provision(repository.path(), operations.path()).expect("provision");

    std::fs::write(worktree.path().join("seed.txt"), "changed\n").expect("edit");
    std::fs::write(worktree.path().join("added.txt"), "new file\n").expect("add");

    let diff = String::from_utf8_lossy(&worktree.diff().expect("diff")).into_owned();
    assert!(diff.contains("seed.txt"), "{diff}");
    assert!(
        diff.contains("added.txt"),
        "an added file must appear in the diff, not just the file list: {diff}"
    );
    assert!(diff.contains("changed"), "{diff}");
}

/// Reaping on drop is what makes every exit path safe -- success, limit, cancellation, or a return
/// added later.
#[test]
fn the_worktree_is_reaped_when_dropped() {
    let repository = repository("subagent-worktree-reap");
    let operations = crate::test_support::TempDirectory::new("subagent-worktree-ops-6");

    let path = {
        let worktree =
            ChildWorktree::provision(repository.path(), operations.path()).expect("provision");
        std::fs::write(worktree.path().join("dirty.txt"), "uncommitted\n").expect("dirty");
        worktree.path().to_path_buf()
    };

    assert!(
        !path.exists(),
        "a dropped worktree must not survive, even with uncommitted files in it"
    );
}

#[test]
fn a_change_set_larger_than_the_bound_is_refused() {
    let repository = repository("subagent-worktree-too-many");
    let operations = crate::test_support::TempDirectory::new("subagent-worktree-ops-7");
    let worktree =
        ChildWorktree::provision(repository.path(), operations.path()).expect("provision");

    for index in 0..=MAX_CHANGE_SET_FILES {
        std::fs::write(worktree.path().join(format!("file-{index}.txt")), "x\n").expect("write");
    }

    assert_eq!(worktree.capture().err(), Some(WorktreeError::TooManyFiles));
}

#[test]
fn status_lines_parse_into_a_status_and_the_post_rename_path() {
    assert_eq!(
        parse_status_line(" M src/main.rs"),
        Some(('M', "src/main.rs".to_owned()))
    );
    assert_eq!(
        parse_status_line("?? added.txt"),
        Some(('?', "added.txt".to_owned()))
    );
    // A rename reports both sides; the ChangeSet carries where the file ended up.
    assert_eq!(
        parse_status_line("R  old.rs -> new.rs"),
        Some(('R', "new.rs".to_owned()))
    );
    assert_eq!(
        parse_status_line("D  gone.rs"),
        Some(('D', "gone.rs".to_owned()))
    );
    assert_eq!(parse_status_line(""), None);
    assert_eq!(parse_status_line("M"), None);
}

#[test]
fn binary_content_is_detected_from_its_bytes() {
    assert!(is_binary(&[0x00, 0x01, 0x02]));
    assert!(!is_binary(b"plain text\n"));
    assert!(!is_binary(b""));
}

#[test]
fn every_worktree_error_carries_an_actionable_message() {
    for error in [
        WorktreeError::NotARepository,
        WorktreeError::WorkspaceNotClean,
        WorktreeError::GitFailure,
        WorktreeError::TooManyFiles,
    ] {
        assert!(!error.message().is_empty());
    }
}
