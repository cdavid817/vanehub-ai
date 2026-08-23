// The built-in drivers these describe land with Task Group 10; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What the host itself contributes as a connector, and what seeding it may and may not do.
//!
//! The same three rules as lifecycle Hooks, for the same reasons, and one more that matters more
//! here than there.
//!
//! **Seeding creates identity, never user state.** A seed may create a subject and a definition
//! revision. It may not create an instance, a binding, a credential, an enablement, or a
//! configuration. `seed_builtin_connectors` takes the subject and definition repositories and
//! nothing else, so it could not create one if a caller wanted it to. The extra weight here is
//! that a connector instance carries a credential handle: a seed that created instances would be a
//! process running on every launch, next to secrets a person configured.
//!
//! **Ownership is checked, not assumed**, in both directions, with no `INSERT OR REPLACE`.
//!
//! **Upgrades are new revisions**, recorded beside the old one under a new built-in snapshot.
//!
//! Existing IM connectors, MCP servers, and GitHub readiness are **not** seeded, migrated, or
//! dual-written here. Their projections belong to the task groups that own the drivers; producing
//! descriptor rows for them now would create a second, empty account of state that already exists
//! elsewhere and is still authoritative there.

use super::{ConnectorDefinitionDigest, ConnectorGlobalId, ConnectorSnapshotRef, OwnerExtensionId};

/// The snapshot built-in definitions are recorded under.
///
/// Reserved, because a built-in comes from no extension package. The generation suffix is what
/// makes an upgrade a new revision rather than an edit.
pub(crate) const BUILTIN_CONNECTOR_SNAPSHOT: &str = "builtin-1";

/// The owner a built-in connector is filed under.
///
/// A real, reserved id rather than a null owner: "owned by the host" is a fact worth being able to
/// query, and a nullable owner column would make the ownership check partial.
pub(crate) const BUILTIN_CONNECTOR_OWNER: &str = "vanehub.core";

/// One connector the host contributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuiltinConnectorDescriptor {
    pub(crate) connector: ConnectorGlobalId,
    pub(crate) digest: ConnectorDefinitionDigest,
}

impl BuiltinConnectorDescriptor {
    pub(crate) fn snapshot(&self) -> ConnectorSnapshotRef {
        ConnectorSnapshotRef::parse(BUILTIN_CONNECTOR_SNAPSHOT).unwrap_or_else(|_| unreachable!())
    }

    pub(crate) fn owner() -> OwnerExtensionId {
        OwnerExtensionId::parse(BUILTIN_CONNECTOR_OWNER).unwrap_or_else(|_| unreachable!())
    }
}

/// What the host contributes in this build.
///
/// Empty today. GitHub readiness, the IM connectors, and the MCP projection are the built-in
/// drivers, and each is owned by a Task Group 10 task that also brings its driver, its
/// configuration schema, and its lifecycle. Seeding descriptors for them now would create rows
/// nothing reads and would pre-empt decisions those tasks have to make about legacy ids.
pub(crate) fn builtin_connector_catalog() -> Vec<BuiltinConnectorDescriptor> {
    Vec::new()
}

/// Why a seed could not proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConnectorSeedRejection {
    /// The subject id is already claimed by a different owner.
    OwnerConflict {
        connector: ConnectorGlobalId,
        stored: OwnerExtensionId,
        offered: OwnerExtensionId,
    },
    /// The same `(snapshot, subject)` is recorded with a different definition. Not an upgrade —
    /// an upgrade is a new snapshot.
    DefinitionConflict {
        connector: ConnectorGlobalId,
    },
    Storage(String),
}

impl ConnectorSeedRejection {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::OwnerConflict { .. } => "builtin_connector_owner_conflict",
            Self::DefinitionConflict { .. } => "builtin_connector_definition_conflict",
            Self::Storage(_) => "builtin_connector_seed_storage_failure",
        }
    }
}

pub(crate) fn all_connector_seed_rejections() -> Vec<ConnectorSeedRejection> {
    let placeholder = || ConnectorGlobalId::parse("placeholder").unwrap_or_else(|_| unreachable!());
    vec![
        ConnectorSeedRejection::OwnerConflict {
            connector: placeholder(),
            stored: OwnerExtensionId::parse("acme.other").unwrap_or_else(|_| unreachable!()),
            offered: BuiltinConnectorDescriptor::owner(),
        },
        ConnectorSeedRejection::DefinitionConflict {
            connector: placeholder(),
        },
        ConnectorSeedRejection::Storage(String::new()),
    ]
}

/// What seeding one descriptor did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectorSeedOutcome {
    Seeded,
    AlreadySeeded,
    /// The subject was there; this build's definition is a new revision beside the old one.
    RevisionAdded,
}

impl ConnectorSeedOutcome {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Seeded => "builtin_connector_seeded",
            Self::AlreadySeeded => "builtin_connector_already_seeded",
            Self::RevisionAdded => "builtin_connector_revision_added",
        }
    }
}

/// Whether an existing subject may be seeded under `offered`.
pub(crate) fn decide_connector_owner(
    connector: &ConnectorGlobalId,
    stored: Option<&OwnerExtensionId>,
    offered: &OwnerExtensionId,
) -> Result<(), ConnectorSeedRejection> {
    match stored {
        None => Ok(()),
        Some(stored) if stored == offered => Ok(()),
        Some(stored) => Err(ConnectorSeedRejection::OwnerConflict {
            connector: connector.clone(),
            stored: stored.clone(),
            offered: offered.clone(),
        }),
    }
}
