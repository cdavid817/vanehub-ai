//! Which memories are eligible, and the invariant that the counts add up.
//!
//! Run against the real projection, because the classification lives in one SQL expression and the
//! property under test is that the same expression decides both the refs and the counts. A fake
//! would restate whatever rule it was written from.

use tempfile::TempDir;

use super::sqlite_memory_projection::SqliteMemoryProjection;
use crate::contexts::personalization::application::{
    MemoryEligibilityCriteria, MemoryProjectionPort,
};
use crate::contexts::personalization::domain::{
    AgentId, MemoryAudience, MemoryId, MemoryProvenance, MemoryRecord, MemoryScope,
    MemorySensitivity, MemorySource, MemoryStatus, MemoryType, PersonalizationExclusionReason,
    WorkspaceKey,
};
use crate::platform::database::NativeDatabase;

struct Fixture {
    _directory: TempDir,
    projection: SqliteMemoryProjection,
    next: std::cell::Cell<usize>,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDir::with_prefix(format!("personalization-eligibility-{label}-"))
        .expect("temporary directory");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    Fixture {
        _directory: directory,
        projection: SqliteMemoryProjection::new(database),
        next: std::cell::Cell::new(1),
    }
}

fn workspace(key: &str) -> WorkspaceKey {
    WorkspaceKey::parse(key).expect("workspace key")
}

fn agent(id: &str) -> AgentId {
    AgentId::parse(id).expect("agent id")
}

impl Fixture {
    fn put(&self, scope: MemoryScope, audience: MemoryAudience, status: MemoryStatus) -> MemoryId {
        let index = self.next.get();
        self.next.set(index + 1);
        let id = MemoryId::parse(&format!("01K2MEM{index:019}")).expect("memory id");
        let record = MemoryRecord {
            id: id.clone(),
            name: format!("memory {index}"),
            description: "d".to_string(),
            memory_type: MemoryType::Project,
            content: format!("body {index}"),
            scope,
            audience,
            status,
            source: MemorySource::ExplicitUser,
            provenance: MemoryProvenance::default(),
            sensitivity: MemorySensitivity::Normal,
            revision: 1,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            verified_at: None,
            last_used_at: None,
            use_count: 0,
        };
        self.projection
            .upsert(&record, &record.content_hash())
            .expect("project");
        id
    }

    fn criteria(&self) -> MemoryEligibilityCriteria {
        MemoryEligibilityCriteria {
            agent_id: agent("onepiece"),
            allow_global: true,
            workspace: Some(workspace("ws_alpha")),
            project_only: false,
            limit: 100,
        }
    }
}

fn count_of(
    summary: &crate::contexts::personalization::domain::MemoryEligibilitySummary,
    reason: PersonalizationExclusionReason,
) -> usize {
    summary
        .exclusions
        .iter()
        .find(|entry| entry.reason == reason)
        .map(|entry| entry.count)
        .unwrap_or_default()
}

#[test]
fn an_empty_projection_is_balanced_and_empty() {
    let fixture = fixture("empty");

    let summary = fixture
        .projection
        .eligible_page(&fixture.criteria())
        .expect("eligibility");

    assert_eq!(summary.considered, 0);
    assert_eq!(summary.eligible_total, 0);
    assert!(summary.is_balanced());
    assert!(!summary.truncated);
}

#[test]
fn every_considered_record_is_either_eligible_or_excluded_for_exactly_one_reason() {
    // The invariant the whole summary rests on. Without it, "3 of 40 eligible" leaves 37 records
    // the user has no way to account for.
    let fixture = fixture("balanced");
    fixture.put(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );
    fixture.put(
        MemoryScope::Workspace {
            workspace_key: workspace("ws_alpha"),
        },
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );
    fixture.put(
        MemoryScope::Workspace {
            workspace_key: workspace("ws_other"),
        },
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );
    fixture.put(
        MemoryScope::Global,
        MemoryAudience::SelectedAgents {
            agent_ids: vec![agent("someone-else")],
        },
        MemoryStatus::Active,
    );
    fixture.put(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Archived,
    );
    fixture.put(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Candidate,
    );

    let summary = fixture
        .projection
        .eligible_page(&fixture.criteria())
        .expect("eligibility");

    assert_eq!(summary.considered, 6);
    assert_eq!(
        summary.eligible_total, 2,
        "the global and the matching workspace"
    );
    assert!(summary.is_balanced());
    assert_eq!(
        count_of(&summary, PersonalizationExclusionReason::OtherWorkspace),
        1
    );
    assert_eq!(
        count_of(&summary, PersonalizationExclusionReason::AgentAudience),
        1
    );
    assert_eq!(
        count_of(&summary, PersonalizationExclusionReason::Archived),
        1
    );
    assert_eq!(
        count_of(&summary, PersonalizationExclusionReason::PendingCandidate),
        1
    );
}

#[test]
fn a_selected_audience_admits_the_named_agent_and_no_other() {
    let fixture = fixture("audience");
    fixture.put(
        MemoryScope::Global,
        MemoryAudience::SelectedAgents {
            agent_ids: vec![agent("onepiece")],
        },
        MemoryStatus::Active,
    );

    let summary = fixture
        .projection
        .eligible_page(&fixture.criteria())
        .expect("eligibility");
    assert_eq!(summary.eligible_total, 1);

    let mut other = fixture.criteria();
    other.agent_id = agent("claude-code");
    let summary = fixture
        .projection
        .eligible_page(&other)
        .expect("eligibility");
    assert_eq!(summary.eligible_total, 0);
    assert_eq!(
        count_of(&summary, PersonalizationExclusionReason::AgentAudience),
        1
    );
}

#[test]
fn the_agent_that_produced_a_memory_gets_no_special_access_to_it() {
    // Provenance is not authorization. A memory an Agent wrote for a narrower audience must stay
    // invisible to it once it is no longer in that audience.
    let fixture = fixture("provenance-not-access");
    let index = fixture.next.get();
    fixture.next.set(index + 1);
    let id = MemoryId::parse(&format!("01K2MEM{index:019}")).expect("memory id");
    let record = MemoryRecord {
        id,
        name: "written by onepiece".to_string(),
        description: "d".to_string(),
        memory_type: MemoryType::Project,
        content: "body".to_string(),
        scope: MemoryScope::Global,
        audience: MemoryAudience::SelectedAgents {
            agent_ids: vec![agent("claude-code")],
        },
        status: MemoryStatus::Active,
        source: MemorySource::OnePieceAutomatic,
        provenance: MemoryProvenance {
            source_agent_id: Some(agent("onepiece")),
            ..MemoryProvenance::default()
        },
        sensitivity: MemorySensitivity::Normal,
        revision: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        verified_at: None,
        last_used_at: None,
        use_count: 0,
    };
    fixture
        .projection
        .upsert(&record, &record.content_hash())
        .expect("project");

    let summary = fixture
        .projection
        .eligible_page(&fixture.criteria())
        .expect("eligibility");

    assert_eq!(summary.eligible_total, 0);
    assert_eq!(
        count_of(&summary, PersonalizationExclusionReason::AgentAudience),
        1
    );
}

#[test]
fn a_disabled_global_toggle_and_a_project_only_session_exclude_the_same_records_differently() {
    // Same outcome, different fix. Reporting both as "global memory disabled" would send a user in
    // a project-only session hunting for a toggle that is not the reason.
    let fixture = fixture("global-versus-project");
    fixture.put(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );

    let mut global_off = fixture.criteria();
    global_off.allow_global = false;
    let summary = fixture
        .projection
        .eligible_page(&global_off)
        .expect("eligibility");
    assert_eq!(
        count_of(
            &summary,
            PersonalizationExclusionReason::GlobalMemoryDisabled
        ),
        1
    );

    let mut project_only = fixture.criteria();
    project_only.allow_global = false;
    project_only.project_only = true;
    let summary = fixture
        .projection
        .eligible_page(&project_only)
        .expect("eligibility");
    assert_eq!(
        count_of(&summary, PersonalizationExclusionReason::ProjectOnlySession),
        1
    );
    assert_eq!(
        count_of(
            &summary,
            PersonalizationExclusionReason::GlobalMemoryDisabled
        ),
        0,
        "one primary reason per record, and the session is the outer one"
    );
}

#[test]
fn no_workspace_excludes_every_workspace_scope_rather_than_admitting_all() {
    let fixture = fixture("no-workspace");
    fixture.put(
        MemoryScope::Workspace {
            workspace_key: workspace("ws_alpha"),
        },
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );
    let mut criteria = fixture.criteria();
    criteria.workspace = None;

    let summary = fixture
        .projection
        .eligible_page(&criteria)
        .expect("eligibility");

    assert_eq!(summary.eligible_total, 0);
    assert_eq!(
        count_of(&summary, PersonalizationExclusionReason::OtherWorkspace),
        1
    );
}

#[test]
fn a_page_smaller_than_the_eligible_set_reports_the_exact_total_and_says_it_is_truncated() {
    let fixture = fixture("truncation");
    for _ in 0..5 {
        fixture.put(
            MemoryScope::Global,
            MemoryAudience::AllAgents,
            MemoryStatus::Active,
        );
    }
    let mut criteria = fixture.criteria();
    criteria.limit = 2;

    let summary = fixture
        .projection
        .eligible_page(&criteria)
        .expect("eligibility");

    assert_eq!(summary.refs.len(), 2);
    assert_eq!(summary.eligible_total, 5, "the count is exact regardless");
    assert!(summary.truncated);
    assert!(summary.is_balanced());
}

#[test]
fn a_ref_pins_the_revision_and_the_hash_and_carries_no_body() {
    let fixture = fixture("refs");
    let id = fixture.put(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );

    let summary = fixture
        .projection
        .eligible_page(&fixture.criteria())
        .expect("eligibility");

    let entry = summary.refs.first().expect("one ref");
    assert_eq!(entry.id, id);
    assert_eq!(entry.revision, 1);
    assert!(entry.content_hash.starts_with("sha256:"));
    assert_eq!(entry.scope_hint, "global");
    // The body is the one thing a snapshot must not carry: it is taken before token budgeting, and
    // loading every body to decide what fits would defeat the budgeting it feeds.
    assert!(!entry.description.contains("body"));
}

#[test]
fn the_digest_follows_the_eligible_set_and_not_its_order() {
    let fixture = fixture("digest");
    fixture.put(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );
    fixture.put(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );
    let first = fixture
        .projection
        .eligible_page(&fixture.criteria())
        .expect("eligibility")
        .digest;

    // Re-reading the same store gives the same digest.
    let again = fixture
        .projection
        .eligible_page(&fixture.criteria())
        .expect("eligibility")
        .digest;
    assert_eq!(first, again);

    // A new eligible memory changes it, which is what makes the next snapshot's token differ.
    fixture.put(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );
    let after = fixture
        .projection
        .eligible_page(&fixture.criteria())
        .expect("eligibility")
        .digest;
    assert_ne!(first, after);
}

#[test]
fn a_digest_carries_no_memory_text() {
    // It reaches diagnostics, so it must be a fingerprint of identities rather than of content.
    let fixture = fixture("digest-safety");
    fixture.put(
        MemoryScope::Global,
        MemoryAudience::AllAgents,
        MemoryStatus::Active,
    );

    let digest = fixture
        .projection
        .eligible_page(&fixture.criteria())
        .expect("eligibility")
        .digest;

    assert!(digest
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
    assert!(!digest.contains("body"));
    assert!(!digest.contains("memory"));
}
