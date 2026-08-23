//! Ports the connector subdomain is driven through, and the one read that spans them.
//!
//! Task Group 3 lands the ports, their SQLite adapters, and the reconciliation that makes up for
//! the foreign key `snapshot_id` deliberately does not have. Connecting, executing, and durable
//! credential replacement land with the Connector Lifecycle task group.

mod ports;
mod reconcile;
#[cfg(test)]
mod reconcile_tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use ports::{
    ActiveConnectorSnapshotPort, ConnectorBindingRepository, ConnectorCredentialPort,
    ConnectorDefinitionRepository, ConnectorInstanceRepository, ConnectorSubjectRepository,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use reconcile::{reconcile_connector, reconcile_connectors, recorded_revisions};
