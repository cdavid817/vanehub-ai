//! Extension Platform use cases.

mod developer_mode;
#[cfg(test)]
mod developer_mode_tests;
mod feature_gates;
mod package_verification;
#[cfg(test)]
mod package_verification_tests;
mod ports;
mod publisher_keys;
#[cfg(test)]
mod publisher_keys_tests;
mod snapshot_publication;
#[cfg(test)]
mod snapshot_publication_tests;
#[cfg(test)]
mod tests;

#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use developer_mode::{DeveloperModeClock, DeveloperModeService, DeveloperModeView};
pub(crate) use feature_gates::{FeatureGateService, FeatureGateSnapshot, FeatureGateView};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use package_verification::{PackageVerificationService, PublisherLookupUnavailable};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use ports::{
    DefaultPrerequisites, DeveloperModeAuditEntry, DeveloperModeAuditSink, DeveloperModeRepository,
    FeatureGateAuditEntry, FeatureGateAuditSink, FeatureGateClock, FeatureGateDegradationEntry,
    FeatureGateRepository, FeatureGateWrite, NoForcedDisables, PersistedFeatureGate,
    PublisherKeyDirectory, RuntimeGenerationRepository, SnapshotContentStore,
    SnapshotPointerRepository, TrustedPublisherKeyRepository, VersionClaimRepository,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use publisher_keys::{
    PublisherKeyClock, PublisherKeyPreview, PublisherKeyRequest, RepositoryPublisherKeyDirectory,
    TrustedPublisherKeyService,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use snapshot_publication::{PublishedSnapshot, SnapshotPublicationService};
