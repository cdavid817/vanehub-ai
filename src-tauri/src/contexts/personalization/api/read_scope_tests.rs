//! The governed read scope over the real stack: real v2 files, the real SQLite projection, the
//! real health marker, one process epoch.
//!
//! These are the acceptance matrix's personalization-side rows (`unify-memory-read-scope`,
//! MR-01 to MR-23): the same eligibility domain for the index page, the complete recall relation
//! and the pinned bodies; fail-closed contexts; authoritative revalidation at delivery; and the
//! maintenance/egress boundary that the background index consults.

use std::fs;

use super::compatibility_tests::{fixture as open_fixture, mark_ready, seed, Fixture};
use crate::contexts::personalization::application::{
    MemoryReadRefusal, PolicyRepository, ResolutionRequest, UpdateMemoryPatch,
};
use crate::contexts::personalization::domain::{
    content_hash, AgentId, EffectivePersonalizationSnapshot, MemoryAudience, MemoryId,
    MemoryReadContext, MemoryReadHandle, MemoryRecord, MemoryScope, MemoryStatus,
    PersonalizationPolicyPatch, PersonalizationPolicyScope, PolicyToggle, SessionId,
    SessionPersonalizationMode, WorkspaceBinding, WorkspaceIdentity, WorkspaceKey, WorkspaceKind,
};
use crate::contexts::personalization::infrastructure::SqlitePolicyRepository;
use crate::platform::database::NativeDatabase;

const AGENT_A: &str = "agent-a";
const AGENT_B: &str = "agent-b";
const W1: &str = "ws_one";
const W2: &str = "ws_two";

fn agent(id: &str) -> AgentId {
    AgentId::parse(id).expect("agent id")
}

fn workspace_key(key: &str) -> WorkspaceKey {
    WorkspaceKey::parse(key).expect("workspace key")
}

fn workspace(key: &str) -> WorkspaceIdentity {
    WorkspaceIdentity::new(
        workspace_key(key),
        format!("/code/{key}"),
        WorkspaceKind::Local,
    )
}

fn in_workspace(key: &str) -> MemoryScope {
    MemoryScope::Workspace {
        workspace_key: workspace_key(key),
    }
}

fn selected(ids: &[&str]) -> MemoryAudience {
    MemoryAudience::SelectedAgents {
        agent_ids: ids.iter().map(|id| agent(id)).collect(),
    }
}

fn seed_global_policy(fixture: &Fixture) {
    let database = NativeDatabase::new(fixture.directory_path.clone()).expect("database");
    SqlitePolicyRepository::new(database)
        .seed_default_global(super::compatibility_tests::now())
        .expect("seed global policy");
}

fn disable_global_access(fixture: &Fixture) {
    let current = fixture
        .api
        .policy(&PersonalizationPolicyScope::Global)
        .expect("policy")
        .expect("global policy row");
    fixture
        .api
        .patch_policy(
            &PersonalizationPolicyScope::Global,
            Some(current.revision()),
            PersonalizationPolicyPatch {
                global_memory_access_mode: Some(PolicyToggle::Disabled),
                ..PersonalizationPolicyPatch::default()
            },
        )
        .expect("disable global access");
}

fn request(
    agent_id: &str,
    mode: SessionPersonalizationMode,
    workspace_key: Option<&str>,
) -> ResolutionRequest {
    ResolutionRequest {
        agent_id: agent(agent_id),
        session_id: SessionId::parse("session-1").expect("session id"),
        workspace: workspace_key.map(workspace),
        session_mode: mode,
        session_override: None,
    }
}

/// The corpus every scope row is judged against. Authors are deliberately mismatched with
/// audiences (every record is produced by `agent-b`), so any admission that leaks through is a
/// scope/audience decision and never provenance.
struct Corpus {
    g_all: MemoryRecord,
    g_a: MemoryRecord,
    w1_all: MemoryRecord,
    w1_a: MemoryRecord,
}

fn seed_corpus(fixture: &Fixture) -> Corpus {
    let corpus = Corpus {
        g_all: seed(
            fixture,
            "G-all",
            MemoryScope::Global,
            MemoryAudience::AllAgents,
        ),
        g_a: seed(fixture, "G-A", MemoryScope::Global, selected(&[AGENT_A])),
        w1_all: seed(
            fixture,
            "W1-all",
            in_workspace(W1),
            MemoryAudience::AllAgents,
        ),
        w1_a: seed(fixture, "W1-A", in_workspace(W1), selected(&[AGENT_A])),
    };
    seed(fixture, "G-B", MemoryScope::Global, selected(&[AGENT_B]));
    seed(fixture, "W1-B", in_workspace(W1), selected(&[AGENT_B]));
    seed(
        fixture,
        "W2-all",
        in_workspace(W2),
        MemoryAudience::AllAgents,
    );
    let archived = seed(
        fixture,
        "archived",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    fixture
        .service
        .update(
            &archived.id,
            archived.revision,
            UpdateMemoryPatch {
                status: Some(MemoryStatus::Archived),
                ..UpdateMemoryPatch::default()
            },
        )
        .expect("archive");
    corpus
}

fn resolve(
    fixture: &Fixture,
    agent_id: &str,
    mode: SessionPersonalizationMode,
    binding: WorkspaceBinding,
) -> (EffectivePersonalizationSnapshot, MemoryReadContext) {
    let workspace_key = match &binding {
        WorkspaceBinding::Resolved(key) => Some(key.as_str().to_string()),
        _ => None,
    };
    let snapshot = fixture
        .api
        .resolve_snapshot(request(agent_id, mode, workspace_key.as_deref()))
        .expect("snapshot");
    let context = fixture
        .api
        .freeze_memory_read_context(&snapshot, binding, "generation-1", None);
    (snapshot, context)
}

fn standard_in_w1(fixture: &Fixture) -> (EffectivePersonalizationSnapshot, MemoryReadContext) {
    resolve(
        fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Resolved(workspace_key(W1)),
    )
}

fn relation_ids(fixture: &Fixture, context: &MemoryReadContext) -> Vec<MemoryId> {
    let mut ids: Vec<MemoryId> = fixture
        .api
        .open_memory_query(context)
        .expect("relation")
        .entries
        .into_iter()
        .map(|handle| handle.id)
        .collect();
    ids.sort();
    ids
}

fn verified_ids(
    fixture: &Fixture,
    snapshot: &EffectivePersonalizationSnapshot,
    context: &MemoryReadContext,
) -> Vec<MemoryId> {
    let mut ids: Vec<MemoryId> = fixture
        .api
        .verify_memory_refs(context, &snapshot.memory.refs)
        .into_iter()
        .map(|entry| entry.handle.id)
        .collect();
    ids.sort();
    ids
}

fn sorted(mut ids: Vec<MemoryId>) -> Vec<MemoryId> {
    ids.sort();
    ids
}

fn handles_for(records: &[&MemoryRecord]) -> Vec<MemoryReadHandle> {
    records
        .iter()
        .map(|record| MemoryReadHandle::of(record))
        .collect()
}

fn memory_file(fixture: &Fixture, id: &MemoryId) -> std::path::PathBuf {
    fixture
        .directory_path
        .join("memory")
        .join(format!("{id}.md"))
}

// --- MR-01 / MR-02 / MR-03 / MR-05 ---------------------------------------------------------------

#[test]
fn a_standard_session_reads_global_and_its_workspace_by_exact_audience_on_every_surface() {
    let fixture = open_fixture("read-scope-standard");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let corpus = seed_corpus(&fixture);
    let (snapshot, context) = standard_in_w1(&fixture);

    let expected = sorted(vec![
        corpus.g_all.id.clone(),
        corpus.g_a.id.clone(),
        corpus.w1_all.id.clone(),
        corpus.w1_a.id.clone(),
    ]);
    // Index page, complete recall relation and pinned bodies all decide the same set.
    assert_eq!(verified_ids(&fixture, &snapshot, &context), expected);
    assert_eq!(relation_ids(&fixture, &context), expected);
    let bodies = fixture
        .api
        .read_pinned_memories(
            &context,
            &handles_for(&[&corpus.g_all, &corpus.g_a, &corpus.w1_all, &corpus.w1_a]),
        )
        .expect("bodies");
    assert_eq!(bodies.len(), 4);
    assert!(bodies
        .iter()
        .any(|body| body.content == "content for W1-A" && body.scope_hint == W1));
}

#[test]
fn disabling_global_access_or_project_only_keeps_the_workspace_and_drops_global_everywhere() {
    for (label, mode, disable) in [
        (
            "read-scope-global-disabled",
            SessionPersonalizationMode::Standard,
            true,
        ),
        (
            "read-scope-project-only",
            SessionPersonalizationMode::ProjectOnly,
            false,
        ),
    ] {
        let fixture = open_fixture(label);
        mark_ready(&fixture);
        seed_global_policy(&fixture);
        if disable {
            disable_global_access(&fixture);
        }
        let corpus = seed_corpus(&fixture);
        let (snapshot, context) = resolve(
            &fixture,
            AGENT_A,
            mode,
            WorkspaceBinding::Resolved(workspace_key(W1)),
        );

        let expected = sorted(vec![corpus.w1_all.id.clone(), corpus.w1_a.id.clone()]);
        assert_eq!(
            verified_ids(&fixture, &snapshot, &context),
            expected,
            "{label}"
        );
        assert_eq!(relation_ids(&fixture, &context), expected, "{label}");
        // The workspace body is actually loadable -- the old compatibility reader could never
        // return it -- and the global one is refused even when asked for by exact handle.
        let bodies = fixture
            .api
            .read_pinned_memories(&context, &handles_for(&[&corpus.w1_a, &corpus.g_all]))
            .expect("bodies");
        assert_eq!(bodies.len(), 1, "{label}");
        assert_eq!(bodies[0].content, "content for W1-A");
    }
}

#[test]
fn an_absent_workspace_reads_global_only_while_an_unresolved_one_reads_nothing() {
    let fixture = open_fixture("read-scope-absent-vs-unresolved");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let corpus = seed_corpus(&fixture);

    let (snapshot, absent) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Absent,
    );
    let expected = sorted(vec![corpus.g_all.id.clone(), corpus.g_a.id.clone()]);
    assert_eq!(verified_ids(&fixture, &snapshot, &absent), expected);
    assert_eq!(relation_ids(&fixture, &absent), expected);

    let (snapshot, unresolved) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Unresolved,
    );
    assert!(verified_ids(&fixture, &snapshot, &unresolved).is_empty());
    assert_eq!(
        fixture.api.open_memory_query(&unresolved).unwrap_err(),
        MemoryReadRefusal::ReadDenied
    );
    assert_eq!(
        fixture
            .api
            .read_pinned_memories(&unresolved, &handles_for(&[&corpus.g_all]))
            .unwrap_err(),
        MemoryReadRefusal::ReadDenied
    );
}

// --- MR-04 -----------------------------------------------------------------------------------

#[test]
fn a_temporary_session_is_refused_by_every_governed_read_before_any_record_is_touched() {
    let fixture = open_fixture("read-scope-temporary");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let corpus = seed_corpus(&fixture);
    let (snapshot, context) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Temporary,
        WorkspaceBinding::Resolved(workspace_key(W1)),
    );

    assert!(!context.permits_any_read());
    assert!(snapshot.memory.refs.is_empty());
    assert!(fixture
        .api
        .verify_memory_refs(&context, &snapshot.memory.refs)
        .is_empty());
    assert_eq!(
        fixture.api.open_memory_query(&context).unwrap_err(),
        MemoryReadRefusal::ReadDenied
    );
    assert_eq!(
        fixture
            .api
            .read_pinned_memories(&context, &handles_for(&[&corpus.g_all]))
            .unwrap_err(),
        MemoryReadRefusal::ReadDenied
    );
}

// --- MR-06 -----------------------------------------------------------------------------------

#[test]
fn a_selected_audience_admits_the_exact_stable_id_and_never_a_prefix_case_or_wildcard_variant() {
    let fixture = open_fixture("read-scope-exact-audience");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let record = seed(
        &fixture,
        "for-agent-1",
        MemoryScope::Global,
        selected(&["agent-1"]),
    );

    let ids_for = |agent_id: &str| {
        let (snapshot, context) = resolve(
            &fixture,
            agent_id,
            SessionPersonalizationMode::Standard,
            WorkspaceBinding::Absent,
        );
        (
            verified_ids(&fixture, &snapshot, &context),
            relation_ids(&fixture, &context),
        )
    };
    let admitted = (vec![record.id.clone()], vec![record.id.clone()]);
    assert_eq!(ids_for("agent-1"), admitted);
    for impostor in [
        "agent-10", "agent-1_", "AGENT-1", "agent-%", "agent", "gent-1",
    ] {
        assert_eq!(
            ids_for(impostor),
            (Vec::new(), Vec::new()),
            "{impostor} must not be admitted"
        );
    }
}

// --- MR-07 -----------------------------------------------------------------------------------

#[test]
fn a_projected_row_this_build_cannot_classify_is_excluded_rather_than_guessed_eligible() {
    let fixture = open_fixture("read-scope-invalid-rows");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let good = seed(
        &fixture,
        "good",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    // Corrupt three projection rows directly, the way a newer build or a partial migration could
    // leave them: an unknown scope kind, an unparseable audience, and a global row carrying a
    // workspace key.
    let database = NativeDatabase::new(fixture.directory_path.clone()).expect("database");
    let connection = database.connection().expect("connection");
    for (suffix, sql) in [
        ("unknown-scope", "scope_kind = 'repository'"),
        (
            "bad-audience",
            "audience_json = '{\"selected_agents\": [\"agent-a\"'",
        ),
        ("global-with-workspace", "workspace_key = 'ws_zzz'"),
    ] {
        let record = seed(
            &fixture,
            suffix,
            MemoryScope::Global,
            MemoryAudience::AllAgents,
        );
        connection
            .execute(
                &format!("UPDATE personalization_memory_projection SET {sql} WHERE memory_id = ?1"),
                [record.id.as_str()],
            )
            .expect("corrupt row");
    }

    let (snapshot, context) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Absent,
    );
    assert_eq!(snapshot.memory.eligible_total, 1);
    assert_eq!(
        snapshot
            .memory
            .exclusions
            .iter()
            .find(|entry| entry.reason
                == crate::contexts::personalization::domain::PersonalizationExclusionReason::InvalidRecord)
            .map(|entry| entry.count),
        Some(3)
    );
    assert_eq!(
        verified_ids(&fixture, &snapshot, &context),
        vec![good.id.clone()]
    );
    assert_eq!(relation_ids(&fixture, &context), vec![good.id]);
}

// --- MR-11 -----------------------------------------------------------------------------------

#[test]
fn the_recall_relation_is_complete_past_the_two_hundred_ref_injection_page() {
    let fixture = open_fixture("read-scope-beyond-two-hundred");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    for index in 0..205 {
        seed(
            &fixture,
            &format!("note-{index:03}"),
            MemoryScope::Global,
            MemoryAudience::AllAgents,
        );
    }
    let (snapshot, context) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Absent,
    );

    assert_eq!(snapshot.memory.refs.len(), 200);
    assert!(snapshot.memory.truncated);
    assert_eq!(snapshot.memory.eligible_total, 205);
    let relation = fixture.api.open_memory_query(&context).expect("relation");
    assert!(relation.complete);
    assert_eq!(relation.entries.len(), 205);
}

// --- MR-15 / MR-16 / MR-18 -------------------------------------------------------------------

#[test]
fn a_pinned_handle_is_dropped_when_the_record_moves_and_never_replaced_by_the_newer_version() {
    let fixture = open_fixture("read-scope-stale-handles");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let record = seed(
        &fixture,
        "npm-only",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    let (_, context) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Absent,
    );
    let pinned = MemoryReadHandle::of(&record);

    // An in-app edit advances the revision: the old handle yields nothing, and the body that
    // arrives for a fresh handle is the new text, not a mix.
    let updated = fixture
        .service
        .update(
            &record.id,
            record.revision,
            UpdateMemoryPatch {
                content: Some("Uses pnpm after all.".to_string()),
                ..UpdateMemoryPatch::default()
            },
        )
        .expect("edit")
        .record;
    let stale = fixture
        .api
        .read_pinned_memories(&context, std::slice::from_ref(&pinned))
        .expect("read");
    assert!(
        stale.is_empty(),
        "the old handle must not be silently upgraded"
    );
    let fresh = fixture
        .api
        .read_pinned_memories(&context, &[MemoryReadHandle::of(&updated)])
        .expect("read");
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].content, "Uses pnpm after all.");

    // An audience narrowed without touching the body: same revision after the projection is
    // rebuilt by the service, but the authority fingerprint moved.
    let narrowed = fixture
        .service
        .update(
            &updated.id,
            updated.revision,
            UpdateMemoryPatch {
                audience: Some(selected(&[AGENT_B])),
                ..UpdateMemoryPatch::default()
            },
        )
        .expect("narrow")
        .record;
    assert!(fixture
        .api
        .read_pinned_memories(&context, &[MemoryReadHandle::of(&updated)])
        .expect("read")
        .is_empty());
    // And the narrowed record is not admitted for agent-a even by its current handle.
    assert!(fixture
        .api
        .read_pinned_memories(&context, &[MemoryReadHandle::of(&narrowed)])
        .expect("read")
        .is_empty());
}

#[test]
fn an_external_edit_that_keeps_the_revision_is_detected_by_the_hash_and_dropped() {
    let fixture = open_fixture("read-scope-external-edit");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let record = seed(
        &fixture,
        "npm-only",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    let (_, context) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Absent,
    );
    let pinned = MemoryReadHandle::of(&record);

    // Rewrite the body and keep the frontmatter revision, the way an editor would, but keep the
    // file internally consistent by recomputing its own hash line.
    let path = memory_file(&fixture, &record.id);
    let raw = fs::read_to_string(&path).expect("read file");
    let (header, _body) = raw.split_once("\n---\n\n").expect("frontmatter");
    let new_body = "Edited outside the application.";
    let header = header
        .lines()
        .map(|line| {
            if line.starts_with("content_hash:") {
                format!("content_hash: {}", content_hash(new_body))
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&path, format!("{header}\n---\n\n{new_body}")).expect("write file");

    let bodies = fixture
        .api
        .read_pinned_memories(&context, &[pinned])
        .expect("read");
    assert!(
        bodies.is_empty(),
        "same revision, different hash: not the pinned body"
    );

    // A torn file -- body changed, hash line not -- is unreadable and equally absent.
    fs::write(&path, format!("{header}\n---\n\nTorn body.")).expect("write torn file");
    assert!(fixture
        .api
        .read_pinned_memories(&context, &[MemoryReadHandle::of(&record)])
        .expect("read")
        .is_empty());
}

#[test]
fn an_archived_record_is_dropped_at_delivery_even_when_its_handle_is_otherwise_intact() {
    let fixture = open_fixture("read-scope-archived-at-delivery");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let record = seed(
        &fixture,
        "retired",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    let (snapshot, context) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Absent,
    );
    assert_eq!(snapshot.memory.refs.len(), 1);
    let archived = fixture
        .service
        .update(
            &record.id,
            record.revision,
            UpdateMemoryPatch {
                status: Some(MemoryStatus::Archived),
                ..UpdateMemoryPatch::default()
            },
        )
        .expect("archive")
        .record;

    // The frozen snapshot still lists it; delivery does not honour that listing.
    assert!(fixture
        .api
        .verify_memory_refs(&context, &snapshot.memory.refs)
        .is_empty());
    assert!(fixture
        .api
        .read_pinned_memories(&context, &[MemoryReadHandle::of(&archived)])
        .expect("read")
        .is_empty());
    assert!(relation_ids(&fixture, &context).is_empty());
}

// --- MR-19 -----------------------------------------------------------------------------------

#[test]
fn two_records_with_one_name_are_told_apart_by_id_and_only_the_pinned_one_is_delivered() {
    let fixture = open_fixture("read-scope-same-name");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let first = seed(
        &fixture,
        "package-manager",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    let second = seed(
        &fixture,
        "package-manager",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    assert_ne!(first.id, second.id);
    let (_, context) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Absent,
    );

    let bodies = fixture
        .api
        .read_pinned_memories(&context, &[MemoryReadHandle::of(&second)])
        .expect("read");
    assert_eq!(bodies.len(), 1);
    assert_eq!(bodies[0].handle.id, second.id);
    assert_eq!(bodies[0].content, "content for package-manager");

    // An invented id resolves to nothing, not to "the first record with that name".
    let invented = MemoryReadHandle {
        id: MemoryId::parse("01K2INVENTED00000000000001").expect("id"),
        ..MemoryReadHandle::of(&first)
    };
    assert!(fixture
        .api
        .read_pinned_memories(&context, &[invented])
        .expect("read")
        .is_empty());
}

// --- MR-09 / MR-10 -----------------------------------------------------------------------------

#[test]
fn a_context_not_minted_by_this_host_or_altered_after_minting_is_refused() {
    let fixture = open_fixture("read-scope-inauthentic");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let corpus = seed_corpus(&fixture);
    let (_, context) = standard_in_w1(&fixture);

    let mut widened = context.clone();
    widened.workspace_allowed = Some(workspace_key(W2));
    assert_eq!(
        fixture.api.open_memory_query(&widened).unwrap_err(),
        MemoryReadRefusal::Inauthentic
    );
    assert_eq!(
        fixture
            .api
            .read_pinned_memories(&widened, &handles_for(&[&corpus.g_all]))
            .unwrap_err(),
        MemoryReadRefusal::Inauthentic
    );

    // The same fields minted by another process epoch: a second stack over its own directory
    // issues a context that this one must not honour.
    let other = open_fixture("read-scope-inauthentic-other-epoch");
    mark_ready(&other);
    seed_global_policy(&other);
    let (_, foreign) = resolve(
        &other,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Resolved(workspace_key(W1)),
    );
    assert_eq!(
        fixture.api.open_memory_query(&foreign).unwrap_err(),
        MemoryReadRefusal::Inauthentic
    );
}

// --- MR-22 / MR-23 -----------------------------------------------------------------------------

#[test]
fn index_maintenance_lists_every_active_record_and_marks_scoped_or_restricted_bodies_keyword_only()
{
    let fixture = open_fixture("read-scope-index-maintenance");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let corpus = seed_corpus(&fixture);
    seed(
        &fixture,
        "candidate",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    let records = fixture
        .api
        .index_maintenance_records()
        .expect("maintenance records");
    let restriction = |id: &MemoryId| {
        records
            .iter()
            .find(|record| record.handle.id == *id)
            .map(|record| record.egress_restricted)
    };
    // Scoped and audience-restricted records are indexed (locally searchable) but never eligible
    // for body egress; global all-Agent records are.
    assert_eq!(restriction(&corpus.g_all.id), Some(false));
    assert_eq!(restriction(&corpus.g_a.id), Some(true));
    assert_eq!(restriction(&corpus.w1_all.id), Some(true));
    assert_eq!(restriction(&corpus.w1_a.id), Some(true));
    // The archived seed from the corpus is absent; nothing inactive is indexed.
    assert!(
        records
            .iter()
            .all(|record| record.handle.id != corpus.g_all.id
                || record.content == "content for G-all")
    );
    assert_eq!(
        records.len(),
        8,
        "seven active corpus records plus the candidate-named active one"
    );

    let queued = |record: &MemoryRecord| (record.id.clone(), record.content_hash());
    let decisions = fixture.api.embedding_egress(&[
        queued(&corpus.g_all),
        queued(&corpus.w1_all),
        queued(&corpus.g_a),
        (corpus.g_all.id.clone(), "sha256:stale".to_string()),
    ]);
    let permitted: Vec<bool> = decisions.iter().map(|d| d.permitted).collect();
    assert_eq!(permitted, vec![true, false, false, false]);

    // A public record queued, then re-scoped before dispatch: the same authoritative check now
    // refuses the body that the queue still holds.
    fixture
        .service
        .update(
            &corpus.g_all.id,
            corpus.g_all.revision,
            UpdateMemoryPatch {
                scope: Some(in_workspace(W1)),
                ..UpdateMemoryPatch::default()
            },
        )
        .expect("re-scope");
    let after = fixture.api.embedding_egress(&[queued(&corpus.g_all)]);
    assert!(!after[0].permitted);
}

#[test]
fn a_context_frozen_under_one_maintenance_generation_is_refused_after_the_store_moves_on() {
    // MR-26: a context is bound to the migration generation it was frozen under; a store that has
    // since gone into repair or been re-migrated is a different store.
    let fixture = open_fixture("read-scope-generation");
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    seed(
        &fixture,
        "note",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    let (_, context) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Absent,
    );
    assert_eq!(relation_ids(&fixture, &context).len(), 1);

    let mut state = super::compatibility_tests::migration_state_of(&fixture);
    state.repair_required = true;
    super::compatibility_tests::save_migration_state(&fixture, &state);

    assert_eq!(
        fixture.api.open_memory_query(&context).unwrap_err(),
        MemoryReadRefusal::Unhealthy
    );
}

// --- MR-32 -----------------------------------------------------------------------------------

/// Enumerates `count` eligible records and reports how long the complete relation and a bounded
/// pinned read take. Bodies are never loaded for the relation: the projection answers it alone.
/// What one pool size measured: the relation and the bounded body read, as P50/P95 over
/// `SAMPLES` runs, plus the planner's answer and the process's peak resident set.
struct ScaleMeasurement {
    relation_p50: std::time::Duration,
    relation_p95: std::time::Duration,
    pinned_p50: std::time::Duration,
    pinned_p95: std::time::Duration,
    bodies_loaded: usize,
    reconcile: std::time::Duration,
    plan: Vec<String>,
    peak_rss_kib: Option<u64>,
}

const SAMPLES: usize = 20;

fn percentile(samples: &mut [std::time::Duration], percent: usize) -> std::time::Duration {
    samples.sort();
    let index = (samples.len() * percent / 100).min(samples.len() - 1);
    samples[index]
}

/// `VmHWM` from `/proc/self/status`: the peak resident set of this test process so far. Linux
/// only, and a process-wide number when tests run in parallel, so it is reported, never asserted.
fn peak_rss_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// Seeds `count` active global records the way an external editor would -- straight into the v2
/// directory -- and projects them with one reconcile. The owner's `create` path fsyncs and
/// publishes every record individually, which is the right behaviour for one memory and the
/// wrong tool for ten thousand of them.
fn seed_pool_through_the_file_store(fixture: &Fixture, count: usize) -> std::time::Duration {
    use crate::contexts::personalization::application::MemoryIdGeneratorPort;
    use crate::contexts::personalization::domain::{
        MemoryProvenance, MemorySensitivity, MemorySource, MemoryType,
    };
    use crate::contexts::personalization::infrastructure::{compose, UuidMemoryIdGenerator};

    let directory = fixture.directory_path.join("memory");
    let now = super::compatibility_tests::now();
    for index in 0..count {
        let record = MemoryRecord {
            id: UuidMemoryIdGenerator.generate(),
            name: format!("scale-{index:05}"),
            description: "seeded".to_string(),
            memory_type: MemoryType::Project,
            content: format!("content for scale-{index:05}"),
            scope: MemoryScope::Global,
            audience: MemoryAudience::AllAgents,
            status: MemoryStatus::Active,
            source: MemorySource::ExplicitUser,
            provenance: MemoryProvenance::default(),
            sensitivity: MemorySensitivity::Normal,
            revision: 1,
            created_at: now,
            updated_at: now,
            verified_at: None,
            last_used_at: None,
            use_count: 0,
        };
        fs::write(directory.join(record.file_name()), compose(&record)).expect("seed file");
    }
    let started = std::time::Instant::now();
    let outcome = fixture.api.reconcile_memories().expect("reconcile");
    assert_eq!(outcome.rebuilt_projection_rows, count);
    assert!(outcome.failures.is_empty());
    started.elapsed()
}

fn measure_relation(count: usize) -> ScaleMeasurement {
    use crate::contexts::personalization::application::MemoryEligibilityCriteria;
    use crate::contexts::personalization::infrastructure::SqliteMemoryProjection;

    let fixture = open_fixture(&format!("read-scope-scale-{count}"));
    mark_ready(&fixture);
    seed_global_policy(&fixture);
    let reconcile = seed_pool_through_the_file_store(&fixture, count);
    let (_, context) = resolve(
        &fixture,
        AGENT_A,
        SessionPersonalizationMode::Standard,
        WorkspaceBinding::Absent,
    );

    let mut relation_samples = Vec::with_capacity(SAMPLES);
    let mut handles: Vec<MemoryReadHandle> = Vec::new();
    for _ in 0..SAMPLES {
        let started = std::time::Instant::now();
        let relation = fixture.api.open_memory_query(&context).expect("relation");
        relation_samples.push(started.elapsed());
        assert!(relation.complete);
        assert_eq!(relation.entries.len(), count);
        // A recall delivers at most twenty bodies however large the pool is.
        handles = relation.entries.iter().rev().take(20).cloned().collect();
    }
    let mut pinned_samples = Vec::with_capacity(SAMPLES);
    let mut bodies_loaded = 0;
    for _ in 0..SAMPLES {
        let started = std::time::Instant::now();
        let bodies = fixture
            .api
            .read_pinned_memories(&context, &handles)
            .expect("bodies");
        pinned_samples.push(started.elapsed());
        bodies_loaded = bodies.len();
    }
    assert_eq!(bodies_loaded, 20.min(count));

    let plan = SqliteMemoryProjection::new(
        NativeDatabase::new(fixture.directory_path.clone()).expect("database"),
    )
    .explain_eligibility_page(
        &MemoryEligibilityCriteria {
            agent_id: agent(AGENT_A),
            allow_global: true,
            workspace: None,
            project_only: false,
            limit: 0,
        },
        1_000,
    )
    .expect("query plan");

    ScaleMeasurement {
        relation_p50: percentile(&mut relation_samples, 50),
        relation_p95: percentile(&mut relation_samples, 95),
        pinned_p50: percentile(&mut pinned_samples, 50),
        pinned_p95: percentile(&mut pinned_samples, 95),
        bodies_loaded,
        reconcile,
        plan,
        peak_rss_kib: peak_rss_kib(),
    }
}

fn report(pool: usize, measured: &ScaleMeasurement) {
    eprintln!(
        "MR-32 pool={pool} relation_p50={:?} relation_p95={:?} pinned_p50={:?} pinned_p95={:?} \
         bodies_loaded={} reconcile={:?} peak_rss_kib={:?}",
        measured.relation_p50,
        measured.relation_p95,
        measured.pinned_p50,
        measured.pinned_p95,
        measured.bodies_loaded,
        measured.reconcile,
        measured.peak_rss_kib
    );
    for line in &measured.plan {
        eprintln!("MR-32 pool={pool} plan: {line}");
    }
}

#[test]
fn a_thousand_record_pool_enumerates_completely_and_reads_only_the_bounded_candidates() {
    let measured = measure_relation(1_000);
    report(1_000, &measured);
    assert_eq!(measured.bodies_loaded, 20);
    // The page is served by the primary-key order, not a sort of the whole table.
    assert!(!measured
        .plan
        .iter()
        .any(|line| line.contains("USE TEMP B-TREE FOR ORDER BY")));
}

/// Manual scale measurement: `cargo test -- --ignored a_ten_thousand_record_pool`. Recorded in the
/// change's verification notes; never a CI threshold.
#[test]
#[ignore = "manual scale measurement; prints timings"]
fn a_ten_thousand_record_pool_enumerates_completely_within_budget() {
    let measured = measure_relation(10_000);
    report(10_000, &measured);
    assert_eq!(measured.bodies_loaded, 20);
}
