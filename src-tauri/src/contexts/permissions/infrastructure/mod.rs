//! Concrete adapters: SQLite repositories, schema/migration, clock, and id generation.

mod audit_repository;
mod clock;
mod default_template_adapter;
mod event_adapter;
mod grant_repository;
mod ids;
#[cfg(test)]
mod migration_equivalence_tests;
mod principal_repository;
pub(crate) mod schema;

pub(crate) use audit_repository::SqliteAuditRepository;
pub(crate) use clock::PermissionsSystemClock;
pub(crate) use default_template_adapter::DesktopDefaultTemplateAdapter;
pub(crate) use event_adapter::TauriPendingApprovalEventAdapter;
pub(crate) use grant_repository::SqliteGrantRepository;
pub(crate) use ids::PermissionsUuidIdGenerator;
pub(crate) use principal_repository::SqlitePrincipalRepository;
