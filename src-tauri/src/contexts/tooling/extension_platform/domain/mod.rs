//! Extension Platform domain model.
//!
//! Task Group 0 establishes only the capability-gate vocabulary. Package, manifest, snapshot,
//! lifecycle, runtime, and contribution models arrive with their own task groups.

mod error;
mod feature;

pub(crate) use error::FeatureGateError;
pub(crate) use feature::{
    evaluate_gate, ExtensionPlatformFeature, FeatureGateEvaluation, FeatureGateStatus,
    PrerequisiteReason, ALL_FEATURES,
};
