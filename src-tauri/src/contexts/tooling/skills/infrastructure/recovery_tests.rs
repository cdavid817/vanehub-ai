//! Recovery against the real repository and the real filesystem.
//!
//! The application-layer tests prove the reconciliation logic with fakes. These prove the same
//! recovery on the production stack — a migrated SQLite database and `ManagedSkillFilesystem`
//! writing real directories — because the defect lived exactly at the seam between them: the
//! registry answered "does it exist" in one store and the filesystem answered it in another.

use super::{
    filesystem::compose_document, CachedEffectiveSkillCatalog, FilesystemSkillLayerProvider,
    LayeredSkillPackageReader, ManagedSkillFilesystem, SqliteSkillRepository, SystemSkillClock,
    SystemSkillDerivedCache, SystemSkillPackages,
};
use crate::contexts::tooling::skills::application::{
    BuiltinCleanupStatus, BuiltinReconciliationOutcome, BuiltinReconciliationState,
    ManagedSkillSource, SkillApplicationError, SkillApplicationService, SkillFilesystemPort,
    SkillLayerProvider, SkillLogEvent, SkillLogLevel, SkillLoggingPort, SkillPackageMaterializer,
    SkillPackageReader, SkillReconciliationRepository, SkillRecord, SkillRepository,
    SkillScopeQuery, SkillWorkspaceSelectionPort, BUILTIN_RECONCILIATION_VERSION,
};
use crate::contexts::tooling::skills::domain::{
    builtin_definitions, SkillAvailability, SkillDriftIssueType, SkillId, SkillKey, SkillLayer,
    SkillLocation, SkillOrigin, SkillScope, SkillSource,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct RecordingLogging {
    events: Mutex<Vec<SkillLogEvent>>,
}

impl SkillLoggingPort for RecordingLogging {
    fn record(&self, event: &SkillLogEvent) -> Result<(), SkillApplicationError> {
        self.events.lock().expect("log events").push(event.clone());
        Ok(())
    }
}

struct NoWorkspace;

impl SkillWorkspaceSelectionPort for NoWorkspace {
    fn select_workspace_directory(&self) -> Result<Option<String>, SkillApplicationError> {
        Ok(None)
    }
}

struct Stack {
    _home: TempDirectory,
    _data: TempDirectory,
    service: SkillApplicationService,
    logging: Arc<RecordingLogging>,
    repository: Arc<SqliteSkillRepository>,
    home_root: std::path::PathBuf,
}

impl Stack {
    fn new(label: &str) -> Self {
        let home = TempDirectory::new(&format!("{label}-home"));
        let data = TempDirectory::new(&format!("{label}-data"));
        let database = NativeDatabase::new(data.path().to_path_buf()).expect("test database");
        database.connection().expect("migrated database");
        let repository = Arc::new(SqliteSkillRepository::new(database));
        let logging = Arc::new(RecordingLogging::default());
        let home_root = home.path().to_path_buf();
        let service = SkillApplicationService::new(
            repository.clone(),
            repository.clone(),
            Arc::new(ManagedSkillFilesystem::with_home_root(home_root.clone())),
            Arc::new(NoWorkspace),
            Arc::new(SystemSkillClock),
            logging.clone(),
        );
        Self {
            _home: home,
            _data: data,
            service,
            logging,
            repository,
            home_root,
        }
    }

    fn global(&self) -> SkillLocation {
        SkillLocation::new(SkillScope::Global, None).expect("global location")
    }

    /// Reproduces the observed installation: the source directories are on disk and the registry
    /// knows nothing about them, with no deletion tombstones to explain the absence.
    fn diverge(&self) {
        self.service
            .list_skills(SkillScopeQuery {
                location: self.global(),
            })
            .expect("initial seeding");
        let records = self.repository_records();
        assert_eq!(records.len(), builtin_definitions().len());
        for record in &records {
            assert!(
                std::path::Path::new(&record.managed_source.skill_md_path).is_file(),
                "the source must exist on disk before the registry is cleared"
            );
        }
        for record in records {
            // No tombstone: the observed installation had none, which is what makes the state a
            // divergence rather than an intentional deletion.
            self.repository
                .delete_skill(&record.key, false, "2026-01-01T00:00:00Z")
                .expect("clear registry row");
        }
        assert!(self.repository_records().is_empty());
        self.logging.events.lock().expect("log events").clear();
    }

    fn repository_records(&self) -> Vec<SkillRecord> {
        self.repository.list(&self.global()).expect("list records")
    }
}

#[test]
fn a_registry_that_lost_its_rows_recovers_on_the_next_listing() {
    let stack = Stack::new("skill-recovery");
    stack.diverge();

    let listed = stack
        .service
        .list_skills(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("listing must recover rather than fail");

    assert_eq!(
        listed.skills.len(),
        builtin_definitions().len(),
        "every built-in whose source survived must be registered again"
    );
    assert!(listed
        .skills
        .iter()
        .all(|skill| skill.source == SkillSource::Builtin));
    assert!(
        stack
            .logging
            .events
            .lock()
            .expect("log events")
            .iter()
            .all(|event| event.level != SkillLogLevel::Error),
        "an already-present built-in is an expected state, not an error"
    );
}

#[test]
fn recovery_keeps_a_users_edits_to_a_builtin_source() {
    let stack = Stack::new("skill-recovery-edit");
    stack.diverge();

    let edited = stack.home_root.join(".vanehub/skills/code-review/SKILL.md");
    let original = std::fs::read_to_string(&edited).expect("read built-in source");
    let modified = original.replace("description: ", "description: EDITED ");
    assert_ne!(
        original, modified,
        "the fixture must actually change the file"
    );
    std::fs::write(&edited, &modified).expect("write edited source");

    stack
        .service
        .list_skills(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("listing");

    assert_eq!(
        std::fs::read_to_string(&edited).expect("read back"),
        modified,
        "adoption must not overwrite a file the user edited"
    );
    // Drift compares the record against disk, and an adopted record already describes disk, so it
    // has nothing to report. Divergence from the shipped definition is a different comparison and
    // is reported by seeding itself.
    let drift = stack
        .service
        .detect_skill_drift(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("drift detection");
    assert!(
        !drift
            .issues
            .iter()
            .any(|issue| issue.skill_id == "code-review"),
        "an adopted record must not be reported as drifted against its own source: {:?}",
        drift.issues
    );

    let record = stack
        .repository_records()
        .into_iter()
        .find(|record| record.key.id.as_str() == "code-review")
        .expect("the edited built-in must still be registered");
    assert!(
        record.metadata.description.starts_with("EDITED"),
        "the record must describe the file, not the shipped definition: {}",
        record.metadata.description
    );

    // Nothing downstream compares an adopted built-in against what shipped, so seeding has to say
    // it — otherwise the divergence is invisible everywhere.
    let events = stack.logging.events.lock().expect("log events");
    assert!(
        events.iter().any(|event| event
            .message
            .contains("differs from the shipped definition")
            && event.message.contains("code-review")),
        "the divergence must be reported: {:?}",
        events
            .iter()
            .map(|event| &event.message)
            .collect::<Vec<_>>()
    );
}

/// Adoption has to produce an ordinary registry record, not a second-class one — a Skill nobody
/// can bind is no better than a Skill that is missing.
#[test]
fn an_adopted_builtin_can_be_bound_and_mounted_like_any_other() {
    let stack = Stack::new("skill-recovery-binding");
    stack.diverge();

    let listed = stack
        .service
        .list_skills(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("listing");
    assert_eq!(listed.skills.len(), builtin_definitions().len());

    let key = listed
        .skills
        .iter()
        .find(|skill| skill.key.id.as_str() == "code-review")
        .expect("adopted built-in")
        .key
        .clone();
    let bound = stack
        .service
        .bind_skill_to_cli_agent(key, "claude-code".to_string())
        .expect("an adopted Skill must be bindable");

    let binding = bound
        .bindings
        .iter()
        .find(|binding| binding.agent_id == "claude-code")
        .expect("the binding must be recorded");
    assert!(binding.mounted, "the Skill must actually mount");
    assert!(
        std::path::Path::new(&binding.mounted_path).exists(),
        "the mount must exist on disk: {}",
        binding.mounted_path
    );
}

#[test]
fn one_unusable_source_does_not_cost_the_other_builtins() {
    let stack = Stack::new("skill-recovery-unusable");
    stack.diverge();

    let broken = stack
        .home_root
        .join(".vanehub/skills/tdd-discipline/SKILL.md");
    std::fs::write(&broken, "not a skill document").expect("corrupt the source");

    let listed = stack
        .service
        .list_skills(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("an unusable source must not fail the whole listing");

    assert_eq!(
        listed.skills.len(),
        builtin_definitions().len() - 1,
        "the usable built-ins must register even though one could not"
    );
    let events = stack.logging.events.lock().expect("log events");
    let failure = events
        .iter()
        .find(|event| event.level == SkillLogLevel::Error)
        .expect("the unusable source must be reported");
    assert_eq!(failure.skill_id.as_deref(), Some("tdd-discipline"));
    assert!(
        failure.message.contains("could not be parsed"),
        "the diagnostic must name the reason: {}",
        failure.message
    );
}

struct RuntimeStack {
    _home: TempDirectory,
    _data: TempDirectory,
    service: SkillApplicationService,
    logging: Arc<RecordingLogging>,
    repository: Arc<SqliteSkillRepository>,
    filesystem: Arc<ManagedSkillFilesystem>,
    packages: Arc<SystemSkillPackages>,
    materializer: Arc<SystemSkillDerivedCache>,
    home_root: std::path::PathBuf,
}

impl RuntimeStack {
    fn new(label: &str) -> Self {
        let home = TempDirectory::new(&format!("{label}-home"));
        let data = TempDirectory::new(&format!("{label}-data"));
        let database = NativeDatabase::new(data.path().to_path_buf()).expect("test database");
        database.connection().expect("migrated database");
        let repository = Arc::new(SqliteSkillRepository::new(database));
        let logging = Arc::new(RecordingLogging::default());
        let home_root = home.path().to_path_buf();
        let filesystem = Arc::new(ManagedSkillFilesystem::with_home_root(home_root.clone()));
        let packages = Arc::new(SystemSkillPackages);
        let catalog = Arc::new(CachedEffectiveSkillCatalog::new(vec![
            Arc::new(FilesystemSkillLayerProvider::with_root(
                SkillLayer::User,
                home_root.clone(),
            )),
            packages.clone(),
        ]));
        let materializer = Arc::new(SystemSkillDerivedCache::with_root(
            home_root.join(".vanehub/cache/skills/system"),
            packages.clone(),
        ));
        let package_reader = Arc::new(LayeredSkillPackageReader::with_home_root(
            packages.clone(),
            home_root.clone(),
        ));
        let service = SkillApplicationService::new(
            repository.clone(),
            repository.clone(),
            filesystem.clone(),
            Arc::new(NoWorkspace),
            Arc::new(SystemSkillClock),
            logging.clone(),
        )
        .with_effective_catalog(catalog)
        .with_system_materializer(materializer.clone())
        .with_builtin_reconciliation(packages.clone(), repository.clone(), filesystem.clone())
        .with_effective_package_reader(package_reader);
        Self {
            _home: home,
            _data: data,
            service,
            logging,
            repository,
            filesystem,
            packages,
            materializer,
            home_root,
        }
    }

    fn global(&self) -> SkillLocation {
        SkillLocation::new(SkillScope::Global, None).expect("global")
    }

    fn package(
        &self,
        id: &str,
    ) -> crate::contexts::tooling::skills::application::SkillPackageDescriptor {
        self.packages
            .inventory(None)
            .expect("System inventory")
            .into_iter()
            .find(|package| package.metadata.id.as_str() == id)
            .expect("System package")
    }

    fn write_legacy(&self, id: &str, enabled: bool, edit_description: bool) -> SkillRecord {
        let package = self.package(id);
        let mut document = self
            .packages
            .read_document(&package)
            .expect("System document");
        if edit_description {
            document.metadata.description =
                format!("User override: {}", document.metadata.description);
        }
        let directory = self.home_root.join(".vanehub/skills").join(id);
        let skill_file = directory.join("SKILL.md");
        std::fs::create_dir_all(&directory).expect("legacy directory");
        std::fs::write(&skill_file, compose_document(&document)).expect("legacy document");
        SkillRecord {
            key: SkillKey::new(package.metadata.id, self.global()),
            source: SkillSource::Builtin,
            enabled,
            managed_source: ManagedSkillSource {
                skill_dir: directory.to_string_lossy().to_string(),
                skill_md_path: skill_file.to_string_lossy().to_string(),
                content_hash: self.filesystem.content_hash_for(&document),
            },
            metadata: document.metadata,
            bindings: Vec::new(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
            resolved_metadata: None,
        }
    }

    fn save(&self, record: &SkillRecord) {
        self.repository
            .save_skills(std::slice::from_ref(record), &[])
            .expect("save legacy record");
    }

    fn state(&self, id: &str) -> BuiltinReconciliationState {
        self.repository
            .builtin_reconciliation(&SkillId::parse(id).expect("id"))
            .expect("reconciliation state")
            .expect("state exists")
    }

    fn records(&self) -> Vec<SkillRecord> {
        self.repository.list(&self.global()).expect("records")
    }
}

#[test]
fn runtime_reconciliation_preserves_authority_and_survives_partial_failure_and_repeats() {
    let stack = RuntimeStack::new("effective-runtime-matrix");
    let unchanged_disabled = stack.write_legacy("code-review", false, false);
    stack.save(&unchanged_disabled);
    let divergent = stack.write_legacy("tdd-discipline", true, true);
    stack.save(&divergent);
    let missing_record = stack.write_legacy("api-doc-generation", true, false);
    let tombstoned = stack.write_legacy("code-security-scan", true, false);
    stack.save(&tombstoned);
    stack
        .repository
        .delete_skill(&tombstoned.key, true, "2026-01-01T00:00:00Z")
        .expect("tombstone");
    let invalid = stack.home_root.join(".vanehub/skills/unit-test-generation");
    std::fs::create_dir_all(&invalid).expect("invalid directory");
    std::fs::write(invalid.join("SKILL.md"), "invalid").expect("invalid source");

    let first = stack
        .service
        .list_skills(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("partial reconciliation remains available");
    assert_eq!(first.skills.len(), 5);

    let unchanged = first
        .skills
        .iter()
        .find(|record| record.key.id.as_str() == "code-review")
        .expect("unchanged record");
    assert_eq!(unchanged.source, SkillSource::Builtin);
    assert!(!unchanged.enabled, "legacy disablement must survive");
    assert!(unchanged.managed_source.skill_dir.contains("cache"));
    assert!(!std::path::Path::new(&unchanged_disabled.managed_source.skill_dir).exists());
    assert_eq!(
        stack.state("code-review").cleanup_status,
        BuiltinCleanupStatus::Complete
    );

    let override_record = first
        .skills
        .iter()
        .find(|record| record.key.id.as_str() == "tdd-discipline")
        .expect("override record");
    assert_eq!(override_record.source, SkillSource::User);
    assert!(std::path::Path::new(&divergent.managed_source.skill_md_path).is_file());
    assert_eq!(
        stack.state("tdd-discipline").outcome,
        BuiltinReconciliationOutcome::MigratedOverride
    );
    assert_eq!(
        stack.state("api-doc-generation").outcome,
        BuiltinReconciliationOutcome::System,
        "a surviving source without a row must be recovered"
    );
    assert_eq!(
        stack.state("code-security-scan").outcome,
        BuiltinReconciliationOutcome::Deleted
    );
    assert!(stack
        .records()
        .iter()
        .all(|record| record.key.id.as_str() != "code-security-scan"));
    assert_eq!(
        stack.state("unit-test-generation").outcome,
        BuiltinReconciliationOutcome::Invalid
    );
    assert!(stack
        .logging
        .events
        .lock()
        .expect("events")
        .iter()
        .any(|event| {
            event.level == SkillLogLevel::Error
                && event.skill_id.as_deref() == Some("unit-test-generation")
                && !event.message.contains("invalid\n")
        }));

    let second = stack
        .service
        .list_skills(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("repeat reconciliation");
    assert_eq!(second.skills.len(), first.skills.len());
    assert!(!std::path::Path::new(&missing_record.managed_source.skill_dir).exists());
}

#[test]
fn pending_cleanup_is_recovered_and_explicit_restore_reveals_system_authority() {
    let stack = RuntimeStack::new("effective-runtime-recovery");
    let legacy = stack.write_legacy("code-review", true, false);
    let package = stack.package("code-review");
    let managed_source = stack
        .materializer
        .materialize(&package)
        .expect("materialize System package");
    let record = SkillRecord {
        key: legacy.key.clone(),
        source: SkillSource::Builtin,
        enabled: true,
        managed_source,
        metadata: package.metadata.clone(),
        bindings: Vec::new(),
        created_at: legacy.created_at.clone(),
        updated_at: legacy.updated_at.clone(),
        resolved_metadata: None,
    };
    let pending = BuiltinReconciliationState {
        skill_id: package.metadata.id.clone(),
        reconciliation_version: BUILTIN_RECONCILIATION_VERSION,
        outcome: BuiltinReconciliationOutcome::System,
        system_revision: package.revision.clone(),
        legacy_revision: Some(legacy.managed_source.content_hash.clone()),
        cleanup_status: BuiltinCleanupStatus::Pending,
        backup_path: None,
        error_code: None,
        enabled: true,
        deletion_intent: false,
        effective_layer: SkillLayer::System,
        origin: SkillOrigin::Shipped,
        availability: SkillAvailability::Available,
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    };
    stack
        .repository
        .save_builtin_reconciliation(&pending, Some(&record), false)
        .expect("simulate committed migration before cleanup");

    stack
        .service
        .list_skills(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("resume cleanup");
    let recovered = stack.state("code-review");
    assert_eq!(recovered.cleanup_status, BuiltinCleanupStatus::Complete);
    assert!(recovered
        .backup_path
        .as_deref()
        .is_some_and(|path| std::path::Path::new(path).is_dir()));

    let delete_key = SkillKey::new(
        SkillId::parse("tdd-discipline").expect("id"),
        stack.global(),
    );
    stack
        .service
        .list_skills(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("seed remaining System records");
    stack
        .service
        .delete_skill(delete_key)
        .expect("delete System record");
    let restored = stack
        .service
        .restore_builtin(SkillId::parse("tdd-discipline").expect("id"))
        .expect("restore System authority");
    assert_eq!(restored.source, SkillSource::Builtin);
    let effective = restored.effective_metadata();
    assert_eq!(effective.layer, SkillLayer::System);
    assert!(effective.immutable);
    assert!(restored.managed_source.skill_dir.contains("cache"));
    assert!(!stack
        .repository
        .deleted_builtin_ids()
        .expect("tombstones")
        .contains(&restored.key.id));
}

#[cfg(windows)]
#[expect(
    clippy::permissions_set_readonly_false,
    reason = "the Windows read-only file attribute must be cleared to exercise cache repair"
)]
fn make_test_file_writable(path: &str) {
    let mut permissions = std::fs::metadata(path)
        .expect("cache metadata")
        .permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(path, permissions).expect("allow test corruption");
}

#[cfg(unix)]
fn make_test_file_writable(path: &str) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)
        .expect("cache metadata")
        .permissions();
    permissions.set_mode(permissions.mode() | 0o200);
    std::fs::set_permissions(path, permissions).expect("allow test corruption");
}

#[test]
fn drift_repairs_immutable_system_cache_but_adopts_mutable_override_changes() {
    let stack = RuntimeStack::new("effective-runtime-drift");
    let override_source = stack.write_legacy("tdd-discipline", true, true);
    stack.save(&override_source);
    let listed = stack
        .service
        .list_skills(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("initial reconciliation");
    let system = listed
        .skills
        .iter()
        .find(|record| record.key.id.as_str() == "code-review")
        .expect("System record")
        .clone();
    make_test_file_writable(&system.managed_source.skill_md_path);
    std::fs::write(
        &system.managed_source.skill_md_path,
        "corrupt derived cache",
    )
    .expect("corrupt derived cache");

    let override_content = std::fs::read_to_string(&override_source.managed_source.skill_md_path)
        .expect("override content");
    std::fs::write(
        &override_source.managed_source.skill_md_path,
        override_content.replace("User override:", "User revised override:"),
    )
    .expect("revise override");

    let tombstone_key = SkillKey::new(
        SkillId::parse("code-security-scan").expect("id"),
        stack.global(),
    );
    stack
        .service
        .delete_skill(tombstone_key)
        .expect("intentional deletion");
    let drift = stack
        .service
        .detect_skill_drift(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("drift");
    assert!(
        drift.issues.iter().any(|issue| {
            issue.skill_id == "code-review"
            && issue.issue_type
                == crate::contexts::tooling::skills::domain::SkillDriftIssueType::MetadataChanged
        }),
        "drift issues: {:?}",
        drift.issues
    );
    assert!(drift.issues.iter().any(|issue| {
        issue.skill_id == "tdd-discipline"
            && issue.issue_type
                == crate::contexts::tooling::skills::domain::SkillDriftIssueType::MetadataChanged
    }));
    assert!(drift.issues.iter().any(|issue| {
        issue.skill_id == "code-security-scan"
            && issue.issue_type
                == crate::contexts::tooling::skills::domain::SkillDriftIssueType::DeletedBuiltin
    }));

    let synced = stack
        .service
        .sync_skill_drift(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("sync");
    assert!(
        synced.failed.is_empty(),
        "sync failures: {:?}",
        synced.failed
    );
    assert!(synced.restored.contains(&"code-review".to_string()));
    assert!(synced.restored.contains(&"tdd-discipline".to_string()));
    let package = stack.package("code-review");
    let expected = compose_document(
        &stack
            .packages
            .read_document(&package)
            .expect("System document"),
    );
    assert_eq!(
        std::fs::read_to_string(&system.managed_source.skill_md_path).expect("repaired cache"),
        expected
    );
    assert!(stack
        .records()
        .into_iter()
        .find(|record| record.key.id.as_str() == "tdd-discipline")
        .expect("override")
        .metadata
        .description
        .starts_with("User revised override:"));
}

#[test]
fn reported_legacy_builtin_cache_drift_converges_for_the_entire_affected_set() {
    const AFFECTED_SKILLS: [&str; 4] = [
        "api-doc-generation",
        "code-review",
        "code-security-scan",
        "readme-generation",
    ];
    let stack = RuntimeStack::new("reported-legacy-builtin-drift");
    let listed = stack
        .service
        .list_skills(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("seed immutable System packages");

    for skill_id in AFFECTED_SKILLS {
        let record = listed
            .skills
            .iter()
            .find(|record| record.key.id.as_str() == skill_id)
            .expect("affected System record");
        make_test_file_writable(&record.managed_source.skill_md_path);
        let current = std::fs::read_to_string(&record.managed_source.skill_md_path)
            .expect("current derived cache");
        std::fs::write(
            &record.managed_source.skill_md_path,
            format!("{current}\nlegacy-registry-cache-snapshot: {skill_id}\n"),
        )
        .expect("write legacy cache divergence");
    }

    let before = stack
        .service
        .detect_skill_drift(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("detect reported drift");
    for skill_id in AFFECTED_SKILLS {
        assert!(
            before.issues.iter().any(|issue| {
                issue.skill_id == skill_id
                    && issue.issue_type == SkillDriftIssueType::MetadataChanged
            }),
            "the legacy fixture must reproduce drift for {skill_id}: {:?}",
            before.issues
        );
    }

    let synchronized = stack
        .service
        .sync_skill_drift(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("synchronize affected built-ins");
    assert!(
        synchronized.failed.is_empty(),
        "sync failures: {:?}",
        synchronized.failed
    );
    for skill_id in AFFECTED_SKILLS {
        assert!(synchronized.restored.contains(&skill_id.to_string()));
        assert!(synchronized
            .resolved_from
            .issues
            .iter()
            .any(|issue| issue.skill_id == skill_id));
    }

    let after = stack
        .service
        .detect_skill_drift(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("detect post-repair drift");
    assert!(
        after
            .issues
            .iter()
            .all(|issue| !AFFECTED_SKILLS.contains(&issue.skill_id.as_str())),
        "repaired built-ins must not reappear: {:?}",
        after.issues
    );

    let repeated = stack
        .service
        .sync_skill_drift(SkillScopeQuery {
            location: stack.global(),
        })
        .expect("repeat synchronization");
    assert!(repeated.resolved_from.issues.is_empty());
    assert!(repeated.restored.is_empty());
}
