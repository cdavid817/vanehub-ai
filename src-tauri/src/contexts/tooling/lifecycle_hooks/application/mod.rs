//! Ports the Hook subdomain drives its storage through, and the one read that spans them.
//!
//! Task Group 3 lands the ports, their SQLite adapters, and the reconciliation that makes up for
//! the foreign key `snapshot_id` deliberately does not have; the services that orchestrate
//! dispatch land with Task Group 7.

mod ports;
mod reconcile;
#[cfg(test)]
mod reconcile_tests;
mod seed;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use ports::{
    ActiveExtensionSnapshotPort, HookBindingRepository, HookDefinitionRepository,
    HookExecutionRepository, HookSubjectRepository,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use reconcile::{reconcile_subject, reconcile_subjects, recorded_revisions};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use seed::{seed_builtin_hooks, HookSeedReport};
