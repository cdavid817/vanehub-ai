use super::attempt_repository::{
    AttemptDispatch, AttemptTerminalUpdate, AttemptVerificationDispatch, VerificationEvidenceUpdate,
};
use super::repository::{storage_error, SqlitePlanRepository};
use crate::contexts::task_orchestration::application::{
    AttemptRepairContext, PlanApplicationError,
};
use crate::contexts::task_orchestration::domain::{
    ResourceLimits, SubTaskSpec, VerificationCommand,
};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FinalRepairDispatch {
    pub(crate) finalization_id: String,
    pub(crate) repair_id: String,
    pub(crate) attempt: AttemptDispatch,
}

impl SqlitePlanRepository {
    pub(crate) fn claim_final_repair(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<Option<FinalRepairDispatch>, PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let candidate = transaction
            .query_row(
                r#"SELECT finalization.id, finalization.sequence, run.project_path,
                          run.worktree_path, version.planner_profile_id,
                          policy.max_attempts_per_subtask, policy.repair_eligible_classes
                   FROM plan_runs AS run
                   JOIN plan_versions AS version ON version.id = run.plan_version_id
                   JOIN plan_run_policies AS policy ON policy.plan_run_id = run.id
                   JOIN plan_finalizations AS finalization ON finalization.plan_run_id = run.id
                   WHERE run.id = ?1 AND run.status = 'action_required'
                     AND finalization.status = 'failed'
                     AND NOT EXISTS (
                         SELECT 1 FROM plan_finalizations AS newer
                         WHERE newer.plan_run_id = run.id
                           AND newer.sequence > finalization.sequence
                     )
                     AND NOT EXISTS (
                         SELECT 1 FROM plan_final_repair_attempts AS repair
                         WHERE repair.finalization_id = finalization.id
                     )"#,
                [run_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, u16>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, u16>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(storage_error)?;
        let Some((
            finalization_id,
            sequence,
            project_path,
            worktree_path,
            profile_id,
            maximum,
            raw_classes,
        )) = candidate
        else {
            return Ok(None);
        };
        let classes: Vec<String> = serde_json::from_str(&raw_classes).map_err(storage_error)?;
        if sequence >= maximum || !classes.iter().any(|class| class == "verification_failed") {
            return Ok(None);
        }
        let worktree_path = worktree_path.ok_or(PlanApplicationError::Conflict)?;
        let profile_id = profile_id.ok_or_else(|| {
            PlanApplicationError::Validation(
                "approved Plan version has no captured OnePiece Profile".to_string(),
            )
        })?;
        let mut evidence_statement = transaction
            .prepare(
                r#"SELECT command_id, output_summary
                   FROM plan_final_verification_evidence
                   WHERE finalization_id = ?1 AND status != 'passed'
                   ORDER BY created_at, id LIMIT 8"#,
            )
            .map_err(storage_error)?;
        let evidence = evidence_statement
            .query_map([&finalization_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        drop(evidence_statement);
        let changed_files = final_changed_files(&transaction, run_id)?;
        let repair_id = Uuid::new_v4().to_string();
        transaction
            .execute(
                r#"INSERT INTO plan_final_repair_attempts
                   (id, finalization_id, sequence, status, started_at)
                   VALUES (?1, ?2, 1, 'dispatching', ?3)"#,
                params![repair_id, finalization_id, now],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                "UPDATE plan_runs SET status = 'running', updated_at = ?2 WHERE id = ?1 AND status = 'action_required'",
                params![run_id, now],
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        let repair = AttemptRepairContext {
            attempt_sequence: sequence,
            remaining_attempts: maximum.saturating_sub(sequence),
            error_class: "verification_failed".to_string(),
            failed_command_ids: evidence.iter().map(|item| item.0.clone()).collect(),
            output_summaries: evidence
                .into_iter()
                .filter_map(|item| item.1)
                .map(|summary| summary.chars().take(500).collect())
                .collect(),
            changed_files,
        };
        Ok(Some(FinalRepairDispatch {
            finalization_id,
            repair_id,
            attempt: AttemptDispatch {
                plan_run_id: run_id.to_string(),
                subtask_run_id: "final-repair".to_string(),
                task: SubTaskSpec {
                    id: "final-repair".to_string(),
                    title: "Repair integrated Plan verification".to_string(),
                    description: "Fix the bounded final verification failures without changing the approved task graph.".to_string(),
                    acceptance_criteria: vec!["The failing final checks are repaired.".to_string()],
                    criterion_evidence: Vec::new(),
                    ordinal: 0,
                    assigned_role: "worker".to_string(),
                    limits: ResourceLimits {
                        token_budget: Some(8_000),
                        tool_call_limit: Some(30),
                        timeout_seconds: Some(900),
                    },
                    validation_commands: Vec::new(),
                },
                project_path,
                worktree_path,
                profile_id,
                direct_predecessor_ids: Vec::new(),
                predecessor_sources: Vec::new(),
                repair: Some(repair),
            },
        }))
    }

    pub(crate) fn start_final_repair(
        &self,
        dispatch: &FinalRepairDispatch,
        session_id: &str,
        profile_id: &str,
    ) -> Result<(), PlanApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE plan_final_repair_attempts
               SET status = 'running', session_id = ?2, profile_id = ?3
               WHERE id = ?1 AND finalization_id = ?4 AND status = 'dispatching'"#,
                params![
                    dispatch.repair_id,
                    session_id,
                    profile_id,
                    dispatch.finalization_id
                ],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        Ok(())
    }

    pub(crate) fn fail_final_repair_dispatch(
        &self,
        dispatch: &FinalRepairDispatch,
        error_class: &str,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        transaction.execute(
            "UPDATE plan_final_repair_attempts SET status = 'failed', error_class = ?2, completed_at = ?3 WHERE id = ?1 AND status = 'dispatching'",
            params![dispatch.repair_id, error_class, now],
        ).map_err(storage_error)?;
        transaction.execute(
            "UPDATE plan_runs SET status = 'action_required', updated_at = ?2 WHERE id = ?1 AND status = 'running'",
            params![dispatch.attempt.plan_run_id, now],
        ).map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn finish_final_repair(
        &self,
        dispatch: &FinalRepairDispatch,
        update: &AttemptTerminalUpdate,
        operation_id: Option<&str>,
        execution_run_id: Option<&str>,
        succeeded: bool,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let status = if succeeded { "succeeded" } else { "failed" };
        let changed = transaction
            .execute(
                r#"UPDATE plan_final_repair_attempts SET status = ?2, operation_id = ?3,
                      execution_run_id = ?4, token_usage = ?5, tool_call_count = ?6,
                      error_class = ?7, completed_at = ?8
               WHERE id = ?1 AND status = 'running'"#,
                params![
                    dispatch.repair_id,
                    status,
                    operation_id,
                    execution_run_id,
                    update.token_usage,
                    update.tool_call_count,
                    update.error_class,
                    now
                ],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        transaction.execute(
            "UPDATE plan_runs SET status = ?2, updated_at = ?3 WHERE id = ?1 AND status = 'running'",
            params![dispatch.attempt.plan_run_id, if succeeded { "final_verifying" } else { "action_required" }, now],
        ).map_err(storage_error)?;
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn load_final_verification(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<AttemptVerificationDispatch, PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let header = transaction
            .query_row(
                r#"SELECT run.worktree_path, policy.final_validation_commands
                   FROM plan_runs AS run
                   JOIN plan_run_policies AS policy ON policy.plan_run_id = run.id
                   WHERE run.id = ?1 AND run.status = 'final_verifying'
                     AND run.worktree_path IS NOT NULL"#,
                [run_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::Conflict)?;
        let existing = transaction
            .query_row(
                r#"SELECT id FROM plan_finalizations
                   WHERE plan_run_id = ?1 AND status = 'running'
                   ORDER BY sequence DESC LIMIT 1"#,
                [run_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_error)?;
        let finalization_id = if let Some(id) = existing {
            id
        } else {
            let id = Uuid::new_v4().to_string();
            let sequence: u32 = transaction
                .query_row(
                    "SELECT COALESCE(MAX(sequence), 0) + 1 FROM plan_finalizations WHERE plan_run_id = ?1",
                    [run_id],
                    |row| row.get(0),
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    r#"INSERT INTO plan_finalizations
                       (id, plan_run_id, sequence, status, created_at)
                       VALUES (?1, ?2, ?3, 'running', ?4)"#,
                    params![id, run_id, sequence, now],
                )
                .map_err(storage_error)?;
            id
        };
        transaction.commit().map_err(storage_error)?;
        Ok(AttemptVerificationDispatch {
            attempt_id: finalization_id,
            plan_run_id: run_id.to_string(),
            subtask_run_id: String::new(),
            worktree_path: header.0,
            commands: serde_json::from_str::<Vec<VerificationCommand>>(&header.1)
                .map_err(storage_error)?,
        })
    }

    pub(crate) fn finish_final_verification(
        &self,
        dispatch: &AttemptVerificationDispatch,
        evidence: &[VerificationEvidenceUpdate],
        passed: bool,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        for item in evidence {
            let duration_ms = item
                .duration_ms
                .map(i64::try_from)
                .transpose()
                .map_err(storage_error)?;
            transaction
                .execute(
                    r#"INSERT INTO plan_final_verification_evidence
                       (id, finalization_id, command_id, status, exit_code, duration_ms,
                        output_summary, created_at)
                       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
                    params![
                        Uuid::new_v4().to_string(),
                        dispatch.attempt_id,
                        item.command_id,
                        item.status,
                        item.exit_code,
                        duration_ms,
                        item.output_summary,
                        now
                    ],
                )
                .map_err(storage_error)?;
        }
        let final_status = if passed { "succeeded" } else { "failed" };
        let run_status = if passed {
            "awaiting_acceptance"
        } else {
            "action_required"
        };
        let finalized = transaction
            .execute(
                "UPDATE plan_finalizations SET status = ?2, completed_at = ?3 WHERE id = ?1 AND status = 'running'",
                params![dispatch.attempt_id, final_status, now],
            )
            .map_err(storage_error)?;
        let projected = transaction
            .execute(
                "UPDATE plan_runs SET status = ?2, updated_at = ?3 WHERE id = ?1 AND status = 'final_verifying'",
                params![dispatch.plan_run_id, run_status, now],
            )
            .map_err(storage_error)?;
        if finalized != 1 || projected != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        transaction.commit().map_err(storage_error)
    }
}

fn final_changed_files(
    transaction: &rusqlite::Transaction<'_>,
    run_id: &str,
) -> Result<Vec<String>, PlanApplicationError> {
    let mut statement = transaction
        .prepare(
            "SELECT changed_files FROM plan_subtask_runs WHERE plan_run_id = ?1 ORDER BY ordinal",
        )
        .map_err(storage_error)?;
    let values = statement
        .query_map([run_id], |row| row.get::<_, String>(0))
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    let mut files = values
        .into_iter()
        .flat_map(|value| serde_json::from_str::<Vec<String>>(&value).unwrap_or_default())
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    files.truncate(50);
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::database::NativeDatabase;
    use crate::test_support::TempDirectory;

    fn repository() -> SqlitePlanRepository {
        let directory = TempDirectory::new("plan-finalization");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqlitePlanRepository::new(database);
        repository.connection().expect("connection").execute_batch(
            r#"INSERT INTO plans (id, status, current_version, created_at, updated_at)
               VALUES ('plan', 'approved', 1, 'now', 'now');
               INSERT INTO plan_versions
                   (id, plan_id, version, goal, project_path, base_ref, created_at, approved_at)
               VALUES ('version', 'plan', 1, 'goal', 'C:\code', 'main', 'now', 'now');
               INSERT INTO plan_runs
                   (id, plan_id, plan_version_id, status, project_path, base_ref,
                    worktree_path, created_at, updated_at)
               VALUES ('run', 'plan', 'version', 'final_verifying', 'C:\code', 'main',
                       'C:\worktree', 'now', 'now');
               INSERT INTO plan_run_policies
                   (plan_run_id, discovery_status, discovery_limitations,
                    max_attempts_per_subtask, repair_eligible_classes, final_validation_commands)
               VALUES ('run', 'complete', '[]', 3, '["verification_failed"]',
                       '[{"id":"final","program":"cargo","args":["test"],"workingDirectory":null,"timeoutSeconds":300,"required":true}]');"#,
        ).expect("fixture");
        repository
    }

    #[test]
    fn finalization_is_singleton_and_retains_guarded_evidence() {
        let repository = repository();
        let first = repository
            .load_final_verification("run", "2026-08-12T00:00:00Z")
            .expect("first");
        let second = repository
            .load_final_verification("run", "2026-08-12T00:00:01Z")
            .expect("second");
        assert_eq!(first.attempt_id, second.attempt_id);
        assert_eq!(first.commands.len(), 1);
        repository
            .finish_final_verification(
                &first,
                &[VerificationEvidenceUpdate {
                    command_id: "final".into(),
                    status: "passed".into(),
                    exit_code: Some(0),
                    duration_ms: Some(25),
                    output_summary: Some("bounded summary".into()),
                }],
                true,
                "2026-08-12T00:00:02Z",
            )
            .expect("finish");
        let connection = repository.connection().expect("connection");
        let state: String = connection
            .query_row("SELECT status FROM plan_runs WHERE id = 'run'", [], |row| {
                row.get(0)
            })
            .expect("run status");
        let evidence: String = connection
            .query_row(
                "SELECT output_summary FROM plan_final_verification_evidence WHERE finalization_id = ?1",
                [&first.attempt_id],
                |row| row.get(0),
            )
            .expect("evidence");
        assert_eq!(state, "awaiting_acceptance");
        assert_eq!(evidence, "bounded summary");
    }

    #[test]
    fn failed_final_verification_requires_action_before_acceptance() {
        let repository = repository();
        let dispatch = repository
            .load_final_verification("run", "2026-08-12T00:00:00Z")
            .expect("dispatch");
        repository
            .finish_final_verification(&dispatch, &[], false, "2026-08-12T00:00:01Z")
            .expect("finish");
        assert_eq!(
            repository.run_status("run").expect("status").as_str(),
            "action_required"
        );
        assert!(repository
            .accept_run("run", "2026-08-12T00:00:02Z")
            .is_err());
    }

    #[test]
    fn final_verification_retry_is_bounded_and_retains_prior_evidence() {
        let repository = repository();
        for sequence in 1..=3 {
            let dispatch = repository
                .load_final_verification("run", &format!("2026-08-12T00:00:0{sequence}Z"))
                .expect("dispatch");
            repository
                .finish_final_verification(
                    &dispatch,
                    &[VerificationEvidenceUpdate {
                        command_id: "final".into(),
                        status: "failed".into(),
                        exit_code: Some(1),
                        duration_ms: Some(25),
                        output_summary: Some(format!("failure {sequence}")),
                    }],
                    false,
                    &format!("2026-08-12T00:00:1{sequence}Z"),
                )
                .expect("finish");
            if sequence < 3 {
                repository
                    .retry_final_verification("run", &format!("2026-08-12T00:00:2{sequence}Z"))
                    .expect("retry");
                repository
                    .connection()
                    .expect("connection")
                    .execute(
                        "UPDATE plan_runs SET status = 'final_verifying' WHERE id = 'run'",
                        [],
                    )
                    .expect("project final verification");
            }
        }
        assert!(repository
            .retry_final_verification("run", "2026-08-12T00:00:30Z")
            .is_err());
        let retained: u16 = repository
            .connection()
            .expect("connection")
            .query_row(
                "SELECT COUNT(*) FROM plan_final_verification_evidence",
                [],
                |row| row.get(0),
            )
            .expect("retained evidence");
        assert_eq!(retained, 3);
    }

    #[test]
    fn final_repair_claim_is_singleton_and_uses_bounded_failed_evidence() {
        let repository = repository();
        repository
            .connection()
            .expect("connection")
            .execute(
                "UPDATE plan_versions SET planner_profile_id = 'profile' WHERE id = 'version'",
                [],
            )
            .expect("profile");
        let verification = repository
            .load_final_verification("run", "2026-08-12T00:00:00Z")
            .expect("verification");
        repository
            .finish_final_verification(
                &verification,
                &[VerificationEvidenceUpdate {
                    command_id: "final".into(),
                    status: "failed".into(),
                    exit_code: Some(1),
                    duration_ms: Some(20),
                    output_summary: Some("bounded final failure".into()),
                }],
                false,
                "2026-08-12T00:00:01Z",
            )
            .expect("failed verification");
        let repair = repository
            .claim_final_repair("run", "2026-08-12T00:00:02Z")
            .expect("claim")
            .expect("repair");
        let context = repair.attempt.repair.as_ref().expect("context");
        assert_eq!(context.failed_command_ids, ["final"]);
        assert_eq!(context.output_summaries, ["bounded final failure"]);
        assert_eq!(context.remaining_attempts, 2);
        assert!(repository
            .claim_final_repair("run", "2026-08-12T00:00:03Z")
            .expect("duplicate")
            .is_none());
        repository
            .fail_final_repair_dispatch(&repair, "session_creation_failed", "2026-08-12T00:00:04Z")
            .expect("fail repair");
        assert_eq!(
            repository.run_status("run").expect("status").as_str(),
            "action_required"
        );
    }

    #[test]
    fn successful_final_repair_returns_to_a_new_verification() {
        let repository = repository();
        repository
            .connection()
            .expect("connection")
            .execute(
                "UPDATE plan_versions SET planner_profile_id = 'profile' WHERE id = 'version'",
                [],
            )
            .expect("profile");
        let first = repository
            .load_final_verification("run", "2026-08-12T00:00:00Z")
            .expect("first");
        repository
            .finish_final_verification(&first, &[], false, "2026-08-12T00:00:01Z")
            .expect("failed verification");
        let repair = repository
            .claim_final_repair("run", "2026-08-12T00:00:02Z")
            .expect("claim")
            .expect("repair");
        repository
            .connection()
            .expect("connection")
            .execute(
                "UPDATE plan_final_repair_attempts SET status = 'running' WHERE id = ?1",
                [&repair.repair_id],
            )
            .expect("start repair fixture");
        repository
            .finish_final_repair(
                &repair,
                &AttemptTerminalUpdate {
                    result_summary: Some("repaired".into()),
                    changed_files: Vec::new(),
                    token_usage: 10,
                    tool_call_count: 2,
                    error_class: None,
                },
                Some("operation"),
                Some("execution"),
                true,
                "2026-08-12T00:00:03Z",
            )
            .expect("finish repair");
        let second = repository
            .load_final_verification("run", "2026-08-12T00:00:04Z")
            .expect("second");
        assert_ne!(first.attempt_id, second.attempt_id);
    }

    #[test]
    fn cancelling_final_verification_settles_the_run_and_finalization() {
        let repository = repository();
        repository
            .load_final_verification("run", "2026-08-12T00:00:00Z")
            .expect("verification");
        repository
            .request_cancel("run", "2026-08-12T00:00:01Z")
            .expect("cancel");
        assert_eq!(
            repository.run_status("run").expect("status").as_str(),
            "cancelled"
        );
        let final_status: String = repository
            .connection()
            .expect("connection")
            .query_row(
                "SELECT status FROM plan_finalizations WHERE plan_run_id = 'run'",
                [],
                |row| row.get(0),
            )
            .expect("finalization");
        assert_eq!(final_status, "cancelled");
    }
}
