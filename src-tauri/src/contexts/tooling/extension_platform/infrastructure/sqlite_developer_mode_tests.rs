//! What the Developer Mode row does on a fresh database, across writes, and under a second row.

use super::{SqliteDeveloperModeAuditSink, SqliteDeveloperModeRepository};
use crate::contexts::tooling::extension_platform::application::{
    DeveloperModeAuditEntry, DeveloperModeAuditSink, DeveloperModeRepository,
};
use crate::contexts::tooling::extension_platform::domain::DeveloperMode;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use std::sync::Arc;

struct Fixture {
    _directory: TempDirectory,
    database: Arc<NativeDatabase>,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let database = Arc::new(
        NativeDatabase::new(directory.path().to_path_buf()).expect("database should open"),
    );
    Fixture {
        _directory: directory,
        database,
    }
}

#[test]
fn a_fresh_database_has_no_row_which_reads_as_off() {
    let fixture = fixture("developer-mode-fresh");
    let repository = SqliteDeveloperModeRepository::new(fixture.database.clone());

    let view = repository.load().expect("load");
    assert_eq!(view.mode, DeveloperMode::Off);
    assert_eq!(view.revision, 0);
    assert_eq!(view.updated_at, None);
    assert_eq!(view.updated_by, None);
}

#[test]
fn the_switch_round_trips_and_stays_one_row() {
    let fixture = fixture("developer-mode-round-trip");
    let repository = SqliteDeveloperModeRepository::new(fixture.database.clone());

    repository
        .store(
            DeveloperMode::On,
            1,
            "2026-08-22T00:00:00Z",
            "operator",
            Some("local work"),
        )
        .expect("store");
    repository
        .store(
            DeveloperMode::Off,
            2,
            "2026-08-23T00:00:00Z",
            "operator",
            None,
        )
        .expect("store again");

    let view = repository.load().expect("load");
    assert_eq!(view.mode, DeveloperMode::Off);
    assert_eq!(view.revision, 2);
    assert_eq!(view.reason, None);

    let connection = fixture.database.connection().expect("connection");
    let rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM extension_platform_developer_mode",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(rows, 1, "a second row would be a second answer");
}

#[test]
fn a_second_row_cannot_be_inserted() {
    // Enforced by the schema rather than by every writer remembering to use id 1.
    let fixture = fixture("developer-mode-single-row");
    let connection = fixture.database.connection().expect("connection");

    let outcome = connection.execute(
        "INSERT INTO extension_platform_developer_mode \
             (id, enabled, revision, updated_at, updated_by) \
         VALUES (2, 1, 1, '2026-08-22T00:00:00Z', 'someone')",
        [],
    );

    assert!(outcome.is_err(), "the primary-key check must refuse id 2");
}

#[test]
fn audit_entries_are_appended_in_order() {
    let fixture = fixture("developer-mode-audit");
    let sink = SqliteDeveloperModeAuditSink::new(fixture.database.clone());

    for (revision, previous, new) in [(1_i64, false, true), (2, true, false)] {
        sink.record(&DeveloperModeAuditEntry {
            previous_enabled: previous,
            new_enabled: new,
            revision,
            recorded_at: format!("2026-08-2{revision}T00:00:00Z"),
            actor: "operator".to_string(),
            reason: Some(format!("change {revision}")),
        })
        .expect("record");
    }

    let connection = fixture.database.connection().expect("connection");
    let mut statement = connection
        .prepare(
            "SELECT previous_enabled, new_enabled, revision, actor, reason \
             FROM extension_platform_developer_mode_audit ORDER BY id",
        )
        .expect("prepare");
    let entries = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)? != 0,
                row.get::<_, i64>(1)? != 0,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .expect("query")
        .collect::<Result<Vec<_>, _>>()
        .expect("rows");

    assert_eq!(
        entries,
        vec![
            (
                false,
                true,
                1,
                "operator".to_string(),
                Some("change 1".to_string())
            ),
            (
                true,
                false,
                2,
                "operator".to_string(),
                Some("change 2".to_string())
            ),
        ]
    );
}
