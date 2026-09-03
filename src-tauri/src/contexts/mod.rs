//! Bounded contexts for native domain and application behavior.

pub(crate) mod agent_runtime;
pub(crate) mod artifacts;
pub(crate) mod browser_automation;
pub(crate) mod cli_delegation;
pub(crate) mod code_execution;
pub(crate) mod code_intelligence;
pub(crate) mod communications;
pub(crate) mod desktop;
pub(crate) mod execution_observability;
pub(crate) mod goals;
pub(crate) mod local_media;
pub(crate) mod operations;
pub(crate) mod permissions;
// Built bottom-up by `add-unified-personalization-governance`: the domain compiles and is tested
// before any repository, command, or runtime adapter consumes it. The allow comes off in the task
// group that wires `PersonalizationApi` into the composition root.
#[allow(dead_code, unused_imports)]
pub(crate) mod personalization;
pub(crate) mod retrieval;
pub(crate) mod sessions;
// Assessment remains dormant until its queue and repository are wired during the evolution rollout.
#[allow(dead_code, unused_imports)]
pub(crate) mod skill_evolution_assessment;
// Curator rollout starts with a dormant pure domain before native commands are enabled.
#[allow(dead_code, unused_imports)]
pub(crate) mod skill_evolution_curation;
// System activity is introduced behind the projector rollout and has no interactive-session API.
#[allow(dead_code, unused_imports)]
pub(crate) mod skill_evolution_system_activity;
// The evidence context is intentionally dormant until its bounded ingestion worker is enabled.
#[allow(dead_code, unused_imports)]
pub(crate) mod skill_evolution_evidence;
// Generation starts as a dormant governance domain until consented jobs are assembled.
#[allow(dead_code, unused_imports)]
pub(crate) mod skill_evolution_generation;
// Orchestration starts as a dormant domain until its durable scheduler is assembled.
#[allow(dead_code, unused_imports)]
pub(crate) mod skill_evolution_orchestration;
pub(crate) use skill_evolution_orchestration::infrastructure::apply_notification_schema as apply_notifications;
pub(crate) use skill_evolution_system_activity::infrastructure::apply_query_schema as apply_activity_query;
pub(crate) use skill_evolution_system_activity::infrastructure::apply_source_outbox_schema as apply_outboxes;
pub(crate) mod ssh_connections;
pub(crate) mod tooling;
pub(crate) mod web_research;
pub(crate) mod work_board;
pub(crate) mod workspaces;
