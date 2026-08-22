//! Extension Platform use cases.

mod feature_gates;
mod package_verification;
#[cfg(test)]
mod package_verification_tests;
mod ports;
#[cfg(test)]
mod tests;

pub(crate) use feature_gates::{FeatureGateService, FeatureGateSnapshot, FeatureGateView};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use package_verification::{PackageVerificationService, PublisherLookupUnavailable};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use ports::{
    DefaultPrerequisites, FeatureGateAuditEntry, FeatureGateAuditSink, FeatureGateClock,
    FeatureGateDegradationEntry, FeatureGateRepository, FeatureGateWrite, NoForcedDisables,
    PersistedFeatureGate, PublisherKeyDirectory,
};
