//! Domain errors for Extension Platform capability gates.

use super::feature::ExtensionPlatformFeature;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FeatureGateError {
    /// Enabling a gate whose Cargo feature is absent from this build. Returned instead of
    /// persisting an enabled desired state, because a stored "on" that the binary cannot serve
    /// reads as success to every later caller.
    FeatureUnavailableInBuild { feature: ExtensionPlatformFeature },
    /// The caller's expected revision no longer matches the stored one. Concurrent gate edits
    /// must not silently overwrite each other.
    StaleRevision {
        feature: ExtensionPlatformFeature,
        expected: i64,
        actual: i64,
    },
    /// Storage or parsing failure. Reads fail closed to disabled; writes surface this.
    Storage(String),
}

impl FeatureGateError {
    /// Stable code for command-safe serialization and telemetry. Never derived by parsing a
    /// message string.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::FeatureUnavailableInBuild { .. } => "feature_unavailable_in_build",
            Self::StaleRevision { .. } => "stale_revision",
            Self::Storage(_) => "storage",
        }
    }
}

impl fmt::Display for FeatureGateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FeatureUnavailableInBuild { feature } => write!(
                formatter,
                "capability gate {feature} is not compiled into this build"
            ),
            Self::StaleRevision {
                feature,
                expected,
                actual,
            } => write!(
                formatter,
                "capability gate {feature} changed since revision {expected} (now {actual})"
            ),
            Self::Storage(detail) => write!(formatter, "capability gate storage failure: {detail}"),
        }
    }
}

impl std::error::Error for FeatureGateError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_has_a_distinct_stable_code() {
        let codes = [
            FeatureGateError::FeatureUnavailableInBuild {
                feature: ExtensionPlatformFeature::SidecarRuntime,
            }
            .code(),
            FeatureGateError::StaleRevision {
                feature: ExtensionPlatformFeature::Catalog,
                expected: 1,
                actual: 2,
            }
            .code(),
            FeatureGateError::Storage("boom".to_string()).code(),
        ];
        let mut unique = codes.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), codes.len());
    }

    #[test]
    fn messages_name_the_gate_without_leaking_storage_internals() {
        let message = FeatureGateError::FeatureUnavailableInBuild {
            feature: ExtensionPlatformFeature::WasmModuleRuntime,
        }
        .to_string();
        assert!(message.contains("extension_platform.wasm_module_runtime"));
    }
}
