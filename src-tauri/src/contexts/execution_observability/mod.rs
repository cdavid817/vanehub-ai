//! Correlated execution runs, bounded timelines, and telemetry application ports.

pub(crate) mod api;
pub(crate) mod application;
pub(crate) mod domain;
mod evaluation_api;
pub(crate) mod infrastructure;
pub(crate) use evaluation_api::{EvaluationApi, StartEvaluationRequest};
