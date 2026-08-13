use super::*;
use crate::contexts::tooling::skills::domain::{
    builtin_definitions, replay_overlay_scope_chain, BaseSkillResource, OverlayBaseWitness,
    OverlayDocument, OverlayFile, OverlayPatch, OverlayScope, OverlayScopeReplayInput,
    OverlayTrust, RegisteredSkillInspection, SkillAvailability, SkillBindingInspection,
    SkillBindingPlan, SkillDelivery, SkillDomainError, SkillDriftInspection, SkillDriftIssue,
    SkillDriftIssueType, SkillId, SkillKey, SkillLayer, SkillLocation, SkillMetadata,
    SkillMountObservation, SkillMountPath, SkillOrigin, SkillScope, SkillSource,
    SkillSourceInspection, SkillTrust, SkillType, UnregisteredSkillInspection,
};
use crate::test_support::TempDirectory;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

struct RepositoryState {
    records: BTreeMap<SkillKey, SkillRecord>,
    deleted_builtin_ids: BTreeSet<SkillId>,
    mount_configurations: Vec<AgentMountConfiguration>,
    drift_snapshots: Vec<SkillDriftReport>,
    synchronization_count: usize,
    api_agent_bindings: BTreeSet<(SkillKey, String)>,
    api_agents: BTreeSet<String>,
}

impl Default for RepositoryState {
    fn default() -> Self {
        Self {
            records: BTreeMap::new(),
            deleted_builtin_ids: BTreeSet::new(),
            mount_configurations: ["claude-code", "codex-cli", "gemini-cli", "opencode"]
                .into_iter()
                .map(|agent_id| AgentMountConfiguration {
                    agent_id: agent_id.to_string(),
                    configured_path: None,
                })
                .collect(),
            drift_snapshots: Vec::new(),
            synchronization_count: 0,
            api_agent_bindings: BTreeSet::new(),
            api_agents: BTreeSet::new(),
        }
    }
}

#[derive(Default)]
struct FakeRepository {
    state: Mutex<RepositoryState>,
    next_write_failure: Mutex<Option<String>>,
}

impl FakeRepository {
    fn insert(&self, record: SkillRecord) {
        self.state
            .lock()
            .expect("repository state")
            .records
            .insert(record.key.clone(), record);
    }

    fn record(&self, key: &SkillKey) -> Option<SkillRecord> {
        self.state
            .lock()
            .expect("repository state")
            .records
            .get(key)
            .cloned()
    }

    fn tombstone_builtin(&self, id: &SkillId) {
        let mut state = self.state.lock().expect("repository state");
        state.records.remove(&SkillKey::new(id.clone(), global()));
        state.deleted_builtin_ids.insert(id.clone());
    }

    fn fail_next_write(&self, message: &str) {
        *self.next_write_failure.lock().expect("next write failure") = Some(message.to_string());
    }

    fn check_write(&self) -> Result<(), SkillApplicationError> {
        match self
            .next_write_failure
            .lock()
            .expect("next write failure")
            .take()
        {
            Some(message) => Err(SkillApplicationError::Repository(message)),
            None => Ok(()),
        }
    }
}

impl SkillRepository for FakeRepository {
    fn list(&self, location: &SkillLocation) -> Result<Vec<SkillRecord>, SkillApplicationError> {
        Ok(self
            .state
            .lock()
            .expect("repository state")
            .records
            .values()
            .filter(|record| &record.key.location == location)
            .cloned()
            .collect())
    }

    fn get(&self, key: &SkillKey) -> Result<Option<SkillRecord>, SkillApplicationError> {
        Ok(self.record(key))
    }

    fn deleted_builtin_ids(&self) -> Result<Vec<SkillId>, SkillApplicationError> {
        Ok(self
            .state
            .lock()
            .expect("repository state")
            .deleted_builtin_ids
            .iter()
            .cloned()
            .collect())
    }

    fn agent_mount_configurations(
        &self,
    ) -> Result<Vec<AgentMountConfiguration>, SkillApplicationError> {
        Ok(self
            .state
            .lock()
            .expect("repository state")
            .mount_configurations
            .clone())
    }

    fn is_api_agent(&self, agent_id: &str) -> Result<bool, SkillApplicationError> {
        Ok(self
            .state
            .lock()
            .expect("repository state")
            .api_agents
            .contains(agent_id))
    }

    fn compatible_agents(&self) -> Result<Vec<SkillCompatibleAgent>, SkillApplicationError> {
        let state = self.state.lock().expect("repository state");
        let mut agents = state
            .mount_configurations
            .iter()
            .map(|configuration| SkillCompatibleAgent {
                id: configuration.agent_id.clone(),
                display_name: configuration.agent_id.clone(),
                kind: SkillAgentKind::Cli,
            })
            .collect::<Vec<_>>();
        agents.extend(
            state
                .api_agents
                .iter()
                .map(|agent_id| SkillCompatibleAgent {
                    id: agent_id.clone(),
                    display_name: agent_id.clone(),
                    kind: SkillAgentKind::Api,
                }),
        );
        Ok(agents)
    }

    fn api_agent_bindings_for_location(
        &self,
        location: &SkillLocation,
    ) -> Result<BTreeMap<String, Vec<String>>, SkillApplicationError> {
        let state = self.state.lock().expect("repository state");
        let mut result = BTreeMap::<String, Vec<String>>::new();
        for (key, agent_id) in &state.api_agent_bindings {
            if &key.location == location {
                result
                    .entry(key.id.as_str().to_string())
                    .or_default()
                    .push(agent_id.clone());
            }
        }
        Ok(result)
    }

    fn enabled_skills_bound_to(
        &self,
        agent_id: &str,
    ) -> Result<Vec<SkillRecord>, SkillApplicationError> {
        Ok(self
            .state
            .lock()
            .expect("repository state")
            .records
            .values()
            .filter(|record| {
                record.enabled
                    && record
                        .bindings
                        .iter()
                        .any(|binding| binding.agent_id == agent_id)
            })
            .cloned()
            .collect())
    }

    fn save_skills(
        &self,
        records: &[SkillRecord],
        clear_deleted_builtin_ids: &[SkillId],
    ) -> Result<(), SkillApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        for record in records {
            state.records.insert(record.key.clone(), record.clone());
        }
        for id in clear_deleted_builtin_ids {
            state.deleted_builtin_ids.remove(id);
        }
        Ok(())
    }

    fn delete_skill(
        &self,
        key: &SkillKey,
        record_builtin_tombstone: bool,
        _deleted_at: &str,
    ) -> Result<(), SkillApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        state.records.remove(key);
        if record_builtin_tombstone {
            state.deleted_builtin_ids.insert(key.id.clone());
        }
        Ok(())
    }

    fn save_mount_path(
        &self,
        agent_id: &str,
        mount_path: &SkillMountPath,
        affected_records: &[SkillRecord],
        _updated_at: &str,
    ) -> Result<(), SkillApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        let configuration = state
            .mount_configurations
            .iter_mut()
            .find(|configuration| configuration.agent_id == agent_id)
            .expect("registered agent");
        configuration.configured_path = Some(mount_path.clone());
        for record in affected_records {
            state.records.insert(record.key.clone(), record.clone());
        }
        Ok(())
    }

    fn save_drift_snapshot(&self, report: &SkillDriftReport) -> Result<(), SkillApplicationError> {
        self.check_write()?;
        self.state
            .lock()
            .expect("repository state")
            .drift_snapshots
            .push(report.clone());
        Ok(())
    }

    fn save_synchronization(
        &self,
        records: &[SkillRecord],
        clear_deleted_builtin_ids: &[SkillId],
        report: &SkillDriftReport,
    ) -> Result<(), SkillApplicationError> {
        self.check_write()?;
        let mut state = self.state.lock().expect("repository state");
        for record in records {
            state.records.insert(record.key.clone(), record.clone());
        }
        for id in clear_deleted_builtin_ids {
            state.deleted_builtin_ids.remove(id);
        }
        state.drift_snapshots.push(report.clone());
        state.synchronization_count += 1;
        Ok(())
    }
}

impl SkillApiBindingRepository for FakeRepository {
    fn bind_api_agent(
        &self,
        key: &SkillKey,
        agent_id: &str,
        _now: &str,
    ) -> Result<(), SkillApplicationError> {
        self.state
            .lock()
            .expect("repository state")
            .api_agent_bindings
            .insert((key.clone(), agent_id.to_string()));
        Ok(())
    }

    fn unbind_api_agent(
        &self,
        key: &SkillKey,
        agent_id: &str,
    ) -> Result<(), SkillApplicationError> {
        self.state
            .lock()
            .expect("repository state")
            .api_agent_bindings
            .remove(&(key.clone(), agent_id.to_string()));
        Ok(())
    }

    fn api_agent_bindings(&self, key: &SkillKey) -> Result<Vec<String>, SkillApplicationError> {
        Ok(self
            .state
            .lock()
            .expect("repository state")
            .api_agent_bindings
            .iter()
            .filter(|(bound_key, _)| bound_key == key)
            .map(|(_, agent_id)| agent_id.clone())
            .collect())
    }

    fn enabled_skills_bound_to_api_agent(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Vec<SkillRecord>, SkillApplicationError> {
        let state = self.state.lock().expect("repository state");
        Ok(state
            .api_agent_bindings
            .iter()
            .filter(|(_, bound_agent_id)| bound_agent_id == agent_id)
            .filter_map(|(key, _)| state.records.get(key))
            .filter(|record| {
                record.enabled
                    && (record.key.location.scope == SkillScope::Global
                        || record.key.location.workspace_path.as_deref() == workspace_path)
            })
            .cloned()
            .collect())
    }
}

#[derive(Default)]
struct FakeFilesystem {
    events: Mutex<Vec<String>>,
    transactions: Mutex<usize>,
    binding_plans: Mutex<Vec<SkillBindingPlan>>,
    binding_failure: Mutex<Option<SkillApplicationError>>,
    inspection: Mutex<Option<SkillDriftInspection>>,
    preview_content: Mutex<String>,
    migration_failure_for: Mutex<Option<String>>,
    refresh_id_override: Mutex<Option<String>>,
    unreadable_ids: Mutex<BTreeSet<String>>,
    /// Skill ids whose source directory is already on disk. Models the state this change exists to
    /// recover from: a source present with no registry record behind it.
    existing_sources: Mutex<BTreeSet<String>>,
    /// Subset of `existing_sources` whose `SKILL.md` cannot be read or parsed.
    unreadable_sources: Mutex<BTreeSet<String>>,
    /// Source directories drift inspection should report when no registry record covers them.
    unregistered_sources: Mutex<BTreeSet<String>>,
}

impl FakeFilesystem {
    fn push_event(&self, event: impl Into<String>) {
        self.events
            .lock()
            .expect("filesystem events")
            .push(event.into());
    }

    fn source(location: &SkillLocation, id: &SkillId, hash: &str) -> ManagedSkillSource {
        let root = match location.scope {
            SkillScope::Global => "global".to_string(),
            SkillScope::Workspace => format!(
                "workspace/{}",
                location.workspace_path.as_deref().unwrap_or_default()
            ),
        };
        ManagedSkillSource {
            skill_dir: format!("{root}/{}", id.as_str()),
            skill_md_path: format!("{root}/{}/SKILL.md", id.as_str()),
            content_hash: hash.to_string(),
        }
    }
}

impl SkillFilesystemPort for FakeFilesystem {
    fn begin_mutation(&self) -> Result<SkillFilesystemTransaction, SkillApplicationError> {
        let mut transactions = self.transactions.lock().expect("transactions");
        *transactions += 1;
        let transaction = SkillFilesystemTransaction {
            id: format!("fs-tx-{transactions}"),
        };
        self.push_event(format!("begin:{}", transaction.id));
        Ok(transaction)
    }

    fn commit_mutation(&self, transaction: SkillFilesystemTransaction) {
        self.push_event(format!("commit:{}", transaction.id));
    }

    fn rollback_mutation(&self, transaction: SkillFilesystemTransaction) {
        self.push_event(format!("rollback:{}", transaction.id));
    }

    fn probe_source(
        &self,
        location: &SkillLocation,
        id: &SkillId,
    ) -> Result<SkillSourceProbe, SkillApplicationError> {
        if !self
            .existing_sources
            .lock()
            .expect("existing sources")
            .contains(id.as_str())
        {
            return Ok(SkillSourceProbe::Absent);
        }
        if self
            .unreadable_sources
            .lock()
            .expect("unreadable sources")
            .contains(id.as_str())
        {
            return Ok(SkillSourceProbe::Unusable(format!(
                "SKILL.md for {} could not be parsed",
                id.as_str()
            )));
        }
        // The metadata comes from the file, not from whatever the caller expected to find there,
        // so a test can tell an adopted record apart from a freshly created one. The hash differs
        // from `content_hash_for`, modelling a source that no longer matches what shipped.
        Ok(SkillSourceProbe::Present(Box::new(SkillImportedSource {
            metadata: metadata(id.as_str()),
            source: Self::source(location, id, &format!("on-disk-hash-{}", id.as_str())),
        })))
    }

    fn content_hash_for(&self, document: &SkillDocument) -> String {
        format!("shipped-hash-{}", document.metadata.id.as_str())
    }

    fn create_source(
        &self,
        _transaction: &SkillFilesystemTransaction,
        location: &SkillLocation,
        id: &SkillId,
        _document: &SkillDocument,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        // Mirrors the real filesystem: creating over an existing directory is refused.
        if self
            .existing_sources
            .lock()
            .expect("existing sources")
            .contains(id.as_str())
        {
            return Err(SkillApplicationError::Conflict(id.as_str().to_string()));
        }
        self.push_event(format!("create:{}", id.as_str()));
        Ok(Self::source(location, id, &format!("hash-{}", id.as_str())))
    }

    fn replace_source(
        &self,
        _transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        _document: &SkillDocument,
        _expected_content_hash: &str,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        self.push_event(format!("replace:{}", record.key.id.as_str()));
        Ok(Self::source(
            &record.key.location,
            &record.key.id,
            "replacement-hash",
        ))
    }

    fn import_source(
        &self,
        _transaction: &SkillFilesystemTransaction,
        location: &SkillLocation,
        source_path: &str,
    ) -> Result<SkillImportedSource, SkillApplicationError> {
        self.push_event(format!("import:{source_path}"));
        let metadata = metadata("imported-skill");
        Ok(SkillImportedSource {
            source: Self::source(location, &metadata.id, "imported-hash"),
            metadata,
        })
    }

    fn inspect_import_metadata(
        &self,
        _source_path: &str,
    ) -> Result<SkillMetadata, SkillApplicationError> {
        Ok(metadata("imported-skill"))
    }

    fn remove_skill(
        &self,
        _transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        remove_source: bool,
    ) -> Result<(), SkillApplicationError> {
        self.push_event(format!("remove:{}:{remove_source}", record.key.id.as_str()));
        Ok(())
    }

    fn reconcile_bindings(
        &self,
        _transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        plan: &SkillBindingPlan,
        mount_paths: &[AgentMountConfiguration],
    ) -> Result<Vec<SkillAgentBinding>, SkillApplicationError> {
        self.push_event(format!("bindings:{}", record.key.id.as_str()));
        self.binding_plans
            .lock()
            .expect("binding plans")
            .push(plan.clone());
        if let Some(error) = self.binding_failure.lock().expect("binding failure").take() {
            return Err(error);
        }
        Ok(plan
            .desired_agent_ids
            .iter()
            .map(|agent_id| {
                let mount_path = mount_paths
                    .iter()
                    .find(|configuration| configuration.agent_id == *agent_id)
                    .and_then(|configuration| configuration.configured_path.clone())
                    .expect("effective mount path");
                SkillAgentBinding {
                    agent_id: agent_id.clone(),
                    mounted_path: format!("{}/{}", mount_path.as_str(), record.key.id.as_str()),
                    mounted: plan.mount.contains(agent_id),
                    mount_path,
                }
            })
            .collect())
    }

    fn migrate_binding(
        &self,
        _transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        agent_id: &str,
        old_mount_path: &SkillMountPath,
        new_mount_path: &SkillMountPath,
    ) -> Result<SkillMountRepair, SkillApplicationError> {
        self.push_event(format!("migrate:{}:{agent_id}", record.key.id.as_str()));
        if self
            .migration_failure_for
            .lock()
            .expect("migration failure")
            .as_deref()
            == Some(record.key.id.as_str())
        {
            return Err(SkillApplicationError::Filesystem(
                "mount target is occupied".to_string(),
            ));
        }
        Ok(SkillMountRepair {
            binding: SkillAgentBinding {
                agent_id: agent_id.to_string(),
                mount_path: new_mount_path.clone(),
                mounted_path: format!("{}/{}", new_mount_path.as_str(), record.key.id.as_str()),
                mounted: true,
            },
            removed_path: Some(format!(
                "{}/{}",
                old_mount_path.as_str(),
                record.key.id.as_str()
            )),
            overwritten: Vec::new(),
            backed_up: Vec::new(),
        })
    }

    fn read_source(&self, record: &SkillRecord) -> Result<String, SkillApplicationError> {
        if self
            .unreadable_ids
            .lock()
            .expect("unreadable ids")
            .contains(record.key.id.as_str())
        {
            return Err(SkillApplicationError::Filesystem(
                "injected unreadable Skill".to_string(),
            ));
        }
        Ok(self
            .preview_content
            .lock()
            .expect("preview content")
            .clone())
    }

    fn observe_bindings(&self, _records: &mut [SkillRecord]) -> Result<(), SkillApplicationError> {
        Ok(())
    }

    fn inspect_drift(
        &self,
        location: &SkillLocation,
        records: &[SkillRecord],
        deleted_builtin_ids: &[SkillId],
    ) -> Result<SkillDriftInspection, SkillApplicationError> {
        if let Some(inspection) = self.inspection.lock().expect("drift inspection").clone() {
            return Ok(inspection);
        }
        // Mirrors the real inspection: a source only counts as unregistered while no record
        // covers it, so adopting one has to make the issue go away.
        let registered_ids = records
            .iter()
            .map(|record| record.key.id.as_str().to_string())
            .collect::<BTreeSet<_>>();
        let unregistered_sources = self
            .unregistered_sources
            .lock()
            .expect("unregistered sources")
            .iter()
            .filter(|id| !registered_ids.contains(*id))
            .map(|id| UnregisteredSkillInspection {
                id: id.clone(),
                path: format!("/skills/{id}"),
            })
            .collect();
        Ok(SkillDriftInspection {
            location: location.clone(),
            registered: Vec::new(),
            unregistered_sources,
            deleted_builtin_ids: deleted_builtin_ids.to_vec(),
        })
    }

    fn repair_binding(
        &self,
        _transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
        agent_id: &str,
        mount_path: &SkillMountPath,
    ) -> Result<SkillMountRepair, SkillApplicationError> {
        self.push_event(format!("repair:{}:{agent_id}", record.key.id.as_str()));
        Ok(SkillMountRepair {
            binding: SkillAgentBinding {
                agent_id: agent_id.to_string(),
                mount_path: mount_path.clone(),
                mounted_path: format!("{}/{}", mount_path.as_str(), record.key.id.as_str()),
                mounted: true,
            },
            removed_path: None,
            overwritten: Vec::new(),
            backed_up: Vec::new(),
        })
    }

    fn refresh_source(
        &self,
        record: &SkillRecord,
        _issue: &SkillDriftIssue,
    ) -> Result<SkillSourceRefresh, SkillApplicationError> {
        self.push_event(format!("refresh:{}", record.key.id.as_str()));
        let refreshed_id = self
            .refresh_id_override
            .lock()
            .expect("refresh id override")
            .clone()
            .unwrap_or_else(|| record.key.id.as_str().to_string());
        Ok(SkillSourceRefresh {
            metadata: SkillMetadata::new(
                refreshed_id,
                format!("Refreshed {}", record.key.id.as_str()),
                "Refreshed description",
                "testing",
                "2.0.0",
                vec!["refreshed".to_string()],
            )
            .expect("refreshed metadata"),
            content_hash: "refreshed-hash".to_string(),
        })
    }
}

struct FixedSelection;

impl SkillWorkspaceSelectionPort for FixedSelection {
    fn select_workspace_directory(&self) -> Result<Option<String>, SkillApplicationError> {
        Ok(Some("D:/workspace".to_string()))
    }
}

struct FixedClock;

impl SkillClockPort for FixedClock {
    fn now(&self) -> String {
        "2026-07-18T00:00:00Z".to_string()
    }
}

#[derive(Default)]
struct FakeLogging {
    events: Mutex<Vec<SkillLogEvent>>,
    fail: Mutex<bool>,
}

impl SkillLoggingPort for FakeLogging {
    fn record(&self, event: &SkillLogEvent) -> Result<(), SkillApplicationError> {
        self.events.lock().expect("log events").push(event.clone());
        if *self.fail.lock().expect("log failure") {
            Err(SkillApplicationError::Logging(
                "logging unavailable".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

struct Fixture {
    service: SkillApplicationService,
    repository: Arc<FakeRepository>,
    filesystem: Arc<FakeFilesystem>,
    logging: Arc<FakeLogging>,
}

impl Fixture {
    fn new() -> Self {
        let repository = Arc::new(FakeRepository::default());
        let filesystem = Arc::new(FakeFilesystem::default());
        let logging = Arc::new(FakeLogging::default());
        Self {
            service: SkillApplicationService::new(
                repository.clone(),
                repository.clone(),
                filesystem.clone(),
                Arc::new(FixedSelection),
                Arc::new(FixedClock),
                logging.clone(),
            ),
            repository,
            filesystem,
            logging,
        }
    }
}

#[derive(Default)]
struct FakeEffectiveCatalog {
    definitions: Mutex<Vec<SkillPackageDescriptor>>,
}

impl FakeEffectiveCatalog {
    fn set(&self, definitions: Vec<SkillPackageDescriptor>) {
        *self.definitions.lock().expect("effective definitions") = definitions;
    }
}

impl EffectiveSkillCatalogPort for FakeEffectiveCatalog {
    fn effective_catalog(
        &self,
        workspace_path: Option<&str>,
    ) -> Result<Vec<EffectiveSkill>, SkillApplicationError> {
        Ok(resolve_effective_catalog(
            workspace_path,
            self.definitions
                .lock()
                .expect("effective definitions")
                .clone(),
        ))
    }

    fn invalidate(&self, _workspace_path: Option<&str>) {}
}

#[derive(Default)]
struct FakeOverlayAppliedSnapshots {
    snapshots: Mutex<BTreeMap<String, OverlayAppliedSkillSnapshot>>,
    resources: Mutex<BTreeMap<(String, String), SkillResourceDocument>>,
    resource_errors: Mutex<BTreeMap<(String, String), SkillApplicationError>>,
}

impl FakeOverlayAppliedSnapshots {
    fn insert(&self, skill_id: &SkillId, snapshot: OverlayAppliedSkillSnapshot) {
        self.snapshots
            .lock()
            .expect("Overlay-applied snapshots")
            .insert(skill_id.as_str().to_string(), snapshot);
    }

    fn insert_resource(&self, skill_id: &SkillId, relative_path: &str, content: &str) {
        self.resources
            .lock()
            .expect("Overlay resource documents")
            .insert(
                (skill_id.as_str().to_string(), relative_path.to_string()),
                SkillResourceDocument {
                    content: content.to_string(),
                    size_bytes: content.len() as u64,
                },
            );
    }

    fn insert_resource_error(
        &self,
        skill_id: &SkillId,
        relative_path: &str,
        error: SkillApplicationError,
    ) {
        self.resource_errors
            .lock()
            .expect("Overlay resource errors")
            .insert(
                (skill_id.as_str().to_string(), relative_path.to_string()),
                error,
            );
    }
}

impl OverlayAppliedSkillSnapshotPort for FakeOverlayAppliedSnapshots {
    fn read_overlay_applied_package(
        &self,
        canonical_skill_id: &SkillId,
        _workspace_identity: Option<&str>,
    ) -> Result<OverlayAppliedSkillSnapshot, SkillApplicationError> {
        self.snapshots
            .lock()
            .expect("Overlay-applied snapshots")
            .get(canonical_skill_id.as_str())
            .cloned()
            .ok_or_else(|| SkillApplicationError::NotFound(canonical_skill_id.as_str().into()))
    }

    fn read_overlay_applied_resource(
        &self,
        canonical_skill_id: &SkillId,
        _workspace_identity: Option<&str>,
        expected_revision: &str,
        logical_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError> {
        let snapshot = self.read_overlay_applied_package(canonical_skill_id, None)?;
        if snapshot.replay.effective().effective_hash() != expected_revision {
            return Err(SkillApplicationError::ConcurrentModification(
                canonical_skill_id.as_str().to_string(),
            ));
        }
        let key = (
            canonical_skill_id.as_str().to_string(),
            logical_path.to_string(),
        );
        if let Some(error) = self
            .resource_errors
            .lock()
            .expect("Overlay resource errors")
            .get(&key)
        {
            return Err(error.clone());
        }
        self.resources
            .lock()
            .expect("Overlay resource documents")
            .get(&key)
            .cloned()
            .ok_or_else(|| SkillApplicationError::NotFound(logical_path.to_string()))
    }
}

#[derive(Default)]
struct FakeEffectiveReader {
    documents: Mutex<BTreeMap<String, SkillDocument>>,
    unreadable: Mutex<BTreeSet<String>>,
    resources: Mutex<BTreeMap<String, Vec<SkillPackageResource>>>,
    resource_documents: Mutex<BTreeMap<(String, String), SkillResourceDocument>>,
    resource_errors: Mutex<BTreeMap<(String, String), SkillApplicationError>>,
    observed_revisions: Mutex<Vec<(String, String)>>,
}

impl FakeEffectiveReader {
    fn insert(&self, package: &SkillPackageDescriptor, body: &str) {
        self.documents.lock().expect("package documents").insert(
            package.package_key.clone(),
            SkillDocument {
                metadata: package.metadata.clone(),
                body: body.to_string(),
            },
        );
    }

    fn insert_resource(
        &self,
        package: &SkillPackageDescriptor,
        relative_path: &str,
        content: &str,
    ) {
        self.resources
            .lock()
            .expect("package resources")
            .entry(package.package_key.clone())
            .or_default()
            .push(SkillPackageResource {
                relative_path: relative_path.to_string(),
                media_type: "text/markdown".to_string(),
                size_bytes: content.len() as u64,
                content_hash: format!("fixture-resource-{}", content.len()),
            });
        self.resource_documents
            .lock()
            .expect("resource documents")
            .insert(
                (package.package_key.clone(), relative_path.to_string()),
                SkillResourceDocument {
                    content: content.to_string(),
                    size_bytes: content.len() as u64,
                },
            );
    }

    fn observe(&self, operation: &str, package: &SkillPackageDescriptor) {
        self.observed_revisions
            .lock()
            .expect("observed revisions")
            .push((operation.to_string(), package.revision.clone()));
    }
}

impl SkillPackageReader for FakeEffectiveReader {
    fn read_document(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<SkillDocument, SkillApplicationError> {
        self.observe("document", package);
        if self
            .unreadable
            .lock()
            .expect("unreadable packages")
            .contains(&package.package_key)
        {
            return Err(SkillApplicationError::Filesystem(
                "fixture package is unreadable".to_string(),
            ));
        }
        self.documents
            .lock()
            .expect("package documents")
            .get(&package.package_key)
            .cloned()
            .ok_or_else(|| SkillApplicationError::NotFound(package.package_key.clone()))
    }

    fn list_resources(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<Vec<SkillPackageResource>, SkillApplicationError> {
        self.observe("resource-index", package);
        Ok(self
            .resources
            .lock()
            .expect("package resources")
            .get(&package.package_key)
            .cloned()
            .unwrap_or_default())
    }

    fn read_resource(
        &self,
        package: &SkillPackageDescriptor,
        relative_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError> {
        self.observe("resource-read", package);
        let key = (package.package_key.clone(), relative_path.to_string());
        if let Some(error) = self
            .resource_errors
            .lock()
            .expect("resource errors")
            .get(&key)
        {
            return Err(error.clone());
        }
        self.resource_documents
            .lock()
            .expect("resource documents")
            .get(&key)
            .cloned()
            .ok_or_else(|| SkillApplicationError::NotFound(relative_path.to_string()))
    }
}

struct FakeEffectiveMaterializer;

impl SkillPackageMaterializer for FakeEffectiveMaterializer {
    fn materialize(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        Ok(ManagedSkillSource {
            skill_dir: format!(
                "cache/{}/{}",
                package.metadata.id.as_str(),
                package.revision
            ),
            skill_md_path: format!(
                "cache/{}/{}/SKILL.md",
                package.metadata.id.as_str(),
                package.revision
            ),
            content_hash: package.revision.clone(),
        })
    }
}

struct FakeOverlayEffectiveMaterializer {
    snapshots: Arc<FakeOverlayAppliedSnapshots>,
}

impl SkillPackageMaterializer for FakeOverlayEffectiveMaterializer {
    fn materialize(
        &self,
        package: &SkillPackageDescriptor,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
        let snapshot = self.snapshots.read_overlay_applied_package(
            &package.metadata.id,
            package.workspace_path.as_deref(),
        )?;
        let revision = snapshot.replay.effective().effective_hash();
        Ok(ManagedSkillSource {
            skill_dir: format!(
                "cache/effective/{}/{}",
                package.metadata.id.as_str(),
                revision
            ),
            skill_md_path: format!(
                "cache/effective/{}/{}/SKILL.md",
                package.metadata.id.as_str(),
                revision
            ),
            content_hash: revision.to_string(),
        })
    }
}

#[derive(Default)]
struct FakeUsageRepository {
    summaries: Mutex<BTreeMap<SkillUsageIdentity, SkillUsageSummary>>,
    fail_mutations: Mutex<bool>,
    recovered_corrupt_state: Mutex<bool>,
}

impl SkillUsageRepository for FakeUsageRepository {
    fn summaries(
        &self,
        _location: &SkillLocation,
        identities: &[SkillUsageIdentity],
    ) -> Result<SkillUsageRead, SkillApplicationError> {
        let stored = self.summaries.lock().expect("usage summaries");
        Ok(SkillUsageRead {
            summaries: identities
                .iter()
                .filter_map(|identity| {
                    stored
                        .get(identity)
                        .cloned()
                        .map(|summary| (identity.clone(), summary))
                })
                .collect(),
            recovered_corrupt_state: *self.recovered_corrupt_state.lock().expect("usage recovery"),
        })
    }

    fn bump(
        &self,
        _location: &SkillLocation,
        identity: &SkillUsageIdentity,
        activity: SkillUsageActivity,
        timestamp: &str,
        revision_witness: &str,
    ) -> Result<SkillUsageMutation, SkillApplicationError> {
        if *self.fail_mutations.lock().expect("usage failure") {
            return Err(SkillApplicationError::Filesystem(
                "usage unavailable".to_string(),
            ));
        }
        let mut summaries = self.summaries.lock().expect("usage summaries");
        let summary = summaries.entry(identity.clone()).or_default();
        match activity {
            SkillUsageActivity::View => {
                summary.view_count += 1;
                summary.last_viewed_at = Some(timestamp.to_string());
            }
            SkillUsageActivity::Use => {
                summary.use_count += 1;
                summary.last_used_at = Some(timestamp.to_string());
            }
        }
        summary.revision_witness = Some(revision_witness.to_string());
        Ok(SkillUsageMutation {
            summary: summary.clone(),
            recovered_corrupt_state: false,
        })
    }
}

struct EffectiveFixture {
    service: SkillApplicationService,
    repository: Arc<FakeRepository>,
    filesystem: Arc<FakeFilesystem>,
    catalog: Arc<FakeEffectiveCatalog>,
    reader: Arc<FakeEffectiveReader>,
    logging: Arc<FakeLogging>,
    usage: Arc<FakeUsageRepository>,
}

impl EffectiveFixture {
    fn new() -> Self {
        let repository = Arc::new(FakeRepository::default());
        let filesystem = Arc::new(FakeFilesystem::default());
        let logging = Arc::new(FakeLogging::default());
        let catalog = Arc::new(FakeEffectiveCatalog::default());
        let reader = Arc::new(FakeEffectiveReader::default());
        let usage = Arc::new(FakeUsageRepository::default());
        let service = SkillApplicationService::new(
            repository.clone(),
            repository.clone(),
            filesystem.clone(),
            Arc::new(FixedSelection),
            Arc::new(FixedClock),
            logging.clone(),
        )
        .with_effective_catalog(catalog.clone())
        .with_effective_package_reader(reader.clone())
        .with_system_materializer(Arc::new(FakeEffectiveMaterializer))
        .with_effective_materializer(Arc::new(FakeEffectiveMaterializer))
        .with_usage_repository(usage.clone());
        Self {
            service,
            repository,
            filesystem,
            catalog,
            reader,
            logging,
            usage,
        }
    }
}

#[test]
fn overlay_applied_effective_hash_is_shared_by_all_runtime_consumers() {
    let fixture = EffectiveFixture::new();
    let mut overlay = OverlayDocument::new(
        id("overlay-runtime-skill"),
        OverlayScope::User,
        None,
        OverlayBaseWitness::new("system:overlay-runtime-skill", "base-body", "base-package")
            .expect("base witness"),
        OverlayTrust::trusted_local(1),
        "2026-08-11T12:00:00Z",
    )
    .expect("Overlay document");
    overlay.patches.push(
        OverlayPatch::new(
            "patch-1",
            "Base",
            "Overlay",
            false,
            "base-body",
            "2026-08-11T12:00:00Z",
        )
        .expect("Overlay patch"),
    );
    let base_resources = vec![BaseSkillResource {
        logical_path: "references/shared.md".to_string(),
        media_type: "text/markdown".to_string(),
        size_bytes: 16,
        content_hash: "shared-resource-hash".to_string(),
        source_layer: SkillLayer::System,
    }];
    let replay = replay_overlay_scope_chain(
        "Base instructions.",
        &base_resources,
        &[OverlayScopeReplayInput::verified(&overlay)],
        None,
        4,
    );
    let effective_hash = replay.effective().effective_hash().to_string();
    let effective_instructions = replay.effective().instructions().to_string();
    let mut package = package(
        "overlay-runtime-skill",
        "system-overlay-runtime-skill",
        SkillLayer::System,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::Eager),
    );
    package.revision = effective_hash.clone();
    fixture.reader.insert(&package, &effective_instructions);
    fixture
        .reader
        .insert_resource(&package, "references/shared.md", "Shared reference");
    *fixture
        .filesystem
        .preview_content
        .lock()
        .expect("preview content") = effective_instructions.clone();
    fixture.catalog.set(vec![package.clone()]);

    let listed = fixture
        .service
        .list_skills(SkillScopeQuery { location: global() })
        .expect("effective list");
    let listed_skill = listed
        .skills
        .iter()
        .find(|record| record.key.id == package.metadata.id)
        .expect("listed Overlay-applied Skill");
    assert_eq!(listed_skill.managed_source.content_hash, effective_hash);
    assert!(listed_skill
        .managed_source
        .skill_dir
        .ends_with(&effective_hash));
    let preview = fixture
        .service
        .preview_skill(SkillKey::new(package.metadata.id.clone(), global()))
        .expect("list preview");
    assert_eq!(preview.content, effective_instructions);

    register_known_agent_effective(&fixture, "my-api-agent");
    fixture
        .service
        .bind_skill_to_api_agent(
            SkillKey::new(package.metadata.id.clone(), global()),
            "my-api-agent".to_string(),
        )
        .expect("API binding");
    let prompts = fixture
        .service
        .bound_skill_prompts_for_api_agent("my-api-agent", None)
        .expect("eager prompt");
    assert_eq!(prompts[0].body, effective_instructions);

    let loaded = match fixture
        .service
        .load_skill_for_agent(SkillLoadRequest {
            id_or_alias: package.metadata.id.as_str().to_string(),
            workspace_path: None,
        })
        .expect("on-demand load")
    {
        SkillLoadOutcome::Loaded(loaded) => loaded,
        SkillLoadOutcome::Refused(refusal) => panic!("unexpected load refusal: {refusal:?}"),
    };
    assert_eq!(loaded.revision, effective_hash);
    assert_eq!(loaded.content, effective_instructions);
    let resource_uri = loaded.resources.references[0].uri.clone();
    let resource = match fixture
        .service
        .read_skill_resource_for_agent(SkillResourceReadRequest {
            uri: resource_uri,
            revision: effective_hash.clone(),
            workspace_path: None,
        })
        .expect("resource read")
    {
        SkillResourceReadOutcome::Read(resource) => resource,
        SkillResourceReadOutcome::Refused(refusal) => {
            panic!("unexpected resource refusal: {refusal:?}")
        }
    };
    assert_eq!(resource.revision, effective_hash);
    assert_eq!(resource.content, "Shared reference");

    let observations = fixture
        .reader
        .observed_revisions
        .lock()
        .expect("observed revisions");
    assert!(observations
        .iter()
        .all(|(_, revision)| revision == &effective_hash));
    assert!(
        observations
            .iter()
            .filter(|(operation, _)| operation == "document")
            .count()
            >= 2
    );
    assert!(observations
        .iter()
        .any(|(operation, _)| operation == "resource-index"));
    assert!(observations
        .iter()
        .any(|(operation, _)| operation == "resource-read"));
}

#[test]
fn cli_bindings_use_the_overlay_effective_derived_source() {
    let mut fixture = EffectiveFixture::new();
    let package = package(
        "overlay-cli-skill",
        "user-overlay-cli-skill",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::OnDemand),
    );
    fixture.reader.insert(&package, "Base instructions");
    fixture.catalog.set(vec![package.clone()]);
    let snapshots = Arc::new(FakeOverlayAppliedSnapshots::default());
    snapshots.insert(
        &package.metadata.id,
        overlay_applied_snapshot(&package, "Base instructions", "Overlay instructions"),
    );
    let effective_hash = snapshots
        .snapshots
        .lock()
        .expect("Overlay snapshots")
        .get(package.metadata.id.as_str())
        .expect("Overlay snapshot")
        .replay
        .effective()
        .effective_hash()
        .to_string();
    fixture.service = fixture
        .service
        .clone()
        .with_overlay_applied_snapshots(snapshots.clone())
        .with_effective_materializer(Arc::new(FakeOverlayEffectiveMaterializer { snapshots }));

    let bound = fixture
        .service
        .set_bindings(
            SkillKey::new(package.metadata.id.clone(), global()),
            vec!["codex-cli".to_string()],
        )
        .expect("CLI binding");

    assert_eq!(bound.managed_source.content_hash, effective_hash);
    assert!(bound.managed_source.skill_dir.contains("cache/effective"));
    assert!(!bound.managed_source.skill_dir.contains("skill_overlays"));
    assert!(!bound.managed_source.skill_dir.contains("payload"));
    assert_eq!(bound.bindings.len(), 1);
    assert!(bound.bindings[0].mounted);
}

fn package(
    id_value: &str,
    key: &str,
    layer: SkillLayer,
    workspace_path: Option<&str>,
    skill_type: Option<SkillType>,
    delivery: Option<SkillDelivery>,
) -> SkillPackageDescriptor {
    SkillPackageDescriptor {
        package_key: key.to_string(),
        workspace_path: workspace_path.map(str::to_string),
        metadata: SkillMetadata::with_classification(
            id_value,
            format!("Name {id_value}"),
            "Description",
            "testing",
            "1.0.0",
            Vec::new(),
            Vec::new(),
            skill_type,
            delivery,
        )
        .expect("package metadata"),
        layer,
        origin: if layer == SkillLayer::System {
            SkillOrigin::Shipped
        } else {
            SkillOrigin::Created
        },
        trust: SkillTrust::Trusted,
        availability: if skill_type == Some(SkillType::Utility) {
            SkillAvailability::Unsupported
        } else {
            SkillAvailability::Available
        },
        revision: format!("revision-{key}"),
        source_path: (layer != SkillLayer::System).then(|| format!("source/{key}")),
    }
}

fn id(value: &str) -> SkillId {
    SkillId::parse(value).expect("skill id")
}

fn global() -> SkillLocation {
    SkillLocation::new(SkillScope::Global, None).expect("global location")
}

fn workspace() -> SkillLocation {
    SkillLocation::new(SkillScope::Workspace, Some("D:/workspace")).expect("workspace location")
}

fn metadata(value: &str) -> SkillMetadata {
    SkillMetadata::new(
        value,
        format!("Name {value}"),
        "Description",
        "testing",
        "1.0.0",
        vec![value.to_string()],
    )
    .expect("metadata")
}

fn record(
    value: &str,
    location: SkillLocation,
    source: SkillSource,
    enabled: bool,
    agent_ids: &[&str],
) -> SkillRecord {
    let skill_id = id(value);
    SkillRecord {
        key: SkillKey::new(skill_id.clone(), location.clone()),
        source,
        enabled,
        managed_source: FakeFilesystem::source(&location, &skill_id, "original-hash"),
        metadata: metadata(value),
        bindings: agent_ids
            .iter()
            .map(|agent_id| {
                let mount_path = SkillMountPath::parse(match *agent_id {
                    "claude-code" => ".claude/skills",
                    "codex-cli" => ".codex/skills",
                    "gemini-cli" => ".gemini/skills",
                    "opencode" => ".opencode/skills",
                    _ => ".vanehub/skills",
                })
                .expect("mount path");
                SkillAgentBinding {
                    agent_id: (*agent_id).to_string(),
                    mounted_path: format!("{}/{value}", mount_path.as_str()),
                    mounted: enabled,
                    mount_path,
                }
            })
            .collect(),
        created_at: "2026-07-17T00:00:00Z".to_string(),
        updated_at: "2026-07-17T00:00:00Z".to_string(),
        resolved_metadata: None,
    }
}

#[test]
fn listing_seeds_the_exact_builtin_catalog_once_and_calculates_stats() {
    let fixture = Fixture::new();

    let first = fixture
        .service
        .list_skills(SkillScopeQuery { location: global() })
        .expect("first list");
    assert_eq!(first.skills.len(), builtin_definitions().len());
    assert_eq!(first.stats.total, 6);
    assert_eq!(first.stats.enabled, 6);
    assert_eq!(first.stats.mounted, 0);
    assert!(first
        .skills
        .iter()
        .all(|skill| skill.source == SkillSource::Builtin));

    let event_count = fixture
        .filesystem
        .events
        .lock()
        .expect("filesystem events")
        .len();
    let second = fixture
        .service
        .list_skills(SkillScopeQuery { location: global() })
        .expect("second list");
    assert_eq!(second, first);
    assert_eq!(
        fixture
            .filesystem
            .events
            .lock()
            .expect("filesystem events")
            .len(),
        event_count
    );
    assert_eq!(
        fixture.logging.events.lock().expect("log events")[0].action,
        SkillLogAction::SeedBuiltins
    );
}

#[test]
fn create_coordinates_source_binding_persistence_transaction_and_semantic_log() {
    let fixture = Fixture::new();
    let created = fixture
        .service
        .create_skill(SkillCreateRequest {
            id: id("sample-skill"),
            location: workspace(),
            metadata: metadata("sample-skill"),
            body: "Body".to_string(),
            enabled: true,
            bound_agent_ids: vec![
                "codex-cli".to_string(),
                "claude-code".to_string(),
                "codex-cli".to_string(),
            ],
            source: None,
        })
        .expect("created skill");

    assert_eq!(created.source, SkillSource::User);
    assert_eq!(
        created.bound_agent_ids(),
        vec!["claude-code".to_string(), "codex-cli".to_string()]
    );
    assert!(created.bindings.iter().all(|binding| binding.mounted));
    assert_eq!(
        fixture.repository.record(&created.key),
        Some(created.clone())
    );
    let events = fixture.filesystem.events.lock().expect("filesystem events");
    assert!(events.first().expect("begin event").starts_with("begin:"));
    assert!(events.last().expect("commit event").starts_with("commit:"));
    drop(events);
    let log = fixture.logging.events.lock().expect("log events")[0].clone();
    assert_eq!(log.action, SkillLogAction::Create);
    assert_eq!(log.level, SkillLogLevel::Info);
    assert_eq!(log.skill_id.as_deref(), Some("sample-skill"));
}

#[test]
fn repository_failure_rolls_back_filesystem_mutation_and_logs_the_error() {
    let fixture = Fixture::new();
    fixture.repository.fail_next_write("database unavailable");
    let key = SkillKey::new(id("rollback-skill"), workspace());

    let error = fixture
        .service
        .create_skill(SkillCreateRequest {
            id: key.id.clone(),
            location: key.location.clone(),
            metadata: metadata("rollback-skill"),
            body: "Body".to_string(),
            enabled: false,
            bound_agent_ids: Vec::new(),
            source: None,
        })
        .expect_err("repository failure");

    assert_eq!(
        error,
        SkillApplicationError::Repository("database unavailable".to_string())
    );
    assert_eq!(fixture.repository.record(&key), None);
    assert!(fixture
        .filesystem
        .events
        .lock()
        .expect("filesystem events")
        .last()
        .expect("rollback event")
        .starts_with("rollback:"));
    assert_eq!(
        fixture.logging.events.lock().expect("log events")[0].level,
        SkillLogLevel::Error
    );
}

#[test]
fn update_rejects_an_identity_change_before_opening_a_filesystem_transaction() {
    let fixture = Fixture::new();
    let existing = record("stable-skill", workspace(), SkillSource::User, true, &[]);
    fixture.repository.insert(existing.clone());

    let error = fixture
        .service
        .update_skill(SkillUpdateRequest {
            key: existing.key,
            metadata: metadata("different-skill"),
            body: "Changed".to_string(),
            expected_content_hash: existing.managed_source.content_hash,
        })
        .expect_err("immutable id");

    assert_eq!(
        error,
        SkillApplicationError::Domain(SkillDomainError::UpdateIdChanged)
    );
    assert!(fixture
        .filesystem
        .events
        .lock()
        .expect("filesystem events")
        .is_empty());
}

#[test]
fn system_content_update_is_rejected_before_a_filesystem_transaction() {
    let fixture = Fixture::new();
    let existing = record("code-review", global(), SkillSource::Builtin, true, &[]);
    fixture.repository.insert(existing.clone());

    let error = fixture
        .service
        .update_skill(SkillUpdateRequest {
            key: existing.key,
            metadata: existing.metadata,
            body: "Changed".to_string(),
            expected_content_hash: existing.managed_source.content_hash,
        })
        .expect_err("immutable System package");

    assert_eq!(
        error,
        SkillApplicationError::ImmutablePackage("code-review".to_string())
    );
    assert!(fixture
        .filesystem
        .events
        .lock()
        .expect("filesystem events")
        .is_empty());
}

#[test]
fn import_cannot_overwrite_system_content_and_opens_no_transaction() {
    let fixture = Fixture::new();
    let existing = record("imported-skill", global(), SkillSource::Builtin, true, &[]);
    fixture.repository.insert(existing);

    let error = fixture
        .service
        .import_skill(SkillImportRequest {
            location: global(),
            source_path: "D:/incoming/system-overwrite".to_string(),
            enabled: true,
            bound_agent_ids: Vec::new(),
        })
        .expect_err("immutable System package");

    assert_eq!(
        error,
        SkillApplicationError::ImmutablePackage("imported-skill".to_string())
    );
    assert!(fixture
        .filesystem
        .events
        .lock()
        .expect("filesystem events")
        .is_empty());
}

#[test]
fn enablement_and_binding_use_cases_apply_domain_plans_to_all_desired_agents() {
    let fixture = Fixture::new();
    let existing = record(
        "bound-skill",
        workspace(),
        SkillSource::User,
        false,
        &["codex-cli"],
    );
    let key = existing.key.clone();
    fixture.repository.insert(existing);

    let enabled = fixture
        .service
        .set_enabled(key.clone(), true)
        .expect("enable skill");
    assert!(enabled.enabled);
    assert!(enabled.bindings[0].mounted);

    let rebound = fixture
        .service
        .set_bindings(key, vec!["claude-code".to_string()])
        .expect("rebind skill");
    assert_eq!(rebound.bound_agent_ids(), vec!["claude-code"]);
    assert!(rebound.bindings[0].mounted);
    let plans = fixture
        .filesystem
        .binding_plans
        .lock()
        .expect("binding plans");
    assert_eq!(plans[0].mount, vec!["codex-cli"]);
    assert_eq!(plans[1].bind, vec!["claude-code"]);
    assert_eq!(plans[1].unbind, vec!["codex-cli"]);
}

#[test]
fn mount_path_update_migrates_bound_skills_and_persists_the_configuration() {
    let fixture = Fixture::new();
    let existing = record(
        "mounted-skill",
        global(),
        SkillSource::User,
        true,
        &["codex-cli"],
    );
    let key = existing.key.clone();
    fixture.repository.insert(existing);

    let report = fixture
        .service
        .update_mount_path(
            "codex-cli".to_string(),
            SkillMountPath::parse(".custom/skills").expect("custom mount path"),
        )
        .expect("mount migration");

    assert_eq!(report.migrated, vec!["mounted-skill"]);
    assert_eq!(report.old_mount_path.as_str(), ".codex/skills");
    assert_eq!(report.new_mount_path.as_str(), ".custom/skills");
    assert_eq!(
        fixture
            .repository
            .record(&key)
            .expect("updated record")
            .bindings[0]
            .mount_path
            .as_str(),
        ".custom/skills"
    );
    let listed = fixture.service.list_mount_paths().expect("mount paths");
    let codex = listed
        .iter()
        .find(|path| path.agent_id == "codex-cli")
        .expect("codex path");
    assert!(!codex.is_default);
    assert_eq!(codex.mount_path.as_str(), ".custom/skills");
}

#[test]
fn partial_mount_migration_is_reported_and_logged_as_a_warning() {
    let fixture = Fixture::new();
    fixture.repository.insert(record(
        "migrated-skill",
        global(),
        SkillSource::User,
        true,
        &["codex-cli"],
    ));
    fixture.repository.insert(record(
        "conflicting-skill",
        global(),
        SkillSource::User,
        true,
        &["codex-cli"],
    ));
    *fixture
        .filesystem
        .migration_failure_for
        .lock()
        .expect("migration failure") = Some("conflicting-skill".to_string());

    let report = fixture
        .service
        .update_mount_path(
            "codex-cli".to_string(),
            SkillMountPath::parse(".custom/skills").expect("custom mount path"),
        )
        .expect("partial mount migration");

    assert_eq!(report.migrated, vec!["migrated-skill"]);
    assert_eq!(report.failed.len(), 1);
    assert_eq!(report.failed[0].skill_id, "conflicting-skill");
    let event = fixture.logging.events.lock().expect("log events")[0].clone();
    assert_eq!(event.action, SkillLogAction::UpdateMountPath);
    assert_eq!(event.level, SkillLogLevel::Warn);
}

#[test]
fn deleting_and_restoring_a_builtin_manages_its_tombstone_atomically() {
    let fixture = Fixture::new();
    let builtin = record("code-review", global(), SkillSource::Builtin, true, &[]);
    let key = builtin.key.clone();
    fixture.repository.insert(builtin);

    fixture
        .service
        .delete_skill(key.clone())
        .expect("delete builtin");
    assert!(fixture
        .filesystem
        .events
        .lock()
        .expect("filesystem events")
        .iter()
        .any(|event| event == "remove:code-review:false"));
    assert_eq!(fixture.repository.record(&key), None);
    assert_eq!(
        fixture
            .repository
            .deleted_builtin_ids()
            .expect("tombstones"),
        vec![id("code-review")]
    );

    let restored = fixture
        .service
        .restore_builtin(id("code-review"))
        .expect("restore builtin");
    assert_eq!(restored.source, SkillSource::Builtin);
    assert!(restored.enabled);
    assert_eq!(fixture.repository.record(&key), Some(restored));
    assert!(fixture
        .repository
        .deleted_builtin_ids()
        .expect("tombstones")
        .is_empty());
}

#[test]
fn preview_import_and_workspace_selection_delegate_to_explicit_ports() {
    let fixture = Fixture::new();
    let existing = record("preview-skill", workspace(), SkillSource::User, true, &[]);
    let key = existing.key.clone();
    fixture.repository.insert(existing.clone());
    *fixture
        .filesystem
        .preview_content
        .lock()
        .expect("preview content") = "# Preview".to_string();

    let preview = fixture.service.preview_skill(key).expect("skill preview");
    assert_eq!(preview.content, "# Preview");
    assert_eq!(preview.path, existing.managed_source.skill_md_path);

    let imported = fixture
        .service
        .import_skill(SkillImportRequest {
            location: workspace(),
            source_path: "D:/incoming/SKILL.md".to_string(),
            enabled: true,
            bound_agent_ids: vec!["opencode".to_string()],
        })
        .expect("import skill");
    assert_eq!(imported.source, SkillSource::Imported);
    assert_eq!(imported.key.id.as_str(), "imported-skill");
    assert_eq!(imported.bound_agent_ids(), vec!["opencode"]);
    assert_eq!(
        fixture
            .service
            .select_workspace_directory()
            .expect("workspace selection")
            .as_deref(),
        Some("D:/workspace")
    );
}

#[test]
fn drift_detection_classifies_inspection_and_persists_a_stable_snapshot() {
    let fixture = Fixture::new();
    let location = workspace();
    let existing = record(
        "drifted-skill",
        location.clone(),
        SkillSource::User,
        true,
        &["codex-cli"],
    );
    fixture.repository.insert(existing);
    *fixture
        .filesystem
        .inspection
        .lock()
        .expect("drift inspection") = Some(SkillDriftInspection {
        location: location.clone(),
        registered: vec![RegisteredSkillInspection {
            id: id("drifted-skill"),
            enabled: true,
            expected_content_hash: "original-hash".to_string(),
            source: SkillSourceInspection::Present {
                path: "workspace/drifted-skill/SKILL.md".to_string(),
                content_hash: "changed-hash".to_string(),
            },
            bindings: vec![SkillBindingInspection {
                agent_id: "codex-cli".to_string(),
                mounted_path: ".codex/skills/drifted-skill".to_string(),
                observation: SkillMountObservation::Missing,
            }],
        }],
        unregistered_sources: Vec::new(),
        deleted_builtin_ids: Vec::new(),
    });

    let first = fixture
        .service
        .detect_skill_drift(SkillScopeQuery {
            location: location.clone(),
        })
        .expect("first drift report");
    let second = fixture
        .service
        .detect_skill_drift(SkillScopeQuery { location })
        .expect("second drift report");

    assert_eq!(
        first
            .issues
            .iter()
            .map(|issue| issue.issue_type)
            .collect::<Vec<_>>(),
        vec![
            SkillDriftIssueType::MetadataChanged,
            SkillDriftIssueType::MissingMount
        ]
    );
    assert_eq!(second.drift_hash, first.drift_hash);
    assert!(!first.drift_hash.is_empty());
    assert_eq!(
        fixture
            .repository
            .state
            .lock()
            .expect("repository state")
            .drift_snapshots
            .len(),
        2
    );
}

#[test]
fn drift_sync_merges_multiple_repairs_and_commits_successful_changes_once() {
    let fixture = Fixture::new();
    fixture
        .service
        .list_skills(SkillScopeQuery { location: global() })
        .expect("seed builtins");
    fixture.repository.tombstone_builtin(&id("code-review"));
    let existing = record(
        "drifted-skill",
        global(),
        SkillSource::User,
        true,
        &["codex-cli"],
    );
    let existing_key = existing.key.clone();
    fixture.repository.insert(existing);
    fixture
        .filesystem
        .events
        .lock()
        .expect("filesystem events")
        .clear();
    fixture.logging.events.lock().expect("log events").clear();
    *fixture
        .filesystem
        .inspection
        .lock()
        .expect("drift inspection") = Some(SkillDriftInspection {
        location: global(),
        registered: vec![RegisteredSkillInspection {
            id: id("drifted-skill"),
            enabled: true,
            expected_content_hash: "original-hash".to_string(),
            source: SkillSourceInspection::Present {
                path: "global/drifted-skill/SKILL.md".to_string(),
                content_hash: "changed-hash".to_string(),
            },
            bindings: vec![SkillBindingInspection {
                agent_id: "codex-cli".to_string(),
                mounted_path: ".codex/skills/drifted-skill".to_string(),
                observation: SkillMountObservation::Missing,
            }],
        }],
        unregistered_sources: Vec::new(),
        deleted_builtin_ids: vec![id("code-review")],
    });

    let result = fixture
        .service
        .sync_skill_drift(SkillScopeQuery { location: global() })
        .expect("drift sync");

    assert_eq!(result.mounted, vec!["drifted-skill"]);
    assert!(result.restored.contains(&"drifted-skill".to_string()));
    assert!(!result.restored.contains(&"code-review".to_string()));
    assert!(result.failed.is_empty());
    let synchronized = fixture
        .repository
        .record(&existing_key)
        .expect("synchronized record");
    assert_eq!(synchronized.metadata.name, "Refreshed drifted-skill");
    assert_eq!(synchronized.managed_source.content_hash, "refreshed-hash");
    assert!(synchronized.bindings[0].mounted);
    assert!(fixture
        .repository
        .record(&SkillKey::new(id("code-review"), global()))
        .is_none());
    let state = fixture.repository.state.lock().expect("repository state");
    assert!(state.deleted_builtin_ids.contains(&id("code-review")));
    assert_eq!(state.synchronization_count, 1);
    drop(state);
    assert!(fixture
        .filesystem
        .events
        .lock()
        .expect("filesystem events")
        .last()
        .expect("commit event")
        .starts_with("commit:"));
    assert_eq!(
        fixture.logging.events.lock().expect("log events")[0].action,
        SkillLogAction::SyncDrift
    );
}

#[test]
fn diagnostic_storage_failure_does_not_hide_a_successful_use_case() {
    let fixture = Fixture::new();
    *fixture.logging.fail.lock().expect("log failure") = true;

    let created = fixture
        .service
        .create_skill(SkillCreateRequest {
            id: id("logged-skill"),
            location: global(),
            metadata: metadata("logged-skill"),
            body: "Body".to_string(),
            enabled: false,
            bound_agent_ids: Vec::new(),
            source: None,
        })
        .expect("successful create");

    assert_eq!(created.key.id.as_str(), "logged-skill");
    assert_eq!(fixture.logging.events.lock().expect("log events").len(), 1);
}

fn register_known_agent(fixture: &Fixture, agent_id: &str) {
    fixture
        .repository
        .state
        .lock()
        .expect("repository state")
        .api_agents
        .insert(agent_id.to_string());
}

#[test]
fn binding_to_an_unknown_agent_is_rejected() {
    let fixture = Fixture::new();
    let existing = record("fixture-skill", global(), SkillSource::User, true, &[]);
    fixture.repository.insert(existing.clone());

    let error = fixture
        .service
        .bind_skill_to_api_agent(existing.key, "never-registered".to_string())
        .expect_err("unknown Agent id");

    assert!(matches!(error, SkillApplicationError::Validation(_)));
}

fn canonical_test_path(directory: &TempDirectory) -> String {
    let canonical = directory
        .path()
        .canonicalize()
        .expect("canonical test path");
    let value = canonical.to_string_lossy();
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
}

#[test]
fn drift_sync_refuses_refreshed_metadata_with_a_different_identity() {
    let fixture = Fixture::new();
    let existing = record("stable-skill", global(), SkillSource::User, true, &[]);
    fixture.repository.insert(existing.clone());
    *fixture
        .filesystem
        .refresh_id_override
        .lock()
        .expect("refresh override") = Some("different-skill".to_string());
    *fixture
        .filesystem
        .inspection
        .lock()
        .expect("drift inspection") = Some(SkillDriftInspection {
        location: global(),
        registered: vec![RegisteredSkillInspection {
            id: existing.key.id.clone(),
            enabled: true,
            expected_content_hash: existing.managed_source.content_hash.clone(),
            source: SkillSourceInspection::Present {
                path: existing.managed_source.skill_md_path.clone(),
                content_hash: "changed-hash".to_string(),
            },
            bindings: Vec::new(),
        }],
        unregistered_sources: Vec::new(),
        deleted_builtin_ids: Vec::new(),
    });

    let result = fixture
        .service
        .sync_skill_drift(SkillScopeQuery { location: global() })
        .expect("best-effort drift sync");

    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].skill_id, "stable-skill");
    assert_eq!(
        fixture
            .repository
            .record(&existing.key)
            .expect("unchanged record")
            .metadata
            .id,
        existing.metadata.id
    );
}

#[test]
fn api_and_cli_binding_operations_reject_the_wrong_agent_kind() {
    let fixture = Fixture::new();
    register_known_agent(&fixture, "my-api-agent");
    let existing = record("fixture-skill", global(), SkillSource::User, true, &[]);
    fixture.repository.insert(existing.clone());

    let api_error = fixture
        .service
        .bind_skill_to_api_agent(existing.key.clone(), "codex-cli".to_string())
        .expect_err("CLI agent rejected for API binding");
    let cli_error = fixture
        .service
        .bind_skill_to_cli_agent(existing.key, "my-api-agent".to_string())
        .expect_err("API agent rejected for CLI binding");

    assert!(matches!(api_error, SkillApplicationError::Validation(_)));
    assert!(matches!(
        cli_error,
        SkillApplicationError::Domain(SkillDomainError::UnknownAgent(_))
    ));
}

#[test]
fn granular_cli_bindings_do_not_lose_independent_concurrent_changes() {
    let fixture = Fixture::new();
    let existing = record("fixture-skill", global(), SkillSource::User, true, &[]);
    fixture.repository.insert(existing.clone());
    let first = fixture.service.clone();
    let second = fixture.service.clone();
    let first_key = existing.key.clone();
    let second_key = existing.key.clone();

    let first_thread = std::thread::spawn(move || {
        first.bind_skill_to_cli_agent(first_key, "codex-cli".to_string())
    });
    let second_thread = std::thread::spawn(move || {
        second.bind_skill_to_cli_agent(second_key, "claude-code".to_string())
    });
    first_thread
        .join()
        .expect("first thread")
        .expect("first bind");
    second_thread
        .join()
        .expect("second thread")
        .expect("second bind");

    let stored = fixture
        .repository
        .record(&existing.key)
        .expect("stored Skill");
    assert_eq!(
        stored.bound_agent_ids(),
        vec!["claude-code".to_string(), "codex-cli".to_string()]
    );
}

#[test]
fn mount_root_rejection_preserves_bindings_and_logs_safe_agent_context() {
    let fixture = Fixture::new();
    let existing = record("fixture-skill", global(), SkillSource::User, true, &[]);
    fixture.repository.insert(existing.clone());
    *fixture
        .filesystem
        .binding_failure
        .lock()
        .expect("binding failure") = Some(SkillApplicationError::MountRootExternalLink(
        "codex-cli".to_string(),
    ));

    let error = fixture
        .service
        .bind_skill_to_cli_agent(existing.key.clone(), "codex-cli".to_string())
        .expect_err("external mount root rejection");

    assert_eq!(
        error,
        SkillApplicationError::MountRootExternalLink("codex-cli".to_string())
    );
    assert!(fixture
        .repository
        .record(&existing.key)
        .expect("unchanged record")
        .bound_agent_ids()
        .is_empty());
    assert!(fixture
        .filesystem
        .events
        .lock()
        .expect("filesystem events")
        .last()
        .expect("rollback event")
        .starts_with("rollback:"));
    let log = fixture
        .logging
        .events
        .lock()
        .expect("log events")
        .last()
        .expect("binding log")
        .clone();
    assert_eq!(log.action, SkillLogAction::BindCliAgent);
    assert_eq!(log.level, SkillLogLevel::Error);
    assert_eq!(log.skill_id.as_deref(), Some("fixture-skill"));
    assert_eq!(
        log.context,
        BTreeMap::from([("agentId".to_string(), "codex-cli".to_string())])
    );
    let sensitive_paths = [
        r"C:\Users\developer\.codex\skills",
        r"D:\external-skill-manager\skills",
    ];
    for sensitive_path in sensitive_paths {
        assert!(!log.message.contains(sensitive_path));
        assert!(!log
            .context
            .values()
            .any(|value| value.contains(sensitive_path)));
    }
}

#[test]
fn restoring_a_builtin_requires_an_explicit_tombstone() {
    let fixture = Fixture::new();
    let error = fixture
        .service
        .restore_builtin(id("code-review"))
        .expect_err("restore without tombstone");

    assert!(matches!(error, SkillApplicationError::Validation(_)));
}

#[test]
fn bound_skill_prompts_strip_frontmatter_and_exclude_disabled_skills() {
    let fixture = Fixture::new();
    register_known_agent(&fixture, "my-api-agent");
    let enabled = record("enabled-skill", global(), SkillSource::User, true, &[]);
    let disabled = record("disabled-skill", global(), SkillSource::User, false, &[]);
    fixture.repository.insert(enabled.clone());
    fixture.repository.insert(disabled.clone());
    *fixture.filesystem.preview_content.lock().expect("preview content") =
        "---\nid: enabled-skill\nname: Enabled Skill\ndescription: d\ncategory: c\nversion: 1.0.0\ntriggers:\n  - t\n---\n\n# Enabled Skill\n\nDo the thing.\n"
            .to_string();

    fixture
        .service
        .bind_skill_to_api_agent(enabled.key.clone(), "my-api-agent".to_string())
        .expect("bind enabled skill");
    fixture
        .service
        .bind_skill_to_api_agent(disabled.key.clone(), "my-api-agent".to_string())
        .expect("bind disabled skill");

    let prompts = fixture
        .service
        .bound_skill_prompts_for_api_agent("my-api-agent", None)
        .expect("bound skill prompts");

    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].name, "Name enabled-skill");
    assert_eq!(prompts[0].body, "# Enabled Skill\n\nDo the thing.");
}

#[test]
fn bound_skill_prompts_are_workspace_isolated_and_skip_one_unreadable_source() {
    let fixture = Fixture::new();
    register_known_agent(&fixture, "my-api-agent");
    let first_workspace = TempDirectory::new("Skill API workspace first");
    let second_workspace = TempDirectory::new("Skill API workspace second");
    let first_path = canonical_test_path(&first_workspace);
    let second_path = canonical_test_path(&second_workspace);
    let global_record = record("global-skill", global(), SkillSource::User, true, &[]);
    let first_record = record(
        "first-workspace-skill",
        SkillLocation::new(SkillScope::Workspace, Some(&first_path)).expect("first location"),
        SkillSource::User,
        true,
        &[],
    );
    let second_record = record(
        "second-workspace-skill",
        SkillLocation::new(SkillScope::Workspace, Some(&second_path)).expect("second location"),
        SkillSource::User,
        true,
        &[],
    );
    for skill in [&global_record, &first_record, &second_record] {
        fixture.repository.insert(skill.clone());
        fixture
            .service
            .bind_skill_to_api_agent(skill.key.clone(), "my-api-agent".to_string())
            .expect("API binding");
    }
    fixture
        .filesystem
        .unreadable_ids
        .lock()
        .expect("unreadable ids")
        .insert(global_record.key.id.as_str().to_string());
    *fixture.filesystem.preview_content.lock().expect("preview content") =
        "---\nid: placeholder\nname: Placeholder\ndescription: d\ncategory: c\nversion: 1.0.0\ntriggers:\n  - t\n---\n\nHealthy body"
            .to_string();

    let prompts = fixture
        .service
        .bound_skill_prompts_for_api_agent("my-api-agent", Some(&first_path))
        .expect("workspace prompts");

    assert_eq!(
        prompts
            .iter()
            .map(|prompt| prompt.id.as_str())
            .collect::<Vec<_>>(),
        vec!["first-workspace-skill"]
    );
    assert!(fixture
        .logging
        .events
        .lock()
        .expect("log events")
        .iter()
        .any(|event| event.level == SkillLogLevel::Warn
            && event.skill_id.as_deref() == Some("global-skill")));
}

#[test]
fn canonical_binding_follows_the_effective_winner_without_rewrite() {
    let fixture = EffectiveFixture::new();
    register_known_agent_effective(&fixture, "my-api-agent");
    let system = package(
        "layered-skill",
        "system-layered",
        SkillLayer::System,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::Eager),
    );
    fixture.reader.insert(&system, "System body");
    fixture.catalog.set(vec![system.clone()]);
    let key = SkillKey::new(id("layered-skill"), global());
    fixture
        .service
        .bind_skill_to_api_agent(key.clone(), "my-api-agent".to_string())
        .expect("canonical binding");
    let system_preview = fixture
        .service
        .preview_skill(key.clone())
        .expect("System preview");
    assert_eq!(system_preview.path, "skill://layered-skill/");
    assert_eq!(system_preview.effective.layer, SkillLayer::System);
    assert!(system_preview.effective.immutable);
    let bindings_before = fixture
        .repository
        .state
        .lock()
        .expect("repository state")
        .api_agent_bindings
        .clone();

    let user = package(
        "layered-skill",
        "user-layered",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::Eager),
    );
    fixture.reader.insert(&user, "User winner body");
    fixture.catalog.set(vec![system, user]);
    let prompts = fixture
        .service
        .bound_skill_prompts_for_api_agent("my-api-agent", None)
        .expect("effective prompt");
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].body, "User winner body");
    assert_eq!(
        fixture
            .repository
            .state
            .lock()
            .expect("repository state")
            .api_agent_bindings,
        bindings_before
    );

    let listed = fixture
        .service
        .list_skills(SkillScopeQuery { location: global() })
        .expect("effective list");
    let layered = listed
        .skills
        .iter()
        .find(|record| record.key.id.as_str() == "layered-skill")
        .expect("one effective row");
    let effective = layered.effective_metadata();
    assert_eq!(effective.layer, SkillLayer::User);
    assert_eq!(effective.shadowed.len(), 1);
    assert_eq!(effective.shadowed[0].layer, SkillLayer::System);

    let disabled = fixture
        .service
        .set_enabled(key, false)
        .expect("disable effective winner");
    assert_eq!(
        disabled.effective_metadata().availability,
        SkillAvailability::Disabled
    );
    assert_eq!(
        fixture
            .repository
            .state
            .lock()
            .expect("repository state")
            .api_agent_bindings,
        bindings_before
    );
}

#[test]
fn view_and_eager_use_tracking_are_separate_from_packages_and_surface_in_inventory() {
    let fixture = EffectiveFixture::new();
    register_known_agent_effective(&fixture, "my-api-agent");
    let package = package(
        "usage-visible-skill",
        "usage-visible-user",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::Eager),
    );
    fixture.reader.insert(&package, "Unchanged instructions");
    fixture.catalog.set(vec![package.clone()]);
    let original = fixture
        .reader
        .documents
        .lock()
        .expect("documents")
        .get(&package.package_key)
        .cloned()
        .expect("package document");

    let viewed = fixture.service.bump_view(&package).expect("view summary");
    assert_eq!(viewed.view_count, 1);
    fixture
        .service
        .bind_skill_to_api_agent(
            SkillKey::new(package.metadata.id.clone(), global()),
            "my-api-agent".to_string(),
        )
        .expect("bind");
    let prompts = fixture
        .service
        .bound_skill_prompts_for_api_agent("my-api-agent", None)
        .expect("prompts");
    assert_eq!(prompts.len(), 1);

    let listed = fixture
        .service
        .list_skills(SkillScopeQuery { location: global() })
        .expect("inventory");
    let usage = listed.skills[0].effective_metadata().usage;
    assert_eq!(usage.view_count, 1);
    assert_eq!(usage.use_count, 1);
    assert_eq!(
        usage.revision_witness.as_deref(),
        Some(package.revision.as_str())
    );
    assert_eq!(
        fixture
            .reader
            .documents
            .lock()
            .expect("documents")
            .get(&package.package_key),
        Some(&original)
    );
}

#[test]
fn usage_tracking_failure_is_logged_but_does_not_fail_prompt_assembly() {
    let fixture = EffectiveFixture::new();
    register_known_agent_effective(&fixture, "my-api-agent");
    let package = package(
        "usage-failure-skill",
        "usage-failure-user",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::Eager),
    );
    fixture.reader.insert(&package, "Healthy instructions");
    fixture.catalog.set(vec![package.clone()]);
    fixture
        .service
        .bind_skill_to_api_agent(
            SkillKey::new(package.metadata.id.clone(), global()),
            "my-api-agent".to_string(),
        )
        .expect("bind");
    *fixture.usage.fail_mutations.lock().expect("usage failure") = true;

    let prompts = fixture
        .service
        .bound_skill_prompts_for_api_agent("my-api-agent", None)
        .expect("tracking is best effort");
    assert_eq!(prompts.len(), 1);
    assert!(fixture
        .logging
        .events
        .lock()
        .expect("log events")
        .iter()
        .any(|event| event.action == SkillLogAction::TrackUse
            && event.level == SkillLogLevel::Warn
            && event.context.get("reason").map(String::as_str) == Some("filesystem")));
}

#[test]
fn progressive_discovery_is_metadata_only_filtered_bounded_and_deterministic() {
    let fixture = EffectiveFixture::new();
    let alpha = package(
        "alpha-skill",
        "user-alpha",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::OnDemand),
    );
    let beta = package(
        "beta-skill",
        "user-beta",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::Eager),
    );
    let utility = package(
        "utility-skill",
        "user-utility-discovery",
        SkillLayer::User,
        None,
        Some(SkillType::Utility),
        Some(SkillDelivery::OnDemand),
    );
    fixture.catalog.set(vec![utility, beta, alpha]);

    let result = fixture
        .service
        .list_skills_for_agent(SkillDiscoveryRequest {
            skill_type: Some(SkillType::Role),
            limit: Some(1),
            ..SkillDiscoveryRequest::default()
        })
        .expect("discovery");
    assert_eq!(result.skills.len(), 1);
    assert_eq!(result.skills[0].id, "alpha-skill");
    assert_eq!(result.skills[0].delivery, SkillDelivery::OnDemand);
    assert!(result.truncated);
    assert!(fixture
        .reader
        .documents
        .lock()
        .expect("documents")
        .is_empty());
}

#[test]
fn progressive_load_resolves_alias_truncates_replaces_base_uri_and_tracks_view() {
    let fixture = EffectiveFixture::new();
    let mut package = package(
        "developer-skill",
        "user-developer",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::OnDemand),
    );
    package.metadata.aliases = vec![id("dev")];
    let body = format!("Use {{skill_base_dir}} references. {}", "甲".repeat(12_100));
    fixture.reader.insert(&package, &body);
    fixture
        .reader
        .insert_resource(&package, "references/guide.md", "Guide body");
    fixture.catalog.set(vec![package.clone()]);

    let outcome = fixture
        .service
        .load_skill_for_agent(SkillLoadRequest {
            id_or_alias: "dev".to_string(),
            workspace_path: None,
        })
        .expect("load");
    let SkillLoadOutcome::Loaded(loaded) = outcome else {
        panic!("expected loaded Skill");
    };
    assert_eq!(loaded.id, "developer-skill");
    assert_eq!(loaded.base_uri, "skill://developer-skill/");
    assert!(loaded.content.contains("skill://developer-skill/"));
    assert_eq!(loaded.content.chars().count(), MAX_INLINE_SKILL_CHARACTERS);
    assert!(loaded.truncated);
    assert_eq!(
        loaded.resources.references[0].uri,
        "skill://developer-skill/references/guide.md"
    );
    assert_eq!(
        fixture.usage.summaries.lock().expect("usage summaries")[&SkillUsageIdentity {
            id: package.metadata.id,
            layer: SkillLayer::User,
        }]
            .view_count,
        1
    );

    *fixture.usage.fail_mutations.lock().expect("usage failure") = true;
    assert!(matches!(
        fixture
            .service
            .load_skill_for_agent(SkillLoadRequest {
                id_or_alias: "developer-skill".to_string(),
                workspace_path: None,
            })
            .expect("tracking remains best effort"),
        SkillLoadOutcome::Loaded(_)
    ));
    assert!(fixture
        .logging
        .events
        .lock()
        .expect("events")
        .iter()
        .any(
            |event| event.action == SkillLogAction::TrackView && event.level == SkillLogLevel::Warn
        ));
}

#[test]
fn progressive_load_and_resource_reads_share_the_overlay_snapshot_and_revision() {
    let mut fixture = EffectiveFixture::new();
    let package = package(
        "overlay-role",
        "user-overlay-role",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::OnDemand),
    );
    fixture.reader.insert(&package, "Base instructions");
    fixture
        .reader
        .insert_resource(&package, "references/guide.md", "Base guide");
    fixture.catalog.set(vec![package.clone()]);

    let base_resources = vec![BaseSkillResource {
        logical_path: "references/guide.md".to_string(),
        media_type: "text/markdown".to_string(),
        size_bytes: 10,
        content_hash: "base-guide-hash".to_string(),
        source_layer: SkillLayer::User,
    }];
    let base_replay =
        replay_overlay_scope_chain("Base instructions", &base_resources, &[], None, 0);
    let base_snapshot = OverlayEffectivePackageSnapshot {
        canonical_skill_id: package.metadata.id.clone(),
        base_identity: package.package_key.clone(),
        base_layer: package.layer,
        instructions: "Base instructions".to_string(),
        resources: base_resources.clone(),
        instruction_hash: base_replay.base().instruction_hash().to_string(),
        package_hash: base_replay.base().effective_hash().to_string(),
    };
    let mut overlay = OverlayDocument::new(
        package.metadata.id.clone(),
        OverlayScope::User,
        None,
        OverlayBaseWitness::new(
            &base_snapshot.base_identity,
            &base_snapshot.instruction_hash,
            &base_snapshot.package_hash,
        )
        .expect("base witness"),
        OverlayTrust::trusted_local(1),
        "2026-08-11T00:00:00Z",
    )
    .expect("Overlay document");
    overlay.patches.push(
        OverlayPatch::new(
            "role-patch",
            "Base instructions",
            "Overlay instructions at {skill_base_dir}",
            false,
            &base_snapshot.instruction_hash,
            "2026-08-11T00:00:00Z",
        )
        .expect("Overlay patch"),
    );
    overlay.files.push(
        OverlayFile::new(
            "guide-file",
            "references/guide.md",
            "text/markdown",
            13,
            "overlay-guide-hash",
            "payloads/overlay-guide-hash",
            "2026-08-11T00:00:00Z",
        )
        .expect("Overlay guide"),
    );
    overlay.files.push(
        OverlayFile::new(
            "binary-file",
            "assets/logo.png",
            "image/png",
            4,
            "overlay-logo-hash",
            "payloads/overlay-logo-hash",
            "2026-08-11T00:00:00Z",
        )
        .expect("Overlay image"),
    );
    let replay = replay_overlay_scope_chain(
        "Base instructions",
        &base_resources,
        &[OverlayScopeReplayInput::verified(&overlay)],
        None,
        8,
    );
    let revision = replay.effective().effective_hash().to_string();
    let snapshots = Arc::new(FakeOverlayAppliedSnapshots::default());
    snapshots.insert(
        &package.metadata.id,
        OverlayAppliedSkillSnapshot {
            base: base_snapshot,
            replay,
        },
    );
    snapshots.insert_resource(&package.metadata.id, "references/guide.md", "Overlay guide");
    snapshots.insert_resource_error(
        &package.metadata.id,
        "assets/logo.png",
        SkillApplicationError::BinaryResource,
    );
    fixture.service = fixture
        .service
        .clone()
        .with_overlay_applied_snapshots(snapshots);

    let SkillLoadOutcome::Loaded(loaded) = fixture
        .service
        .load_skill_for_agent(SkillLoadRequest {
            id_or_alias: "overlay-role".to_string(),
            workspace_path: None,
        })
        .expect("Overlay-applied load")
    else {
        panic!("expected loaded Overlay Skill");
    };
    assert_eq!(
        loaded.content,
        "Overlay instructions at skill://overlay-role/"
    );
    assert_eq!(loaded.revision, revision);
    assert_eq!(loaded.resources.references.len(), 1);
    assert_eq!(loaded.resources.assets.len(), 1);

    let read = |relative_path: &str, revision: &str| SkillResourceReadRequest {
        uri: format!("skill://overlay-role/{relative_path}"),
        revision: revision.to_string(),
        workspace_path: None,
    };
    let SkillResourceReadOutcome::Read(resource) = fixture
        .service
        .read_skill_resource_for_agent(read("references/guide.md", &revision))
        .expect("Overlay resource read")
    else {
        panic!("expected readable Overlay resource");
    };
    assert_eq!(resource.content, "Overlay guide");
    assert_eq!(resource.revision, revision);

    for (relative_path, witness, expected_reason) in [
        (
            "references/guide.md",
            package.revision.as_str(),
            SkillAccessRefusalReason::StaleRevision,
        ),
        (
            "assets/logo.png",
            revision.as_str(),
            SkillAccessRefusalReason::BinaryResource,
        ),
    ] {
        let SkillResourceReadOutcome::Refused(refusal) = fixture
            .service
            .read_skill_resource_for_agent(read(relative_path, witness))
            .expect("resource refusal")
        else {
            panic!("expected resource refusal");
        };
        assert_eq!(refusal.reason, expected_reason);
    }
}

#[test]
fn progressive_load_refuses_utility_unavailable_and_ambiguous_aliases() {
    let fixture = EffectiveFixture::new();
    let utility = package(
        "utility-skill",
        "user-utility-load",
        SkillLayer::User,
        None,
        Some(SkillType::Utility),
        Some(SkillDelivery::OnDemand),
    );
    fixture.catalog.set(vec![utility]);
    let outcome = fixture
        .service
        .load_skill_for_agent(SkillLoadRequest {
            id_or_alias: "utility-skill".to_string(),
            workspace_path: None,
        })
        .expect("utility refusal");
    assert!(matches!(
        outcome,
        SkillLoadOutcome::Refused(SkillAccessRefusal {
            reason: SkillAccessRefusalReason::UtilityNotLoadable,
            ..
        })
    ));

    let disabled = package(
        "disabled-skill",
        "user-disabled-load",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::OnDemand),
    );
    fixture.reader.insert(&disabled, "Disabled instructions");
    fixture.catalog.set(vec![disabled.clone()]);
    fixture
        .service
        .set_enabled(SkillKey::new(disabled.metadata.id.clone(), global()), false)
        .expect("disable");
    let outcome = fixture
        .service
        .load_skill_for_agent(SkillLoadRequest {
            id_or_alias: "disabled-skill".to_string(),
            workspace_path: None,
        })
        .expect("disabled refusal");
    assert!(matches!(
        outcome,
        SkillLoadOutcome::Refused(SkillAccessRefusal {
            reason: SkillAccessRefusalReason::Disabled,
            ..
        })
    ));

    let mut first = package(
        "first-skill",
        "user-first-alias",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::OnDemand),
    );
    let mut second = package(
        "second-skill",
        "user-second-alias",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::OnDemand),
    );
    first.metadata.aliases = vec![id("shared")];
    second.metadata.aliases = vec![id("shared")];
    fixture.catalog.set(vec![second, first]);
    let outcome = fixture
        .service
        .load_skill_for_agent(SkillLoadRequest {
            id_or_alias: "shared".to_string(),
            workspace_path: None,
        })
        .expect("ambiguous refusal");
    let SkillLoadOutcome::Refused(refusal) = outcome else {
        panic!("expected refusal");
    };
    assert_eq!(refusal.reason, SkillAccessRefusalReason::AmbiguousAlias);
    assert_eq!(refusal.conflicting_ids, vec!["first-skill", "second-skill"]);
}

#[test]
fn utility_resolution_uses_exact_overlay_revision_but_load_remains_refused() {
    let mut fixture = EffectiveFixture::new();
    let utility = package(
        "utility-review",
        "user-utility-review",
        SkillLayer::User,
        None,
        Some(SkillType::Utility),
        Some(SkillDelivery::OnDemand),
    );
    fixture.reader.insert(&utility, "Base utility instructions");
    fixture.catalog.set(vec![utility.clone()]);
    let snapshots = Arc::new(FakeOverlayAppliedSnapshots::default());
    let snapshot = overlay_applied_snapshot(
        &utility,
        "Base utility instructions",
        "Overlay utility instructions",
    );
    let revision = snapshot.replay.effective().effective_hash().to_string();
    snapshots.insert(&utility.metadata.id, snapshot);
    fixture.service = fixture
        .service
        .clone()
        .with_overlay_applied_snapshots(snapshots);

    let UtilitySkillResolutionOutcome::Resolved(resolved) = fixture
        .service
        .resolve_utility_for_execution("utility-review", None)
        .expect("Utility resolution")
    else {
        panic!("expected resolved Utility");
    };
    assert_eq!(resolved.id, "utility-review");
    assert_eq!(resolved.revision, revision);
    assert_eq!(resolved.instructions, "Overlay utility instructions");
    assert!(resolved.workspace_path.is_none());

    assert!(matches!(
        fixture
            .service
            .load_skill_for_agent(SkillLoadRequest {
                id_or_alias: "utility-review".to_string(),
                workspace_path: None,
            })
            .expect("load refusal"),
        SkillLoadOutcome::Refused(SkillAccessRefusal {
            reason: SkillAccessRefusalReason::UtilityNotLoadable,
            ..
        })
    ));
}

#[test]
fn utility_resolution_refuses_role_and_untrusted_utility() {
    let fixture = EffectiveFixture::new();
    let role = package(
        "role-only",
        "user-role-only",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::OnDemand),
    );
    fixture.reader.insert(&role, "Role instructions");
    let mut utility = package(
        "imported-utility",
        "registry-imported-utility",
        SkillLayer::Registry,
        None,
        Some(SkillType::Utility),
        Some(SkillDelivery::OnDemand),
    );
    utility.trust = SkillTrust::Untrusted;
    fixture.reader.insert(&utility, "Untrusted instructions");
    fixture.catalog.set(vec![role, utility]);

    for id in ["role-only", "imported-utility"] {
        let UtilitySkillResolutionOutcome::Refused(refusal) = fixture
            .service
            .resolve_utility_for_execution(id, None)
            .expect("refusal")
        else {
            panic!("expected refusal for {id}");
        };
        assert_eq!(refusal.reason, SkillAccessRefusalReason::Unsupported);
    }
}

#[test]
fn progressive_resource_reads_require_current_indexed_safe_text() {
    let fixture = EffectiveFixture::new();
    let package = package(
        "resource-skill",
        "user-resource",
        SkillLayer::User,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::OnDemand),
    );
    fixture.reader.insert(&package, "Instructions");
    fixture
        .reader
        .insert_resource(&package, "references/guide.md", "Guide body");
    fixture
        .reader
        .insert_resource(&package, "assets/large.txt", &"x".repeat(70_000));
    fixture.catalog.set(vec![package.clone()]);

    let request = |uri: &str, revision: &str| SkillResourceReadRequest {
        uri: uri.to_string(),
        revision: revision.to_string(),
        workspace_path: None,
    };
    let outcome = fixture
        .service
        .read_skill_resource_for_agent(request(
            "skill://resource-skill/references/guide.md",
            &package.revision,
        ))
        .expect("read");
    assert!(matches!(outcome, SkillResourceReadOutcome::Read(_)));

    for (uri, revision, reason) in [
        (
            "skill://resource-skill/references/guide.md",
            "stale",
            SkillAccessRefusalReason::StaleRevision,
        ),
        (
            "skill://resource-skill/references/missing.md",
            package.revision.as_str(),
            SkillAccessRefusalReason::UnindexedResource,
        ),
        (
            "skill://resource-skill/references/../secret.md",
            package.revision.as_str(),
            SkillAccessRefusalReason::InvalidUri,
        ),
        (
            "skill://resource-skill/assets/large.txt",
            package.revision.as_str(),
            SkillAccessRefusalReason::OversizedResource,
        ),
    ] {
        let outcome = fixture
            .service
            .read_skill_resource_for_agent(request(uri, revision))
            .expect("refusal");
        assert!(matches!(
            outcome,
            SkillResourceReadOutcome::Refused(SkillAccessRefusal { reason: actual, .. })
                if actual == reason
        ));
    }

    fixture
        .reader
        .resource_errors
        .lock()
        .expect("resource errors")
        .insert(
            (
                package.package_key.clone(),
                "references/guide.md".to_string(),
            ),
            SkillApplicationError::BinaryResource,
        );
    let outcome = fixture
        .service
        .read_skill_resource_for_agent(request(
            "skill://resource-skill/references/guide.md",
            &package.revision,
        ))
        .expect("binary refusal");
    assert!(matches!(
        outcome,
        SkillResourceReadOutcome::Refused(SkillAccessRefusal {
            reason: SkillAccessRefusalReason::BinaryResource,
            ..
        })
    ));
    assert!(fixture
        .logging
        .events
        .lock()
        .expect("events")
        .iter()
        .all(|event| !event.message.contains("Guide body")
            && event
                .context
                .values()
                .all(|value| !value.contains("Guide body"))));
}

#[test]
fn effective_prompt_filters_delivery_utility_shadowing_workspace_and_unreadable_packages() {
    let fixture = EffectiveFixture::new();
    register_known_agent_effective(&fixture, "my-api-agent");
    let workspace_dir = TempDirectory::new("effective-prompt-workspace");
    let workspace_path = canonical_test_path(&workspace_dir);
    let definitions = vec![
        package(
            "legacy-skill",
            "user-legacy",
            SkillLayer::User,
            None,
            None,
            None,
        ),
        package(
            "on-demand-skill",
            "user-on-demand",
            SkillLayer::User,
            None,
            Some(SkillType::Role),
            Some(SkillDelivery::OnDemand),
        ),
        package(
            "utility-skill",
            "user-utility",
            SkillLayer::User,
            None,
            Some(SkillType::Utility),
            Some(SkillDelivery::Eager),
        ),
        package(
            "shadowed-skill",
            "system-shadowed",
            SkillLayer::System,
            None,
            Some(SkillType::Role),
            Some(SkillDelivery::Eager),
        ),
        package(
            "shadowed-skill",
            "user-shadowed",
            SkillLayer::User,
            None,
            Some(SkillType::Role),
            Some(SkillDelivery::Eager),
        ),
        package(
            "workspace-skill",
            "system-workspace",
            SkillLayer::System,
            None,
            Some(SkillType::Role),
            Some(SkillDelivery::Eager),
        ),
        package(
            "workspace-skill",
            "project-workspace",
            SkillLayer::Project,
            Some(&workspace_path),
            Some(SkillType::Role),
            Some(SkillDelivery::Eager),
        ),
        package(
            "unreadable-skill",
            "user-unreadable",
            SkillLayer::User,
            None,
            Some(SkillType::Role),
            Some(SkillDelivery::Eager),
        ),
    ];
    for definition in &definitions {
        fixture
            .reader
            .insert(definition, &format!("Body from {}", definition.package_key));
    }
    fixture
        .reader
        .unreadable
        .lock()
        .expect("unreadable packages")
        .insert("user-unreadable".to_string());
    fixture.catalog.set(definitions);
    for skill_id in [
        "legacy-skill",
        "on-demand-skill",
        "utility-skill",
        "shadowed-skill",
        "workspace-skill",
        "unreadable-skill",
    ] {
        fixture
            .service
            .bind_skill_to_api_agent(
                SkillKey::new(id(skill_id), global()),
                "my-api-agent".to_string(),
            )
            .expect("binding");
    }

    let prompts = fixture
        .service
        .bound_skill_prompts_for_api_agent("my-api-agent", Some(&workspace_path))
        .expect("effective prompts");
    assert_eq!(
        prompts
            .iter()
            .map(|prompt| prompt.id.as_str())
            .collect::<Vec<_>>(),
        vec!["workspace-skill", "legacy-skill", "shadowed-skill"]
    );
    assert_eq!(prompts[0].body, "Body from project-workspace");
    assert_eq!(prompts[2].body, "Body from user-shadowed");
    assert!(fixture
        .logging
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| {
            event.skill_id.as_deref() == Some("unreadable-skill")
                && event.action == SkillLogAction::ResolveApiPrompt
        }));
}

#[test]
fn effective_prompt_budgets_whole_bodies_and_tracking_failure_is_best_effort() {
    let fixture = EffectiveFixture::new();
    register_known_agent_effective(&fixture, "my-api-agent");
    let definitions = [
        ("a-first", "a", 7_975),
        ("b-second", "b", 7_975),
        ("c-no-room", "c", 100),
        ("d-small", "d", 1),
        ("oversized", "x", 8_001),
    ]
    .into_iter()
    .map(|(id_value, key, size)| {
        let definition = package(
            id_value,
            key,
            SkillLayer::User,
            None,
            Some(SkillType::Role),
            Some(SkillDelivery::Eager),
        );
        fixture.reader.insert(&definition, &key.repeat(size));
        definition
    })
    .collect::<Vec<_>>();
    fixture.catalog.set(definitions.clone());
    for definition in &definitions {
        fixture
            .service
            .bind_skill_to_api_agent(
                SkillKey::new(definition.metadata.id.clone(), global()),
                "my-api-agent".to_string(),
            )
            .expect("binding");
    }
    *fixture.logging.fail.lock().expect("logging failure") = true;

    let prompts = fixture
        .service
        .bound_skill_prompts_for_api_agent("my-api-agent", None)
        .expect("tracking failure must not fail prompt assembly");
    assert_eq!(
        prompts
            .iter()
            .map(|prompt| prompt.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a-first", "b-second", "d-small"]
    );
    let events = fixture.logging.events.lock().expect("events");
    assert_eq!(
        events
            .iter()
            .filter(|event| event.action == SkillLogAction::TrackUse)
            .count(),
        prompts.len()
    );
    assert!(events.iter().any(|event| {
        event.skill_id.as_deref() == Some("oversized")
            && event.context.get("reason") == Some(&"individual-budget".to_string())
    }));
    assert!(events.iter().any(|event| {
        event.skill_id.as_deref() == Some("c-no-room")
            && event.context.get("reason") == Some(&"aggregate-budget".to_string())
    }));
}

#[test]
fn eager_prompts_use_last_healthy_overlay_content_with_existing_budgets_and_usage() {
    let mut fixture = EffectiveFixture::new();
    register_known_agent_effective(&fixture, "my-api-agent");
    let overlay_snapshots = Arc::new(FakeOverlayAppliedSnapshots::default());
    let definitions = [
        ("a-first", "Base A".to_string(), "a".repeat(7_900)),
        ("b-second", "Base B".to_string(), "b".repeat(7_900)),
        ("c-no-room", "Base C".to_string(), "c".repeat(200)),
        ("compact", "z".repeat(8_100), "Overlay compact".to_string()),
        ("oversized", "Base oversized".to_string(), "x".repeat(8_001)),
    ]
    .into_iter()
    .map(|(id_value, base, effective)| {
        let definition = package(
            id_value,
            id_value,
            SkillLayer::User,
            None,
            Some(SkillType::Role),
            Some(SkillDelivery::Eager),
        );
        fixture.reader.insert(&definition, &base);
        overlay_snapshots.insert(
            &definition.metadata.id,
            overlay_applied_snapshot(&definition, &base, &effective),
        );
        definition
    })
    .collect::<Vec<_>>();
    fixture.catalog.set(definitions.clone());
    fixture.service = fixture
        .service
        .clone()
        .with_overlay_applied_snapshots(overlay_snapshots.clone());
    for definition in &definitions {
        fixture
            .service
            .bind_skill_to_api_agent(
                SkillKey::new(definition.metadata.id.clone(), global()),
                "my-api-agent".to_string(),
            )
            .expect("binding");
    }

    let prompts = fixture
        .service
        .bound_skill_prompts_for_api_agent("my-api-agent", None)
        .expect("Overlay-applied prompts");
    assert_eq!(
        prompts
            .iter()
            .map(|prompt| prompt.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a-first", "b-second", "compact"]
    );
    assert_eq!(prompts[2].body, "Overlay compact");

    let usage = fixture.usage.summaries.lock().expect("usage summaries");
    assert_eq!(usage.len(), prompts.len());
    for definition in definitions.iter().filter(|definition| {
        ["a-first", "b-second", "compact"].contains(&definition.metadata.id.as_str())
    }) {
        let summary = usage
            .get(&SkillUsageIdentity {
                id: definition.metadata.id.clone(),
                layer: definition.layer,
            })
            .expect("usage summary");
        let expected_hash = overlay_snapshots
            .snapshots
            .lock()
            .expect("Overlay-applied snapshots")
            .get(definition.metadata.id.as_str())
            .expect("Overlay-applied snapshot")
            .replay
            .effective()
            .effective_hash()
            .to_string();
        assert_eq!(summary.use_count, 1);
        assert_eq!(
            summary.revision_witness.as_deref(),
            Some(expected_hash.as_str())
        );
    }
    assert!(fixture
        .logging
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| {
            event.skill_id.as_deref() == Some("oversized")
                && event.context.get("reason").map(String::as_str) == Some("individual-budget")
        }));
    assert!(fixture
        .logging
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| {
            event.skill_id.as_deref() == Some("c-no-room")
                && event.context.get("reason").map(String::as_str) == Some("aggregate-budget")
        }));
}

#[test]
fn eager_prompt_uses_lower_scope_content_when_a_higher_overlay_conflicts() {
    let mut fixture = EffectiveFixture::new();
    register_known_agent_effective(&fixture, "my-api-agent");
    let definition = package(
        "last-healthy-eager",
        "last-healthy-eager",
        SkillLayer::System,
        None,
        Some(SkillType::Role),
        Some(SkillDelivery::Eager),
    );
    fixture.reader.insert(&definition, "Base");
    fixture.catalog.set(vec![definition.clone()]);
    let base_replay = replay_overlay_scope_chain("Base", &[], &[], None, 0);
    let base_snapshot = OverlayEffectivePackageSnapshot {
        canonical_skill_id: definition.metadata.id.clone(),
        base_identity: definition.package_key.clone(),
        base_layer: definition.layer,
        instructions: "Base".to_string(),
        resources: Vec::new(),
        instruction_hash: base_replay.base().instruction_hash().to_string(),
        package_hash: base_replay.base().effective_hash().to_string(),
    };
    let witness = OverlayBaseWitness::new(
        &base_snapshot.base_identity,
        &base_snapshot.instruction_hash,
        &base_snapshot.package_hash,
    )
    .expect("base witness");
    let mut system = OverlayDocument::new(
        definition.metadata.id.clone(),
        OverlayScope::System,
        None,
        witness.clone(),
        OverlayTrust::trusted_local(1),
        "2026-08-11T00:00:00Z",
    )
    .expect("System Overlay");
    system.patches.push(
        OverlayPatch::new(
            "system-patch",
            "Base",
            "System healthy",
            false,
            &base_snapshot.instruction_hash,
            "2026-08-11T00:00:00Z",
        )
        .expect("System patch"),
    );
    let mut user = OverlayDocument::new(
        definition.metadata.id.clone(),
        OverlayScope::User,
        None,
        witness,
        OverlayTrust::trusted_local(1),
        "2026-08-11T00:00:00Z",
    )
    .expect("User Overlay");
    user.patches.push(
        OverlayPatch::new(
            "conflicting-user-patch",
            "Missing target",
            "Unsafe partial result",
            false,
            &base_snapshot.instruction_hash,
            "2026-08-11T00:00:00Z",
        )
        .expect("User patch"),
    );
    let replay = replay_overlay_scope_chain(
        "Base",
        &[],
        &[
            OverlayScopeReplayInput::verified(&system),
            OverlayScopeReplayInput::verified(&user),
        ],
        None,
        8,
    );
    let snapshots = Arc::new(FakeOverlayAppliedSnapshots::default());
    snapshots.insert(
        &definition.metadata.id,
        OverlayAppliedSkillSnapshot {
            base: base_snapshot,
            replay,
        },
    );
    fixture.service = fixture
        .service
        .clone()
        .with_overlay_applied_snapshots(snapshots);
    fixture
        .service
        .bind_skill_to_api_agent(
            SkillKey::new(definition.metadata.id.clone(), global()),
            "my-api-agent".to_string(),
        )
        .expect("binding");

    let prompts = fixture
        .service
        .bound_skill_prompts_for_api_agent("my-api-agent", None)
        .expect("last healthy prompt");
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].body, "System healthy");
}

fn overlay_applied_snapshot(
    package: &SkillPackageDescriptor,
    base: &str,
    effective: &str,
) -> OverlayAppliedSkillSnapshot {
    let base_replay = replay_overlay_scope_chain(base, &[], &[], None, 0);
    let base_snapshot = OverlayEffectivePackageSnapshot {
        canonical_skill_id: package.metadata.id.clone(),
        base_identity: package.package_key.clone(),
        base_layer: package.layer,
        instructions: base.to_string(),
        resources: Vec::new(),
        instruction_hash: base_replay.base().instruction_hash().to_string(),
        package_hash: base_replay.base().effective_hash().to_string(),
    };
    let mut overlay = OverlayDocument::new(
        package.metadata.id.clone(),
        OverlayScope::User,
        None,
        OverlayBaseWitness::new(
            &base_snapshot.base_identity,
            &base_snapshot.instruction_hash,
            &base_snapshot.package_hash,
        )
        .expect("base witness"),
        OverlayTrust::trusted_local(1),
        "2026-08-11T00:00:00Z",
    )
    .expect("Overlay document");
    overlay.patches.push(
        OverlayPatch::new(
            "prompt-patch",
            base,
            effective,
            false,
            &base_snapshot.instruction_hash,
            "2026-08-11T00:00:00Z",
        )
        .expect("Overlay patch"),
    );
    let replay = replay_overlay_scope_chain(
        base,
        &[],
        &[OverlayScopeReplayInput::verified(&overlay)],
        None,
        8,
    );
    OverlayAppliedSkillSnapshot {
        base: base_snapshot,
        replay,
    }
}

fn register_known_agent_effective(fixture: &EffectiveFixture, agent_id: &str) {
    fixture
        .repository
        .state
        .lock()
        .expect("repository state")
        .api_agents
        .insert(agent_id.to_string());
}

#[test]
fn unbinding_removes_a_skill_from_the_bound_list() {
    let fixture = Fixture::new();
    register_known_agent(&fixture, "my-api-agent");
    let existing = record("fixture-skill", global(), SkillSource::User, true, &[]);
    fixture.repository.insert(existing.clone());
    fixture
        .service
        .bind_skill_to_api_agent(existing.key.clone(), "my-api-agent".to_string())
        .expect("bind");

    fixture
        .service
        .unbind_skill_from_api_agent(existing.key.clone(), "my-api-agent".to_string())
        .expect("unbind");

    assert!(fixture
        .service
        .list_api_agent_bindings(existing.key)
        .expect("bindings")
        .is_empty());
}

/// The state observed on a real installation: every built-in source directory present on disk,
/// zero registry rows, no deletion tombstones. Seeding must recover from it without user action.
#[test]
fn seeding_adopts_builtin_sources_that_exist_without_a_registry_record() {
    let fixture = Fixture::new();
    {
        let mut existing = fixture
            .filesystem
            .existing_sources
            .lock()
            .expect("existing sources");
        for definition in builtin_definitions() {
            existing.insert(
                definition
                    .metadata()
                    .expect("metadata")
                    .id
                    .as_str()
                    .to_string(),
            );
        }
    }

    let listed = fixture
        .service
        .list_skills(SkillScopeQuery { location: global() })
        .expect("listing must recover rather than fail");

    assert_eq!(
        listed.skills.len(),
        builtin_definitions().len(),
        "every built-in with a source on disk must end up registered"
    );
    assert!(listed
        .skills
        .iter()
        .all(|skill| skill.source == SkillSource::Builtin));
    let events = fixture.filesystem.events.lock().expect("filesystem events");
    assert!(
        !events.iter().any(|event| event.starts_with("replace:")),
        "adoption must register what is on disk, not overwrite it: {events:?}"
    );
    drop(events);

    // The record has to describe the file, not the shipped definition — otherwise the registry
    // claims content the file does not have, and drift can never see the difference.
    let stored = fixture.repository.state.lock().expect("repository state");
    for record in stored.records.values() {
        assert_eq!(
            record.managed_source.content_hash,
            format!("on-disk-hash-{}", record.key.id.as_str()),
            "the adopted record must carry the hash of the file on disk"
        );
        assert_eq!(
            record.metadata,
            metadata(record.key.id.as_str()),
            "the adopted record must carry the metadata the file declares"
        );
    }
}

/// `UnregisteredSource` was detected and then ignored, leaving a reported issue that no action
/// could clear. Synchronization now resolves it the same way seeding does.
#[test]
fn synchronization_adopts_an_unregistered_source_and_clears_the_issue() {
    let fixture = Fixture::new();
    fixture
        .filesystem
        .unregistered_sources
        .lock()
        .expect("unregistered sources")
        .insert("stray-skill".to_string());
    fixture
        .filesystem
        .existing_sources
        .lock()
        .expect("existing sources")
        .insert("stray-skill".to_string());

    let before = fixture
        .service
        .detect_skill_drift(SkillScopeQuery { location: global() })
        .expect("drift detection");
    assert!(
        before.issues.iter().any(|issue| issue.issue_type
            == SkillDriftIssueType::UnregisteredSource
            && issue.skill_id == "stray-skill"),
        "the fixture must start from the state being repaired"
    );

    let synced = fixture
        .service
        .sync_skill_drift(SkillScopeQuery { location: global() })
        .expect("synchronization");
    assert!(
        synced.restored.contains(&"stray-skill".to_string()),
        "the adopted Skill must be reported as repaired: {synced:?}"
    );
    assert!(synced.failed.is_empty(), "{:?}", synced.failed);

    let after = fixture
        .service
        .detect_skill_drift(SkillScopeQuery { location: global() })
        .expect("drift detection");
    assert!(
        !after
            .issues
            .iter()
            .any(|issue| issue.issue_type == SkillDriftIssueType::UnregisteredSource),
        "registering the source must clear the issue: {:?}",
        after.issues
    );
    assert_eq!(
        fixture
            .repository
            .state
            .lock()
            .expect("repository state")
            .records
            .values()
            .filter(|record| record.key.id.as_str() == "stray-skill")
            .map(|record| record.source)
            .collect::<Vec<_>>(),
        vec![SkillSource::User],
        "a source with no built-in definition is adopted as a user Skill"
    );
}

/// An unregistered source that cannot be parsed has to say why, rather than leaving behind an
/// issue that reappears on every detection with no explanation.
#[test]
fn synchronization_reports_why_an_unusable_source_could_not_be_adopted() {
    let fixture = Fixture::new();
    for set in [
        &fixture.filesystem.unregistered_sources,
        &fixture.filesystem.existing_sources,
        &fixture.filesystem.unreadable_sources,
    ] {
        set.lock().expect("source set").insert("broken".to_string());
    }

    let synced = fixture
        .service
        .sync_skill_drift(SkillScopeQuery { location: global() })
        .expect("synchronization");

    let failure = synced
        .failed
        .iter()
        .find(|failure| failure.skill_id == "broken")
        .expect("the unusable source must be reported as failed");
    assert!(
        failure.reason.contains("could not be parsed"),
        "the failure must name the reason: {}",
        failure.reason
    );
}

/// Adoption during synchronization must respect the same deletion tombstones seeding respects.
#[test]
fn synchronization_leaves_a_deleted_builtin_unregistered() {
    let fixture = Fixture::new();
    fixture.repository.tombstone_builtin(&id("code-review"));
    for set in [
        &fixture.filesystem.unregistered_sources,
        &fixture.filesystem.existing_sources,
    ] {
        set.lock()
            .expect("source set")
            .insert("code-review".to_string());
    }

    let synced = fixture
        .service
        .sync_skill_drift(SkillScopeQuery { location: global() })
        .expect("synchronization");

    assert!(!synced.restored.contains(&"code-review".to_string()));
    assert!(
        !synced
            .failed
            .iter()
            .any(|failure| failure.skill_id == "code-review"),
        "an intentional deletion is not a failure"
    );
    assert!(
        !fixture
            .repository
            .state
            .lock()
            .expect("repository state")
            .records
            .values()
            .any(|record| record.key.id.as_str() == "code-review"),
        "synchronization must not resurrect a deleted built-in"
    );
}

/// One unusable built-in cost this installation the other five, because a single transaction
/// discarded every record when the first source refused to be created.
#[test]
fn one_unusable_builtin_does_not_strand_the_others() {
    let fixture = Fixture::new();
    {
        let mut existing = fixture
            .filesystem
            .existing_sources
            .lock()
            .expect("existing sources");
        existing.insert("tdd-discipline".to_string());
    }
    fixture
        .filesystem
        .unreadable_sources
        .lock()
        .expect("unreadable sources")
        .insert("tdd-discipline".to_string());

    let listed = fixture
        .service
        .list_skills(SkillScopeQuery { location: global() })
        .expect("an unusable source must not fail the whole listing");

    assert_eq!(
        listed.skills.len(),
        builtin_definitions().len() - 1,
        "the five usable built-ins must register even though one could not"
    );
    assert!(
        !listed
            .skills
            .iter()
            .any(|skill| skill.key.id.as_str() == "tdd-discipline"),
        "the unusable one must be absent rather than half-registered"
    );
}

/// Adoption must not undo an intentional deletion, which is an existing guarantee.
#[test]
fn adoption_leaves_an_intentionally_deleted_builtin_unregistered() {
    let fixture = Fixture::new();
    fixture.repository.tombstone_builtin(&id("code-review"));
    {
        let mut existing = fixture
            .filesystem
            .existing_sources
            .lock()
            .expect("existing sources");
        for definition in builtin_definitions() {
            existing.insert(
                definition
                    .metadata()
                    .expect("metadata")
                    .id
                    .as_str()
                    .to_string(),
            );
        }
    }

    let listed = fixture
        .service
        .list_skills(SkillScopeQuery { location: global() })
        .expect("listing");

    assert!(
        !listed
            .skills
            .iter()
            .any(|skill| skill.key.id.as_str() == "code-review"),
        "a source on disk must not resurrect a built-in the user deleted"
    );
}
