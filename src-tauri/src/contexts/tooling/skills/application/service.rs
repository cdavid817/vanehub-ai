use super::{
    build_resource_index, logical_base_uri, parse_logical_uri, preview_package, truncate_chars,
    AgentMountConfiguration, BuiltinCleanupStatus, BuiltinReconciliationOutcome,
    BuiltinReconciliationState, EffectiveSkillCatalogPort, OverlayAppliedSkillSnapshotPort,
    SkillAccessRefusal, SkillAccessRefusalReason, SkillAgentMountPath, SkillApiBindingRepository,
    SkillApplicationError, SkillClockPort, SkillConfigurationContext, SkillConfigurationOverview,
    SkillCreateRequest, SkillDiscoveryEntry, SkillDiscoveryRequest, SkillDiscoveryResult,
    SkillDocument, SkillDriftReport, SkillEffectiveMetadata, SkillFailure, SkillFilesystemPort,
    SkillFilesystemTransaction, SkillImportRequest, SkillImportedSource, SkillLegacySourcePort,
    SkillListResult, SkillLoadOutcome, SkillLoadRequest, SkillLoadResult, SkillLogAction,
    SkillLogEvent, SkillLogLevel, SkillLoggingPort, SkillMountMigrationReport, SkillMountRepair,
    SkillOverview, SkillPackageMaterializer, SkillPackageReader, SkillPreview, SkillPromptForAgent,
    SkillReconciliationRepository, SkillRecord, SkillRepository, SkillResourceReadOutcome,
    SkillResourceReadRequest, SkillResourceReadResult, SkillScopeQuery, SkillShadowSummary,
    SkillSourceProbe, SkillStats, SkillSyncResult, SkillUpdateRequest, SkillUsageActivity,
    SkillUsageIdentity, SkillUsageRepository, SkillUsageSummary, SkillWorkspaceSelectionPort,
    UtilitySkillExecutionSnapshot, UtilitySkillResolutionOutcome, BUILTIN_RECONCILIATION_VERSION,
    MAX_DISCOVERY_QUERY_CHARACTERS, MAX_DISCOVERY_RESULTS, MAX_INLINE_SKILL_CHARACTERS,
    MAX_RESOURCE_BYTES,
};
use crate::contexts::tooling::skills::domain::{
    builtin_definition, builtin_definitions, builtin_restore_plan, default_mount_path,
    deletion_policy, detect_drift, plan_binding_change, plan_enablement, resolve_skill_identity,
    source_for_user_create, validate_create_identity, validate_update_identity,
    BuiltinSkillDefinition, SkillAvailability, SkillDomainError, SkillDriftIssueType, SkillId,
    SkillIdentityCandidate, SkillKey, SkillLayer, SkillLocation, SkillLookupOutcome, SkillMetadata,
    SkillMountPath, SkillOrigin, SkillScope, SkillSource, SkillTrust, SkillType,
};

/// Whether reconciling a built-in created a new source or adopted one that was already on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltinSeedOutcome {
    Created,
    Adopted {
        /// Whether the adopted file is still what the shipped definition would have written.
        matches_definition: bool,
    },
}
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, Mutex};

const SKILL_PER_ITEM_CHARACTER_BUDGET: usize = 8_000;
const SKILL_AGGREGATE_CHARACTER_BUDGET: usize = 16_000;

#[derive(Clone)]
pub(crate) struct SkillApplicationService {
    repository: Arc<dyn SkillRepository>,
    api_bindings: Arc<dyn SkillApiBindingRepository>,
    filesystem: Arc<dyn SkillFilesystemPort>,
    selection: Arc<dyn SkillWorkspaceSelectionPort>,
    clock: Arc<dyn SkillClockPort>,
    logging: Arc<dyn SkillLoggingPort>,
    effective_catalog: Option<Arc<dyn EffectiveSkillCatalogPort>>,
    system_materializer: Option<Arc<dyn SkillPackageMaterializer>>,
    effective_materializer: Option<Arc<dyn SkillPackageMaterializer>>,
    system_package_reader: Option<Arc<dyn SkillPackageReader>>,
    effective_package_reader: Option<Arc<dyn SkillPackageReader>>,
    reconciliation_repository: Option<Arc<dyn SkillReconciliationRepository>>,
    legacy_source: Option<Arc<dyn SkillLegacySourcePort>>,
    usage_repository: Option<Arc<dyn SkillUsageRepository>>,
    overlay_applied_snapshots: Option<Arc<dyn OverlayAppliedSkillSnapshotPort>>,
    mutation_coordinator: Arc<Mutex<()>>,
}

impl SkillApplicationService {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        repository: Arc<dyn SkillRepository>,
        api_bindings: Arc<dyn SkillApiBindingRepository>,
        filesystem: Arc<dyn SkillFilesystemPort>,
        selection: Arc<dyn SkillWorkspaceSelectionPort>,
        clock: Arc<dyn SkillClockPort>,
        logging: Arc<dyn SkillLoggingPort>,
    ) -> Self {
        Self {
            repository,
            api_bindings,
            filesystem,
            selection,
            clock,
            logging,
            effective_catalog: None,
            system_materializer: None,
            effective_materializer: None,
            system_package_reader: None,
            effective_package_reader: None,
            reconciliation_repository: None,
            legacy_source: None,
            usage_repository: None,
            overlay_applied_snapshots: None,
            mutation_coordinator: Arc::new(Mutex::new(())),
        }
    }

    pub(crate) fn with_effective_catalog(
        mut self,
        effective_catalog: Arc<dyn EffectiveSkillCatalogPort>,
    ) -> Self {
        self.effective_catalog = Some(effective_catalog);
        self
    }

    pub(crate) fn with_system_materializer(
        mut self,
        system_materializer: Arc<dyn SkillPackageMaterializer>,
    ) -> Self {
        self.system_materializer = Some(system_materializer);
        self
    }

    pub(crate) fn with_effective_materializer(
        mut self,
        effective_materializer: Arc<dyn SkillPackageMaterializer>,
    ) -> Self {
        self.effective_materializer = Some(effective_materializer);
        self
    }

    pub(crate) fn with_builtin_reconciliation(
        mut self,
        system_package_reader: Arc<dyn SkillPackageReader>,
        reconciliation_repository: Arc<dyn SkillReconciliationRepository>,
        legacy_source: Arc<dyn SkillLegacySourcePort>,
    ) -> Self {
        self.system_package_reader = Some(system_package_reader);
        self.reconciliation_repository = Some(reconciliation_repository);
        self.legacy_source = Some(legacy_source);
        self
    }

    pub(crate) fn with_effective_package_reader(
        mut self,
        reader: Arc<dyn SkillPackageReader>,
    ) -> Self {
        self.effective_package_reader = Some(reader);
        self
    }

    pub(crate) fn with_usage_repository(
        mut self,
        repository: Arc<dyn SkillUsageRepository>,
    ) -> Self {
        self.usage_repository = Some(repository);
        self
    }

    pub(crate) fn with_overlay_applied_snapshots(
        mut self,
        snapshots: Arc<dyn OverlayAppliedSkillSnapshotPort>,
    ) -> Self {
        self.overlay_applied_snapshots = Some(snapshots);
        self
    }

    pub(crate) fn list_skills(
        &self,
        query: SkillScopeQuery,
    ) -> Result<SkillListResult, SkillApplicationError> {
        self.ensure_builtins()?;
        let mut skills = self.effective_records(&query.location)?;
        self.filesystem.observe_bindings(&mut skills)?;
        let stats = SkillStats {
            total: skills.len(),
            enabled: skills.iter().filter(|skill| skill.enabled).count(),
            mounted: skills
                .iter()
                .filter(|skill| skill.bindings.iter().any(|binding| binding.mounted))
                .count(),
        };
        Ok(SkillListResult { skills, stats })
    }

    pub(crate) fn list_skills_for_agent(
        &self,
        request: SkillDiscoveryRequest,
    ) -> Result<SkillDiscoveryResult, SkillApplicationError> {
        self.ensure_builtins()?;
        let limit = request.limit.unwrap_or(50);
        if !(1..=MAX_DISCOVERY_RESULTS).contains(&limit)
            || request
                .query
                .as_deref()
                .is_some_and(|query| query.chars().count() > MAX_DISCOVERY_QUERY_CHARACTERS)
        {
            return Err(SkillApplicationError::Validation(
                "Skill discovery filters exceed their limits".to_string(),
            ));
        }
        let location = progressive_location(request.workspace_path.as_deref())?;
        let query = request.query.as_deref().map(str::to_lowercase);
        let records = self.effective_records(&location)?;
        for record in &records {
            let effective = record.effective_metadata();
            for availability in std::iter::once(effective.availability)
                .chain(effective.shadowed.iter().map(|shadow| shadow.availability))
                .filter(|availability| {
                    matches!(
                        availability,
                        SkillAvailability::Invalid | SkillAvailability::Conflicting
                    )
                })
            {
                let refusal = access_refusal(
                    record.key.id.as_str(),
                    Some(record.key.id.as_str()),
                    refusal_for_availability(availability),
                );
                self.record_access_refusal(
                    SkillLogAction::ListAgentSkills,
                    &refusal,
                    Some(effective.layer),
                    None,
                );
            }
        }
        let mut skills = records
            .into_iter()
            .filter_map(|record| {
                let effective = record.effective_metadata();
                let matches_query = query.as_ref().is_none_or(|query| {
                    record.key.id.as_str().to_lowercase().contains(query)
                        || record.metadata.name.to_lowercase().contains(query)
                        || record.metadata.description.to_lowercase().contains(query)
                        || record
                            .metadata
                            .aliases
                            .iter()
                            .any(|alias| alias.as_str().to_lowercase().contains(query))
                });
                (matches_query
                    && request
                        .skill_type
                        .is_none_or(|value| value == effective.skill_type)
                    && request
                        .delivery
                        .is_none_or(|value| value == effective.delivery)
                    && request
                        .availability
                        .is_none_or(|value| value == effective.availability))
                .then(|| SkillDiscoveryEntry {
                    id: record.key.id.as_str().to_string(),
                    name: record.metadata.name,
                    description: record.metadata.description,
                    aliases: record
                        .metadata
                        .aliases
                        .into_iter()
                        .map(|alias| alias.as_str().to_string())
                        .collect(),
                    skill_type: effective.skill_type,
                    delivery: effective.delivery,
                    layer: effective.layer,
                    availability: effective.availability,
                    version: record.metadata.version,
                })
            })
            .collect::<Vec<_>>();
        skills.sort_by(|left, right| left.id.cmp(&right.id));
        let truncated = skills.len() > limit;
        skills.truncate(limit);
        let mut context = BTreeMap::new();
        context.insert("count".to_string(), skills.len().to_string());
        context.insert("truncated".to_string(), truncated.to_string());
        let _ = self.logging.record(&SkillLogEvent {
            action: SkillLogAction::ListAgentSkills,
            level: SkillLogLevel::Info,
            skill_id: None,
            message: "Listed effective Skills for agent discovery".to_string(),
            timestamp: self.clock.now(),
            context,
        });
        Ok(SkillDiscoveryResult { skills, truncated })
    }

    pub(crate) fn load_skill_for_agent(
        &self,
        request: SkillLoadRequest,
    ) -> Result<SkillLoadOutcome, SkillApplicationError> {
        self.ensure_builtins()?;
        let requested = SkillId::parse(&request.id_or_alias)?;
        let location = progressive_location(request.workspace_path.as_deref())?;
        let resolved = self.resolve_progressive_package(&location, &requested)?;
        let (package, _) = match resolved {
            Ok(value) => value,
            Err(refusal) => {
                self.record_access_refusal(SkillLogAction::LoadSkill, &refusal, None, None);
                return Ok(SkillLoadOutcome::Refused(refusal));
            }
        };
        let workspace_identity = (location.scope == SkillScope::Workspace)
            .then_some(location.workspace_path.as_deref())
            .flatten();
        let loaded = if let Some(snapshots) = &self.overlay_applied_snapshots {
            snapshots
                .read_overlay_applied_package(&package.metadata.id, workspace_identity)
                .and_then(|snapshot| {
                    let effective = snapshot.replay.effective();
                    let resources = effective
                        .resources()
                        .iter()
                        .map(|resource| super::SkillPackageResource {
                            relative_path: resource.logical_path.clone(),
                            media_type: resource.media_type.clone(),
                            size_bytes: resource.size_bytes,
                            content_hash: resource.content_hash.clone(),
                        })
                        .collect();
                    Ok((
                        effective.instructions().to_string(),
                        build_resource_index(&package.metadata.id, resources)?,
                        effective.effective_hash().to_string(),
                    ))
                })
        } else {
            let reader = self.effective_package_reader.as_ref().ok_or_else(|| {
                SkillApplicationError::Validation("Effective package reader is unavailable".into())
            })?;
            reader.read_document(&package).and_then(|document| {
                let resources = reader
                    .list_resources(&package)
                    .and_then(|resources| build_resource_index(&package.metadata.id, resources))?;
                Ok((document.body, resources, package.revision.clone()))
            })
        };
        let (instructions, resources, revision) = match loaded {
            Ok(loaded) => loaded,
            Err(error) => {
                let refusal = access_refusal(
                    request.id_or_alias,
                    Some(package.metadata.id.as_str()),
                    resource_error_reason(&error),
                );
                self.record_access_refusal(
                    SkillLogAction::LoadSkill,
                    &refusal,
                    Some(package.layer),
                    None,
                );
                return Ok(SkillLoadOutcome::Refused(refusal));
            }
        };
        let base_uri = logical_base_uri(&package.metadata.id);
        let expanded = instructions.replace("{skill_base_dir}", &base_uri);
        let (content, truncated) = truncate_chars(&expanded, MAX_INLINE_SKILL_CHARACTERS);
        let result = SkillLoadResult {
            id: package.metadata.id.as_str().to_string(),
            name: package.metadata.name.clone(),
            content,
            truncated,
            revision: revision.clone(),
            base_uri,
            resources,
        };
        let mut context = BTreeMap::new();
        context.insert("layer".to_string(), package.layer.as_str().to_string());
        context.insert("truncated".to_string(), truncated.to_string());
        context.insert(
            "characters".to_string(),
            result.content.chars().count().to_string(),
        );
        let _ = self.logging.record(&SkillLogEvent {
            action: SkillLogAction::LoadSkill,
            level: SkillLogLevel::Info,
            skill_id: Some(result.id.clone()),
            message: "Loaded effective Skill instructions".to_string(),
            timestamp: self.clock.now(),
            context,
        });
        let mut witnessed_package = package;
        witnessed_package.revision = revision;
        self.bump_view(&witnessed_package);
        Ok(SkillLoadOutcome::Loaded(result))
    }

    pub(crate) fn resolve_utility_for_execution(
        &self,
        id_or_alias: &str,
        workspace_path: Option<&str>,
    ) -> Result<UtilitySkillResolutionOutcome, SkillApplicationError> {
        self.ensure_builtins()?;
        let requested = SkillId::parse(id_or_alias)?;
        let location = progressive_location(workspace_path)?;
        let catalog = self.effective_catalog.as_ref().ok_or_else(|| {
            SkillApplicationError::Validation("Effective Skill catalog is unavailable".into())
        })?;
        let workspace = (location.scope == SkillScope::Workspace)
            .then_some(location.workspace_path.as_deref())
            .flatten();
        let records = self
            .effective_records(&location)?
            .into_iter()
            .map(|record| (record.key.id.clone(), record))
            .collect::<BTreeMap<_, _>>();
        let candidates = records
            .values()
            .map(|record| SkillIdentityCandidate {
                id: record.key.id.clone(),
                aliases: record.metadata.aliases.clone(),
                availability: utility_resolution_availability(record),
            })
            .collect::<Vec<_>>();
        let resolved = match resolve_skill_identity(&requested, &candidates) {
            SkillLookupOutcome::Resolved(id) => id,
            SkillLookupOutcome::Unavailable { id, availability } => {
                return Ok(UtilitySkillResolutionOutcome::Refused(access_refusal(
                    requested.as_str(),
                    Some(id.as_str()),
                    refusal_for_availability(availability),
                )))
            }
            SkillLookupOutcome::Ambiguous(ids) => {
                return Ok(UtilitySkillResolutionOutcome::Refused(SkillAccessRefusal {
                    requested: requested.as_str().to_string(),
                    canonical_id: None,
                    reason: SkillAccessRefusalReason::AmbiguousAlias,
                    conflicting_ids: ids
                        .into_iter()
                        .take(8)
                        .map(|id| id.as_str().to_string())
                        .collect(),
                }))
            }
            SkillLookupOutcome::NotFound => {
                return Ok(UtilitySkillResolutionOutcome::Refused(access_refusal(
                    requested.as_str(),
                    None,
                    SkillAccessRefusalReason::NotFound,
                )))
            }
        };
        let record = records
            .get(&resolved)
            .ok_or_else(|| SkillApplicationError::NotFound(resolved.as_str().to_string()))?;
        if record.effective_metadata().skill_type != SkillType::Utility {
            return Ok(UtilitySkillResolutionOutcome::Refused(access_refusal(
                requested.as_str(),
                Some(resolved.as_str()),
                SkillAccessRefusalReason::Unsupported,
            )));
        }
        let package = catalog
            .effective_catalog(workspace)?
            .into_iter()
            .find(|skill| skill.effective.metadata.id == resolved)
            .map(|skill| skill.effective)
            .ok_or_else(|| SkillApplicationError::NotFound(resolved.as_str().to_string()))?;
        let (instructions, revision) = if let Some(snapshots) = &self.overlay_applied_snapshots {
            let snapshot =
                snapshots.read_overlay_applied_package(&package.metadata.id, workspace)?;
            (
                snapshot.replay.effective().instructions().to_string(),
                snapshot.replay.effective().effective_hash().to_string(),
            )
        } else {
            let reader = self.effective_package_reader.as_ref().ok_or_else(|| {
                SkillApplicationError::Validation("Effective package reader is unavailable".into())
            })?;
            (reader.read_document(&package)?.body, package.revision)
        };
        if instructions.trim().is_empty()
            || instructions.chars().count() > MAX_INLINE_SKILL_CHARACTERS
        {
            return Ok(UtilitySkillResolutionOutcome::Refused(access_refusal(
                requested.as_str(),
                Some(resolved.as_str()),
                SkillAccessRefusalReason::OversizedResource,
            )));
        }
        Ok(UtilitySkillResolutionOutcome::Resolved(
            UtilitySkillExecutionSnapshot {
                id: resolved.as_str().to_string(),
                revision,
                instructions,
                workspace_path: workspace.map(str::to_string),
            },
        ))
    }

    pub(crate) fn read_skill_resource_for_agent(
        &self,
        request: SkillResourceReadRequest,
    ) -> Result<SkillResourceReadOutcome, SkillApplicationError> {
        self.ensure_builtins()?;
        let parsed = match parse_logical_uri(&request.uri) {
            Ok(parsed) if parsed.relative_path.is_some() => parsed,
            _ => {
                let refusal = access_refusal(
                    bounded_requested(&request.uri),
                    None,
                    SkillAccessRefusalReason::InvalidUri,
                );
                self.record_access_refusal(SkillLogAction::ReadSkillResource, &refusal, None, None);
                return Ok(SkillResourceReadOutcome::Refused(refusal));
            }
        };
        let Some(relative_path) = parsed.relative_path else {
            return Ok(SkillResourceReadOutcome::Refused(access_refusal(
                bounded_requested(&request.uri),
                None,
                SkillAccessRefusalReason::InvalidUri,
            )));
        };
        let location = progressive_location(request.workspace_path.as_deref())?;
        let resolved = self.resolve_progressive_package(&location, &parsed.id)?;
        let (package, _) = match resolved {
            Ok(value) => value,
            Err(refusal) => {
                self.record_access_refusal(SkillLogAction::ReadSkillResource, &refusal, None, None);
                return Ok(SkillResourceReadOutcome::Refused(refusal));
            }
        };
        let workspace_identity = (location.scope == SkillScope::Workspace)
            .then_some(location.workspace_path.as_deref())
            .flatten();
        let overlay_snapshot = match self.overlay_applied_snapshots.as_ref() {
            Some(snapshots) => match snapshots
                .read_overlay_applied_package(&package.metadata.id, workspace_identity)
            {
                Ok(snapshot) => Some(snapshot),
                Err(error) => {
                    return Ok(SkillResourceReadOutcome::Refused(self.resource_refusal(
                        &request.uri,
                        &package,
                        resource_error_reason(&error),
                        None,
                    )))
                }
            },
            None => None,
        };
        let effective_revision = overlay_snapshot
            .as_ref()
            .map(|snapshot| snapshot.replay.effective().effective_hash())
            .unwrap_or(&package.revision);
        if request.revision != effective_revision {
            let refusal = access_refusal(
                request.uri,
                Some(package.metadata.id.as_str()),
                SkillAccessRefusalReason::StaleRevision,
            );
            self.record_access_refusal(
                SkillLogAction::ReadSkillResource,
                &refusal,
                Some(package.layer),
                None,
            );
            return Ok(SkillResourceReadOutcome::Refused(refusal));
        }
        let indexed_resources = if let Some(snapshot) = &overlay_snapshot {
            snapshot
                .replay
                .effective()
                .resources()
                .iter()
                .map(|resource| super::SkillPackageResource {
                    relative_path: resource.logical_path.clone(),
                    media_type: resource.media_type.clone(),
                    size_bytes: resource.size_bytes,
                    content_hash: resource.content_hash.clone(),
                })
                .collect()
        } else {
            let reader = self.effective_package_reader.as_ref().ok_or_else(|| {
                SkillApplicationError::Validation("Effective package reader is unavailable".into())
            })?;
            reader.list_resources(&package)?
        };
        let index = match build_resource_index(&package.metadata.id, indexed_resources) {
            Ok(index) => index,
            Err(error) => {
                return Ok(SkillResourceReadOutcome::Refused(self.resource_refusal(
                    &request.uri,
                    &package,
                    resource_error_reason(&error),
                    None,
                )))
            }
        };
        let Some(entry) = index.entry(&relative_path) else {
            return Ok(SkillResourceReadOutcome::Refused(self.resource_refusal(
                &request.uri,
                &package,
                SkillAccessRefusalReason::UnindexedResource,
                None,
            )));
        };
        if entry.size_bytes > MAX_RESOURCE_BYTES {
            return Ok(SkillResourceReadOutcome::Refused(self.resource_refusal(
                &request.uri,
                &package,
                SkillAccessRefusalReason::OversizedResource,
                Some(entry.size_bytes),
            )));
        }
        let resource_read = if let Some(snapshots) = &self.overlay_applied_snapshots {
            snapshots.read_overlay_applied_resource(
                &package.metadata.id,
                workspace_identity,
                &request.revision,
                &relative_path,
            )
        } else {
            let reader = self.effective_package_reader.as_ref().ok_or_else(|| {
                SkillApplicationError::Validation("Effective package reader is unavailable".into())
            })?;
            reader.read_resource(&package, &relative_path)
        };
        let resource = match resource_read {
            Ok(resource) => resource,
            Err(error) => {
                return Ok(SkillResourceReadOutcome::Refused(self.resource_refusal(
                    &request.uri,
                    &package,
                    resource_error_reason(&error),
                    Some(entry.size_bytes),
                )))
            }
        };
        let result = SkillResourceReadResult {
            id: package.metadata.id.as_str().to_string(),
            uri: request.uri,
            revision: request.revision,
            content: resource.content,
            size_bytes: resource.size_bytes,
        };
        let mut context = BTreeMap::new();
        context.insert("layer".to_string(), package.layer.as_str().to_string());
        context.insert("sizeBytes".to_string(), result.size_bytes.to_string());
        let _ = self.logging.record(&SkillLogEvent {
            action: SkillLogAction::ReadSkillResource,
            level: SkillLogLevel::Info,
            skill_id: Some(result.id.clone()),
            message: "Read indexed Skill resource".to_string(),
            timestamp: self.clock.now(),
            context,
        });
        Ok(SkillResourceReadOutcome::Read(result))
    }

    pub(crate) fn skill_overview(
        &self,
        query: SkillScopeQuery,
    ) -> Result<SkillOverview, SkillApplicationError> {
        self.ensure_builtins()?;
        let mut skills = self.effective_records(&query.location)?;
        self.filesystem.observe_bindings(&mut skills)?;
        let stats = skill_stats(&skills);
        let mount_paths = self.list_mount_paths()?;
        let agents = self.repository.compatible_agents()?;
        let api_agent_bindings = self
            .repository
            .api_agent_bindings_for_location(&query.location)?;
        let deleted = self.repository.deleted_builtin_ids()?;
        let inspection = self
            .filesystem
            .inspect_drift(&query.location, &skills, &deleted)?;
        let issues = detect_drift(&inspection);
        let drift = SkillDriftReport {
            location: query.location.clone(),
            drift_hash: drift_hash(&issues),
            issues,
        };
        let restore_candidates = if query.location.scope == SkillScope::Global {
            self.repository
                .deleted_builtin_ids()?
                .into_iter()
                .map(|id| id.as_str().to_string())
                .collect()
        } else {
            Vec::new()
        };
        Ok(SkillOverview {
            skills,
            stats,
            mount_paths,
            agents,
            api_agent_bindings,
            drift,
            restore_candidates,
        })
    }

    pub(crate) fn list_mount_paths(
        &self,
    ) -> Result<Vec<SkillAgentMountPath>, SkillApplicationError> {
        self.repository
            .agent_mount_configurations()?
            .into_iter()
            .map(|configuration| {
                let is_default = configuration.configured_path.is_none();
                let mount_path = configuration.configured_path.map_or_else(
                    || SkillMountPath::parse(default_mount_path(&configuration.agent_id)),
                    Ok,
                )?;
                Ok(SkillAgentMountPath {
                    agent_id: configuration.agent_id,
                    mount_path,
                    is_default,
                })
            })
            .collect()
    }

    pub(crate) fn update_mount_path(
        &self,
        agent_id: String,
        new_mount_path: SkillMountPath,
    ) -> Result<SkillMountMigrationReport, SkillApplicationError> {
        let result = self.update_mount_path_work(&agent_id, new_mount_path);
        let success_level = result
            .as_ref()
            .ok()
            .filter(|report| !report.failed.is_empty())
            .map(|_| SkillLogLevel::Warn)
            .unwrap_or(SkillLogLevel::Info);
        self.observe_with_level(SkillLogAction::UpdateMountPath, None, success_level, result)
    }

    pub(crate) fn create_skill(
        &self,
        request: SkillCreateRequest,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let skill_id = request.id.as_str().to_string();
        let result = self.create_skill_work(request);
        self.observe(SkillLogAction::Create, Some(skill_id), result)
    }

    pub(crate) fn update_skill(
        &self,
        request: SkillUpdateRequest,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let skill_id = request.key.id.as_str().to_string();
        let result = self.update_skill_work(request);
        self.observe(SkillLogAction::Update, Some(skill_id), result)
    }

    pub(crate) fn delete_skill(&self, key: SkillKey) -> Result<(), SkillApplicationError> {
        let skill_id = key.id.as_str().to_string();
        let result = self.delete_skill_work(&key);
        self.observe(SkillLogAction::Delete, Some(skill_id), result)
    }

    pub(crate) fn restore_builtin(
        &self,
        id: SkillId,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let skill_id = id.as_str().to_string();
        let result = self.restore_builtin_work(&id);
        self.observe(SkillLogAction::Restore, Some(skill_id), result)
    }

    pub(crate) fn set_enabled(
        &self,
        key: SkillKey,
        enabled: bool,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let skill_id = key.id.as_str().to_string();
        let result = self.set_enabled_work(&key, enabled);
        self.observe(SkillLogAction::SetEnabled, Some(skill_id), result)
    }

    pub(crate) fn set_bindings(
        &self,
        key: SkillKey,
        agent_ids: Vec<String>,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let skill_id = key.id.as_str().to_string();
        let result = self.set_bindings_work(&key, agent_ids);
        self.observe(SkillLogAction::SetBindings, Some(skill_id), result)
    }

    pub(crate) fn bind_skill_to_cli_agent(
        &self,
        key: SkillKey,
        agent_id: String,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let skill_id = key.id.as_str().to_string();
        let result = self.change_cli_binding(&key, &agent_id, true);
        self.observe_for_agent(
            SkillLogAction::BindCliAgent,
            Some(skill_id),
            &agent_id,
            result,
        )
    }

    pub(crate) fn unbind_skill_from_cli_agent(
        &self,
        key: SkillKey,
        agent_id: String,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let skill_id = key.id.as_str().to_string();
        let result = self.change_cli_binding(&key, &agent_id, false);
        self.observe_for_agent(
            SkillLogAction::UnbindCliAgent,
            Some(skill_id),
            &agent_id,
            result,
        )
    }

    /// Binds `key` to `agent_id` for API-agent system-prompt injection (`add-agent-skill-support`)
    /// — a non-mount binding, distinct from `set_bindings`' CLI mount-path binding.
    pub(crate) fn bind_skill_to_api_agent(
        &self,
        key: SkillKey,
        agent_id: String,
    ) -> Result<(), SkillApplicationError> {
        let skill_id = key.id.as_str().to_string();
        let result = self.bind_skill_to_api_agent_work(&key, &agent_id);
        self.observe(SkillLogAction::SetApiAgentBinding, Some(skill_id), result)
    }

    pub(crate) fn unbind_skill_from_api_agent(
        &self,
        key: SkillKey,
        agent_id: String,
    ) -> Result<(), SkillApplicationError> {
        let skill_id = key.id.as_str().to_string();
        let result = (|| {
            self.load(&key)?;
            self.ensure_api_agent(&agent_id)?;
            self.api_bindings.unbind_api_agent(&key, &agent_id)
        })();
        self.observe(SkillLogAction::SetApiAgentBinding, Some(skill_id), result)
    }

    pub(crate) fn list_api_agent_bindings(
        &self,
        key: SkillKey,
    ) -> Result<Vec<String>, SkillApplicationError> {
        self.api_bindings.api_agent_bindings(&key)
    }

    /// Reads every enabled Skill bound to `agent_id`, resolving each one's source file into
    /// `{name, body}` ready for system-prompt injection. Used by `agent_runtime`'s
    /// `AgentSkillPort` adapter, not by any Skill-management UI flow.
    pub(crate) fn bound_skill_prompts_for_api_agent(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Vec<SkillPromptForAgent>, SkillApplicationError> {
        let canonical_workspace = workspace_path
            .map(|path| {
                std::path::Path::new(path)
                    .canonicalize()
                    .map(|canonical| {
                        let value = canonical.to_string_lossy();
                        value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
                    })
                    .map_err(|error| {
                        SkillApplicationError::Validation(format!(
                            "Active workspace path is invalid: {error}"
                        ))
                    })
            })
            .transpose()?;
        let records = self
            .api_bindings
            .enabled_skills_bound_to_api_agent(agent_id, canonical_workspace.as_deref())?;
        if let (Some(catalog), Some(reader)) =
            (&self.effective_catalog, &self.effective_package_reader)
        {
            let bindings = records
                .into_iter()
                .map(|record| (record.key.id.clone(), record))
                .collect::<BTreeMap<_, _>>();
            let mut effective = catalog.effective_catalog(canonical_workspace.as_deref())?;
            effective.sort_by(|left, right| {
                left.effective
                    .layer
                    .rank()
                    .cmp(&right.effective.layer.rank())
                    .then_with(|| {
                        left.effective
                            .workspace_path
                            .cmp(&right.effective.workspace_path)
                    })
                    .then_with(|| left.effective.metadata.id.cmp(&right.effective.metadata.id))
            });
            let mut prompts = Vec::new();
            let mut used = 0_usize;
            for skill in effective {
                let package = skill.effective;
                let runtime_available = self
                    .reconciliation_repository
                    .as_ref()
                    .map(|repository| repository.builtin_reconciliation(&package.metadata.id))
                    .transpose()?
                    .flatten()
                    .is_none_or(|state| {
                        !matches!(
                            state.outcome,
                            BuiltinReconciliationOutcome::Invalid
                                | BuiltinReconciliationOutcome::Deleted
                        )
                    });
                if !bindings.contains_key(&package.metadata.id)
                    || !runtime_available
                    || package.availability != SkillAvailability::Available
                    || package.metadata.skill_type != SkillType::Role
                    || package.metadata.delivery
                        != crate::contexts::tooling::skills::domain::SkillDelivery::Eager
                {
                    continue;
                }
                if let Some(snapshots) = &self.overlay_applied_snapshots {
                    match snapshots.read_overlay_applied_package(
                        &package.metadata.id,
                        canonical_workspace.as_deref(),
                    ) {
                        Ok(snapshot) => {
                            let mut effective_package = package.clone();
                            effective_package.revision =
                                snapshot.replay.effective().effective_hash().to_string();
                            self.consider_prompt(
                                &mut prompts,
                                &mut used,
                                &effective_package,
                                snapshot.replay.effective().instructions().to_string(),
                            );
                        }
                        Err(_) => self.record_prompt_skip(&package.metadata.id, "unreadable", None),
                    }
                } else {
                    match reader.read_document(&package) {
                        Ok(document) => {
                            self.consider_prompt(&mut prompts, &mut used, &package, document.body)
                        }
                        Err(_) => self.record_prompt_skip(&package.metadata.id, "unreadable", None),
                    }
                }
            }
            return Ok(prompts);
        }

        let mut prompts = Vec::new();
        let mut used = 0_usize;
        for record in records {
            if record.metadata.skill_type != SkillType::Role
                || record.metadata.delivery
                    != crate::contexts::tooling::skills::domain::SkillDelivery::Eager
            {
                continue;
            }
            match self.filesystem.read_source(&record) {
                Ok(raw) => {
                    let package = super::SkillPackageDescriptor {
                        package_key: record.managed_source.skill_dir.clone(),
                        workspace_path: record.key.location.workspace_path.clone(),
                        metadata: record.metadata.clone(),
                        layer: record.effective_metadata().layer,
                        origin: record.effective_metadata().origin,
                        trust: record.effective_metadata().trust,
                        availability: SkillAvailability::Available,
                        revision: record.managed_source.content_hash.clone(),
                        source_path: Some(record.managed_source.skill_dir.clone()),
                    };
                    self.consider_prompt(
                        &mut prompts,
                        &mut used,
                        &package,
                        strip_frontmatter(&raw),
                    );
                }
                Err(_) => self.record_prompt_skip(&record.key.id, "unreadable", None),
            }
        }
        Ok(prompts)
    }

    fn consider_prompt(
        &self,
        prompts: &mut Vec<SkillPromptForAgent>,
        used: &mut usize,
        package: &super::SkillPackageDescriptor,
        body: String,
    ) {
        let length = format!("## {}\n{}", package.metadata.name, body)
            .chars()
            .count();
        let reason = if length > SKILL_PER_ITEM_CHARACTER_BUDGET {
            Some("individual-budget")
        } else if used.saturating_add(length) > SKILL_AGGREGATE_CHARACTER_BUDGET {
            Some("aggregate-budget")
        } else {
            None
        };
        if let Some(reason) = reason {
            self.record_prompt_skip(&package.metadata.id, reason, Some(length));
            return;
        }
        *used += length;
        prompts.push(SkillPromptForAgent {
            id: package.metadata.id.as_str().to_string(),
            name: package.metadata.name.clone(),
            body,
            revision: package.revision.clone(),
        });
        self.bump_use(package);
    }

    pub(crate) fn bump_view(
        &self,
        package: &super::SkillPackageDescriptor,
    ) -> Option<SkillUsageSummary> {
        self.bump_usage(package, SkillUsageActivity::View, BTreeMap::new())
    }

    pub(crate) fn bump_use(
        &self,
        package: &super::SkillPackageDescriptor,
    ) -> Option<SkillUsageSummary> {
        self.bump_usage(package, SkillUsageActivity::Use, BTreeMap::new())
    }

    fn bump_usage(
        &self,
        package: &super::SkillPackageDescriptor,
        activity: SkillUsageActivity,
        mut context: BTreeMap<String, String>,
    ) -> Option<SkillUsageSummary> {
        context.insert("layer".to_string(), package.layer.as_str().to_string());
        context.insert("revision".to_string(), package.revision.clone());
        let action = match activity {
            SkillUsageActivity::View => SkillLogAction::TrackView,
            SkillUsageActivity::Use => SkillLogAction::TrackUse,
        };
        let timestamp = self.clock.now();
        let location = match package.layer {
            SkillLayer::Project => package
                .workspace_path
                .as_deref()
                .and_then(|path| SkillLocation::new(SkillScope::Workspace, Some(path)).ok()),
            _ => SkillLocation::new(SkillScope::Global, None).ok(),
        };
        let result = self.usage_repository.as_ref().and_then(|repository| {
            let location = location.as_ref()?;
            let identity = SkillUsageIdentity {
                id: package.metadata.id.clone(),
                layer: package.layer,
            };
            match repository.bump(location, &identity, activity, &timestamp, &package.revision) {
                Ok(mutation) => {
                    if mutation.recovered_corrupt_state {
                        context.insert("recoveredCorruptState".to_string(), "true".to_string());
                    }
                    Some(mutation.summary)
                }
                Err(error) => {
                    context.insert("reason".to_string(), usage_error_code(&error).to_string());
                    None
                }
            }
        });
        let level = if self.usage_repository.is_some() && result.is_none() {
            SkillLogLevel::Warn
        } else {
            SkillLogLevel::Info
        };
        let message = match (activity, level) {
            (SkillUsageActivity::View, SkillLogLevel::Warn) => "Skill view tracking failed",
            (SkillUsageActivity::View, _) => "Recorded Skill view",
            (SkillUsageActivity::Use, SkillLogLevel::Warn) => "Skill use tracking failed",
            (SkillUsageActivity::Use, _) => "Recorded eager Skill use",
        };
        let _ = self.logging.record(&SkillLogEvent {
            action,
            level,
            skill_id: Some(package.metadata.id.as_str().to_string()),
            message: message.to_string(),
            timestamp,
            context,
        });
        result
    }

    fn record_prompt_skip(&self, id: &SkillId, reason: &str, size: Option<usize>) {
        let mut context = BTreeMap::new();
        context.insert("reason".to_string(), reason.to_string());
        if let Some(size) = size {
            context.insert("characters".to_string(), size.to_string());
        }
        let _ = self.logging.record(&SkillLogEvent {
            action: SkillLogAction::ResolveApiPrompt,
            level: SkillLogLevel::Warn,
            skill_id: Some(id.as_str().to_string()),
            message: "Skipped Skill during API prompt assembly".to_string(),
            timestamp: self.clock.now(),
            context,
        });
    }

    fn bind_skill_to_api_agent_work(
        &self,
        key: &SkillKey,
        agent_id: &str,
    ) -> Result<(), SkillApplicationError> {
        let record = self.load(key)?;
        self.ensure_api_agent(agent_id)?;
        if self.repository.get(key)?.is_none() {
            self.repository
                .save_skills(std::slice::from_ref(&record), &[])?;
        }
        self.api_bindings
            .bind_api_agent(key, agent_id, &self.clock.now())
    }

    fn ensure_api_agent(&self, agent_id: &str) -> Result<(), SkillApplicationError> {
        if self.repository.is_api_agent(agent_id)? {
            Ok(())
        } else {
            Err(SkillApplicationError::Validation(format!(
                "Unknown Agent id: {agent_id}"
            )))
        }
    }

    pub(crate) fn preview_skill(
        &self,
        key: SkillKey,
    ) -> Result<SkillPreview, SkillApplicationError> {
        let record = self.load(&key)?;
        let effective = record.effective_metadata();
        let path = if effective.layer == SkillLayer::System {
            format!("skill://{}/", record.key.id.as_str())
        } else {
            record.managed_source.skill_md_path.clone()
        };
        Ok(SkillPreview {
            key,
            content: self.filesystem.read_source(&record)?,
            path,
            effective,
        })
    }

    /// Resolves the configuration schema through the same effective-package path as `preview_skill`
    /// rather than through the registry. A record rebuilt from SQLite carries metadata without the
    /// `config_schema` frontmatter block, so reading the schema from there would report every
    /// configurable Skill as having none.
    pub(crate) fn configuration_context(
        &self,
        key: &SkillKey,
    ) -> Result<SkillConfigurationContext, SkillApplicationError> {
        let record = self.load(key)?;
        let overview = record.effective_metadata().configuration;
        Ok(SkillConfigurationContext::from_winning_metadata(
            &record.metadata,
            overview,
        ))
    }

    pub(crate) fn import_skill(
        &self,
        request: SkillImportRequest,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let result = self.import_skill_work(request);
        let skill_id = result
            .as_ref()
            .ok()
            .map(|record| record.key.id.as_str().to_string());
        self.observe(SkillLogAction::Import, skill_id, result)
    }

    pub(crate) fn detect_skill_drift(
        &self,
        query: SkillScopeQuery,
    ) -> Result<SkillDriftReport, SkillApplicationError> {
        let result = self.detect_skill_drift_work(&query.location);
        self.observe(SkillLogAction::DetectDrift, None, result)
    }

    pub(crate) fn sync_skill_drift(
        &self,
        query: SkillScopeQuery,
    ) -> Result<SkillSyncResult, SkillApplicationError> {
        let result = self.sync_skill_drift_work(query.location);
        let success_level = result
            .as_ref()
            .ok()
            .filter(|sync| !sync.failed.is_empty())
            .map(|_| SkillLogLevel::Warn)
            .unwrap_or(SkillLogLevel::Info);
        self.observe_with_level(SkillLogAction::SyncDrift, None, success_level, result)
    }

    pub(crate) fn select_workspace_directory(
        &self,
    ) -> Result<Option<String>, SkillApplicationError> {
        self.selection.select_workspace_directory()
    }

    fn ensure_builtins(&self) -> Result<(), SkillApplicationError> {
        if self.system_reconciliation_ready() {
            return self.reconcile_system_builtins();
        }
        let location = SkillLocation::new(SkillScope::Global, None)?;
        let existing = self
            .repository
            .list(&location)?
            .into_iter()
            .map(|record| record.key.id)
            .collect::<BTreeSet<_>>();
        let deleted = self
            .repository
            .deleted_builtin_ids()?
            .into_iter()
            .collect::<BTreeSet<_>>();
        let mut missing = Vec::new();
        for definition in builtin_definitions().iter().copied() {
            let metadata = definition.metadata()?;
            if !existing.contains(&metadata.id) && !deleted.contains(&metadata.id) {
                missing.push((definition, metadata));
            }
        }
        if missing.is_empty() {
            return Ok(());
        }

        // Reconciled one Skill at a time rather than in a single all-or-nothing transaction: a
        // shared transaction means one unusable source discards the records for every other
        // built-in, which is how an installation ends up with zero rows instead of five.
        let mut created = Vec::new();
        let mut adopted = Vec::new();
        let mut diverged = Vec::new();
        let mut failed = Vec::new();
        for (definition, metadata) in &missing {
            let id = metadata.id.as_str().to_string();
            match self.reconcile_builtin(&location, *definition, metadata) {
                Ok(BuiltinSeedOutcome::Created) => created.push(id),
                Ok(BuiltinSeedOutcome::Adopted { matches_definition }) => {
                    if !matches_definition {
                        diverged.push(id.clone());
                    }
                    adopted.push(id);
                }
                Err(error) => failed.push((id, error)),
            }
        }
        self.report_builtin_seeding(&created, &adopted, &diverged, &failed);
        if !created.is_empty() || !adopted.is_empty() {
            self.invalidate_effective_catalog();
        }
        Ok(())
    }

    fn system_reconciliation_ready(&self) -> bool {
        self.effective_catalog.is_some()
            && self.system_materializer.is_some()
            && self.system_package_reader.is_some()
            && self.reconciliation_repository.is_some()
            && self.legacy_source.is_some()
    }

    fn system_package(
        &self,
        id: &SkillId,
    ) -> Result<super::SkillPackageDescriptor, SkillApplicationError> {
        self.effective_catalog
            .as_ref()
            .ok_or_else(|| SkillApplicationError::NotFound(id.as_str().to_string()))?
            .effective_catalog(None)?
            .into_iter()
            .flat_map(|skill| std::iter::once(skill.effective).chain(skill.shadowed))
            .find(|package| package.layer == SkillLayer::System && package.metadata.id == *id)
            .ok_or_else(|| SkillApplicationError::NotFound(id.as_str().to_string()))
    }

    fn effective_records(
        &self,
        location: &SkillLocation,
    ) -> Result<Vec<SkillRecord>, SkillApplicationError> {
        let Some(catalog) = &self.effective_catalog else {
            return self.repository.list(location);
        };
        if self.effective_package_reader.is_none() || self.system_materializer.is_none() {
            return self.repository.list(location);
        }
        let workspace = (location.scope == SkillScope::Workspace)
            .then_some(location.workspace_path.as_deref())
            .flatten();
        let existing = self
            .repository
            .list(location)?
            .into_iter()
            .map(|record| (record.key.id.clone(), record))
            .collect::<BTreeMap<_, _>>();
        let deleted = self
            .repository
            .deleted_builtin_ids()?
            .into_iter()
            .collect::<BTreeSet<_>>();
        let mut records = Vec::new();
        for effective in catalog.effective_catalog(workspace)? {
            if deleted.contains(&effective.effective.metadata.id) {
                continue;
            }
            let id = effective.effective.metadata.id.clone();
            records.push(self.record_for_effective(location, effective, existing.get(&id))?);
        }
        records.sort_by(|left, right| left.key.id.cmp(&right.key.id));
        self.attach_usage(location, &mut records);
        Ok(records)
    }

    fn attach_usage(&self, location: &SkillLocation, records: &mut [SkillRecord]) {
        let Some(repository) = &self.usage_repository else {
            return;
        };
        let identities = records
            .iter()
            .map(|record| SkillUsageIdentity {
                id: record.key.id.clone(),
                layer: record.effective_metadata().layer,
            })
            .collect::<Vec<_>>();
        let (project, non_project): (Vec<_>, Vec<_>) = identities
            .into_iter()
            .partition(|identity| identity.layer == SkillLayer::Project);
        let mut summaries = BTreeMap::new();
        let global_location = SkillLocation::new(SkillScope::Global, None)
            .expect("global Skill location is always valid");
        for (usage_location, requested) in [
            (location, project.as_slice()),
            (&global_location, non_project.as_slice()),
        ] {
            if requested.is_empty() {
                continue;
            }
            match repository.summaries(usage_location, requested) {
                Ok(read) => {
                    summaries.extend(read.summaries);
                    if read.recovered_corrupt_state {
                        self.record_usage_read_warning("corrupt-state-recovered");
                    }
                }
                Err(error) => self.record_usage_read_warning(usage_error_code(&error)),
            }
        }
        for record in records {
            let mut effective = record.effective_metadata();
            let identity = SkillUsageIdentity {
                id: record.key.id.clone(),
                layer: effective.layer,
            };
            effective.usage = summaries.get(&identity).cloned().unwrap_or_default();
            record.resolved_metadata = Some(effective);
        }
    }

    fn record_usage_read_warning(&self, reason: &str) {
        let mut context = BTreeMap::new();
        context.insert("reason".to_string(), reason.to_string());
        let _ = self.logging.record(&SkillLogEvent {
            action: SkillLogAction::TrackView,
            level: SkillLogLevel::Warn,
            skill_id: None,
            message: "Skill usage summaries are unavailable".to_string(),
            timestamp: self.clock.now(),
            context,
        });
    }

    fn resolve_progressive_package(
        &self,
        location: &SkillLocation,
        requested: &SkillId,
    ) -> Result<
        Result<(super::SkillPackageDescriptor, SkillRecord), SkillAccessRefusal>,
        SkillApplicationError,
    > {
        let catalog = self.effective_catalog.as_ref().ok_or_else(|| {
            SkillApplicationError::Validation("Effective Skill catalog is unavailable".into())
        })?;
        let records = self
            .effective_records(location)?
            .into_iter()
            .map(|record| (record.key.id.clone(), record))
            .collect::<BTreeMap<_, _>>();
        let candidates = records
            .values()
            .map(|record| SkillIdentityCandidate {
                id: record.key.id.clone(),
                aliases: record.metadata.aliases.clone(),
                availability: record.effective_metadata().availability,
            })
            .collect::<Vec<_>>();
        let resolved = match resolve_skill_identity(requested, &candidates) {
            SkillLookupOutcome::Resolved(id) => id,
            SkillLookupOutcome::Unavailable { id, availability } => {
                let record = records.get(&id);
                let reason = if record.is_some_and(|record| {
                    record.effective_metadata().skill_type == SkillType::Utility
                }) {
                    SkillAccessRefusalReason::UtilityNotLoadable
                } else {
                    refusal_for_availability(availability)
                };
                return Ok(Err(access_refusal(
                    requested.as_str(),
                    Some(id.as_str()),
                    reason,
                )));
            }
            SkillLookupOutcome::Ambiguous(ids) => {
                return Ok(Err(SkillAccessRefusal {
                    requested: requested.as_str().to_string(),
                    canonical_id: None,
                    reason: SkillAccessRefusalReason::AmbiguousAlias,
                    conflicting_ids: ids
                        .into_iter()
                        .take(8)
                        .map(|id| id.as_str().to_string())
                        .collect(),
                }));
            }
            SkillLookupOutcome::NotFound => {
                return Ok(Err(access_refusal(
                    requested.as_str(),
                    None,
                    SkillAccessRefusalReason::NotFound,
                )));
            }
        };
        let record = records
            .get(&resolved)
            .cloned()
            .ok_or_else(|| SkillApplicationError::NotFound(resolved.as_str().to_string()))?;
        if record.effective_metadata().skill_type == SkillType::Utility {
            return Ok(Err(access_refusal(
                requested.as_str(),
                Some(resolved.as_str()),
                SkillAccessRefusalReason::UtilityNotLoadable,
            )));
        }
        let workspace = (location.scope == SkillScope::Workspace)
            .then_some(location.workspace_path.as_deref())
            .flatten();
        let package = catalog
            .effective_catalog(workspace)?
            .into_iter()
            .find(|skill| skill.effective.metadata.id == resolved)
            .map(|skill| skill.effective)
            .ok_or_else(|| SkillApplicationError::NotFound(resolved.as_str().to_string()))?;
        Ok(Ok((package, record)))
    }

    fn resource_refusal(
        &self,
        requested: &str,
        package: &super::SkillPackageDescriptor,
        reason: SkillAccessRefusalReason,
        size: Option<u64>,
    ) -> SkillAccessRefusal {
        let refusal = access_refusal(
            bounded_requested(requested),
            Some(package.metadata.id.as_str()),
            reason,
        );
        self.record_access_refusal(
            SkillLogAction::ReadSkillResource,
            &refusal,
            Some(package.layer),
            size,
        );
        refusal
    }

    fn record_access_refusal(
        &self,
        action: SkillLogAction,
        refusal: &SkillAccessRefusal,
        layer: Option<SkillLayer>,
        size: Option<u64>,
    ) {
        let mut context = BTreeMap::new();
        context.insert("reason".to_string(), refusal.reason.as_str().to_string());
        if let Some(layer) = layer {
            context.insert("layer".to_string(), layer.as_str().to_string());
        }
        if let Some(size) = size {
            context.insert("sizeBytes".to_string(), size.to_string());
        }
        if !refusal.conflicting_ids.is_empty() {
            context.insert(
                "conflictCount".to_string(),
                refusal.conflicting_ids.len().to_string(),
            );
        }
        let _ = self.logging.record(&SkillLogEvent {
            action,
            level: SkillLogLevel::Warn,
            skill_id: refusal.canonical_id.clone(),
            message: "Refused progressive Skill access".to_string(),
            timestamp: self.clock.now(),
            context,
        });
    }

    fn record_for_effective(
        &self,
        location: &SkillLocation,
        effective: super::EffectiveSkill,
        existing: Option<&SkillRecord>,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let package = effective.effective;
        let enabled = existing.is_none_or(|record| record.enabled);
        let managed_source = if let Some(materializer) = &self.effective_materializer {
            materializer.materialize(&package)?
        } else {
            match package.layer {
                SkillLayer::System => match existing.filter(|record| {
                    record.source == SkillSource::Builtin
                        && record.managed_source.content_hash == package.revision
                }) {
                    Some(record) => record.managed_source.clone(),
                    None => self
                        .system_materializer
                        .as_ref()
                        .ok_or_else(|| {
                            SkillApplicationError::NotFound(
                                package.metadata.id.as_str().to_string(),
                            )
                        })?
                        .materialize(&package)?,
                },
                SkillLayer::Project | SkillLayer::User => {
                    let directory = package.source_path.as_deref().ok_or_else(|| {
                        SkillApplicationError::Filesystem(
                            "Effective filesystem Skill has no source".to_string(),
                        )
                    })?;
                    super::ManagedSkillSource {
                        skill_dir: directory.to_string(),
                        skill_md_path: std::path::Path::new(directory)
                            .join("SKILL.md")
                            .to_string_lossy()
                            .to_string(),
                        content_hash: existing
                            .filter(|record| record.effective_metadata().layer == package.layer)
                            .map(|record| record.managed_source.content_hash.clone())
                            .unwrap_or_else(|| package.revision.clone()),
                    }
                }
                SkillLayer::Registry => {
                    return Err(SkillApplicationError::Validation(
                        "Registry Skill materialization is unavailable".to_string(),
                    ))
                }
            }
        };
        let mut origin = package.origin;
        let reconciliation = self
            .reconciliation_repository
            .as_ref()
            .map(|repository| repository.builtin_reconciliation(&package.metadata.id))
            .transpose()?
            .flatten();
        if reconciliation.as_ref().is_some_and(|state| {
            state.outcome == BuiltinReconciliationOutcome::MigratedOverride
                && package.layer == SkillLayer::User
        }) {
            origin = SkillOrigin::Migrated;
        }
        let availability = if !enabled {
            SkillAvailability::Disabled
        } else if reconciliation
            .as_ref()
            .is_some_and(|state| state.outcome == BuiltinReconciliationOutcome::Invalid)
        {
            SkillAvailability::Invalid
        } else {
            package.availability
        };
        let source = match package.layer {
            SkillLayer::System => SkillSource::Builtin,
            _ if package.origin == SkillOrigin::Imported => SkillSource::Imported,
            _ => SkillSource::User,
        };
        let now = self.clock.now();
        Ok(SkillRecord {
            key: SkillKey::new(package.metadata.id.clone(), location.clone()),
            source,
            enabled,
            managed_source,
            metadata: package.metadata.clone(),
            bindings: existing
                .map(|record| record.bindings.clone())
                .unwrap_or_default(),
            created_at: existing
                .map(|record| record.created_at.clone())
                .unwrap_or_else(|| now.clone()),
            updated_at: existing
                .map(|record| record.updated_at.clone())
                .unwrap_or(now),
            resolved_metadata: Some(SkillEffectiveMetadata {
                layer: package.layer,
                origin,
                trust: package.trust,
                availability,
                skill_type: package.metadata.skill_type,
                delivery: package.metadata.delivery,
                compatibility_defaults: package.metadata.compatibility_defaults,
                immutable: !package.layer.content_is_mutable(),
                shadowed: effective
                    .shadowed
                    .into_iter()
                    .take(8)
                    .map(|shadowed| SkillShadowSummary {
                        layer: shadowed.layer,
                        origin: shadowed.origin,
                        version: shadowed.metadata.version,
                        availability: shadowed.availability,
                    })
                    .collect(),
                usage: SkillUsageSummary::default(),
                // Derived from the winning package rather than the record or any shadowed
                // revision, so a higher-priority Skill's schema is the one that counts.
                configuration: SkillConfigurationOverview::from_winning_metadata(
                    &package.metadata,
                    Some(package.revision.clone()),
                    package.workspace_path.is_some(),
                ),
            }),
        })
    }

    fn reconcile_system_builtins(&self) -> Result<(), SkillApplicationError> {
        let catalog = self
            .effective_catalog
            .as_ref()
            .expect("checked by system_reconciliation_ready")
            .effective_catalog(None)?;
        let mut packages = catalog
            .into_iter()
            .flat_map(|skill| std::iter::once(skill.effective).chain(skill.shadowed))
            .filter(|package| package.layer == SkillLayer::System)
            .collect::<Vec<_>>();
        packages.sort_by(|left, right| left.metadata.id.cmp(&right.metadata.id));
        packages.dedup_by(|left, right| left.metadata.id == right.metadata.id);

        let deleted = self
            .repository
            .deleted_builtin_ids()?
            .into_iter()
            .collect::<BTreeSet<_>>();
        let mut completed = 0_usize;
        let mut failed = Vec::new();
        for package in &packages {
            match self.reconcile_system_builtin(package, deleted.contains(&package.metadata.id)) {
                Ok(()) => completed += 1,
                Err(error) => {
                    let id = package.metadata.id.as_str().to_string();
                    self.record_seed_event(
                        SkillLogLevel::Error,
                        Some(id.clone()),
                        format!("System Skill reconciliation failed for {id}: {error}"),
                    );
                    failed.push(id);
                }
            }
        }
        let level = if failed.is_empty() {
            SkillLogLevel::Info
        } else {
            SkillLogLevel::Warn
        };
        self.record_seed_event(
            level,
            None,
            format!(
                "System Skill reconciliation completed {completed}/{}; failed: {}",
                packages.len(),
                if failed.is_empty() {
                    "none".to_string()
                } else {
                    failed.join(", ")
                }
            ),
        );
        if completed > 0 {
            self.invalidate_effective_catalog();
        }
        Ok(())
    }

    fn reconcile_system_builtin(
        &self,
        package: &super::SkillPackageDescriptor,
        deletion_intent: bool,
    ) -> Result<(), SkillApplicationError> {
        let repository = self
            .reconciliation_repository
            .as_ref()
            .expect("checked by system_reconciliation_ready");
        let location = SkillLocation::new(SkillScope::Global, None)?;
        let id = &package.metadata.id;
        let now = self.clock.now();
        let previous = repository.builtin_reconciliation(id)?;
        let key = SkillKey::new(id.clone(), location.clone());
        let existing = self.repository.get(&key)?;

        if deletion_intent {
            let state = reconciliation_state(
                package,
                BuiltinReconciliationOutcome::Deleted,
                BuiltinCleanupStatus::NotRequired,
                None,
                false,
                true,
                SkillLayer::System,
                SkillOrigin::Shipped,
                SkillAvailability::Disabled,
                now,
            );
            return repository.save_builtin_reconciliation(&state, None, false);
        }

        if let Some(state) = previous.as_ref().filter(|state| {
            state.reconciliation_version == BUILTIN_RECONCILIATION_VERSION
                && state.system_revision == package.revision
        }) {
            if state.outcome == BuiltinReconciliationOutcome::System
                && state.cleanup_status == BuiltinCleanupStatus::Pending
            {
                return self.complete_legacy_cleanup(&location, id);
            }
            if state.outcome == BuiltinReconciliationOutcome::Deleted
                || (state.outcome != BuiltinReconciliationOutcome::Invalid && existing.is_some())
            {
                return Ok(());
            }
        }

        let probe = self.filesystem.probe_source(&location, id)?;
        if let SkillSourceProbe::Unusable(reason) = &probe {
            let enabled = existing.as_ref().is_none_or(|record| record.enabled);
            let state = reconciliation_state(
                package,
                BuiltinReconciliationOutcome::Invalid,
                BuiltinCleanupStatus::NotRequired,
                None,
                enabled,
                false,
                SkillLayer::User,
                SkillOrigin::Migrated,
                SkillAvailability::Invalid,
                now,
            );
            repository.save_builtin_reconciliation(&state, None, false)?;
            return Err(SkillApplicationError::Filesystem(reason.clone()));
        }

        let preview = preview_package(
            package,
            self.system_package_reader
                .as_ref()
                .expect("checked by system_reconciliation_ready")
                .as_ref(),
        )?;
        let legacy_document = match &probe {
            SkillSourceProbe::Present(_) => Some(
                self.legacy_source
                    .as_ref()
                    .expect("checked by system_reconciliation_ready")
                    .read_legacy_document(&location, id)?,
            ),
            SkillSourceProbe::Absent => None,
            SkillSourceProbe::Unusable(_) => unreachable!("handled above"),
        };
        let equivalent = legacy_document
            .as_ref()
            .is_none_or(|legacy| documents_semantically_equal(legacy, &preview.document));

        if equivalent {
            let managed_source = self
                .system_materializer
                .as_ref()
                .expect("checked by system_reconciliation_ready")
                .materialize(package)?;
            let enabled = existing.as_ref().is_none_or(|record| record.enabled);
            let record = reconciled_record(
                existing.as_ref(),
                &location,
                package.metadata.clone(),
                managed_source,
                SkillSource::Builtin,
                enabled,
                &now,
            );
            let cleanup_status = if legacy_document.is_some() {
                BuiltinCleanupStatus::Pending
            } else {
                BuiltinCleanupStatus::NotRequired
            };
            let legacy_revision = match &probe {
                SkillSourceProbe::Present(source) => Some(source.source.content_hash.clone()),
                _ => None,
            };
            let state = reconciliation_state(
                package,
                BuiltinReconciliationOutcome::System,
                cleanup_status,
                legacy_revision,
                enabled,
                false,
                SkillLayer::System,
                SkillOrigin::Shipped,
                availability_for(enabled),
                now,
            );
            repository.save_builtin_reconciliation(&state, Some(&record), false)?;
            if cleanup_status == BuiltinCleanupStatus::Pending {
                self.complete_legacy_cleanup(&location, id)?;
            }
            return Ok(());
        }

        let adopted = match probe {
            SkillSourceProbe::Present(source) => *source,
            _ => unreachable!("a divergent document requires a present source"),
        };
        validate_update_identity(id, &adopted.metadata)?;
        let enabled = existing.as_ref().is_none_or(|record| record.enabled);
        let legacy_revision = adopted.source.content_hash.clone();
        let record = reconciled_record(
            existing.as_ref(),
            &location,
            adopted.metadata.clone(),
            adopted.source,
            SkillSource::User,
            enabled,
            &now,
        );
        let state = reconciliation_state(
            package,
            BuiltinReconciliationOutcome::MigratedOverride,
            BuiltinCleanupStatus::NotRequired,
            Some(legacy_revision),
            enabled,
            false,
            SkillLayer::User,
            SkillOrigin::Migrated,
            availability_for(enabled),
            now,
        );
        repository.save_builtin_reconciliation(&state, Some(&record), false)
    }

    fn complete_legacy_cleanup(
        &self,
        location: &SkillLocation,
        id: &SkillId,
    ) -> Result<(), SkillApplicationError> {
        let backup_path = self
            .legacy_source
            .as_ref()
            .expect("checked by system_reconciliation_ready")
            .archive_legacy_source(location, id, BUILTIN_RECONCILIATION_VERSION)?;
        self.reconciliation_repository
            .as_ref()
            .expect("checked by system_reconciliation_ready")
            .complete_builtin_cleanup(id, backup_path.as_deref(), &self.clock.now())
    }

    /// Reports what a seeding pass did. Each unusable source is named with the Skill it belongs to,
    /// so an investigation starts at the file that is actually broken rather than at the seeding
    /// code; the pass itself is then summarized once, at a level that reflects whether anything was
    /// actually left undone.
    fn report_builtin_seeding(
        &self,
        created: &[String],
        adopted: &[String],
        diverged: &[String],
        failed: &[(String, SkillApplicationError)],
    ) {
        for (id, error) in failed {
            self.record_seed_event(
                SkillLogLevel::Error,
                Some(id.clone()),
                format!("built-in Skill {id} could not be registered: {error}"),
            );
        }

        let mut summary = if created.is_empty() {
            "seeded no built-in Skills".to_string()
        } else {
            format!("created built-in Skills {}", created.join(", "))
        };
        if !adopted.is_empty() {
            summary.push_str(&format!(
                ", adopted existing sources for {}",
                adopted.join(", ")
            ));
        }
        // Drift compares a record against its own source, and an adopted record already matches
        // its source — so nothing downstream would ever mention that the adopted content is not
        // what shipped. Seeding is the only place that knows both, so it is the place that says so.
        if !diverged.is_empty() {
            summary.push_str(&format!(
                ", adopted content differs from the shipped definition for {}",
                diverged.join(", ")
            ));
        }
        if !failed.is_empty() {
            summary.push_str(&format!(
                ", left {} unregistered: {}",
                failed.len(),
                failed
                    .iter()
                    .map(|(id, _)| id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let level = if failed.is_empty() {
            SkillLogLevel::Info
        } else {
            SkillLogLevel::Warn
        };
        self.record_seed_event(level, None, summary);
    }

    /// Brings one built-in Skill's registry record in line with what is on disk.
    ///
    /// The registry answers "is it registered"; the filesystem answers "is it there". Seeding has
    /// to consult both, because a source present without a record is a recoverable state, not a
    /// failure — and it is not rare: sources live under the user's home while records live in the
    /// application database, so the two can diverge whenever those lifecycles differ.
    fn reconcile_builtin(
        &self,
        location: &SkillLocation,
        definition: BuiltinSkillDefinition,
        metadata: &SkillMetadata,
    ) -> Result<BuiltinSeedOutcome, SkillApplicationError> {
        let probe = self.filesystem.probe_source(location, &metadata.id)?;
        if let SkillSourceProbe::Unusable(reason) = probe {
            return Err(SkillApplicationError::Filesystem(reason));
        }

        let shipped = SkillDocument {
            metadata: metadata.clone(),
            body: definition.body.to_string(),
        };
        self.transact(|transaction| {
            let (adopted, outcome) = match &probe {
                // Adoption registers the file as it stands. Overwriting would silently destroy a
                // user's edits to fix a problem they did not cause, so the record describes disk
                // and the difference from the shipped definition is reported instead.
                SkillSourceProbe::Present(adopted) => {
                    let matches_definition =
                        adopted.source.content_hash == self.filesystem.content_hash_for(&shipped);
                    (
                        adopted.as_ref().clone(),
                        BuiltinSeedOutcome::Adopted { matches_definition },
                    )
                }
                SkillSourceProbe::Absent => (
                    SkillImportedSource {
                        source: self.filesystem.create_source(
                            transaction,
                            location,
                            &metadata.id,
                            &shipped,
                        )?,
                        metadata: metadata.clone(),
                    },
                    BuiltinSeedOutcome::Created,
                ),
                SkillSourceProbe::Unusable(_) => unreachable!("returned before the transaction"),
            };
            let record = self.record_for_adopted_source(
                location,
                &metadata.id,
                SkillSource::Builtin,
                adopted,
            )?;
            self.repository.save_skills(&[record], &[])?;
            Ok(outcome)
        })
    }

    /// Builds the registry record for a source that already exists on disk.
    ///
    /// The record describes the file rather than the definition that was expected there, so the
    /// registry never claims content the file does not have. A file whose frontmatter names a
    /// different Skill is refused rather than registered under a key it disagrees with.
    fn record_for_adopted_source(
        &self,
        location: &SkillLocation,
        id: &SkillId,
        source: SkillSource,
        adopted: SkillImportedSource,
    ) -> Result<SkillRecord, SkillApplicationError> {
        validate_update_identity(id, &adopted.metadata)?;
        let now = self.clock.now();
        Ok(SkillRecord {
            key: SkillKey::new(id.clone(), location.clone()),
            source,
            enabled: true,
            managed_source: adopted.source,
            metadata: adopted.metadata,
            bindings: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
            resolved_metadata: None,
        })
    }

    /// An already-present built-in is an expected state, so it is reported at info level. Logging
    /// it as an error trains readers to ignore the channel that carries real failures.
    fn record_seed_event(&self, level: SkillLogLevel, skill_id: Option<String>, message: String) {
        let _ = self.logging.record(&SkillLogEvent {
            level,
            action: SkillLogAction::SeedBuiltins,
            skill_id,
            message,
            timestamp: self.clock.now(),
            context: BTreeMap::new(),
        });
    }

    fn update_mount_path_work(
        &self,
        agent_id: &str,
        new_mount_path: SkillMountPath,
    ) -> Result<SkillMountMigrationReport, SkillApplicationError> {
        let configurations = self.repository.agent_mount_configurations()?;
        let configuration = configurations
            .iter()
            .find(|configuration| configuration.agent_id == agent_id)
            .ok_or_else(|| SkillDomainError::UnknownAgent(agent_id.to_string()))?;
        let old_mount_path = configuration
            .configured_path
            .clone()
            .map_or_else(|| SkillMountPath::parse(default_mount_path(agent_id)), Ok)?;
        let records = self
            .repository
            .enabled_skills_bound_to(agent_id)?
            .into_iter()
            .map(|record| self.load(&record.key))
            .collect::<Result<Vec<_>, _>>()?;
        self.transact(|transaction| {
            let mut report = SkillMountMigrationReport {
                agent_id: agent_id.to_string(),
                old_mount_path: old_mount_path.clone(),
                new_mount_path: new_mount_path.clone(),
                migrated: Vec::new(),
                removed: Vec::new(),
                overwritten: Vec::new(),
                backed_up: Vec::new(),
                failed: Vec::new(),
            };
            let mut updated_records = Vec::new();
            for mut record in records.clone() {
                match self.filesystem.migrate_binding(
                    transaction,
                    &record,
                    agent_id,
                    &old_mount_path,
                    &new_mount_path,
                ) {
                    Ok(repair) => {
                        apply_mount_repair(&mut record, repair.clone());
                        report.migrated.push(record.key.id.as_str().to_string());
                        if let Some(path) = repair.removed_path {
                            report.removed.push(path);
                        }
                        report.overwritten.extend(repair.overwritten);
                        report.backed_up.extend(repair.backed_up);
                        updated_records.push(record);
                    }
                    Err(error) => report.failed.push(SkillFailure {
                        skill_id: record.key.id.as_str().to_string(),
                        reason: error.to_string(),
                    }),
                }
            }
            self.repository.save_mount_path(
                agent_id,
                &new_mount_path,
                &updated_records,
                &self.clock.now(),
            )?;
            Ok(report)
        })
    }

    fn create_skill_work(
        &self,
        request: SkillCreateRequest,
    ) -> Result<SkillRecord, SkillApplicationError> {
        validate_create_identity(&request.id, &request.metadata)?;
        let source = source_for_user_create(request.source)?;
        let key = SkillKey::new(request.id.clone(), request.location.clone());
        if self.repository.get(&key)?.is_some() {
            return Err(SkillApplicationError::Conflict(
                request.id.as_str().to_string(),
            ));
        }
        let mount_paths = self.effective_mount_configurations()?;
        let plan = plan_binding_change(
            &[],
            &request.bound_agent_ids,
            &registered_agent_ids(&mount_paths),
            request.enabled,
        )?;
        self.transact(|transaction| {
            let managed_source = self.filesystem.create_source(
                transaction,
                &request.location,
                &request.id,
                &SkillDocument {
                    metadata: request.metadata.clone(),
                    body: request.body.clone(),
                },
            )?;
            let now = self.clock.now();
            let mut record = SkillRecord {
                key: key.clone(),
                source,
                enabled: request.enabled,
                managed_source,
                metadata: request.metadata.clone(),
                bindings: Vec::new(),
                created_at: now.clone(),
                updated_at: now,
                resolved_metadata: None,
            };
            record.bindings =
                self.filesystem
                    .reconcile_bindings(transaction, &record, &plan, &mount_paths)?;
            self.repository.save_skills(&[record.clone()], &[])?;
            Ok(record)
        })
    }

    fn update_skill_work(
        &self,
        request: SkillUpdateRequest,
    ) -> Result<SkillRecord, SkillApplicationError> {
        validate_update_identity(&request.key.id, &request.metadata)?;
        let mut record = self.load(&request.key)?;
        if record.effective_metadata().immutable {
            return Err(SkillApplicationError::ImmutablePackage(
                record.key.id.as_str().to_string(),
            ));
        }
        self.transact(|transaction| {
            record.managed_source = self.filesystem.replace_source(
                transaction,
                &record,
                &SkillDocument {
                    metadata: request.metadata.clone(),
                    body: request.body.clone(),
                },
                &request.expected_content_hash,
            )?;
            record.metadata = request.metadata.clone();
            record.updated_at = self.clock.now();
            self.repository.save_skills(&[record.clone()], &[])?;
            Ok(record.clone())
        })
    }

    fn delete_skill_work(&self, key: &SkillKey) -> Result<(), SkillApplicationError> {
        let record = self.load(key)?;
        let policy = deletion_policy(record.source);
        self.transact(|transaction| {
            if policy.remove_source || policy.remove_bindings {
                self.filesystem
                    .remove_skill(transaction, &record, policy.remove_source)?;
            }
            self.repository
                .delete_skill(key, policy.record_builtin_tombstone, &self.clock.now())
        })
    }

    fn restore_builtin_work(&self, id: &SkillId) -> Result<SkillRecord, SkillApplicationError> {
        let plan = builtin_restore_plan(id)?;
        if self
            .repository
            .get(&SkillKey::new(id.clone(), plan.location.clone()))?
            .is_some()
            || !self.repository.deleted_builtin_ids()?.contains(id)
        {
            return Err(SkillApplicationError::Validation(format!(
                "Built-in Skill is not eligible for restore: {}",
                id.as_str()
            )));
        }
        if self.system_reconciliation_ready() {
            return self.restore_system_builtin(id, &plan.location);
        }
        self.transact(|transaction| {
            let managed_source = self.filesystem.create_source(
                transaction,
                &plan.location,
                id,
                &SkillDocument {
                    metadata: plan.metadata.clone(),
                    body: plan.body.to_string(),
                },
            )?;
            let now = self.clock.now();
            let record = SkillRecord {
                key: SkillKey::new(id.clone(), plan.location.clone()),
                source: plan.source,
                enabled: plan.enabled,
                managed_source,
                metadata: plan.metadata.clone(),
                bindings: Vec::new(),
                created_at: now.clone(),
                updated_at: now,
                resolved_metadata: None,
            };
            self.repository
                .save_skills(std::slice::from_ref(&record), std::slice::from_ref(id))?;
            Ok(record)
        })
    }

    fn restore_system_builtin(
        &self,
        id: &SkillId,
        location: &SkillLocation,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let package = self.system_package(id)?;
        let managed_source = self
            .system_materializer
            .as_ref()
            .expect("checked by system_reconciliation_ready")
            .materialize(&package)?;
        let probe = self.filesystem.probe_source(location, id)?;
        let cleanup_status = if matches!(probe, SkillSourceProbe::Absent) {
            BuiltinCleanupStatus::NotRequired
        } else {
            BuiltinCleanupStatus::Pending
        };
        let legacy_revision = match probe {
            SkillSourceProbe::Present(source) => Some(source.source.content_hash),
            _ => None,
        };
        let now = self.clock.now();
        let record = reconciled_record(
            None,
            location,
            package.metadata.clone(),
            managed_source,
            SkillSource::Builtin,
            true,
            &now,
        );
        let state = reconciliation_state(
            &package,
            BuiltinReconciliationOutcome::System,
            cleanup_status,
            legacy_revision,
            true,
            false,
            SkillLayer::System,
            SkillOrigin::Shipped,
            SkillAvailability::Available,
            now,
        );
        self.reconciliation_repository
            .as_ref()
            .expect("checked by system_reconciliation_ready")
            .save_builtin_reconciliation(&state, Some(&record), true)?;
        if cleanup_status == BuiltinCleanupStatus::Pending {
            self.complete_legacy_cleanup(location, id)?;
        }
        self.invalidate_effective_catalog();
        self.load(&record.key)
    }

    fn set_enabled_work(
        &self,
        key: &SkillKey,
        enabled: bool,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let mut record = self.load(key)?;
        let mount_paths = self.effective_mount_configurations()?;
        let plan = plan_enablement(&record.bound_agent_ids(), enabled);
        self.transact(|transaction| {
            record.enabled = enabled;
            if let Some(effective) = &mut record.resolved_metadata {
                effective.availability = if !enabled {
                    SkillAvailability::Disabled
                } else if effective.skill_type == SkillType::Utility {
                    SkillAvailability::Unsupported
                } else {
                    SkillAvailability::Available
                };
            }
            record.updated_at = self.clock.now();
            record.bindings =
                self.filesystem
                    .reconcile_bindings(transaction, &record, &plan, &mount_paths)?;
            self.repository.save_skills(&[record.clone()], &[])?;
            Ok(record.clone())
        })
    }

    fn set_bindings_work(
        &self,
        key: &SkillKey,
        agent_ids: Vec<String>,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let mut record = self.load(key)?;
        let mount_paths = self.effective_mount_configurations()?;
        let plan = plan_binding_change(
            &record.bound_agent_ids(),
            &agent_ids,
            &registered_agent_ids(&mount_paths),
            record.enabled,
        )?;
        self.transact(|transaction| {
            record.updated_at = self.clock.now();
            record.bindings =
                self.filesystem
                    .reconcile_bindings(transaction, &record, &plan, &mount_paths)?;
            self.repository.save_skills(&[record.clone()], &[])?;
            Ok(record.clone())
        })
    }

    fn change_cli_binding(
        &self,
        key: &SkillKey,
        agent_id: &str,
        bind: bool,
    ) -> Result<SkillRecord, SkillApplicationError> {
        self.transact(|transaction| {
            let mut record = self.load(key)?;
            let mount_paths = self.effective_mount_configurations()?;
            if !registered_agent_ids(&mount_paths).contains(agent_id) {
                return Err(SkillDomainError::UnknownAgent(agent_id.to_string()).into());
            }
            let mut desired = record
                .bound_agent_ids()
                .into_iter()
                .collect::<BTreeSet<_>>();
            if bind {
                desired.insert(agent_id.to_string());
            } else {
                desired.remove(agent_id);
            }
            let desired = desired.into_iter().collect::<Vec<_>>();
            let plan = plan_binding_change(
                &record.bound_agent_ids(),
                &desired,
                &registered_agent_ids(&mount_paths),
                record.enabled,
            )?;
            record.updated_at = self.clock.now();
            record.bindings =
                self.filesystem
                    .reconcile_bindings(transaction, &record, &plan, &mount_paths)?;
            self.repository.save_skills(&[record.clone()], &[])?;
            Ok(record)
        })
    }

    fn import_skill_work(
        &self,
        request: SkillImportRequest,
    ) -> Result<SkillRecord, SkillApplicationError> {
        let inspected = self
            .filesystem
            .inspect_import_metadata(&request.source_path)?;
        let inspected_key = SkillKey::new(inspected.id.clone(), request.location.clone());
        if let Some(existing) = self.repository.get(&inspected_key)? {
            if existing.effective_metadata().immutable {
                return Err(SkillApplicationError::ImmutablePackage(
                    inspected.id.as_str().to_string(),
                ));
            }
            return Err(SkillApplicationError::Conflict(
                inspected.id.as_str().to_string(),
            ));
        }
        let mount_paths = self.effective_mount_configurations()?;
        self.transact(|transaction| {
            let imported = self.filesystem.import_source(
                transaction,
                &request.location,
                &request.source_path,
            )?;
            let key = SkillKey::new(imported.metadata.id.clone(), request.location.clone());
            if self.repository.get(&key)?.is_some() {
                return Err(SkillApplicationError::Conflict(key.id.as_str().to_string()));
            }
            let plan = plan_binding_change(
                &[],
                &request.bound_agent_ids,
                &registered_agent_ids(&mount_paths),
                request.enabled,
            )?;
            let now = self.clock.now();
            let mut record = SkillRecord {
                key,
                source: SkillSource::Imported,
                enabled: request.enabled,
                managed_source: imported.source,
                metadata: imported.metadata,
                bindings: Vec::new(),
                created_at: now.clone(),
                updated_at: now,
                resolved_metadata: None,
            };
            record.bindings =
                self.filesystem
                    .reconcile_bindings(transaction, &record, &plan, &mount_paths)?;
            self.repository.save_skills(&[record.clone()], &[])?;
            Ok(record)
        })
    }

    fn detect_skill_drift_work(
        &self,
        location: &SkillLocation,
    ) -> Result<SkillDriftReport, SkillApplicationError> {
        self.ensure_builtins()?;
        let records = self.effective_records(location)?;
        let deleted = self.repository.deleted_builtin_ids()?;
        let inspection = self
            .filesystem
            .inspect_drift(location, &records, &deleted)?;
        let issues = detect_drift(&inspection);
        let report = SkillDriftReport {
            location: location.clone(),
            drift_hash: drift_hash(&issues),
            issues,
        };
        self.repository.save_drift_snapshot(&report)?;
        Ok(report)
    }

    fn sync_skill_drift_work(
        &self,
        location: SkillLocation,
    ) -> Result<SkillSyncResult, SkillApplicationError> {
        let report = self.detect_skill_drift_work(&location)?;
        let mount_paths = self.effective_mount_configurations()?;
        let records = self
            .effective_records(&location)?
            .into_iter()
            .map(|record| (record.key.clone(), record))
            .collect::<BTreeMap<_, _>>();
        let deleted_builtins = self
            .repository
            .deleted_builtin_ids()?
            .into_iter()
            .collect::<BTreeSet<_>>();
        self.transact(|transaction| {
            let mut changed = BTreeMap::new();
            let cleared_tombstones = Vec::new();
            let mut result = SkillSyncResult {
                mounted: Vec::new(),
                unmounted: Vec::new(),
                overwritten: Vec::new(),
                backed_up: Vec::new(),
                restored: Vec::new(),
                failed: Vec::new(),
                resolved_from: report.clone(),
            };

            for issue in &report.issues {
                match issue.issue_type {
                    SkillDriftIssueType::MissingMount | SkillDriftIssueType::Conflict => {
                        let repair = (|| {
                            let agent_id = issue.agent_id.as_deref().ok_or_else(|| {
                                SkillApplicationError::Filesystem(
                                    "Drift issue is missing its Agent id".to_string(),
                                )
                            })?;
                            let key =
                                SkillKey::new(SkillId::parse(&issue.skill_id)?, location.clone());
                            let mut record = changed
                                .get(&key)
                                .or_else(|| records.get(&key))
                                .cloned()
                                .ok_or_else(|| {
                                    SkillApplicationError::NotFound(issue.skill_id.clone())
                                })?;
                            let mount_path = mount_path_for_agent(&mount_paths, agent_id)?;
                            let repair = self.filesystem.repair_binding(
                                transaction,
                                &record,
                                agent_id,
                                &mount_path,
                            )?;
                            apply_mount_repair(&mut record, repair.clone());
                            Ok::<_, SkillApplicationError>((key, record, repair))
                        })();
                        match repair {
                            Ok((key, record, repair)) => {
                                result.mounted.push(issue.skill_id.clone());
                                result.overwritten.extend(repair.overwritten);
                                result.backed_up.extend(repair.backed_up);
                                changed.insert(key, record);
                            }
                            Err(error) => result.failed.push(SkillFailure {
                                skill_id: issue.skill_id.clone(),
                                reason: error.to_string(),
                            }),
                        }
                    }
                    SkillDriftIssueType::MetadataChanged => {
                        let refresh = (|| {
                            let key =
                                SkillKey::new(SkillId::parse(&issue.skill_id)?, location.clone());
                            let mut record = changed
                                .get(&key)
                                .or_else(|| records.get(&key))
                                .cloned()
                                .ok_or_else(|| {
                                    SkillApplicationError::NotFound(issue.skill_id.clone())
                                })?;
                            if record.source == SkillSource::Builtin
                                && self.system_reconciliation_ready()
                            {
                                let package = self.system_package(&record.key.id)?;
                                record.managed_source = self
                                    .system_materializer
                                    .as_ref()
                                    .expect("checked by system_reconciliation_ready")
                                    .materialize(&package)?;
                                record.metadata = package.metadata;
                            } else {
                                let refreshed = self.filesystem.refresh_source(&record, issue)?;
                                validate_update_identity(&record.key.id, &refreshed.metadata)?;
                                record.metadata = refreshed.metadata;
                                record.managed_source.content_hash = refreshed.content_hash;
                            }
                            record.updated_at = self.clock.now();
                            Ok::<_, SkillApplicationError>((key, record))
                        })();
                        match refresh {
                            Ok((key, record)) => {
                                result.restored.push(issue.skill_id.clone());
                                changed.insert(key, record);
                            }
                            Err(error) => result.failed.push(SkillFailure {
                                skill_id: issue.skill_id.clone(),
                                reason: error.to_string(),
                            }),
                        }
                    }
                    // A source without a record is the same recoverable state seeding handles, so
                    // synchronization resolves it the same way. Leaving it as a no-op reported an
                    // issue the user had no action available for.
                    SkillDriftIssueType::UnregisteredSource => {
                        let adoption = (|| {
                            let id = SkillId::parse(&issue.skill_id)?;
                            // An intentional deletion still wins: a source on disk must not
                            // resurrect a built-in the user removed.
                            if deleted_builtins.contains(&id) {
                                return Ok(None);
                            }
                            let source = match self.filesystem.probe_source(&location, &id)? {
                                SkillSourceProbe::Present(source) => *source,
                                SkillSourceProbe::Absent => {
                                    return Err(SkillApplicationError::NotFound(
                                        issue.skill_id.clone(),
                                    ))
                                }
                                SkillSourceProbe::Unusable(reason) => {
                                    return Err(SkillApplicationError::Filesystem(reason))
                                }
                            };
                            let origin = if builtin_definition(&id).is_some() {
                                SkillSource::Builtin
                            } else {
                                SkillSource::User
                            };
                            let record =
                                self.record_for_adopted_source(&location, &id, origin, source)?;
                            Ok::<_, SkillApplicationError>(Some((record.key.clone(), record)))
                        })();
                        match adoption {
                            Ok(Some((key, record))) => {
                                result.restored.push(issue.skill_id.clone());
                                changed.insert(key, record);
                            }
                            Ok(None) => {}
                            Err(error) => result.failed.push(SkillFailure {
                                skill_id: issue.skill_id.clone(),
                                reason: error.to_string(),
                            }),
                        }
                    }
                    SkillDriftIssueType::DeletedBuiltin => {}
                    SkillDriftIssueType::MissingSource => {
                        let repair = (|| {
                            let key =
                                SkillKey::new(SkillId::parse(&issue.skill_id)?, location.clone());
                            let mut record = changed
                                .get(&key)
                                .or_else(|| records.get(&key))
                                .cloned()
                                .ok_or_else(|| {
                                    SkillApplicationError::NotFound(issue.skill_id.clone())
                                })?;
                            if record.source != SkillSource::Builtin
                                || !self.system_reconciliation_ready()
                            {
                                return Ok(None);
                            }
                            let package = self.system_package(&record.key.id)?;
                            record.managed_source = self
                                .system_materializer
                                .as_ref()
                                .expect("checked by system_reconciliation_ready")
                                .materialize(&package)?;
                            record.metadata = package.metadata;
                            record.updated_at = self.clock.now();
                            Ok::<_, SkillApplicationError>(Some((key, record)))
                        })();
                        match repair {
                            Ok(Some((key, record))) => {
                                result.restored.push(issue.skill_id.clone());
                                changed.insert(key, record);
                            }
                            Ok(None) => {}
                            Err(error) => result.failed.push(SkillFailure {
                                skill_id: issue.skill_id.clone(),
                                reason: error.to_string(),
                            }),
                        }
                    }
                }
            }

            self.repository.save_synchronization(
                &changed.into_values().collect::<Vec<_>>(),
                &cleared_tombstones,
                &report,
            )?;
            Ok(result)
        })
    }

    fn effective_mount_configurations(
        &self,
    ) -> Result<Vec<AgentMountConfiguration>, SkillApplicationError> {
        self.repository
            .agent_mount_configurations()?
            .into_iter()
            .map(|configuration| {
                let path = configuration.configured_path.map_or_else(
                    || SkillMountPath::parse(default_mount_path(&configuration.agent_id)),
                    Ok,
                )?;
                Ok(AgentMountConfiguration {
                    agent_id: configuration.agent_id,
                    configured_path: Some(path),
                })
            })
            .collect()
    }

    fn load(&self, key: &SkillKey) -> Result<SkillRecord, SkillApplicationError> {
        if self.effective_catalog.is_some()
            && self.effective_package_reader.is_some()
            && self.system_materializer.is_some()
        {
            return self
                .effective_records(&key.location)?
                .into_iter()
                .find(|record| record.key.id == key.id)
                .ok_or_else(|| SkillApplicationError::NotFound(key.id.as_str().to_string()));
        }
        self.repository
            .get(key)?
            .ok_or_else(|| SkillApplicationError::NotFound(key.id.as_str().to_string()))
    }

    fn transact<T>(
        &self,
        work: impl FnOnce(&SkillFilesystemTransaction) -> Result<T, SkillApplicationError>,
    ) -> Result<T, SkillApplicationError> {
        let _guard = self.mutation_coordinator.lock().map_err(|error| {
            SkillApplicationError::Filesystem(format!(
                "Skill mutation coordinator is unavailable: {error}"
            ))
        })?;
        let transaction = self.filesystem.begin_mutation()?;
        match work(&transaction) {
            Ok(value) => {
                self.filesystem.commit_mutation(transaction);
                Ok(value)
            }
            Err(error) => {
                self.filesystem.rollback_mutation(transaction);
                Err(error)
            }
        }
    }

    fn observe<T>(
        &self,
        action: SkillLogAction,
        skill_id: Option<String>,
        result: Result<T, SkillApplicationError>,
    ) -> Result<T, SkillApplicationError> {
        self.observe_with_level(action, skill_id, SkillLogLevel::Info, result)
    }

    fn observe_for_agent<T>(
        &self,
        action: SkillLogAction,
        skill_id: Option<String>,
        agent_id: &str,
        result: Result<T, SkillApplicationError>,
    ) -> Result<T, SkillApplicationError> {
        self.observe_with_context(
            action,
            skill_id,
            SkillLogLevel::Info,
            BTreeMap::from([("agentId".to_string(), agent_id.to_string())]),
            result,
        )
    }

    fn observe_with_level<T>(
        &self,
        action: SkillLogAction,
        skill_id: Option<String>,
        success_level: SkillLogLevel,
        result: Result<T, SkillApplicationError>,
    ) -> Result<T, SkillApplicationError> {
        self.observe_with_context(action, skill_id, success_level, BTreeMap::new(), result)
    }

    fn observe_with_context<T>(
        &self,
        action: SkillLogAction,
        skill_id: Option<String>,
        success_level: SkillLogLevel,
        context: BTreeMap<String, String>,
        result: Result<T, SkillApplicationError>,
    ) -> Result<T, SkillApplicationError> {
        let (level, message) = match &result {
            Ok(_) => (
                success_level,
                format!("Skill {} completed", action.as_str()),
            ),
            Err(error) => (SkillLogLevel::Error, error.to_string()),
        };
        let _ = self.logging.record(&SkillLogEvent {
            action,
            level,
            skill_id,
            message,
            timestamp: self.clock.now(),
            context,
        });
        if result.is_ok() && action.invalidates_effective_catalog() {
            self.invalidate_effective_catalog();
        }
        result
    }

    fn invalidate_effective_catalog(&self) {
        if let Some(catalog) = &self.effective_catalog {
            catalog.invalidate(None);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn reconciliation_state(
    package: &super::SkillPackageDescriptor,
    outcome: BuiltinReconciliationOutcome,
    cleanup_status: BuiltinCleanupStatus,
    legacy_revision: Option<String>,
    enabled: bool,
    deletion_intent: bool,
    effective_layer: SkillLayer,
    origin: SkillOrigin,
    availability: SkillAvailability,
    updated_at: String,
) -> BuiltinReconciliationState {
    BuiltinReconciliationState {
        skill_id: package.metadata.id.clone(),
        reconciliation_version: BUILTIN_RECONCILIATION_VERSION,
        outcome,
        system_revision: package.revision.clone(),
        legacy_revision,
        cleanup_status,
        backup_path: None,
        error_code: (outcome == BuiltinReconciliationOutcome::Invalid)
            .then(|| "legacy-source-invalid".to_string()),
        enabled,
        deletion_intent,
        effective_layer,
        origin,
        availability,
        updated_at,
    }
}

fn reconciled_record(
    existing: Option<&SkillRecord>,
    location: &SkillLocation,
    metadata: SkillMetadata,
    managed_source: super::ManagedSkillSource,
    source: SkillSource,
    enabled: bool,
    now: &str,
) -> SkillRecord {
    SkillRecord {
        key: SkillKey::new(metadata.id.clone(), location.clone()),
        source,
        enabled,
        managed_source,
        metadata,
        bindings: existing
            .map(|record| record.bindings.clone())
            .unwrap_or_default(),
        created_at: existing
            .map(|record| record.created_at.clone())
            .unwrap_or_else(|| now.to_string()),
        updated_at: now.to_string(),
        resolved_metadata: None,
    }
}

fn availability_for(enabled: bool) -> SkillAvailability {
    if enabled {
        SkillAvailability::Available
    } else {
        SkillAvailability::Disabled
    }
}

fn usage_error_code(error: &SkillApplicationError) -> &'static str {
    match error {
        SkillApplicationError::ConcurrentModification(_) => "concurrent-modification",
        SkillApplicationError::Filesystem(_) => "filesystem",
        SkillApplicationError::Validation(_) => "validation",
        _ => "tracking-unavailable",
    }
}

fn progressive_location(
    workspace_path: Option<&str>,
) -> Result<SkillLocation, SkillApplicationError> {
    match workspace_path {
        Some(path) => SkillLocation::new(SkillScope::Workspace, Some(path)).map_err(Into::into),
        None => SkillLocation::new(SkillScope::Global, None).map_err(Into::into),
    }
}

fn refusal_for_availability(availability: SkillAvailability) -> SkillAccessRefusalReason {
    match availability {
        SkillAvailability::Available => SkillAccessRefusalReason::Unreadable,
        SkillAvailability::Disabled => SkillAccessRefusalReason::Disabled,
        SkillAvailability::Invalid => SkillAccessRefusalReason::Invalid,
        SkillAvailability::Conflicting => SkillAccessRefusalReason::Conflicting,
        SkillAvailability::Unsupported => SkillAccessRefusalReason::Unsupported,
    }
}

fn utility_resolution_availability(record: &SkillRecord) -> SkillAvailability {
    let metadata = record.effective_metadata();
    if !record.enabled {
        return SkillAvailability::Disabled;
    }
    if metadata.skill_type != SkillType::Utility || metadata.trust != SkillTrust::Trusted {
        return SkillAvailability::Unsupported;
    }
    match metadata.availability {
        SkillAvailability::Unsupported => SkillAvailability::Available,
        availability => availability,
    }
}

fn resource_error_reason(error: &SkillApplicationError) -> SkillAccessRefusalReason {
    match error {
        SkillApplicationError::InvalidResourceUri => SkillAccessRefusalReason::InvalidUri,
        SkillApplicationError::ResourceEscape => SkillAccessRefusalReason::EscapingResource,
        SkillApplicationError::BinaryResource => SkillAccessRefusalReason::BinaryResource,
        SkillApplicationError::OversizedResource => SkillAccessRefusalReason::OversizedResource,
        SkillApplicationError::ConcurrentModification(_) => SkillAccessRefusalReason::StaleRevision,
        _ => SkillAccessRefusalReason::Unreadable,
    }
}

fn access_refusal(
    requested: impl Into<String>,
    canonical_id: Option<&str>,
    reason: SkillAccessRefusalReason,
) -> SkillAccessRefusal {
    SkillAccessRefusal {
        requested: bounded_requested(&requested.into()),
        canonical_id: canonical_id.map(str::to_string),
        reason,
        conflicting_ids: Vec::new(),
    }
}

fn bounded_requested(requested: &str) -> String {
    requested.chars().take(512).collect()
}

fn documents_semantically_equal(left: &SkillDocument, right: &SkillDocument) -> bool {
    left.body.trim() == right.body.trim()
        && left.metadata.id == right.metadata.id
        && left.metadata.name == right.metadata.name
        && left.metadata.description == right.metadata.description
        && left.metadata.category == right.metadata.category
        && left.metadata.version == right.metadata.version
        && left.metadata.triggers == right.metadata.triggers
        && left.metadata.aliases == right.metadata.aliases
        && left.metadata.skill_type == right.metadata.skill_type
        && left.metadata.delivery == right.metadata.delivery
}

/// Strips a SKILL.md file's YAML frontmatter block, returning everything after the closing
/// `---` — the instructional body suitable for system-prompt injection, without the id/name/
/// description/category/version/triggers metadata the frontmatter carries for tooling. Falls
/// back to the full trimmed content if no frontmatter block is found, mirroring
/// `infrastructure::filesystem::document::parse`'s own frontmatter-detection technique.
fn strip_frontmatter(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n");
    let body = normalized
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---"))
        .map(|(_, remainder)| remainder);
    body.unwrap_or(normalized.as_str()).trim().to_string()
}

fn registered_agent_ids(configurations: &[AgentMountConfiguration]) -> BTreeSet<String> {
    configurations
        .iter()
        .map(|configuration| configuration.agent_id.clone())
        .collect()
}

fn skill_stats(skills: &[SkillRecord]) -> SkillStats {
    SkillStats {
        total: skills.len(),
        enabled: skills.iter().filter(|skill| skill.enabled).count(),
        mounted: skills
            .iter()
            .filter(|skill| skill.bindings.iter().any(|binding| binding.mounted))
            .count(),
    }
}

fn mount_path_for_agent(
    configurations: &[AgentMountConfiguration],
    agent_id: &str,
) -> Result<SkillMountPath, SkillApplicationError> {
    configurations
        .iter()
        .find(|configuration| configuration.agent_id == agent_id)
        .and_then(|configuration| configuration.configured_path.clone())
        .ok_or_else(|| SkillDomainError::UnknownAgent(agent_id.to_string()).into())
}

fn apply_mount_repair(record: &mut SkillRecord, repair: SkillMountRepair) {
    if let Some(existing) = record
        .bindings
        .iter_mut()
        .find(|binding| binding.agent_id == repair.binding.agent_id)
    {
        *existing = repair.binding;
    } else {
        record.bindings.push(repair.binding);
        record
            .bindings
            .sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
    }
}

fn drift_hash(issues: &[crate::contexts::tooling::skills::domain::SkillDriftIssue]) -> String {
    let mut hasher = DefaultHasher::new();
    for issue in issues {
        format!(
            "{:?}|{}|{:?}|{:?}|{}",
            issue.issue_type, issue.skill_id, issue.agent_id, issue.path, issue.message
        )
        .hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}
