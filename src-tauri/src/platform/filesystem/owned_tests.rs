//! What the ownership checks refuse, including the substitutions they exist for.

use super::{
    create_owned_directory, ensure_owned_root, remove_owned_tree, verify_owned, OwnershipError,
};
use crate::test_support::TempDirectory;
use std::path::Path;

/// Creates a directory symlink, or reports that this platform will not let the test run.
///
/// On Windows a directory symlink needs either developer mode or elevation, so the tests that use
/// one skip rather than fail: a machine without the privilege is not a machine where the guard is
/// broken.
fn link_directory(target: &Path, link: &Path) -> bool {
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(target, link).is_ok()
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

#[test]
fn a_root_is_created_once_and_confirmed_to_be_a_real_directory() {
    let home = TempDirectory::new("owned-root");
    let root = home.path().join("extensions");

    assert_eq!(ensure_owned_root(&root), Ok(()));
    assert!(root.is_dir());
    assert_eq!(ensure_owned_root(&root), Ok(()), "and again");
}

#[test]
fn a_root_that_is_a_file_is_refused_rather_than_written_through() {
    let home = TempDirectory::new("owned-root-file");
    let root = home.path().join("extensions");
    std::fs::write(&root, b"not a directory").expect("write file");

    assert_eq!(ensure_owned_root(&root), Err(OwnershipError::NotOwned));
}

#[test]
fn directories_are_created_beneath_the_root_and_confirmed_afterwards() {
    let home = TempDirectory::new("owned-create");
    let root = home.path().join("extensions");
    let target = root.join("packages/sha256/abc");

    assert_eq!(create_owned_directory(&root, &target), Ok(()));
    assert!(target.is_dir());
    assert_eq!(verify_owned(&root, &target), Ok(()));
    assert_eq!(
        create_owned_directory(&root, &target),
        Ok(()),
        "an existing directory is inspected, not blindly reused"
    );
}

#[test]
fn a_path_outside_the_root_is_refused_before_anything_is_touched() {
    let home = TempDirectory::new("owned-outside");
    let root = home.path().join("extensions");
    let elsewhere = home.path().join("elsewhere");

    assert_eq!(
        create_owned_directory(&root, &elsewhere),
        Err(OwnershipError::OutsideRoot)
    );
    assert!(!elsewhere.exists());

    // `strip_prefix` compares components rather than resolving them, so a traversal inside the
    // remainder would strip successfully and then be walked. It is refused explicitly.
    assert_eq!(
        create_owned_directory(&root, &root.join("..").join("escape")),
        Err(OwnershipError::OutsideRoot)
    );
}

#[test]
fn a_component_replaced_by_a_link_is_refused_rather_than_followed() {
    let home = TempDirectory::new("owned-link");
    let root = home.path().join("extensions");
    let outside = home.path().join("outside");
    std::fs::create_dir_all(&outside).expect("outside directory");
    std::fs::create_dir_all(&root).expect("root");

    if !link_directory(&outside, &root.join("packages")) {
        return;
    }

    // This is the substitution the whole module exists for: `create_dir_all` would have happily
    // created `outside/sha256/abc` and every later write would have landed there.
    assert_eq!(
        create_owned_directory(&root, &root.join("packages/sha256/abc")),
        Err(OwnershipError::NotOwned)
    );
    assert!(!outside.join("sha256").exists());
    assert_eq!(
        verify_owned(&root, &root.join("packages")),
        Err(OwnershipError::NotOwned)
    );
}

#[test]
fn a_root_replaced_by_a_link_is_refused() {
    let home = TempDirectory::new("owned-link-root");
    let outside = home.path().join("outside");
    std::fs::create_dir_all(&outside).expect("outside directory");
    let root = home.path().join("extensions");

    if !link_directory(&outside, &root) {
        return;
    }

    assert_eq!(ensure_owned_root(&root), Err(OwnershipError::NotOwned));
}

#[test]
fn removal_confirms_ownership_first_and_treats_an_absent_path_as_done() {
    let home = TempDirectory::new("owned-remove");
    let root = home.path().join("extensions");
    let target = root.join("scratch/install-1/generation-1");
    create_owned_directory(&root, &target).expect("create");
    std::fs::write(target.join("file.bin"), b"content").expect("write");

    assert_eq!(remove_owned_tree(&root, &target), Ok(()));
    assert!(!target.exists());
    assert_eq!(
        remove_owned_tree(&root, &target),
        Ok(()),
        "cleanup runs on failure paths and at startup, where already-gone is the outcome wanted"
    );
    assert_eq!(
        remove_owned_tree(&root, &home.path().join("elsewhere")),
        Err(OwnershipError::OutsideRoot)
    );
}

#[test]
fn removal_refuses_a_tree_reached_through_a_link() {
    let home = TempDirectory::new("owned-remove-link");
    let root = home.path().join("extensions");
    let outside = home.path().join("outside");
    std::fs::create_dir_all(outside.join("precious")).expect("outside tree");
    std::fs::write(outside.join("precious/file.bin"), b"content").expect("write");
    std::fs::create_dir_all(&root).expect("root");

    if !link_directory(&outside, &root.join("scratch")) {
        return;
    }

    assert_eq!(
        remove_owned_tree(&root, &root.join("scratch/precious")),
        Err(OwnershipError::NotOwned)
    );
    assert!(
        outside.join("precious/file.bin").is_file(),
        "a cleanup helper that followed a link would have deleted somebody else's tree"
    );
}

#[test]
fn every_error_has_a_distinct_stable_code() {
    let mut codes = [
        OwnershipError::OutsideRoot.code(),
        OwnershipError::NotOwned.code(),
        OwnershipError::Io.code(),
    ];
    let total = codes.len();
    codes.sort_unstable();
    let mut deduped = codes.to_vec();
    deduped.dedup();
    assert_eq!(deduped.len(), total);
}
