//! Extension Platform use cases.

mod feature_gates;
mod ports;
#[cfg(test)]
mod tests;

pub(crate) use feature_gates::{FeatureGateService, FeatureGateSnapshot, FeatureGateView};
pub(crate) use ports::{
    DefaultPrerequisites, FeatureGateAuditEntry, FeatureGateAuditSink, FeatureGateClock,
    FeatureGateDegradationEntry, FeatureGateRepository, FeatureGateWrite, NoForcedDisables,
    PersistedFeatureGate,
};
