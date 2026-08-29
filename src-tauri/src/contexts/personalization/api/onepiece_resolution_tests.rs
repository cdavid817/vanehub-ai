//! What one OnePiece generation actually resolves to, over the real stack.
//!
//! These run against the real SQLite policy repository, the real Markdown memory store, the real
//! projection and the real migration health marker — not the fakes the resolver's own unit tests
//! use. What they are for is the wiring: a rule that holds in the resolver and is then defeated by
//! a projection query, a scope key that round-trips differently through SQLite than through a fake,
//! or a health marker nobody wrote.
//!
//! They enter at `PersonalizationApi::resolve_snapshot` rather than through the runtime adapter,
//! because a session's personalization mode has no persisted source yet (task 8.3) and the adapter
//! therefore resolves every session as standard. Entering here is what lets project-only and
//! temporary be exercised at all; the adapter's own translation is asserted in
//! `bootstrap::personalization_bridge_tests`, and prompt assembly from a snapshot in the OnePiece
//! adapter's tests.

use super::compatibility_tests::{fixture, mark_ready, seed, Fixture};
use crate::contexts::personalization::application::{
    CandidateSubmission, LegacySettingField, PolicyRepository, ResolutionRequest,
};
use crate::contexts::personalization::domain::{
    AgentId, CreateMemoryCandidate, MemoryAudience, MemoryCandidateOperation, MemoryDeliveryMode,
    MemoryProvenance, MemoryScope, MemorySource, MemoryType, SessionId, SessionPersonalizationMode,
    WorkspaceIdentity, WorkspaceKey, WorkspaceKind,
};
use crate::contexts::personalization::infrastructure::SqlitePolicyRepository;
use crate::platform::database::NativeDatabase;

const ONEPIECE: &str = "onepiece";

/// An Agent id that appears in no built-in list.
///
/// Registration is dynamic, so resolution must work for an Agent this code has never heard of. A
/// test that only ever asked about `onepiece` would pass just as happily against a hard-coded
/// match on the built-in set, which is precisely the shape this change exists to remove.
const SYNTHETIC_AGENT: &str = "synthetic-agent-9f2c";

fn workspace(key: &str) -> WorkspaceIdentity {
    WorkspaceIdentity::new(
        WorkspaceKey::parse(key).expect("workspace key"),
        format!("D:/code/{key}"),
        WorkspaceKind::Local,
    )
}

fn request(agent_id: &str) -> ResolutionRequest {
    ResolutionRequest {
        agent_id: AgentId::parse(agent_id).expect("agent id"),
        session_id: SessionId::parse("session-1").expect("session id"),
        workspace: None,
        session_mode: SessionPersonalizationMode::Standard,
        session_override: None,
    }
}

/// Writes the validated global policy every resolution needs, the way startup does.
///
/// Without one the resolver fails closed, which is correct and is asserted in its own tests — but
/// it would make every assertion below pass for the wrong reason. The default row is memory-
/// enabled, which is what a migrated installation lands on.
fn seed_global_policy(fixture: &Fixture) {
    let database = NativeDatabase::new(fixture.directory_path.clone()).expect("database");
    SqlitePolicyRepository::new(database)
        .seed_default_global(super::compatibility_tests::now())
        .expect("seed global policy");
}

fn eligible_names(fixture: &Fixture, request: ResolutionRequest) -> Vec<String> {
    fixture
        .api
        .resolve_snapshot(request)
        .expect("snapshot")
        .memory
        .refs
        .into_iter()
        .map(|entry| entry.name)
        .collect()
}

#[test]
fn a_global_memory_is_eligible_for_every_agent_including_one_never_registered_here() {
    let fixture = fixture("resolution-global");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    seed(
        &fixture,
        "global-note",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    assert_eq!(
        eligible_names(&fixture, request(ONEPIECE)),
        vec!["global-note".to_string()]
    );
    assert_eq!(
        eligible_names(&fixture, request(SYNTHETIC_AGENT)),
        vec!["global-note".to_string()]
    );
}

/// A workspace memory belongs to one workspace, and every other session is a different workspace.
#[test]
fn a_workspace_memory_is_eligible_only_in_the_workspace_that_owns_it() {
    let fixture = fixture("resolution-workspace");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let owner = workspace("ws_owner");
    let other = workspace("ws_other");
    seed(
        &fixture,
        "workspace-note",
        MemoryScope::Workspace {
            workspace_key: owner.key().clone(),
        },
        MemoryAudience::AllAgents,
    );

    let inside = ResolutionRequest {
        workspace: Some(owner),
        ..request(ONEPIECE)
    };
    let outside = ResolutionRequest {
        workspace: Some(other),
        ..request(ONEPIECE)
    };

    assert_eq!(
        eligible_names(&fixture, inside),
        vec!["workspace-note".to_string()]
    );
    assert!(eligible_names(&fixture, outside).is_empty());
    // No workspace at all is not the same as a different one, and neither may see it.
    assert!(eligible_names(&fixture, request(ONEPIECE)).is_empty());
}

/// Audience is per-Agent, and an Agent absent from it sees nothing — including the built-in one.
#[test]
fn an_agent_scoped_audience_admits_only_the_agents_it_names() {
    let fixture = fixture("resolution-audience");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    seed(
        &fixture,
        "for-the-synthetic-agent",
        MemoryScope::Global,
        MemoryAudience::SelectedAgents {
            agent_ids: vec![AgentId::parse(SYNTHETIC_AGENT).expect("agent id")],
        },
    );

    assert_eq!(
        eligible_names(&fixture, request(SYNTHETIC_AGENT)),
        vec!["for-the-synthetic-agent".to_string()]
    );
    assert!(eligible_names(&fixture, request(ONEPIECE)).is_empty());
}

/// Project-only keeps the workspace and drops everything global.
#[test]
fn a_project_only_session_sees_its_workspace_and_nothing_global() {
    let fixture = fixture("resolution-project-only");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let here = workspace("ws_project");
    seed(
        &fixture,
        "global-note",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    seed(
        &fixture,
        "workspace-note",
        MemoryScope::Workspace {
            workspace_key: here.key().clone(),
        },
        MemoryAudience::AllAgents,
    );

    let names = eligible_names(
        &fixture,
        ResolutionRequest {
            workspace: Some(here),
            session_mode: SessionPersonalizationMode::ProjectOnly,
            ..request(ONEPIECE)
        },
    );

    assert_eq!(names, vec!["workspace-note".to_string()]);
}

/// "Read everything global" is the one interpretation a project-isolated session must never
/// degrade to, so a project-only session that reaches resolution without a workspace is denied
/// rather than widened.
#[test]
fn a_project_only_session_without_a_workspace_is_denied_rather_than_widened() {
    let fixture = fixture("resolution-project-only-nowhere");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    seed(
        &fixture,
        "global-note",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    let snapshot = fixture
        .api
        .resolve_snapshot(ResolutionRequest {
            session_mode: SessionPersonalizationMode::ProjectOnly,
            ..request(ONEPIECE)
        })
        .expect("snapshot");

    assert!(!snapshot.memory_access.read);
    assert!(snapshot.memory.refs.is_empty());
}

/// A temporary session keeps the user's instructions and loses long-term memory in every
/// direction — including proposing one, which a read-only check would have left open.
#[test]
fn a_temporary_session_keeps_instructions_and_loses_memory_in_every_direction() {
    let fixture = fixture("resolution-temporary");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let revision = fixture.api.legacy_settings().expect("settings").revision;
    fixture
        .api
        .save_legacy_setting(
            LegacySettingField::StyleRules("Be terse.".to_string()),
            revision,
        )
        .expect("style rules");
    seed(
        &fixture,
        "global-note",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    let snapshot = fixture
        .api
        .resolve_snapshot(ResolutionRequest {
            session_mode: SessionPersonalizationMode::Temporary,
            ..request(ONEPIECE)
        })
        .expect("snapshot");

    assert!(snapshot
        .instruction_segments
        .iter()
        .any(|segment| segment.text == "Be terse."));
    assert!(!snapshot.memory_access.read);
    assert!(!snapshot.memory_access.explicit_save);
    assert!(!snapshot.memory_access.automatic_extraction);
    assert!(!snapshot.memory_access.candidate_creation);
    assert!(!snapshot.memory_access.retrieval_write);
    assert_eq!(snapshot.memory_access.delivery, MemoryDeliveryMode::None);
    assert!(snapshot.memory.refs.is_empty());
}

/// The whole point of the candidate split, asserted over the real store.
#[test]
fn an_extracted_proposal_becomes_a_candidate_and_leaves_active_memory_untouched() {
    let fixture = fixture("resolution-candidate");
    mark_ready(&fixture);
    seed_global_policy(&fixture);

    let outcome = fixture
        .api
        .submit_memory_candidates(CandidateSubmission {
            proposals: vec![MemoryCandidateOperation::Create(CreateMemoryCandidate {
                name: "proposed-note".to_string(),
                description: "Proposed by extraction".to_string(),
                memory_type: MemoryType::Project,
                content: "Never pnpm in this repo.".to_string(),
                scope: MemoryScope::Global,
                audience: MemoryAudience::AllAgents,
            })],
            source: MemorySource::OnePieceAutomatic,
            provenance: MemoryProvenance::default(),
            eligible_targets: Vec::new(),
        })
        .expect("submission");

    assert_eq!(outcome.accepted_count(), 1);
    // Not in what a generation would read, and not in the compatibility view `MEMORY.md` is
    // rebuilt from. A proposal that reached either would be an unapproved suggestion arriving as
    // an established fact.
    assert!(eligible_names(&fixture, request(ONEPIECE)).is_empty());
    assert!(fixture
        .api
        .compatibility_memories()
        .expect("listing")
        .is_empty());
}

/// Bodies are fetched at the revision the snapshot pinned.
///
/// A memory edited after a generation began is absent rather than silently newer, so the body in
/// the prompt is the body the index described.
#[test]
fn a_pinned_body_whose_record_moved_is_absent_rather_than_newer() {
    let fixture = fixture("resolution-pinned-body");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let record = seed(
        &fixture,
        "npm-only",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    let pinned = fixture
        .api
        .resolve_snapshot(request(ONEPIECE))
        .expect("snapshot")
        .memory
        .refs;
    assert_eq!(pinned.len(), 1);

    let by_handle = fixture
        .api
        .compatibility_memories_by_handle(&[record.file_name()])
        .expect("bodies");
    assert_eq!(by_handle.len(), 1);
    assert_eq!(by_handle[0].revision, pinned[0].revision);

    fixture
        .service
        .update(
            &record.id,
            record.revision,
            crate::contexts::personalization::application::UpdateMemoryPatch {
                content: Some("Uses pnpm after all.".to_string()),
                ..Default::default()
            },
        )
        .expect("edit");

    let after = fixture
        .api
        .compatibility_memories_by_handle(&[record.file_name()])
        .expect("bodies");
    assert_ne!(after[0].revision, pinned[0].revision);
}

/// Memory turned off is not "memory temporarily empty".
///
/// Reading is denied rather than the eligible set merely coming back empty, so nothing downstream
/// can mistake "the user disabled this" for "there is nothing stored yet".
#[test]
fn disabling_memory_denies_reading_rather_than_returning_an_empty_set() {
    let fixture = fixture("resolution-memory-off");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    seed(
        &fixture,
        "global-note",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    assert_eq!(eligible_names(&fixture, request(ONEPIECE)).len(), 1);

    let revision = fixture.api.legacy_settings().expect("settings").revision;
    fixture
        .api
        .save_legacy_setting(LegacySettingField::MemoryEnabled(false), revision)
        .expect("disable memory");

    let snapshot = fixture
        .api
        .resolve_snapshot(request(ONEPIECE))
        .expect("snapshot");

    assert!(!snapshot.memory_access.read);
    assert!(snapshot.memory.refs.is_empty());
}

/// Migration health is part of the answer, not a separate check a caller might forget.
#[test]
fn memory_stays_denied_until_migration_reports_ready() {
    let fixture = fixture("resolution-not-ready");
    seed_global_policy(&fixture);

    let before = fixture
        .api
        .resolve_snapshot(request(ONEPIECE))
        .expect("snapshot");
    assert!(!before.memory_access.read);

    mark_ready(&fixture);
    seed(
        &fixture,
        "global-note",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    let after = fixture
        .api
        .resolve_snapshot(request(ONEPIECE))
        .expect("snapshot");
    assert!(after.memory_access.read);
    assert_eq!(after.memory.refs.len(), 1);
}
