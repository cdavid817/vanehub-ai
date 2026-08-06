//! Concrete adapters: SQLite repositories, schema/migration, clock, and id generation.

mod audit_repository;
mod claude_code_hook_adapter;
mod clock;
mod default_template_adapter;
mod event_adapter;
mod grant_repository;
mod hook_bridge_discovery;
mod hook_bridge_mapping;
mod hook_bridge_server;
mod hook_bridge_wait_registry;
mod ids;
#[cfg(test)]
mod migration_equivalence_tests;
mod principal_repository;
pub(crate) mod schema;

pub(crate) use audit_repository::SqliteAuditRepository;
pub(crate) use claude_code_hook_adapter::ClaudeCodeHookAdapter;
pub(crate) use clock::PermissionsSystemClock;
pub(crate) use default_template_adapter::DesktopDefaultTemplateAdapter;
pub(crate) use event_adapter::TauriPendingApprovalEventAdapter;
pub(crate) use grant_repository::SqliteGrantRepository;
pub(crate) use hook_bridge_server::start_hook_bridge_server;
pub(crate) use hook_bridge_wait_registry::HookWaitRegistry;
pub(crate) use ids::PermissionsUuidIdGenerator;
pub(crate) use principal_repository::SqlitePrincipalRepository;
