//! What survives about a session's personalization mode, and what cannot change it.

use super::*;

/// The mode a session was created in outlives the process it was created in.
///
/// Read back through a repository built over the same directory, which is what a restart is: the
/// database is reopened and the row is read by a new reader. A mode that only existed in memory
/// would silently become standard on the next launch, and a user who chose temporary would find
/// the following session remembering things.
#[test]
fn a_session_mode_survives_reopening_the_database() {
    let fixture = fixture("session-mode-restart");
    for (index, mode) in [
        SessionPersonalizationMode::Standard,
        SessionPersonalizationMode::ProjectOnly,
        SessionPersonalizationMode::Temporary,
    ]
    .into_iter()
    .enumerate()
    {
        let mut record = session_record(
            &format!("session-{index}"),
            SessionLifecycle::Idle,
            "Fixture",
            "2026-08-25T09:00:00Z",
        );
        record.personalization_mode = mode;
        record.workspace.folder = Some("D:/code/project".to_string());
        fixture
            .repository
            .create_session(&record, SessionActivation::PreserveActive)
            .expect("create");

        let reopened = SqliteSessionsRepository::new(fixture.database.clone());
        let read = SessionRepository::find(
            &reopened,
            &SessionId::parse(format!("session-{index}")).expect("session id"),
        )
        .expect("read")
        .expect("session");

        assert_eq!(read.personalization_mode, mode);
    }
}

/// Archiving is about whether a session is listed, not about what it retains.
///
/// The two are independent on purpose: a user who archives a temporary conversation has not asked
/// for it to start remembering things, and one who unarchives a project-only one has not asked for
/// it to widen.
#[test]
fn archiving_and_unarchiving_leave_the_mode_alone() {
    let fixture = fixture("session-mode-archive");
    let mut record = session_record(
        "session-1",
        SessionLifecycle::Idle,
        "Fixture",
        "2026-08-25T09:00:00Z",
    );
    record.personalization_mode = SessionPersonalizationMode::Temporary;
    fixture
        .repository
        .create_session(&record, SessionActivation::PreserveActive)
        .expect("create");

    let id = SessionId::parse("session-1").expect("session id");
    let mut stored = SessionRepository::find(&fixture.repository, &id)
        .expect("read")
        .expect("session");
    stored.aggregate.archive();
    SessionRepository::save(&fixture.repository, &stored).expect("archive");
    let archived = SessionRepository::find(&fixture.repository, &id)
        .expect("read")
        .expect("session");
    let mut stored = archived.clone();
    stored.aggregate.unarchive();
    SessionRepository::save(&fixture.repository, &stored).expect("unarchive");
    let restored = SessionRepository::find(&fixture.repository, &id)
        .expect("read")
        .expect("session");

    assert_eq!(
        archived.personalization_mode,
        SessionPersonalizationMode::Temporary
    );
    assert_eq!(
        restored.personalization_mode,
        SessionPersonalizationMode::Temporary
    );
}

/// A row written before the column existed opens, and opens as standard.
///
/// The migration backfills `'standard'`, so this is what a database created by an older build
/// looks like after upgrading. Reading it as anything else — or failing to read it — would make a
/// version upgrade lose or change sessions the user already had.
#[test]
fn a_row_written_before_the_column_existed_reads_as_standard() {
    let fixture = fixture("session-mode-legacy-row");
    let record = session_record(
        "session-1",
        SessionLifecycle::Idle,
        "Fixture",
        "2026-08-25T09:00:00Z",
    );
    fixture
        .repository
        .create_session(&record, SessionActivation::PreserveActive)
        .expect("create");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "UPDATE sessions SET personalization_mode = 'standard' WHERE id = 'session-1'",
            [],
        )
        .expect("backfilled value");

    let read = SessionRepository::find(
        &fixture.repository,
        &SessionId::parse("session-1").expect("session id"),
    )
    .expect("read")
    .expect("session");

    assert_eq!(
        read.personalization_mode,
        SessionPersonalizationMode::Standard
    );
}
