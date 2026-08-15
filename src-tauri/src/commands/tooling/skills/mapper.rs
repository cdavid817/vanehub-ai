use super::dto;
use crate::contexts::tooling::skills::api as skill;
use crate::platform::logging::redact_text;

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
        delegation_capability: delegation_capability(
            effective.skill_type,
            effective.trust,
            effective.availability,
        ),
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

fn delegation_capability(
    skill_type: skill::SkillType,
    trust: skill::SkillTrust,
    availability: skill::SkillAvailability,
) -> dto::SkillDelegationCapability {
    if skill_type != skill::SkillType::Utility {
        return dto::SkillDelegationCapability::default();
    }
    let supported = trust == skill::SkillTrust::Trusted
        && matches!(
            availability,
            skill::SkillAvailability::Available | skill::SkillAvailability::Unsupported
        );
    dto::SkillDelegationCapability {
        supported,
        reason: if supported {
            "available"
        } else {
            "skill-unavailable"
        }
        .to_string(),
    }
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

pub(super) fn configuration_view_to_dto(
    skill_id: String,
    view: skill::SkillConfigurationView,
) -> dto::SkillConfigurationView {
    let (state, unsupported_reason) = match &view.overview.state {
        skill::SkillConfigurableState::NotEvaluated => {
            (dto::SkillConfigurableState::NotEvaluated, None)
        }
        skill::SkillConfigurableState::NotConfigurable => {
            (dto::SkillConfigurableState::NotConfigurable, None)
        }
        skill::SkillConfigurableState::Configurable { .. } => {
            (dto::SkillConfigurableState::Configurable, None)
        }
        skill::SkillConfigurableState::SchemaUnsupported { reason } => (
            dto::SkillConfigurableState::SchemaUnsupported,
            Some(reason.clone()),
        ),
    };
    // The resolved state accounts for stored values, so it supersedes the schema-only readiness
    // the overview reports before any record is read.
    let readiness = view
        .resolved
        .as_ref()
        .map(|resolved| resolved.readiness)
        .unwrap_or(view.overview.readiness);
    let drift = view
        .resolved
        .as_ref()
        .map(|resolved| resolved.drift)
        .or(view.overview.drift);
    dto::SkillConfigurationView {
        skill_id,
        state,
        unsupported_reason,
        schema_hash: view
            .schema
            .as_ref()
            .map(|schema| schema.hash.clone())
            .or_else(|| view.overview.state.schema_hash().map(str::to_string)),
        base_revision: view.overview.base_revision,
        workspace_identity: view.workspace_identity,
        available_scopes: view
            .overview
            .available_scopes
            .into_iter()
            .map(config_scope_to_dto)
            .collect(),
        descriptor: view.schema.map(descriptor_to_dto),
        resolution: view.resolved.map(resolution_to_dto),
        readiness: readiness_to_dto(readiness),
        drift: drift.map(config_drift_to_dto),
        runtime: dto::SkillConfigurationRuntimeSupport {
            native_api: consumption_to_dto(skill::consumption_for_binding(true)),
            external_cli: consumption_to_dto(skill::consumption_for_binding(false)),
        },
    }
}

pub(super) fn configuration_request(
    skill_id: String,
    input: dto::SkillConfigurationWriteInput,
) -> Result<(skill::SkillKey, skill::SkillConfigurationRequest), skill::SkillError> {
    let key = skill::SkillKey::new(
        skill::SkillId::parse(skill_id)?,
        location(input.scope, input.workspace_path.as_deref())?,
    );
    let scope = config_scope_to_domain(input.config_scope);
    // A User record is workspace-independent, so it is keyed by the empty identity even when the
    // editor is open inside a workspace.
    let workspace_identity = match scope {
        skill::SkillConfigScope::User => String::new(),
        skill::SkillConfigScope::Project => key.location.workspace_path.clone().unwrap_or_default(),
    };
    let request = skill::SkillConfigurationRequest {
        skill_id: key.id.as_str().to_string(),
        scope,
        workspace_identity,
        schema_hash: input.schema_hash,
        base_revision: input.base_revision,
        expected_revision: input.expected_revision.map(skill::SkillConfigRevision::new),
        values: input
            .values
            .into_iter()
            .map(|entry| (entry.key, value_to_domain(entry.value)))
            .collect(),
        secret_intents: input
            .secret_intents
            .into_iter()
            .map(|entry| (entry.key, secret_intent_to_domain(entry.intent)))
            .collect(),
    };
    Ok((key, request))
}

pub(super) fn configuration_scope_target(
    skill_id: String,
    input: dto::SkillConfigurationScopeInput,
) -> Result<(skill::SkillKey, skill::SkillConfigScope), skill::SkillError> {
    Ok((
        skill::SkillKey::new(
            skill::SkillId::parse(skill_id)?,
            location(input.scope, input.workspace_path.as_deref())?,
        ),
        config_scope_to_domain(input.config_scope),
    ))
}

pub(super) fn reconciliation_plan(
    plan: dto::SkillConfigurationReconciliationPlan,
) -> skill::ReconciliationPlan {
    skill::ReconciliationPlan {
        fields: plan
            .fields
            .into_iter()
            .map(|choice| {
                (
                    choice.key,
                    match choice.choice {
                        dto::SkillConfigurationObsoleteAction::MapTo { target_key } => {
                            skill::ObsoleteFieldChoice::MapTo(target_key)
                        }
                        dto::SkillConfigurationObsoleteAction::Discard => {
                            skill::ObsoleteFieldChoice::Discard
                        }
                    },
                )
            })
            .collect(),
        clear_secrets: plan.clear_secrets,
    }
}

pub(super) fn retention_to_domain(
    retention: dto::SkillConfigurationRetention,
) -> skill::DeletionRetention {
    match retention {
        dto::SkillConfigurationRetention::Retain => skill::DeletionRetention::Retain,
        dto::SkillConfigurationRetention::Delete => skill::DeletionRetention::Delete,
    }
}

pub(super) fn save_outcome_to_dto(
    outcome: Result<skill::SkillConfigurationSaveResult, skill::SkillConfigurationError>,
) -> dto::SkillConfigurationSaveOutcome {
    match outcome {
        Ok(result) => dto::SkillConfigurationSaveOutcome::Saved {
            result: Box::new(dto::SkillConfigurationSaveResult {
                record: record_state_to_dto(result.record),
                preview: resolution_to_dto(result.preview),
                recovery: recovery_to_dto(result.recovery),
            }),
        },
        Err(error) => dto::SkillConfigurationSaveOutcome::Rejected {
            rejection: Box::new(rejection_to_dto(error)),
        },
    }
}

pub(super) fn resolution_outcome_to_dto(
    outcome: Result<skill::ResolvedSkillConfiguration, skill::SkillConfigurationError>,
) -> dto::SkillConfigurationResolutionOutcome {
    match outcome {
        Ok(resolution) => dto::SkillConfigurationResolutionOutcome::Resolved {
            resolution: resolution_to_dto(resolution),
        },
        Err(error) => dto::SkillConfigurationResolutionOutcome::Rejected {
            rejection: Box::new(rejection_to_dto(error)),
        },
    }
}

pub(super) fn retention_outcome_to_dto(
    outcome: Result<skill::SecretRecovery, skill::SkillConfigurationError>,
) -> dto::SkillConfigurationRetentionOutcome {
    match outcome {
        Ok(recovery) => dto::SkillConfigurationRetentionOutcome::Applied {
            recovery: recovery_to_dto(recovery),
        },
        Err(error) => dto::SkillConfigurationRetentionOutcome::Rejected {
            rejection: Box::new(rejection_to_dto(error)),
        },
    }
}

fn descriptor_to_dto(schema: skill::SkillConfigSchema) -> dto::SkillConfigurationDescriptor {
    dto::SkillConfigurationDescriptor {
        schema_hash: schema.hash,
        groups: schema.groups.into_iter().map(group_to_dto).collect(),
        fields: schema.fields.into_iter().map(field_to_dto).collect(),
    }
}

fn group_to_dto(group: skill::SkillConfigGroup) -> dto::SkillConfigurationGroupDescriptor {
    dto::SkillConfigurationGroupDescriptor {
        key: group.key,
        declared_key: group.declared_key,
        presentation: presentation_to_dto(group.presentation),
    }
}

fn field_to_dto(field: skill::SkillConfigField) -> dto::SkillConfigurationFieldDescriptor {
    dto::SkillConfigurationFieldDescriptor {
        key: field.key,
        declared_key: field.declared_key,
        group: field.group,
        field_type: field_type_to_dto(field.field_type),
        required: field.required,
        secret: field.secret,
        default_value: field.default.map(value_to_dto),
        choices: field.choices.into_iter().map(value_to_dto).collect(),
        constraints: dto::SkillConfigurationConstraints {
            minimum: field.constraints.minimum,
            maximum: field.constraints.maximum,
            min_length: field.constraints.min_length,
            max_length: field.constraints.max_length,
            min_items: field.constraints.min_items,
            max_items: field.constraints.max_items,
        },
        presentation: presentation_to_dto(field.presentation),
        description: field.description,
    }
}

fn presentation_to_dto(
    presentation: skill::SkillConfigPresentation,
) -> dto::SkillConfigurationPresentation {
    dto::SkillConfigurationPresentation {
        label: presentation.label,
        label_key: presentation.label_key,
        help: presentation.help,
        order: presentation.order,
        advanced: presentation.advanced,
        multiline: presentation.multiline,
    }
}

fn field_type_to_dto(field_type: skill::SkillConfigFieldType) -> dto::SkillConfigurationFieldType {
    let kind = match field_type {
        skill::SkillConfigFieldType::Scalar(_) => dto::SkillConfigurationFieldKind::Scalar,
        skill::SkillConfigFieldType::List(_) => dto::SkillConfigurationFieldKind::List,
    };
    dto::SkillConfigurationFieldType {
        kind,
        scalar: match field_type.item_type() {
            skill::SkillConfigScalarType::Text => dto::SkillConfigurationScalarType::String,
            skill::SkillConfigScalarType::Integer => dto::SkillConfigurationScalarType::Integer,
            skill::SkillConfigScalarType::Number => dto::SkillConfigurationScalarType::Number,
            skill::SkillConfigScalarType::Boolean => dto::SkillConfigurationScalarType::Boolean,
        },
    }
}

fn value_to_dto(value: skill::SkillConfigValue) -> dto::SkillConfigurationValue {
    match value {
        skill::SkillConfigValue::Text(text) => dto::SkillConfigurationValue::String(text),
        skill::SkillConfigValue::Integer(number) => dto::SkillConfigurationValue::Integer(number),
        skill::SkillConfigValue::Number(number) => dto::SkillConfigurationValue::Number(number),
        skill::SkillConfigValue::Boolean(flag) => dto::SkillConfigurationValue::Boolean(flag),
        skill::SkillConfigValue::List(items) => {
            dto::SkillConfigurationValue::List(items.into_iter().map(value_to_dto).collect())
        }
    }
}

fn value_to_domain(value: dto::SkillConfigurationValue) -> skill::SkillConfigValue {
    match value {
        dto::SkillConfigurationValue::String(text) => skill::SkillConfigValue::Text(text),
        dto::SkillConfigurationValue::Integer(number) => skill::SkillConfigValue::Integer(number),
        dto::SkillConfigurationValue::Number(number) => skill::SkillConfigValue::Number(number),
        dto::SkillConfigurationValue::Boolean(flag) => skill::SkillConfigValue::Boolean(flag),
        dto::SkillConfigurationValue::List(items) => {
            skill::SkillConfigValue::List(items.into_iter().map(value_to_domain).collect())
        }
    }
}

fn secret_intent_to_domain(
    intent: dto::SkillConfigurationSecretIntent,
) -> skill::SkillSecretIntent {
    match intent {
        dto::SkillConfigurationSecretIntent::Preserve => skill::SkillSecretIntent::Preserve,
        dto::SkillConfigurationSecretIntent::Replace { value } => {
            skill::SkillSecretIntent::Replace(value)
        }
        dto::SkillConfigurationSecretIntent::Clear => skill::SkillSecretIntent::Clear,
    }
}

fn resolution_to_dto(
    resolution: skill::ResolvedSkillConfiguration,
) -> dto::SkillConfigurationResolution {
    dto::SkillConfigurationResolution {
        properties: resolution
            .properties
            .into_iter()
            .map(property_to_dto)
            .collect(),
        drift: config_drift_to_dto(resolution.drift),
        readiness: readiness_to_dto(resolution.readiness),
        scopes: resolution
            .scopes
            .into_iter()
            .map(scope_state_to_dto)
            .collect(),
    }
}

fn property_to_dto(property: skill::SkillConfigProperty) -> dto::SkillConfigurationProperty {
    dto::SkillConfigurationProperty {
        key: property.key,
        // A secret resolves without a value by construction; this only makes that explicit at the
        // boundary that would otherwise be the one to leak it.
        value: property
            .value
            .filter(|_| !property.secret)
            .map(value_to_dto),
        provenance: match property.provenance {
            skill::SkillConfigProvenance::Project => dto::SkillConfigurationProvenance::Project,
            skill::SkillConfigProvenance::User => dto::SkillConfigurationProvenance::User,
            skill::SkillConfigProvenance::SchemaDefault => {
                dto::SkillConfigurationProvenance::Default
            }
            skill::SkillConfigProvenance::Missing => dto::SkillConfigurationProvenance::Missing,
        },
        secret: property.secret,
        secret_state: property.secret_state.map(secret_state_to_dto),
    }
}

fn secret_state_to_dto(state: skill::SkillSecretState) -> dto::SkillConfigurationSecretState {
    match state {
        skill::SkillSecretState::Configured => dto::SkillConfigurationSecretState::Configured,
        skill::SkillSecretState::Missing => dto::SkillConfigurationSecretState::Missing,
        skill::SkillSecretState::Error => dto::SkillConfigurationSecretState::Error,
    }
}

fn scope_state_to_dto(state: skill::ScopeConfigurationState) -> dto::SkillConfigurationScopeState {
    dto::SkillConfigurationScopeState {
        scope: config_scope_to_dto(state.scope),
        stored_revision: state.stored_revision.value(),
        schema_hash: state.schema_hash,
        drift: config_drift_to_dto(state.drift),
        configured_property_count: state.configured_property_count,
        configured_secret_count: state.configured_secret_count,
    }
}

fn record_state_to_dto(
    record: skill::StoredSkillConfiguration,
) -> dto::SkillConfigurationRecordState {
    dto::SkillConfigurationRecordState {
        scope: config_scope_to_dto(record.scope),
        workspace_identity: (!record.workspace_identity.is_empty())
            .then_some(record.workspace_identity),
        schema_hash: record.schema_hash,
        base_revision: record.base_revision,
        stored_revision: record.stored_revision.value(),
        validation_state: config_drift_to_dto(record.validation_state),
        values: record
            .values
            .into_iter()
            .map(|(key, value)| dto::SkillConfigurationValueEntry {
                key,
                value: value_to_dto(value),
            })
            .collect(),
        secret_keys: record.secret_keys,
        orphaned_at: record.orphaned_at,
        cleanup_state: match record.cleanup_state {
            skill::SkillConfigCleanupState::None => dto::SkillConfigurationCleanupState::None,
            skill::SkillConfigCleanupState::Pending => dto::SkillConfigurationCleanupState::Pending,
            skill::SkillConfigCleanupState::Failed => dto::SkillConfigurationCleanupState::Failed,
        },
    }
}

fn recovery_to_dto(recovery: skill::SecretRecovery) -> dto::SkillConfigurationRecovery {
    match recovery {
        skill::SecretRecovery::Clean => dto::SkillConfigurationRecovery::Clean,
        skill::SecretRecovery::Incomplete { properties } => {
            dto::SkillConfigurationRecovery::Incomplete { properties }
        }
    }
}

/// Every branch reports the property that failed and the witnesses needed to recover. None of them
/// carries a secret value or a credential alias, and the repository message is redacted because it
/// is the one built from a lower-level error that can quote a path.
fn rejection_to_dto(error: skill::SkillConfigurationError) -> dto::SkillConfigurationRejection {
    let rejection = |code: dto::SkillConfigurationRejectionCode, message: String| {
        dto::SkillConfigurationRejection {
            code,
            message,
            property_key: None,
            properties: Vec::new(),
            expected: None,
            actual: None,
            bytes: None,
            current: None,
        }
    };
    match error {
        skill::SkillConfigurationError::UnknownProperty { key } => {
            let mut value = rejection(
                dto::SkillConfigurationRejectionCode::UnknownProperty,
                format!("Property is not declared by the effective schema: {key}"),
            );
            value.property_key = Some(key);
            value
        }
        skill::SkillConfigurationError::NotConfigurable { key } => {
            let mut value = rejection(
                dto::SkillConfigurationRejectionCode::NotConfigurable,
                format!("Property cannot be written through this operation: {key}"),
            );
            value.property_key = Some(key);
            value
        }
        skill::SkillConfigurationError::InvalidValue { key, reason } => {
            let mut value = rejection(
                dto::SkillConfigurationRejectionCode::InvalidValue,
                format!("Property {key} is invalid: {reason}"),
            );
            value.property_key = Some(key);
            value
        }
        skill::SkillConfigurationError::PayloadTooLarge { bytes } => {
            let mut value = rejection(
                dto::SkillConfigurationRejectionCode::PayloadTooLarge,
                format!("Configuration payload is too large: {bytes} bytes"),
            );
            value.bytes = Some(bytes);
            value
        }
        skill::SkillConfigurationError::SchemaChanged { expected, actual } => {
            let mut value = rejection(
                dto::SkillConfigurationRejectionCode::SchemaChanged,
                "The Skill configuration schema changed; reload before saving".to_string(),
            );
            value.expected = Some(expected);
            value.actual = Some(actual);
            value
        }
        skill::SkillConfigurationError::BaseRevisionChanged { expected, actual } => {
            let mut value = rejection(
                dto::SkillConfigurationRejectionCode::BaseRevisionChanged,
                "The effective Skill revision changed; reload before saving".to_string(),
            );
            value.expected = Some(expected);
            value.actual = Some(actual);
            value
        }
        skill::SkillConfigurationError::Stale { current } => {
            let mut value = rejection(
                dto::SkillConfigurationRejectionCode::Stale,
                "The stored configuration changed since it was loaded".to_string(),
            );
            value.current = current.map(|record| record_state_to_dto(*record));
            value
        }
        skill::SkillConfigurationError::InvalidWorkspace => rejection(
            dto::SkillConfigurationRejectionCode::InvalidWorkspace,
            "Project-scoped configuration requires a canonical workspace".to_string(),
        ),
        skill::SkillConfigurationError::CredentialFailure { key, reason } => {
            let mut value = rejection(
                dto::SkillConfigurationRejectionCode::CredentialFailure,
                format!("Credential store rejected property {key}: {reason}"),
            );
            value.property_key = Some(key);
            value
        }
        skill::SkillConfigurationError::RepositoryFailure { reason } => rejection(
            dto::SkillConfigurationRejectionCode::RepositoryFailure,
            redact_text(&reason),
        ),
        skill::SkillConfigurationError::RecoveryRequired { properties } => {
            let mut value = rejection(
                dto::SkillConfigurationRejectionCode::RecoveryRequired,
                "Some credentials could not be reconciled and need cleanup".to_string(),
            );
            value.properties = properties;
            value
        }
        skill::SkillConfigurationError::ReconciliationIncomplete { key } => {
            let mut value = rejection(
                dto::SkillConfigurationRejectionCode::ReconciliationIncomplete,
                format!("Obsolete property needs an explicit decision: {key}"),
            );
            value.property_key = Some(key);
            value
        }
    }
}

fn config_scope_to_dto(scope: skill::SkillConfigScope) -> dto::SkillConfigurationScope {
    match scope {
        skill::SkillConfigScope::User => dto::SkillConfigurationScope::User,
        skill::SkillConfigScope::Project => dto::SkillConfigurationScope::Project,
    }
}

fn config_scope_to_domain(scope: dto::SkillConfigurationScope) -> skill::SkillConfigScope {
    match scope {
        dto::SkillConfigurationScope::User => skill::SkillConfigScope::User,
        dto::SkillConfigurationScope::Project => skill::SkillConfigScope::Project,
    }
}

fn config_drift_to_dto(drift: skill::SkillConfigDrift) -> dto::SkillConfigurationDrift {
    match drift {
        skill::SkillConfigDrift::Compatible => dto::SkillConfigurationDrift::Compatible,
        skill::SkillConfigDrift::MigrationRequired => {
            dto::SkillConfigurationDrift::MigrationRequired
        }
        skill::SkillConfigDrift::Invalid => dto::SkillConfigurationDrift::Invalid,
    }
}

fn readiness_to_dto(readiness: skill::SkillConfigReadiness) -> dto::SkillConfigurationReadiness {
    match readiness {
        skill::SkillConfigReadiness::Ready => dto::SkillConfigurationReadiness::Ready,
        skill::SkillConfigReadiness::MissingRequired => {
            dto::SkillConfigurationReadiness::MissingRequired
        }
        skill::SkillConfigReadiness::MigrationRequired => {
            dto::SkillConfigurationReadiness::MigrationRequired
        }
        skill::SkillConfigReadiness::Invalid => dto::SkillConfigurationReadiness::Invalid,
        skill::SkillConfigReadiness::NotConfigurable => {
            dto::SkillConfigurationReadiness::NotConfigurable
        }
    }
}

fn consumption_to_dto(
    consumption: skill::ConfigurationConsumption,
) -> dto::SkillConfigurationConsumption {
    match consumption {
        skill::ConfigurationConsumption::Native => dto::SkillConfigurationConsumption::Native,
        skill::ConfigurationConsumption::UnsupportedExternalCli => {
            dto::SkillConfigurationConsumption::UnsupportedExternalCli
        }
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
    fn utility_delegation_capability_requires_trust_and_runtime_availability() {
        assert_eq!(
            delegation_capability(
                skill::SkillType::Utility,
                skill::SkillTrust::Trusted,
                skill::SkillAvailability::Unsupported,
            ),
            dto::SkillDelegationCapability {
                supported: true,
                reason: "available".to_string(),
            }
        );
        assert_eq!(
            delegation_capability(
                skill::SkillType::Utility,
                skill::SkillTrust::Untrusted,
                skill::SkillAvailability::Available,
            ),
            dto::SkillDelegationCapability {
                supported: false,
                reason: "skill-unavailable".to_string(),
            }
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
                    "reason": "not-utility"
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

    #[test]
    fn configuration_response_reports_secret_presence_without_the_value() {
        // The secret deliberately carries a value no resolver would produce, so the assertion
        // proves the boundary drops it rather than that nothing ever set one.
        let resolution = resolution_to_dto(skill::ResolvedSkillConfiguration {
            properties: vec![
                skill::SkillConfigProperty {
                    key: "api_key".to_string(),
                    value: Some(skill::SkillConfigValue::Text("must-not-appear".to_string())),
                    provenance: skill::SkillConfigProvenance::User,
                    secret: true,
                    secret_state: Some(skill::SkillSecretState::Configured),
                },
                skill::SkillConfigProperty {
                    key: "retries".to_string(),
                    value: Some(skill::SkillConfigValue::Integer(3)),
                    provenance: skill::SkillConfigProvenance::Project,
                    secret: false,
                    secret_state: None,
                },
            ],
            drift: skill::SkillConfigDrift::MigrationRequired,
            readiness: skill::SkillConfigReadiness::MigrationRequired,
            scopes: vec![skill::ScopeConfigurationState {
                scope: skill::SkillConfigScope::Project,
                stored_revision: skill::SkillConfigRevision::new(4),
                schema_hash: "schema-hash".to_string(),
                drift: skill::SkillConfigDrift::MigrationRequired,
                configured_property_count: 1,
                configured_secret_count: 1,
            }],
        });

        let value = serde_json::to_value(resolution).expect("resolution DTO");

        assert_eq!(
            value,
            json!({
                "properties": [
                    {
                        "key": "api_key",
                        "value": null,
                        "provenance": "user",
                        "secret": true,
                        "secretState": "configured"
                    },
                    {
                        "key": "retries",
                        "value": { "type": "integer", "value": 3 },
                        "provenance": "project",
                        "secret": false,
                        "secretState": null
                    }
                ],
                "drift": "migration-required",
                "readiness": "migration-required",
                "scopes": [{
                    "scope": "project",
                    "storedRevision": 4,
                    "schemaHash": "schema-hash",
                    "drift": "migration-required",
                    "configuredPropertyCount": 1,
                    "configuredSecretCount": 1
                }]
            })
        );
    }

    #[test]
    fn stale_configuration_save_returns_the_record_that_won() {
        let outcome = save_outcome_to_dto(Err(skill::SkillConfigurationError::Stale {
            current: Some(Box::new(skill::StoredSkillConfiguration {
                skill_id: "fixture-skill".to_string(),
                scope: skill::SkillConfigScope::User,
                workspace_identity: String::new(),
                schema_hash: "schema-hash".to_string(),
                base_revision: "revision-2".to_string(),
                stored_revision: skill::SkillConfigRevision::new(7),
                validation_state: skill::SkillConfigDrift::Compatible,
                values: vec![("retries".to_string(), skill::SkillConfigValue::Integer(5))],
                secret_keys: vec!["api_key".to_string()],
                orphaned_at: None,
                cleanup_state: skill::SkillConfigCleanupState::None,
            })),
        }));

        let value = serde_json::to_value(outcome).expect("save outcome DTO");

        assert_eq!(value["status"], "rejected");
        assert_eq!(value["rejection"]["code"], "stale");
        assert_eq!(value["rejection"]["current"]["storedRevision"], 7);
        assert_eq!(
            value["rejection"]["current"]["workspaceIdentity"],
            json!(null)
        );
        assert_eq!(value["rejection"]["current"]["secretKeys"][0], "api_key");
        assert_eq!(
            value["rejection"]["current"]["values"][0],
            json!({ "key": "retries", "value": { "type": "integer", "value": 5 } })
        );
    }

    #[test]
    fn configuration_write_input_accepts_the_camel_case_contract() {
        let input: dto::SkillConfigurationWriteInput = serde_json::from_value(json!({
            "scope": "global",
            "workspacePath": null,
            "configScope": "user",
            "schemaHash": "schema-hash",
            "baseRevision": "revision-2",
            "expectedRevision": 2,
            "values": [{ "key": "retries", "value": { "type": "integer", "value": 3 } }],
            "secretIntents": [
                { "key": "api_key", "intent": { "action": "replace", "value": "top-secret" } }
            ]
        }))
        .expect("write input");

        let (key, request) =
            configuration_request("fixture-skill".to_string(), input).expect("request");

        assert_eq!(key.id.as_str(), "fixture-skill");
        assert_eq!(request.scope, skill::SkillConfigScope::User);
        // A User record is keyed by the empty identity even when a workspace is open.
        assert!(request.workspace_identity.is_empty());
        assert_eq!(
            request.expected_revision.map(|value| value.value()),
            Some(2)
        );
        assert_eq!(
            request.values,
            vec![("retries".to_string(), skill::SkillConfigValue::Integer(3))]
        );
        assert_eq!(
            request.secret_intents,
            vec![(
                "api_key".to_string(),
                skill::SkillSecretIntent::Replace("top-secret".to_string())
            )]
        );
    }

    #[test]
    fn secret_replacement_intent_is_not_printable() {
        let rendered = format!(
            "{:?}",
            dto::SkillConfigurationSecretIntent::Replace {
                value: "top-secret".to_string(),
            }
        );

        assert_eq!(rendered, "Replace(<redacted>)");
    }
}
