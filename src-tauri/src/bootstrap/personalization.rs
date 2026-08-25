//! Composition root for personalization, and the startup maintenance it gates on.
//!
//! Everything concrete is assembled here rather than inside the context, and the two adapters that
//! reach into other contexts — the pre-file row store and the pre-governance settings — live here
//! for the same reason: personalization publishes ports, and satisfying one from another context
//! belongs at the boundary, not inside the provider.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::contexts::agent_runtime::application::{AgentMemoryPort, AgentRegistryRepository};
use crate::contexts::agent_runtime::infrastructure::{
    migrate_memory_rows, FileAgentMemoryStore, SqliteAgentMemoryRepository,
};
use crate::contexts::desktop::api::{
    DesktopSettingsApi, PersonalizationSaveRejection, PersonalizationSettingsBridge,
    PersonalizationSettingsSnapshot,
};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::personalization::api::PersonalizationApi;
use crate::contexts::personalization::application::{
    AgentCapabilityPort, ClockPort, LegacyMemoryMigrationPorts, LegacyMemoryMigrationService,
    LegacyPersonalizationSettings, LegacyPersonalizationSettingsPort, LegacyRowMigrationPort,
    LegacySettingField, LegacySettingsCompatibility, LegacySettingsView, MemoryApplicationService,
    PersonalizationApplicationError, PolicyResolutionService, RetrievalIndexPort,
    StartupMaintenancePorts, StartupMaintenanceService, WorkspaceIdentityResolver,
};
use crate::contexts::personalization::domain::{
    MemoryId, MemoryRecord, PersonalizationRuntimeCapabilities,
};
use crate::contexts::personalization::infrastructure::{
    FileLegacyMemorySource, MaintenanceGate, MarkdownDerivedIndex, MarkdownMemoryRepository,
    SqliteCandidateRepository, SqliteLegacyAddressAlias, SqliteLegacyPolicyMigration,
    SqliteMemoryProjection, SqliteMigrationJournal, SqliteMigrationState, SqlitePolicyRepository,
    UuidMemoryIdGenerator,
};
use crate::contexts::retrieval::api::RetrievalApi;
use crate::platform::database::NativeDatabase;

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// The directory both the pre-v2 store and the governed store live in.
const MEMORY_DIRECTORY_NAME: &str = "memory";

/// Everything the application needs from personalization, plus the maintenance that has to finish
/// before any of it answers with data.
pub(crate) struct PersonalizationAssembly {
    pub(crate) api: PersonalizationApi,
    pub(crate) maintenance: Arc<StartupMaintenanceService>,
    /// Resolves one immutable snapshot per generation. Assembled here and handed to the runtime
    /// adapters as they land; nothing else in this context owns policy resolution.
    pub(crate) resolver: Arc<PolicyResolutionService>,
}

/// Assembles the whole stack. Does not run maintenance — that is `spawn_startup_maintenance`, so a
/// caller cannot accidentally block startup on it.
pub(crate) fn assemble_personalization(
    database: NativeDatabase,
    data_root: &Path,
    settings: DesktopSettingsApi,
    agents: Arc<dyn AgentRegistryRepository>,
    retrieval_index: Arc<dyn RetrievalIndexPort>,
    clock: Arc<dyn ClockPort>,
) -> std::result::Result<PersonalizationAssembly, String> {
    let settings_for_bridge = settings.clone();
    let memory_root: PathBuf = data_root.join(MEMORY_DIRECTORY_NAME);
    let repository = Arc::new(
        MarkdownMemoryRepository::new(memory_root.clone(), Arc::new(UuidMemoryIdGenerator))
            .map_err(|error| format!("Memory directory is unavailable: {error}"))?,
    );
    let projection = Arc::new(SqliteMemoryProjection::new(database.clone()));
    let projection_for_resolver = projection.clone();
    let memories = Arc::new(MemoryApplicationService::new(
        repository.clone(),
        repository.clone(),
        projection.clone(),
        Arc::new(MarkdownDerivedIndex::new(memory_root.clone())),
        retrieval_index,
        clock.clone(),
    ));

    let policies = Arc::new(SqlitePolicyRepository::new(database.clone()));
    let policies_for_resolver = policies.clone();
    let aliases = Arc::new(SqliteLegacyAddressAlias::new(database.clone()));
    let state = Arc::new(SqliteMigrationState::new(database.clone()));
    let sources = Arc::new(
        FileLegacyMemorySource::new(memory_root.clone(), repository.lock())
            .map_err(|error| format!("Legacy memory directory is unavailable: {error}"))?,
    );

    let migration = Arc::new(LegacyMemoryMigrationService::new(
        LegacyMemoryMigrationPorts {
            sources,
            repository: repository.clone(),
            projection,
            journal: Arc::new(SqliteMigrationJournal::new(database.clone())),
            aliases: aliases.clone(),
            identity: Arc::new(WorkspaceIdentityResolver::for_this_platform()),
            ids: Arc::new(UuidMemoryIdGenerator),
            clock: clock.clone(),
        },
    ));

    // One gate object, shared by the orchestration and the published boundary, so both name the
    // same directory. The store builds its own from the same root; they agree because the lock file
    // is derived from the root, not passed around.
    let gate: Arc<dyn crate::contexts::personalization::application::MaintenanceGatePort> =
        Arc::new(
            MaintenanceGate::new(&memory_root)
                .map_err(|error| format!("Maintenance gate is unavailable: {error}"))?,
        );
    let maintenance = Arc::new(StartupMaintenanceService::new(StartupMaintenancePorts {
        gate: gate.clone(),
        state,
        policies: policies.clone(),
        policy_migration: Arc::new(SqliteLegacyPolicyMigration::new(database.clone())),
        legacy_settings: Arc::new(DesktopLegacySettingsAdapter::new(settings.clone())),
        rows: Arc::new(RowStoreMigrationAdapter::new(
            database.clone(),
            data_root.to_path_buf(),
        )),
        memories: migration,
        derived: memories.clone(),
        clock: clock.clone(),
    }));

    // Kept alive so the candidate table has an owner once the review UI lands; assembling it here
    // rather than later keeps every personalization construction in one place.
    let _candidates = Arc::new(SqliteCandidateRepository::new(database));

    let api = PersonalizationApi::new(
        memories,
        gate,
        maintenance.clone(),
        Arc::new(LegacySettingsCompatibility::new(policies, clock)),
        aliases,
        Arc::new(WorkspaceIdentityResolver::for_this_platform()),
    );
    // Bound here rather than by the caller, so there is no assembly order in which the settings
    // page is live while the legacy rows are still its truth.
    settings_for_bridge.bind_personalization(Arc::new(GovernedSettingsBridge::new(api.clone())));
    let resolver = Arc::new(PolicyResolutionService::new(
        policies_for_resolver,
        Arc::new(RegistryAgentCapabilities::new(agents)),
        projection_for_resolver,
        maintenance.clone(),
    ));
    Ok(PersonalizationAssembly {
        api,
        maintenance,
        resolver,
    })
}

/// Runs startup maintenance off the startup path.
///
/// A thread rather than an await: nothing else may wait on this. Settings, workspaces, CLI
/// management and agent generation all start and stay usable while it runs — only memory is
/// unavailable, and only until it finishes.
pub(crate) fn spawn_startup_maintenance(
    maintenance: Arc<StartupMaintenanceService>,
    diagnostics: Arc<dyn DiagnosticLogPort>,
) {
    std::thread::spawn(move || {
        let health = maintenance.run();
        let severity = if health.allows_memory_use() {
            LogSeverity::Info
        } else {
            LogSeverity::Warn
        };
        let mut context = BTreeMap::new();
        // A code, never a path and never a memory body: this line is written to a log file.
        context.insert("health".to_string(), health.as_str().to_string());
        let _ = diagnostics.write_diagnostic(DiagnosticLog {
            severity,
            category: "personalization.maintenance".to_string(),
            message: "Personalization startup maintenance finished.".to_string(),
            context,
        });
    });
}

/// Satisfies the desktop settings page's personalization bridge from the governed policy.
///
/// This is the cutover itself: once bound, the five legacy keys are read from and written to the
/// policy, and the settings rows behind them are never consulted again for those fields. They stay
/// deserializable so nothing that still reads the whole row set breaks, but they are no longer
/// anybody's truth.
pub(crate) struct GovernedSettingsBridge {
    personalization: PersonalizationApi,
}

impl GovernedSettingsBridge {
    pub(crate) fn new(personalization: PersonalizationApi) -> Self {
        Self { personalization }
    }

    fn snapshot(view: LegacySettingsView) -> PersonalizationSettingsSnapshot {
        PersonalizationSettingsSnapshot {
            about_user: view.settings.about_user.unwrap_or_default(),
            style_rules: view.settings.style_rules.unwrap_or_default(),
            custom_instructions_enabled: view.settings.custom_instructions_enabled.unwrap_or(false),
            memory_enabled: view.settings.memory_enabled.unwrap_or(false),
            tool_assisted_extraction_enabled: view
                .settings
                .tool_assisted_extraction_enabled
                .unwrap_or(false),
            revision: view.revision,
        }
    }
}

impl PersonalizationSettingsBridge for GovernedSettingsBridge {
    fn view(&self) -> std::result::Result<PersonalizationSettingsSnapshot, String> {
        self.personalization
            .legacy_settings()
            .map(Self::snapshot)
            .map_err(|error| error.to_string())
    }

    fn save(
        &self,
        key: &str,
        value: &str,
        expected_revision: u64,
    ) -> std::result::Result<PersonalizationSettingsSnapshot, PersonalizationSaveRejection> {
        let field = LegacySettingField::from_key_and_value(key, value).ok_or_else(|| {
            PersonalizationSaveRejection::Unavailable(
                "this key is not a personalization setting".to_string(),
            )
        })?;
        match self
            .personalization
            .save_legacy_setting(field, expected_revision)
        {
            Ok(view) => Ok(Self::snapshot(view)),
            Err(PersonalizationApplicationError::RevisionConflict(conflict)) => {
                Err(PersonalizationSaveRejection::Conflict {
                    expected: conflict.expected,
                    current: conflict.current,
                })
            }
            Err(error) => Err(PersonalizationSaveRejection::Unavailable(error.to_string())),
        }
    }

    fn owns(&self, key: &str) -> bool {
        // Asked of the same rule the write path uses, so a key can never be claimed here and then
        // refused there.
        LegacySettingField::from_key_and_value(key, "").is_some()
    }
}

/// Satisfies personalization's Agent capability port from the Agent registry.
///
/// Capabilities come from the launch kind the registry records, never from an Agent id. Nothing
/// here enumerates Agents or branches on which one it is, so an Agent registered while the
/// application is running resolves through exactly the same path as one that shipped with it —
/// and an Agent nobody registered resolves to `None` rather than to a default surface it never
/// declared.
pub(crate) struct RegistryAgentCapabilities {
    registry: Arc<dyn AgentRegistryRepository>,
}

impl RegistryAgentCapabilities {
    pub(crate) fn new(registry: Arc<dyn AgentRegistryRepository>) -> Self {
        Self { registry }
    }

    /// What a launch shape can actually consume.
    ///
    /// A CLI owns its own context and its own instruction files, so VaneHub may add instructions to
    /// what it sends but must not claim to manage a memory index inside it or to compact it. An API
    /// Agent has no external context, so VaneHub owns the whole prompt and every surface applies.
    fn for_launch(kind: &str) -> PersonalizationRuntimeCapabilities {
        match kind {
            "api" => PersonalizationRuntimeCapabilities {
                supports_custom_instructions: true,
                supports_memory_index: true,
                supports_selected_memory_bodies: true,
                supports_automatic_extraction: true,
            },
            "cli" => PersonalizationRuntimeCapabilities {
                supports_custom_instructions: true,
                supports_memory_index: true,
                // The CLI runs its own turn loop, so VaneHub has no point at which to inspect a
                // completed exchange and propose a memory from it.
                supports_selected_memory_bodies: false,
                supports_automatic_extraction: false,
            },
            // Browser and native-desktop Agents are launched, not driven, and a launch shape this
            // build has never heard of is the same situation: VaneHub composes no prompt for them,
            // so it declares nothing rather than assuming.
            _ => PersonalizationRuntimeCapabilities::none(),
        }
    }
}

impl AgentCapabilityPort for RegistryAgentCapabilities {
    fn capabilities(
        &self,
        agent_id: &crate::contexts::personalization::domain::AgentId,
    ) -> Result<Option<PersonalizationRuntimeCapabilities>> {
        let found = self.registry.find(agent_id.as_str()).map_err(|error| {
            PersonalizationApplicationError::Storage(format!("agent_registry_unreadable: {error}"))
        })?;
        Ok(found.map(|agent| Self::for_launch(agent.launch().kind_str())))
    }
}

/// Personalization's retrieval coordination, satisfied by the pull-based retrieval worker.
///
/// Retrieval reconciles its documents against a snapshot of what exists rather than accepting
/// per-record pushes, so `upsert` and `revoke` are wake-ups: they say the snapshot changed.
/// `indexed_ids` reports nothing, and deliberately — orphan removal belongs to the reconciliation
/// that reads that snapshot, and answering with retrieval's document ids would have two components
/// deleting the same rows on different schedules.
///
/// Deferred because retrieval is assembled after the memory port it depends on. Before it is bound,
/// every call is a no-op: the worker performs a full reconcile on its first pass anyway, so nothing
/// that happens in the gap is lost.
#[derive(Default)]
pub(crate) struct DeferredRetrievalIndex {
    worker: std::sync::OnceLock<RetrievalApi>,
}

impl DeferredRetrievalIndex {
    pub(crate) fn bind(&self, retrieval: RetrievalApi) {
        let _ = self.worker.set(retrieval);
    }

    fn wake(&self) {
        if let Some(retrieval) = self.worker.get() {
            retrieval.wake_worker();
        }
    }
}

impl RetrievalIndexPort for DeferredRetrievalIndex {
    fn upsert(&self, _record: &MemoryRecord) -> Result<()> {
        self.wake();
        Ok(())
    }

    fn revoke(&self, _id: &MemoryId) -> Result<()> {
        self.wake();
        Ok(())
    }

    fn revoke_all(&self, ids: &[MemoryId]) -> Result<usize> {
        self.wake();
        Ok(ids.len())
    }

    fn indexed_ids(&self) -> Result<Vec<MemoryId>> {
        Ok(Vec::new())
    }
}

/// The pre-file row conversion, satisfied from the context that still owns those rows.
///
/// Frozen: it runs the existing conversion unchanged. What changes is who decides *when* — the
/// orchestration, once, gated by a durable marker, rather than a second startup path racing the
/// file migration over the same directory.
struct RowStoreMigrationAdapter {
    database: NativeDatabase,
    data_root: PathBuf,
}

impl RowStoreMigrationAdapter {
    fn new(database: NativeDatabase, data_root: PathBuf) -> Self {
        Self {
            database,
            data_root,
        }
    }
}

impl LegacyRowMigrationPort for RowStoreMigrationAdapter {
    fn convert_rows_to_legacy_files(&self) -> Result<usize> {
        let store = FileAgentMemoryStore::new(&self.data_root).map_err(|error| {
            PersonalizationApplicationError::Storage(format!("row_store_unavailable: {error}"))
        })?;
        let rows = SqliteAgentMemoryRepository::new(self.database.clone())
            .list_all()
            .map_err(|error| {
                PersonalizationApplicationError::Storage(format!("row_store_unreadable: {error}"))
            })?;
        let outcome = migrate_memory_rows(&store, &rows).map_err(|error| {
            PersonalizationApplicationError::Storage(format!("row_migration_failed: {error}"))
        })?;
        Ok(outcome.migrated)
    }
}

/// Reads the pre-governance personalization fields once, so they can be migrated.
///
/// Every field is reported as `Some`, because the settings service resolves defaults before this
/// can see them and there is no way left to tell "never saved" from "saved as the default". That
/// distinction was preserved for as long as it existed; here the resolved value is the honest one.
struct DesktopLegacySettingsAdapter {
    settings: DesktopSettingsApi,
}

impl DesktopLegacySettingsAdapter {
    fn new(settings: DesktopSettingsApi) -> Self {
        Self { settings }
    }
}

impl LegacyPersonalizationSettingsPort for DesktopLegacySettingsAdapter {
    fn load(&self) -> Result<LegacyPersonalizationSettings> {
        let view = self.settings.get_settings().map_err(|error| {
            PersonalizationApplicationError::Storage(format!("legacy_settings_unreadable: {error}"))
        })?;
        let settings = view.settings;
        Ok(LegacyPersonalizationSettings {
            about_user: Some(settings.custom_instructions_about_user().to_string()),
            style_rules: Some(settings.custom_instructions_style_rules().to_string()),
            custom_instructions_enabled: Some(settings.custom_instructions_enabled()),
            memory_enabled: Some(settings.memory_enabled()),
            tool_assisted_extraction_enabled: Some(settings.memory_tool_assisted_chats_enabled()),
        })
    }
}
