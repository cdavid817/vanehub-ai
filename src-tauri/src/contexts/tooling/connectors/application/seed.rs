// The built-in drivers this seeds land with Task Group 10; see the domain's `builtin.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Seeding the host's own connector contributions, on every launch, without touching user state.
//!
//! Takes the subject and definition repositories and **nothing else**. There is no instance
//! repository, no binding repository, and no credential port in reach, so this cannot create an
//! instance, a binding, a credential, or an enablement. That matters more here than for Hooks: a
//! connector instance carries a credential handle, and a seed that created instances would be a
//! process running on every launch, next to secrets a person configured.

use super::{ConnectorDefinitionRepository, ConnectorSubjectRepository};
use crate::contexts::tooling::connectors::domain::{
    decide_connector_owner, BuiltinConnectorDescriptor, ConnectorDefinitionOutcome,
    ConnectorDefinitionRevision, ConnectorSeedOutcome, ConnectorSeedRejection, ConnectorSubject,
};

/// What one pass of the seed did, per descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ConnectorSeedReport {
    pub(crate) seeded: usize,
    pub(crate) already_seeded: usize,
    pub(crate) revisions_added: usize,
}

impl ConnectorSeedReport {
    fn record(&mut self, outcome: ConnectorSeedOutcome) {
        match outcome {
            ConnectorSeedOutcome::Seeded => self.seeded += 1,
            ConnectorSeedOutcome::AlreadySeeded => self.already_seeded += 1,
            ConnectorSeedOutcome::RevisionAdded => self.revisions_added += 1,
        }
    }

    /// Whether this pass changed anything. A repeated launch reports `false`.
    pub(crate) const fn changed_anything(&self) -> bool {
        self.seeded > 0 || self.revisions_added > 0
    }
}

/// Seeds every descriptor in `catalog`.
///
/// Stops at the first rejection. Descriptors already applied stay applied: each is its own subject
/// and its own immutable revision, so a partial pass leaves a consistent database and the next
/// launch resumes. Rolling the earlier ones back would mean deleting subjects that instances,
/// bindings, or credential handles may already reference — which `ON DELETE RESTRICT` refuses, and
/// rightly.
pub(crate) fn seed_builtin_connectors(
    subjects: &dyn ConnectorSubjectRepository,
    definitions: &dyn ConnectorDefinitionRepository,
    catalog: &[BuiltinConnectorDescriptor],
    at: &str,
) -> Result<ConnectorSeedReport, ConnectorSeedRejection> {
    let mut report = ConnectorSeedReport::default();
    for descriptor in catalog {
        report.record(seed_one(subjects, definitions, descriptor, at)?);
    }
    Ok(report)
}

fn seed_one(
    subjects: &dyn ConnectorSubjectRepository,
    definitions: &dyn ConnectorDefinitionRepository,
    descriptor: &BuiltinConnectorDescriptor,
    at: &str,
) -> Result<ConnectorSeedOutcome, ConnectorSeedRejection> {
    let storage = ConnectorSeedRejection::Storage;
    let owner = BuiltinConnectorDescriptor::owner();

    let stored = subjects
        .get(&descriptor.connector)
        .map_err(storage)?
        .map(|subject| subject.owner_extension);
    decide_connector_owner(&descriptor.connector, stored.as_ref(), &owner)?;

    // Idempotent, and leaves `first_seen_at` and the owner alone.
    subjects
        .ensure(&ConnectorSubject {
            connector: descriptor.connector.clone(),
            owner_extension: owner,
            first_seen_at: at.to_string(),
        })
        .map_err(storage)?;

    let outcome = definitions
        .record(&ConnectorDefinitionRevision {
            snapshot: descriptor.snapshot(),
            connector: descriptor.connector.clone(),
            digest: descriptor.digest.clone(),
            recorded_at: at.to_string(),
        })
        .map_err(storage)?;

    Ok(match outcome {
        ConnectorDefinitionOutcome::Recorded if stored.is_some() => {
            ConnectorSeedOutcome::RevisionAdded
        }
        ConnectorDefinitionOutcome::Recorded => ConnectorSeedOutcome::Seeded,
        ConnectorDefinitionOutcome::AlreadyRecorded => ConnectorSeedOutcome::AlreadySeeded,
        ConnectorDefinitionOutcome::Conflict(_) => {
            return Err(ConnectorSeedRejection::DefinitionConflict {
                connector: descriptor.connector.clone(),
            })
        }
    })
}
