use super::*;
use crate::contexts::tooling::skills::domain::{
    builtin_definitions, RegisteredSkillInspection, SkillBindingInspection, SkillBindingPlan,
    SkillDomainError, SkillDriftInspection, SkillDriftIssue, SkillDriftIssueType, SkillId,
    SkillKey, SkillLocation, SkillMetadata, SkillMountObservation, SkillMountPath, SkillScope,
    SkillSource, SkillSourceInspection,
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

    fn create_source(
        &self,
        _transaction: &SkillFilesystemTransaction,
        location: &SkillLocation,
        id: &SkillId,
        _document: &SkillDocument,
    ) -> Result<ManagedSkillSource, SkillApplicationError> {
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

    fn remove_skill(
        &self,
        _transaction: &SkillFilesystemTransaction,
        record: &SkillRecord,
    ) -> Result<(), SkillApplicationError> {
        self.push_event(format!("remove:{}", record.key.id.as_str()));
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
        _records: &[SkillRecord],
        deleted_builtin_ids: &[SkillId],
    ) -> Result<SkillDriftInspection, SkillApplicationError> {
        Ok(self
            .inspection
            .lock()
            .expect("drift inspection")
            .clone()
            .unwrap_or_else(|| SkillDriftInspection {
                location: location.clone(),
                registered: Vec::new(),
                unregistered_sources: Vec::new(),
                deleted_builtin_ids: deleted_builtin_ids.to_vec(),
            }))
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
        assert!(!log.context.values().any(|value| value.contains(sensitive_path)));
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
