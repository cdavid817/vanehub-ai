//! The wire shapes the CLI environment commands return.
//!
//! Separate types from the domain and from any stored row. Two reasons, both load-bearing: a
//! stored document may carry fields the frontend must not see, and a domain rename must not
//! silently change a wire contract the TypeScript side asserts.
//!
//! Every enum crosses as its domain `as_str()` value rather than as a re-declared serde enum. A
//! second enum here would be a second source of truth that drifts silently; `as_str` is already
//! covered by a drift test on both sides.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliEnvironmentSnapshotDto {
    pub(crate) schema_version: u16,
    pub(crate) agent_id: String,
    /// Identity from the backend registry, carried so no UI has to keep a second copy of the
    /// catalog. A frontend list of display names drifts the first time a tool is renamed here.
    pub(crate) display_name: String,
    pub(crate) provider: String,
    /// Every name this executable may carry. Windows shims mean one CLI appears under several.
    pub(crate) executable_names: Vec<String>,
    pub(crate) scope: String,
    pub(crate) overall_state: String,
    pub(crate) freshness: String,
    pub(crate) environment_fingerprint: String,
    pub(crate) installations: Vec<CliInstallationDto>,
    /// What this host's PATH would actually run. `null` means nothing is on PATH -- a real answer.
    pub(crate) path_selected_installation_id: Option<String>,
    /// What the backend would act on. Differs from the PATH-selected one exactly when something is
    /// wrong worth showing.
    pub(crate) recommended_installation_id: Option<String>,
    pub(crate) discovery: String,
    pub(crate) executable: String,
    pub(crate) authentication: String,
    pub(crate) readiness: String,
    pub(crate) compatibility: String,
    pub(crate) update: String,
    pub(crate) conflicts: Vec<CliConflictDto>,
    pub(crate) sources: Vec<CliSourceSummaryDto>,
    pub(crate) allowed_actions: Vec<CliAllowedActionDto>,
    pub(crate) last_mutation: Option<CliMutationSummaryDto>,
    pub(crate) last_operation_id: Option<String>,
    pub(crate) checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliInstallationDto {
    pub(crate) id: String,
    pub(crate) executable_path: String,
    pub(crate) canonical_path: Option<String>,
    /// Launcher aliases folded into this installation, so the UI can explain why three files on
    /// disk are one entry.
    pub(crate) alias_paths: Vec<String>,
    pub(crate) target_missing: bool,
    pub(crate) reported_version: Option<String>,
    pub(crate) source_id: Option<String>,
    pub(crate) source_kind: String,
    pub(crate) source_confidence: String,
    pub(crate) path_priority: Option<u32>,
    pub(crate) environment_origin: String,
    pub(crate) executable_status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliConflictDto {
    pub(crate) kind: String,
    pub(crate) severity: String,
    pub(crate) installation_ids: Vec<String>,
    pub(crate) blocks_mutation: bool,
    pub(crate) blocks_launch: bool,
    /// Stable code the frontend localizes. It never parses the kind or a message string.
    pub(crate) reason_code: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliSourceSummaryDto {
    pub(crate) source_id: String,
    pub(crate) kind: String,
    pub(crate) supported_on_this_platform: bool,
    pub(crate) available_version_count: Option<usize>,
    /// This source's own list, newest first. A target selector reads it rather than rebuilding one,
    /// and two sources' lists are never merged.
    pub(crate) available_versions: Vec<String>,
    pub(crate) capabilities: CliSourceCapabilitiesDto,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliSourceCapabilitiesDto {
    pub(crate) install: String,
    pub(crate) upgrade: String,
    pub(crate) downgrade: String,
    pub(crate) reinstall: String,
    pub(crate) uninstall: bool,
    pub(crate) repair: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliAllowedActionDto {
    pub(crate) action: String,
    pub(crate) source_id: String,
    pub(crate) target_mode: String,
    pub(crate) default_target: Option<String>,
    pub(crate) requires_target_selection: bool,
    pub(crate) reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliMutationSummaryDto {
    pub(crate) outcome: String,
    pub(crate) source_id: String,
    pub(crate) action: String,
    pub(crate) target_version: Option<String>,
    pub(crate) operation_id: String,
    pub(crate) completed_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliActionPlanDto {
    pub(crate) id: String,
    /// Submitted back on execution. A plan revised in between is refused rather than run.
    pub(crate) revision: u32,
    pub(crate) agent_id: String,
    pub(crate) action: String,
    pub(crate) source_id: String,
    pub(crate) installation_id: Option<String>,
    pub(crate) current_version: Option<String>,
    pub(crate) target_version: Option<String>,
    pub(crate) channel: Option<String>,
    /// Exactly what will run, as argv. Never a shell string.
    pub(crate) command_preview: CliCommandPreviewDto,
    pub(crate) preconditions: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) requires_elevation: bool,
    pub(crate) requires_network: bool,
    pub(crate) state: String,
    pub(crate) created_at: String,
    pub(crate) expires_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliCommandPreviewDto {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliBulkActionPlanDto {
    pub(crate) id: String,
    pub(crate) revision: u32,
    pub(crate) items: Vec<CliBulkActionItemDto>,
    /// Tools excluded from the batch, each with the reason. A silently shorter list would read as
    /// "everything is up to date".
    pub(crate) skipped: Vec<CliBulkSkipDto>,
    pub(crate) created_at: String,
    pub(crate) expires_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliBulkActionItemDto {
    pub(crate) agent_id: String,
    pub(crate) plan_id: String,
    pub(crate) source_id: String,
    pub(crate) current_version: Option<String>,
    pub(crate) target_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliBulkSkipDto {
    pub(crate) agent_id: String,
    pub(crate) reason: String,
}

/// What a `prepare_*` command returns: an id to watch, before any process or network work.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliOperationHandleDto {
    pub(crate) operation_id: String,
}

#[cfg(test)]
#[path = "dto_tests.rs"]
mod tests;
