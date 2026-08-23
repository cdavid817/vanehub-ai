// The preview and confirm operations that produce these land with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What may be persisted as operation evidence, and what may be pruned.
//!
//! ## Why a separate admitted type
//!
//! `ExtensionInstallWitness` is derived from a manifest, and a manifest is written by whoever
//! built the package. Its collections are as long as the author made them and its strings are as
//! long as the author made them. A repository that took one directly would be a repository whose
//! row size is chosen by an extension author — an unbounded write reachable by anyone who can get
//! a package as far as a preview.
//!
//! So the repository takes a `PersistableOperationWitness`, which has one constructor and cannot
//! be built without passing every bound. The check is not in the repository, because a check in a
//! repository is one a second repository method will not have.
//!
//! ## What a witness may contain
//!
//! Only the typed facts a reviewer was shown. There is no field for a JSON blob, a configuration
//! map, an environment, a raw payload, captured stdout or stderr, a secret, or a host path — and
//! the guard is that no such field exists, not that something redacts one. Adding one breaks the
//! exhaustive destructuring in `witness_bounds_tests`, which is the point.
//!
//! ## What retention may remove
//!
//! Never evidence for a package an installation is running, could roll back to, or has
//! quarantined; never evidence for an operation that has not finished; never a row written under a
//! schema version this build does not know. The first three are the states in which the evidence
//! is still the answer to a live question. The fourth is the one people get wrong: a newer build's
//! row is not corrupt, it is simply not this build's to interpret, and deleting what you cannot
//! read is how a downgrade destroys the record the upgrade was keeping.

use super::install_witness::canonical_witness_bytes;
use super::{ExtensionInstallWitness, PackageHash};

/// The schema version this build writes and understands.
///
/// Stored per row. A row carrying a higher one was written by a newer build; this build may read
/// around it but must not prune it.
pub(crate) const WITNESS_SCHEMA_VERSION: i64 = 1;

/// Every bound a witness must satisfy before it may be stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WitnessLimits {
    /// The canonical encoding's total length. The one bound that cannot be evaded by splitting a
    /// large value across several small fields.
    pub(crate) max_total_bytes: usize,
    pub(crate) max_dependencies: usize,
    pub(crate) max_capability_lines: usize,
    pub(crate) max_contributions: usize,
    /// Longest any single stored string may be.
    pub(crate) max_field_characters: usize,
}

/// Sized for a large but real extension, not for the largest a format could express.
pub(crate) const DEFAULT_WITNESS_LIMITS: WitnessLimits = WitnessLimits {
    max_total_bytes: 64 * 1024,
    max_dependencies: 128,
    max_capability_lines: 256,
    max_contributions: 512,
    max_field_characters: 512,
};

/// Why a witness may not be stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WitnessRejection {
    /// The canonical encoding exceeds `max_total_bytes`.
    TooLarge {
        bytes: usize,
        limit: usize,
    },
    TooManyDependencies {
        count: usize,
        limit: usize,
    },
    TooManyCapabilities {
        count: usize,
        limit: usize,
    },
    TooManyContributions {
        count: usize,
        limit: usize,
    },
    /// One stored string is longer than `max_field_characters`.
    FieldTooLong {
        field: &'static str,
        limit: usize,
    },
    /// A stored string carries a NUL or another control character.
    ///
    /// Refused rather than stripped: a value that has to be altered to be storable is a value
    /// whose stored form no longer matches the one a reviewer approved, and the witness exists to
    /// be compared against what was approved.
    ControlCharacter {
        field: &'static str,
    },
    /// The witness no longer agrees with its own digest.
    NotSelfConsistent,
    /// A schema version this build does not write.
    UnsupportedSchemaVersion {
        version: i64,
    },
}

impl WitnessRejection {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::TooLarge { .. } => "witness_too_large",
            Self::TooManyDependencies { .. } => "witness_too_many_dependencies",
            Self::TooManyCapabilities { .. } => "witness_too_many_capabilities",
            Self::TooManyContributions { .. } => "witness_too_many_contributions",
            Self::FieldTooLong { .. } => "witness_field_too_long",
            Self::ControlCharacter { .. } => "witness_control_character",
            Self::NotSelfConsistent => "witness_not_self_consistent",
            Self::UnsupportedSchemaVersion { .. } => "witness_unsupported_schema_version",
        }
    }
}

pub(crate) fn all_witness_rejections() -> Vec<WitnessRejection> {
    vec![
        WitnessRejection::TooLarge { bytes: 0, limit: 0 },
        WitnessRejection::TooManyDependencies { count: 0, limit: 0 },
        WitnessRejection::TooManyCapabilities { count: 0, limit: 0 },
        WitnessRejection::TooManyContributions { count: 0, limit: 0 },
        WitnessRejection::FieldTooLong {
            field: "",
            limit: 0,
        },
        WitnessRejection::ControlCharacter { field: "" },
        WitnessRejection::NotSelfConsistent,
        WitnessRejection::UnsupportedSchemaVersion { version: 0 },
    ]
}

/// A witness that has passed every bound. The only thing a repository accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersistableOperationWitness {
    witness: ExtensionInstallWitness,
    schema_version: i64,
}

impl PersistableOperationWitness {
    /// Admits a witness for storage, or says why not.
    ///
    /// The only constructor, and it is not `pub(crate) fn new`: the name is the reminder that
    /// producing one is a decision with a failure mode.
    pub(crate) fn admit(
        witness: ExtensionInstallWitness,
        limits: WitnessLimits,
    ) -> Result<Self, WitnessRejection> {
        // First, because everything after it reads fields the digest is supposed to cover. A
        // witness whose subject was edited after issue is not evidence of anything.
        if !witness.is_self_consistent() {
            return Err(WitnessRejection::NotSelfConsistent);
        }

        let subject = witness.subject();
        if subject.dependencies.len() > limits.max_dependencies {
            return Err(WitnessRejection::TooManyDependencies {
                count: subject.dependencies.len(),
                limit: limits.max_dependencies,
            });
        }
        let capability_count = subject.capabilities.added.len()
            + subject.capabilities.removed.len()
            + subject.capabilities.unchanged.len();
        if capability_count > limits.max_capability_lines {
            return Err(WitnessRejection::TooManyCapabilities {
                count: capability_count,
                limit: limits.max_capability_lines,
            });
        }
        if subject.contributions.len() > limits.max_contributions {
            return Err(WitnessRejection::TooManyContributions {
                count: subject.contributions.len(),
                limit: limits.max_contributions,
            });
        }

        for (field, value) in stored_strings(&witness) {
            if value.chars().count() > limits.max_field_characters {
                return Err(WitnessRejection::FieldTooLong {
                    field,
                    limit: limits.max_field_characters,
                });
            }
            if value.chars().any(char::is_control) {
                return Err(WitnessRejection::ControlCharacter { field });
            }
        }

        // Last, because it is the bound that cannot be evaded: a thousand fields each just under
        // the per-field limit still fails here.
        let bytes = canonical_witness_bytes(subject).len();
        if bytes > limits.max_total_bytes {
            return Err(WitnessRejection::TooLarge {
                bytes,
                limit: limits.max_total_bytes,
            });
        }

        Ok(Self {
            witness,
            schema_version: WITNESS_SCHEMA_VERSION,
        })
    }

    pub(crate) fn witness(&self) -> &ExtensionInstallWitness {
        &self.witness
    }

    pub(crate) const fn schema_version(&self) -> i64 {
        self.schema_version
    }
}

/// Every string this witness would put in a row, paired with the field it came from.
fn stored_strings(witness: &ExtensionInstallWitness) -> Vec<(&'static str, String)> {
    let subject = witness.subject();
    let mut values = vec![
        ("extension", subject.extension.as_str().to_string()),
        ("version", subject.version.to_string()),
        ("package_hash", subject.package_hash.as_str().to_string()),
        (
            "manifest_digest",
            subject.manifest_digest.as_str().to_string(),
        ),
        ("signature_state", subject.signature.state.to_string()),
        ("trust_profile", subject.trust_profile.as_str().to_string()),
    ];
    values.extend(
        subject
            .dependencies
            .iter()
            .map(|dependency| ("dependency_id", dependency.id.clone())),
    );
    for line in subject
        .capabilities
        .added
        .iter()
        .chain(&subject.capabilities.removed)
        .chain(&subject.capabilities.unchanged)
    {
        values.push(("capability", line.clone()));
    }
    values.extend(
        subject
            .contributions
            .iter()
            .map(|id| ("contribution", id.as_str().to_string())),
    );
    values
}

/// How many witnesses to keep per extension.
///
/// A window of zero would let retention empty an extension's evidence entirely, which is the one
/// thing the protection rules below exist to prevent from happening by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WitnessRetention(usize);

pub(crate) const DEFAULT_WITNESS_RETENTION: usize = 50;

impl WitnessRetention {
    pub(crate) const fn new(keep: usize) -> Option<Self> {
        if keep == 0 {
            return None;
        }
        Some(Self(keep))
    }

    pub(crate) const fn keep(self) -> usize {
        self.0
    }
}

impl Default for WitnessRetention {
    fn default() -> Self {
        Self(DEFAULT_WITNESS_RETENTION)
    }
}

/// What retention must not touch, supplied by the caller.
///
/// The repository cannot work these out: operations live in an in-memory registry with no table,
/// and which package is active, rolled back to, or quarantined is the installation flow's
/// knowledge. A repository that guessed would be a repository that deletes evidence on the one
/// launch its guess is wrong.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WitnessProtection {
    /// Operations that have not finished. Their evidence is not history yet.
    pub(crate) unfinished_operations: Vec<String>,
    /// Packages an installation runs, can roll back to, or has quarantined.
    pub(crate) protected_packages: Vec<PackageHash>,
}

impl WitnessProtection {
    pub(crate) fn protects_operation(&self, operation_id: &str) -> bool {
        self.unfinished_operations
            .iter()
            .any(|held| held == operation_id)
    }

    pub(crate) fn protects_package(&self, package_hash: &str) -> bool {
        self.protected_packages
            .iter()
            .any(|held| held.as_str() == package_hash)
    }
}

/// Whether one stored row may be pruned.
///
/// Pure, so the rule is one place and the repository only has to supply the row.
pub(crate) fn is_prunable(
    protection: &WitnessProtection,
    operation_id: &str,
    package_hash: &str,
    schema_version: i64,
    inside_window: bool,
) -> bool {
    !inside_window
        && schema_version <= WITNESS_SCHEMA_VERSION
        && !protection.protects_operation(operation_id)
        && !protection.protects_package(package_hash)
}
