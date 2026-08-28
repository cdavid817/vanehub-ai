//! What the wire boundary accepts, and what it hands back.

use super::super::error::CommandErrorCategory;
use super::{dto, mapper};
use crate::contexts::personalization::domain::{
    MemoryCursor, MemoryId, MemoryPage, MemoryScopeFilter, MemorySource, MemoryStatus,
    MemorySummary, MemoryType,
};
use chrono::{DateTime, Utc};

fn memory_id(value: &str) -> MemoryId {
    MemoryId::parse(value).expect("test id is well formed")
}

fn timestamp() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-02-01T09:00:00Z")
        .expect("test timestamp is well formed")
        .with_timezone(&Utc)
}

fn summary(id: &str) -> MemorySummary {
    MemorySummary {
        id: memory_id(id),
        name: "prefers-metric-units".to_string(),
        description: "Uses metric units in explanations.".to_string(),
        memory_type: MemoryType::User,
        scope_kind: "global",
        workspace_key: None,
        audience_is_restricted: false,
        status: MemoryStatus::Active,
        source: MemorySource::ExplicitUser,
        source_agent_id: None,
        revision: 4,
        updated_at: timestamp(),
    }
}

/// An unrecognised string is refused, never read as the nearest known one.
///
/// A build that does not understand a status, type or scope cannot safely treat it as the most
/// permissive one it does know. Defaulting would turn a caller's typo into a wider query than the
/// user asked for, and the widening would be invisible — the results would simply look right.
#[test]
fn an_unknown_string_is_refused_rather_than_read_as_the_nearest_known_one() {
    let cases = [
        dto::MemoryQueryInput {
            status: Some("deleted".to_string()),
            ..dto::MemoryQueryInput::default()
        },
        dto::MemoryQueryInput {
            memory_type: Some("untyped".to_string()),
            ..dto::MemoryQueryInput::default()
        },
        dto::MemoryQueryInput {
            scope_kind: Some("everything".to_string()),
            ..dto::MemoryQueryInput::default()
        },
    ];

    for input in cases {
        let error = mapper::memory_query(input.clone()).expect_err("unknown values are refused");
        assert_eq!(
            error.category(),
            CommandErrorCategory::Validation,
            "{input:?}"
        );
    }
}

/// A candidate is not something the memory list may hand back.
///
/// It is a queue entry awaiting review, and returning one through the memory query would present
/// unapproved text as though a user had already accepted it into the store.
#[test]
fn a_candidate_cannot_be_reached_through_the_memory_list() {
    let error = mapper::memory_query(dto::MemoryQueryInput {
        status: Some("candidate".to_string()),
        ..dto::MemoryQueryInput::default()
    })
    .expect_err("a candidate is not a memory status a query may ask for");

    assert_eq!(error.category(), CommandErrorCategory::Validation);
    assert!(error.message().contains("candidate"));
}

/// An absent filter means "everything", and it says so rather than needing a workspace.
///
/// A screen opening the list before it knows which workspace is selected sends no scope at all;
/// refusing that would make the first render an error.
#[test]
fn an_absent_scope_filter_reads_as_every_scope() {
    let query = mapper::memory_query(dto::MemoryQueryInput::default()).expect("no filter is valid");

    assert_eq!(query.scope, MemoryScopeFilter::Any);
    assert!(query.statuses.is_empty());
    assert!(query.memory_types.is_empty());
}

/// A workspace filter without a workspace addresses nothing.
///
/// Guessing which workspace was meant would list another one's memories, so the caller is told the
/// request was incomplete instead.
#[test]
fn a_workspace_filter_is_refused_without_a_workspace() {
    let error = mapper::memory_query(dto::MemoryQueryInput {
        scope_kind: Some("workspace".to_string()),
        ..dto::MemoryQueryInput::default()
    })
    .expect_err("a workspace filter needs a workspace");

    assert_eq!(error.category(), CommandErrorCategory::Validation);
}

/// The cursor a page hands out is the cursor the next page accepts.
///
/// It is opaque on purpose — a screen that built its own would be depending on the store's sort
/// key — so the only guarantee owed to a caller is that quoting it back works.
#[test]
fn a_page_cursor_survives_the_round_trip_it_was_issued_for() {
    let cursor = MemoryCursor {
        sort_key: "2026-02-01T09:00:00Z".to_string(),
        id: memory_id("mem-0000000000000042"),
    };
    let page = MemoryPage {
        items: vec![summary("mem-0000000000000042")],
        next_cursor: Some(cursor.clone()),
        total_matched: Some(1),
    };

    let rendered = mapper::page_to_dto(page)
        .next_cursor
        .expect("a page with a next cursor renders it");
    let parsed = mapper::memory_query(dto::MemoryQueryInput {
        cursor: Some(rendered),
        ..dto::MemoryQueryInput::default()
    })
    .expect("the cursor a page issued is readable")
    .cursor
    .expect("the parsed query carries the cursor");

    assert_eq!(parsed, cursor);
}

/// A cursor that did not come from a page is refused.
///
/// A hand-built one would encode an ordering assumption the store never promised, and reading it
/// as position zero would silently restart the list at the top mid-scroll.
#[test]
fn a_cursor_the_store_did_not_issue_is_refused() {
    let error = mapper::memory_query(dto::MemoryQueryInput {
        cursor: Some("mem-0000000000000042".to_string()),
        ..dto::MemoryQueryInput::default()
    })
    .expect_err("a cursor without the field separator is unreadable");

    assert_eq!(error.category(), CommandErrorCategory::Validation);
}

/// A list entry carries no body, and this checks the shape that actually crosses the boundary.
///
/// The domain summary has no body field to leak, but the wire type is edited independently: adding
/// `content` here to save a round trip would put every stored body on screen at list time, past
/// whatever the detail call checks before returning one.
#[test]
fn a_list_entry_serializes_without_a_memory_body() {
    let page = MemoryPage {
        items: vec![summary("mem-0000000000000042")],
        next_cursor: None,
        total_matched: None,
    };

    let rendered = serde_json::to_value(mapper::page_to_dto(page)).expect("the view serializes");
    let entry = rendered["items"][0]
        .as_object()
        .expect("a rendered entry is an object");

    for absent in [
        "content",
        "body",
        "text",
        "legacyFolder",
        "path",
        "filePath",
    ] {
        assert!(!entry.contains_key(absent), "{absent} reached a list entry");
    }
    assert_eq!(entry["id"], "mem-0000000000000042");
    assert_eq!(entry["revision"], 4);
}

/// A scope named after a key is refused without that key.
///
/// `agent` without an agent and `workspace-agent` without both address nothing, and picking a
/// nearby layer would edit policy the user never opened.
#[test]
fn a_policy_scope_is_refused_without_the_key_it_is_named_after() {
    let cases = [
        ("agent", None, None),
        ("workspace", None, None),
        ("workspace-agent", Some("claude-code"), None),
        ("workspace-agent", None, Some("ws-1")),
        ("session", None, None),
    ];

    for (kind, agent, workspace) in cases {
        let error = mapper::policy_scope(kind, agent, workspace)
            .expect_err("an incomplete scope addresses nothing");
        assert_eq!(error.category(), CommandErrorCategory::Validation, "{kind}");
    }
}

/// Archived memories are deleted only when the user asked for them.
///
/// Preview and execute both read this, and the token issued by one names the statuses it
/// authorises — so an inverted branch here would not merely widen a reset, it would delete records
/// the preview told the user were safe.
#[test]
fn a_reset_touches_archived_memories_only_when_asked_to() {
    assert_eq!(mapper::reset_statuses(false), vec![MemoryStatus::Active]);
    assert_eq!(
        mapper::reset_statuses(true),
        vec![MemoryStatus::Active, MemoryStatus::Archived]
    );
}
