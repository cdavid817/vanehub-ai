//! Domain values to wire shapes.
//!
//! One direction only. Nothing here parses a DTO back into a domain value: a command receives
//! identifiers and a revision, and the backend resolves everything else from what it stored.

use crate::contexts::tooling::cli::domain::action::CliAllowedAction;
use crate::contexts::tooling::cli::domain::bulk::CliBulkActionPlan;
use crate::contexts::tooling::cli::domain::installation::{CliConflict, CliInstallation};
use crate::contexts::tooling::cli::domain::plan::CliActionPlan;
use crate::contexts::tooling::cli::domain::registry;
use crate::contexts::tooling::cli::domain::snapshot::{CliEnvironmentSnapshot, CliMutationSummary};
use crate::contexts::tooling::cli::domain::source::{CliSourceCapabilities, CliSourceSummary};

use super::dto::{
    CliActionPlanDto, CliAllowedActionDto, CliBulkActionItemDto, CliBulkActionPlanDto,
    CliBulkSkipDto, CliCommandPreviewDto, CliConflictDto, CliEnvironmentSnapshotDto,
    CliInstallationDto, CliMutationSummaryDto, CliSourceCapabilitiesDto, CliSourceSummaryDto,
};

pub(super) fn snapshot_to_dto(snapshot: CliEnvironmentSnapshot) -> CliEnvironmentSnapshotDto {
    // The registry is the one place tool identity lives. An unregistered agent id cannot reach a
    // snapshot, but if one ever did, echoing its id is truthful where inventing a name is not.
    let definition = registry::definition(snapshot.agent_id.as_str());
    CliEnvironmentSnapshotDto {
        schema_version: snapshot.schema_version,
        display_name: definition
            .map(|tool| tool.display_name.to_string())
            .unwrap_or_else(|| snapshot.agent_id.as_str().to_string()),
        provider: definition
            .map(|tool| tool.provider.to_string())
            .unwrap_or_default(),
        executable_names: definition
            .map(|tool| {
                tool.executable_names
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect()
            })
            .unwrap_or_default(),
        agent_id: snapshot.agent_id.as_str().to_string(),
        scope: snapshot.scope.as_str().to_string(),
        overall_state: snapshot.overall_state.as_str().to_string(),
        freshness: snapshot.freshness.as_str().to_string(),
        environment_fingerprint: snapshot.environment_fingerprint,
        path_selected_installation_id: snapshot
            .path_selected_installation_id
            .as_ref()
            .map(|id| id.as_str().to_string()),
        recommended_installation_id: snapshot
            .recommended_installation_id
            .as_ref()
            .map(|id| id.as_str().to_string()),
        discovery: snapshot.discovery.as_str().to_string(),
        executable: snapshot.executable.as_str().to_string(),
        authentication: snapshot.authentication.as_str().to_string(),
        readiness: snapshot.readiness.as_str().to_string(),
        compatibility: snapshot.compatibility.as_str().to_string(),
        update: snapshot.update.as_str().to_string(),
        conflicts: snapshot.conflicts.iter().map(conflict_to_dto).collect(),
        sources: snapshot.sources.iter().map(source_to_dto).collect(),
        allowed_actions: snapshot.allowed_actions.iter().map(action_to_dto).collect(),
        last_mutation: snapshot.last_mutation.as_ref().map(mutation_to_dto),
        last_operation_id: snapshot.last_operation_id,
        checked_at: snapshot.checked_at.map(|value| value.to_rfc3339()),
        installations: snapshot
            .installations
            .into_iter()
            .map(installation_to_dto)
            .collect(),
    }
}

fn installation_to_dto(installation: CliInstallation) -> CliInstallationDto {
    CliInstallationDto {
        id: installation.id.as_str().to_string(),
        executable_path: installation.executable_path,
        canonical_path: installation.canonical_path,
        alias_paths: installation.alias_paths,
        target_missing: installation.target_missing,
        reported_version: installation
            .reported_version
            .map(|version| version.as_str().to_string()),
        source_id: installation
            .source_id
            .as_ref()
            .map(|id| id.as_str().to_string()),
        source_kind: installation.source_kind.as_str().to_string(),
        source_confidence: installation.source_confidence.as_str().to_string(),
        path_priority: installation.path_priority,
        environment_origin: installation.environment_origin.as_str().to_string(),
        executable_status: installation.executable_status.as_str().to_string(),
    }
}

fn conflict_to_dto(conflict: &CliConflict) -> CliConflictDto {
    CliConflictDto {
        kind: conflict.kind.as_str().to_string(),
        severity: conflict.severity.as_str().to_string(),
        installation_ids: conflict
            .installations
            .iter()
            .map(|id| id.as_str().to_string())
            .collect(),
        blocks_mutation: conflict.blocks_mutation,
        blocks_launch: conflict.blocks_launch,
        reason_code: conflict.reason_code.to_string(),
    }
}

fn source_to_dto(source: &CliSourceSummary) -> CliSourceSummaryDto {
    CliSourceSummaryDto {
        source_id: source.source_id.as_str().to_string(),
        kind: source.kind.as_str().to_string(),
        supported_on_this_platform: source.supported_on_this_platform,
        available_version_count: source.available_version_count,
        available_versions: source
            .available_versions
            .iter()
            .map(|version| version.as_str().to_string())
            .collect(),
        capabilities: capabilities_to_dto(&source.capabilities),
    }
}

fn capabilities_to_dto(capabilities: &CliSourceCapabilities) -> CliSourceCapabilitiesDto {
    CliSourceCapabilitiesDto {
        install: capabilities.install.as_str().to_string(),
        upgrade: capabilities.upgrade.as_str().to_string(),
        downgrade: capabilities.downgrade.as_str().to_string(),
        reinstall: capabilities.reinstall.as_str().to_string(),
        uninstall: capabilities.uninstall,
        repair: capabilities.repair.as_str().to_string(),
    }
}

fn action_to_dto(action: &CliAllowedAction) -> CliAllowedActionDto {
    CliAllowedActionDto {
        action: action.action.as_str().to_string(),
        source_id: action.source_id.as_str().to_string(),
        target_mode: action.target_mode.as_str().to_string(),
        default_target: action.default_target.clone(),
        requires_target_selection: action.requires_target_selection,
        reason_code: action.reason_code.map(|code| code.as_str().to_string()),
    }
}

fn mutation_to_dto(mutation: &CliMutationSummary) -> CliMutationSummaryDto {
    CliMutationSummaryDto {
        outcome: mutation.outcome.as_str().to_string(),
        source_id: mutation.source_id.as_str().to_string(),
        action: mutation.action.clone(),
        target_version: mutation.target_version.clone(),
        operation_id: mutation.operation_id.clone(),
        completed_at: mutation.completed_at.to_rfc3339(),
    }
}

pub(super) fn plan_to_dto(plan: CliActionPlan) -> CliActionPlanDto {
    CliActionPlanDto {
        id: plan.id.as_str().to_string(),
        revision: plan.revision,
        agent_id: plan.agent_id.as_str().to_string(),
        action: plan.action.as_str().to_string(),
        source_id: plan.source_id.as_str().to_string(),
        installation_id: plan
            .installation_id
            .as_ref()
            .map(|id| id.as_str().to_string()),
        current_version: plan.current_version,
        target_version: plan.target_version,
        channel: plan.channel,
        command_preview: CliCommandPreviewDto {
            program: plan.command_preview.program,
            args: plan.command_preview.args,
        },
        preconditions: plan
            .preconditions
            .iter()
            .map(|precondition| precondition.as_str().to_string())
            .collect(),
        warnings: plan
            .warnings
            .iter()
            .map(|warning| warning.as_str().to_string())
            .collect(),
        requires_elevation: plan.requires_elevation,
        requires_network: plan.requires_network,
        state: plan.state.as_str().to_string(),
        created_at: plan.created_at.to_rfc3339(),
        expires_at: plan.expires_at.to_rfc3339(),
    }
}

pub(super) fn bulk_plan_to_dto(plan: CliBulkActionPlan) -> CliBulkActionPlanDto {
    CliBulkActionPlanDto {
        id: plan.id.as_str().to_string(),
        revision: plan.revision,
        items: plan
            .items
            .into_iter()
            .map(|item| CliBulkActionItemDto {
                agent_id: item.agent_id.as_str().to_string(),
                plan_id: item.plan_id.as_str().to_string(),
                source_id: item.source_id.as_str().to_string(),
                current_version: item.current_version,
                target_version: item.target_version,
            })
            .collect(),
        skipped: plan
            .skipped
            .into_iter()
            .map(|skip| CliBulkSkipDto {
                agent_id: skip.agent_id.as_str().to_string(),
                reason: skip.reason.as_str().to_string(),
            })
            .collect(),
        created_at: plan.created_at.to_rfc3339(),
        expires_at: plan.expires_at.to_rfc3339(),
    }
}
