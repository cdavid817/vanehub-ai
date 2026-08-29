use std::{sync::Arc, thread, time::Duration};

use crate::{
    contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, OperationsError},
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

use super::super::api::SkillEvolutionOrchestrationApi;

#[derive(Default)]
struct NoopLogs;

impl DiagnosticLogPort for NoopLogs {
    fn write_diagnostic(&self, _log: DiagnosticLog) -> Result<(), OperationsError> {
        Ok(())
    }
}

#[test]
fn desktop_background_runs_while_window_independent_and_stops_on_quit() {
    let directory = TempDirectory::new("orchestration-background-lifecycle");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let api = SkillEvolutionOrchestrationApi::new(database.clone(), Arc::new(NoopLogs));
    api.update_policy_projection(
        "workspace:0123456789abcdef01234567",
        0,
        "observe",
        vec!["skill-one".into()],
        false,
        100,
    )
    .expect("observe policy");
    let lifecycle = api.background_lifecycle();
    lifecycle.start(Duration::from_millis(10)).expect("start");
    assert!(lifecycle.is_running());
    wait_for_receipts(&database, 2);
    lifecycle.shutdown().expect("shutdown");
    assert!(!lifecycle.is_running());
    let stopped_count = receipt_count(&database);
    thread::sleep(Duration::from_millis(30));
    assert_eq!(receipt_count(&database), stopped_count);
}

fn wait_for_receipts(database: &NativeDatabase, expected: i64) {
    for _ in 0..50 {
        if receipt_count(database) >= expected {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("background lifecycle did not publish due triggers");
}

fn receipt_count(database: &NativeDatabase) -> i64 {
    database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM evolution_trigger_receipts",
            [],
            |row| row.get(0),
        )
        .expect("receipt count")
}
