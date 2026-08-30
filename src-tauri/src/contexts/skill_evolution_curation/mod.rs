//! Human-governed, witness-bound Skill evolution curation.

pub(crate) mod api;
mod api_action_support;
mod api_actions;
mod api_models;
mod api_notifications;
mod api_queries;
mod api_system_policy;
pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;
mod rollback_candidate;
