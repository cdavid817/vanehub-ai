use super::*;
use crate::contexts::skill_evolution_curation::application::{
    CuratorNotificationPort, CuratorNotificationService,
};
use crate::contexts::skill_evolution_curation::domain::CuratorEventKind;
use rusqlite::{params, Connection};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

struct Publisher {
    fail: AtomicBool,
    payloads: Mutex<Vec<serde_json::Value>>,
}

impl Publisher {
    fn new(fail: bool) -> Self {
        Self {
            fail: AtomicBool::new(fail),
            payloads: Mutex::new(Vec::new()),
        }
    }
}

impl CuratorNotificationPort for Publisher {
    fn publish(
        &self,
        event: &crate::contexts::skill_evolution_curation::application::CuratorNotificationEvent,
    ) -> Result<(), ()> {
        self.payloads
            .lock()
            .expect("payload lock")
            .push(serde_json::to_value(event).expect("safe event"));
        if self.fail.load(Ordering::SeqCst) {
            Err(())
        } else {
            Ok(())
        }
    }
}

fn database(enabled: bool) -> Connection {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch("CREATE TABLE evolution_assessment_attempts (attempt_id TEXT PRIMARY KEY);")
        .expect("assessment boundary");
    apply_schema(&connection).expect("schema");
    connection
        .execute(
            "INSERT INTO evolution_assessment_attempts (attempt_id) VALUES ('assessment-1')",
            [],
        )
        .expect("assessment");
    connection
        .execute(
            "INSERT INTO evolution_curator_candidates
         (candidate_id,schema_version,workspace_id,seed_id,seed_revision,assessment_attempt_id,
          assessment_revision,target_skill_id,target_revision,overlay_scope,route,risk,confidence,
          policy_witness_hash,witness_hash,snapshot_json,state,revision,created_at_ms,updated_at_ms)
         VALUES ('candidate-1',1,'workspace-1','seed-1','1','assessment-1','1','review','target-1',
          'project','needs_human_review','medium','high','policy-hash','candidate-hash',
          '{\"privateRationale\":\"must never leave storage\"}','ready_for_review',7,1,2)",
            [],
        )
        .expect("candidate");
    connection
        .execute(
            "INSERT INTO evolution_curator_policy
         (workspace_id,schema_version,policy_json,policy_hash,revision,updated_at_ms)
         VALUES (?1,1,?2,'policy-hash',1,1)",
            params![
                "workspace-1",
                format!("{{\"notificationsEnabled\":{enabled}}}")
            ],
        )
        .expect("policy");
    connection
}

fn queue(connection: &mut Connection, kind: CuratorEventKind) {
    let transaction = connection.transaction().expect("transaction");
    queue_notification_receipt(&transaction, "candidate-1", 7, kind).expect("queue receipt");
    transaction.commit().expect("commit");
}

#[test]
fn receipts_deduplicate_and_publish_only_the_safe_projection() {
    let mut connection = database(true);
    queue(&mut connection, CuratorEventKind::DraftAssessed);
    queue(&mut connection, CuratorEventKind::DraftAssessed);
    let publisher = Publisher::new(false);
    let mut store = SqliteCuratorNotificationStore::new(&connection);
    let report = CuratorNotificationService::new(&mut store, &publisher)
        .dispatch(10)
        .expect("dispatch");

    assert_eq!(report.delivered, 1);
    let payloads = publisher.payloads.lock().expect("payload lock");
    let object = payloads[0].as_object().expect("notification object");
    assert_eq!(object.len(), 11);
    assert_eq!(object["candidateId"], "candidate-1");
    assert_eq!(object["eventKind"], "pending_review");
    assert!(!payloads[0].to_string().contains("privateRationale"));
    assert_eq!(
        receipt_status(&connection, "candidate-1", 7, "pending_review").as_deref(),
        Some("delivered")
    );
}

#[test]
fn disabled_policy_suppresses_delivery() {
    let mut connection = database(false);
    queue(&mut connection, CuratorEventKind::Rejected);
    let publisher = Publisher::new(false);
    let mut store = SqliteCuratorNotificationStore::new(&connection);
    let report = CuratorNotificationService::new(&mut store, &publisher)
        .dispatch(10)
        .expect("dispatch");

    assert_eq!(report.delivered, 0);
    assert!(publisher.payloads.lock().expect("payload lock").is_empty());
    assert_eq!(
        receipt_status(&connection, "candidate-1", 7, "rejection").as_deref(),
        Some("suppressed")
    );
}

#[test]
fn failed_delivery_is_recovered_without_changing_curator_state() {
    let mut connection = database(true);
    queue(&mut connection, CuratorEventKind::ApplicationFailed);
    let publisher = Publisher::new(true);
    let mut store = SqliteCuratorNotificationStore::new(&connection);
    let first = CuratorNotificationService::new(&mut store, &publisher)
        .dispatch(10)
        .expect("failed delivery is recorded");
    assert_eq!(first.failed, 1);
    assert_eq!(
        receipt_status(&connection, "candidate-1", 7, "apply_failure").as_deref(),
        Some("failed")
    );

    publisher.fail.store(false, Ordering::SeqCst);
    let recovered = CuratorNotificationService::new(&mut store, &publisher)
        .dispatch(11)
        .expect("recovery delivery");
    assert_eq!(recovered.delivered, 1);
    assert_eq!(
        receipt_status(&connection, "candidate-1", 7, "apply_failure").as_deref(),
        Some("delivered")
    );
    let state: String = connection
        .query_row(
            "SELECT state FROM evolution_curator_candidates WHERE candidate_id='candidate-1'",
            [],
            |row| row.get(0),
        )
        .expect("candidate state");
    assert_eq!(state, "ready_for_review");
}
