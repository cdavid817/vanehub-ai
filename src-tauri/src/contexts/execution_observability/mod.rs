//! Correlated execution runs, bounded timelines, and telemetry application ports.

pub(crate) mod api;
pub(crate) mod application;
pub(crate) mod domain;
mod evaluation_api;
mod evidence_api;
pub(crate) mod infrastructure;
pub(crate) use evaluation_api::{EvaluationApi, StartEvaluationRequest};
pub(crate) use evidence_api::ExecutionEvidenceApi;
