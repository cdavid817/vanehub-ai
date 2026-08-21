//! Transport shapes for Extension Platform capability gates.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExtensionPlatformFeatureDto {
    Catalog,
    ExternalPackages,
    LifecycleHooks,
    AuthorizationRules,
    Connectors,
    WasmModuleRuntime,
    SidecarRuntime,
}

/// Effective status as a discriminated union.
///
/// `NotCompiled` and `RuntimeDisabled` stay separate members on the wire too. A UI that collapsed
/// them would tell an operator to "turn it on" for a gate no amount of toggling can reach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FeatureGateStatusDto {
    NotCompiled,
    RuntimeDisabled,
    Enabled,
    BlockedByPrerequisite { reason: String },
    ForcedDisabled { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeatureGateDto {
    pub(crate) feature: ExtensionPlatformFeatureDto,
    pub(crate) status: FeatureGateStatusDto,
    pub(crate) build_available: bool,
    pub(crate) desired_enabled: bool,
    pub(crate) revision: i64,
    pub(crate) updated_at: Option<String>,
    pub(crate) updated_by: Option<String>,
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FeatureGateOverviewDto {
    pub(crate) gates: Vec<FeatureGateDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetFeatureGateRequest {
    pub(crate) feature: ExtensionPlatformFeatureDto,
    pub(crate) desired_enabled: bool,
    /// The revision the caller last observed. A mismatch is rejected rather than overwritten.
    pub(crate) expected_revision: i64,
    pub(crate) reason: Option<String>,
}
