//! Domain-to-transport conversion for capability gates.

use super::dto;
use crate::contexts::tooling::extension_platform::api::{
    ExtensionPlatformFeature, FeatureGateSnapshot, FeatureGateStatus, FeatureGateView,
};

pub(super) fn feature_from_dto(
    feature: dto::ExtensionPlatformFeatureDto,
) -> ExtensionPlatformFeature {
    match feature {
        dto::ExtensionPlatformFeatureDto::Catalog => ExtensionPlatformFeature::Catalog,
        dto::ExtensionPlatformFeatureDto::ExternalPackages => {
            ExtensionPlatformFeature::ExternalPackages
        }
        dto::ExtensionPlatformFeatureDto::LifecycleHooks => {
            ExtensionPlatformFeature::LifecycleHooks
        }
        dto::ExtensionPlatformFeatureDto::AuthorizationRules => {
            ExtensionPlatformFeature::AuthorizationRules
        }
        dto::ExtensionPlatformFeatureDto::Connectors => ExtensionPlatformFeature::Connectors,
        dto::ExtensionPlatformFeatureDto::WasmModuleRuntime => {
            ExtensionPlatformFeature::WasmModuleRuntime
        }
        dto::ExtensionPlatformFeatureDto::SidecarRuntime => {
            ExtensionPlatformFeature::SidecarRuntime
        }
    }
}

fn feature_to_dto(feature: ExtensionPlatformFeature) -> dto::ExtensionPlatformFeatureDto {
    match feature {
        ExtensionPlatformFeature::Catalog => dto::ExtensionPlatformFeatureDto::Catalog,
        ExtensionPlatformFeature::ExternalPackages => {
            dto::ExtensionPlatformFeatureDto::ExternalPackages
        }
        ExtensionPlatformFeature::LifecycleHooks => {
            dto::ExtensionPlatformFeatureDto::LifecycleHooks
        }
        ExtensionPlatformFeature::AuthorizationRules => {
            dto::ExtensionPlatformFeatureDto::AuthorizationRules
        }
        ExtensionPlatformFeature::Connectors => dto::ExtensionPlatformFeatureDto::Connectors,
        ExtensionPlatformFeature::WasmModuleRuntime => {
            dto::ExtensionPlatformFeatureDto::WasmModuleRuntime
        }
        ExtensionPlatformFeature::SidecarRuntime => {
            dto::ExtensionPlatformFeatureDto::SidecarRuntime
        }
    }
}

fn status_to_dto(status: &FeatureGateStatus) -> dto::FeatureGateStatusDto {
    match status {
        FeatureGateStatus::NotCompiled => dto::FeatureGateStatusDto::NotCompiled,
        FeatureGateStatus::RuntimeDisabled => dto::FeatureGateStatusDto::RuntimeDisabled,
        FeatureGateStatus::Enabled => dto::FeatureGateStatusDto::Enabled,
        FeatureGateStatus::BlockedByPrerequisite(reason) => {
            dto::FeatureGateStatusDto::BlockedByPrerequisite {
                reason: reason.as_str().to_string(),
            }
        }
        FeatureGateStatus::ForcedDisabled { reason } => dto::FeatureGateStatusDto::ForcedDisabled {
            reason: reason.clone(),
        },
    }
}

fn view_to_dto(view: &FeatureGateView) -> dto::FeatureGateDto {
    dto::FeatureGateDto {
        feature: feature_to_dto(view.feature),
        status: status_to_dto(&view.status),
        build_available: view.build_available,
        desired_enabled: view.desired_enabled,
        revision: view.revision,
        updated_at: view.updated_at.clone(),
        updated_by: view.updated_by.clone(),
        reason: view.reason.clone(),
    }
}

pub(super) fn snapshot_to_dto(snapshot: &FeatureGateSnapshot) -> dto::FeatureGateOverviewDto {
    dto::FeatureGateOverviewDto {
        gates: snapshot.views().map(view_to_dto).collect(),
    }
}
