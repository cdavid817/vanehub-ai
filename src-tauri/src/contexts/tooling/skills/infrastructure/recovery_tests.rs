//! Recovery against the real repository and the real filesystem.
//!
//! The application-layer tests prove the reconciliation logic with fakes. These prove the same
//! recovery on the production stack — a migrated SQLite database and `ManagedSkillFilesystem`
//! writing real directories — because the defect lived exactly at the seam between them: the
//! registry answered "does it exist" in one store and the filesystem answered it in another.

use super::{ManagedSkillFilesystem, SqliteSkillRepository, SystemSkillClock};
use crate::contexts::tooling::skills::application::{
    SkillApplicationError, SkillApplicationService, SkillLogEvent, SkillLogLevel, SkillLoggingPort,
    SkillRecord, SkillRepository, SkillScopeQuery, SkillWorkspaceSelectionPort,
};
use crate::contexts::tooling::skills::domain::{
    builtin_definitions, SkillLocation, SkillScope, SkillSource,
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
