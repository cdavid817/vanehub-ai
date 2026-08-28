mod chat_profile;
mod creation_context;
mod operation_adapter;
mod personalization_mode_schema;
mod review_decision_repository;
mod review_decision_schema;
mod review_repository;
mod rows;
mod runtime_support;
pub(crate) mod scheduled_tasks;
mod schema;
mod sqlite_repository;
mod transactions;
mod usage;
mod usage_accounting;
mod usage_accounting_projection;

pub(crate) use review_decision_repository::SqliteReviewDecisionRepository;
pub(crate) use review_decision_schema::{
    apply_review_decision_schema, apply_review_file_witness_schema,
    repair_missing_review_decision_schema, repair_missing_review_file_witness,
};
pub(crate) use review_repository::{
    apply_schema as apply_review_schema, SqliteReviewRepository, SystemReviewClock, UuidReviewIds,
};
pub(crate) use schema::{
    apply_configuration_schema, apply_loop_ownership_schema, apply_message_speaker_schema,
    apply_session_seat_schema, apply_stable_participant_schema,
};
pub(crate) use sqlite_repository::SqliteSessionsRepository;
pub(crate) use usage::apply_schema as apply_usage_schema;
pub(crate) use usage_accounting::apply_schema as apply_usage_accounting_schema;

#[cfg(test)]
mod tests;
pub(crate) use chat_profile::SqliteSessionChatProfileAdapter;
pub(crate) use creation_context::{SessionAgentEligibilityAdapter, SessionCreationContextAdapter};
pub(crate) use operation_adapter::SessionOperationAdapter;
pub(crate) use runtime_support::{
    AgentSessionRuntimeAdapter, SessionFileAdapter, SystemSessionClock,
    UnifiedSessionLoggingAdapter, UuidSessionIdentities,
};

pub(crate) use personalization_mode_schema::apply_schema as apply_personalization_mode_schema;
