//! Directories the application owns, and the checks that keep them that way.
//!
//! A root under application data is not automatically application-owned. Anything with write
//! access to the parent can replace a directory with a symlink pointing somewhere else, and every
//! subsequent `create_dir_all`, write, and `remove_dir_all` then operates on that somewhere else.
//! The checks here walk a path one component at a time and refuse the first that is a link or is
//! not a directory, so a substituted component is caught before it is followed rather than after.
//!
//! `symlink_metadata` throughout, never `metadata`: the latter follows links, which would report
//! exactly the thing being checked for as a perfectly ordinary directory.

use std::path::{Component, Path};

/// Why a path is not one the application may operate on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OwnershipError {
    /// The path is not beneath the root it was supposed to be under.
    OutsideRoot,
    /// A component on the way is a link, or is not a directory.
    NotOwned,
    /// The filesystem refused the operation.
    Io,
}

impl OwnershipError {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::OutsideRoot => "path_outside_root",
            Self::NotOwned => "path_not_application_owned",
            Self::Io => "filesystem_error",
        }
    }
}

/// Confirms `root` exists as a real directory, creating it if it is absent.
///
/// Separate from the per-component walk because the root itself has no parent to be checked from,
/// and because a caller usually wants it created once rather than re-verified per operation.
/// Inspects before creating. A root that is already a file or a link is reported as not owned
/// rather than as whatever error `create_dir_all` happens to produce, which is both more
/// actionable and the same answer on every platform.
pub(crate) fn ensure_owned_root(root: &Path) -> Result<(), OwnershipError> {
    if let Ok(metadata) = std::fs::symlink_metadata(root) {
        return if metadata.file_type().is_symlink() || !metadata.is_dir() {
            Err(OwnershipError::NotOwned)
        } else {
            Ok(())
        };
    }
    std::fs::create_dir_all(root).map_err(|_| OwnershipError::Io)?;
    let metadata = std::fs::symlink_metadata(root).map_err(|_| OwnershipError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(OwnershipError::NotOwned);
    }
    Ok(())
}

/// Creates every directory from `root` down to `directory`, refusing to follow or create anything
/// that is not a plain directory.
///
/// Unlike `create_dir_all` this does not treat "it already exists" as success without looking:
/// an existing component is inspected, and one that is a link or a file stops the walk.
pub(crate) fn create_owned_directory(root: &Path, directory: &Path) -> Result<(), OwnershipError> {
    ensure_owned_root(root)?;
    let relative = relative_within(root, directory)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(OwnershipError::NotOwned);
            }
            Ok(_) => {}
            Err(_) => std::fs::create_dir(&current).map_err(|_| OwnershipError::Io)?,
        }
    }
    Ok(())
}

/// Confirms `path` exists beneath `root` and that nothing on the way to it is a link.
pub(crate) fn verify_owned(root: &Path, path: &Path) -> Result<(), OwnershipError> {
    ensure_owned_root(root)?;
    let relative = relative_within(root, path)?;
    let mut current = root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        current.push(component);
        let metadata = std::fs::symlink_metadata(&current).map_err(|_| OwnershipError::Io)?;
        if metadata.file_type().is_symlink() {
            return Err(OwnershipError::NotOwned);
        }
        // Every component but the last has to be a directory; the last may be either.
        if components.peek().is_some() && !metadata.is_dir() {
            return Err(OwnershipError::NotOwned);
        }
    }
    Ok(())
}

/// Removes a subtree, and only after confirming the application owns every step of the way to it.
///
/// A path that is already gone is a success. Cleanup runs on failure paths and at startup, where
/// "the thing I was about to remove is not there" is the outcome being asked for, not a problem.
pub(crate) fn remove_owned_tree(root: &Path, path: &Path) -> Result<(), OwnershipError> {
    match verify_owned(root, path) {
        Ok(()) => {}
        // Distinguished from `NotOwned`: nothing to remove is what the caller wanted.
        Err(OwnershipError::Io) if !path.exists() => return Ok(()),
        Err(error) => return Err(error),
    }
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(OwnershipError::Io),
    }
}

/// The part of `path` below `root`, refusing anything that is not actually below it.
///
/// `strip_prefix` compares components rather than resolving them, so a `..` inside the remainder
/// would still be stripped successfully and then walked. Refusing every non-normal component is
/// what makes the walk meaningful.
fn relative_within<'a>(root: &Path, path: &'a Path) -> Result<&'a Path, OwnershipError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| OwnershipError::OutsideRoot)?;
    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(OwnershipError::OutsideRoot);
    }
    Ok(relative)
}
