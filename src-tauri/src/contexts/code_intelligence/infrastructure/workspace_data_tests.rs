use super::*;

#[test]
fn two_workspaces_never_share_a_directory() {
    let app_data = Path::new("C:/app");
    let first = workspace_data_directory(app_data, "java", Path::new("C:/code/alpha"));
    let second = workspace_data_directory(app_data, "java", Path::new("C:/code/beta"));

    assert_ne!(first, second);
    // Same workspace, same directory: the index has to be found again on the next start, which is
    // the whole reason it is derived rather than freshly generated.
    assert_eq!(
        first,
        workspace_data_directory(app_data, "java", Path::new("C:/code/alpha"))
    );
}

#[test]
fn the_workspace_path_never_appears_in_the_directory_name() {
    let directory = workspace_data_directory(
        Path::new("C:/app"),
        "java",
        Path::new("C:/code/secret-client"),
    );
    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .expect("directory name");

    assert!(!name.contains("secret-client"));
    assert!(!name.contains("code"));
    // A truncated SHA-256 rendered as hex.
    assert_eq!(name.len(), 32);
    assert!(name.chars().all(|character| character.is_ascii_hexdigit()));
}

#[test]
fn two_languages_never_share_a_directory() {
    let app_data = Path::new("C:/app");
    let root = Path::new("C:/code/alpha");

    assert_ne!(
        workspace_data_directory(app_data, "java", root),
        workspace_data_directory(app_data, "kotlin", root)
    );
}

#[test]
fn revocation_removes_the_directory_rather_than_reporting_that_it_tried() {
    let app_data = tempfile::tempdir().expect("app data");
    let root = Path::new("C:/code/alpha");
    let directory = workspace_data_directory(app_data.path(), "java", root);
    std::fs::create_dir_all(directory.join("index")).expect("create index");
    std::fs::write(directory.join("index/contents"), b"indexed source").expect("write index");
    assert!(directory.exists());

    remove_workspace_data(app_data.path(), ["java"], root);

    // Asserted on the filesystem, not on a call having been made. This is the one failure here
    // with a privacy shape: a revoked workspace must stop having a server-built index of its
    // source on disk.
    assert!(!directory.exists());
}

#[test]
fn revoking_one_workspace_leaves_another_alone() {
    let app_data = tempfile::tempdir().expect("app data");
    let kept = workspace_data_directory(app_data.path(), "java", Path::new("C:/code/beta"));
    std::fs::create_dir_all(&kept).expect("create kept");

    remove_workspace_data(app_data.path(), ["java"], Path::new("C:/code/alpha"));

    assert!(kept.exists());
}

#[test]
fn removing_a_directory_that_was_never_created_is_not_an_error() {
    let app_data = tempfile::tempdir().expect("app data");

    // Revocation runs for every registered language, and most of them have no data directory at
    // all. It has to be uneventful for those rather than something a caller has to filter.
    remove_workspace_data(
        app_data.path(),
        ["java", "rust"],
        Path::new("C:/code/alpha"),
    );
}
