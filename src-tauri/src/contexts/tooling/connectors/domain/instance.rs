// See `identity.rs` for why this lands ahead of its production caller.
#![cfg_attr(not(test), allow(dead_code))]

//! A configured connector, and where it is bound.
//!
//! ## Desired state, not live state
//!
//! An instance records `desired_enabled` and nothing about whether it is connected right now.
//! `connecting` and `connected` are properties of a socket that outlive nothing: writing them down
//! means every crash leaves a row claiming a connection that does not exist, and every reader has
//! to decide whether to believe it. Live state belongs to whatever holds the connection; storage
//! holds what the user asked for.
//!
//! ## What a missing definition does not do
//!
//! When no active snapshot declares a connector, its instances, bindings, and credential handles
//! stay exactly where they are. Nothing is deleted, nothing is rebound. The only consequence is
//! that new connect and execute attempts are refused. Deleting on absence would mean an extension
//! that failed to activate for thirty seconds during an upgrade cost the user every credential
//! they had configured for it.

use super::{
    BindingId, ConnectorGlobalId, ConnectorTarget, CredentialHandle, DisplayLabel, InstanceId,
    LabelKey, PublicConfiguration,
};

/// One configured connector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorInstance {
    /// Identity. Not the label: renaming keeps every binding and the credential.
    pub(crate) instance: InstanceId,
    /// The subject, never a versioned definition.
    pub(crate) connector: ConnectorGlobalId,
    pub(crate) display_label: DisplayLabel,
    pub(crate) desired_enabled: bool,
    pub(crate) configuration: PublicConfiguration,
    /// Names an entry in the OS credential store, or nothing yet. Never a secret.
    pub(crate) credential: Option<CredentialHandle>,
    pub(crate) revision: i64,
    pub(crate) updated_at: String,
}

impl ConnectorInstance {
    /// The normalised form uniqueness is decided on.
    ///
    /// Derived rather than stored alongside as an independent field, so the two cannot disagree.
    pub(crate) fn label_key(&self) -> LabelKey {
        self.display_label.key()
    }
}

/// Where an instance is in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorBinding {
    pub(crate) binding: BindingId,
    pub(crate) instance: InstanceId,
    pub(crate) target: ConnectorTarget,
    pub(crate) enabled: bool,
    pub(crate) revision: i64,
    pub(crate) updated_at: String,
}

/// Why an instance could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConnectorInstanceError {
    /// Someone else changed the instance since the caller read it.
    StaleRevision {
        expected: i64,
        actual: i64,
    },
    /// No subject with this connector id.
    UnknownSubject,
    /// Another instance of the same connector already uses a label that normalises to this one.
    ///
    /// Refused rather than disambiguated: two instances a person cannot tell apart in a list is
    /// how a credential gets attached to the wrong one.
    DuplicateLabel {
        existing: InstanceId,
    },
    Storage(String),
}

impl ConnectorInstanceError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::StaleRevision { .. } => "connector_instance_stale_revision",
            Self::UnknownSubject => "unknown_connector_subject",
            Self::DuplicateLabel { .. } => "duplicate_connector_label",
            Self::Storage(_) => "connector_instance_storage_failure",
        }
    }
}

pub(crate) fn all_connector_instance_errors() -> Vec<ConnectorInstanceError> {
    vec![
        ConnectorInstanceError::StaleRevision {
            expected: 0,
            actual: 0,
        },
        ConnectorInstanceError::UnknownSubject,
        ConnectorInstanceError::DuplicateLabel {
            existing: InstanceId::parse("placeholder").unwrap_or_else(|_| unreachable!()),
        },
        ConnectorInstanceError::Storage(String::new()),
    ]
}

/// Why a binding could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ConnectorBindingError {
    StaleRevision {
        expected: i64,
        actual: i64,
    },
    /// No instance with this id.
    UnknownInstance,
    Storage(String),
}

impl ConnectorBindingError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::StaleRevision { .. } => "connector_binding_stale_revision",
            Self::UnknownInstance => "unknown_connector_instance",
            Self::Storage(_) => "connector_binding_storage_failure",
        }
    }
}

pub(crate) fn all_connector_binding_errors() -> Vec<ConnectorBindingError> {
    vec![
        ConnectorBindingError::StaleRevision {
            expected: 0,
            actual: 0,
        },
        ConnectorBindingError::UnknownInstance,
        ConnectorBindingError::Storage(String::new()),
    ]
}

/// One requested change to an instance.
///
/// A named parameter object rather than eight positional arguments: several of those are adjacent
/// and similarly typed, which is a call whose arguments get swapped and still compiles.
///
/// There is deliberately no credential field. Attaching one is `attach_credential`, so an ordinary
/// settings edit cannot clear a credential by omitting it -- the failure mode of a single "save
/// everything" call is a form that round-trips a `None` and silently detaches a secret the user
/// still needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstanceEdit<'a> {
    pub(crate) instance: &'a InstanceId,
    pub(crate) connector: &'a ConnectorGlobalId,
    pub(crate) label: &'a DisplayLabel,
    pub(crate) desired_enabled: bool,
    pub(crate) configuration: &'a PublicConfiguration,
    /// `ABSENT_REVISION` for a create.
    pub(crate) expected_revision: i64,
    pub(crate) at: &'a str,
}

/// The revision an instance or binding that does not exist yet is treated as having.
///
/// A caller that read "nothing there" and then writes passes this, so creating and updating go
/// through the same compare-and-swap and a create cannot silently overwrite something that
/// appeared in between.
pub(crate) const ABSENT_REVISION: i64 = 0;
