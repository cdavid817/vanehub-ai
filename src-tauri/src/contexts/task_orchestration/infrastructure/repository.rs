use crate::contexts::task_orchestration::application::{
    decide_serial_schedule, RunProjection, ScheduleDecision, ScheduleNode,
};
use crate::contexts::task_orchestration::application::{PlanApplicationError, PlanRepositoryPort};
use crate::contexts::task_orchestration::domain::{
    validate_plan_execution_policy, validate_plan_graph, CriterionEvidenceBinding,
    CriterionEvidenceKind, DependencyEdge, PlanDiscoveryMetadata, PlanDiscoveryStatus, PlanDraft,
    PlanExecutionPolicy, PlanRunStatus, PlanStatus, ResourceLimits, SubTaskRunStatus, SubTaskSpec,
    VerificationCommand,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, types::Type, OptionalExtension, Row, Transaction};
use uuid::Uuid;

const DEFAULT_PLAN_RUN_TIMEOUT_SECONDS: i64 = 2 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScheduleClaim {
    pub(crate) subtask_run_id: Option<String>,
    pub(crate) decision: ScheduleDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanRunPreparation {
    pub(crate) id: String,
    pub(crate) status: PlanRunStatus,
    pub(crate) project_path: String,
    pub(crate) base_ref: String,
}

pub(crate) struct PlanRunWorktree<'a> {
    pub(crate) project_path: &'a str,
    pub(crate) base_oid: &'a str,
    pub(crate) path: &'a str,
    pub(crate) name: &'a str,
    pub(crate) branch: &'a str,
}

#[derive(Clone)]
pub(crate) struct SqlitePlanRepository {
    database: NativeDatabase,
}

impl SqlitePlanRepository {
    pub(crate) fn record_generation_failure(
        &self,
        plan_id: Option<&str>,
        requested_version: u32,
        failure_class: &str,
        safe_action: &str,
    ) -> Result<(), PlanApplicationError> {
        self.connection()?
            .execute(
                r#"INSERT INTO plan_generation_failures (
                       id, plan_id, requested_version, failure_class, safe_action, created_at
                   ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
                params![
                    Uuid::new_v4().to_string(),
                    plan_id,
                    requested_version,
                    failure_class,
                    safe_action,
                    now_text()
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn save_draft(&self, draft: &PlanDraft) -> Result<(), PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let existing: Option<(u32, String)> = transaction
            .query_row(
                "SELECT current_version, status FROM plans WHERE id = ?1",
                [&draft.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        if let Some((current_version, _)) = existing {
            if draft.version < current_version {
                return Err(PlanApplicationError::Conflict);
            }
            let approved_at: Option<String> = transaction
                .query_row(
                    "SELECT approved_at FROM plan_versions WHERE id = ?1 AND plan_id = ?2",
                    params![draft.version_id, draft.id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(storage_error)?
                .flatten();
            if approved_at.is_some() {
                return Err(PlanApplicationError::Conflict);
            }
            delete_editable_version(&transaction, &draft.version_id)?;
            transaction
                .execute(
                    "UPDATE plans SET status = ?2, current_version = ?3, updated_at = ?4 WHERE id = ?1",
                    params![draft.id, PlanStatus::Draft.as_str(), draft.version, now_text()],
                )
                .map_err(storage_error)?;
        } else {
            transaction
                .execute(
                    "INSERT INTO plans (id, status, current_version, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![draft.id, PlanStatus::Draft.as_str(), draft.version, now_text()],
                )
                .map_err(storage_error)?;
        }

        insert_version(&transaction, draft)?;
        transaction.commit().map_err(storage_error)
    }

    pub(crate) fn find_latest_draft(
        &self,
        plan_id: &str,
    ) -> Result<Option<PlanDraft>, PlanApplicationError> {
        let connection = self.connection()?;
        let version = connection
            .query_row(
                r#"SELECT version.id, version.plan_id, version.version, version.goal,
                          version.project_path, version.base_ref, version.planner_profile_id,
                          version.discovery_status, version.discovery_limitations,
                          version.max_attempts_per_subtask, version.repair_eligible_classes,
                          version.final_validation_commands
                   FROM plan_versions AS version
                   JOIN plans AS plan ON plan.id = version.plan_id
                   WHERE plan.id = ?1 AND version.version = plan.current_version"#,
                [plan_id],
                read_draft_header,
            )
            .optional()
            .map_err(storage_error)?;
        version
            .map(|mut draft| {
                draft.subtasks = read_subtasks(&connection, &draft.version_id)?;
                draft.dependencies = read_dependencies(&connection, &draft.version_id)?;
                Ok(draft)
            })
            .transpose()
    }

    pub(crate) fn list_plan_versions(
        &self,
        plan_id: &str,
    ) -> Result<Vec<PlanDraft>, PlanApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                r#"SELECT id, plan_id, version, goal, project_path, base_ref, planner_profile_id,
                          discovery_status, discovery_limitations, max_attempts_per_subtask,
                          repair_eligible_classes, final_validation_commands
                   FROM plan_versions WHERE plan_id = ?1 ORDER BY version DESC"#,
            )
            .map_err(storage_error)?;
        let mut versions = statement
            .query_map([plan_id], read_draft_header)
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        for version in &mut versions {
            version.subtasks = read_subtasks(&connection, &version.version_id)?;
            version.dependencies = read_dependencies(&connection, &version.version_id)?;
        }
        Ok(versions)
    }

    pub(crate) fn delete_draft_plan(&self, plan_id: &str) -> Result<(), PlanApplicationError> {
        let changed = self
            .connection()?
            .execute(
                "DELETE FROM plans WHERE id = ?1 AND status = 'draft'",
                [plan_id],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn approve_latest(
        &self,
        plan_id: &str,
        now: &str,
    ) -> Result<String, PlanApplicationError> {
        self.approve_latest_for_session(plan_id, None, now)
    }

    pub(crate) fn approve_latest_for_session(
        &self,
        plan_id: &str,
        originating_session_id: Option<&str>,
        now: &str,
    ) -> Result<String, PlanApplicationError> {
        let draft = self
            .find_latest_draft(plan_id)?
            .ok_or(PlanApplicationError::NotFound)?;
        let validation = validate_plan_graph(&draft)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        validate_plan_execution_policy(&draft)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let changed = transaction
            .execute(
                "UPDATE plans SET status = ?2, updated_at = ?3 WHERE id = ?1 AND status = ?4 AND current_version = ?5",
                params![
                    plan_id,
                    PlanStatus::Draft
                        .approve()
                        .map_err(|error| PlanApplicationError::Validation(error.to_string()))?
                        .as_str(),
                    now,
                    PlanStatus::Draft.as_str(),
                    draft.version
                ],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        transaction
            .execute(
                "UPDATE plan_versions SET approved_at = ?2 WHERE id = ?1 AND approved_at IS NULL",
                params![draft.version_id, now],
            )
            .map_err(storage_error)?;
        let run_id = Uuid::new_v4().to_string();
        transaction
            .execute(
                r#"INSERT INTO plan_runs (
                       id, plan_id, plan_version_id, status, project_path, base_ref,
                       originating_session_id, simulated, created_at, updated_at
                   ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?8)"#,
                params![
                    run_id,
                    draft.id,
                    draft.version_id,
                    PlanRunStatus::Queued.as_str(),
                    draft.project_path,
                    draft.base_ref,
                    originating_session_id,
                    now
                ],
            )
            .map_err(storage_error)?;
        transaction
            .execute(
                r#"INSERT INTO plan_run_policies (
                       plan_run_id, planner_profile_id, discovery_status, discovery_limitations,
                       max_attempts_per_subtask, repair_eligible_classes,
                       final_validation_commands
                   ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
                params![
                    run_id,
                    draft.planner_profile_id,
                    draft.discovery.status.as_str(),
                    serde_json::to_string(&draft.discovery.limitations).map_err(storage_error)?,
                    draft.execution_policy.max_attempts_per_subtask,
                    serde_json::to_string(&draft.execution_policy.repair_eligible_classes)
                        .map_err(storage_error)?,
                    serde_json::to_string(&draft.execution_policy.final_validation_commands)
                        .map_err(storage_error)?,
                ],
            )
            .map_err(storage_error)?;
        for subtask in &draft.subtasks {
            transaction
                .execute(
                    r#"INSERT INTO plan_subtask_runs (
                           id, plan_run_id, subtask_id, status, topological_rank, ordinal,
                           created_at, updated_at
                       ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)"#,
                    params![
                        Uuid::new_v4().to_string(),
                        run_id,
                        subtask.id,
                        SubTaskRunStatus::Pending.as_str(),
                        validation.ranks[&subtask.id],
                        subtask.ordinal,
                        now
                    ],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(run_id)
    }

    pub(crate) fn claim_subtask(
        &self,
        subtask_run_id: &str,
        expected: SubTaskRunStatus,
        now: &str,
    ) -> Result<bool, PlanApplicationError> {
        let changed = self
            .connection()?
            .execute(
                "UPDATE plan_subtask_runs SET status = ?2, updated_at = ?3 WHERE id = ?1 AND status = ?4",
                params![
                    subtask_run_id,
                    SubTaskRunStatus::Dispatching.as_str(),
                    now,
                    expected.as_str()
                ],
            )
            .map_err(storage_error)?;
        Ok(changed == 1)
    }

    pub(crate) fn begin_preparation(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<PlanRunPreparation, PlanApplicationError> {
        PlanRunStatus::Queued
            .transition(PlanRunStatus::Preparing)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let changed = transaction
            .execute(
                "UPDATE plan_runs SET status = 'preparing', started_at = ?2, updated_at = ?2 WHERE id = ?1 AND status = 'queued' AND worktree_path IS NULL",
                params![run_id, now],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        let preparation = transaction
            .query_row(
                "SELECT id, status, project_path, base_ref FROM plan_runs WHERE id = ?1",
                [run_id],
                |row| {
                    let status: String = row.get(1)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        status,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .map_err(storage_error)?;
        transaction.commit().map_err(storage_error)?;
        Ok(PlanRunPreparation {
            id: preparation.0,
            status: PlanRunStatus::parse(&preparation.1).ok_or_else(|| {
                PlanApplicationError::Storage("unknown PlanRun status".to_string())
            })?,
            project_path: preparation.2,
            base_ref: preparation.3,
        })
    }

    pub(crate) fn attach_worktree_and_start(
        &self,
        run_id: &str,
        worktree: &PlanRunWorktree<'_>,
        now: &str,
    ) -> Result<(), PlanApplicationError> {
        PlanRunStatus::Preparing
            .transition(PlanRunStatus::Running)
            .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE plan_runs
                   SET status = 'running', project_path = ?2, base_oid = ?3,
                       worktree_path = ?4, worktree_name = ?5, worktree_branch = ?6,
                       updated_at = ?7
                   WHERE id = ?1 AND status = 'preparing' AND worktree_path IS NULL"#,
                params![
                    run_id,
                    worktree.project_path,
                    worktree.base_oid,
                    worktree.path,
                    worktree.name,
                    worktree.branch,
                    now
                ],
            )
            .map_err(storage_error)?;
        if changed != 1 {
            return Err(PlanApplicationError::Conflict);
        }
        Ok(())
    }

    pub(crate) fn schedule_next(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<ScheduleClaim, PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let run_header: Option<(String, Option<String>)> = transaction
            .query_row(
                "SELECT status, started_at FROM plan_runs WHERE id = ?1",
                [run_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        let (run_status, started_at) = run_header.ok_or(PlanApplicationError::NotFound)?;
        if PlanRunStatus::parse(&run_status) != Some(PlanRunStatus::Running) {
            return Err(PlanApplicationError::Conflict);
        }
        if plan_run_timed_out(started_at.as_deref(), now)? {
            let blocked_ids = read_schedule_nodes(&transaction, run_id)?
                .into_iter()
                .filter(|node| {
                    matches!(
                        node.status,
                        SubTaskRunStatus::Pending | SubTaskRunStatus::Ready
                    )
                })
                .map(|node| node.id)
                .collect::<Vec<_>>();
            transaction
                .execute(
                    "UPDATE plan_subtask_runs SET status = 'blocked', updated_at = ?2 WHERE plan_run_id = ?1 AND status IN ('pending', 'ready')",
                    params![run_id, now],
                )
                .map_err(storage_error)?;
            transaction
                .execute(
                    "UPDATE plan_runs SET status = 'failed', updated_at = ?2, completed_at = ?2 WHERE id = ?1 AND status = 'running'",
                    params![run_id, now],
                )
                .map_err(storage_error)?;
            transaction.commit().map_err(storage_error)?;
            return Ok(ScheduleClaim {
                subtask_run_id: None,
                decision: ScheduleDecision {
                    next_id: None,
                    blocked_ids,
                    projection: RunProjection::Failed,
                },
            });
        }
        let mut nodes = read_schedule_nodes(&transaction, run_id)?;
        let predecessors = read_schedule_predecessors(&transaction, run_id)?;
        for node in &mut nodes {
            node.predecessors = predecessors.get(&node.id).cloned().unwrap_or_default();
        }
        let decision = decide_serial_schedule(&nodes);
        for blocked_id in &decision.blocked_ids {
            transaction
                .execute(
                    r#"UPDATE plan_subtask_runs SET status = ?2, updated_at = ?3
                       WHERE id = ?1 AND plan_run_id = ?4 AND status IN ('pending', 'ready')"#,
                    params![blocked_id, SubTaskRunStatus::Blocked.as_str(), now, run_id],
                )
                .map_err(storage_error)?;
        }
        if let Some(next_id) = &decision.next_id {
            transaction
                .execute(
                    r#"UPDATE plan_subtask_runs SET status = ?2, updated_at = ?3
                       WHERE id = ?1 AND plan_run_id = ?4 AND status = 'pending'"#,
                    params![next_id, SubTaskRunStatus::Ready.as_str(), now, run_id],
                )
                .map_err(storage_error)?;
        } else if decision.projection != RunProjection::Continue {
            let next = match decision.projection {
                RunProjection::AwaitingAcceptance => PlanRunStatus::FinalVerifying,
                RunProjection::Failed => PlanRunStatus::ActionRequired,
                RunProjection::Continue => unreachable!(),
            };
            PlanRunStatus::Running
                .transition(next)
                .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
            transaction
                .execute(
                    "UPDATE plan_runs SET status = ?2, updated_at = ?3 WHERE id = ?1 AND status = 'running'",
                    params![run_id, next.as_str(), now],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)?;

        let subtask_run_id = match decision.next_id.as_deref() {
            Some(next_id) if self.claim_subtask(next_id, SubTaskRunStatus::Ready, now)? => {
                Some(next_id.to_string())
            }
            _ => None,
        };
        Ok(ScheduleClaim {
            subtask_run_id,
            decision,
        })
    }

    pub(crate) fn project_after_attempt(
        &self,
        run_id: &str,
        now: &str,
    ) -> Result<RunProjection, PlanApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        let status: String = transaction
            .query_row(
                "SELECT status FROM plan_runs WHERE id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(PlanApplicationError::NotFound)?;
        if PlanRunStatus::parse(&status) != Some(PlanRunStatus::Running) {
            return Ok(RunProjection::Continue);
        }
        let mut nodes = read_schedule_nodes(&transaction, run_id)?;
        let predecessors = read_schedule_predecessors(&transaction, run_id)?;
        for node in &mut nodes {
            node.predecessors = predecessors.get(&node.id).cloned().unwrap_or_default();
        }
        let decision = decide_serial_schedule(&nodes);
        for blocked_id in &decision.blocked_ids {
            transaction
                .execute(
                    r#"UPDATE plan_subtask_runs SET status = 'blocked', updated_at = ?2
                       WHERE id = ?1 AND plan_run_id = ?3 AND status IN ('pending', 'ready')"#,
                    params![blocked_id, now, run_id],
                )
                .map_err(storage_error)?;
        }
        let terminal = match decision.projection {
            RunProjection::AwaitingAcceptance => Some(PlanRunStatus::FinalVerifying),
            RunProjection::Failed => Some(PlanRunStatus::ActionRequired),
            RunProjection::Continue => None,
        };
        if let Some(terminal) = terminal {
            PlanRunStatus::Running
                .transition(terminal)
                .map_err(|error| PlanApplicationError::Validation(error.to_string()))?;
            transaction
                .execute(
                    r#"UPDATE plan_runs SET status = ?2, updated_at = ?3,
                              completed_at = CASE WHEN ?2 = 'failed' THEN ?3 ELSE completed_at END
                       WHERE id = ?1 AND status = 'running'"#,
                    params![run_id, terminal.as_str(), now],
                )
                .map_err(storage_error)?;
        }
        transaction.commit().map_err(storage_error)?;
        Ok(decision.projection)
    }

    pub(super) fn connection(&self) -> Result<PooledSqlite, PlanApplicationError> {
        self.database.connection().map_err(storage_error)
    }
}

impl PlanRepositoryPort for SqlitePlanRepository {
    fn record_generation_failure(
        &self,
        plan_id: Option<&str>,
        requested_version: u32,
        failure_class: &str,
        safe_action: &str,
    ) -> Result<(), PlanApplicationError> {
        SqlitePlanRepository::record_generation_failure(
            self,
            plan_id,
            requested_version,
            failure_class,
            safe_action,
        )
    }

    fn save_draft(&self, draft: &PlanDraft) -> Result<(), PlanApplicationError> {
        SqlitePlanRepository::save_draft(self, draft)
    }

    fn list_plan_versions(&self, plan_id: &str) -> Result<Vec<PlanDraft>, PlanApplicationError> {
        SqlitePlanRepository::list_plan_versions(self, plan_id)
    }

    fn delete_draft_plan(&self, plan_id: &str) -> Result<(), PlanApplicationError> {
        SqlitePlanRepository::delete_draft_plan(self, plan_id)
    }

    fn find_latest_draft(&self, plan_id: &str) -> Result<Option<PlanDraft>, PlanApplicationError> {
        SqlitePlanRepository::find_latest_draft(self, plan_id)
    }
}

fn read_schedule_nodes(
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<Vec<ScheduleNode>, PlanApplicationError> {
    let mut statement = transaction
        .prepare(
            r#"SELECT id, status, topological_rank, ordinal
               FROM plan_subtask_runs
               WHERE plan_run_id = ?1
               ORDER BY topological_rank, ordinal, id"#,
        )
        .map_err(storage_error)?;
    let nodes = statement
        .query_map([run_id], |row| {
            let status: String = row.get(1)?;
            let status = SubTaskRunStatus::parse(&status).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    format!("unknown SubTaskRun status: {status}").into(),
                )
            })?;
            Ok(ScheduleNode {
                id: row.get(0)?,
                status,
                topological_rank: row.get(2)?,
                ordinal: row.get(3)?,
                predecessors: Vec::new(),
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(nodes)
}

fn read_schedule_predecessors(
    transaction: &Transaction<'_>,
    run_id: &str,
) -> Result<std::collections::BTreeMap<String, Vec<String>>, PlanApplicationError> {
    let mut statement = transaction
        .prepare(
            r#"SELECT successor_run.id, predecessor_run.id
               FROM plan_runs AS run
               JOIN plan_subtask_dependencies AS dependency
                 ON dependency.plan_version_id = run.plan_version_id
               JOIN plan_subtask_runs AS successor_run
                 ON successor_run.plan_run_id = run.id
                AND successor_run.subtask_id = dependency.successor_id
               JOIN plan_subtask_runs AS predecessor_run
                 ON predecessor_run.plan_run_id = run.id
                AND predecessor_run.subtask_id = dependency.predecessor_id
               WHERE run.id = ?1
               ORDER BY predecessor_run.topological_rank,
                        predecessor_run.ordinal,
                        predecessor_run.id"#,
        )
        .map_err(storage_error)?;
    let pairs = statement
        .query_map([run_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    let mut result = std::collections::BTreeMap::<String, Vec<String>>::new();
    for (successor, predecessor) in pairs {
        result.entry(successor).or_default().push(predecessor);
    }
    Ok(result)
}

fn insert_version(
    transaction: &Transaction<'_>,
    draft: &PlanDraft,
) -> Result<(), PlanApplicationError> {
    transaction
        .execute(
            r#"INSERT INTO plan_versions (
                   id, plan_id, version, goal, project_path, base_ref, planner_profile_id,
                   discovery_status, discovery_limitations, max_attempts_per_subtask,
                   repair_eligible_classes, final_validation_commands, created_at
               ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"#,
            params![
                draft.version_id,
                draft.id,
                draft.version,
                draft.goal,
                draft.project_path,
                draft.base_ref,
                draft.planner_profile_id,
                draft.discovery.status.as_str(),
                serde_json::to_string(&draft.discovery.limitations).map_err(storage_error)?,
                draft.execution_policy.max_attempts_per_subtask,
                serde_json::to_string(&draft.execution_policy.repair_eligible_classes)
                    .map_err(storage_error)?,
                serde_json::to_string(&draft.execution_policy.final_validation_commands)
                    .map_err(storage_error)?,
                now_text()
            ],
        )
        .map_err(storage_error)?;
    for task in &draft.subtasks {
        let criteria = serde_json::to_string(&task.acceptance_criteria).map_err(storage_error)?;
        let commands = serde_json::to_string(&task.validation_commands).map_err(storage_error)?;
        let timeout_seconds = task
            .limits
            .timeout_seconds
            .map(i64::try_from)
            .transpose()
            .map_err(storage_error)?;
        transaction
            .execute(
                r#"INSERT INTO plan_subtasks (
                       id, plan_version_id, ordinal, title, description, acceptance_criteria,
                       assigned_role, token_budget, tool_call_limit, timeout_seconds,
                       validation_commands
                   ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"#,
                params![
                    task.id,
                    draft.version_id,
                    task.ordinal,
                    task.title,
                    task.description,
                    criteria,
                    task.assigned_role,
                    task.limits.token_budget,
                    task.limits.tool_call_limit,
                    timeout_seconds,
                    commands
                ],
            )
            .map_err(storage_error)?;
        for binding in &task.criterion_evidence {
            let kind = match binding.kind {
                CriterionEvidenceKind::Automated => "automated",
                CriterionEvidenceKind::Manual => "manual",
            };
            transaction
                .execute(
                    r#"INSERT INTO plan_criterion_evidence_bindings (
                           plan_version_id, subtask_id, criterion_index, evidence_kind, command_id
                       ) VALUES (?1, ?2, ?3, ?4, ?5)"#,
                    params![
                        draft.version_id,
                        task.id,
                        binding.criterion_index,
                        kind,
                        binding.command_id
                    ],
                )
                .map_err(storage_error)?;
        }
    }
    for edge in &draft.dependencies {
        transaction
            .execute(
                r#"INSERT INTO plan_subtask_dependencies (
                       plan_version_id, predecessor_id, successor_id
                   ) VALUES (?1, ?2, ?3)"#,
                params![draft.version_id, edge.predecessor_id, edge.successor_id],
            )
            .map_err(storage_error)?;
    }
    Ok(())
}

fn delete_editable_version(
    transaction: &Transaction<'_>,
    version_id: &str,
) -> Result<(), PlanApplicationError> {
    transaction
        .execute(
            "DELETE FROM plan_versions WHERE id = ?1 AND approved_at IS NULL",
            [version_id],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn read_draft_header(row: &Row<'_>) -> rusqlite::Result<PlanDraft> {
    let discovery_status = row.get::<_, String>(7)?;
    Ok(PlanDraft {
        version_id: row.get(0)?,
        id: row.get(1)?,
        version: row.get(2)?,
        goal: row.get(3)?,
        project_path: row.get(4)?,
        base_ref: row.get(5)?,
        planner_profile_id: row.get(6)?,
        discovery: PlanDiscoveryMetadata {
            status: PlanDiscoveryStatus::parse(&discovery_status).ok_or_else(|| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown Plan discovery status `{discovery_status}`"),
                    )),
                )
            })?,
            limitations: json_column(row, 8)?,
        },
        execution_policy: PlanExecutionPolicy {
            max_attempts_per_subtask: row.get(9)?,
            repair_eligible_classes: json_column(row, 10)?,
            final_validation_commands: json_column(row, 11)?,
        },
        subtasks: Vec::new(),
        dependencies: Vec::new(),
    })
}

fn read_subtasks(
    connection: &PooledSqlite,
    version_id: &str,
) -> Result<Vec<SubTaskSpec>, PlanApplicationError> {
    let mut statement = connection
        .prepare(
            r#"SELECT id, title, description, acceptance_criteria, ordinal, assigned_role,
                      token_budget, tool_call_limit, timeout_seconds, validation_commands
               FROM plan_subtasks WHERE plan_version_id = ?1 ORDER BY ordinal, id"#,
        )
        .map_err(storage_error)?;
    let mut subtasks = statement
        .query_map([version_id], |row| {
            let criteria: String = row.get(3)?;
            let commands: String = row.get(9)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                criteria,
                row.get::<_, u16>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<u32>>(6)?,
                row.get::<_, Option<u32>>(7)?,
                row.get::<_, Option<i64>>(8)?,
                commands,
            ))
        })
        .map_err(storage_error)?
        .map(|row| {
            let (
                id,
                title,
                description,
                criteria,
                ordinal,
                assigned_role,
                token_budget,
                tool_call_limit,
                timeout_seconds,
                commands,
            ) = row.map_err(storage_error)?;
            let timeout_seconds = timeout_seconds
                .map(u64::try_from)
                .transpose()
                .map_err(storage_error)?;
            Ok(SubTaskSpec {
                id,
                title,
                description,
                acceptance_criteria: serde_json::from_str(&criteria).map_err(storage_error)?,
                criterion_evidence: Vec::new(),
                ordinal,
                assigned_role,
                limits: ResourceLimits {
                    token_budget,
                    tool_call_limit,
                    timeout_seconds,
                },
                validation_commands: serde_json::from_str::<Vec<VerificationCommand>>(&commands)
                    .map_err(storage_error)?,
            })
        })
        .collect::<Result<Vec<_>, PlanApplicationError>>()?;
    drop(statement);
    for task in &mut subtasks {
        task.criterion_evidence = read_criterion_evidence(connection, version_id, &task.id)?;
    }
    Ok(subtasks)
}

fn read_criterion_evidence(
    connection: &PooledSqlite,
    version_id: &str,
    subtask_id: &str,
) -> Result<Vec<CriterionEvidenceBinding>, PlanApplicationError> {
    let mut statement = connection
        .prepare(
            r#"SELECT criterion_index, evidence_kind, command_id
               FROM plan_criterion_evidence_bindings
               WHERE plan_version_id = ?1 AND subtask_id = ?2
               ORDER BY criterion_index"#,
        )
        .map_err(storage_error)?;
    let bindings = statement
        .query_map(params![version_id, subtask_id], |row| {
            let value: String = row.get(1)?;
            let kind = match value.as_str() {
                "automated" => CriterionEvidenceKind::Automated,
                "manual" => CriterionEvidenceKind::Manual,
                _ => {
                    return Err(rusqlite::Error::FromSqlConversionFailure(
                        1,
                        Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("unknown criterion evidence kind `{value}`"),
                        )),
                    ));
                }
            };
            Ok(CriterionEvidenceBinding {
                criterion_index: row.get(0)?,
                kind,
                command_id: row.get(2)?,
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(bindings)
}

fn json_column<T: serde::de::DeserializeOwned>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    let value: String = row.get(index)?;
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

fn read_dependencies(
    connection: &PooledSqlite,
    version_id: &str,
) -> Result<Vec<DependencyEdge>, PlanApplicationError> {
    let mut statement = connection
        .prepare(
            r#"SELECT predecessor_id, successor_id FROM plan_subtask_dependencies
               WHERE plan_version_id = ?1 ORDER BY predecessor_id, successor_id"#,
        )
        .map_err(storage_error)?;
    let dependencies = statement
        .query_map([version_id], |row| {
            Ok(DependencyEdge {
                predecessor_id: row.get(0)?,
                successor_id: row.get(1)?,
            })
        })
        .map_err(storage_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error)?;
    Ok(dependencies)
}

fn now_text() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn plan_run_timed_out(started_at: Option<&str>, now: &str) -> Result<bool, PlanApplicationError> {
    let Some(started_at) = started_at else {
        return Ok(false);
    };
    let started = chrono::DateTime::parse_from_rfc3339(started_at).map_err(storage_error)?;
    let current = chrono::DateTime::parse_from_rfc3339(now).map_err(storage_error)?;
    Ok(current.signed_duration_since(started).num_seconds() >= DEFAULT_PLAN_RUN_TIMEOUT_SECONDS)
}

pub(super) fn storage_error(error: impl std::fmt::Display) -> PlanApplicationError {
    PlanApplicationError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::task_orchestration::domain::VerificationCommand;
    use crate::contexts::task_orchestration::infrastructure::{
        AttemptTerminalUpdate, RecoveryEvidence, RecoveryEvidenceGateway, RecoveryTerminal,
        VerificationEvidenceUpdate,
    };
    use crate::test_support::TempDirectory;

    fn repository() -> SqlitePlanRepository {
        let directory = TempDirectory::new("plan-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        SqlitePlanRepository::new(database)
    }

    struct FixedRecoveryEvidence(RecoveryEvidence);

    impl RecoveryEvidenceGateway for FixedRecoveryEvidence {
        fn inspect(
            &self,
            _session_id: Option<&str>,
            _execution_run_id: Option<&str>,
            _operation_id: Option<&str>,
        ) -> RecoveryEvidence {
            self.0
        }
    }

    struct CountingRecoveryEvidence {
        evidence: RecoveryEvidence,
        inspections: std::sync::atomic::AtomicUsize,
    }

    impl CountingRecoveryEvidence {
        fn new(evidence: RecoveryEvidence) -> Self {
            Self {
                evidence,
                inspections: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    impl RecoveryEvidenceGateway for CountingRecoveryEvidence {
        fn inspect(
            &self,
            _session_id: Option<&str>,
            _execution_run_id: Option<&str>,
            _operation_id: Option<&str>,
        ) -> RecoveryEvidence {
            self.inspections
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.evidence
        }
    }

    fn draft() -> PlanDraft {
        PlanDraft {
            id: "plan-1".into(),
            version_id: "plan-1-v1".into(),
            version: 1,
            goal: "Implement Plan execution".into(),
            project_path: "C:\\code\\app".into(),
            base_ref: "main".into(),
            planner_profile_id: None,
            discovery: PlanDiscoveryMetadata {
                status: PlanDiscoveryStatus::Complete,
                limitations: vec!["language server unavailable".into()],
            },
            execution_policy: PlanExecutionPolicy {
                final_validation_commands: vec![VerificationCommand {
                    id: "final-test".into(),
                    program: "cargo".into(),
                    args: vec!["test".into()],
                    working_directory: None,
                    timeout_seconds: 300,
                    required: true,
                }],
                ..PlanExecutionPolicy::default()
            },
            subtasks: vec![task("design", 0), task("implement", 1)],
            dependencies: vec![DependencyEdge {
                predecessor_id: "design".into(),
                successor_id: "implement".into(),
            }],
        }
    }

    fn task(id: &str, ordinal: u16) -> SubTaskSpec {
        SubTaskSpec {
            id: id.into(),
            title: id.into(),
            description: format!("Complete {id}"),
            acceptance_criteria: vec![format!("{id} accepted")],
            criterion_evidence: vec![CriterionEvidenceBinding {
                criterion_index: 0,
                kind: CriterionEvidenceKind::Automated,
                command_id: Some(format!("test-{id}")),
            }],
            ordinal,
            assigned_role: "worker".into(),
            limits: ResourceLimits {
                token_budget: Some(2_000),
                tool_call_limit: Some(20),
                timeout_seconds: Some(600),
            },
            validation_commands: vec![VerificationCommand {
                id: format!("test-{id}"),
                program: "cargo".into(),
                args: vec!["test".into()],
                working_directory: None,
                timeout_seconds: 300,
                required: true,
            }],
        }
    }

    fn attach_worktree(
        repository: &SqlitePlanRepository,
        run_id: &str,
        base_oid: &str,
        path: &str,
        name: &str,
        branch: &str,
        now: &str,
    ) {
        repository
            .attach_worktree_and_start(
                run_id,
                &PlanRunWorktree {
                    project_path: "C:\\code\\app",
                    base_oid,
                    path,
                    name,
                    branch,
                },
                now,
            )
            .expect("attach worktree");
    }

    #[test]
    fn draft_version_crud_lists_complete_graphs_and_rejects_approved_deletion() {
        let repository = repository();
        let first = draft();
        repository.save_draft(&first).expect("save first");
        let mut second = first.clone();
        second.version_id = "plan-1-v2".into();
        second.version = 2;
        second.goal = "Implement the revised Plan execution".into();
        repository.save_draft(&second).expect("save second");

        let versions = repository.list_plan_versions("plan-1").expect("versions");
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0], second);
        assert_eq!(versions[1], first);
        repository
            .delete_draft_plan("plan-1")
            .expect("delete draft");
        assert!(repository
            .list_plan_versions("plan-1")
            .expect("deleted versions")
            .is_empty());

        repository
            .save_draft(&draft())
            .expect("save approved candidate");
        repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        assert!(matches!(
            repository.delete_draft_plan("plan-1"),
            Err(PlanApplicationError::Conflict)
        ));
    }

    #[test]
    fn approval_retains_opaque_originating_session_association() {
        let repository = repository();
        repository.save_draft(&draft()).expect("save");
        let run_id = repository
            .approve_latest_for_session("plan-1", Some("origin-session-1"), "2026-08-08T00:00:00Z")
            .expect("approve");

        let reopened = SqlitePlanRepository::new(repository.database.clone());
        let linked = reopened
            .find_run_for_originating_session("origin-session-1")
            .expect("lookup")
            .expect("associated run");
        assert_eq!(linked.id, run_id);
        assert!(reopened
            .find_run_for_originating_session("unrelated-session")
            .expect("unrelated lookup")
            .is_none());
        assert_eq!(
            reopened
                .get_run_detail(&run_id)
                .expect("detail")
                .originating_session_id
                .as_deref(),
            Some("origin-session-1")
        );
    }

    #[test]
    fn associated_multitask_repair_flow_survives_pause_restart_and_final_verification() {
        let repository = repository();
        let mut plan = draft();
        plan.planner_profile_id = Some("profile-1".into());
        repository.save_draft(&plan).expect("save discovered plan");
        let run_id = repository
            .approve_latest_for_session("plan-1", Some("origin-session"), "2026-08-08T00:00:00Z")
            .expect("approve associated plan");
        repository
            .begin_preparation(&run_id, "2026-08-08T00:00:01Z")
            .expect("prepare");
        attach_worktree(
            &repository,
            &run_id,
            "0123456789abcdef",
            "C:\\code\\associated-plan-run",
            "associated-plan-run",
            "vanehub/associated-plan-run",
            "2026-08-08T00:00:02Z",
        );

        let first_task = repository
            .schedule_next(&run_id, "2026-08-08T00:00:03Z")
            .expect("schedule first task")
            .subtask_run_id
            .expect("first task claim");
        let first_attempt = repository
            .create_attempt(&first_task, "2026-08-08T00:00:04Z")
            .expect("first attempt");
        insert_attempt_session(&repository, "attempt-session-1");
        repository
            .start_attempt(
                &first_task,
                &first_attempt.id,
                "attempt-session-1",
                "profile-1",
                None,
                "2026-08-08T00:00:05Z",
            )
            .expect("start first attempt");
        repository
            .finish_attempt_generation(
                &first_task,
                &first_attempt.id,
                &AttemptTerminalUpdate {
                    result_summary: Some("initial implementation".into()),
                    changed_files: vec!["src/first.rs".into()],
                    token_usage: 10,
                    tool_call_count: 1,
                    error_class: None,
                },
                true,
                "2026-08-08T00:00:06Z",
            )
            .expect("enter verification");
        assert_eq!(
            repository.run_status(&run_id).expect("status"),
            PlanRunStatus::Verifying
        );
        let failed_verification = repository
            .load_attempt_verification(&first_task)
            .expect("load failed verification");
        repository
            .finish_attempt_verification(
                &failed_verification,
                &[VerificationEvidenceUpdate {
                    command_id: "test-design".into(),
                    status: "failed".into(),
                    exit_code: Some(1),
                    duration_ms: Some(5),
                    output_summary: Some("bounded failure summary".into()),
                }],
                false,
                "required check failed",
                "2026-08-08T00:00:07Z",
            )
            .expect("record verification failure");
        assert!(repository
            .auto_retry_failed_attempt(&run_id, &first_attempt.id, "2026-08-08T00:00:08Z")
            .expect("schedule repair"));

        let repair_task = repository
            .schedule_next(&run_id, "2026-08-08T00:00:09Z")
            .expect("schedule repair")
            .subtask_run_id
            .expect("repair claim");
        assert_eq!(repair_task, first_task);
        let repair_attempt = repository
            .create_attempt(&repair_task, "2026-08-08T00:00:10Z")
            .expect("repair attempt");
        insert_attempt_session(&repository, "attempt-session-2");
        repository
            .start_attempt(
                &repair_task,
                &repair_attempt.id,
                "attempt-session-2",
                "profile-1",
                None,
                "2026-08-08T00:00:11Z",
            )
            .expect("start repair");
        assert_eq!(
            repository.run_status(&run_id).expect("status"),
            PlanRunStatus::Repairing
        );
        repository
            .finish_attempt_generation(
                &repair_task,
                &repair_attempt.id,
                &AttemptTerminalUpdate {
                    result_summary: Some("repaired implementation".into()),
                    changed_files: vec!["src/first.rs".into()],
                    token_usage: 8,
                    tool_call_count: 1,
                    error_class: None,
                },
                true,
                "2026-08-08T00:00:12Z",
            )
            .expect("verify repair");
        repository
            .request_pause(&run_id, "2026-08-08T00:00:13Z")
            .expect("pause at verification boundary");
        let repair_verification = repository
            .load_attempt_verification(&repair_task)
            .expect("load repair verification");
        repository
            .finish_attempt_verification(
                &repair_verification,
                &[],
                true,
                "repair verified",
                "2026-08-08T00:00:14Z",
            )
            .expect("finish repair verification");
        repository
            .settle_control_boundary(&run_id, "2026-08-08T00:00:15Z")
            .expect("settle pause");

        let reopened = SqlitePlanRepository::new(repository.database.clone());
        assert_eq!(
            reopened.run_status(&run_id).expect("reopened status"),
            PlanRunStatus::Paused
        );
        assert_eq!(
            reopened
                .find_run_for_originating_session("origin-session")
                .expect("association lookup")
                .expect("associated run")
                .id,
            run_id
        );
        reopened
            .resume_run(&run_id, "2026-08-08T00:00:16Z")
            .expect("resume after restart");

        let second_task = reopened
            .schedule_next(&run_id, "2026-08-08T00:00:17Z")
            .expect("schedule second task")
            .subtask_run_id
            .expect("second task claim");
        let second_attempt = reopened
            .create_attempt(&second_task, "2026-08-08T00:00:18Z")
            .expect("second attempt");
        insert_attempt_session(&reopened, "attempt-session-3");
        reopened
            .start_attempt(
                &second_task,
                &second_attempt.id,
                "attempt-session-3",
                "profile-1",
                None,
                "2026-08-08T00:00:19Z",
            )
            .expect("start second task");
        reopened
            .finish_attempt_generation(
                &second_task,
                &second_attempt.id,
                &AttemptTerminalUpdate {
                    result_summary: Some("dependent implementation".into()),
                    changed_files: vec!["src/second.rs".into()],
                    token_usage: 6,
                    tool_call_count: 1,
                    error_class: None,
                },
                true,
                "2026-08-08T00:00:20Z",
            )
            .expect("verify second task");
        let second_verification = reopened
            .load_attempt_verification(&second_task)
            .expect("load second verification");
        reopened
            .finish_attempt_verification(
                &second_verification,
                &[],
                true,
                "second task verified",
                "2026-08-08T00:00:21Z",
            )
            .expect("finish second verification");
        assert_eq!(
            reopened
                .project_after_attempt(&run_id, "2026-08-08T00:00:22Z")
                .expect("project final verification"),
            RunProjection::AwaitingAcceptance
        );
        let final_verification = reopened
            .load_final_verification(&run_id, "2026-08-08T00:00:23Z")
            .expect("load final verification");
        reopened
            .finish_final_verification(&final_verification, &[], true, "2026-08-08T00:00:24Z")
            .expect("finish final verification");
        assert_eq!(
            reopened.run_status(&run_id).expect("acceptance status"),
            PlanRunStatus::AwaitingAcceptance
        );
        reopened
            .accept_run(&run_id, "2026-08-08T00:00:25Z")
            .expect("accept run");
        assert_eq!(
            reopened.run_status(&run_id).expect("completed status"),
            PlanRunStatus::Completed
        );
    }

    fn insert_attempt_session(repository: &SqlitePlanRepository, session_id: &str) {
        repository
            .connection()
            .expect("connection")
            .execute(
                r#"INSERT INTO sessions (
                       id, title, agent_id, interaction_mode, lifecycle_state,
                       pinned, archived, created_at, updated_at
                   ) VALUES (?1, ?2, 'onepiece', 'api', 'running', 0, 0, ?3, ?3)"#,
                params![
                    session_id,
                    format!("Attempt {session_id}"),
                    "2026-08-08T00:00:00Z"
                ],
            )
            .expect("session");
    }

    fn create_inflight_recovery_attempt(
        repository: &SqlitePlanRepository,
    ) -> (String, String, String) {
        let mut plan = draft();
        plan.planner_profile_id = Some("profile-1".into());
        repository.save_draft(&plan).expect("save");
        let run_id = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        repository
            .begin_preparation(&run_id, "2026-08-08T00:00:01Z")
            .expect("prepare");
        attach_worktree(
            repository,
            &run_id,
            "0123456789abcdef",
            "C:\\code\\retained-plan-worktree",
            "plan-run",
            "vanehub/plan-run",
            "2026-08-08T00:00:02Z",
        );
        let task_id = repository
            .schedule_next(&run_id, "2026-08-08T00:00:03Z")
            .expect("schedule")
            .subtask_run_id
            .expect("claim");
        let attempt = repository
            .create_attempt(&task_id, "2026-08-08T00:00:04Z")
            .expect("attempt");
        insert_attempt_session(repository, "session-recovery-evidence");
        repository
            .start_attempt(
                &task_id,
                &attempt.id,
                "session-recovery-evidence",
                "profile-1",
                Some("operation-recovery-evidence"),
                "2026-08-08T00:00:05Z",
            )
            .expect("start");
        (run_id, task_id, attempt.id)
    }

    #[test]
    fn round_trips_normalized_draft_and_dependencies() {
        let repository = repository();
        let draft = draft();
        repository.save_draft(&draft).expect("save");
        assert_eq!(
            repository
                .find_latest_draft("plan-1")
                .expect("find")
                .expect("draft"),
            draft
        );
    }

    #[test]
    fn approval_snapshots_tasks_atomically_and_only_once() {
        let repository = repository();
        repository.save_draft(&draft()).expect("save");
        let run_id = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        assert!(!run_id.is_empty());
        let connection = repository.connection().expect("connection");
        let policy: (Option<String>, String, String, u16, String, String) = connection
            .query_row(
                r#"SELECT planner_profile_id, discovery_status, discovery_limitations,
                          max_attempts_per_subtask, repair_eligible_classes,
                          final_validation_commands
                   FROM plan_run_policies WHERE plan_run_id = ?1"#,
                [&run_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("policy snapshot");
        assert_eq!(policy.0, None);
        assert_eq!(policy.1, "complete");
        assert_eq!(policy.2, r#"["language server unavailable"]"#);
        assert_eq!(policy.3, 3);
        assert_eq!(policy.4, r#"["verification_failed"]"#);
        assert!(policy.5.contains("final-test"));
        let bindings: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM plan_criterion_evidence_bindings WHERE plan_version_id = 'plan-1-v1'",
                [],
                |row| row.get(0),
            )
            .expect("evidence bindings");
        assert_eq!(bindings, 2);
        drop(connection);
        assert!(matches!(
            repository.approve_latest("plan-1", "2026-08-08T00:00:01Z"),
            Err(PlanApplicationError::Conflict)
        ));
    }

    #[test]
    fn compare_and_set_claim_allows_one_dispatch() {
        let repository = repository();
        repository.save_draft(&draft()).expect("save");
        repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        let connection = repository.connection().expect("connection");
        let subtask_run_id: String = connection
            .query_row(
                "SELECT id FROM plan_subtask_runs ORDER BY ordinal LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("task run");
        connection
            .execute(
                "UPDATE plan_subtask_runs SET status = 'ready' WHERE id = ?1",
                [&subtask_run_id],
            )
            .expect("ready");
        drop(connection);

        assert!(repository
            .claim_subtask(
                &subtask_run_id,
                SubTaskRunStatus::Ready,
                "2026-08-08T00:00:01Z"
            )
            .expect("first claim"));
        assert!(!repository
            .claim_subtask(
                &subtask_run_id,
                SubTaskRunStatus::Ready,
                "2026-08-08T00:00:02Z"
            )
            .expect("second claim"));
    }

    #[test]
    fn persisted_scheduler_releases_only_verified_predecessors() {
        let repository = repository();
        repository.save_draft(&draft()).expect("save");
        let run_id = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        let connection = repository.connection().expect("connection");
        connection
            .execute(
                "UPDATE plan_runs SET status = 'running' WHERE id = ?1",
                [&run_id],
            )
            .expect("run");
        drop(connection);

        let first = repository
            .schedule_next(&run_id, "2026-08-08T00:00:01Z")
            .expect("first tick")
            .subtask_run_id
            .expect("first claim");
        assert!(repository
            .schedule_next(&run_id, "2026-08-08T00:00:02Z")
            .expect("overlapping tick")
            .subtask_run_id
            .is_none());
        let connection = repository.connection().expect("connection");
        connection
            .execute(
                "UPDATE plan_subtask_runs SET status = 'succeeded' WHERE id = ?1",
                [&first],
            )
            .expect("verify");
        drop(connection);

        let second = repository
            .schedule_next(&run_id, "2026-08-08T00:00:03Z")
            .expect("second tick")
            .subtask_run_id
            .expect("second claim");
        assert_ne!(first, second);
    }

    #[test]
    fn verified_tasks_project_to_integrated_final_verification() {
        let repository = repository();
        repository.save_draft(&draft()).expect("save");
        let run_id = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        let connection = repository.connection().expect("connection");
        connection
            .execute(
                "UPDATE plan_runs SET status = 'running' WHERE id = ?1",
                [&run_id],
            )
            .expect("run");
        connection
            .execute(
                "UPDATE plan_subtask_runs SET status = 'succeeded' WHERE plan_run_id = ?1",
                [&run_id],
            )
            .expect("verified tasks");
        drop(connection);

        assert_eq!(
            repository
                .project_after_attempt(&run_id, "2026-08-08T00:00:01Z")
                .expect("projection"),
            RunProjection::AwaitingAcceptance
        );
        let status: String = repository
            .connection()
            .expect("connection")
            .query_row(
                "SELECT status FROM plan_runs WHERE id = ?1",
                [&run_id],
                |row| row.get(0),
            )
            .expect("status");
        assert_eq!(status, "final_verifying");
    }

    #[test]
    fn preparation_persists_base_and_canonical_worktree_identity() {
        let repository = repository();
        repository.save_draft(&draft()).expect("save");
        let run_id = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        let preparation = repository
            .begin_preparation(&run_id, "2026-08-08T00:00:01Z")
            .expect("prepare");
        assert_eq!(preparation.status, PlanRunStatus::Preparing);
        attach_worktree(
            &repository,
            &run_id,
            "0123456789abcdef",
            "C:\\code\\app-plan-run",
            "plan-run",
            "vanehub/plan-run",
            "2026-08-08T00:00:02Z",
        );
        let connection = repository.connection().expect("connection");
        let persisted: (String, String, String, String, String) = connection
            .query_row(
                "SELECT status, base_oid, worktree_path, worktree_name, worktree_branch FROM plan_runs WHERE id = ?1",
                [&run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .expect("run");
        assert_eq!(
            persisted,
            (
                "running".into(),
                "0123456789abcdef".into(),
                "C:\\code\\app-plan-run".into(),
                "plan-run".into(),
                "vanehub/plan-run".into(),
            )
        );
    }

    #[test]
    fn retry_creates_distinct_attempts_and_retains_prior_session_evidence() {
        let repository = repository();
        let mut plan = draft();
        plan.planner_profile_id = Some("profile-1".into());
        repository.save_draft(&plan).expect("save");
        let run_id = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        repository
            .begin_preparation(&run_id, "2026-08-08T00:00:01Z")
            .expect("prepare");
        attach_worktree(
            &repository,
            &run_id,
            "0123456789abcdef",
            "C:\\code\\app-plan-run",
            "plan-run",
            "vanehub/plan-run",
            "2026-08-08T00:00:02Z",
        );
        let subtask_run_id = repository
            .schedule_next(&run_id, "2026-08-08T00:00:03Z")
            .expect("schedule")
            .subtask_run_id
            .expect("claim");
        let dispatch = repository
            .load_attempt_dispatch(&subtask_run_id)
            .expect("dispatch");
        assert_eq!(dispatch.profile_id, "profile-1");
        assert_eq!(dispatch.worktree_path, "C:\\code\\app-plan-run");

        let first = repository
            .create_attempt(&subtask_run_id, "2026-08-08T00:00:04Z")
            .expect("first attempt");
        insert_attempt_session(&repository, "session-1");
        repository
            .start_attempt(
                &subtask_run_id,
                &first.id,
                "session-1",
                "profile-1",
                Some("operation-1"),
                "2026-08-08T00:00:05Z",
            )
            .expect("start first");
        repository
            .finish_attempt_generation(
                &subtask_run_id,
                &first.id,
                &AttemptTerminalUpdate {
                    result_summary: Some("first partial result".into()),
                    changed_files: vec!["src/first.rs".into()],
                    token_usage: 120,
                    tool_call_count: 4,
                    error_class: Some("provider_failed".into()),
                },
                false,
                "2026-08-08T00:00:06Z",
            )
            .expect("finish first");

        repository
            .connection()
            .expect("connection")
            .execute(
                "UPDATE plan_subtask_runs SET status = 'ready' WHERE id = ?1 AND status = 'failed'",
                [&subtask_run_id],
            )
            .expect("retry intent");
        assert!(repository
            .claim_subtask(
                &subtask_run_id,
                SubTaskRunStatus::Ready,
                "2026-08-08T00:00:07Z"
            )
            .expect("retry claim"));
        let second = repository
            .create_attempt(&subtask_run_id, "2026-08-08T00:00:08Z")
            .expect("second attempt");
        insert_attempt_session(&repository, "session-2");
        repository
            .start_attempt(
                &subtask_run_id,
                &second.id,
                "session-2",
                "profile-1",
                Some("operation-2"),
                "2026-08-08T00:00:09Z",
            )
            .expect("start second");
        repository
            .correlate_attempt_execution(
                &second.id,
                Some("operation-2-correlated"),
                Some("6ba7b810-9dad-41d1-80b4-00c04fd430c8"),
            )
            .expect("correlate second");
        repository
            .finish_attempt_generation(
                &subtask_run_id,
                &second.id,
                &AttemptTerminalUpdate {
                    result_summary: Some("retry completed".into()),
                    changed_files: vec!["src/second.rs".into()],
                    token_usage: 80,
                    tool_call_count: 2,
                    error_class: None,
                },
                true,
                "2026-08-08T00:00:10Z",
            )
            .expect("finish second");
        let verification = repository
            .load_attempt_verification(&subtask_run_id)
            .expect("verification dispatch");
        assert_eq!(verification.attempt_id, second.id);
        assert_eq!(verification.commands.len(), 1);
        repository
            .finish_attempt_verification(
                &verification,
                &[VerificationEvidenceUpdate {
                    command_id: "test-design".into(),
                    status: "passed".into(),
                    exit_code: Some(0),
                    duration_ms: Some(25),
                    output_summary: Some("stdout:\nall tests passed".into()),
                }],
                true,
                "1/1 validation commands passed; required checks passed.",
                "2026-08-08T00:00:11Z",
            )
            .expect("verification complete");
        assert!(repository
            .schedule_next(&run_id, "2026-08-08T00:00:12Z")
            .expect("dependent released")
            .subtask_run_id
            .is_some());

        assert_eq!(first.sequence, 1);
        assert_eq!(second.sequence, 2);
        assert_ne!(first.id, second.id);
        let connection = repository.connection().expect("connection");
        let mut statement = connection
            .prepare(
                r#"SELECT sequence, status, session_id, operation_id, execution_run_id,
                          token_usage, tool_call_count, error_class
                   FROM plan_subtask_attempts
                   WHERE subtask_run_id = ?1 ORDER BY sequence"#,
            )
            .expect("statement");
        let attempts = statement
            .query_map([&subtask_run_id], |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, u32>(5)?,
                    row.get::<_, u32>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .expect("attempt query")
            .collect::<Result<Vec<_>, _>>()
            .expect("attempt rows");
        assert_eq!(
            attempts,
            vec![
                (
                    1,
                    "failed".into(),
                    "session-1".into(),
                    "operation-1".into(),
                    None,
                    120,
                    4,
                    Some("provider_failed".into()),
                ),
                (
                    2,
                    "succeeded".into(),
                    "session-2".into(),
                    "operation-2-correlated".into(),
                    Some("6ba7b810-9dad-41d1-80b4-00c04fd430c8".into()),
                    80,
                    2,
                    None,
                ),
            ]
        );
        let persisted_evidence: (String, Option<i32>, String) = connection
            .query_row(
                r#"SELECT status, exit_code, output_summary
                   FROM plan_verification_evidence WHERE attempt_id = ?1"#,
                [&second.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("evidence");
        assert_eq!(
            persisted_evidence,
            ("passed".into(), Some(0), "stdout:\nall tests passed".into())
        );
        assert_eq!(
            repository
                .get_attempt_evidence(&second.id)
                .expect("attempt evidence")
                .len(),
            1
        );
        drop(statement);
        drop(connection);
        let detail = repository.get_run_detail(&run_id).expect("run detail");
        assert_eq!(detail.summary.total_tasks, 2);
        assert_eq!(detail.tasks[0].attempts.len(), 2);
        let projection = serde_json::to_value(detail).expect("serialize bounded projection");
        assert!(projection["tasks"][0]["attempts"][1]
            .get("evidence")
            .is_none());
    }

    #[test]
    fn cancelled_verification_retains_evidence_and_aligns_terminal_states() {
        let repository = repository();
        repository.save_draft(&draft()).expect("save");
        let run_id = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        repository
            .begin_preparation(&run_id, "2026-08-08T00:00:01Z")
            .expect("prepare");
        attach_worktree(
            &repository,
            &run_id,
            "0123456789abcdef",
            "C:\\code\\app-plan-run",
            "plan-run",
            "vanehub/plan-run",
            "2026-08-08T00:00:02Z",
        );
        let subtask_run_id = repository
            .schedule_next(&run_id, "2026-08-08T00:00:03Z")
            .expect("schedule")
            .subtask_run_id
            .expect("claim");
        let attempt = repository
            .create_attempt(&subtask_run_id, "2026-08-08T00:00:04Z")
            .expect("attempt");
        insert_attempt_session(&repository, "session-cancel");
        repository
            .start_attempt(
                &subtask_run_id,
                &attempt.id,
                "session-cancel",
                "profile-1",
                None,
                "2026-08-08T00:00:05Z",
            )
            .expect("start");
        repository
            .finish_attempt_generation(
                &subtask_run_id,
                &attempt.id,
                &AttemptTerminalUpdate {
                    result_summary: Some("generated".into()),
                    changed_files: vec!["src/lib.rs".into()],
                    token_usage: 20,
                    tool_call_count: 2,
                    error_class: None,
                },
                true,
                "2026-08-08T00:00:06Z",
            )
            .expect("generation");
        let verification = repository
            .load_attempt_verification(&subtask_run_id)
            .expect("verification");
        repository
            .request_cancel(&run_id, "2026-08-08T00:00:07Z")
            .expect("cancel intent");
        repository
            .cancel_attempt_verification(
                &verification,
                &[VerificationEvidenceUpdate {
                    command_id: "test-design".into(),
                    status: "cancelled".into(),
                    exit_code: None,
                    duration_ms: Some(10),
                    output_summary: Some("bounded cancellation evidence".into()),
                }],
                "verification cancelled",
                "2026-08-08T00:00:08Z",
            )
            .expect("cancel verification");
        repository
            .settle_control_boundary(&run_id, "2026-08-08T00:00:08Z")
            .expect("settle");

        let connection = repository.connection().expect("connection");
        let states: (String, String, String) = connection
            .query_row(
                r#"SELECT run.status, task.status, attempt.status
                   FROM plan_runs run
                   JOIN plan_subtask_runs task ON task.plan_run_id = run.id
                   JOIN plan_subtask_attempts attempt ON attempt.subtask_run_id = task.id
                   WHERE run.id = ?1 AND task.id = ?2 AND attempt.id = ?3"#,
                params![run_id, subtask_run_id, attempt.id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("states");
        assert_eq!(
            states,
            ("cancelled".into(), "cancelled".into(), "cancelled".into())
        );
        let output: String = connection
            .query_row(
                "SELECT output_summary FROM plan_verification_evidence WHERE attempt_id = ?1",
                [&attempt.id],
                |row| row.get(0),
            )
            .expect("evidence");
        assert_eq!(output, "bounded cancellation evidence");
    }

    #[test]
    fn run_summary_queries_page_by_stable_cursor() {
        let repository = repository();
        for index in 0..27 {
            let mut plan = draft();
            plan.id = format!("plan-{index:02}");
            plan.version_id = format!("plan-{index:02}-v1");
            repository.save_draft(&plan).expect("save");
            repository
                .approve_latest(&plan.id, &format!("2026-08-08T00:00:{index:02}Z"))
                .expect("approve");
        }
        let first = repository.list_run_summaries(None).expect("first page");
        assert_eq!(first.items.len(), 25);
        let cursor = first.next_cursor.expect("cursor");
        let second = repository
            .list_run_summaries(Some(&cursor))
            .expect("second page");
        assert_eq!(second.items.len(), 2);
        assert_eq!(second.next_cursor, None);
        assert!(matches!(
            repository.list_run_summaries(Some("missing")),
            Err(PlanApplicationError::Validation(_))
        ));
    }

    #[test]
    fn durable_pause_blocks_claims_until_resume_and_rejects_invalid_repause() {
        let repository = repository();
        repository.save_draft(&draft()).expect("save");
        let run_id = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        repository
            .begin_preparation(&run_id, "2026-08-08T00:00:01Z")
            .expect("prepare");
        attach_worktree(
            &repository,
            &run_id,
            "0123456789abcdef",
            "C:\\code\\app-plan-run",
            "plan-run",
            "vanehub/plan-run",
            "2026-08-08T00:00:02Z",
        );

        let paused = repository
            .request_pause(&run_id, "2026-08-08T00:00:03Z")
            .expect("pause");
        assert_eq!(paused.run_status, PlanRunStatus::Paused);
        assert!(matches!(
            repository.schedule_next(&run_id, "2026-08-08T00:00:04Z"),
            Err(PlanApplicationError::Conflict)
        ));
        assert!(matches!(
            repository.request_pause(&run_id, "2026-08-08T00:00:05Z"),
            Err(PlanApplicationError::Conflict)
        ));
        let resumed = repository
            .resume_run(&run_id, "2026-08-08T00:00:06Z")
            .expect("resume");
        assert_eq!(resumed.run_status, PlanRunStatus::Running);
        assert!(repository
            .schedule_next(&run_id, "2026-08-08T00:00:07Z")
            .expect("schedule")
            .subtask_run_id
            .is_some());
        let controls: Vec<(String, String)> = {
            let connection = repository.connection().expect("connection");
            let mut statement = connection
                .prepare(
                    "SELECT kind, status FROM plan_control_requests WHERE plan_run_id = ?1 ORDER BY requested_at",
                )
                .expect("statement");
            statement
                .query_map([&run_id], |row| Ok((row.get(0)?, row.get(1)?)))
                .expect("query")
                .collect::<Result<Vec<_>, _>>()
                .expect("controls")
        };
        assert_eq!(
            controls,
            [
                ("pause".into(), "completed".into()),
                ("resume".into(), "completed".into())
            ]
        );
    }

    #[test]
    fn cancellation_and_final_acceptance_are_explicit_terminal_transitions() {
        let repository = repository();
        repository.save_draft(&draft()).expect("save");
        let cancelled_run = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        repository
            .begin_preparation(&cancelled_run, "2026-08-08T00:00:01Z")
            .expect("prepare");
        attach_worktree(
            &repository,
            &cancelled_run,
            "0123456789abcdef",
            "C:\\code\\app-plan-run",
            "plan-run",
            "vanehub/plan-run",
            "2026-08-08T00:00:02Z",
        );
        let cancelled = repository
            .request_cancel(&cancelled_run, "2026-08-08T00:00:03Z")
            .expect("cancel");
        assert_eq!(cancelled.run_status, PlanRunStatus::Cancelled);
        assert_eq!(
            repository
                .get_run_detail(&cancelled_run)
                .expect("cancelled detail")
                .worktree_path
                .as_deref(),
            Some("C:\\code\\app-plan-run")
        );
        let connection = repository.connection().expect("connection");
        let unfinished: u32 = connection
            .query_row(
                "SELECT COUNT(*) FROM plan_subtask_runs WHERE plan_run_id = ?1 AND status <> 'cancelled'",
                [&cancelled_run],
                |row| row.get(0),
            )
            .expect("tasks");
        assert_eq!(unfinished, 0);
        drop(connection);

        let mut second = draft();
        second.id = "plan-2".into();
        second.version_id = "plan-2-v1".into();
        repository.save_draft(&second).expect("save second");
        let accepted_run = repository
            .approve_latest("plan-2", "2026-08-08T00:00:04Z")
            .expect("approve second");
        repository
            .begin_preparation(&accepted_run, "2026-08-08T00:00:05Z")
            .expect("prepare second");
        attach_worktree(
            &repository,
            &accepted_run,
            "fedcba9876543210",
            "C:\\code\\completed-plan-worktree",
            "plan-run-completed",
            "vanehub/plan-run-completed",
            "2026-08-08T00:00:06Z",
        );
        let connection = repository.connection().expect("connection");
        connection
            .execute(
                "UPDATE plan_runs SET status = 'awaiting_acceptance' WHERE id = ?1",
                [&accepted_run],
            )
            .expect("awaiting acceptance");
        drop(connection);
        repository
            .accept_run(&accepted_run, "2026-08-08T00:00:07Z")
            .expect("accept");
        let connection = repository.connection().expect("connection");
        let status: String = connection
            .query_row(
                "SELECT status FROM plan_runs WHERE id = ?1",
                [&accepted_run],
                |row| row.get(0),
            )
            .expect("status");
        assert_eq!(status, "completed");
        drop(connection);
        assert_eq!(
            repository
                .get_run_detail(&accepted_run)
                .expect("completed detail")
                .worktree_path
                .as_deref(),
            Some("C:\\code\\completed-plan-worktree")
        );
        assert!(matches!(
            repository.accept_run(&accepted_run, "2026-08-08T00:00:08Z"),
            Err(PlanApplicationError::Conflict)
        ));
    }

    #[test]
    fn startup_recovery_retains_worktree_and_requires_explicit_redispatch() {
        let repository = repository();
        let mut plan = draft();
        plan.planner_profile_id = Some("profile-1".into());
        repository.save_draft(&plan).expect("save");
        let run_id = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        repository
            .begin_preparation(&run_id, "2026-08-08T00:00:01Z")
            .expect("prepare");
        attach_worktree(
            &repository,
            &run_id,
            "0123456789abcdef",
            "C:\\code\\retained-plan-worktree",
            "plan-run",
            "vanehub/plan-run",
            "2026-08-08T00:00:02Z",
        );
        let subtask_run_id = repository
            .schedule_next(&run_id, "2026-08-08T00:00:03Z")
            .expect("schedule")
            .subtask_run_id
            .expect("claim");
        let first = repository
            .create_attempt(&subtask_run_id, "2026-08-08T00:00:04Z")
            .expect("attempt");
        insert_attempt_session(&repository, "session-before-restart");
        repository
            .start_attempt(
                &subtask_run_id,
                &first.id,
                "session-before-restart",
                "profile-1",
                Some("operation-before-restart"),
                "2026-08-08T00:00:05Z",
            )
            .expect("start");

        assert_eq!(
            repository
                .recover_ambiguous_inflight(
                    &FixedRecoveryEvidence(RecoveryEvidence::default()),
                    "2026-08-08T00:01:00Z",
                )
                .expect("startup recovery"),
            std::slice::from_ref(&run_id)
        );
        let detail = repository.get_run_detail(&run_id).expect("detail");
        assert_eq!(detail.summary.status, "recovery_required");
        assert_eq!(
            detail.worktree_path.as_deref(),
            Some("C:\\code\\retained-plan-worktree")
        );
        assert_eq!(detail.tasks[0].status, "interrupted");
        assert_eq!(detail.tasks[0].attempts[0].status, "interrupted");
        assert_eq!(
            detail.tasks[0].attempts[0].session_id.as_deref(),
            Some("session-before-restart")
        );
        assert!(matches!(
            repository.schedule_next(&run_id, "2026-08-08T00:01:01Z"),
            Err(PlanApplicationError::Conflict)
        ));

        repository
            .recover_run(&run_id, "2026-08-08T00:01:02Z")
            .expect("explicit recovery");
        repository
            .resume_run(&run_id, "2026-08-08T00:01:03Z")
            .expect("resume");
        let retry_task = repository
            .schedule_next(&run_id, "2026-08-08T00:01:04Z")
            .expect("retry claim")
            .subtask_run_id
            .expect("retry task");
        assert_eq!(retry_task, subtask_run_id);
        let second = repository
            .create_attempt(&retry_task, "2026-08-08T00:01:05Z")
            .expect("new attempt");
        assert_eq!(second.sequence, 2);
        assert_ne!(second.id, first.id);
    }

    #[test]
    fn startup_recovery_reconciles_conclusive_failed_and_cancelled_evidence() {
        for (evidence, expected) in [
            (
                RecoveryEvidence {
                    session: Some(RecoveryTerminal::Failed),
                    operation: Some(RecoveryTerminal::Failed),
                },
                "failed",
            ),
            (
                RecoveryEvidence {
                    session: None,
                    operation: Some(RecoveryTerminal::Cancelled),
                },
                "cancelled",
            ),
        ] {
            let repository = repository();
            let (run_id, _, _) = create_inflight_recovery_attempt(&repository);
            repository
                .recover_ambiguous_inflight(
                    &FixedRecoveryEvidence(evidence),
                    "2026-08-08T00:01:00Z",
                )
                .expect("reconcile");
            let detail = repository.get_run_detail(&run_id).expect("detail");
            assert_eq!(detail.summary.status, expected);
            assert_eq!(detail.tasks[0].status, expected);
            assert_eq!(detail.tasks[0].attempts[0].status, expected);
            assert_eq!(
                detail.tasks[0].attempts[0].error_class.as_deref(),
                Some(if expected == "failed" {
                    "restart_reconciled_failed"
                } else {
                    "restart_reconciled_cancelled"
                })
            );
        }
    }

    #[test]
    fn startup_recovery_projects_shared_success_and_gates_conflicts() {
        let success_repository = repository();
        let (run_id, _, _) = create_inflight_recovery_attempt(&success_repository);
        let success_evidence = CountingRecoveryEvidence::new(RecoveryEvidence {
            session: Some(RecoveryTerminal::Succeeded),
            operation: None,
        });
        success_repository
            .recover_ambiguous_inflight(&success_evidence, "2026-08-08T00:01:00Z")
            .expect("reconcile success");
        let detail = success_repository
            .get_run_detail(&run_id)
            .expect("success detail");
        assert_eq!(detail.summary.status, "running");
        assert_eq!(detail.tasks[0].status, "succeeded");
        assert_eq!(detail.tasks[0].attempts[0].status, "succeeded");
        assert_eq!(
            detail.tasks[0].attempts[0].error_class.as_deref(),
            Some("restart_reconciled_succeeded")
        );
        assert_eq!(
            success_repository
                .recover_ambiguous_inflight(&success_evidence, "2026-08-08T00:01:01Z")
                .expect("repeat recovery"),
            Vec::<String>::new()
        );
        assert_eq!(
            success_evidence
                .inspections
                .load(std::sync::atomic::Ordering::SeqCst),
            1
        );
        assert!(success_repository
            .schedule_next(&run_id, "2026-08-08T00:01:02Z")
            .is_ok());

        let conflict_repository = repository();
        let (run_id, _, _) = create_inflight_recovery_attempt(&conflict_repository);
        conflict_repository
            .recover_ambiguous_inflight(
                &FixedRecoveryEvidence(RecoveryEvidence {
                    session: Some(RecoveryTerminal::Failed),
                    operation: Some(RecoveryTerminal::Succeeded),
                }),
                "2026-08-08T00:01:00Z",
            )
            .expect("reconcile conflict");
        let detail = conflict_repository
            .get_run_detail(&run_id)
            .expect("conflict detail");
        assert_eq!(detail.summary.status, "recovery_required");
        assert_eq!(detail.tasks[0].status, "interrupted");
        assert!(matches!(
            conflict_repository.schedule_next(&run_id, "2026-08-08T00:01:01Z"),
            Err(PlanApplicationError::Conflict)
        ));
    }

    #[test]
    fn plan_run_timeout_stops_new_claims_and_retains_the_worktree() {
        let repository = repository();
        repository.save_draft(&draft()).expect("save");
        let run_id = repository
            .approve_latest("plan-1", "2026-08-08T00:00:00Z")
            .expect("approve");
        repository
            .begin_preparation(&run_id, "2026-08-08T00:00:00Z")
            .expect("prepare");
        attach_worktree(
            &repository,
            &run_id,
            "0123456789abcdef",
            "C:\\code\\retained-plan-worktree",
            "plan-run",
            "vanehub/plan-run",
            "2026-08-08T00:00:01Z",
        );
        let timeout = repository
            .schedule_next(&run_id, "2026-08-08T02:00:00Z")
            .expect("timeout projection");
        assert_eq!(timeout.decision.projection, RunProjection::Failed);
        assert!(timeout.subtask_run_id.is_none());
        let detail = repository.get_run_detail(&run_id).expect("detail");
        assert_eq!(detail.summary.status, "failed");
        assert_eq!(
            detail.worktree_path.as_deref(),
            Some("C:\\code\\retained-plan-worktree")
        );
        assert!(detail.tasks.iter().all(|task| task.status == "blocked"));
    }
}
