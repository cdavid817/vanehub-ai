use super::{
    AgentMountConfiguration, SkillAgentMountPath, SkillApplicationError, SkillClockPort,
    SkillCreateRequest, SkillDocument, SkillDriftReport, SkillFailure, SkillFilesystemPort,
    SkillFilesystemTransaction, SkillImportRequest, SkillListResult, SkillLogAction, SkillLogEvent,
    SkillLogLevel, SkillLoggingPort, SkillMountMigrationReport, SkillMountRepair, SkillPreview,
    SkillRecord, SkillRepository, SkillScopeQuery, SkillStats, SkillSyncResult, SkillUpdateRequest,
    SkillWorkspaceSelectionPort,
};
use crate::contexts::tooling::skills::domain::{
    builtin_definitions, builtin_restore_plan, default_mount_path, deletion_policy, detect_drift,
    plan_binding_change, plan_enablement, source_for_user_create, validate_create_identity,
    validate_update_identity, SkillDomainError, SkillDriftIssueType, SkillId, SkillKey,
    SkillLocation, SkillMountPath, SkillScope, SkillSource,
};
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SkillApplicationService {
    repository: Arc<dyn SkillRepository>,
    filesystem: Arc<dyn SkillFilesystemPort>,
    selection: Arc<dyn SkillWorkspaceSelectionPort>,
    clock: Arc<dyn SkillClockPort>,
    logging: Arc<dyn SkillLoggingPort>,
}

impl SkillApplicationService {
    pub(crate) fn new(
        repository: Arc<dyn SkillRepository>,
        filesystem: Arc<dyn SkillFilesystemPort>,
        selection: Arc<dyn SkillWorkspaceSelectionPort>,
        clock: Arc<dyn SkillClockPort>,
        logging: Arc<dyn SkillLoggingPort>,
    ) -> Self {
        Self {
            repository,
            filesystem,
            selection,
            clock,
            logging,
        }
    }

    pub(crate) fn list_skills(
        &self,
        query: SkillScopeQuery,
    ) -> Result<SkillListResult, SkillApplicationError> {
        self.ensure_builtins()?;
        let mut skills = self.repository.list(&query.location)?;
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

    pub(crate) fn preview_skill(
        &self,
        key: SkillKey,
    ) -> Result<SkillPreview, SkillApplicationError> {
        let record = self.load(&key)?;
        Ok(SkillPreview {
            key,
            content: self.filesystem.read_source(&record)?,
            path: record.managed_source.skill_md_path,
        })
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

        let result = self.transact(|transaction| {
            let mut records = Vec::with_capacity(missing.len());
            for (definition, metadata) in &missing {
                let managed_source = self.filesystem.create_source(
                    transaction,
                    &location,
                    &metadata.id,
                    &SkillDocument {
                        metadata: metadata.clone(),
                        body: definition.body.to_string(),
                    },
                )?;
                let now = self.clock.now();
                records.push(SkillRecord {
                    key: SkillKey::new(metadata.id.clone(), location.clone()),
                    source: SkillSource::Builtin,
                    enabled: true,
                    managed_source,
                    metadata: metadata.clone(),
                    bindings: Vec::new(),
                    created_at: now.clone(),
                    updated_at: now,
                });
            }
            self.repository.save_skills(&records, &[])
        });
        self.observe(SkillLogAction::SeedBuiltins, None, result)
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
        let records = self.repository.enabled_skills_bound_to(agent_id)?;
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
        let mount_paths = self.effective_mount_configurations()?;
        let plan = plan_binding_change(
            &record.bound_agent_ids(),
            &request.bound_agent_ids,
            &registered_agent_ids(&mount_paths),
            request.enabled,
        )?;
        self.transact(|transaction| {
            record.managed_source = self.filesystem.replace_source(
                transaction,
                &record,
                &SkillDocument {
                    metadata: request.metadata.clone(),
                    body: request.body.clone(),
                },
            )?;
            record.metadata = request.metadata.clone();
            record.enabled = request.enabled;
            record.updated_at = self.clock.now();
            record.bindings =
                self.filesystem
                    .reconcile_bindings(transaction, &record, &plan, &mount_paths)?;
            self.repository.save_skills(&[record.clone()], &[])?;
            Ok(record.clone())
        })
    }

    fn delete_skill_work(&self, key: &SkillKey) -> Result<(), SkillApplicationError> {
        let record = self.load(key)?;
        let policy = deletion_policy(record.source);
        self.transact(|transaction| {
            if policy.remove_source || policy.remove_bindings {
                self.filesystem.remove_skill(transaction, &record)?;
            }
            self.repository
                .delete_skill(key, policy.record_builtin_tombstone, &self.clock.now())
        })
    }

    fn restore_builtin_work(&self, id: &SkillId) -> Result<SkillRecord, SkillApplicationError> {
        let plan = builtin_restore_plan(id)?;
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
            };
            self.repository
                .save_skills(std::slice::from_ref(&record), std::slice::from_ref(id))?;
            Ok(record)
        })
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

    fn import_skill_work(
        &self,
        request: SkillImportRequest,
    ) -> Result<SkillRecord, SkillApplicationError> {
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
        let records = self.repository.list(location)?;
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
            .repository
            .list(&location)?
            .into_iter()
            .map(|record| (record.key.clone(), record))
            .collect::<BTreeMap<_, _>>();
        self.transact(|transaction| {
            let mut changed = BTreeMap::new();
            let mut cleared_tombstones = Vec::new();
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
                            let refreshed = self.filesystem.refresh_source(&record, issue)?;
                            record.metadata = refreshed.metadata;
                            record.managed_source.content_hash = refreshed.content_hash;
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
                    SkillDriftIssueType::DeletedBuiltin => {
                        let restore = (|| {
                            let id = SkillId::parse(&issue.skill_id)?;
                            let plan = builtin_restore_plan(&id)?;
                            let managed_source = self.filesystem.create_source(
                                transaction,
                                &plan.location,
                                &id,
                                &SkillDocument {
                                    metadata: plan.metadata.clone(),
                                    body: plan.body.to_string(),
                                },
                            )?;
                            let now = self.clock.now();
                            let record = SkillRecord {
                                key: SkillKey::new(id.clone(), plan.location),
                                source: plan.source,
                                enabled: plan.enabled,
                                managed_source,
                                metadata: plan.metadata,
                                bindings: Vec::new(),
                                created_at: now.clone(),
                                updated_at: now,
                            };
                            Ok::<_, SkillApplicationError>((id, record))
                        })();
                        match restore {
                            Ok((id, record)) => {
                                result.restored.push(issue.skill_id.clone());
                                changed.insert(record.key.clone(), record);
                                cleared_tombstones.push(id);
                            }
                            Err(error) => result.failed.push(SkillFailure {
                                skill_id: issue.skill_id.clone(),
                                reason: error.to_string(),
                            }),
                        }
                    }
                    SkillDriftIssueType::MissingSource
                    | SkillDriftIssueType::UnregisteredSource => {}
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
        self.repository
            .get(key)?
            .ok_or_else(|| SkillApplicationError::NotFound(key.id.as_str().to_string()))
    }

    fn transact<T>(
        &self,
        work: impl FnOnce(&SkillFilesystemTransaction) -> Result<T, SkillApplicationError>,
    ) -> Result<T, SkillApplicationError> {
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

    fn observe_with_level<T>(
        &self,
        action: SkillLogAction,
        skill_id: Option<String>,
        success_level: SkillLogLevel,
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
            context: BTreeMap::new(),
        });
        result
    }
}

fn registered_agent_ids(configurations: &[AgentMountConfiguration]) -> BTreeSet<String> {
    configurations
        .iter()
        .map(|configuration| configuration.agent_id.clone())
        .collect()
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
