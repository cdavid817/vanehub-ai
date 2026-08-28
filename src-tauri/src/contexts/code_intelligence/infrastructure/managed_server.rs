//! Installing and removing a language server VaneHub fetched.
//!
//! Everything about the download and the unpacking belongs to `managed_install`; what lives here
//! is where the result goes and when it is removed. Two rules shape it:
//!
//! - **A partial install must not look installed.** Extraction already discards its destination on
//!   any failure, so the finished directory is moved into place only once extraction returns. An
//!   interrupted install leaves nothing rather than a directory that fails later at launch with a
//!   missing launcher.
//! - **A failed reinstall must leave the working install alone.** The new tree is staged beside the
//!   live one and swapped in by rename, so the only moment the language has no install is between
//!   two metadata operations rather than for the length of a copy that can fail halfway.
//! - **Only VaneHub's own copy is ever removed.** A manual override names a directory the user
//!   created, and uninstalling a managed install must not touch it.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::contexts::code_intelligence::domain::models::Language;
use crate::contexts::code_intelligence::domain::registry::DistributionFormat;
use crate::contexts::tooling::api::{
    extract_tar_gz, extract_zip, ArtifactRequest, ManagedArtifactRetriever, ManagedInstallError,
};

/// Where a language's managed install lives.
///
/// One per language rather than one per version: this capability has no version selection, so a
/// second directory would be one nothing ever chooses between.
pub(crate) fn managed_install_directory(data_directory: &Path, language_id: &str) -> PathBuf {
    data_directory.join("lsp").join(language_id).join("install")
}

/// Whether a managed install exists for this language.
pub(crate) fn managed_install(data_directory: &Path, language_id: &str) -> Option<PathBuf> {
    let directory = managed_install_directory(data_directory, language_id);
    directory.is_dir().then_some(directory)
}

/// Fetches, unpacks, and places a language's declared distribution.
pub(crate) fn install_managed_server(
    retriever: &dyn ManagedArtifactRetriever,
    data_directory: &Path,
    language: Language,
    cancelled: &AtomicBool,
) -> Result<PathBuf, ManagedInstallError> {
    let Some(distribution) = language.distribution.as_ref() else {
        return Err(ManagedInstallError::Refused(
            "this language declares no published distribution".to_string(),
        ));
    };
    let destination = managed_install_directory(data_directory, language.id);
    let parent = destination
        .parent()
        .ok_or_else(|| ManagedInstallError::Refused("invalid install location".to_string()))?;
    std::fs::create_dir_all(parent).map_err(transfer)?;

    let artifact = retriever.retrieve(
        ArtifactRequest {
            url: distribution.url,
            policy: &distribution.policy,
            integrity: distribution.integrity,
            file_name: archive_file_name(distribution.format),
            // Unpacked by this process rather than executed, so the owner bit buys nothing.
            executable: false,
        },
        cancelled,
    )?;
    let extracted = match distribution.format {
        DistributionFormat::Zip => extract_zip(&artifact.path, distribution.extraction)?,
        DistributionFormat::TarGz => extract_tar_gz(&artifact.path, distribution.extraction)?,
    };

    let source = match distribution.root_inside_archive {
        Some(nested) => extracted.directory.path().join(nested),
        None => extracted.directory.path().to_path_buf(),
    };
    if !source.is_dir() {
        return Err(ManagedInstallError::Refused(
            "the archive did not contain the expected install root".to_string(),
        ));
    }

    // `TempDir` removes its path on drop, so the finished tree is copied out rather than renamed:
    // a rename would leave the handle pointing at nothing and, across filesystems, fail outright.
    // It is copied *beside* the live install, not over it -- see `swap_into_place`.
    let staged = parent.join(STAGED_DIRECTORY);
    if staged.exists() {
        std::fs::remove_dir_all(&staged).map_err(transfer)?;
    }
    copy_tree(&source, &staged).inspect_err(|_| {
        // The half-copied tree would otherwise be swapped in and read as installed.
        let _ = std::fs::remove_dir_all(&staged);
    })?;
    swap_into_place(&staged, &destination, parent)?;
    Ok(destination)
}

/// The directory a new install is built in before it replaces the live one.
///
/// A sibling rather than a temporary directory elsewhere, so the swap is a same-filesystem rename.
const STAGED_DIRECTORY: &str = "install.incoming";

/// The directory the outgoing install is parked in while the new one is renamed in.
const RETIRED_DIRECTORY: &str = "install.previous";

/// Replaces the live install with the staged one, keeping the live one until the swap succeeds.
///
/// Copying straight over the destination would mean deleting a working install and then spending
/// the length of a 50 MB copy in a state where a disk-full or permission error leaves the language
/// with nothing -- a reinstall that fails would be strictly worse than not pressing the button.
/// Here the only gap is between two renames, and a failed second rename puts the original back.
///
/// Replacing rather than merging: a previous install's leftover files under a new one is how a
/// stale launcher survives an upgrade and then gets picked by the glob.
fn swap_into_place(
    staged: &Path,
    destination: &Path,
    parent: &Path,
) -> Result<(), ManagedInstallError> {
    let retired = parent.join(RETIRED_DIRECTORY);
    if retired.exists() {
        std::fs::remove_dir_all(&retired).map_err(transfer)?;
    }
    let had_previous = destination.exists();
    if had_previous {
        // On Windows this is where a running server stops the operation, and it stops it before
        // anything has been destroyed.
        std::fs::rename(destination, &retired)
            .inspect_err(|_| {
                let _ = std::fs::remove_dir_all(staged);
            })
            .map_err(transfer)?;
    }
    if let Err(error) = std::fs::rename(staged, destination) {
        if had_previous {
            // Best effort, and the only path that can leave the language uninstalled: the restore
            // failing means the filesystem refused two renames in a row.
            let _ = std::fs::rename(&retired, destination);
        }
        let _ = std::fs::remove_dir_all(staged);
        return Err(transfer(error));
    }
    // The old install is gone from the live path already; failing to delete it now costs disk
    // rather than correctness, and the next install removes it.
    let _ = std::fs::remove_dir_all(&retired);
    Ok(())
}

/// Removes only the directory VaneHub created.
///
/// The caller stops the language's processes first. On Windows a directory a process still holds
/// open simply will not delete, so that ordering is a requirement rather than politeness.
pub(crate) fn uninstall_managed_server(
    data_directory: &Path,
    language_id: &str,
) -> Result<(), ManagedInstallError> {
    let directory = managed_install_directory(data_directory, language_id);
    if !directory.is_dir() {
        return Ok(());
    }
    std::fs::remove_dir_all(&directory).map_err(transfer)
}

/// The name the downloaded archive lands under. Never derived from the URL.
const fn archive_file_name(format: DistributionFormat) -> &'static str {
    match format {
        DistributionFormat::Zip => "server.zip",
        DistributionFormat::TarGz => "server.tar.gz",
    }
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), ManagedInstallError> {
    std::fs::create_dir_all(destination).map_err(transfer)?;
    for entry in std::fs::read_dir(source).map_err(transfer)? {
        let entry = entry.map_err(transfer)?;
        let target = destination.join(entry.file_name());
        // `file_type` rather than `metadata`: the extraction guard already refused links, so a
        // link here would mean something wrote into the destination behind our back.
        let file_type = entry.file_type().map_err(transfer)?;
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &target).map_err(transfer)?;
        } else {
            return Err(ManagedInstallError::Refused(
                "the extracted tree contains an entry that is neither a file nor a directory"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn transfer(error: std::io::Error) -> ManagedInstallError {
    ManagedInstallError::Transfer(crate::platform::logging::redact_text(&error.to_string()))
}

#[cfg(test)]
#[path = "managed_server_tests.rs"]
mod tests;
