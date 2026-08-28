use chrono::{DateTime, Duration, TimeZone, Utc};
use tempfile::TempDir;

use super::sqlite_memory_projection::SqliteMemoryProjection;
use crate::contexts::personalization::application::MemoryProjectionPort;
use crate::contexts::personalization::domain::{
    AgentId, MemoryAudience, MemoryId, MemoryOrder, MemoryProvenance, MemoryQuery, MemoryRecord,
    MemoryScope, MemoryScopeFilter, MemorySensitivity, MemorySource, MemoryStatus, MemoryType,
    WorkspaceKey,
};
use crate::platform::database::NativeDatabase;

struct Fixture {
    _directory: TempDir,
    projection: SqliteMemoryProjection,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDir::with_prefix(format!("personalization-projection-{label}-"))
        .expect("temporary directory");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    Fixture {
        _directory: directory,
        projection: SqliteMemoryProjection::new(database),
    }
}

fn base_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

/// Ids are ordered lexicographically so a keyset tie-break is predictable in assertions.
fn memory_id(index: usize) -> MemoryId {
    MemoryId::parse(&format!("01K2MEM{index:019}")).expect("memory id")
}

fn record(index: usize, name: &str, updated_at: DateTime<Utc>) -> MemoryRecord {
    MemoryRecord {
        id: memory_id(index),
        name: name.to_string(),
        description: format!("description {index}"),
        memory_type: MemoryType::Project,
        content: "body that must never reach a list page".to_string(),
        scope: MemoryScope::Global,
        audience: MemoryAudience::AllAgents,
        status: MemoryStatus::Active,
        source: MemorySource::ExplicitUser,
        provenance: MemoryProvenance::default(),
        sensitivity: MemorySensitivity::Normal,
        revision: 1,
        created_at: base_time(),
        updated_at,
        verified_at: None,
        last_used_at: None,
        use_count: 0,
    }
}

fn store(fixture: &Fixture, record: &MemoryRecord) {
    fixture
        .projection
        .upsert(record, "hash")
        .expect("projection upsert");
}

#[test]
fn a_record_round_trips_into_a_summary() {
    let fixture = fixture("round-trip");
    let mut subject = record(1, "Use npm", base_time());
    subject.provenance = MemoryProvenance {
        source_agent_id: Some(AgentId::parse("claude-code").expect("agent")),
        ..MemoryProvenance::default()
    };
    subject.scope = MemoryScope::Workspace {
        workspace_key: WorkspaceKey::parse("ws_1").expect("workspace"),
    };
    subject.audience = MemoryAudience::SelectedAgents {
        agent_ids: vec![AgentId::parse("codex-cli").expect("agent")],
    };
    store(&fixture, &subject);

    let page = fixture
        .projection
        .list_page(&MemoryQuery::default())
        .expect("page");
    assert_eq!(page.items.len(), 1);
    let summary = &page.items[0];
    assert_eq!(summary.id, subject.id);
    assert_eq!(summary.name, "Use npm");
    assert_eq!(summary.description, "description 1");
    assert_eq!(summary.memory_type, MemoryType::Project);
    assert_eq!(summary.scope_kind, "workspace");
    assert_eq!(
        summary.workspace_key,
        Some(WorkspaceKey::parse("ws_1").expect("workspace"))
    );
    assert!(summary.audience_is_restricted);
    assert_eq!(summary.status, MemoryStatus::Active);
    assert_eq!(summary.source, MemorySource::ExplicitUser);
    assert_eq!(
        summary.source_agent_id,
        Some(AgentId::parse("claude-code").expect("agent"))
    );
    assert_eq!(summary.revision, 1);
    assert_eq!(summary.updated_at, base_time());
}

#[test]
fn upserting_the_same_id_updates_rather_than_duplicating() {
    let fixture = fixture("upsert");
    let mut subject = record(1, "First", base_time());
    store(&fixture, &subject);
    subject.name = "Renamed".to_string();
    subject.revision = 2;
    store(&fixture, &subject);

    let page = fixture
        .projection
        .list_page(&MemoryQuery::default())
        .expect("page");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].name, "Renamed");
    assert_eq!(page.items[0].revision, 2);
}

#[test]
fn paging_visits_every_row_exactly_once_even_when_timestamps_tie() {
    // Duplicate timestamps are routine — a migration writes a whole directory in one pass — and a
    // cursor on the timestamp alone would skip or repeat rows at the page boundary.
    let fixture = fixture("ties");
    for index in 0..7 {
        store(
            &fixture,
            &record(index, &format!("Memory {index}"), base_time()),
        );
    }

    let mut seen = Vec::new();
    let mut cursor = None;
    loop {
        let query = MemoryQuery {
            cursor: cursor.clone(),
            ..MemoryQuery::default()
        }
        .with_page_size(3);
        let page = fixture.projection.list_page(&query).expect("page");
        assert!(page.items.len() <= 3);
        seen.extend(page.items.iter().map(|item| item.id.clone()));
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }

    assert_eq!(seen.len(), 7, "every row must appear exactly once");
    let unique: std::collections::BTreeSet<_> = seen.iter().collect();
    assert_eq!(unique.len(), 7, "no row may be repeated across pages");
}

#[test]
fn paging_orders_newest_first_by_default() {
    let fixture = fixture("order");
    store(&fixture, &record(1, "old", base_time()));
    store(
        &fixture,
        &record(2, "new", base_time() + Duration::hours(2)),
    );

    let page = fixture
        .projection
        .list_page(&MemoryQuery::default())
        .expect("page");
    assert_eq!(page.items[0].name, "new");
    assert_eq!(page.items[1].name, "old");
}

#[test]
fn name_ordering_pages_on_the_name_key() {
    let fixture = fixture("name-order");
    for (index, name) in ["charlie", "alpha", "bravo"].iter().enumerate() {
        store(&fixture, &record(index, name, base_time()));
    }

    let first = fixture
        .projection
        .list_page(
            &MemoryQuery {
                order: MemoryOrder::NameAscending,
                ..MemoryQuery::default()
            }
            .with_page_size(2),
        )
        .expect("page");
    assert_eq!(
        first
            .items
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec!["alpha", "bravo"]
    );

    let second = fixture
        .projection
        .list_page(
            &MemoryQuery {
                order: MemoryOrder::NameAscending,
                cursor: first.next_cursor.clone(),
                ..MemoryQuery::default()
            }
            .with_page_size(2),
        )
        .expect("page");
    assert_eq!(
        second
            .items
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        vec!["charlie"]
    );
    assert!(second.next_cursor.is_none());
}

#[test]
fn a_page_never_exceeds_the_maximum_even_when_a_caller_asks_for_more() {
    let fixture = fixture("bound");
    for index in 0..5 {
        store(
            &fixture,
            &record(index, &format!("Memory {index}"), base_time()),
        );
    }
    let page = fixture
        .projection
        .list_page(&MemoryQuery::default().with_page_size(100_000))
        .expect("page");
    assert_eq!(page.items.len(), 5);
    assert!(page.next_cursor.is_none());
}

#[test]
fn filters_narrow_by_scope_status_type_and_source_agent() {
    let fixture = fixture("filters");
    let workspace = WorkspaceKey::parse("ws_1").expect("workspace");

    let mut global_active = record(1, "global active", base_time());
    store(&fixture, &global_active);

    let mut workspace_active = record(2, "workspace active", base_time());
    workspace_active.scope = MemoryScope::Workspace {
        workspace_key: workspace.clone(),
    };
    store(&fixture, &workspace_active);

    let mut candidate = record(3, "candidate", base_time());
    candidate.status = MemoryStatus::Candidate;
    store(&fixture, &candidate);

    global_active.id = memory_id(4);
    global_active.name = "reference".to_string();
    global_active.memory_type = MemoryType::Reference;
    global_active.provenance = MemoryProvenance {
        source_agent_id: Some(AgentId::parse("codex-cli").expect("agent")),
        ..MemoryProvenance::default()
    };
    store(&fixture, &global_active);

    let by_workspace = fixture
        .projection
        .list_page(&MemoryQuery {
            scope: MemoryScopeFilter::Workspace {
                workspace_key: workspace.clone(),
            },
            ..MemoryQuery::default()
        })
        .expect("page");
    assert_eq!(by_workspace.items.len(), 1);
    assert_eq!(by_workspace.items[0].name, "workspace active");

    let global_only = fixture
        .projection
        .list_page(&MemoryQuery {
            scope: MemoryScopeFilter::GlobalOnly,
            ..MemoryQuery::default()
        })
        .expect("page");
    assert_eq!(global_only.items.len(), 3);

    let candidates = fixture
        .projection
        .list_page(&MemoryQuery {
            statuses: vec![MemoryStatus::Candidate],
            ..MemoryQuery::default()
        })
        .expect("page");
    assert_eq!(candidates.items.len(), 1);
    assert_eq!(candidates.items[0].name, "candidate");

    let references = fixture
        .projection
        .list_page(&MemoryQuery {
            memory_types: vec![MemoryType::Reference],
            ..MemoryQuery::default()
        })
        .expect("page");
    assert_eq!(references.items.len(), 1);

    let by_agent = fixture
        .projection
        .list_page(&MemoryQuery {
            source_agent_id: Some(AgentId::parse("codex-cli").expect("agent")),
            ..MemoryQuery::default()
        })
        .expect("page");
    assert_eq!(by_agent.items.len(), 1);
    assert_eq!(by_agent.items[0].name, "reference");
}

#[test]
fn an_audience_filter_matches_whole_agent_ids_only() {
    // `agent-1` must not match a memory restricted to `agent-10`.
    let fixture = fixture("audience");
    let mut restricted = record(1, "for agent-10", base_time());
    restricted.audience = MemoryAudience::SelectedAgents {
        agent_ids: vec![AgentId::parse("agent-10").expect("agent")],
    };
    store(&fixture, &restricted);

    let mut open = record(2, "for everyone", base_time());
    open.audience = MemoryAudience::AllAgents;
    store(&fixture, &open);

    let narrow = fixture
        .projection
        .list_page(&MemoryQuery {
            audience_agent_id: Some(AgentId::parse("agent-1").expect("agent")),
            ..MemoryQuery::default()
        })
        .expect("page");
    assert_eq!(
        narrow.items.len(),
        1,
        "only the all-Agents memory is visible to agent-1"
    );
    assert_eq!(narrow.items[0].name, "for everyone");

    let exact = fixture
        .projection
        .list_page(&MemoryQuery {
            audience_agent_id: Some(AgentId::parse("agent-10").expect("agent")),
            ..MemoryQuery::default()
        })
        .expect("page");
    assert_eq!(exact.items.len(), 2);
}

#[test]
fn search_matches_name_and_description_and_escapes_wildcards() {
    let fixture = fixture("search");
    store(&fixture, &record(1, "Use npm", base_time()));
    store(&fixture, &record(2, "Use pnpm", base_time()));

    let matches = fixture
        .projection
        .list_page(&MemoryQuery {
            search: Some("pnpm".to_string()),
            ..MemoryQuery::default()
        })
        .expect("page");
    assert_eq!(matches.items.len(), 1);
    assert_eq!(matches.items[0].name, "Use pnpm");

    // A bare `%` would otherwise match everything, which reads as "search is broken" rather than
    // "no memory contains a percent sign".
    let literal = fixture
        .projection
        .list_page(&MemoryQuery {
            search: Some("%".to_string()),
            ..MemoryQuery::default()
        })
        .expect("page");
    assert!(literal.items.is_empty());

    let by_description = fixture
        .projection
        .list_page(&MemoryQuery {
            search: Some("description 2".to_string()),
            ..MemoryQuery::default()
        })
        .expect("page");
    assert_eq!(by_description.items.len(), 1);
}

#[test]
fn reset_counts_split_by_scope_and_status() {
    let fixture = fixture("counts");
    store(&fixture, &record(1, "global", base_time()));

    let mut workspace_record = record(2, "workspace", base_time());
    workspace_record.scope = MemoryScope::Workspace {
        workspace_key: WorkspaceKey::parse("ws_1").expect("workspace"),
    };
    store(&fixture, &workspace_record);

    let mut candidate = record(3, "candidate", base_time());
    candidate.status = MemoryStatus::Candidate;
    store(&fixture, &candidate);

    let counts = fixture
        .projection
        .count_for_reset(&MemoryScopeFilter::Any, &[])
        .expect("counts");
    assert_eq!(counts.matched, 3);
    assert_eq!(counts.global, 2);
    assert_eq!(counts.workspace, 1);
    assert_eq!(counts.candidates, 1);
    assert_eq!(
        counts.malformed, 0,
        "the projection cannot see files it never projected"
    );

    let active_only = fixture
        .projection
        .count_for_reset(&MemoryScopeFilter::Any, &[MemoryStatus::Active])
        .expect("counts");
    assert_eq!(active_only.matched, 2);
    assert_eq!(active_only.candidates, 0);
}

#[test]
fn removing_and_clearing_leave_no_projected_ids() {
    let fixture = fixture("clear");
    for index in 0..3 {
        store(
            &fixture,
            &record(index, &format!("Memory {index}"), base_time()),
        );
    }
    assert_eq!(fixture.projection.projected_ids().expect("ids").len(), 3);

    assert!(fixture.projection.remove(&memory_id(0)).expect("remove"));
    assert!(
        !fixture
            .projection
            .remove(&memory_id(0))
            .expect("remove again"),
        "removing a row twice is a no-op, not an error"
    );
    assert_eq!(fixture.projection.projected_ids().expect("ids").len(), 2);

    assert_eq!(fixture.projection.clear().expect("clear"), 2);
    assert!(fixture.projection.projected_ids().expect("ids").is_empty());
}
