use super::*;
use std::io::Write;
use std::time::{Duration, SystemTime};

#[test]
fn creates_unique_versioned_directory_and_exclusive_file() {
    let root = crate::test_support::TempDirectory::new("private-relay-fs");
    let first = PrivateRelayDirectory::create_in(root.path()).expect("first directory");
    let second = PrivateRelayDirectory::create_in(root.path()).expect("second directory");
    assert_ne!(first.path(), second.path());
    assert_eq!(
        first.path().parent().and_then(Path::file_name),
        Some("v1".as_ref())
    );

    let mut file = first.create_file("server.json").expect("private file");
    file.write_all(b"secret").expect("write");
    assert_eq!(
        first
            .create_file("server.json")
            .expect_err("create_new")
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    assert!(first.create_file("../escape.json").is_err());
}

#[test]
fn directory_and_guard_creation_failures_leave_no_owned_artifacts() {
    let root = crate::test_support::TempDirectory::new("private-relay-create-failure");
    let blocked_root = root.write("blocked-root", "not-a-directory");
    assert!(PrivateRelayDirectory::create_in(&blocked_root).is_err());
    assert!(blocked_root.is_file());
    assert!(!blocked_root.join(RELAY_CACHE_VERSION).exists());

    let directory = PrivateRelayDirectory::create_in(root.path()).expect("directory");
    let invocation_path = directory.path().to_path_buf();
    fs::remove_dir(&invocation_path).expect("inject missing invocation directory");
    assert!(directory.guard().is_err());
    assert!(!invocation_path.exists());
}

#[test]
fn artifact_creation_failure_remains_owned_by_idempotent_cleanup() {
    let root = crate::test_support::TempDirectory::new("private-relay-file-failure");
    let directory = PrivateRelayDirectory::create_in(root.path()).expect("directory");
    let invocation_path = directory.path().to_path_buf();
    let guard = directory.guard().expect("guard");
    let mut file = directory.create_file("secret.json").expect("secret file");
    file.write_all(b"raw-relay-artifact-secret")
        .expect("write secret");
    drop(file);

    assert_eq!(
        directory
            .create_file("secret.json")
            .expect_err("exclusive creation failure")
            .kind(),
        io::ErrorKind::AlreadyExists
    );
    guard.cleanup().expect("cleanup after creation failure");
    guard.cleanup().expect("idempotent cleanup");
    assert!(!invocation_path.exists());
}

#[test]
fn guard_cleanup_is_recursive_and_idempotent() {
    let root = crate::test_support::TempDirectory::new("private-relay-cleanup");
    let directory = PrivateRelayDirectory::create_in(root.path()).expect("directory");
    let path = directory.path().to_path_buf();
    directory.create_file("server.json").expect("file");
    let guard = directory.guard().expect("guard");

    guard.cleanup().expect("first cleanup");
    guard.cleanup().expect("second cleanup");
    assert!(!path.exists());
}

#[test]
fn cloned_guard_keeps_the_directory_owned_until_the_last_lifecycle_releases_it() {
    let root = crate::test_support::TempDirectory::new("private-relay-clone");
    let directory = PrivateRelayDirectory::create_in(root.path()).expect("directory");
    let path = directory.path().to_path_buf();
    let guard = directory.guard().expect("guard");
    let process_guard = guard.clone();

    drop(guard);
    assert!(path.exists());
    drop(process_guard);
    assert!(!path.exists());
}

#[test]
fn scavenger_removes_only_stale_owned_directories() {
    let root = crate::test_support::TempDirectory::new("private-relay-scavenger");
    let stale = PrivateRelayDirectory::create_in(root.path()).expect("stale directory");
    let stale_path = stale.path().to_path_buf();
    let stale_guard = stale.guard().expect("stale guard");
    let cutoff = SystemTime::now();
    std::thread::sleep(Duration::from_millis(50));
    let fresh = PrivateRelayDirectory::create_in(root.path()).expect("fresh directory");
    let fresh_path = fresh.path().to_path_buf();
    let fresh_guard = fresh.guard().expect("fresh guard");
    let version_root = fresh_path.parent().expect("version root");
    let unrelated = version_root.join("unrelated-directory");
    fs::create_dir(&unrelated).expect("unrelated directory");
    fs::write(version_root.join("unrelated-file"), b"keep").expect("unrelated file");

    scavenge_stale_in(root.path(), cutoff).expect("scavenge");

    assert!(!stale_path.exists());
    assert!(fresh_path.exists());
    assert!(unrelated.exists());
    assert!(version_root.join("unrelated-file").exists());
    drop(stale_guard);
    drop(fresh_guard);
}

#[test]
fn scavenger_never_follows_an_invocation_symlink_outside_the_version_root() {
    let root = crate::test_support::TempDirectory::new("private-relay-containment");
    let directory = PrivateRelayDirectory::create_in(root.path()).expect("directory");
    let version_root = directory
        .path()
        .parent()
        .expect("version root")
        .to_path_buf();
    let guard = directory.guard().expect("guard");
    let outside = root.path().join("outside");
    fs::create_dir(&outside).expect("outside");
    fs::write(outside.join("keep.txt"), b"keep").expect("outside file");
    let link = version_root.join("invocation-stale-link");
    if create_directory_symlink(&outside, &link).is_err() {
        return;
    }

    scavenge_stale_in(root.path(), SystemTime::now() + Duration::from_secs(60)).expect("scavenge");

    assert!(outside.join("keep.txt").exists());
    assert!(fs::symlink_metadata(link).is_ok());
    drop(guard);
}

#[test]
fn aborted_preparation_drops_every_partially_written_artifact() {
    let root = crate::test_support::TempDirectory::new("private-relay-partial");
    let owned_path = {
        let directory = PrivateRelayDirectory::create_in(root.path()).expect("directory");
        let path = directory.path().to_path_buf();
        let guard = directory.guard().expect("guard");
        directory.create_file("first.json").expect("first file");
        directory.create_file("second.json").expect("second file");
        drop(guard);
        path
    };
    assert!(!owned_path.exists());
}

#[cfg(windows)]
#[test]
fn windows_directory_and_file_dacls_allow_only_the_current_user() {
    let root = crate::test_support::TempDirectory::new("private-relay-windows-acl");
    let directory = PrivateRelayDirectory::create_in(root.path()).expect("directory");
    let file_path = directory.path().join("server.json");
    drop(directory.create_file("server.json").expect("file"));

    let user = windows_acl_report::current_process_user_sid().expect("current user SID");
    let readings = [
        ("directory", directory.path().to_path_buf()),
        ("file", file_path),
    ];

    // Read both before asserting either, so one run reports the whole picture instead of
    // stopping at whichever object happens to be examined first.
    let mut failures = String::new();
    for (label, path) in &readings {
        let reading =
            windows_acl_report::read_dacl(path, user.clone()).expect("read DACL for reading");
        // Printed on success too. A security contract that only speaks when it breaks gives
        // nobody a baseline to compare the break against.
        eprintln!("{}", reading.describe(label, path));
        if !reading.satisfies_private_current_user_contract() {
            failures.push_str(&reading.describe(label, path));
        }
    }
    assert!(failures.is_empty(), "{failures}");
}

/// Applies a deliberately wrong DACL to a real directory and returns what the check makes of it.
///
/// Written against the filesystem rather than by hand-building a `DaclReading`, because half of
/// what is under test is whether the *reader* can see the defect at all. A check that judges a
/// struct correctly but cannot observe an extra ACE on disk would still pass every negative case
/// and still miss a real one.
#[cfg(windows)]
fn reading_after_applying(sddl: &str) -> Vec<String> {
    let root = crate::test_support::TempDirectory::new("private-relay-negative-acl");
    let target = root.path().join("object");
    std::fs::create_dir_all(&target).expect("negative fixture directory");
    let user = windows_acl_report::current_process_user_sid().expect("current user SID");
    windows_acl::apply_sddl_for_tests(&target, &sddl.replace("{user}", &user))
        .expect("apply negative DACL");
    let reading = windows_acl_report::read_dacl(&target, user).expect("read negative DACL");
    reading.violations()
}

#[cfg(windows)]
#[test]
fn the_windows_privacy_check_rejects_every_way_the_dacl_can_be_wrong() {
    // Each case is a DACL that grants more, or differently, than the contract allows. If any of
    // them produced no violation, the check would be decoration: a structural comparison nothing
    // has ever falsified is worth exactly as much as the string comparison it replaced.
    let cases: [(&str, &str); 8] = [
        (
            "extra ACE for Everyone",
            "D:P(A;;FA;;;{user})(A;;FA;;;S-1-1-0)",
        ),
        (
            "extra ACE for Users",
            "D:P(A;;FA;;;{user})(A;;FA;;;S-1-5-32-545)",
        ),
        (
            "extra ACE for Authenticated Users",
            "D:P(A;;FA;;;{user})(A;;FA;;;S-1-5-11)",
        ),
        (
            "missing ACE for the current user",
            "D:P(A;;FA;;;S-1-5-32-544)",
        ),
        ("DACL not protected", "D:(A;;FA;;;{user})"),
        ("inheritance flags set", "D:P(A;OICI;FA;;;{user})"),
        ("wrong access mask", "D:P(A;;FR;;;{user})"),
        (
            // A deny placed after an allow does not deny: Windows stops at the first match. The
            // ordering is the guarantee, which is why the reader never sorts.
            "non-canonical ACE order",
            "D:P(A;;FA;;;{user})(D;;FA;;;S-1-1-0)",
        ),
    ];

    for (label, sddl) in cases {
        let violations = reading_after_applying(sddl);
        assert!(
            !violations.is_empty(),
            "the check accepted a DACL it must reject: {label} ({sddl})"
        );
    }
}

#[cfg(windows)]
#[test]
fn the_windows_privacy_check_names_the_specific_defect_it_found() {
    // Rejecting for the wrong reason is only accidentally correct, and would keep passing if the
    // reader stopped observing the field the case is actually about.
    let everyone = reading_after_applying("D:P(A;;FA;;;{user})(A;;FA;;;S-1-1-0)").join(" ");
    assert!(everyone.contains("S-1-1-0"), "{everyone}");

    let unprotected = reading_after_applying("D:(A;;FA;;;{user})").join(" ");
    assert!(unprotected.contains("not protected"), "{unprotected}");

    let inheritance = reading_after_applying("D:P(A;OICI;FA;;;{user})").join(" ");
    assert!(inheritance.contains("inheritance flags"), "{inheritance}");

    let mask = reading_after_applying("D:P(A;;FR;;;{user})").join(" ");
    assert!(mask.contains("expected 0x001f01ff"), "{mask}");
}

#[cfg(unix)]
fn create_directory_symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_directory_symlink(target: &Path, link: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

#[cfg(unix)]
#[test]
fn unix_permissions_are_private_before_writing() {
    use std::os::unix::fs::PermissionsExt;
    let root = crate::test_support::TempDirectory::new("private-relay-permissions");
    let directory = PrivateRelayDirectory::create_in(root.path()).expect("directory");
    let file = directory.create_file("server.json").expect("file");
    drop(file);
    assert_eq!(
        fs::metadata(directory.path())
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(directory.path().join("server.json"))
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn create_or_truncate_overwrites_a_stale_file_left_by_a_prior_crash() {
    // A caller names this file with a pid plus a process-global counter; once a pid is recycled, a
    // prior crash can leave a stale file at that exact name. `open_private_file`'s `create_new`
    // would turn that into a permanent failure; this primitive exists specifically to overwrite it
    // instead, since the current process already owns the name by construction.
    let root = crate::test_support::TempDirectory::new("private-relay-create-or-truncate");
    let path = root.path().join("stale-temp-file");
    fs::write(&path, b"stale content from a prior crash").expect("seed a stale file");

    let mut file = create_or_truncate_private_file(&path).expect("must overwrite, not fail");
    file.write_all(b"fresh").expect("write");
    drop(file);

    assert_eq!(fs::read(&path).expect("read back"), b"fresh");
}

#[cfg(unix)]
#[test]
fn create_or_truncate_is_private_from_creation() {
    use std::os::unix::fs::PermissionsExt;
    let root = crate::test_support::TempDirectory::new("private-relay-create-or-truncate-mode");
    let path = root.path().join("temp-file");
    drop(create_or_truncate_private_file(&path).expect("create"));
    assert_eq!(
        fs::metadata(&path).expect("metadata").permissions().mode() & 0o777,
        0o600
    );
}
