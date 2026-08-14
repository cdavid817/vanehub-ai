use super::*;
use crate::platform::process::std_command;

fn git(root: &Path, args: &[&str]) {
    let status = std_command("git")
        .expect("command")
        .args(args)
        .current_dir(root)
        .status()
        .expect("git");
    assert!(status.success(), "git {args:?}");
}

fn output(root: &Path, args: &[&str]) -> String {
    String::from_utf8(
        std_command("git")
            .expect("command")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git")
            .stdout,
    )
    .expect("utf8")
    .trim()
    .to_string()
}

#[test]
fn captures_tracked_untracked_binary_and_preserves_real_index() {
    let temp = tempfile::tempdir().expect("temp");
    let root = temp.path().join("repo");
    fs::create_dir(&root).expect("repo");
    git(&root, &["init"]);
    git(&root, &["config", "user.email", "test@example.com"]);
    git(&root, &["config", "user.name", "Test"]);
    fs::write(root.join("tracked.txt"), "before\n").expect("tracked");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "base"]);
    let base = output(&root, &["rev-parse", "HEAD"]);
    let index_before = fs::read(root.join(".git/index")).expect("index");

    fs::write(root.join("tracked.txt"), "after\n").expect("modify");
    fs::write(root.join("new.txt"), "new\n").expect("untracked");
    fs::write(root.join("image.bin"), [0_u8, 1, 2, 0, 3]).expect("binary");
    let control = temp.path().join("control");
    fs::create_dir(&control).expect("control");

    let capture = GitDelegationChangeSetCapture::new()
        .capture(&root, &control, &base)
        .expect("capture");
    assert_eq!(capture.base_commit, base);
    assert_eq!(capture.files.len(), 3);
    assert!(capture.files.iter().any(|file| file.path == "new.txt"));
    assert!(capture
        .files
        .iter()
        .any(|file| file.path == "image.bin" && file.binary));
    assert!(String::from_utf8_lossy(&capture.canonical_patch).contains("GIT binary patch"));
    assert!(capture.diff_hash.starts_with("sha256:"));
    assert_eq!(
        fs::read(root.join(".git/index")).expect("index after"),
        index_before
    );
}

#[test]
fn captures_rename_delete_unicode_and_binary_rename_deterministically() {
    let temp = tempfile::tempdir().expect("temp");
    let root = temp.path().join("repo");
    fs::create_dir(&root).expect("repo");
    git(&root, &["init"]);
    git(&root, &["config", "user.email", "test@example.com"]);
    git(&root, &["config", "user.name", "Test"]);
    fs::write(root.join("rename.txt"), "rename me\n").expect("rename source");
    fs::write(root.join("delete.txt"), "delete me\n").expect("delete source");
    fs::write(root.join("binary-old.bin"), [0_u8, 1, 0, 2]).expect("binary source");
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "base"]);
    let base = output(&root, &["rev-parse", "HEAD"]);

    fs::rename(root.join("rename.txt"), root.join("新名称.txt")).expect("unicode rename");
    fs::rename(root.join("binary-old.bin"), root.join("binary-new.bin")).expect("binary rename");
    fs::remove_file(root.join("delete.txt")).expect("delete");
    let first_control = temp.path().join("control-one");
    let second_control = temp.path().join("control-two");
    fs::create_dir(&first_control).expect("control one");
    fs::create_dir(&second_control).expect("control two");

    let adapter = GitDelegationChangeSetCapture::new();
    let first = adapter
        .capture(&root, &first_control, &base)
        .expect("first capture");
    let second = adapter
        .capture(&root, &second_control, &base)
        .expect("second capture");

    assert_eq!(first.diff_hash, second.diff_hash);
    assert_eq!(first.canonical_patch, second.canonical_patch);
    assert!(first.files.iter().any(|file| {
        file.kind == DelegationChangeKind::Renamed
            && file.previous_path.as_deref() == Some("rename.txt")
            && file.path == "新名称.txt"
    }));
    assert!(first
        .files
        .iter()
        .any(|file| file.kind == DelegationChangeKind::Deleted && file.path == "delete.txt"));
    assert!(first.files.iter().any(|file| {
        file.kind == DelegationChangeKind::Renamed && file.path == "binary-new.bin" && file.binary
    }));
}

#[test]
fn parses_mode_change_and_renamed_binary_numstat_protocol() {
    let binary_paths = parse_binary_paths(b"-\t-\t\0old.bin\0new.bin\0").expect("numstat");
    assert_eq!(binary_paths, vec!["new.bin"]);
    let files =
        parse_raw(b":100644 100755 1111111 2222222 M\0script.sh\0", &[]).expect("raw mode change");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].before_mode.as_deref(), Some("100644"));
    assert_eq!(files[0].after_mode.as_deref(), Some("100755"));
}
