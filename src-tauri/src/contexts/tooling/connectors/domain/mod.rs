//! Connector subjects, versioned definitions, configured instances, and where they are bound.
//!
//! Storage-shaped on purpose: Task Group 3 lands what a connector *is* and what may be written
//! about one. Connecting, executing, and changing a credential land with the Connector Lifecycle
//! task group.

mod definition;
#[cfg(test)]
mod definition_tests;
mod identity;
#[cfg(test)]
mod identity_tests;
mod instance;
#[cfg(test)]
mod instance_tests;
mod reconciliation;
#[cfg(test)]
mod reconciliation_tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use definition::{
    decide_connector_definition, ConnectorDefinitionContentConflict, ConnectorDefinitionOutcome,
    ConnectorDefinitionRevision, ConnectorSubject,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use identity::{
    BindingId, ConnectorDefinitionDigest, ConnectorGlobalId, ConnectorIdentifierKind,
    ConnectorIdentityError, ConnectorSnapshotRef, ConnectorTarget, CredentialHandle, DisplayLabel,
    InstanceId, LabelKey, OwnerExtensionId, PublicConfiguration, TargetKind,
    ALL_CONNECTOR_IDENTIFIER_KINDS, ALL_TARGET_KINDS, GLOBAL_TARGET_KEY,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use instance::{
    all_connector_binding_errors, all_connector_instance_errors, ConnectorBinding,
    ConnectorBindingError, ConnectorInstance, ConnectorInstanceError, InstanceEdit,
    ABSENT_REVISION,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use reconciliation::{
    judge_connector, ActiveConnectorSnapshot, ConnectorFacts, ConnectorReadiness, ConnectorVerdict,
    ALL_CONNECTOR_READINESS,
};

/// Every stable failure code this subdomain can present to a caller.
#[cfg(test)]
pub(crate) fn registered_connector_failures() -> Vec<&'static str> {
    let mut codes: Vec<&'static str> = ALL_CONNECTOR_IDENTIFIER_KINDS
        .iter()
        .map(|kind| kind.code())
        .collect();
    codes.extend(
        all_connector_instance_errors()
            .iter()
            .map(ConnectorInstanceError::code)
            .collect::<Vec<_>>(),
    );
    codes.extend(
        all_connector_binding_errors()
            .iter()
            .map(ConnectorBindingError::code)
            .collect::<Vec<_>>(),
    );
    codes.push(ConnectorDefinitionOutcome::Recorded.code());
    codes.push(ConnectorDefinitionOutcome::AlreadyRecorded.code());
    codes.push("connector_definition_content_conflict");
    codes
}
