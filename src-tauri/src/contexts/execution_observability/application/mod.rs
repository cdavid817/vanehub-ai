mod error;
mod evaluation_engine;
mod evaluation_verifier;
pub(crate) mod evidence;

mod ports;
mod waterfall;
#[cfg(test)]
mod waterfall_tests;

pub(crate) use evaluation_engine::*;
pub(crate) use evaluation_verifier::*;
pub(crate) use evidence::*;

pub(crate) use error::ExecutionTelemetryError;
pub(crate) use ports::{
    EvaluationRepositoryPort, ExecutionIdentityPort, ExecutionObservabilityRepositoryPort,
    ExecutionSettingsPort, ExecutionTelemetryPort, ObservabilityCredentialPort,
};

#[cfg(test)]
pub(crate) mod test_adapter;
pub(crate) use waterfall::{derive_waterfall, SpanWaterfallMetadata};
