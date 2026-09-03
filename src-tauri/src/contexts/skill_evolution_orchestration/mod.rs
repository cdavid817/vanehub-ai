//! Durable, bounded coordination for the Skill-evolution pipeline.

pub(crate) mod api;
mod api_actions;
mod api_history_queries;
mod api_notifications;
mod api_queries;
pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;

#[cfg(test)]
mod api_tests;
