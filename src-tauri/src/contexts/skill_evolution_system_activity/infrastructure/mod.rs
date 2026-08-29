mod dashboard_repository;
mod notification_read_repository;
mod notification_repository;
mod preferences_repository;
mod projection_batch_repository;
mod projection_repository;
mod query_builder;
mod query_repository;
mod retention_repository;
mod safe_identity_repository;
mod schema;
mod source_adapters;
mod target_projection_adapter;
mod target_receipt_repository;
mod timeline_repository;
mod unread_repository;

pub(crate) use dashboard_repository::*;
pub(crate) use notification_repository::*;
pub(crate) use preferences_repository::*;
pub(crate) use projection_batch_repository::*;
pub(crate) use projection_repository::*;
pub(crate) use query_repository::*;
pub(crate) use retention_repository::*;
pub(crate) use schema::{apply_query_schema, apply_schema, apply_source_outbox_schema};
pub(crate) use source_adapters::*;
pub(crate) use target_projection_adapter::*;
pub(crate) use timeline_repository::*;
pub(crate) use unread_repository::*;

#[cfg(test)]
mod dashboard_repository_tests;
#[cfg(test)]
mod notification_repository_tests;
#[cfg(test)]
mod preferences_repository_tests;
#[cfg(test)]
mod projection_batch_repository_tests;
#[cfg(test)]
mod projection_repository_tests;
#[cfg(test)]
mod query_repository_tests;
#[cfg(test)]
mod retention_repository_tests;
#[cfg(test)]
mod source_adapters_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timeline_repository_tests;
#[cfg(test)]
mod unread_repository_tests;
