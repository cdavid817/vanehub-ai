// Assembled in bootstrap with the install flow in Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Publishing package content by rename, and the pointer write that follows it.

use crate::contexts::tooling::extension_platform::application::{
    SnapshotContentStore, SnapshotPointerRepository,
};
use crate::contexts::tooling::extension_platform::domain::{
    ContentPublication, ExtensionId, InstallationId, ManifestDigest, PackageHash, SnapshotId,
    SnapshotPointer, SnapshotPublicationError, SnapshotRecord, StagedRecovery,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension};
use semver::Version;
use std::path::Path;
use std::sync::Arc;

use super::roots::ExtensionRoots;

/// Content published by moving a staged directory into the content-addressed store.
///
/// A rename within one volume is the only publication step that is atomic on both platforms this
/// ships for. Copying would leave a window where a partly written snapshot is visible under its
/// final name, which is the one state a content-addressed store must never have.
pub(crate) struct FilesystemSnapshotContentStore {
    roots: ExtensionRoots,
}

impl FilesystemSnapshotContentStore {
    pub(crate) fn new(roots: ExtensionRoots) -> Self {
        Self { roots }
    }
}

impl SnapshotContentStore for FilesystemSnapshotContentStore {
    fn publish(&self, staged: &Path, hash: &PackageHash) -> Result<ContentPublication, String> {
        let destination = self
            .roots
            .package(hash)
            .map_err(|error| format!("package path is unusable: {}", error.code()))?;
        let Some(parent) = destination.parent() else {
            return Err("package path has no parent".to_string());
        };
        self.roots
            .create(parent)
            .map_err(|error| format!("package root is unavailable: {}", error.code()))?;

        // Checked first because rename onto an existing directory fails on both platforms, and
        // because content addressed by its own digest means what is there is what would be written.
        if destination.exists() {
            self.discard_staged(staged)?;
            return Ok(ContentPublication::AlreadyPresent);
        }

        match std::fs::rename(staged, &destination) {
            Ok(()) => Ok(ContentPublication::Published),
            // The window between the check and the rename belongs to a concurrent install of the
            // same package. It put identical bytes there, so this is still a success.
            Err(_) if destination.exists() => {
                self.discard_staged(staged)?;
                Ok(ContentPublication::AlreadyPresent)
            }
            Err(error) => Err(error.to_string()),
        }
    }

    fn discard_staged(&self, staged: &Path) -> Result<(), String> {
        self.roots
            .discard(staged)
            .map_err(|error| format!("staged content could not be removed: {}", error.code()))
    }
}

pub(crate) struct SqliteSnapshotPointerRepository {
    database: Arc<NativeDatabase>,
    installation: InstallationId,
}

impl SqliteSnapshotPointerRepository {
    /// `installation` is the id this repository writes pointers under. One per installed
    /// extension, supplied by the caller that created it.
    pub(crate) fn new(database: Arc<NativeDatabase>, installation: InstallationId) -> Self {
        Self {
            database,
            installation,
        }
    }

    fn connection(&self) -> Result<PooledSqlite, String> {
        self.database
            .connection()
            .map_err(|error| error.to_string())
    }
}

fn pointer_failure(reason: String) -> SnapshotPublicationError {
    SnapshotPublicationError::Pointer {
        reason,
        // Filled in by the service, which is the only thing that knows whether the staged content
        // went. Storage cannot answer for it.
        recovery: StagedRecovery::Clean,
    }
}

impl SnapshotPointerRepository for SqliteSnapshotPointerRepository {
    fn active(&self, extension: &ExtensionId) -> Result<Option<SnapshotPointer>, String> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT installation_id, extension_id, active_snapshot_id, previous_snapshot_id, \
                        revision, updated_at \
                 FROM extension_platform_installations WHERE extension_id = ?1",
                params![extension.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| error.to_string())?
            .map(read_pointer)
            .transpose()
    }

    fn point_at(
        &self,
        record: &SnapshotRecord,
        expected_revision: i64,
    ) -> Result<SnapshotPointer, SnapshotPublicationError> {
        let connection = self.connection().map_err(pointer_failure)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| pointer_failure(error.to_string()))?;

        let current: Option<(String, i64)> = transaction
            .query_row(
                "SELECT active_snapshot_id, revision FROM extension_platform_installations \
                 WHERE extension_id = ?1",
                params![record.extension.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|error| pointer_failure(error.to_string()))?;
        let (previous, current_revision) = match &current {
            Some((active, revision)) => (Some(active.clone()), *revision),
            None => (None, 0),
        };
        if current_revision != expected_revision {
            return Err(SnapshotPublicationError::StaleRevision {
                expected: expected_revision,
                actual: current_revision,
            });
        }

        // The snapshot row first: a pointer naming a snapshot nobody recorded would be an
        // installation that cannot describe itself. Both writes are in one transaction, so the
        // pair either lands or does not.
        transaction
            .execute(
                "INSERT INTO extension_platform_snapshots \
                     (snapshot_id, extension_id, version, package_hash, manifest_digest, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT(snapshot_id) DO NOTHING",
                params![
                    record.snapshot.as_str(),
                    record.extension.as_str(),
                    record.version.to_string(),
                    record.package_hash.as_str(),
                    record.manifest_digest.as_str(),
                    record.created_at,
                ],
            )
            .map_err(|error| pointer_failure(error.to_string()))?;

        let revision = current_revision + 1;
        transaction
            .execute(
                "INSERT INTO extension_platform_installations \
                     (installation_id, extension_id, active_snapshot_id, previous_snapshot_id, \
                      revision, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT(extension_id) DO UPDATE SET \
                     active_snapshot_id = excluded.active_snapshot_id, \
                     previous_snapshot_id = excluded.previous_snapshot_id, \
                     revision = excluded.revision, \
                     updated_at = excluded.updated_at",
                params![
                    self.installation.as_str(),
                    record.extension.as_str(),
                    record.snapshot.as_str(),
                    previous,
                    revision,
                    record.created_at,
                ],
            )
            .map_err(|error| pointer_failure(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| pointer_failure(error.to_string()))?;

        Ok(SnapshotPointer {
            installation: self.installation.clone(),
            extension: record.extension.clone(),
            active: record.snapshot.clone(),
            previous: previous
                .as_deref()
                .and_then(|value| SnapshotId::parse(value).ok()),
            revision,
            updated_at: record.created_at.clone(),
        })
    }
}

type PointerRow = (String, String, String, Option<String>, i64, String);

/// Rebuilds a pointer from a row, refusing one whose stored identifiers no longer parse.
///
/// A row that cannot be read is an error rather than an absence: "no installation" and "an
/// installation this build cannot describe" call for completely different handling, and treating
/// the second as the first would look like the extension was never installed.
fn read_pointer(row: PointerRow) -> Result<SnapshotPointer, String> {
    let (installation, extension, active, previous, revision, updated_at) = row;
    Ok(SnapshotPointer {
        installation: InstallationId::parse(&installation)
            .map_err(|error| error.code().to_string())?,
        extension: ExtensionId::parse(&extension).map_err(|error| error.code().to_string())?,
        active: SnapshotId::parse(&active).map_err(|error| error.code().to_string())?,
        previous: match previous {
            Some(value) => {
                Some(SnapshotId::parse(&value).map_err(|error| error.code().to_string())?)
            }
            None => None,
        },
        revision,
        updated_at,
    })
}

/// Reads a snapshot row back. Present for reconciliation and rollback, which need the record
/// rather than the pointer.
pub(crate) fn read_snapshot(
    database: &NativeDatabase,
    snapshot: &SnapshotId,
) -> Result<Option<SnapshotRecord>, String> {
    let connection = database.connection().map_err(|error| error.to_string())?;
    let row: Option<(String, String, String, String, String)> = connection
        .query_row(
            "SELECT extension_id, version, package_hash, manifest_digest, created_at \
             FROM extension_platform_snapshots WHERE snapshot_id = ?1",
            params![snapshot.as_str()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;

    let Some((extension, version, package_hash, manifest_digest, created_at)) = row else {
        return Ok(None);
    };
    Ok(Some(SnapshotRecord {
        snapshot: snapshot.clone(),
        extension: ExtensionId::parse(&extension).map_err(|error| error.code().to_string())?,
        version: Version::parse(&version).map_err(|error| error.to_string())?,
        package_hash: PackageHash::parse(&package_hash)
            .map_err(|error| error.code().to_string())?,
        manifest_digest: ManifestDigest::parse(&manifest_digest)
            .ok_or_else(|| "invalid_manifest_digest".to_string())?,
        created_at,
    }))
}
