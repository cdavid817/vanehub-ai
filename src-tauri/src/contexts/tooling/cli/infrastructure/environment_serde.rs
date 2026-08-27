//! Versioned JSON encoding for persisted CLI environment values.
//!
//! Domain types carry no serde derives on purpose: storage column names and wire aliases have no
//! business leaking into them, and the persisted shape must be able to change independently. The
//! conversion is explicit here, in both directions, and **decoding is fallible everywhere**.
//!
//! Every document carries a `documentVersion`. A version this build does not know yields a typed
//! error, so an older binary reading a newer database reports storage trouble instead of silently
//! constructing a half-populated snapshot.

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use crate::contexts::tooling::cli::domain::action::{
    CliActionKind, CliActionReasonCode, CliAllowedAction,
};
use crate::contexts::tooling::cli::domain::bulk::{
    CliBulkActionItem, CliBulkActionPlan, CliBulkSkip, CliBulkSkipReason,
};
use crate::contexts::tooling::cli::domain::catalog::{
    CliCatalogStatus, CliCatalogUnavailableReason, CliVersionCatalog,
};
use crate::contexts::tooling::cli::domain::ids::{
    CliActionPlanId, CliBulkPlanId, CliInstallationId, CliSourceId, CliToolId,
};
use crate::contexts::tooling::cli::domain::installation::{
    CliConflict, CliConflictKind, CliConflictSeverity, CliEnvironmentOrigin, CliInstallation,
};
use crate::contexts::tooling::cli::domain::plan::{
    CliActionPlan, CliActionPlanState, CliCommandPreview, CliFallbackPolicy, CliPlanWarning,
    CliPrecondition,
};
use crate::contexts::tooling::cli::domain::snapshot::{
    CliEnvironmentScope, CliEnvironmentSnapshot, CliMutationOutcome, CliMutationSummary,
    SNAPSHOT_SCHEMA_VERSION,
};
use crate::contexts::tooling::cli::domain::source::{
    CliDynamicCapability, CliSourceCapabilities, CliSourceConfidence, CliSourceKind,
    CliSourceManagement, CliSourceSummary, CliTargetVersionMode,
};
use crate::contexts::tooling::cli::domain::status::{
    CliAuthenticationStatus, CliCompatibilityStatus, CliDiscoveryStatus, CliExecutableStatus,
    CliFreshness, CliOverallState, CliReadinessStatus, CliUpdateStatus,
};
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

/// Bumped when the persisted layout changes in a way an older build cannot read.
const DOCUMENT_VERSION: u64 = 1;

type Decoded<T> = Result<T, String>;

fn field<'a>(value: &'a Value, key: &str) -> Decoded<&'a Value> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field `{key}`"))
}

fn text(value: &Value, key: &str) -> Decoded<String> {
    field(value, key)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("field `{key}` is not a string"))
}

fn optional_text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn flag(value: &Value, key: &str) -> Decoded<bool> {
    field(value, key)?
        .as_bool()
        .ok_or_else(|| format!("field `{key}` is not a boolean"))
}

fn array<'a>(value: &'a Value, key: &str) -> Decoded<&'a Vec<Value>> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("field `{key}` is not an array"))
}

fn timestamp(value: &Value, key: &str) -> Decoded<DateTime<Utc>> {
    let raw = text(value, key)?;
    DateTime::parse_from_rfc3339(&raw)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|error| format!("field `{key}` is not an RFC 3339 timestamp: {error}"))
}

fn optional_timestamp(value: &Value, key: &str) -> Decoded<Option<DateTime<Utc>>> {
    match value.get(key).and_then(Value::as_str) {
        None => Ok(None),
        Some(raw) => DateTime::parse_from_rfc3339(raw)
            .map(|parsed| Some(parsed.with_timezone(&Utc)))
            .map_err(|error| format!("field `{key}` is not an RFC 3339 timestamp: {error}")),
    }
}

fn tool_id(value: &Value, key: &str) -> Decoded<CliToolId> {
    CliToolId::new(text(value, key)?).map_err(|error| error.to_string())
}

fn source_id_of(value: &Value, key: &str) -> Decoded<CliSourceId> {
    CliSourceId::new(text(value, key)?).map_err(|error| error.to_string())
}

fn installation_id(value: &Value, key: &str) -> Decoded<CliInstallationId> {
    CliInstallationId::new(text(value, key)?).map_err(|error| error.to_string())
}

fn check_version(value: &Value) -> Decoded<()> {
    let version = value
        .get("documentVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| "missing documentVersion".to_string())?;
    if version != DOCUMENT_VERSION {
        // An older binary reading a newer row. Reporting storage trouble is correct; guessing at
        // the shape would produce a snapshot that looks valid and is not.
        return Err(format!(
            "documentVersion {version} is not readable by this build (expects {DOCUMENT_VERSION})"
        ));
    }
    Ok(())
}

/// Maps a wire string back to an enum, naming the field when it does not match.
macro_rules! parse_enum {
    ($value:expr, $key:expr, $ty:ty, [$($variant:expr),+ $(,)?]) => {{
        let raw = text($value, $key)?;
        let found: Option<$ty> = [$($variant),+]
            .into_iter()
            .find(|candidate: &$ty| candidate.as_str() == raw);
        found.ok_or_else(|| format!("field `{}` has unknown value `{}`", $key, raw))?
    }};
}

// ---------------------------------------------------------------------------------------------
// Snapshot

pub(super) fn encode_snapshot(snapshot: &CliEnvironmentSnapshot) -> Value {
    json!({
        "documentVersion": DOCUMENT_VERSION,
        "schemaVersion": snapshot.schema_version,
        "agentId": snapshot.agent_id.as_str(),
        "scope": snapshot.scope.as_str(),
        "overallState": snapshot.overall_state.as_str(),
        "freshness": snapshot.freshness.as_str(),
        "environmentFingerprint": snapshot.environment_fingerprint,
        "installations": snapshot.installations.iter().map(encode_installation).collect::<Vec<_>>(),
        "pathSelectedInstallationId": snapshot.path_selected_installation_id.as_ref().map(CliInstallationId::as_str),
        "recommendedInstallationId": snapshot.recommended_installation_id.as_ref().map(CliInstallationId::as_str),
        "discovery": snapshot.discovery.as_str(),
        "executable": snapshot.executable.as_str(),
        "authentication": snapshot.authentication.as_str(),
        "readiness": snapshot.readiness.as_str(),
        "compatibility": snapshot.compatibility.as_str(),
        "update": snapshot.update.as_str(),
        "conflicts": snapshot.conflicts.iter().map(encode_conflict).collect::<Vec<_>>(),
        "sources": snapshot.sources.iter().map(encode_source_summary).collect::<Vec<_>>(),
        "allowedActions": snapshot.allowed_actions.iter().map(encode_allowed_action).collect::<Vec<_>>(),
        "lastMutation": snapshot.last_mutation.as_ref().map(encode_mutation_summary),
        "lastOperationId": snapshot.last_operation_id,
        "checkedAt": snapshot.checked_at.map(|value| value.to_rfc3339()),
    })
}

pub(super) fn decode_snapshot(value: Value) -> Decoded<CliEnvironmentSnapshot> {
    check_version(&value)?;
    let schema_version = field(&value, "schemaVersion")?
        .as_u64()
        .ok_or_else(|| "schemaVersion is not a number".to_string())?;
    if schema_version != u64::from(SNAPSHOT_SCHEMA_VERSION) {
        return Err(format!(
            "snapshot schemaVersion {schema_version} is unknown"
        ));
    }

    let mut installations = Vec::new();
    for entry in array(&value, "installations")? {
        installations.push(decode_installation(entry)?);
    }
    let mut conflicts = Vec::new();
    for entry in array(&value, "conflicts")? {
        conflicts.push(decode_conflict(entry)?);
    }
    let mut sources = Vec::new();
    for entry in array(&value, "sources")? {
        sources.push(decode_source_summary(entry)?);
    }
    let mut allowed_actions = Vec::new();
    for entry in array(&value, "allowedActions")? {
        allowed_actions.push(decode_allowed_action(entry)?);
    }

    Ok(CliEnvironmentSnapshot {
        schema_version: SNAPSHOT_SCHEMA_VERSION,
        agent_id: tool_id(&value, "agentId")?,
        scope: CliEnvironmentScope::LocalDesktop,
        overall_state: parse_enum!(
            &value,
            "overallState",
            CliOverallState,
            [
                CliOverallState::Broken,
                CliOverallState::Conflict,
                CliOverallState::NeedsAuth,
                CliOverallState::UpdateAvailable,
                CliOverallState::Ready,
                CliOverallState::Missing,
                CliOverallState::Unknown,
            ]
        ),
        freshness: parse_enum!(
            &value,
            "freshness",
            CliFreshness,
            [
                CliFreshness::Never,
                CliFreshness::Fresh,
                CliFreshness::Stale,
                CliFreshness::Refreshing,
            ]
        ),
        environment_fingerprint: text(&value, "environmentFingerprint")?,
        installations,
        path_selected_installation_id: optional_id(&value, "pathSelectedInstallationId")?,
        recommended_installation_id: optional_id(&value, "recommendedInstallationId")?,
        discovery: parse_enum!(
            &value,
            "discovery",
            CliDiscoveryStatus,
            [
                CliDiscoveryStatus::NotScanned,
                CliDiscoveryStatus::NotFound,
                CliDiscoveryStatus::FoundOne,
                CliDiscoveryStatus::FoundMultiple,
            ]
        ),
        executable: decode_executable_status(&value, "executable")?,
        authentication: parse_enum!(
            &value,
            "authentication",
            CliAuthenticationStatus,
            [
                CliAuthenticationStatus::Authenticated,
                CliAuthenticationStatus::Required,
                CliAuthenticationStatus::Expired,
                CliAuthenticationStatus::Unknown,
                CliAuthenticationStatus::NotApplicable,
            ]
        ),
        readiness: parse_enum!(
            &value,
            "readiness",
            CliReadinessStatus,
            [
                CliReadinessStatus::Ready,
                CliReadinessStatus::NeedsAuth,
                CliReadinessStatus::MissingDependency,
                CliReadinessStatus::Misconfigured,
                CliReadinessStatus::Broken,
                CliReadinessStatus::Unknown,
            ]
        ),
        compatibility: parse_enum!(
            &value,
            "compatibility",
            CliCompatibilityStatus,
            [
                CliCompatibilityStatus::Supported,
                CliCompatibilityStatus::UnsupportedVersion,
                CliCompatibilityStatus::UnsupportedPlatform,
                CliCompatibilityStatus::Unknown,
            ]
        ),
        update: parse_enum!(
            &value,
            "update",
            CliUpdateStatus,
            [
                CliUpdateStatus::NotApplicable,
                CliUpdateStatus::UpToDate,
                CliUpdateStatus::Available,
                CliUpdateStatus::Ahead,
                CliUpdateStatus::CatalogUnavailable,
                CliUpdateStatus::Unknown,
            ]
        ),
        conflicts,
        sources,
        allowed_actions,
        last_mutation: value
            .get("lastMutation")
            .filter(|entry| !entry.is_null())
            .map(decode_mutation_summary)
            .transpose()?,
        last_operation_id: optional_text(&value, "lastOperationId"),
        checked_at: optional_timestamp(&value, "checkedAt")?,
    })
}

fn optional_id(value: &Value, key: &str) -> Decoded<Option<CliInstallationId>> {
    match value.get(key).and_then(Value::as_str) {
        None => Ok(None),
        Some(raw) => CliInstallationId::new(raw)
            .map(Some)
            .map_err(|error| error.to_string()),
    }
}

fn decode_executable_status(value: &Value, key: &str) -> Decoded<CliExecutableStatus> {
    Ok(parse_enum!(
        value,
        key,
        CliExecutableStatus,
        [
            CliExecutableStatus::NotApplicable,
            CliExecutableStatus::Healthy,
            CliExecutableStatus::Broken,
            CliExecutableStatus::TimedOut,
            CliExecutableStatus::PermissionDenied,
            CliExecutableStatus::UnsupportedArchitecture,
            CliExecutableStatus::Unknown,
        ]
    ))
}

// ---------------------------------------------------------------------------------------------
// Installation

fn encode_installation(installation: &CliInstallation) -> Value {
    json!({
        "id": installation.id.as_str(),
        "executablePath": installation.executable_path,
        "canonicalPath": installation.canonical_path,
        "aliasPaths": installation.alias_paths,
        "targetMissing": installation.target_missing,
        "reportedVersion": installation.reported_version.as_ref().map(NormalizedCliVersion::as_str),
        "sourceId": installation.source_id.as_ref().map(CliSourceId::as_str),
        "sourceKind": installation.source_kind.as_str(),
        "sourceConfidence": installation.source_confidence.as_str(),
        "pathPriority": installation.path_priority,
        "environmentOrigin": installation.environment_origin.as_str(),
        "executableStatus": installation.executable_status.as_str(),
    })
}

fn decode_installation(value: &Value) -> Decoded<CliInstallation> {
    let alias_paths = array(value, "aliasPaths")?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "aliasPaths entry is not a string".to_string())
        })
        .collect::<Decoded<Vec<_>>>()?;

    Ok(CliInstallation {
        id: installation_id(value, "id")?,
        executable_path: text(value, "executablePath")?,
        canonical_path: optional_text(value, "canonicalPath"),
        alias_paths,
        target_missing: flag(value, "targetMissing")?,
        reported_version: optional_text(value, "reportedVersion").map(NormalizedCliVersion::parse),
        source_id: match value.get("sourceId").and_then(Value::as_str) {
            None => None,
            Some(raw) => Some(CliSourceId::new(raw).map_err(|error| error.to_string())?),
        },
        source_kind: parse_enum!(
            value,
            "sourceKind",
            CliSourceKind,
            [
                CliSourceKind::Npm,
                CliSourceKind::Winget,
                CliSourceKind::VendorInstaller,
                CliSourceKind::Homebrew,
                CliSourceKind::Bun,
                CliSourceKind::Volta,
                CliSourceKind::Desktop,
                CliSourceKind::System,
                CliSourceKind::Manual,
                CliSourceKind::Unknown,
            ]
        ),
        source_confidence: parse_enum!(
            value,
            "sourceConfidence",
            CliSourceConfidence,
            [
                CliSourceConfidence::Unknown,
                CliSourceConfidence::Inferred,
                CliSourceConfidence::Verified,
            ]
        ),
        path_priority: value
            .get("pathPriority")
            .and_then(Value::as_u64)
            .map(|value| u32::try_from(value).unwrap_or(u32::MAX)),
        environment_origin: parse_enum!(
            value,
            "environmentOrigin",
            CliEnvironmentOrigin,
            [
                CliEnvironmentOrigin::Path,
                CliEnvironmentOrigin::KnownLocation
            ]
        ),
        executable_status: decode_executable_status(value, "executableStatus")?,
    })
}

// ---------------------------------------------------------------------------------------------
// Conflict, source summary, allowed action, mutation summary

fn encode_conflict(conflict: &CliConflict) -> Value {
    json!({
        "kind": conflict.kind.as_str(),
        "severity": conflict.severity.as_str(),
        "installations": conflict.installations.iter().map(CliInstallationId::as_str).collect::<Vec<_>>(),
        "blocksMutation": conflict.blocks_mutation,
        "blocksLaunch": conflict.blocks_launch,
        "reasonCode": conflict.reason_code,
    })
}

fn decode_conflict(value: &Value) -> Decoded<CliConflict> {
    let installations = array(value, "installations")?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .ok_or_else(|| "conflict installation is not a string".to_string())
                .and_then(|raw| CliInstallationId::new(raw).map_err(|e| e.to_string()))
        })
        .collect::<Decoded<Vec<_>>>()?;
    let kind = parse_enum!(
        value,
        "kind",
        CliConflictKind,
        [
            CliConflictKind::DuplicateLauncherAlias,
            CliConflictKind::PathShadowing,
            CliConflictKind::BrokenPathPrecedence,
            CliConflictKind::MultipleInstallationSources,
            CliConflictKind::VersionDivergence,
            CliConflictKind::AmbiguousSourceOwnership,
            CliConflictKind::EnvironmentPathDivergence,
            CliConflictKind::ArchitectureMismatch,
            CliConflictKind::StaleLauncherTarget,
        ]
    );
    Ok(CliConflict {
        kind,
        severity: parse_enum!(
            value,
            "severity",
            CliConflictSeverity,
            [
                CliConflictSeverity::Info,
                CliConflictSeverity::Warning,
                CliConflictSeverity::Error,
            ]
        ),
        installations,
        blocks_mutation: flag(value, "blocksMutation")?,
        blocks_launch: flag(value, "blocksLaunch")?,
        // Derived from the kind rather than trusted from the row: the two must agree, and the kind
        // is the authority.
        reason_code: kind.as_str(),
    })
}

fn encode_source_summary(summary: &CliSourceSummary) -> Value {
    json!({
        "sourceId": summary.source_id.as_str(),
        "kind": summary.kind.as_str(),
        "capabilities": encode_capabilities(&summary.capabilities),
        "supportedOnThisPlatform": summary.supported_on_this_platform,
        "availableVersionCount": summary.available_version_count,
        "management": summary.management.as_str(),
        "guidanceCode": summary.guidance_code,
        "availableVersions": summary
            .available_versions
            .iter()
            .map(NormalizedCliVersion::as_str)
            .collect::<Vec<_>>(),
    })
}

fn decode_source_summary(value: &Value) -> Decoded<CliSourceSummary> {
    Ok(CliSourceSummary {
        source_id: source_id_of(value, "sourceId")?,
        kind: parse_enum!(
            value,
            "kind",
            CliSourceKind,
            [
                CliSourceKind::Npm,
                CliSourceKind::Winget,
                CliSourceKind::VendorInstaller,
                CliSourceKind::Homebrew,
                CliSourceKind::Bun,
                CliSourceKind::Volta,
                CliSourceKind::Desktop,
                CliSourceKind::System,
                CliSourceKind::Manual,
                CliSourceKind::Unknown,
            ]
        ),
        capabilities: decode_capabilities(field(value, "capabilities")?)?,
        supported_on_this_platform: flag(value, "supportedOnThisPlatform")?,
        management: parse_enum!(
            value,
            "management",
            CliSourceManagement,
            [
                CliSourceManagement::Managed,
                CliSourceManagement::DetectOnly
            ]
        ),
        // Re-derived from the kind rather than trusted from the row: a stored code from an
        // older build would otherwise outlive the wording it referred to.
        guidance_code: parse_enum!(
            value,
            "kind",
            CliSourceKind,
            [
                CliSourceKind::Npm,
                CliSourceKind::Winget,
                CliSourceKind::VendorInstaller,
                CliSourceKind::Homebrew,
                CliSourceKind::Bun,
                CliSourceKind::Volta,
                CliSourceKind::Desktop,
                CliSourceKind::System,
                CliSourceKind::Manual,
                CliSourceKind::Unknown,
            ]
        )
        .guidance_code(),
        available_version_count: value
            .get("availableVersionCount")
            .and_then(Value::as_u64)
            .map(|count| count as usize),
        available_versions: array(value, "availableVersions")?
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .map(NormalizedCliVersion::parse)
                    .ok_or_else(|| "availableVersions entry is not a string".to_string())
            })
            .collect::<Decoded<Vec<_>>>()?,
    })
}

fn encode_capabilities(capabilities: &CliSourceCapabilities) -> Value {
    json!({
        "install": capabilities.install.as_str(),
        "upgrade": capabilities.upgrade.as_str(),
        "downgrade": capabilities.downgrade.as_str(),
        "reinstall": capabilities.reinstall.as_str(),
        "uninstall": capabilities.uninstall,
        "repairRequiresPreflight": capabilities.repair.needs_preflight(),
    })
}

fn decode_capabilities(value: &Value) -> Decoded<CliSourceCapabilities> {
    let mode = |key: &str| -> Decoded<CliTargetVersionMode> {
        Ok(parse_enum!(
            value,
            key,
            CliTargetVersionMode,
            [
                CliTargetVersionMode::Unsupported,
                CliTargetVersionMode::LatestOnly,
                CliTargetVersionMode::Exact,
            ]
        ))
    };
    Ok(CliSourceCapabilities {
        install: mode("install")?,
        upgrade: mode("upgrade")?,
        downgrade: mode("downgrade")?,
        reinstall: mode("reinstall")?,
        uninstall: flag(value, "uninstall")?,
        repair: if flag(value, "repairRequiresPreflight")? {
            CliDynamicCapability::RequiresPreflight
        } else {
            CliDynamicCapability::Unsupported
        },
    })
}

fn encode_allowed_action(action: &CliAllowedAction) -> Value {
    json!({
        "action": action.action.as_str(),
        "sourceId": action.source_id.as_str(),
        "targetMode": action.target_mode.as_str(),
        "defaultTarget": action.default_target,
        "requiresTargetSelection": action.requires_target_selection,
        "reasonCode": action.reason_code.map(CliActionReasonCode::as_str),
    })
}

fn decode_allowed_action(value: &Value) -> Decoded<CliAllowedAction> {
    let reason_code = match value.get("reasonCode").and_then(Value::as_str) {
        None => None,
        Some(raw) => Some(
            [
                CliActionReasonCode::AlreadyCurrent,
                CliActionReasonCode::DetectOnlySource,
                CliActionReasonCode::CatalogUnavailable,
                CliActionReasonCode::SourceUnavailableOnPlatform,
                CliActionReasonCode::UnorderedVersions,
                CliActionReasonCode::SourceOwnershipUnproven,
                CliActionReasonCode::ActionUnsupportedBySource,
                CliActionReasonCode::ActiveInstallationBroken,
                CliActionReasonCode::ConflictBlocksMutation,
            ]
            .into_iter()
            .find(|candidate| candidate.as_str() == raw)
            .ok_or_else(|| format!("unknown action reason code `{raw}`"))?,
        ),
    };
    Ok(CliAllowedAction {
        action: decode_action_kind(value, "action")?,
        source_id: source_id_of(value, "sourceId")?,
        target_mode: parse_enum!(
            value,
            "targetMode",
            CliTargetVersionMode,
            [
                CliTargetVersionMode::Unsupported,
                CliTargetVersionMode::LatestOnly,
                CliTargetVersionMode::Exact,
            ]
        ),
        default_target: optional_text(value, "defaultTarget"),
        requires_target_selection: flag(value, "requiresTargetSelection")?,
        reason_code,
    })
}

fn decode_action_kind(value: &Value, key: &str) -> Decoded<CliActionKind> {
    Ok(parse_enum!(
        value,
        key,
        CliActionKind,
        [
            CliActionKind::Install,
            CliActionKind::Upgrade,
            CliActionKind::Downgrade,
            CliActionKind::Reinstall,
            CliActionKind::Uninstall,
            CliActionKind::Repair,
        ]
    ))
}

fn encode_mutation_summary(summary: &CliMutationSummary) -> Value {
    json!({
        "outcome": summary.outcome.as_str(),
        "sourceId": summary.source_id.as_str(),
        "action": summary.action,
        "targetVersion": summary.target_version,
        "operationId": summary.operation_id,
        "completedAt": summary.completed_at.to_rfc3339(),
    })
}

fn decode_mutation_summary(value: &Value) -> Decoded<CliMutationSummary> {
    Ok(CliMutationSummary {
        outcome: parse_enum!(
            value,
            "outcome",
            CliMutationOutcome,
            [
                CliMutationOutcome::Verified,
                CliMutationOutcome::AppliedUnverified,
                CliMutationOutcome::ChangedButFailed,
                CliMutationOutcome::NoChangeFailed,
                CliMutationOutcome::Cancelled,
            ]
        ),
        source_id: source_id_of(value, "sourceId")?,
        action: text(value, "action")?,
        target_version: optional_text(value, "targetVersion"),
        operation_id: text(value, "operationId")?,
        completed_at: timestamp(value, "completedAt")?,
    })
}

// ---------------------------------------------------------------------------------------------
// Catalog

pub(super) fn encode_catalog(catalog: &CliVersionCatalog) -> Value {
    let (status, reason) = match catalog.status {
        CliCatalogStatus::Available => ("available", None),
        CliCatalogStatus::Unavailable(reason) => ("unavailable", Some(reason.as_str())),
    };
    json!({
        "documentVersion": DOCUMENT_VERSION,
        "agentId": catalog.agent_id.as_str(),
        "sourceId": catalog.source_id.as_str(),
        "channel": catalog.channel,
        "versions": catalog.versions.iter().map(NormalizedCliVersion::as_str).collect::<Vec<_>>(),
        "latest": catalog.latest.as_ref().map(NormalizedCliVersion::as_str),
        "fetchedAt": catalog.fetched_at.to_rfc3339(),
        "expiresAt": catalog.expires_at.to_rfc3339(),
        "status": status,
        "unavailableReason": reason,
    })
}

pub(super) fn decode_catalog(value: Value) -> Decoded<CliVersionCatalog> {
    check_version(&value)?;
    let versions = array(&value, "versions")?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(NormalizedCliVersion::parse)
                .ok_or_else(|| "catalog version is not a string".to_string())
        })
        .collect::<Decoded<Vec<_>>>()?;

    let status = match text(&value, "status")?.as_str() {
        "available" => CliCatalogStatus::Available,
        "unavailable" => {
            let raw = optional_text(&value, "unavailableReason")
                .ok_or_else(|| "unavailable catalog has no reason".to_string())?;
            let reason = [
                CliCatalogUnavailableReason::SourceUnavailable,
                CliCatalogUnavailableReason::UnparseableOutput,
                CliCatalogUnavailableReason::QueryFailed,
                CliCatalogUnavailableReason::NotApplicable,
            ]
            .into_iter()
            .find(|candidate| candidate.as_str() == raw)
            .ok_or_else(|| format!("unknown catalog reason `{raw}`"))?;
            CliCatalogStatus::Unavailable(reason)
        }
        other => return Err(format!("unknown catalog status `{other}`")),
    };

    Ok(CliVersionCatalog {
        agent_id: tool_id(&value, "agentId")?,
        source_id: source_id_of(&value, "sourceId")?,
        channel: optional_text(&value, "channel"),
        versions,
        latest: optional_text(&value, "latest").map(NormalizedCliVersion::parse),
        fetched_at: timestamp(&value, "fetchedAt")?,
        expires_at: timestamp(&value, "expiresAt")?,
        status,
    })
}

// ---------------------------------------------------------------------------------------------
// Plans

pub(super) fn encode_plan(plan: &CliActionPlan) -> Value {
    json!({
        "documentVersion": DOCUMENT_VERSION,
        "id": plan.id.as_str(),
        "revision": plan.revision,
        "agentId": plan.agent_id.as_str(),
        "action": plan.action.as_str(),
        "sourceId": plan.source_id.as_str(),
        "installationId": plan.installation_id.as_ref().map(CliInstallationId::as_str),
        "currentVersion": plan.current_version,
        "targetVersion": plan.target_version,
        "channel": plan.channel,
        "commandPreview": {
            "program": plan.command_preview.program,
            "args": plan.command_preview.args,
        },
        "preconditions": plan.preconditions.iter().map(encode_precondition).collect::<Vec<_>>(),
        "warnings": plan.warnings.iter().map(|warning| warning.as_str()).collect::<Vec<_>>(),
        "requiresElevation": plan.requires_elevation,
        "requiresNetwork": plan.requires_network,
        "fallbackPolicy": plan.fallback_policy.as_str(),
        "environmentFingerprint": plan.environment_fingerprint,
        "state": plan.state.as_str(),
        "createdAt": plan.created_at.to_rfc3339(),
        "expiresAt": plan.expires_at.to_rfc3339(),
    })
}

fn encode_precondition(precondition: &CliPrecondition) -> Value {
    match precondition {
        CliPrecondition::SourceExecutableAvailable { source } => {
            json!({ "kind": "source-executable-available", "source": source })
        }
        CliPrecondition::NetworkReachable { host } => {
            json!({ "kind": "network-reachable", "host": host })
        }
        CliPrecondition::ElevatedPrivileges => json!({ "kind": "elevated-privileges" }),
    }
}

fn decode_precondition(value: &Value) -> Decoded<CliPrecondition> {
    match text(value, "kind")?.as_str() {
        "source-executable-available" => Ok(CliPrecondition::SourceExecutableAvailable {
            source: text(value, "source")?,
        }),
        "network-reachable" => Ok(CliPrecondition::NetworkReachable {
            host: text(value, "host")?,
        }),
        "elevated-privileges" => Ok(CliPrecondition::ElevatedPrivileges),
        other => Err(format!("unknown precondition `{other}`")),
    }
}

/// Decodes the `state` column, which outranks the state embedded in the plan document.
///
/// Maintenance sweeps move rows out of `draft` by touching the column alone, so a document read
/// without this reconciliation can still claim `draft` and be admitted a second time.
pub(super) fn decode_plan_state(raw: &str) -> Decoded<CliActionPlanState> {
    [
        CliActionPlanState::Draft,
        CliActionPlanState::Executing,
        CliActionPlanState::Completed,
        CliActionPlanState::Failed,
        CliActionPlanState::Cancelled,
        CliActionPlanState::Expired,
    ]
    .into_iter()
    .find(|candidate| candidate.as_str() == raw)
    .ok_or_else(|| format!("unknown plan state `{raw}`"))
}

pub(super) fn decode_plan(value: Value) -> Decoded<CliActionPlan> {
    check_version(&value)?;
    let preview = field(&value, "commandPreview")?;
    let args = array(preview, "args")?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| "command argument is not a string".to_string())
        })
        .collect::<Decoded<Vec<_>>>()?;

    let mut preconditions = Vec::new();
    for entry in array(&value, "preconditions")? {
        preconditions.push(decode_precondition(entry)?);
    }
    let mut warnings = Vec::new();
    for entry in array(&value, "warnings")? {
        let raw = entry
            .as_str()
            .ok_or_else(|| "warning is not a string".to_string())?;
        warnings.push(
            [
                CliPlanWarning::TargetIsLatestOnly,
                CliPlanWarning::InstallerIntegrityUnverified,
                CliPlanWarning::ExactVersionNotConfirmed,
                CliPlanWarning::ActiveInstallationShadowed,
                CliPlanWarning::DowngradeMayLoseState,
            ]
            .into_iter()
            .find(|candidate| candidate.as_str() == raw)
            .ok_or_else(|| format!("unknown plan warning `{raw}`"))?,
        );
    }

    // The only policy this build understands. A row claiming another one was written by something
    // that allows source fallback, and running it would be exactly the behaviour being removed.
    let policy = text(&value, "fallbackPolicy")?;
    if policy != CliFallbackPolicy::None.as_str() {
        return Err(format!("unknown fallback policy `{policy}`"));
    }

    Ok(CliActionPlan {
        id: CliActionPlanId::new(text(&value, "id")?).map_err(|error| error.to_string())?,
        revision: field(&value, "revision")?
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| "revision is not a u32".to_string())?,
        agent_id: tool_id(&value, "agentId")?,
        action: decode_action_kind(&value, "action")?,
        source_id: source_id_of(&value, "sourceId")?,
        installation_id: optional_id(&value, "installationId")?,
        current_version: optional_text(&value, "currentVersion"),
        target_version: optional_text(&value, "targetVersion"),
        channel: optional_text(&value, "channel"),
        command_preview: CliCommandPreview::new(text(preview, "program")?, args),
        preconditions,
        warnings,
        requires_elevation: flag(&value, "requiresElevation")?,
        requires_network: flag(&value, "requiresNetwork")?,
        fallback_policy: CliFallbackPolicy::None,
        environment_fingerprint: text(&value, "environmentFingerprint")?,
        state: parse_enum!(
            &value,
            "state",
            CliActionPlanState,
            [
                CliActionPlanState::Draft,
                CliActionPlanState::Executing,
                CliActionPlanState::Completed,
                CliActionPlanState::Failed,
                CliActionPlanState::Cancelled,
                CliActionPlanState::Expired,
            ]
        ),
        created_at: timestamp(&value, "createdAt")?,
        expires_at: timestamp(&value, "expiresAt")?,
    })
}

pub(super) fn encode_bulk_plan(bulk: &CliBulkActionPlan) -> Value {
    json!({
        "documentVersion": DOCUMENT_VERSION,
        "id": bulk.id.as_str(),
        "revision": bulk.revision,
        "items": bulk.items.iter().map(encode_bulk_item).collect::<Vec<_>>(),
        "skipped": bulk.skipped.iter().map(|skip| json!({
            "agentId": skip.agent_id.as_str(),
            "reason": skip.reason.as_str(),
        })).collect::<Vec<_>>(),
        "environmentFingerprint": bulk.environment_fingerprint,
        "createdAt": bulk.created_at.to_rfc3339(),
        "expiresAt": bulk.expires_at.to_rfc3339(),
    })
}

fn encode_bulk_item(item: &CliBulkActionItem) -> Value {
    json!({
        "agentId": item.agent_id.as_str(),
        "planId": item.plan_id.as_str(),
        "sourceId": item.source_id.as_str(),
        "currentVersion": item.current_version,
        "targetVersion": item.target_version,
        "requiresElevation": item.requires_elevation,
        "requiresNetwork": item.requires_network,
        "state": item.state.as_str(),
        "skippedReason": item.skipped_reason.map(|reason| reason.as_str()),
    })
}

const ALL_SKIP_REASONS: [CliBulkSkipReason; 12] = [
    CliBulkSkipReason::AlreadyCurrent,
    CliBulkSkipReason::DetectOnlySource,
    CliBulkSkipReason::CatalogUnavailable,
    CliBulkSkipReason::NeedsAuth,
    CliBulkSkipReason::Broken,
    CliBulkSkipReason::NotInstalled,
    CliBulkSkipReason::UnsupportedAction,
    CliBulkSkipReason::UnorderedVersions,
    CliBulkSkipReason::SourceOwnershipUnproven,
    CliBulkSkipReason::PlanStale,
    CliBulkSkipReason::OperationConflict,
    CliBulkSkipReason::InstallationConflict,
];

fn skip_reason(raw: &str) -> Decoded<CliBulkSkipReason> {
    ALL_SKIP_REASONS
        .into_iter()
        .find(|candidate| candidate.as_str() == raw)
        .ok_or_else(|| format!("unknown skip reason `{raw}`"))
}

pub(super) fn decode_bulk_plan(value: Value) -> Decoded<CliBulkActionPlan> {
    check_version(&value)?;
    let mut items = Vec::new();
    for entry in array(&value, "items")? {
        items.push(CliBulkActionItem {
            agent_id: tool_id(entry, "agentId")?,
            plan_id: CliActionPlanId::new(text(entry, "planId")?)
                .map_err(|error| error.to_string())?,
            source_id: source_id_of(entry, "sourceId")?,
            current_version: optional_text(entry, "currentVersion"),
            target_version: optional_text(entry, "targetVersion"),
            requires_elevation: flag(entry, "requiresElevation")?,
            requires_network: flag(entry, "requiresNetwork")?,
            state: parse_enum!(
                entry,
                "state",
                CliActionPlanState,
                [
                    CliActionPlanState::Draft,
                    CliActionPlanState::Executing,
                    CliActionPlanState::Completed,
                    CliActionPlanState::Failed,
                    CliActionPlanState::Cancelled,
                    CliActionPlanState::Expired,
                ]
            ),
            skipped_reason: match entry.get("skippedReason").and_then(Value::as_str) {
                None => None,
                Some(raw) => Some(skip_reason(raw)?),
            },
        });
    }

    let mut skipped = Vec::new();
    for entry in array(&value, "skipped")? {
        skipped.push(CliBulkSkip {
            agent_id: tool_id(entry, "agentId")?,
            reason: skip_reason(&text(entry, "reason")?)?,
        });
    }

    Ok(CliBulkActionPlan {
        id: CliBulkPlanId::new(text(&value, "id")?).map_err(|error| error.to_string())?,
        revision: field(&value, "revision")?
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| "revision is not a u32".to_string())?,
        items,
        skipped,
        environment_fingerprint: text(&value, "environmentFingerprint")?,
        created_at: timestamp(&value, "createdAt")?,
        expires_at: timestamp(&value, "expiresAt")?,
    })
}

/// Maps a legacy `cli_tool_status` row into a stale snapshot.
///
/// Only ever called when no authoritative snapshot exists. Everything the old row cannot establish
/// -- catalogs, update state, authentication, readiness -- is left unknown rather than guessed, and
/// freshness is `Stale` so nothing here is presented as freshly verified.
pub(super) fn legacy_row_to_stale_snapshot(
    agent_id: CliToolId,
    fingerprint: &str,
    detected_path: Option<String>,
    current_version: Option<String>,
    last_checked_at: Option<DateTime<Utc>>,
) -> CliEnvironmentSnapshot {
    let mut snapshot = CliEnvironmentSnapshot::never_scanned(agent_id, fingerprint.to_string());
    if let Some(path) = detected_path.filter(|path| !path.trim().is_empty()) {
        let id = CliInstallationId::new(format!("legacy-{}", path.len()))
            .unwrap_or_else(|_| CliInstallationId::trusted("legacy"));
        snapshot.installations = vec![CliInstallation {
            id: id.clone(),
            executable_path: path,
            canonical_path: None,
            alias_paths: Vec::new(),
            target_missing: false,
            reported_version: current_version.map(NormalizedCliVersion::parse),
            source_id: None,
            source_kind: CliSourceKind::Unknown,
            // The legacy row recorded a path, not proof of ownership.
            source_confidence: CliSourceConfidence::Unknown,
            path_priority: None,
            environment_origin: CliEnvironmentOrigin::KnownLocation,
            // Never re-probed, so its health is genuinely unknown.
            executable_status: CliExecutableStatus::Unknown,
        }];
        snapshot.discovery = CliDiscoveryStatus::FoundOne;
    }
    snapshot.checked_at = last_checked_at;
    snapshot.freshness = CliFreshness::Stale;
    snapshot.recompute_derived(false, false)
}

/// Round-trip helper for tests and for anything that needs the document as a map.
#[cfg(test)]
pub(super) fn as_object(value: &Value) -> &serde_json::Map<String, Value> {
    value.as_object().expect("document is an object")
}

#[cfg(test)]
#[path = "environment_serde_tests.rs"]
mod tests;
