use super::*;

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
        execution_mode: "execute".to_string(),
        provider_id: Some("openai".to_string()),
        model_id: Some("gpt-5-5".to_string()),
        reasoning_depth: Some("high".to_string()),
        streaming: true,
        thinking: true,
        long_context: true,
    };
    let raw = serde_json::to_value(&values).expect("serialize values");
    assert_eq!(raw["executionMode"], "execute");
    let reference = FileReferenceInput {
        id: "reference".to_string(),
        path: "src/main.rs".to_string(),
        name: "main.rs".to_string(),
        size_bytes: Some(12),
        content_hash: None,
        start_line: Some(10),
        end_line: Some(50),
    };
    let serialized = serde_json::to_value(reference).expect("serialize reference");
    assert_eq!(serialized["sizeBytes"], 12);
    assert_eq!(serialized["startLine"], 10);
    assert_eq!(serialized["endLine"], 50);
    // A row written before line ranges existed must still deserialize, as a whole-file
    // reference — this is what makes the added fields need no schema migration.
    let legacy: FileReferenceInput = serde_json::from_str(
        r#"{"id":"legacy","path":"src/main.rs","name":"main.rs","sizeBytes":12,"contentHash":null}"#,
    )
    .expect("deserialize legacy reference");
    assert_eq!(legacy.start_line, None);
    assert_eq!(legacy.end_line, None);
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
            seat_id: "seat-1".to_string(),
            agent_id: "claude-code".to_string(),
            role_id: Some("role-architect".to_string()),
            role_snapshot: None,
            joined_at: "2026-08-07T00:00:00+00:00".to_string(),
            left_at: None,
            provider_thread_id: None,
        },
        SessionSeat {
            seat_id: "seat-2".to_string(),
            agent_id: "codex-cli".to_string(),
            role_id: Some("role-reviewer".to_string()),
            role_snapshot: None,
            joined_at: "2026-08-07T00:00:00+00:00".to_string(),
            left_at: None,
            provider_thread_id: None,
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
        seat_id: "seat-3".to_string(),
        agent_id: "gemini-cli".to_string(),
        role_id: None,
        role_snapshot: None,
        joined_at: "2026-08-07T00:00:01+00:00".to_string(),
        left_at: None,
        provider_thread_id: None,
    });
    let saved = SessionRepository::save(&fixture.repository, &loaded).expect("save seats");
    assert_eq!(saved.seats, loaded.seats);
}

/// A seat's provider thread is what its next turn resumes, so it has to survive the round trip
/// through the seat column. Losing it sends the seat back to a new thread and drops its history,
/// silently -- the turn still succeeds, it just does not remember anything.
#[test]
fn a_seats_provider_thread_survives_the_round_trip_independently_of_other_seats() {
    let fixture = fixture("sessions-seat-threads");
    let mut session = session_record(
        "session-seat-threads",
        SessionLifecycle::Idle,
        "多 Agent 线程",
        "2026-08-07T00:00:00+00:00",
    );
    session.runtime_session_id = Some("claude-thread".to_string());
    session.seats = vec![
        SessionSeat {
            seat_id: "seat-1".to_string(),
            agent_id: "claude-code".to_string(),
            role_id: Some("role-architect".to_string()),
            role_snapshot: None,
            joined_at: "2026-08-07T00:00:00+00:00".to_string(),
            left_at: None,
            provider_thread_id: Some("claude-thread".to_string()),
        },
        SessionSeat {
            seat_id: "seat-2".to_string(),
            agent_id: "codex-cli".to_string(),
            role_id: Some("role-reviewer".to_string()),
            role_snapshot: None,
            joined_at: "2026-08-07T00:00:00+00:00".to_string(),
            left_at: None,
            provider_thread_id: None,
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
    assert_eq!(
        loaded.seats[1].provider_thread_id, None,
        "a seat that has not spoken must not come back holding another seat's thread",
    );

    // The second seat speaks for the first time and reports a thread of its own.
    loaded.seats[1].provider_thread_id = Some("codex-thread".to_string());
    let saved = SessionRepository::save(&fixture.repository, &loaded).expect("save seat thread");
    assert_eq!(
        saved.seats[1].provider_thread_id.as_deref(),
        Some("codex-thread"),
    );
    assert_eq!(
        saved.seats[0].provider_thread_id.as_deref(),
        Some("claude-thread"),
        "recording one seat's thread must not disturb another's",
    );
    assert_eq!(
        saved.runtime_session_id.as_deref(),
        Some("claude-thread"),
        "the session keeps the first seat's thread, which is what pre-seat sessions resume",
    );
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
    fixture
        .database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE sessions SET seats = '[]' WHERE id = ?1",
            [session.id()],
        )
        .expect("clear legacy seats");

    let loaded = SessionRepository::find(&fixture.repository, session.aggregate.id())
        .expect("find single session")
        .expect("single session");
    assert_eq!(
        loaded.seats,
        vec![SessionSeat {
            seat_id: "session-single:seat:0".to_string(),
            agent_id: "codex-cli".to_string(),
            role_id: None,
            role_snapshot: None,
            joined_at: "2026-07-01T00:00:00+00:00".to_string(),
            left_at: None,
            provider_thread_id: None,
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
    seated.speaker_seat_id = Some("seat-reviewer".to_string());
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
    assert_eq!(loaded.speaker_seat_id.as_deref(), Some("seat-reviewer"));

    let loaded_user =
        SessionMessageRepository::find(&fixture.repository, spoken_by_user.message.id())
            .expect("find user")
            .expect("user message");
    assert_eq!(loaded_user.seat_index, None);
}

#[test]
fn message_inserts_allocate_unique_sequences_in_the_current_shared_schema() {
    let fixture = fixture("messages-additive-session-sequence");
    let session = session_record(
        "session-sequences",
        SessionLifecycle::Idle,
        "共享数据库序号",
        "2026-08-08T00:00:00+00:00",
    );
    SessionTransactionPort::create_session(
        &fixture.repository,
        &session,
        SessionActivation::PreserveActive,
    )
    .expect("create session");

    for id in ["message-sequence-1", "message-sequence-2"] {
        let message = message_record(
            id,
            session.id(),
            MessageRole::User,
            MessageStatus::Completed,
            id,
        );
        SessionMessageRepository::insert(&fixture.repository, &message).expect("insert message");
    }

    let connection = fixture.database.connection().expect("connection");
    let sequences = connection
        .prepare(
            "SELECT session_sequence FROM messages WHERE session_id = ?1 ORDER BY session_sequence",
        )
        .expect("prepare sequences")
        .query_map([session.id()], |row| row.get::<_, i64>(0))
        .expect("query sequences")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect sequences");
    assert_eq!(sequences, vec![1, 2]);
    assert_eq!(
        connection
            .query_row(
                "SELECT next_message_sequence FROM sessions WHERE id = ?1",
                [session.id()],
                |row| row.get::<_, i64>(0),
            )
            .expect("next sequence"),
        3
    );
}

#[test]
fn stable_participant_schema_normalizes_legacy_seats_and_backfills_only_valid_speakers() {
    let connection = rusqlite::Connection::open_in_memory().expect("database");
    connection
        .execute_batch(
            r#"
            CREATE TABLE sessions (id TEXT PRIMARY KEY, agent_id TEXT NOT NULL, created_at TEXT NOT NULL, seats TEXT NOT NULL);
            CREATE TABLE messages (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, seat_index INTEGER, created_at TEXT NOT NULL);
            "#,
        )
        .expect("legacy schema");
    connection
        .execute(
            "INSERT INTO sessions(id, agent_id, created_at, seats) VALUES (?1, ?2, ?3, ?4)",
            params![
                "shared",
                "codex-cli",
                "2026-08-01T00:00:00Z",
                r#"[{"agentId":"codex-cli","roleId":"reviewer"},{"agentId":"gemini-cli","roleId":"architect","leftAt":"2026-08-02T00:00:00Z"}]"#
            ],
        )
        .expect("shared session");
    connection
        .execute(
            "INSERT INTO sessions(id, agent_id, created_at, seats) VALUES (?1, ?2, ?3, ?4)",
            params!["single", "claude-code", "2026-08-01T00:00:00Z", "malformed"],
        )
        .expect("single session");
    for (id, session_id, seat_index) in [
        ("valid", "shared", 1_i64),
        ("invalid", "shared", 8_i64),
        ("single-valid", "single", 0_i64),
    ] {
        connection
            .execute(
                "INSERT INTO messages(id, session_id, seat_index, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, session_id, seat_index, "2026-08-03T00:00:00Z"],
            )
            .expect("legacy message");
    }

    apply_stable_participant_schema(&connection).expect("first migration");
    apply_stable_participant_schema(&connection).expect("idempotent migration");

    let shared: String = connection
        .query_row(
            "SELECT seats FROM sessions WHERE id = 'shared'",
            [],
            |row| row.get(0),
        )
        .expect("shared seats");
    let shared = crate::contexts::sessions::domain::decode_seats(
        &shared,
        "shared",
        "codex-cli",
        "2026-08-01T00:00:00Z",
    );
    assert_eq!(shared[0].seat_id, "shared:seat:0");
    assert_eq!(shared[1].seat_id, "shared:seat:1");
    assert_eq!(shared[1].left_at.as_deref(), Some("2026-08-02T00:00:00Z"));
    let speaker = |message_id: &str| -> Option<String> {
        connection
            .query_row(
                "SELECT speaker_seat_id FROM messages WHERE id = ?1",
                [message_id],
                |row| row.get(0),
            )
            .expect("speaker")
    };
    assert_eq!(speaker("valid").as_deref(), Some("shared:seat:1"));
    assert_eq!(speaker("invalid"), None);
    assert_eq!(speaker("single-valid").as_deref(), Some("single:seat:0"));
}
