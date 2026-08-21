//! SQLite behavior for capability-gate desired state and audit.

use super::{SqliteFeatureGateAuditSink, SqliteFeatureGateRepository};
use crate::contexts::tooling::extension_platform::application::{
    FeatureGateAuditEntry, FeatureGateAuditSink, FeatureGateDegradationEntry,
    FeatureGateRepository, FeatureGateWrite,
};
use crate::contexts::tooling::extension_platform::domain::{
    ExtensionPlatformFeature, FeatureGateDegradation,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;
use rusqlite::params;
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

fn write(
    feature: ExtensionPlatformFeature,
    desired_enabled: bool,
    expected_revision: i64,
) -> FeatureGateWrite {
    FeatureGateWrite {
        feature,
        desired_enabled,
        expected_revision,
        updated_at: "2026-08-22T00:00:00Z".to_string(),
        updated_by: "operator".to_string(),
        reason: None,
    }
}

#[test]
fn a_fresh_database_has_no_gate_rows_which_reads_as_every_gate_disabled() {
    let fixture = fixture("extension-gate-empty");
    let repository = SqliteFeatureGateRepository::new(Arc::clone(&fixture.database));

    assert!(repository
        .load_all()
        .expect("load should succeed")
        .is_empty());
}

#[test]
fn a_gate_round_trips_through_storage() {
    let fixture = fixture("extension-gate-round-trip");
    let repository = SqliteFeatureGateRepository::new(Arc::clone(&fixture.database));

    let stored = repository
        .upsert(&FeatureGateWrite {
            reason: Some("gate 1 parity".to_string()),
            ..write(ExtensionPlatformFeature::Catalog, true, 0)
        })
        .expect("upsert should succeed");
    assert_eq!(stored.revision, 1);

    let loaded = repository.load_all().expect("load should succeed");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].feature, ExtensionPlatformFeature::Catalog);
    assert!(loaded[0].desired_enabled);
    assert_eq!(loaded[0].revision, 1);
    assert_eq!(loaded[0].reason.as_deref(), Some("gate 1 parity"));
}

#[test]
fn revision_increments_and_a_stale_writer_is_rejected_without_overwriting() {
    let fixture = fixture("extension-gate-revision");
    let repository = SqliteFeatureGateRepository::new(Arc::clone(&fixture.database));

    repository
        .upsert(&write(ExtensionPlatformFeature::Catalog, true, 0))
        .expect("first write should succeed");

    let error = repository
        .upsert(&write(ExtensionPlatformFeature::Catalog, false, 0))
        .expect_err("a writer holding revision 0 must be rejected");
    assert_eq!(error.code(), "stale_revision");

    let loaded = repository.load_all().expect("load should succeed");
    assert!(loaded[0].desired_enabled, "stale write must not land");
    assert_eq!(loaded[0].revision, 1);

    repository
        .upsert(&write(ExtensionPlatformFeature::Catalog, false, 1))
        .expect("a writer holding the current revision should succeed");
    let loaded = repository.load_all().expect("load should succeed");
    assert!(!loaded[0].desired_enabled);
    assert_eq!(loaded[0].revision, 2);
}

#[test]
fn an_unknown_gate_key_is_skipped_rather_than_failing_the_whole_read() {
    let fixture = fixture("extension-gate-unknown-key");
    let repository = SqliteFeatureGateRepository::new(Arc::clone(&fixture.database));
    repository
        .upsert(&write(ExtensionPlatformFeature::Catalog, true, 0))
        .expect("known gate should persist");

    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO extension_platform_feature_gates \
                 (feature, desired_enabled, revision, updated_at, updated_by, reason) \
             VALUES (?1, 1, 1, '2026-08-22T00:00:00Z', 'operator', NULL)",
            params!["extension_platform.retired_gate"],
        )
        .expect("row should insert");

    let loaded = repository.load_all().expect("load should still succeed");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].feature, ExtensionPlatformFeature::Catalog);
}

#[test]
fn build_availability_is_never_persisted() {
    let fixture = fixture("extension-gate-columns");
    let connection = fixture.database.connection().expect("connection");
    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info('extension_platform_feature_gates')")
        .expect("pragma should prepare");
    let columns: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("columns should read");

    assert_eq!(
        columns,
        vec![
            "feature",
            "desired_enabled",
            "revision",
            "updated_at",
            "updated_by",
            "reason"
        ]
    );
}

#[test]
fn degradation_records_store_only_stable_codes() {
    let fixture = fixture("extension-gate-degradation");
    let sink = SqliteFeatureGateAuditSink::new(Arc::clone(&fixture.database));

    for degradation in [
        FeatureGateDegradation::NeverLoaded,
        FeatureGateDegradation::ReloadFailed,
    ] {
        sink.record_degradation(&FeatureGateDegradationEntry {
            degradation,
            code: "storage",
            recorded_at: "2026-08-22T00:00:00Z".to_string(),
        })
        .expect("degradation should record");
    }

    let connection = fixture.database.connection().expect("connection");
    let mut statement = connection
        .prepare(
            "SELECT degradation, code FROM extension_platform_feature_gate_degradations \
             ORDER BY id ASC",
        )
        .expect("statement should prepare");
    let rows: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("rows should read");

    assert_eq!(
        rows,
        vec![
            ("never_loaded".to_string(), "storage".to_string()),
            ("reload_failed".to_string(), "storage".to_string()),
        ]
    );

    // No free-form detail column: this table is written on exactly the path where a storage
    // error message is most likely to carry a filesystem path.
    let mut columns = connection
        .prepare(
            "SELECT name FROM pragma_table_info('extension_platform_feature_gate_degradations')",
        )
        .expect("pragma should prepare");
    let names: Vec<String> = columns
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("columns should read");
    assert_eq!(names, vec!["id", "degradation", "code", "recorded_at"]);
}

#[test]
fn audit_entries_are_appended_and_retained_in_order() {
    let fixture = fixture("extension-gate-audit");
    let sink = SqliteFeatureGateAuditSink::new(Arc::clone(&fixture.database));

    for (index, (previous, new)) in [(false, true), (true, false)].into_iter().enumerate() {
        sink.record(&FeatureGateAuditEntry {
            feature: ExtensionPlatformFeature::Connectors,
            previous_enabled: previous,
            new_enabled: new,
            revision: index as i64 + 1,
            recorded_at: "2026-08-22T00:00:00Z".to_string(),
            actor: "operator".to_string(),
            reason: None,
        })
        .expect("audit should record");
    }

    let connection = fixture.database.connection().expect("connection");
    let mut statement = connection
        .prepare(
            "SELECT previous_enabled, new_enabled, revision \
             FROM extension_platform_feature_gate_audit \
             WHERE feature = ?1 ORDER BY id ASC",
        )
        .expect("statement should prepare");
    let rows: Vec<(i64, i64, i64)> = statement
        .query_map(
            params![ExtensionPlatformFeature::Connectors.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("query should run")
        .collect::<Result<Vec<_>, _>>()
        .expect("rows should read");

    assert_eq!(rows, vec![(0, 1, 1), (1, 0, 2)]);
}
