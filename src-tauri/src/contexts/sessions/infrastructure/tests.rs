use super::sqlite_repository::{compatibility_search_statement, indexed_search_statement};
use super::*;
use crate::contexts::sessions::application::{
    CategoryRecord, ChatConfigurationValues, FileReferenceInput, LoopSessionOwnership,
    MessagePageQuery, MessageRecord, MessageTokenUsage, MessageUsageRecord,
    SessionCategoryRepository, SessionConfigurationRepository, SessionListScope,
    SessionMessageRepository, SessionRecord, SessionRemoteWorkspace, SessionRepository,
    SessionSearchMatchKind, SessionSearchQuery, SessionSearchResult, SessionSshBinding,
    SessionTransactionPort, SessionUsageAccountingKind, SessionUsageRepository, SessionUsageUnit,
    SessionWorkspace, UsageStatisticsRange,
};
use crate::contexts::sessions::domain::{
    normalize_chat_preferences, CategoryId, CategoryName, ChatConfigurationRequest, FileReference,
    FileReferenceSet, LoopSessionRole, MessageId, MessageRole, MessageStatus, SessionActivation,
    SessionAggregate, SessionCategory, SessionId, SessionLifecycle, SessionMessage, SessionOwner,
    SessionSeat, SessionTitle,
};
use crate::platform::database::{migrate, NativeDatabase};
use crate::test_support::TempDirectory;
use rusqlite::params;
use serde_json::json;

struct Fixture {
    _directory: TempDirectory,
    database: NativeDatabase,
    repository: SqliteSessionsRepository,
}

fn fixture(name: &str) -> Fixture {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    database.connection().expect("migrated connection");
    let repository = SqliteSessionsRepository::new(database.clone());
    Fixture {
        _directory: directory,
        database,
        repository,
    }
}

fn session_record(
    id: &str,
    lifecycle: SessionLifecycle,
    title: &str,
    updated_at: &str,
) -> SessionRecord {
    SessionRecord {
        aggregate: SessionAggregate::rehydrate(
            SessionId::parse(id).expect("session id"),
            SessionTitle::for_creation(Some(title)),
            lifecycle,
            SessionOwner::desktop(),
            None,
            false,
            false,
        ),
        agent_id: "codex-cli".to_string(),
        seats: vec![SessionSeat {
            agent_id: "codex-cli".to_string(),
            role_id: None,
        }],
        interaction_mode: "interactive".to_string(),
        workspace: SessionWorkspace {
            folder: Some("D:\\code\\fixture".to_string()),
            project_path: Some("D:\\code\\fixture".to_string()),
            ..Default::default()
        },
        runtime_session_id: None,
        created_at: "2026-07-01T00:00:00+00:00".to_string(),
        updated_at: updated_at.to_string(),
    }
}

fn message_record(
    id: &str,
    session_id: &str,
    role: MessageRole,
    status: MessageStatus,
    content: &str,
) -> MessageRecord {
    MessageRecord {
        message: SessionMessage::rehydrate(
            MessageId::parse(id).expect("message id"),
            SessionId::parse(session_id).expect("session id"),
            role,
            status,
            FileReferenceSet::new(vec![FileReference::new(
                "reference-1",
                "src/main.rs",
                "main.rs",
                Some(12),
                Some("hash".to_string()),
            )
            .expect("reference")])
            .expect("references"),
        ),
        seat_index: None,
        content: content.to_string(),
        thinking_content: Some("thinking".to_string()),
        tool_use: Some(vec![json!({"id": "tool-1", "name": "read"})]),
        rich_blocks: Some(vec![json!({"id": "block-1", "kind": "card", "v": 1})]),
        token_usage: None,
        error: None,
        created_at: "2026-07-18T10:00:00+00:00".to_string(),
        updated_at: "2026-07-18T10:00:00+00:00".to_string(),
    }
}

#[test]
fn remote_ssh_binding_round_trips_and_updates_without_mutating_snapshot() {
    let fixture = fixture("session-remote-ssh-binding");
    let mut session = session_record(
        "session-remote-binding",
        SessionLifecycle::Idle,
        "Remote",
        "2026-07-24T00:00:00Z",
    );
    session.workspace.remote_workspace = Some(SessionRemoteWorkspace {
        host: "remote.example.test".to_string(),
        port: Some(22),
        user: Some("dev".to_string()),
        path: "/work/app".to_string(),
        display_name: "Remote App".to_string(),
        uri: "ssh://dev@remote.example.test/work/app".to_string(),
    });
    session.workspace.remote_ssh_binding = Some(SessionSshBinding {
        connection_id: "ssh-first".to_string(),
        revision: 3,
    });
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create remote session");

    let mut loaded = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find remote session")
        .expect("remote session");
    assert_eq!(
        loaded.workspace.remote_ssh_binding,
        session.workspace.remote_ssh_binding
    );
    let snapshot = loaded.workspace.remote_workspace.clone();
    loaded.workspace.remote_ssh_binding = Some(SessionSshBinding {
        connection_id: "ssh-second".to_string(),
        revision: 7,
    });
    let rebound =
        SessionRepository::save(&fixture.repository, &loaded).expect("save rebound session");

    assert_eq!(rebound.workspace.remote_workspace, snapshot);
    assert_eq!(
        rebound.workspace.remote_ssh_binding,
        Some(SessionSshBinding {
            connection_id: "ssh-second".to_string(),
            revision: 7,
        })
    );
}

fn usage_record(message_id: &str, session_id: &str, agent_id: &str) -> MessageUsageRecord {
    MessageUsageRecord {
        message_id: message_id.to_string(),
        session_id: session_id.to_string(),
        agent_id: agent_id.to_string(),
        provider_id: Some("openai".to_string()),
        model_id: Some("gpt-5-5".to_string()),
        accounting_kind: SessionUsageAccountingKind::Reported,
        unit: SessionUsageUnit::Tokens,
        input_count: 7,
        output_count: 11,
        cache_read_count: 2,
        cache_creation_count: 3,
        source: "provider".to_string(),
        occurred_at: "2026-07-18T10:00:00+00:00".to_string(),
    }
}

#[test]
fn loop_owned_sessions_round_trip_but_stay_out_of_default_navigation() {
    let fixture = fixture("sessions-loop-ownership");
    let repository = &fixture.repository;
    let normal = session_record(
        "session-normal",
        SessionLifecycle::Idle,
        "Normal session",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &normal, SessionActivation::PreserveActive)
        .expect("normal session");
    let mut role = session_record(
        "session-loop-verifier",
        SessionLifecycle::Idle,
        "Loop verifier",
        "2026-07-18T11:00:00+00:00",
    );
    role.workspace.loop_ownership = Some(LoopSessionOwnership {
        run_id: "run-1".to_string(),
        iteration_id: "iteration-1".to_string(),
        role: LoopSessionRole::Verifier,
    });
    SessionTransactionPort::create_session(repository, &role, SessionActivation::PreserveActive)
        .expect("role session");

    let default_list =
        SessionRepository::list(repository, SessionListScope::Current).expect("default sessions");
    assert_eq!(default_list.len(), 1);
    assert_eq!(default_list[0].id(), "session-normal");
    let all = SessionRepository::list_including_loop_owned(repository, SessionListScope::Current)
        .expect("all sessions");
    assert_eq!(all.len(), 2);
    let loaded = SessionRepository::find(repository, role.aggregate.id())
        .expect("find role")
        .expect("role");
    assert_eq!(
        loaded.workspace.loop_ownership.expect("ownership").role,
        LoopSessionRole::Verifier
    );
    let search = SessionRepository::search(
        repository,
        &SessionSearchQuery {
            text: "Loop verifier".to_string(),
            limit: 10,
        },
    )
    .expect("search");
    assert!(search.is_empty());
}

#[test]
fn repositories_round_trip_rows_and_preserve_bounded_query_contracts() {
    let fixture = fixture("sessions-sqlite-round-trip");
    let repository = &fixture.repository;
    let mut session = session_record(
        "session-round-trip",
        SessionLifecycle::Idle,
        "Needle Session",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::Activate)
        .expect("create session");

    let loaded = SessionRepository::find(repository, session.aggregate.id())
        .expect("find session")
        .expect("session");
    assert_eq!(
        loaded.workspace.project_path.as_deref(),
        Some("D:\\code\\fixture")
    );
    assert_eq!(
        SessionRepository::active_session(repository)
            .expect("active session")
            .expect("active")
            .id(),
        session.id()
    );
    assert_eq!(
        SessionRepository::list(repository, SessionListScope::Current)
            .expect("sessions")
            .len(),
        1
    );

    let category = CategoryRecord {
        category: SessionCategory::new(
            CategoryId::parse("category-1").expect("category id"),
            CategoryName::parse("Work").expect("category name"),
            0,
        ),
        created_at: "2026-07-18T10:00:00+00:00".to_string(),
        updated_at: "2026-07-18T10:00:00+00:00".to_string(),
    };
    SessionCategoryRepository::insert(repository, &category).expect("insert category");
    assert!(SessionCategoryRepository::name_exists(repository, "work", None).expect("name exists"));
    session
        .aggregate
        .assign_category(Some(category.category.id().clone()));
    session.aggregate.set_pinned(true);
    SessionRepository::save(repository, &session).expect("save session");

    let preferences = normalize_chat_preferences(
        "codex-cli",
        ChatConfigurationRequest {
            permission_mode: "agent",
            provider_id: Some("openai"),
            model_id: Some("gpt-5-5"),
            reasoning_depth: Some("high"),
            streaming: true,
            thinking: true,
            long_context: true,
        },
    )
    .expect("preferences");
    SessionConfigurationRepository::save(
        repository,
        session.aggregate.id(),
        &preferences,
        "2026-07-18T11:00:00+00:00",
    )
    .expect("save configuration");
    let configuration = SessionConfigurationRepository::load(repository, session.aggregate.id())
        .expect("configuration")
        .expect("stored configuration");
    assert_eq!(configuration.model_id.as_deref(), Some("gpt-5-5"));

    let message = message_record(
        "message-1",
        session.id(),
        MessageRole::User,
        MessageStatus::Completed,
        "message needle",
    );
    SessionMessageRepository::insert(repository, &message).expect("insert message");
    let listed = SessionMessageRepository::list(
        repository,
        &MessagePageQuery {
            session_id: session.id().to_string(),
            limit: 50,
            before_id: None,
        },
    )
    .expect("messages");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].message.file_references().as_slice()[0].path(),
        "src/main.rs"
    );
    assert_eq!(
        listed[0].rich_blocks.as_ref().expect("rich blocks")[0]["id"],
        "block-1"
    );

    let results = SessionRepository::search(
        repository,
        &SessionSearchQuery {
            text: "needle".to_string(),
            limit: 100,
        },
    )
    .expect("search");
    assert_eq!(results.len(), 1);
    assert!(results[0]
        .matches
        .iter()
        .any(|matched| matched.kind == SessionSearchMatchKind::Title));
    assert!(results[0]
        .matches
        .iter()
        .any(|matched| matched.kind == SessionSearchMatchKind::Message));
}

#[test]
fn sqlite_search_and_message_queries_honor_limits_and_cursors() {
    let fixture = fixture("sessions-bounded-adapter-queries");
    let repository = &fixture.repository;
    for index in 1..=3 {
        let session = session_record(
            &format!("session-search-{index}"),
            SessionLifecycle::Idle,
            &format!("Needle {index}"),
            &format!("2026-07-18T1{index}:00:00+00:00"),
        );
        SessionTransactionPort::create_session(
            repository,
            &session,
            SessionActivation::PreserveActive,
        )
        .expect("create search session");
    }

    let search_results = SessionRepository::search(
        repository,
        &SessionSearchQuery {
            text: "Needle".to_string(),
            limit: 2,
        },
    )
    .expect("bounded search");
    assert_eq!(search_results.len(), 2);

    for (id, created_at) in [
        ("message-page-1", "2026-07-18T10:00:00+00:00"),
        ("message-page-2", "2026-07-18T11:00:00+00:00"),
        ("message-page-3", "2026-07-18T12:00:00+00:00"),
    ] {
        let mut message = message_record(
            id,
            "session-search-1",
            MessageRole::User,
            MessageStatus::Completed,
            id,
        );
        message.created_at = created_at.to_string();
        message.updated_at = created_at.to_string();
        SessionMessageRepository::insert(repository, &message).expect("insert paged message");
    }

    let latest = SessionMessageRepository::list(
        repository,
        &MessagePageQuery {
            session_id: "session-search-1".to_string(),
            limit: 2,
            before_id: None,
        },
    )
    .expect("latest page");
    assert_eq!(
        latest
            .iter()
            .map(|message| message.message.id().as_str())
            .collect::<Vec<_>>(),
        ["message-page-2", "message-page-3"]
    );

    let previous = SessionMessageRepository::list(
        repository,
        &MessagePageQuery {
            session_id: "session-search-1".to_string(),
            limit: 2,
            before_id: Some("message-page-2".to_string()),
        },
    )
    .expect("previous page");
    assert_eq!(
        previous
            .iter()
            .map(|message| message.message.id().as_str())
            .collect::<Vec<_>>(),
        ["message-page-1"]
    );
}

#[test]
fn message_search_index_tracks_content_mutations_and_uses_fts() {
    let fixture = fixture("sessions-message-search-index");
    let repository = &fixture.repository;
    let session = session_record(
        "session-indexed-search",
        SessionLifecycle::Idle,
        "Indexed search",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create indexed session");
    let mut message = message_record(
        "message-indexed-search",
        session.id(),
        MessageRole::User,
        MessageStatus::Completed,
        "alpha indexed needle omega",
    );
    SessionMessageRepository::insert(repository, &message).expect("insert indexed message");

    assert_eq!(fts_match_count(&fixture, "\"indexed needle\""), 1);
    let short_query_results = SessionRepository::search(
        repository,
        &SessionSearchQuery {
            text: "ph".to_string(),
            limit: 10,
        },
    )
    .expect("two-character compatibility search");
    assert_eq!(short_query_results.len(), 1);
    assert!(short_query_results[0]
        .matches
        .iter()
        .any(|matched| matched.kind == SessionSearchMatchKind::Message));
    let plan = fts_query_plan(&fixture, "\"indexed needle\"");
    assert!(
        plan.iter()
            .any(|detail| detail.contains("VIRTUAL TABLE INDEX")),
        "expected FTS virtual-table plan, got {plan:?}"
    );

    message.content = "replacement searchable phrase".to_string();
    SessionMessageRepository::save(repository, &message).expect("update indexed message");
    assert_eq!(fts_match_count(&fixture, "\"indexed needle\""), 0);
    assert_eq!(fts_match_count(&fixture, "\"searchable phrase\""), 1);

    fixture
        .database
        .connection()
        .expect("delete connection")
        .execute(
            "DELETE FROM messages WHERE id = ?1",
            [message.message.id().as_str()],
        )
        .expect("delete indexed message");
    assert_eq!(fts_match_count(&fixture, "\"searchable phrase\""), 0);
}

#[test]
fn fts_migration_keeps_archived_sessions_with_existing_messages_searchable() {
    let fixture = fixture("sessions-archived-search-migration");
    let repository = &fixture.repository;
    {
        let connection = fixture
            .database
            .connection()
            .expect("pre-migration connection");
        connection
            .execute_batch(
                r#"
                DROP TRIGGER messages_fts_insert;
                DROP TRIGGER messages_fts_delete;
                DROP TRIGGER messages_fts_update;
                DROP TABLE session_message_fts;
                DELETE FROM schema_migrations WHERE version = 33;
                "#,
            )
            .expect("simulate schema before message search migration");
    }

    let session = session_record(
        "session-archived-before-fts",
        SessionLifecycle::Idle,
        "Archived migration fixture",
        "2026-07-18T11:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create pre-migration session");
    fixture
        .database
        .connection()
        .expect("archive connection")
        .execute(
            "UPDATE sessions SET archived = 1 WHERE id = ?1",
            [session.id()],
        )
        .expect("archive pre-migration session");
    let message = message_record(
        "message-archived-before-fts",
        session.id(),
        MessageRole::User,
        MessageStatus::Completed,
        "quartz migration searchable payload",
    );
    SessionMessageRepository::insert(repository, &message).expect("insert pre-migration message");

    {
        let connection = fixture.database.connection().expect("migration connection");
        migrate(&connection).expect("apply message search migration");
    }

    let results = SessionRepository::search(
        repository,
        &SessionSearchQuery {
            text: "quartz migration".to_string(),
            limit: 10,
        },
    )
    .expect("search migrated archived session");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].session.id(), session.id());
    assert!(results[0].session.aggregate.is_archived());
    assert!(results[0].matches.iter().any(|matched| {
        matched.kind == SessionSearchMatchKind::Message
            && matched.message_id.as_deref() == Some(message.message.id().as_str())
    }));
}

/// Seeds three sessions whose ordering and per-session newest-match are unambiguous, so
/// the same expectations hold for both the indexed and the compatibility search branch.
///
/// The seeded content carries "目标词" so a three-character query reaches the trigram
/// index while a two-character "目标" substring falls to the compatibility path.
fn seed_search_ranking_fixture(fixture: &Fixture) {
    let repository = &fixture.repository;
    for (id, title, updated_at) in [
        ("session-alpha", "Alpha", "2026-07-18T12:00:00+00:00"),
        ("session-bravo", "Bravo", "2026-07-18T13:00:00+00:00"),
        ("session-charlie", "Charlie", "2026-07-18T11:00:00+00:00"),
    ] {
        let session = session_record(id, SessionLifecycle::Idle, title, updated_at);
        SessionTransactionPort::create_session(
            repository,
            &session,
            SessionActivation::PreserveActive,
        )
        .expect("create ranking session");
    }

    // Distinct timestamps: the later message must win.
    insert_message_at(
        fixture,
        "message-alpha-old",
        "session-alpha",
        "定位 目标词 旧",
        "2026-07-18T10:00:00+00:00",
    );
    insert_message_at(
        fixture,
        "message-alpha-new",
        "session-alpha",
        "定位 目标词 新",
        "2026-07-18T11:00:00+00:00",
    );
    // Identical timestamps: only the rowid tiebreak separates them, and the later insert
    // holds the higher rowid.
    insert_message_at(
        fixture,
        "message-bravo-tie-early",
        "session-bravo",
        "定位 目标词 平局一",
        "2026-07-18T10:00:00+00:00",
    );
    insert_message_at(
        fixture,
        "message-bravo-tie-late",
        "session-bravo",
        "定位 目标词 平局二",
        "2026-07-18T10:00:00+00:00",
    );
    // Charlie must never appear: neither its title nor its message matches.
    insert_message_at(
        fixture,
        "message-charlie",
        "session-charlie",
        "无关内容",
        "2026-07-18T10:00:00+00:00",
    );
}

fn insert_message_at(fixture: &Fixture, id: &str, session_id: &str, content: &str, at: &str) {
    let mut record = message_record(
        id,
        session_id,
        MessageRole::User,
        MessageStatus::Completed,
        content,
    );
    record.created_at = at.to_string();
    record.updated_at = at.to_string();
    SessionMessageRepository::insert(&fixture.repository, &record).expect("insert ranking message");
}

fn searched_session_ids(results: &[SessionSearchResult]) -> Vec<String> {
    results
        .iter()
        .map(|result| result.session.aggregate.id().as_str().to_string())
        .collect()
}

fn matched_message_id(result: &SessionSearchResult) -> Option<String> {
    result
        .matches
        .iter()
        .find(|matched| matched.kind == SessionSearchMatchKind::Message)
        .and_then(|matched| matched.message_id.clone())
}

fn assert_ranking_expectations(results: &[SessionSearchResult], label: &str) {
    assert_eq!(
        searched_session_ids(results),
        vec!["session-bravo".to_string(), "session-alpha".to_string()],
        "{label}: sessions come back newest-updated first, without the non-matching session"
    );
    assert_eq!(
        matched_message_id(&results[0]).as_deref(),
        Some("message-bravo-tie-late"),
        "{label}: the rowid tiebreak decides when created_at ties"
    );
    assert_eq!(
        matched_message_id(&results[1]).as_deref(),
        Some("message-alpha-new"),
        "{label}: the newest matching message is the match context"
    );
}

#[test]
fn short_query_search_orders_sessions_and_picks_the_newest_matching_message() {
    let fixture = fixture("sessions-short-query-ranking");
    seed_search_ranking_fixture(&fixture);

    let results = SessionRepository::search(
        &fixture.repository,
        &SessionSearchQuery {
            // Two characters, below the trigram floor, so this takes the compatibility path.
            text: "目标".to_string(),
            limit: 10,
        },
    )
    .expect("short query search");

    assert_ranking_expectations(&results, "short query");
}

#[test]
fn indexed_query_search_orders_sessions_and_picks_the_newest_matching_message() {
    let fixture = fixture("sessions-indexed-query-ranking");
    seed_search_ranking_fixture(&fixture);

    let results = SessionRepository::search(
        &fixture.repository,
        &SessionSearchQuery {
            // Three characters, so this one reaches the trigram index.
            text: "目标词".to_string(),
            limit: 10,
        },
    )
    .expect("indexed query search");

    assert_ranking_expectations(&results, "indexed query");
}

#[test]
fn search_that_matches_nothing_returns_no_sessions() {
    let fixture = fixture("sessions-search-no-match");
    seed_search_ranking_fixture(&fixture);

    for text in ["零", "零零", "零零零"] {
        let results = SessionRepository::search(
            &fixture.repository,
            &SessionSearchQuery {
                text: text.to_string(),
                limit: 10,
            },
        )
        .expect("no-match search");
        assert!(
            results.is_empty(),
            "{text} matched unexpectedly: {results:?}"
        );
    }
}

fn search_query_plan(fixture: &Fixture, sql: &str, message_query: &str) -> Vec<String> {
    let connection = fixture.database.connection().expect("plan connection");
    let mut statement = connection
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .expect("prepare search plan");
    statement
        .query_map(params![message_query, "%目标%", 10_i64], |row| row.get(3))
        .expect("query search plan")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect search plan")
}

#[test]
fn short_query_plan_seeks_the_session_index_instead_of_ranking_every_match() {
    let fixture = fixture("sessions-short-query-plan");
    seed_search_ranking_fixture(&fixture);

    let plan = search_query_plan(&fixture, &compatibility_search_statement(), "%目标%");

    assert!(
        plan.iter()
            .any(|detail| detail.contains("idx_messages_session_created")),
        "expected a per-session index seek, got {plan:?}"
    );
    assert!(
        !plan.iter().any(|detail| detail.contains("MATERIALIZE")),
        "the compatibility path must not materialize a ranked match set, got {plan:?}"
    );
}

#[test]
fn indexed_query_plan_still_drives_the_full_text_index() {
    let fixture = fixture("sessions-indexed-query-plan");
    seed_search_ranking_fixture(&fixture);

    let plan = search_query_plan(&fixture, &indexed_search_statement(), "\"目标词\"");

    assert!(
        plan.iter()
            .any(|detail| detail.contains("session_message_fts")),
        "expected the FTS branch to keep using the index, got {plan:?}"
    );
}

fn fts_match_count(fixture: &Fixture, query: &str) -> i64 {
    fixture
        .database
        .connection()
        .expect("FTS count connection")
        .query_row(
            "SELECT COUNT(*) FROM session_message_fts WHERE session_message_fts MATCH ?1",
            [query],
            |row| row.get(0),
        )
        .expect("FTS match count")
}

fn fts_query_plan(fixture: &Fixture, query: &str) -> Vec<String> {
    let connection = fixture.database.connection().expect("FTS plan connection");
    let mut statement = connection
        .prepare(
            "EXPLAIN QUERY PLAN SELECT rowid FROM session_message_fts \
             WHERE session_message_fts MATCH ?1",
        )
        .expect("prepare FTS query plan");
    statement
        .query_map([query], |row| row.get(3))
        .expect("query FTS plan")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect FTS plan")
}

#[test]
fn missing_active_session_is_cleared_when_read() {
    let fixture = fixture("sessions-stale-active-pointer");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "UPDATE workflow_state SET active_session_id = 'missing-session' WHERE id = 1",
            [],
        )
        .expect("seed stale active session");

    assert!(SessionRepository::active_session(&fixture.repository)
        .expect("active session")
        .is_none());
    let active = connection
        .query_row(
            "SELECT active_session_id FROM workflow_state WHERE id = 1",
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .expect("read active session");
    assert!(active.is_none());
}

#[test]
fn invalid_persisted_domain_values_fail_explicit_row_mapping() {
    let fixture = fixture("sessions-invalid-row");
    let session = session_record(
        "session-invalid",
        SessionLifecycle::Idle,
        "Invalid",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create session");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE sessions SET source_kind = 'im', source_connector = NULL WHERE id = ?1",
            [session.id()],
        )
        .expect("corrupt owner");

    let result = SessionRepository::find(&fixture.repository, session.aggregate.id());
    assert!(matches!(
        result,
        Err(crate::contexts::sessions::application::SessionsApplicationError::Domain(_))
    ));
}

#[test]
fn create_and_activate_roll_back_when_workflow_update_cannot_commit() {
    let fixture = fixture("sessions-create-rollback");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute("DELETE FROM workflow_state WHERE id = 1", [])
        .expect("remove workflow row");
    let session = session_record(
        "session-rollback",
        SessionLifecycle::Idle,
        "Rollback",
        "2026-07-18T10:00:00+00:00",
    );

    assert!(SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::Activate,
    )
    .is_err());
    let count: i64 = fixture
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM sessions WHERE id = ?1",
            [session.id()],
            |row| row.get(0),
        )
        .expect("session count");
    assert_eq!(count, 0);
}

#[test]
fn category_delete_rolls_back_session_unassignment_on_delete_failure() {
    let fixture = fixture("sessions-category-rollback");
    let repository = &fixture.repository;
    let mut session = session_record(
        "session-category",
        SessionLifecycle::Idle,
        "Category",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let category = CategoryRecord {
        category: SessionCategory::new(
            CategoryId::parse("category-rollback").expect("category id"),
            CategoryName::parse("Rollback").expect("category name"),
            0,
        ),
        created_at: "100".to_string(),
        updated_at: "100".to_string(),
    };
    SessionCategoryRepository::insert(repository, &category).expect("insert category");
    session
        .aggregate
        .assign_category(Some(category.category.id().clone()));
    SessionRepository::save(repository, &session).expect("assign category");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute_batch(
            "CREATE TRIGGER reject_category_delete BEFORE DELETE ON session_categories BEGIN SELECT RAISE(ABORT, 'rejected'); END;",
        )
        .expect("failure trigger");

    assert!(
        SessionTransactionPort::delete_category(repository, category.category.id(), "200",)
            .is_err()
    );
    let connection = fixture.database.connection().expect("connection");
    let assigned: Option<String> = connection
        .query_row(
            "SELECT category_id FROM sessions WHERE id = ?1",
            [session.id()],
            |row| row.get(0),
        )
        .expect("category assignment");
    let category_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM session_categories WHERE id = ?1",
            [category.category.id().as_str()],
            |row| row.get(0),
        )
        .expect("category count");
    assert_eq!(assigned.as_deref(), Some("category-rollback"));
    assert_eq!(category_count, 1);
}

#[test]
fn message_and_usage_commit_together_and_usage_is_message_owned() {
    let fixture = fixture("sessions-message-usage");
    let repository = &fixture.repository;
    let session = session_record(
        "session-usage",
        SessionLifecycle::Idle,
        "Usage",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let streaming = message_record(
        "message-usage",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    SessionMessageRepository::insert(repository, &streaming).expect("insert message");
    let mut completed = streaming.clone();
    completed
        .message
        .transition_to(MessageStatus::Completed)
        .expect("complete transition");
    completed.content = "done".to_string();
    completed.token_usage = Some(MessageTokenUsage {
        input: 7,
        output: 11,
    });
    SessionTransactionPort::complete_message(
        repository,
        &completed,
        Some(&usage_record("message-usage", session.id(), "codex-cli")),
    )
    .expect("complete with usage");

    let statistics = SessionUsageRepository::statistics(
        repository,
        UsageStatisticsRange::All,
        None,
        "2026-07-18T11:00:00+00:00",
    )
    .expect("statistics");
    assert_eq!(statistics.reported.total_tokens, 23);
    assert_eq!(statistics.coverage.reported_responses, 1);
    assert_eq!(statistics.counted_sessions, 1);

    SessionTransactionPort::delete_session(repository, session.aggregate.id())
        .expect("delete session");
    let usage_count: i64 = fixture
        .database
        .connection()
        .expect("connection")
        .query_row("SELECT COUNT(*) FROM usage_records", [], |row| row.get(0))
        .expect("usage count");
    assert_eq!(usage_count, 0);
}

#[test]
fn terminal_usage_message_lookup_returns_the_newest_matching_row() {
    let fixture = fixture("sessions-terminal-usage-lookup");
    let repository = &fixture.repository;
    let session = session_record(
        "session-terminal-usage",
        SessionLifecycle::Idle,
        "Terminal Usage",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");

    for (message_id, updated_at, source) in [
        (
            "message-terminal-older",
            "2026-07-18T10:01:00+00:00",
            "cli-session-log",
        ),
        (
            "message-terminal-newer",
            "2026-07-18T10:02:00+00:00",
            "cli-session-log",
        ),
        (
            "message-provider-newest",
            "2026-07-18T10:03:00+00:00",
            "provider",
        ),
    ] {
        let streaming = message_record(
            message_id,
            session.id(),
            MessageRole::Assistant,
            MessageStatus::Streaming,
            "",
        );
        SessionMessageRepository::insert(repository, &streaming).expect("insert message");
        let mut completed = streaming;
        completed
            .message
            .transition_to(MessageStatus::Completed)
            .expect("complete transition");
        completed.updated_at = updated_at.to_string();
        let mut usage = usage_record(message_id, session.id(), "codex-cli");
        usage.source = source.to_string();
        SessionTransactionPort::complete_message(repository, &completed, Some(&usage))
            .expect("complete with usage");
    }

    let found =
        SessionUsageRepository::terminal_usage_message_id(repository, session.id(), "codex-cli")
            .expect("lookup terminal usage message");
    assert_eq!(found.as_deref(), Some("message-terminal-newer"));
    assert_eq!(
        SessionUsageRepository::terminal_usage_message_id(repository, session.id(), "gemini-cli",)
            .expect("lookup other agent"),
        None
    );
}

#[test]
fn deleting_an_assistant_message_deletes_only_its_owned_usage_record() {
    let fixture = fixture("sessions-message-owned-usage");
    let repository = &fixture.repository;
    let session = session_record(
        "session-message-owner",
        SessionLifecycle::Idle,
        "Message Owner",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let streaming = message_record(
        "message-owned-usage",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    SessionMessageRepository::insert(repository, &streaming).expect("insert message");
    let mut completed = streaming.clone();
    completed
        .message
        .transition_to(MessageStatus::Completed)
        .expect("complete transition");
    SessionTransactionPort::complete_message(
        repository,
        &completed,
        Some(&usage_record(
            "message-owned-usage",
            session.id(),
            "codex-cli",
        )),
    )
    .expect("complete with usage");

    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "DELETE FROM messages WHERE id = ?1",
            ["message-owned-usage"],
        )
        .expect("delete message");
    let usage_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM usage_records", [], |row| row.get(0))
        .expect("usage count");
    let session_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .expect("session count");

    assert_eq!(usage_count, 0);
    assert_eq!(session_count, 1);
}

#[test]
fn usage_failure_rolls_back_the_message_completion() {
    let fixture = fixture("sessions-usage-rollback");
    let repository = &fixture.repository;
    let session = session_record(
        "session-usage-rollback",
        SessionLifecycle::Idle,
        "Usage Rollback",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let streaming = message_record(
        "message-usage-rollback",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    SessionMessageRepository::insert(repository, &streaming).expect("insert message");
    let mut completed = streaming.clone();
    completed
        .message
        .transition_to(MessageStatus::Completed)
        .expect("complete transition");

    assert!(SessionTransactionPort::complete_message(
        repository,
        &completed,
        Some(&usage_record(
            "message-usage-rollback",
            session.id(),
            "missing-agent",
        )),
    )
    .is_err());
    let status: String = fixture
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT status FROM messages WHERE id = ?1",
            ["message-usage-rollback"],
            |row| row.get(0),
        )
        .expect("message status");
    assert_eq!(status, "streaming");
}

#[test]
fn runtime_stream_updates_cannot_resurrect_cancelled_messages_and_sync_active_lifecycle() {
    let fixture = fixture("sessions-runtime-stream-cancel");
    let repository = &fixture.repository;
    let session = session_record(
        "session-runtime-cancel",
        SessionLifecycle::Idle,
        "Runtime Cancel",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::Activate)
        .expect("create active session");
    let streaming = message_record(
        "message-runtime-cancel",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    SessionMessageRepository::insert(repository, &streaming).expect("insert message");

    let mut cancelled = streaming.clone();
    cancelled
        .message
        .transition_to(MessageStatus::Cancelled)
        .expect("cancel transition");
    cancelled.updated_at = "2026-07-18T10:01:00+00:00".to_string();
    assert_eq!(
        SessionTransactionPort::cancel_messages(repository, &[cancelled]).expect("cancel message"),
        vec!["message-runtime-cancel".to_string()]
    );

    let mut stale_stream = streaming;
    stale_stream.content = "late token".to_string();
    stale_stream.updated_at = "2026-07-18T10:02:00+00:00".to_string();
    SessionMessageRepository::save_stream_fields(repository, &stale_stream)
        .expect("save late stream fields");

    let mut running = session;
    running
        .aggregate
        .transition_to(SessionLifecycle::Running)
        .expect("running transition");
    running.updated_at = "2026-07-18T10:03:00+00:00".to_string();
    SessionTransactionPort::save_runtime_session(repository, &running)
        .expect("save runtime session");

    let connection = fixture.database.connection().expect("connection");
    let (status, content): (String, String) = connection
        .query_row(
            "SELECT status, content FROM messages WHERE id = ?1",
            ["message-runtime-cancel"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("message state");
    let workflow_lifecycle: String = connection
        .query_row(
            "SELECT lifecycle_state FROM workflow_state WHERE active_session_id = ?1",
            ["session-runtime-cancel"],
            |row| row.get(0),
        )
        .expect("workflow lifecycle");

    assert_eq!(status, "cancelled");
    assert_eq!(content, "late token");
    assert_eq!(workflow_lifecycle, "running");
}

#[test]
fn orphan_recovery_rolls_back_message_failure_when_session_update_fails() {
    let fixture = fixture("sessions-recovery-rollback");
    let repository = &fixture.repository;
    let session = session_record(
        "session-recovery",
        SessionLifecycle::Running,
        "Recovery",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let streaming = message_record(
        "message-recovery",
        session.id(),
        MessageRole::Assistant,
        MessageStatus::Streaming,
        "",
    );
    SessionMessageRepository::insert(repository, &streaming).expect("insert message");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute_batch(
            "CREATE TRIGGER reject_recovery BEFORE UPDATE OF lifecycle_state ON sessions BEGIN SELECT RAISE(ABORT, 'rejected'); END;",
        )
        .expect("failure trigger");
    let mut failed = session.clone();
    failed
        .aggregate
        .transition_to(SessionLifecycle::Failed)
        .expect("failed lifecycle");

    assert!(SessionTransactionPort::recover_orphaned_session(
        repository,
        &failed,
        "2026-07-18T11:00:00+00:00",
    )
    .is_err());
    let status: String = fixture
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT status FROM messages WHERE id = ?1",
            ["message-recovery"],
            |row| row.get(0),
        )
        .expect("message status");
    assert_eq!(status, "streaming");
}

#[test]
fn usage_schema_backfills_positive_legacy_message_counts_idempotently() {
    let fixture = fixture("sessions-usage-backfill");
    let repository = &fixture.repository;
    let session = session_record(
        "session-backfill",
        SessionLifecycle::Idle,
        "Backfill",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(repository, &session, SessionActivation::PreserveActive)
        .expect("create session");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute("DROP TABLE usage_records", [])
        .expect("drop usage records");
    connection
        .execute(
            r#"
            INSERT INTO messages (
                id, session_id, role, status, content, token_input, token_output,
                created_at, updated_at
            ) VALUES (?1, ?2, 'assistant', 'completed', '', 12, 7, ?3, ?3)
            "#,
            params![
                "message-backfill",
                session.id(),
                "2026-07-18T10:00:00+00:00"
            ],
        )
        .expect("legacy message");
    apply_usage_schema(&connection).expect("first schema apply");
    apply_usage_schema(&connection).expect("second schema apply");

    let row: (String, String, i64, i64) = connection
        .query_row(
            "SELECT accounting_kind, unit, input_count, output_count FROM usage_records",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("backfilled usage");
    assert_eq!(
        row,
        ("estimated".to_string(), "characters".to_string(), 12, 7)
    );
}

#[test]
fn invalid_configuration_json_maps_to_no_persisted_snapshot() {
    let fixture = fixture("sessions-invalid-configuration");
    let session = session_record(
        "session-config-invalid",
        SessionLifecycle::Idle,
        "Configuration",
        "2026-07-18T10:00:00+00:00",
    );
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create session");
    fixture
        .database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE sessions SET chat_preferences = '{not-json}' WHERE id = ?1",
            [session.id()],
        )
        .expect("invalid snapshot");

    assert_eq!(
        SessionConfigurationRepository::load(&fixture.repository, session.aggregate.id())
            .expect("load configuration"),
        None
    );
}

#[test]
fn persisted_configuration_shape_is_separate_from_domain_preferences() {
    let values = ChatConfigurationValues {
        permission_mode: "agent".to_string(),
        provider_id: Some("openai".to_string()),
        model_id: Some("gpt-5-5".to_string()),
        reasoning_depth: Some("high".to_string()),
        streaming: true,
        thinking: true,
        long_context: true,
    };
    let raw = serde_json::to_value(&values).expect("serialize values");
    assert_eq!(raw["permissionMode"], "agent");
    let reference = FileReferenceInput {
        id: "reference".to_string(),
        path: "src/main.rs".to_string(),
        name: "main.rs".to_string(),
        size_bytes: Some(12),
        content_hash: None,
    };
    assert_eq!(
        serde_json::to_value(reference).expect("serialize reference")["sizeBytes"],
        12
    );
}

#[test]
fn seats_survive_a_create_and_are_updated_on_save() {
    let fixture = fixture("sessions-seats");
    let mut session = session_record(
        "session-seats",
        SessionLifecycle::Idle,
        "多 Agent 会话",
        "2026-08-07T00:00:00+00:00",
    );
    session.seats = vec![
        SessionSeat {
            agent_id: "claude-code".to_string(),
            role_id: Some("role-architect".to_string()),
        },
        SessionSeat {
            agent_id: "codex-cli".to_string(),
            role_id: Some("role-reviewer".to_string()),
        },
    ];
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create seated session");

    let mut loaded = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find seated session")
        .expect("seated session");
    assert_eq!(loaded.seats, session.seats);

    // A seat added mid-session must be routable from the next turn, so `save` has to carry seats.
    loaded.seats.push(SessionSeat {
        agent_id: "gemini-cli".to_string(),
        role_id: None,
    });
    let saved = SessionRepository::save(&fixture.repository, &loaded).expect("save seats");
    assert_eq!(saved.seats, loaded.seats);
}

/// Sessions created before seats existed store `[]`, and each must still open as its own Agent.
#[test]
fn a_session_without_seats_reads_as_one_seat() {
    let fixture = fixture("sessions-no-seats");
    let session = session_record(
        "session-single",
        SessionLifecycle::Idle,
        "单 Agent 会话",
        "2026-08-07T00:00:00+00:00",
    );
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create single session");

    let loaded = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find single session")
        .expect("single session");
    assert_eq!(
        loaded.seats,
        vec![SessionSeat {
            agent_id: "codex-cli".to_string(),
            role_id: None,
        }]
    );
}

/// Rendering a thread means naming who spoke, so the seat has to survive persistence.
#[test]
fn a_message_records_the_seat_that_spoke_it() {
    let fixture = fixture("messages-seat-index");
    let session = session_record(
        "session-speakers",
        SessionLifecycle::Idle,
        "多 Agent 会话",
        "2026-08-07T00:00:00+00:00",
    );
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create session");

    let mut seated = message_record(
        "message-seated",
        "session-speakers",
        MessageRole::Assistant,
        MessageStatus::Completed,
        "方案如下",
    );
    seated.seat_index = Some(1);
    SessionMessageRepository::insert(&fixture.repository, &seated).expect("insert seated");

    // A user message has no seat, and a default of 0 would attribute it to the first one.
    let spoken_by_user = message_record(
        "message-user",
        "session-speakers",
        MessageRole::User,
        MessageStatus::Completed,
        "改下登录",
    );
    SessionMessageRepository::insert(&fixture.repository, &spoken_by_user).expect("insert user");

    let loaded = SessionMessageRepository::find(&fixture.repository, seated.message.id())
        .expect("find seated")
        .expect("seated message");
    assert_eq!(loaded.seat_index, Some(1));

    let loaded_user =
        SessionMessageRepository::find(&fixture.repository, spoken_by_user.message.id())
            .expect("find user")
            .expect("user message");
    assert_eq!(loaded_user.seat_index, None);
}
