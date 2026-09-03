use std::sync::{Arc, Mutex};

use crate::{
    contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, OperationsError},
    platform::database::NativeDatabase,
    test_support::TempDirectory,
};

use super::api::SkillEvolutionOrchestrationApi;

#[derive(Default)]
struct CapturingLogs(Mutex<Vec<DiagnosticLog>>);

impl DiagnosticLogPort for CapturingLogs {
    fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), OperationsError> {
        self.0.lock().expect("logs").push(log);
        Ok(())
    }
}

#[test]
fn feedback_authorization_change_queues_both_closed_trigger_families_idempotently() {
    let directory = TempDirectory::new("orchestration-feedback-api");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let logs = Arc::new(CapturingLogs::default());
    let api = SkillEvolutionOrchestrationApi::new(database.clone(), logs.clone());
    api.publish_feedback_change(
        Some("workspace:0123456789abcdef01234567"),
        "message-one",
        2,
        Some("authorization-one:granted"),
    );
    api.publish_feedback_change(
        Some("workspace:0123456789abcdef01234567"),
        "message-one",
        2,
        Some("authorization-one:revoked"),
    );
    api.publish_feedback_change(
        Some("workspace:0123456789abcdef01234567"),
        "message-one",
        2,
        Some("authorization-one:granted"),
    );
    let connection = database.connection().expect("connection");
    let families: String = connection
        .query_row(
            "SELECT GROUP_CONCAT(family, ',') FROM (
               SELECT family FROM evolution_trigger_receipts ORDER BY family
             )",
            [],
            |row| row.get(0),
        )
        .expect("families");
    assert_eq!(
        families,
        "explicit_feedback_commit,relevant_policy_or_skill_change,relevant_policy_or_skill_change"
    );
    assert!(logs.0.lock().expect("logs").is_empty());
}

#[test]
fn missing_workspace_fails_isolated_with_safe_diagnostic() {
    let directory = TempDirectory::new("orchestration-feedback-api-missing");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let logs = Arc::new(CapturingLogs::default());
    let api = SkillEvolutionOrchestrationApi::new(database, logs.clone());
    api.publish_feedback_change(None, "message-one", 1, Some("authorization-one:revoked"));
    let captured = logs.0.lock().expect("logs");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].context["reason"], "workspace-unavailable");
}

#[test]
fn service_projections_default_off_and_expose_only_safe_policy_fields() {
    let directory = TempDirectory::new("orchestration-service-projection");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let api = SkillEvolutionOrchestrationApi::new(database, Arc::new(CapturingLogs::default()));
    let workspace = "workspace:0123456789abcdef01234567";
    let initial = api
        .policy_projection(workspace, 100)
        .expect("initial policy");
    assert_eq!(initial["mode"], "off");
    assert_eq!(initial["revision"], 0);
    let updated = api
        .update_policy_projection(
            workspace,
            0,
            "observe",
            vec!["skill-one".into()],
            false,
            101,
        )
        .expect("observe policy");
    assert_eq!(updated["mode"], "observe");
    assert_eq!(updated["allowedSkillIds"], serde_json::json!(["skill-one"]));
    let encoded = updated.to_string();
    for forbidden in ["prompt", "correction", "terminal", "toolArguments", "diff"] {
        assert!(!encoded.contains(forbidden));
    }
    assert_eq!(
        api.update_policy_projection(workspace, 0, "off", vec![], false, 102),
        Err("stale_conflict".into())
    );
}

#[test]
fn manual_trigger_and_bounded_history_queries_share_the_service_boundary() {
    let directory = TempDirectory::new("orchestration-service-manual");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let api = SkillEvolutionOrchestrationApi::new(database, Arc::new(CapturingLogs::default()));
    let workspace = "workspace:0123456789abcdef01234567";
    let receipt = api
        .request_manual_run(workspace, 200)
        .expect("manual receipt");
    assert_eq!(receipt["queued"], true);
    assert!(receipt["requestId"]
        .as_str()
        .is_some_and(|id| !id.is_empty()));
    let overview = api.scheduler_overview(workspace).expect("overview");
    assert_eq!(overview["pendingTriggers"], 1);
    assert_eq!(api.runs(workspace, None, 0), Err("invalid_input".into()));
    assert_eq!(api.run_detail("missing"), Err("not_found".into()));
}

#[test]
fn eligibility_projection_exposes_safe_draft_provenance_and_preflight_state() {
    let directory = TempDirectory::new("orchestration-eligibility-projection");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let api =
        SkillEvolutionOrchestrationApi::new(database.clone(), Arc::new(CapturingLogs::default()));
    let workspace = "workspace-one";
    let receipt = api.request_manual_run(workspace, 100).expect("request");
    let request_id = receipt["requestId"].as_str().expect("request id");
    let connection = database.connection().expect("connection");
    connection.execute_batch(&format!(
        "INSERT INTO evolution_runs
         (run_id,schema_version,request_id,workspace_id,status,current_stage,policy_witness_hash,
          budget_json,usage_json,revision,created_at_ms,updated_at_ms)
         VALUES ('run-one',1,'{request_id}','{workspace}','completed',NULL,'sha256:policy','{{}}','{{}}',0,100,100);
         INSERT INTO evolution_correction_authorizations VALUES
         ('authorization-one','feedback-one',1,'v1',1,'user','sha256:authorization',100,NULL);
         INSERT INTO evolution_deterministic_drafts VALUES
         ('draft-one','{workspace}','skill-one','authorization-one','assessment-one','producer-v1',
          'sha256:content',10,'deterministic_authorized_correction','sha256:source',100);
         INSERT INTO evolution_auto_eligibility VALUES
         ('eligibility-one','run-one','draft-one','skill-one','eligible','[]','sha256:proof',
          'sha256:preview',100,0);"
    )).expect("safe eligibility fixture");
    drop(connection);
    let page = api.eligibility(workspace, None, 10).expect("eligibility");
    assert_eq!(
        page["items"][0]["draftProvenance"],
        "deterministic_authorized_correction"
    );
    assert_eq!(page["items"][0]["preflightState"], "not_issued");
    let encoded = page.to_string();
    for forbidden in [
        "correction_text",
        "terminal",
        "toolArguments",
        "diffContent",
    ] {
        assert!(!encoded.contains(forbidden));
    }
}

#[test]
fn notification_delivery_failure_is_isolated_from_later_safe_events() {
    let directory = TempDirectory::new("orchestration-notification-isolation");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let connection = database.connection().expect("connection");
    for (event_id, revision) in [("breaker_opened:one:1", 1), ("breaker_opened:two:1", 2)] {
        connection
            .execute(
                "INSERT INTO evolution_orchestration_notification_receipts VALUES
             (?1,1,'breaker_opened','workspace-one',NULL,NULL,NULL,?1,NULL,
              'integrity_failure',NULL,?2,'pending',100,NULL)",
                rusqlite::params![event_id, revision],
            )
            .expect("notification");
    }
    drop(connection);
    let api = SkillEvolutionOrchestrationApi::new(database, Arc::new(CapturingLogs::default()));
    let mut attempts = 0;
    let report = api
        .dispatch_notifications(101, |_| {
            attempts += 1;
            if attempts == 1 {
                Err(())
            } else {
                Ok(())
            }
        })
        .expect("dispatch report");
    assert_eq!(report, serde_json::json!({ "delivered": 1, "failed": 1 }));
    assert_eq!(attempts, 2);
}
