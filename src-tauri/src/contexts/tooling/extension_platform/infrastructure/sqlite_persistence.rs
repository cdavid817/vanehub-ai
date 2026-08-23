// The install and activation flows that call these land with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! SQLite adapters for version claims and runtime generations.
//!
//! Both are written so the database enforces the rule and the adapter reports it, rather than the
//! adapter checking and the database trusting. A claim conflict is decided by reading the row that
//! holds the version; a pointer move is decided by a revision compared inside the same
//! transaction; and a generation that belongs to another installation is refused by a composite
//! foreign key that no amount of application care could substitute for.

use crate::contexts::tooling::extension_platform::application::{
    RuntimeGenerationRepository, VersionClaimRepository,
};
use crate::contexts::tooling::extension_platform::domain::{
    decide_claim, ActiveGeneration, ClaimAuthority, ClaimOutcome, ClaimProvenance, ExtensionId,
    ExtensionInstallWitness, InstallationId, PackageHash, RuntimeGenerationError,
    RuntimeGenerationId, RuntimeGenerationRecord, SnapshotId, VersionClaim,
};
use crate::platform::database::{begin_write_transaction, NativeDatabase, PooledSqlite};
use rusqlite::{params, OptionalExtension};
use semver::Version;
use std::sync::Arc;

pub(crate) struct SqliteVersionClaimRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteVersionClaimRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, String> {
        self.database
            .connection()
            .map_err(|error| error.to_string())
    }
}

impl VersionClaimRepository for SqliteVersionClaimRepository {
    fn held(&self, offered: &VersionClaim) -> Result<Option<VersionClaim>, String> {
        let connection = self.connection()?;
        let row = connection
            .query_row(
                "SELECT package_hash, provenance, first_claimed_at \
                 FROM extension_platform_version_claims \
                 WHERE publisher = ?1 AND extension_id = ?2 AND version = ?3",
                params![
                    offered.authority.as_str(),
                    offered.extension.as_str(),
                    offered.version.to_string()
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| error.to_string())?;

        let Some((package_hash, provenance, first_claimed_at)) = row else {
            return Ok(None);
        };
        Ok(Some(VersionClaim {
            authority: offered.authority.clone(),
            extension: offered.extension.clone(),
            version: offered.version.clone(),
            package_hash: PackageHash::parse(&package_hash)
                .map_err(|error| error.code().to_string())?,
            provenance: ClaimProvenance::parse(&provenance)
                .ok_or_else(|| "invalid_claim_provenance".to_string())?,
            first_claimed_at,
        }))
    }

    fn claim(&self, offered: &VersionClaim, observed_at: &str) -> Result<ClaimOutcome, String> {
        let connection = self.connection()?;
        let transaction =
            begin_write_transaction(&connection).map_err(|error| error.to_string())?;

        // Read inside the transaction, so two claims racing cannot both see "unheld".
        let held: Option<(String, String, String)> = transaction
            .query_row(
                "SELECT package_hash, provenance, first_claimed_at \
                 FROM extension_platform_version_claims \
                 WHERE publisher = ?1 AND extension_id = ?2 AND version = ?3",
                params![
                    offered.authority.as_str(),
                    offered.extension.as_str(),
                    offered.version.to_string()
                ],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(|error| error.to_string())?;

        let held_claim = match held {
            Some((package_hash, provenance, first_claimed_at)) => Some(VersionClaim {
                authority: offered.authority.clone(),
                extension: offered.extension.clone(),
                version: offered.version.clone(),
                package_hash: PackageHash::parse(&package_hash)
                    .map_err(|error| error.code().to_string())?,
                provenance: ClaimProvenance::parse(&provenance)
                    .ok_or_else(|| "invalid_claim_provenance".to_string())?,
                first_claimed_at,
            }),
            None => None,
        };

        let outcome = decide_claim(offered, held_claim.as_ref());
        match &outcome {
            ClaimOutcome::Bound => {
                transaction
                    .execute(
                        "INSERT INTO extension_platform_version_claims \
                             (publisher, extension_id, version, package_hash, provenance, \
                              first_claimed_at) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            offered.authority.as_str(),
                            offered.extension.as_str(),
                            offered.version.to_string(),
                            offered.package_hash.as_str(),
                            offered.provenance.as_str(),
                            offered.first_claimed_at,
                        ],
                    )
                    .map_err(|error| error.to_string())?;
            }
            ClaimOutcome::AlreadyBound => {}
            // Nothing is written, deliberately, and it is not recorded as a conflict either. A
            // claim nobody was entitled to make did not conflict with the incumbent -- filing it
            // beside the genuine claims would tell an operator that two publishers disagree about
            // a version, when what happened is that one of them had no business naming it.
            ClaimOutcome::NamespaceMismatch(_) => {}
            // The refused hash is kept. A conflict that leaves no trace is a finding nobody can
            // act on, and "this version was claimed twice with different bytes" is the finding.
            ClaimOutcome::Conflict(conflict) => {
                transaction
                    .execute(
                        "INSERT INTO extension_platform_version_claim_conflicts \
                             (publisher, extension_id, version, bound_package_hash, \
                              offered_package_hash, offered_provenance, observed_at) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            offered.authority.as_str(),
                            offered.extension.as_str(),
                            offered.version.to_string(),
                            conflict.bound_hash.as_str(),
                            conflict.offered_hash.as_str(),
                            offered.provenance.as_str(),
                            observed_at,
                        ],
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(outcome)
    }

    fn conflicts(&self, extension: &ExtensionId) -> Result<Vec<String>, String> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT offered_package_hash FROM extension_platform_version_claim_conflicts \
                 WHERE extension_id = ?1 ORDER BY id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![extension.as_str()], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(rows)
    }
}

pub(crate) struct SqliteRuntimeGenerationRepository {
    database: Arc<NativeDatabase>,
}

impl SqliteRuntimeGenerationRepository {
    pub(crate) fn new(database: Arc<NativeDatabase>) -> Self {
        Self { database }
    }

    fn connection(&self) -> Result<PooledSqlite, RuntimeGenerationError> {
        self.database
            .connection()
            .map_err(|error| RuntimeGenerationError::Storage(error.to_string()))
    }
}

/// A foreign-key failure is the database saying the generation does not belong to this
/// installation, which is a domain answer rather than a storage failure.
fn generation_error(error: rusqlite::Error) -> RuntimeGenerationError {
    let text = error.to_string();
    if text.contains("FOREIGN KEY") {
        RuntimeGenerationError::UnknownGeneration
    } else {
        RuntimeGenerationError::Storage(text)
    }
}

impl RuntimeGenerationRepository for SqliteRuntimeGenerationRepository {
    fn record(&self, generation: &RuntimeGenerationRecord) -> Result<(), RuntimeGenerationError> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO extension_platform_runtime_generations \
                     (generation_id, installation_id, snapshot_id, started_at) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(generation_id) DO NOTHING",
                params![
                    generation.generation.as_str(),
                    generation.installation.as_str(),
                    generation.snapshot.as_str(),
                    generation.started_at,
                ],
            )
            .map_err(|error| {
                let text = error.to_string();
                if text.contains("FOREIGN KEY") {
                    RuntimeGenerationError::UnknownInstallation
                } else {
                    RuntimeGenerationError::Storage(text)
                }
            })?;
        Ok(())
    }

    fn active(
        &self,
        installation: &InstallationId,
    ) -> Result<Option<ActiveGeneration>, RuntimeGenerationError> {
        let connection = self.connection()?;
        let row = connection
            .query_row(
                "SELECT generation_id, revision, updated_at \
                 FROM extension_platform_active_runtime_generations WHERE installation_id = ?1",
                params![installation.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| RuntimeGenerationError::Storage(error.to_string()))?;

        let Some((generation_id, revision, updated_at)) = row else {
            return Ok(None);
        };
        Ok(Some(ActiveGeneration {
            installation: installation.clone(),
            generation: RuntimeGenerationId::parse(&generation_id)
                .map_err(|error| RuntimeGenerationError::Storage(error.code().to_string()))?,
            revision,
            updated_at,
        }))
    }

    fn activate(
        &self,
        installation: &InstallationId,
        generation: &RuntimeGenerationId,
        expected_revision: i64,
        updated_at: &str,
    ) -> Result<ActiveGeneration, RuntimeGenerationError> {
        let connection = self.connection()?;
        let transaction = begin_write_transaction(&connection)
            .map_err(|error| RuntimeGenerationError::Storage(error.to_string()))?;

        let current_revision: i64 = transaction
            .query_row(
                "SELECT revision FROM extension_platform_active_runtime_generations \
                 WHERE installation_id = ?1",
                params![installation.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| RuntimeGenerationError::Storage(error.to_string()))?
            .unwrap_or(0);
        if current_revision != expected_revision {
            return Err(RuntimeGenerationError::StaleRevision {
                expected: expected_revision,
                actual: current_revision,
            });
        }

        let revision = current_revision + 1;
        transaction
            .execute(
                "INSERT INTO extension_platform_active_runtime_generations \
                     (installation_id, generation_id, revision, updated_at) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(installation_id) DO UPDATE SET \
                     generation_id = excluded.generation_id, \
                     revision = excluded.revision, \
                     updated_at = excluded.updated_at",
                params![
                    installation.as_str(),
                    generation.as_str(),
                    revision,
                    updated_at
                ],
            )
            .map_err(generation_error)?;
        transaction
            .commit()
            .map_err(|error| RuntimeGenerationError::Storage(error.to_string()))?;

        Ok(ActiveGeneration {
            installation: installation.clone(),
            generation: generation.clone(),
            revision,
            updated_at: updated_at.to_string(),
        })
    }
}

/// Records one snapshot's declared dependencies and contributions.
///
/// Written once, with the snapshot, and never edited: an installation has to be describable when
/// the package is gone from disk and when the reading code has changed.
/// One contribution a snapshot declares.
///
/// `declared_digest` is what makes drift detectable at all. Without a second, independently
/// written copy of "what this contribution is", the consuming subdomain's own record would be the
/// only copy -- and a single copy cannot disagree with itself, so `drifted` would be a state the
/// code could name and never reach.
///
/// Optional because not every contribution kind has one yet; a contribution with no declared
/// digest reads downstream as "nothing to dispatch", which is the conservative answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordedContribution {
    pub(crate) global_id: String,
    pub(crate) kind: String,
    pub(crate) local_id: String,
    pub(crate) declared_digest: Option<String>,
}

pub(crate) fn record_snapshot_detail(
    database: &NativeDatabase,
    snapshot: &SnapshotId,
    dependencies: &[(String, String, String, bool)],
    contributions: &[RecordedContribution],
) -> Result<(), String> {
    let connection = database.connection().map_err(|error| error.to_string())?;
    let transaction = begin_write_transaction(&connection).map_err(|error| error.to_string())?;

    for (kind, id, requirement, optional) in dependencies {
        transaction
            .execute(
                "INSERT INTO extension_platform_snapshot_dependencies \
                     (snapshot_id, dependency_kind, dependency_id, version_requirement, optional) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(snapshot_id, dependency_kind, dependency_id) DO NOTHING",
                params![
                    snapshot.as_str(),
                    kind,
                    id,
                    requirement,
                    i64::from(*optional)
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    for contribution in contributions {
        transaction
            .execute(
                "INSERT INTO extension_platform_snapshot_contributions \
                     (snapshot_id, global_id, kind, local_id, contribution_digest) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(snapshot_id, global_id) DO NOTHING",
                params![
                    snapshot.as_str(),
                    contribution.global_id,
                    contribution.kind,
                    contribution.local_id,
                    contribution.declared_digest,
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(())
}

/// Records what a preview was bound to.
///
/// Takes the witness rather than its fields: eleven positional arguments is a call two of whose
/// arguments will eventually be swapped, and every value here is already inside the witness.
///
/// `witness_id` is the key and `(operation_id, witness_digest)` is unique, because the digest
/// covers the state a confirmation is bound to and deliberately not the operation.
pub(crate) fn record_operation_witness(
    database: &NativeDatabase,
    witness_id: &str,
    operation_id: &str,
    witness: &ExtensionInstallWitness,
    issued_at: &str,
) -> Result<(), String> {
    let subject = witness.subject();
    let connection = database.connection().map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO extension_platform_operation_witnesses \
                 (witness_id, operation_id, witness_digest, extension_id, version, package_hash, \
                  manifest_digest, signature_state, trust_profile, issued_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
             ON CONFLICT(operation_id, witness_digest) DO NOTHING",
            params![
                witness_id,
                operation_id,
                witness.digest(),
                subject.extension.as_str(),
                subject.version.to_string(),
                subject.package_hash.as_str(),
                subject.manifest_digest.as_str(),
                subject.signature.state,
                subject.trust_profile.as_str(),
                issued_at,
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Records the bytes a package arrived as. Idempotent: the same digest is the same package.
pub(crate) fn record_package(
    database: &NativeDatabase,
    package_hash: &PackageHash,
    byte_length: u64,
    signature_state: &str,
    key_fingerprint: Option<&str>,
    first_seen_at: &str,
) -> Result<(), String> {
    let connection = database.connection().map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT INTO extension_platform_packages \
                 (package_hash, byte_length, signature_state, publisher_key_fingerprint, \
                  first_seen_at) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(package_hash) DO NOTHING",
            params![
                package_hash.as_str(),
                byte_length as i64,
                signature_state,
                key_fingerprint,
                first_seen_at
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// A claim, from an authority this installation established rather than one a package asserted.
pub(crate) fn claim_for(
    authority: &ClaimAuthority,
    extension: &ExtensionId,
    version: &Version,
    package_hash: &PackageHash,
    provenance: ClaimProvenance,
    at: &str,
) -> VersionClaim {
    VersionClaim {
        authority: authority.clone(),
        extension: extension.clone(),
        version: version.clone(),
        package_hash: package_hash.clone(),
        provenance,
        first_claimed_at: at.to_string(),
    }
}
