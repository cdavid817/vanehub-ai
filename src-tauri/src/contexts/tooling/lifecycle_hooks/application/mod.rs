//! Ports the Hook subdomain drives its storage through, and the one read that spans them.
//!
//! Task Group 3 lands the ports, their SQLite adapters, and the reconciliation that makes up for
//! the foreign key `snapshot_id` deliberately does not have; the services that orchestrate
//! dispatch land with Task Group 7.

mod ports;
mod reconcile;
#[cfg(test)]
mod reconcile_tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use ports::{
    HookBindingRepository, HookDefinitionRepository, HookExecutionRepository,
    HookSubjectRepository, SnapshotProjectionPort,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use reconcile::{reconcile_subject, reconcile_subjects};
