use super::*;

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
            .any(|detail| detail.contains("idx_messages_session_sequence")),
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
