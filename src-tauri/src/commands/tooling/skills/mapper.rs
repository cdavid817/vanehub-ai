use super::dto;
use crate::contexts::tooling::skills::api as skill;

pub(super) fn scope_query(
    input: dto::SkillScopeInput,
) -> Result<skill::SkillScopeQuery, skill::SkillError> {
    Ok(skill::SkillScopeQuery {
        location: location(input.scope, input.workspace_path.as_deref())?,
    })
}

pub(super) fn key(
    skill_id: String,
    input: dto::SkillScopeInput,
) -> Result<skill::SkillKey, skill::SkillError> {
    Ok(skill::SkillKey::new(
        skill::SkillId::parse(skill_id)?,
        location(input.scope, input.workspace_path.as_deref())?,
    ))
}

pub(super) fn mount_path(value: String) -> Result<skill::SkillMountPath, skill::SkillError> {
    skill::SkillMountPath::parse(value).map_err(Into::into)
}

pub(super) fn create_request(
    input: dto::SkillMutationInput,
) -> Result<skill::SkillCreateRequest, skill::SkillError> {
    Ok(skill::SkillCreateRequest {
        id: skill::SkillId::parse(input.id)?,
        location: location(input.scope, input.workspace_path.as_deref())?,
        metadata: metadata(input.metadata)?,
        body: input.body,
        enabled: input.enabled,
        bound_agent_ids: input.bound_agent_ids,
        source: input.source.map(source_to_domain),
    })
}

pub(super) fn update_request(
    skill_id: String,
    input: dto::SkillUpdateInput,
) -> Result<skill::SkillUpdateRequest, skill::SkillError> {
    Ok(skill::SkillUpdateRequest {
        key: skill::SkillKey::new(
            skill::SkillId::parse(skill_id)?,
            location(input.scope, input.workspace_path.as_deref())?,
        ),
        metadata: metadata(input.metadata)?,
        body: input.body,
        expected_content_hash: input.expected_content_hash,
    })
}

pub(super) fn import_request(
    input: dto::SkillImportInput,
) -> Result<skill::SkillImportRequest, skill::SkillError> {
    Ok(skill::SkillImportRequest {
        location: location(input.scope, input.workspace_path.as_deref())?,
        source_path: input.source_path,
        enabled: input.enabled,
        bound_agent_ids: input.bound_agent_ids,
    })
}

pub(super) fn list_to_dto(result: skill::SkillListResult) -> dto::SkillListResult {
    dto::SkillListResult {
        skills: result.skills.into_iter().map(record_to_dto).collect(),
        stats: dto::SkillStats {
            total: result.stats.total,
            enabled: result.stats.enabled,
            mounted: result.stats.mounted,
        },
    }
}

pub(super) fn record_to_dto(record: skill::SkillRecord) -> dto::Skill {
    let bound_agent_ids = record.bound_agent_ids();
    let effective = record.effective_metadata();
    dto::Skill {
        id: record.key.id.as_str().to_string(),
        scope: scope_to_dto(record.key.location.scope),
        workspace_path: record.key.location.workspace_path,
        source: source_to_dto(record.source),
        enabled: record.enabled,
        skill_dir: record.managed_source.skill_dir,
        skill_md_path: record.managed_source.skill_md_path,
        content_hash: record.managed_source.content_hash,
        metadata: metadata_to_dto(record.metadata),
        bound_agent_ids,
        bindings: record
            .bindings
            .into_iter()
            .map(|binding| dto::SkillAgentBinding {
                agent_id: binding.agent_id,
                mount_path: binding.mount_path.as_str().to_string(),
                mounted_path: binding.mounted_path,
                mounted: binding.mounted,
            })
            .collect(),
        created_at: record.created_at,
        updated_at: record.updated_at,
        layer: layer_to_dto(effective.layer),
        origin: origin_to_dto(effective.origin),
        trust: trust_to_dto(effective.trust),
        availability: availability_to_dto(effective.availability),
        delegation_capability: delegation_capability(effective.delegation.as_ref()),
        immutable: effective.immutable,
        shadowed_definitions: effective.shadowed.into_iter().map(shadow_to_dto).collect(),
        usage: dto::SkillUsageSummary {
            view_count: effective.usage.view_count,
            use_count: effective.usage.use_count,
            last_viewed_at: effective.usage.last_viewed_at,
            last_used_at: effective.usage.last_used_at,
            revision_witness: effective.usage.revision_witness,
        },
    }
}

/// `None` means the Skill is not a Utility. The default DTO keeps its historical
/// `not-utility` wire shape so Role Skills never look delegatable.
fn delegation_capability(
    summary: Option<&skill::SkillDelegationSummary>,
) -> dto::SkillDelegationCapability {
    let Some(summary) = summary else {
        return dto::SkillDelegationCapability::default();
    };
    dto::SkillDelegationCapability {
        supported: summary.supported,
        reason: if summary.supported {
            "available"
        } else {
            "skill-unavailable"
        }
        .to_string(),
        unavailable_reason: summary
            .unavailable_reason
            .map(|reason| reason.as_str().to_string()),
        declared_capabilities: capability_ids(&summary.declared_capabilities),
        effective_capabilities: capability_ids(&summary.effective_capabilities),
        requested_limits: dto::SkillDelegationRequestedLimits {
            max_rounds: summary.requested_limits.max_rounds,
            timeout_seconds: summary.requested_limits.timeout_seconds,
            max_context_chars: summary.requested_limits.max_context_chars,
            max_output_chars: summary.requested_limits.max_output_chars,
        },
        effective_limits: summary
            .effective_limits
            .map(|limits| dto::SkillDelegationLimits {
                max_rounds: limits.max_rounds,
                timeout_seconds: limits.timeout_seconds,
                max_context_chars: limits.max_context_chars,
                max_output_chars: limits.max_output_chars,
            }),
        capped_limits: summary
            .capped_limits
            .iter()
            .map(|field| field.as_str().to_string())
            .collect(),
        uses_platform_default: summary.uses_platform_default,
        read_only: summary.read_only,
        history: dto::SkillDelegationHistorySummary {
            attempt_count: summary.history.attempt_count,
            last_attempt_at: summary.history.last_attempt_at.clone(),
            last_status: summary.history.last_status.clone(),
        },
    }
}

fn capability_ids(capabilities: &[skill::SkillDelegationCapabilityId]) -> Vec<String> {
    capabilities
        .iter()
        .map(|capability| capability.as_str().to_string())
        .collect()
}

pub(super) fn mount_paths_to_dto(
    paths: Vec<skill::SkillAgentMountPath>,
) -> Vec<dto::SkillAgentMountPath> {
    paths
        .into_iter()
        .map(|path| dto::SkillAgentMountPath {
            agent_id: path.agent_id,
            mount_path: path.mount_path.as_str().to_string(),
            is_default: path.is_default,
        })
        .collect()
}

pub(super) fn mount_migration_to_dto(
    report: skill::SkillMountMigrationReport,
) -> dto::SkillMountMigrationReport {
    dto::SkillMountMigrationReport {
        agent_id: report.agent_id,
        old_mount_path: report.old_mount_path.as_str().to_string(),
        new_mount_path: report.new_mount_path.as_str().to_string(),
        migrated: report.migrated,
        removed: report.removed,
        overwritten: report.overwritten,
        backed_up: report.backed_up.into_iter().map(backup_to_dto).collect(),
        failed: report.failed.into_iter().map(failure_to_dto).collect(),
    }
}

pub(super) fn preview_to_dto(preview: skill::SkillPreview) -> dto::SkillPreview {
    let effective = preview.effective;
    dto::SkillPreview {
        id: preview.key.id.as_str().to_string(),
        scope: scope_to_dto(preview.key.location.scope),
        workspace_path: preview.key.location.workspace_path,
        content: preview.content,
        path: preview.path,
        layer: layer_to_dto(effective.layer),
        origin: origin_to_dto(effective.origin),
        availability: availability_to_dto(effective.availability),
        immutable: effective.immutable,
        shadowed_definitions: effective.shadowed.into_iter().map(shadow_to_dto).collect(),
    }
}

pub(super) fn load_request(input: dto::SkillLoadInput) -> skill::SkillLoadRequest {
    skill::SkillLoadRequest {
        id_or_alias: input.id_or_alias,
        workspace_path: input.workspace_path,
    }
}

pub(super) fn resource_read_request(
    input: dto::SkillResourceReadInput,
) -> skill::SkillResourceReadRequest {
    skill::SkillResourceReadRequest {
        uri: input.uri,
        revision: input.revision,
        workspace_path: input.workspace_path,
    }
}

pub(super) fn load_outcome_to_dto(outcome: skill::SkillLoadOutcome) -> dto::SkillLoadOutcome {
    match outcome {
        skill::SkillLoadOutcome::Loaded(result) => dto::SkillLoadOutcome::Loaded {
            result: dto::SkillLoadResult {
                id: result.id,
                name: result.name,
                content: result.content,
                truncated: result.truncated,
                revision: result.revision,
                base_uri: result.base_uri,
                resources: resource_index_to_dto(result.resources),
            },
        },
        skill::SkillLoadOutcome::Refused(refusal) => dto::SkillLoadOutcome::Refused {
            refusal: access_refusal_to_dto(refusal),
        },
    }
}

pub(super) fn resource_read_outcome_to_dto(
    outcome: skill::SkillResourceReadOutcome,
) -> dto::SkillResourceReadOutcome {
    match outcome {
        skill::SkillResourceReadOutcome::Read(result) => dto::SkillResourceReadOutcome::Read {
            result: dto::SkillResourceReadResult {
                id: result.id,
                uri: result.uri,
                revision: result.revision,
                content: result.content,
                size_bytes: result.size_bytes,
            },
        },
        skill::SkillResourceReadOutcome::Refused(refusal) => {
            dto::SkillResourceReadOutcome::Refused {
                refusal: access_refusal_to_dto(refusal),
            }
        }
    }
}

fn resource_index_to_dto(index: skill::SkillResourceIndex) -> dto::SkillResourceIndex {
    dto::SkillResourceIndex {
        scripts: index
            .scripts
            .into_iter()
            .map(resource_entry_to_dto)
            .collect(),
        references: index
            .references
            .into_iter()
            .map(resource_entry_to_dto)
            .collect(),
        templates: index
            .templates
            .into_iter()
            .map(resource_entry_to_dto)
            .collect(),
        assets: index
            .assets
            .into_iter()
            .map(resource_entry_to_dto)
            .collect(),
        truncated: index.truncated,
    }
}

fn resource_entry_to_dto(entry: skill::SkillResourceEntry) -> dto::SkillResourceEntry {
    dto::SkillResourceEntry {
        uri: entry.uri,
        relative_path: entry.relative_path,
        size_bytes: entry.size_bytes,
    }
}

fn access_refusal_to_dto(refusal: skill::SkillAccessRefusal) -> dto::SkillAccessRefusal {
    dto::SkillAccessRefusal {
        requested: refusal.requested,
        canonical_id: refusal.canonical_id,
        reason: refusal.reason.as_str().to_string(),
        conflicting_ids: refusal.conflicting_ids,
    }
}

pub(super) fn drift_to_dto(report: skill::SkillDriftReport) -> dto::SkillDriftReport {
    dto::SkillDriftReport {
        scope: scope_to_dto(report.location.scope),
        workspace_path: report.location.workspace_path,
        issues: report
            .issues
            .into_iter()
            .map(|issue| dto::SkillDriftIssue {
                skill_id: issue.skill_id,
                r#type: match issue.issue_type {
                    skill::SkillDriftIssueType::MissingSource => {
                        dto::SkillDriftIssueType::MissingSource
                    }
                    skill::SkillDriftIssueType::MetadataChanged => {
                        dto::SkillDriftIssueType::MetadataChanged
                    }
                    skill::SkillDriftIssueType::UnregisteredSource => {
                        dto::SkillDriftIssueType::UnregisteredSource
                    }
                    skill::SkillDriftIssueType::MissingMount => {
                        dto::SkillDriftIssueType::MissingMount
                    }
                    skill::SkillDriftIssueType::Conflict => dto::SkillDriftIssueType::Conflict,
                    skill::SkillDriftIssueType::DeletedBuiltin => {
                        dto::SkillDriftIssueType::DeletedBuiltin
                    }
                },
                agent_id: issue.agent_id,
                path: issue.path,
                message: issue.message.to_string(),
            })
            .collect(),
        drift_hash: report.drift_hash,
    }
}

pub(super) fn sync_to_dto(result: skill::SkillSyncResult) -> dto::SkillSyncResult {
    dto::SkillSyncResult {
        mounted: result.mounted,
        unmounted: result.unmounted,
        overwritten: result.overwritten,
        backed_up: result.backed_up.into_iter().map(backup_to_dto).collect(),
        restored: result.restored,
        failed: result.failed.into_iter().map(failure_to_dto).collect(),
        resolved_from: drift_to_dto(result.resolved_from),
    }
}

fn location(
    scope: dto::SkillScope,
    workspace_path: Option<&str>,
) -> Result<skill::SkillLocation, skill::SkillError> {
    let scope = scope_to_domain(scope);
    let canonical_workspace = if scope == skill::SkillScope::Workspace {
        let raw = workspace_path
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                skill::SkillError::Validation("Workspace path is required".to_string())
            })?;
        let canonical = std::path::Path::new(raw).canonicalize().map_err(|error| {
            skill::SkillError::Validation(format!(
                "Workspace path must identify an existing directory: {error}"
            ))
        })?;
        if !canonical.is_dir() {
            return Err(skill::SkillError::Validation(
                "Workspace path must identify a directory".to_string(),
            ));
        }
        let value = canonical.to_string_lossy();
        Some(value.strip_prefix(r"\\?\").unwrap_or(&value).to_string())
    } else {
        None
    };
    skill::SkillLocation::new(scope, canonical_workspace.as_deref()).map_err(Into::into)
}

pub(super) fn overview_to_dto(result: skill::SkillOverview) -> dto::SkillOverview {
    dto::SkillOverview {
        skills: result.skills.into_iter().map(record_to_dto).collect(),
        stats: dto::SkillStats {
            total: result.stats.total,
            enabled: result.stats.enabled,
            mounted: result.stats.mounted,
        },
        mount_paths: mount_paths_to_dto(result.mount_paths),
        agents: result
            .agents
            .into_iter()
            .map(|agent| dto::SkillCompatibleAgent {
                id: agent.id,
                display_name: agent.display_name,
                kind: match agent.kind {
                    skill::SkillAgentKind::Cli => dto::SkillAgentKind::Cli,
                    skill::SkillAgentKind::Api => dto::SkillAgentKind::Api,
                },
                supports_utility_delegation: agent.delegation_runtime.supports_delegation(),
            })
            .collect(),
        api_agent_bindings: result.api_agent_bindings,
        drift: drift_to_dto(result.drift),
        restore_candidates: result.restore_candidates,
    }
}

fn metadata(value: dto::SkillMetadata) -> Result<skill::SkillMetadata, skill::SkillError> {
    skill::SkillMetadata::with_classification(
        value.id,
        value.name,
        value.description,
        value.category,
        value.version,
        value.triggers,
        value.aliases,
        value.skill_type.map(skill_type_to_domain),
        value.delivery.map(delivery_to_domain),
    )
    .map_err(Into::into)
}

fn metadata_to_dto(value: skill::SkillMetadata) -> dto::SkillMetadata {
    let compatibility_defaults = dto::SkillCompatibilityDefaults {
        skill_type: value.compatibility_defaults.skill_type,
        delivery: value.compatibility_defaults.delivery,
    };
    dto::SkillMetadata {
        id: value.id.as_str().to_string(),
        name: value.name,
        description: value.description,
        category: value.category,
        version: value.version,
        triggers: value.triggers,
        aliases: value
            .aliases
            .into_iter()
            .map(|alias| alias.as_str().to_string())
            .collect(),
        skill_type: Some(skill_type_to_dto(value.skill_type)),
        delivery: Some(delivery_to_dto(value.delivery)),
        compatibility_defaults,
    }
}

fn skill_type_to_domain(value: dto::SkillType) -> skill::SkillType {
    match value {
        dto::SkillType::Role => skill::SkillType::Role,
        dto::SkillType::Utility => skill::SkillType::Utility,
    }
}

fn skill_type_to_dto(value: skill::SkillType) -> dto::SkillType {
    match value {
        skill::SkillType::Role => dto::SkillType::Role,
        skill::SkillType::Utility => dto::SkillType::Utility,
    }
}

fn delivery_to_domain(value: dto::SkillDelivery) -> skill::SkillDelivery {
    match value {
        dto::SkillDelivery::Eager => skill::SkillDelivery::Eager,
        dto::SkillDelivery::OnDemand => skill::SkillDelivery::OnDemand,
    }
}

fn delivery_to_dto(value: skill::SkillDelivery) -> dto::SkillDelivery {
    match value {
        skill::SkillDelivery::Eager => dto::SkillDelivery::Eager,
        skill::SkillDelivery::OnDemand => dto::SkillDelivery::OnDemand,
    }
}

fn layer_to_dto(value: skill::SkillLayer) -> dto::SkillLayer {
    match value {
        skill::SkillLayer::Project => dto::SkillLayer::Project,
        skill::SkillLayer::User => dto::SkillLayer::User,
        skill::SkillLayer::Registry => dto::SkillLayer::Registry,
        skill::SkillLayer::System => dto::SkillLayer::System,
    }
}

fn origin_to_dto(value: skill::SkillOrigin) -> dto::SkillOrigin {
    match value {
        skill::SkillOrigin::Created => dto::SkillOrigin::Created,
        skill::SkillOrigin::Imported => dto::SkillOrigin::Imported,
        skill::SkillOrigin::Installed => dto::SkillOrigin::Installed,
        skill::SkillOrigin::Shipped => dto::SkillOrigin::Shipped,
        skill::SkillOrigin::Migrated => dto::SkillOrigin::Migrated,
    }
}

fn trust_to_dto(value: skill::SkillTrust) -> dto::SkillTrust {
    match value {
        skill::SkillTrust::Trusted => dto::SkillTrust::Trusted,
        skill::SkillTrust::Untrusted => dto::SkillTrust::Untrusted,
    }
}

fn availability_to_dto(value: skill::SkillAvailability) -> dto::SkillAvailability {
    match value {
        skill::SkillAvailability::Available => dto::SkillAvailability::Available,
        skill::SkillAvailability::Disabled => dto::SkillAvailability::Disabled,
        skill::SkillAvailability::Invalid => dto::SkillAvailability::Invalid,
        skill::SkillAvailability::Conflicting => dto::SkillAvailability::Conflicting,
        skill::SkillAvailability::Unsupported => dto::SkillAvailability::Unsupported,
    }
}

fn shadow_to_dto(value: skill::SkillShadowSummary) -> dto::SkillShadowSummary {
    dto::SkillShadowSummary {
        layer: layer_to_dto(value.layer),
        origin: origin_to_dto(value.origin),
        version: value.version,
        availability: availability_to_dto(value.availability),
    }
}

fn scope_to_domain(scope: dto::SkillScope) -> skill::SkillScope {
    match scope {
        dto::SkillScope::Global => skill::SkillScope::Global,
        dto::SkillScope::Workspace => skill::SkillScope::Workspace,
    }
}

fn scope_to_dto(scope: skill::SkillScope) -> dto::SkillScope {
    match scope {
        skill::SkillScope::Global => dto::SkillScope::Global,
        skill::SkillScope::Workspace => dto::SkillScope::Workspace,
    }
}

fn source_to_domain(source: dto::SkillSource) -> skill::SkillSource {
    match source {
        dto::SkillSource::Builtin => skill::SkillSource::Builtin,
        dto::SkillSource::User => skill::SkillSource::User,
        dto::SkillSource::Imported => skill::SkillSource::Imported,
    }
}

fn source_to_dto(source: skill::SkillSource) -> dto::SkillSource {
    match source {
        skill::SkillSource::Builtin => dto::SkillSource::Builtin,
        skill::SkillSource::User => dto::SkillSource::User,
        skill::SkillSource::Imported => dto::SkillSource::Imported,
    }
}

fn backup_to_dto(backup: skill::SkillBackupEntry) -> dto::SkillBackupEntry {
    dto::SkillBackupEntry {
        original_path: backup.original_path,
        backup_path: backup.backup_path,
    }
}

fn failure_to_dto(failure: skill::SkillFailure) -> dto::SkillFailure {
    dto::SkillFailure {
        skill_id: failure.skill_id,
        reason: failure.reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utility_delegation_capability_projects_the_effective_summary() {
        let available = skill::SkillDelegationSummary::evaluate(
            skill::SkillType::Utility,
            skill::SkillTrust::Trusted,
            skill::SkillAvailability::Unsupported,
            &Default::default(),
        );
        let projected = delegation_capability(available.as_ref());
        assert!(projected.supported);
        assert_eq!(projected.reason, "available");
        assert_eq!(
            projected.declared_capabilities,
            vec![
                "file-read".to_string(),
                "content-search".to_string(),
                "filename-search".to_string(),
                "skill-read".to_string()
            ]
        );
        assert!(projected.read_only);
        assert!(projected.uses_platform_default);
        assert_eq!(
            projected.effective_limits.map(|limits| limits.max_rounds),
            Some(8)
        );

        let untrusted = skill::SkillDelegationSummary::evaluate(
            skill::SkillType::Utility,
            skill::SkillTrust::Untrusted,
            skill::SkillAvailability::Available,
            &Default::default(),
        );
        let projected = delegation_capability(untrusted.as_ref());
        assert!(!projected.supported);
        // The coarse `reason` stays inside its existing frontend union while the specific,
        // repairable cause travels in `unavailableReason`.
        assert_eq!(projected.reason, "skill-unavailable");
        assert_eq!(projected.unavailable_reason.as_deref(), Some("untrusted"));

        assert_eq!(
            delegation_capability(None),
            dto::SkillDelegationCapability::default()
        );
    }
    use crate::contexts::tooling::skills::application::{
        ManagedSkillSource, SkillAccessRefusalReason, SkillAgentBinding,
    };
    use crate::contexts::tooling::skills::domain::SkillDriftIssue;
    use serde_json::json;

    #[test]
    fn mutation_request_accepts_the_existing_camel_case_contract() {
        let workspace = std::env::current_dir()
            .expect("current directory")
            .canonicalize()
            .expect("canonical current directory");
        let workspace_value = workspace.to_string_lossy();
        let expected_workspace = workspace_value
            .strip_prefix(r"\\?\")
            .unwrap_or(&workspace_value)
            .to_string();
        let input: dto::SkillMutationInput = serde_json::from_value(json!({
            "id": "fixture-skill",
            "scope": "workspace",
            "workspacePath": expected_workspace.clone(),
            "metadata": {
                "id": "fixture-skill",
                "name": "Fixture",
                "description": "Description",
                "category": "testing",
                "version": "1.0.0",
                "triggers": ["fixture"]
            },
            "body": "Body",
            "enabled": true,
            "boundAgentIds": ["codex-cli"],
            "source": "user"
        }))
        .expect("mutation DTO");

        let request = create_request(input).expect("create request");

        assert_eq!(request.id.as_str(), "fixture-skill");
        assert_eq!(request.location.scope, skill::SkillScope::Workspace);
        assert_eq!(
            request.location.workspace_path.as_deref(),
            Some(expected_workspace.as_str())
        );
        assert_eq!(request.bound_agent_ids, vec!["codex-cli"]);
    }

    #[test]
    fn drift_response_keeps_kebab_case_type_and_camel_case_fields() {
        let report = drift_to_dto(skill::SkillDriftReport {
            location: skill::SkillLocation::new(skill::SkillScope::Global, None).expect("location"),
            issues: vec![SkillDriftIssue {
                skill_id: "fixture-skill".to_string(),
                issue_type: skill::SkillDriftIssueType::MissingMount,
                agent_id: Some("codex-cli".to_string()),
                path: Some(".codex/skills/fixture-skill".to_string()),
                message: "Agent mount is missing",
            }],
            drift_hash: "fixture-hash".to_string(),
        });

        let value = serde_json::to_value(report).expect("drift DTO");

        assert_eq!(value["issues"][0]["type"], "missing-mount");
        assert_eq!(value["issues"][0]["agentId"], "codex-cli");
        assert_eq!(value["driftHash"], "fixture-hash");
        assert!(value.get("drift_hash").is_none());
    }

    #[test]
    fn skill_response_preserves_the_complete_frontend_contract() {
        let location =
            skill::SkillLocation::new(skill::SkillScope::Global, None).expect("location");
        let record = skill::SkillRecord {
            key: skill::SkillKey::new(
                skill::SkillId::parse("fixture-skill").expect("Skill id"),
                location,
            ),
            source: skill::SkillSource::User,
            enabled: true,
            managed_source: ManagedSkillSource {
                skill_dir: "D:/home/.vanehub/skills/fixture-skill".to_string(),
                skill_md_path: "D:/home/.vanehub/skills/fixture-skill/SKILL.md".to_string(),
                content_hash: "fixture-hash".to_string(),
            },
            metadata: skill::SkillMetadata::new(
                "fixture-skill",
                "Fixture",
                "Description",
                "testing",
                "1.0.0",
                vec!["fixture".to_string()],
            )
            .expect("metadata"),
            bindings: vec![SkillAgentBinding {
                agent_id: "codex-cli".to_string(),
                mount_path: skill::SkillMountPath::parse(".codex/skills").expect("mount path"),
                mounted_path: "D:/home/.codex/skills/fixture-skill".to_string(),
                mounted: true,
            }],
            created_at: "2026-07-17T00:00:00Z".to_string(),
            updated_at: "2026-07-18T00:00:00Z".to_string(),
            resolved_metadata: None,
        };

        let value = serde_json::to_value(record_to_dto(record)).expect("Skill DTO");

        assert_eq!(
            value,
            json!({
                "id": "fixture-skill",
                "scope": "global",
                "workspacePath": null,
                "source": "user",
                "enabled": true,
                "skillDir": "D:/home/.vanehub/skills/fixture-skill",
                "skillMdPath": "D:/home/.vanehub/skills/fixture-skill/SKILL.md",
                "contentHash": "fixture-hash",
                "metadata": {
                    "id": "fixture-skill",
                    "name": "Fixture",
                    "description": "Description",
                    "category": "testing",
                    "version": "1.0.0",
                    "triggers": ["fixture"],
                    "aliases": [],
                    "type": "role",
                    "delivery": "eager",
                    "compatibilityDefaults": {
                        "skillType": true,
                        "delivery": true
                    }
                },
                "boundAgentIds": ["codex-cli"],
                "bindings": [{
                    "agentId": "codex-cli",
                    "mountPath": ".codex/skills",
                    "mountedPath": "D:/home/.codex/skills/fixture-skill",
                    "mounted": true
                }],
                "createdAt": "2026-07-17T00:00:00Z",
                "updatedAt": "2026-07-18T00:00:00Z",
                "layer": "user",
                "origin": "created",
                "trust": "trusted",
                "availability": "available",
                "delegationCapability": {
                    "supported": false,
                    "reason": "not-utility",
                    "unavailableReason": null,
                    "declaredCapabilities": [],
                    "effectiveCapabilities": [],
                    "requestedLimits": {
                        "maxRounds": null,
                        "timeoutSeconds": null,
                        "maxContextChars": null,
                        "maxOutputChars": null
                    },
                    "effectiveLimits": null,
                    "cappedLimits": [],
                    "usesPlatformDefault": false,
                    "readOnly": false,
                    "history": {
                        "attemptCount": 0,
                        "lastAttemptAt": null,
                        "lastStatus": null
                    }
                },
                "immutable": false,
                "shadowedDefinitions": [],
                "usage": {
                    "viewCount": 0,
                    "useCount": 0,
                    "lastViewedAt": null,
                    "lastUsedAt": null,
                    "revisionWitness": null
                }
            })
        );
    }

    #[test]
    fn progressive_outcomes_use_the_shared_frontend_discriminants() {
        let load = load_outcome_to_dto(skill::SkillLoadOutcome::Refused(
            skill::SkillAccessRefusal {
                requested: "utility-skill".to_string(),
                canonical_id: Some("utility-skill".to_string()),
                reason: SkillAccessRefusalReason::UtilityNotLoadable,
                conflicting_ids: Vec::new(),
            },
        ));
        let read = resource_read_outcome_to_dto(skill::SkillResourceReadOutcome::Read(
            crate::contexts::tooling::skills::application::SkillResourceReadResult {
                id: "role-skill".to_string(),
                uri: "skill://role-skill/references/guide.md".to_string(),
                revision: "revision-1".to_string(),
                content: "guide".to_string(),
                size_bytes: 5,
            },
        ));

        assert_eq!(
            serde_json::to_value(load).expect("load DTO"),
            json!({
                "status": "refused",
                "refusal": {
                    "requested": "utility-skill",
                    "canonicalId": "utility-skill",
                    "reason": "utility-not-loadable",
                    "conflictingIds": []
                }
            })
        );
        assert_eq!(
            serde_json::to_value(read).expect("read DTO"),
            json!({
                "status": "read",
                "result": {
                    "id": "role-skill",
                    "uri": "skill://role-skill/references/guide.md",
                    "revision": "revision-1",
                    "content": "guide",
                    "sizeBytes": 5
                }
            })
        );
    }
}
