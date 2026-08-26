//! The composition-boundary adapter, and the evidence for why it exists.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, TimeZone, Utc};
use tempfile::TempDir;

use super::personalization_bridge::{
    instruction_block, memory_access, to_candidate_operation, LegacyMemoryPortBridge,
};
use crate::contexts::agent_runtime::application::{AgentMemoryPort, AgentMemoryProposal};
use crate::contexts::agent_runtime::domain::MemoryType as RuntimeMemoryType;
use crate::contexts::agent_runtime::infrastructure::FileAgentMemoryStore;
use crate::contexts::personalization::api::build_for_tests;
use crate::contexts::personalization::api::CompatibilitySaveInput;
use crate::contexts::personalization::application::{
    ClockPort, CreateMemoryInput, MemoryApplicationService, MigrationStatePort,
    PersonalizationApplicationError, RetrievalIndexPort,
};
use crate::contexts::personalization::domain::{
    AgentId, AgentRuntimeKind, EffectivePersonalizationSnapshot, InstructionField,
    InstructionMergeAction, MemoryAudience, MemoryCandidateOperation, MemoryId, MemoryProvenance,
    MemoryRecord, MemoryScope, MemorySensitivity, MemorySource as GovernedSource, MemoryStatus,
    MemoryType as GovernedType, MigrationPhase, MigrationState, PersonalizationResolutionContext,
    PersonalizationWarningCode, ResolvedInstructionSegment, SessionId, SessionPersonalizationMode,
    WorkspaceKey,
};
use crate::contexts::personalization::infrastructure::SqliteMigrationState;
use crate::platform::database::NativeDatabase;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

struct FixedClock;

impl ClockPort for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        now()
    }
}

#[derive(Default)]
struct FakeRetrievalIndex {
    ids: Mutex<Vec<MemoryId>>,
}

impl RetrievalIndexPort for FakeRetrievalIndex {
    fn upsert(&self, record: &MemoryRecord) -> Result<(), PersonalizationApplicationError> {
        let mut ids = self.ids.lock().expect("ids");
        if !ids.contains(&record.id) {
            ids.push(record.id.clone());
        }
        Ok(())
    }

    fn revoke(&self, id: &MemoryId) -> Result<(), PersonalizationApplicationError> {
        self.ids.lock().expect("ids").retain(|stored| stored != id);
        Ok(())
    }

    fn revoke_all(&self, ids: &[MemoryId]) -> Result<usize, PersonalizationApplicationError> {
        for id in ids {
            self.revoke(id)?;
        }
        Ok(ids.len())
    }

    fn indexed_ids(&self) -> Result<Vec<MemoryId>, PersonalizationApplicationError> {
        Ok(self.ids.lock().expect("ids").clone())
    }
}

struct Fixture {
    _directory: TempDir,
    data_root: std::path::PathBuf,
    memory_root: std::path::PathBuf,
    service: Arc<MemoryApplicationService>,
    migration_state: Arc<SqliteMigrationState>,
    personalization: crate::contexts::personalization::api::PersonalizationApi,
    bridge: LegacyMemoryPortBridge,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDir::with_prefix(format!("bridge-{label}-")).expect("temporary directory");
    let data_root = directory.path().to_path_buf();
    let database = NativeDatabase::new(data_root.clone()).expect("database");
    let memory_root = data_root.join("memory");
    let (api, service) = build_for_tests(
        memory_root.clone(),
        database.clone(),
        Arc::new(FakeRetrievalIndex::default()),
        Arc::new(FixedClock),
    );
    Fixture {
        _directory: directory,
        data_root,
        memory_root,
        service,
        migration_state: Arc::new(SqliteMigrationState::new(database)),
        personalization: api.clone(),
        bridge: LegacyMemoryPortBridge::new(api),
    }
}

fn mark_ready(fixture: &Fixture) {
    fixture
        .migration_state
        .save(&MigrationState {
            generation: 1,
            phase: MigrationPhase::Ready,
            started_at: Some(now()),
            completed_at: Some(now()),
            legacy_rows_migrated_at: Some(now()),
            last_error_code: None,
            repair_required: false,
            last_reconciled_at: None,
        })
        .expect("mark migration complete");
}

fn seed(fixture: &Fixture, name: &str) -> MemoryRecord {
    fixture
        .service
        .create(CreateMemoryInput {
            name: name.to_string(),
            description: "seeded".to_string(),
            memory_type: GovernedType::Project,
            content: format!("content for {name}"),
            scope: MemoryScope::Global,
            audience: MemoryAudience::AllAgents,
            status: MemoryStatus::Active,
            source: GovernedSource::ExplicitUser,
            provenance: MemoryProvenance::default(),
            sensitivity: MemorySensitivity::Normal,
        })
        .expect("seed")
        .record
}

/// A user-confirmed save, through the governed create path.
///
/// It goes to the API directly now. The bridge lost its write when model-originated writes became
/// proposals: nothing in a runtime writes an active memory any more, and the alias handling this
/// exercises is what the user-facing save and candidate approval will both use.
fn save(fixture: &Fixture, name: &str, content: &str) -> Result<(), String> {
    fixture
        .personalization
        .save_compatibility_memory(CompatibilitySaveInput {
            agent_id: Some("onepiece".to_string()),
            workspace: None,
            name: name.to_string(),
            description: "Package manager".to_string(),
            memory_type: Some(GovernedType::Project),
            content: content.to_string(),
            is_automatic: false,
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[test]
fn the_previous_file_store_silently_misreads_a_v2_directory() {
    // The evidence for why the bridge exists. Leaving the old store wired after migration does not
    // fail loudly — it half works, which is worse.
    let fixture = fixture("v1-misreads-v2");
    mark_ready(&fixture);
    seed(&fixture, "Plain name");
    let rich = seed(&fixture, "Ratio 1:2 rules");

    let legacy = FileAgentMemoryStore::new(&fixture.data_root).expect("legacy store");
    let seen = legacy.list_all().expect("legacy listing");

    // The colon-bearing name is legal in v2 — names are no longer filenames — but the v1 reader
    // rejects it as an invalid filename and drops the whole memory.
    assert_eq!(
        seen.len(),
        1,
        "the v1 reader silently drops a v2 memory whose name it cannot use as a filename"
    );
    assert_eq!(seen[0].name, "Plain name");
    // And what it does read, it reads incompletely: v2 writes `memory_type`, v1 looks for `type`.
    assert_eq!(
        seen[0].memory_type, None,
        "the v1 reader loses the type of a v2 record it otherwise accepts"
    );
    assert!(
        seen.iter().all(|memory| memory.id != rich.file_name()),
        "the dropped record is invisible rather than reported"
    );
}

#[test]
fn the_bridge_projects_the_compatibility_view_onto_the_old_port() {
    let fixture = fixture("projection");
    mark_ready(&fixture);
    let record = seed(&fixture, "Global");

    let seen = fixture.bridge.list_all().expect("bridge listing");
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].id,
        record.file_name(),
        "the handle is the v2 file name"
    );
    assert_eq!(seen[0].name, "Global");
    assert_eq!(seen[0].content, "content for Global");
    assert_eq!(seen[0].memory_type, Some(RuntimeMemoryType::Project));
}

#[test]
fn the_bridge_fails_closed_until_migration_is_ready() {
    let fixture = fixture("fail-closed");
    seed(&fixture, "Present but unusable");
    assert!(
        fixture.bridge.list_all().expect("listing").is_empty(),
        "an incomplete migration yields no memories rather than a partial set"
    );

    mark_ready(&fixture);
    assert_eq!(fixture.bridge.list_all().expect("listing").len(), 1);
}

#[test]
fn a_bridge_save_writes_v2_and_never_a_v1_file() {
    let fixture = fixture("save");
    mark_ready(&fixture);
    save(&fixture, "Use npm", "Use npm for this repository.").expect("save");

    let files: Vec<String> = std::fs::read_dir(&fixture.memory_root)
        .expect("read directory")
        .flatten()
        .filter_map(|entry| entry.file_name().to_str().map(String::from))
        .filter(|name| name.ends_with(".md") && name != "MEMORY.md")
        .collect();
    assert_eq!(files.len(), 1, "exactly one authoritative file was written");
    // A v1 write would have produced a name-derived filename beside the v2 one.
    assert!(
        !files.iter().any(|name| name.contains("Use")),
        "no name-derived v1 file may be created: {files:?}"
    );
}

#[test]
fn saving_an_existing_name_updates_in_place_as_it_did_before() {
    let fixture = fixture("save-existing");
    mark_ready(&fixture);
    save(&fixture, "Use npm", "First version.").expect("save");
    let first = fixture.bridge.list_all().expect("listing");
    save(&fixture, "Use npm", "Second version.").expect("save");
    let second = fixture.bridge.list_all().expect("listing");

    assert_eq!(
        second.len(),
        1,
        "a repeated name must not add a second memory"
    );
    assert_eq!(second[0].id, first[0].id, "the same record was updated");
    assert_eq!(second[0].content, "Second version.");
}

#[test]
fn an_empty_save_is_refused_without_writing_anything() {
    let fixture = fixture("empty-save");
    mark_ready(&fixture);
    assert!(save(&fixture, "Blank", "   ").is_err());
    assert!(fixture.bridge.list_all().expect("listing").is_empty());
}

#[test]
fn deleting_uses_the_handle_the_listing_handed_out() {
    let fixture = fixture("delete");
    mark_ready(&fixture);
    let record = seed(&fixture, "Subject");
    let listed = fixture.bridge.list_all().expect("listing");
    assert_eq!(listed[0].id, record.file_name());

    fixture.bridge.delete(&listed[0].id).expect("delete");
    assert!(fixture.bridge.list_all().expect("listing").is_empty());
    assert!(fixture
        .service
        .detail(&record.id)
        .expect("detail")
        .is_none());
}

#[test]
fn delete_all_clears_the_view() {
    let fixture = fixture("delete-all");
    mark_ready(&fixture);
    for index in 0..3 {
        seed(&fixture, &format!("Global {index}"));
    }
    fixture.bridge.delete_all().expect("delete all");
    assert!(fixture.bridge.list_all().expect("listing").is_empty());
}

fn resolved_snapshot(
    segments: Vec<ResolvedInstructionSegment>,
) -> EffectivePersonalizationSnapshot {
    let context = PersonalizationResolutionContext {
        agent_id: AgentId::parse("onepiece").expect("agent id"),
        session_id: SessionId::parse("session-1").expect("session id"),
        workspace: None,
        runtime_kind: AgentRuntimeKind::OnePiece,
        session_mode: SessionPersonalizationMode::Standard,
    };
    EffectivePersonalizationSnapshot {
        instruction_segments: segments,
        ..EffectivePersonalizationSnapshot::fail_closed(
            context,
            PersonalizationWarningCode::NoValidatedPolicy,
        )
    }
}

fn segment(field: InstructionField, text: &str) -> ResolvedInstructionSegment {
    ResolvedInstructionSegment {
        field,
        scope_kind: "global",
        scope_key: "global".to_string(),
        policy_revision: 1,
        merge_action: InstructionMergeAction::Replaced,
        text: text.to_string(),
    }
}

/// The migration must be invisible to a user who changed nothing.
///
/// The expected string is written out rather than compared against the old renderer, which no
/// longer exists: the flat settings type went when the last reader of it did. These are the exact
/// bytes that renderer produced, heading spacing included. A prompt that gained a blank line would
/// invalidate every provider prefix cache in the fleet and would show up in any stored-prompt
/// comparison as a change nobody made.
#[test]
fn the_rendered_instruction_block_is_byte_identical_to_the_flat_settings_rendering() {
    let governed = instruction_block(&resolved_snapshot(vec![
        segment(InstructionField::AboutUser, "Writes Rust."),
        segment(InstructionField::StyleRules, "Be terse."),
    ]))
    .expect("instruction block");

    assert_eq!(
        governed,
        "## Custom Instructions
### Response style
Be terse.

### About the user
Writes Rust."
    );
}

/// Style before the description of the user, whichever order policy resolved them in.
#[test]
fn the_rendered_instruction_block_puts_style_before_the_description_of_the_user() {
    let block = instruction_block(&resolved_snapshot(vec![
        segment(InstructionField::AboutUser, "Writes Rust."),
        segment(InstructionField::StyleRules, "Be terse."),
    ]))
    .expect("instruction block");

    let style = block.find("### Response style").expect("style");
    let about = block.find("### About the user").expect("about");
    assert!(style < about);
}

/// One field resolved renders one sub-heading, not an empty second one.
#[test]
fn the_rendered_instruction_block_omits_a_field_that_resolved_to_nothing() {
    let block = instruction_block(&resolved_snapshot(vec![segment(
        InstructionField::StyleRules,
        "Be terse.",
    )]))
    .expect("instruction block");

    assert!(block.contains("### Response style"));
    assert!(!block.contains("### About the user"));
}

/// No segments is no section. An empty block and an absent one are different states, and only the
/// second is honest about a user who configured nothing.
#[test]
fn no_resolved_segments_render_no_instruction_block_at_all() {
    assert_eq!(instruction_block(&resolved_snapshot(Vec::new())), None);
}

/// The tool-assisted sub-policy narrows extraction; it never widens it.
///
/// A snapshot that denies extraction outright stays denied however the sub-policy reads. The two
/// are combined rather than consulted in turn, so there is no order in which the narrower answer
/// can be overtaken by the broader one.
#[test]
fn the_tool_assisted_sub_policy_cannot_re_enable_extraction_the_snapshot_denied() {
    let denied = resolved_snapshot(Vec::new());

    let access = memory_access(&denied, true);

    assert!(!access.automatic_extraction);
    assert!(!access.automatic_extraction_in_tool_assisted_turns);
}

fn proposed_create() -> AgentMemoryProposal {
    AgentMemoryProposal::Create {
        name: "npm-only".to_string(),
        description: "Package manager".to_string(),
        memory_type: None,
        content: "Never pnpm in this repo.".to_string(),
    }
}

/// Scope comes from the session, never from the proposal.
///
/// A runtime proposal has no scope field at all, which is the point: letting a model name the
/// scope of what it proposes would let it widen what it is allowed to affect by asking.
#[test]
fn a_proposal_takes_the_scope_of_the_session_that_produced_it() {
    let workspace = MemoryScope::Workspace {
        workspace_key: WorkspaceKey::parse("ws_vanehub").expect("workspace key"),
    };

    let scoped = to_candidate_operation(proposed_create(), &workspace).expect("operation");
    let global = to_candidate_operation(proposed_create(), &MemoryScope::Global).expect("op");

    match (scoped, global) {
        (MemoryCandidateOperation::Create(scoped), MemoryCandidateOperation::Create(global)) => {
            assert_eq!(scoped.scope, workspace);
            assert_eq!(global.scope, MemoryScope::Global);
        }
        other => panic!("expected two create candidates, got {other:?}"),
    }
}

/// `Untyped` exists for records migrated from a store that had no types. A proposal that named no
/// type is a project note, not a claim that it predates the taxonomy.
#[test]
fn a_proposal_without_a_type_becomes_a_project_note_rather_than_untyped() {
    let operation = to_candidate_operation(proposed_create(), &MemoryScope::Global);

    match operation {
        Some(MemoryCandidateOperation::Create(create)) => {
            assert_eq!(create.memory_type, GovernedType::Project);
            assert_eq!(create.audience, MemoryAudience::AllAgents);
        }
        other => panic!("expected a create candidate, got {other:?}"),
    }
}

/// The name is how a user finds a memory again, so a silent rename would read as one having
/// disappeared. Extraction may correct what a memory says, not what it is called.
#[test]
fn an_update_proposal_can_never_carry_a_rename() {
    let operation = to_candidate_operation(
        AgentMemoryProposal::Update {
            target_id: "01K2MEM0000000000000000001".to_string(),
            expected_revision: 4,
            description: Some("Corrected".to_string()),
            content: Some("Uses pnpm after all.".to_string()),
        },
        &MemoryScope::Global,
    );

    match operation {
        Some(MemoryCandidateOperation::Update(update)) => {
            assert_eq!(update.name, None);
            assert_eq!(update.expected_target_revision, 4);
        }
        other => panic!("expected an update candidate, got {other:?}"),
    }
}

/// A target id that is not a legal memory id is dropped rather than guessed at.
#[test]
fn a_proposal_naming_an_unparseable_target_translates_to_nothing() {
    let operation = to_candidate_operation(
        AgentMemoryProposal::Archive {
            target_id: "not a memory id".to_string(),
            expected_revision: 1,
        },
        &MemoryScope::Global,
    );

    assert!(operation.is_none());
}

/// The mode the session recorded decides how far personalization reaches.
///
/// The two contexts keep separate enums on purpose, so this translation is the only place they
/// meet. A value this build cannot parse resolves as temporary rather than standard: the narrow
/// reading is the only safe one when a newer build may have written a mode meaning "retain less".
#[test]
fn a_stored_session_mode_translates_and_an_unknown_one_narrows() {
    for (stored, expected) in [
        ("standard", SessionPersonalizationMode::Standard),
        ("", SessionPersonalizationMode::Standard),
        ("project-only", SessionPersonalizationMode::ProjectOnly),
        ("temporary", SessionPersonalizationMode::Temporary),
        (
            "something-a-later-build-wrote",
            SessionPersonalizationMode::Temporary,
        ),
    ] {
        assert_eq!(
            super::personalization_bridge::session_mode_from(stored),
            expected,
            "{stored} translated wrongly"
        );
    }
}
