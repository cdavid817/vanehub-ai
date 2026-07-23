use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LoopEvidenceView, LoopIterationRepository, LoopIterationView,
    LoopRepository, LoopVerifierRecommendation, LoopVerifierResult, SaveLoopVerifierResultRequest,
};
use crate::contexts::agent_runtime::domain::{
    LoopDefinition, LoopDefinitionInput, LoopLimits, LoopRun, LoopRunPhase, LoopRunStatus,
    LoopTerminalReason, LoopVerificationCommand,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub(crate) struct SqliteLoopRepository {
    database: NativeDatabase,
}

impl SqliteLoopRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl LoopRepository for SqliteLoopRepository {
    fn list_definitions(&self) -> Result<Vec<LoopDefinition>, AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!(
                "{} ORDER BY updated_at DESC, id",
                definition_select("loop_definitions")
            ))
            .map_err(loop_error)?;
        let definitions = statement
            .query_map([], read_definition)
            .map_err(loop_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(loop_error)?;
        Ok(definitions)
    }

    fn find_definition(
        &self,
        definition_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        connection
            .query_row(
                &format!("{} WHERE id = ?1", definition_select("loop_definitions")),
                [definition_id],
                read_definition,
            )
            .optional()
            .map_err(loop_error)
    }

    fn create_definition(
        &self,
        definition: &LoopDefinition,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        let stored = StoredDefinition::from_domain(definition);
        connection
            .execute(
                r#"INSERT INTO loop_definitions (
                    id, name, enabled, project_path, base_branch, goal, acceptance_criteria,
                    allowed_paths, protected_paths, worker_agent_id, verifier_agent_id,
                    verification_commands, limits, version, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"#,
                rusqlite::params_from_iter(definition_param_values(&stored)?),
            )
            .map_err(loop_error)?;
        Ok(())
    }

    fn update_definition(
        &self,
        definition: &LoopDefinition,
        expected_version: u64,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        let stored = StoredDefinition::from_domain(definition);
        if stored.version != expected_version.saturating_add(1) {
            return Err(AgentRuntimeApplicationError::Validation(
                "Loop definition version must advance by exactly one.".to_string(),
            ));
        }
        let changed = connection
            .execute(
                r#"UPDATE loop_definitions SET
                    name = ?2, enabled = ?3, project_path = ?4, base_branch = ?5, goal = ?6,
                    acceptance_criteria = ?7, allowed_paths = ?8, protected_paths = ?9,
                    worker_agent_id = ?10, verifier_agent_id = ?11, verification_commands = ?12,
                    limits = ?13, version = ?14, updated_at = ?16
                WHERE id = ?1 AND version = ?17"#,
                rusqlite::params_from_iter(
                    definition_param_values(&stored)?
                        .into_iter()
                        .chain([rusqlite::types::Value::Integer(to_i64(expected_version)?)]),
                ),
            )
            .map_err(loop_error)?;
        require_changed(changed, "Loop definition changed or no longer exists")
    }

    fn delete_definition(&self, definition_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                "DELETE FROM loop_definitions WHERE id = ?1",
                [definition_id],
            )
            .map_err(loop_error)?;
        require_changed(changed, "Loop definition not found")
    }

    fn create_run(
        &self,
        run: &LoopRun,
        definition_snapshot: &LoopDefinition,
        project_path: &str,
        created_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let snapshot = serde_json::to_string(&StoredDefinition::from_domain(definition_snapshot))
            .map_err(loop_error)?;
        self.connection()?
            .execute(
                r#"INSERT INTO loop_runs (
                    id, definition_id, definition_snapshot, status, phase, terminal_reason,
                    current_iteration, consecutive_runtime_errors, consecutive_no_progress,
                    pause_requested, project_path, simulated, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12, ?12)"#,
                params![
                    run.id(),
                    run.definition_id(),
                    snapshot,
                    run.status().as_str(),
                    run.phase().as_str(),
                    run.terminal_reason().map(LoopTerminalReason::as_str),
                    i64::from(run.current_iteration()),
                    i64::from(run.consecutive_runtime_errors()),
                    i64::from(run.consecutive_no_progress()),
                    i64::from(run.pause_requested()),
                    project_path,
                    created_at,
                ],
            )
            .map_err(loop_error)?;
        Ok(())
    }

    fn find_run(&self, run_id: &str) -> Result<Option<LoopRun>, AgentRuntimeApplicationError> {
        self.connection()?
            .query_row(
                r#"SELECT id, definition_id, status, phase, terminal_reason, current_iteration,
                    consecutive_runtime_errors, consecutive_no_progress, pause_requested
                   FROM loop_runs WHERE id = ?1"#,
                [run_id],
                read_run,
            )
            .optional()
            .map_err(loop_error)
    }

    fn list_run_views(
        &self,
        definition_id: Option<&str>,
    ) -> Result<
        Vec<crate::contexts::agent_runtime::application::LoopRunView>,
        AgentRuntimeApplicationError,
    > {
        super::loop_repository_views::list_run_views(&self.database, definition_id)
    }

    fn find_run_view(
        &self,
        run_id: &str,
    ) -> Result<
        Option<crate::contexts::agent_runtime::application::LoopRunView>,
        AgentRuntimeApplicationError,
    > {
        super::loop_repository_views::find_run_view(&self.database, run_id)
    }

    fn has_active_run(&self, definition_id: &str) -> Result<bool, AgentRuntimeApplicationError> {
        self.connection()?
            .query_row(
                r#"SELECT EXISTS(
                    SELECT 1 FROM loop_runs
                    WHERE definition_id = ?1
                      AND status IN ('queued', 'running', 'paused', 'awaiting-acceptance')
                )"#,
                [definition_id],
                |row| row.get(0),
            )
            .map_err(loop_error)
    }

    fn attach_run_operation(
        &self,
        run_id: &str,
        operation_id: &str,
        expected_status: LoopRunStatus,
        updated_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE loop_runs SET active_operation_id = ?2, updated_at = ?3
                   WHERE id = ?1 AND status = ?4 AND active_operation_id IS NULL"#,
                params![run_id, operation_id, updated_at, expected_status.as_str()],
            )
            .map_err(loop_error)?;
        require_changed(
            changed,
            "Loop operation is already attached or the run state changed",
        )
    }

    fn attach_run_worktree(
        &self,
        run_id: &str,
        path: &str,
        name: &str,
        branch: &str,
        expected_status: LoopRunStatus,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE loop_runs
                   SET worktree_path = ?2, worktree_name = ?3, worktree_branch = ?4
                   WHERE id = ?1 AND status = ?5 AND worktree_path IS NULL
                     AND worktree_name IS NULL AND worktree_branch IS NULL"#,
                params![run_id, path, name, branch, expected_status.as_str()],
            )
            .map_err(loop_error)?;
        require_changed(
            changed,
            "Loop worktree is already attached or the run state changed",
        )
    }

    fn save_run_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        updated_at: &str,
        completed_at: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE loop_runs SET status = ?2, phase = ?3, terminal_reason = ?4,
                    current_iteration = ?5, consecutive_runtime_errors = ?6,
                    consecutive_no_progress = ?7, pause_requested = ?8, updated_at = ?9,
                    completed_at = ?10
                   WHERE id = ?1 AND status = ?11"#,
                params![
                    run.id(),
                    run.status().as_str(),
                    run.phase().as_str(),
                    run.terminal_reason().map(LoopTerminalReason::as_str),
                    i64::from(run.current_iteration()),
                    i64::from(run.consecutive_runtime_errors()),
                    i64::from(run.consecutive_no_progress()),
                    i64::from(run.pause_requested()),
                    updated_at,
                    completed_at,
                    expected_status.as_str(),
                ],
            )
            .map_err(loop_error)?;
        require_changed(
            changed,
            "Loop run state changed before this action completed",
        )
    }

    fn save_pause_request(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        expected_pause_requested: bool,
        updated_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE loop_runs SET pause_requested = ?2, updated_at = ?3
                   WHERE id = ?1 AND status = ?4 AND pause_requested = ?5"#,
                params![
                    run.id(),
                    i64::from(run.pause_requested()),
                    updated_at,
                    expected_status.as_str(),
                    i64::from(expected_pause_requested),
                ],
            )
            .map_err(loop_error)?;
        require_changed(
            changed,
            "Loop pause was already requested or the run state changed",
        )
    }

    fn find_run_definition_snapshot(
        &self,
        run_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        let snapshot = self
            .connection()?
            .query_row(
                "SELECT definition_snapshot FROM loop_runs WHERE id = ?1",
                [run_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(loop_error)?;
        snapshot
            .map(|value| {
                serde_json::from_str::<StoredDefinition>(&value)
                    .map_err(loop_error)?
                    .into_domain()
            })
            .transpose()
    }

    fn save_continue_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        feedback: &str,
        updated_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let previous_sequence = run.current_iteration().checked_sub(1).ok_or_else(|| {
            AgentRuntimeApplicationError::Loop("Loop iteration sequence is invalid.".to_string())
        })?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(loop_error)?;
        let feedback_changed = transaction
            .execute(
                r#"UPDATE loop_iterations SET user_feedback = ?3
                   WHERE run_id = ?1 AND sequence = ?2 AND user_feedback IS NULL"#,
                params![run.id(), i64::from(previous_sequence), feedback],
            )
            .map_err(loop_error)?;
        require_changed(
            feedback_changed,
            "Loop continuation feedback is already saved or the iteration changed",
        )?;
        let run_changed = transaction
            .execute(
                r#"UPDATE loop_runs SET status = ?2, phase = ?3, terminal_reason = ?4,
                    current_iteration = ?5, consecutive_runtime_errors = ?6,
                    consecutive_no_progress = ?7, pause_requested = ?8, updated_at = ?9,
                    completed_at = NULL
                   WHERE id = ?1 AND status = ?10"#,
                params![
                    run.id(),
                    run.status().as_str(),
                    run.phase().as_str(),
                    run.terminal_reason().map(LoopTerminalReason::as_str),
                    i64::from(run.current_iteration()),
                    i64::from(run.consecutive_runtime_errors()),
                    i64::from(run.consecutive_no_progress()),
                    i64::from(run.pause_requested()),
                    updated_at,
                    expected_status.as_str(),
                ],
            )
            .map_err(loop_error)?;
        require_changed(
            run_changed,
            "Loop run state changed before continuation completed",
        )?;
        transaction.commit().map_err(loop_error)
    }

    fn list_recoverable_runs(&self) -> Result<Vec<LoopRun>, AgentRuntimeApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                r#"SELECT id, definition_id, status, phase, terminal_reason, current_iteration,
                    consecutive_runtime_errors, consecutive_no_progress, pause_requested
                   FROM loop_runs
                   WHERE status IN ('queued', 'running', 'awaiting-acceptance')
                   ORDER BY created_at, id"#,
            )
            .map_err(loop_error)?;
        let runs = statement
            .query_map([], read_run)
            .map_err(loop_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(loop_error)?;
        Ok(runs)
    }

    fn save_recovery_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        evidence: &LoopEvidenceView,
        updated_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(loop_error)?;
        let run_changed = transaction
            .execute(
                r#"UPDATE loop_runs SET status = ?2, phase = ?3, terminal_reason = ?4,
                    pause_requested = ?5, active_operation_id = NULL, updated_at = ?6
                   WHERE id = ?1 AND status = ?7"#,
                params![
                    run.id(),
                    run.status().as_str(),
                    run.phase().as_str(),
                    run.terminal_reason().map(LoopTerminalReason::as_str),
                    i64::from(run.pause_requested()),
                    updated_at,
                    expected_status.as_str(),
                ],
            )
            .map_err(loop_error)?;
        require_changed(
            run_changed,
            "Loop run state changed before recovery completed",
        )?;
        transaction
            .execute(
                r#"INSERT INTO loop_evidence (
                    id, run_id, iteration_id, kind, status, summary, operation_id, command_id,
                    exit_code, duration_ms, details, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
                params![
                    evidence.id,
                    evidence.run_id,
                    evidence.iteration_id,
                    evidence.kind,
                    evidence.status,
                    evidence.summary,
                    evidence.operation_id,
                    evidence.command_id,
                    evidence.exit_code,
                    evidence.duration_ms.map(to_i64).transpose()?,
                    evidence
                        .details
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()
                        .map_err(loop_error)?,
                    evidence.created_at,
                ],
            )
            .map_err(loop_error)?;
        transaction.commit().map_err(loop_error)
    }
}

impl LoopIterationRepository for SqliteLoopRepository {
    fn insert_iteration(
        &self,
        iteration: &LoopIterationView,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.connection()?
            .execute(
                r#"INSERT INTO loop_iterations (
                    id, run_id, sequence, status, worker_session_id, verifier_session_id,
                    worker_summary, verifier_recommendation, verifier_findings, decision_reason,
                    diff_fingerprint, check_failure_fingerprint, user_feedback, started_at, completed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"#,
                params![
                    iteration.id,
                    iteration.run_id,
                    i64::from(iteration.sequence),
                    iteration.status.as_str(),
                    iteration.worker_session_id,
                    iteration.verifier_session_id,
                    iteration.worker_summary,
                    iteration.verifier_recommendation,
                    serde_json::to_string(&iteration.verifier_findings).map_err(loop_error)?,
                    iteration.decision_reason,
                    iteration.diff_fingerprint,
                    iteration.check_failure_fingerprint,
                    iteration.user_feedback,
                    iteration.started_at,
                    iteration.completed_at,
                ],
            )
            .map_err(loop_error)?;
        Ok(())
    }

    fn attach_worker_session(
        &self,
        iteration_id: &str,
        session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE loop_iterations SET worker_session_id = ?2
                   WHERE id = ?1 AND worker_summary IS NULL"#,
                params![iteration_id, session_id],
            )
            .map_err(loop_error)?;
        require_changed(
            changed,
            "Loop Worker result is already saved or the iteration no longer exists",
        )
    }

    fn attach_verifier_session(
        &self,
        iteration_id: &str,
        session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE loop_iterations SET verifier_session_id = ?2
                   WHERE id = ?1 AND verifier_recommendation IS NULL"#,
                params![iteration_id, session_id],
            )
            .map_err(loop_error)?;
        require_changed(
            changed,
            "Loop Verifier result is already saved or the iteration no longer exists",
        )
    }

    fn save_verifier_result(
        &self,
        request: &SaveLoopVerifierResultRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let findings = serde_json::to_string(&request.result.findings).map_err(loop_error)?;
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE loop_iterations
                   SET verifier_recommendation = ?4, verifier_findings = ?5
                   WHERE id = ?2 AND run_id = ?1 AND verifier_session_id = ?3
                     AND verifier_recommendation IS NULL"#,
                params![
                    request.run_id,
                    request.iteration_id,
                    request.session_id,
                    request.result.recommendation.as_str(),
                    findings
                ],
            )
            .map_err(loop_error)?;
        require_changed(
            changed,
            "Loop Verifier result ownership is invalid or a result is already saved",
        )
    }

    fn save_worker_summary(
        &self,
        run_id: &str,
        iteration_id: &str,
        session_id: &str,
        summary: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE loop_iterations SET worker_summary = ?4
               WHERE run_id = ?1 AND id = ?2 AND worker_session_id = ?3
                 AND worker_summary IS NULL"#,
                params![run_id, iteration_id, session_id, summary],
            )
            .map_err(loop_error)?;
        require_changed(
            changed,
            "Loop Worker result ownership is invalid or already saved",
        )
    }

    fn complete_iteration(
        &self,
        run_id: &str,
        iteration_id: &str,
        status: LoopRunStatus,
        decision_reason: &str,
        completed_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE loop_iterations
               SET status = ?3, decision_reason = ?4, completed_at = ?5
               WHERE run_id = ?1 AND id = ?2 AND completed_at IS NULL"#,
                params![
                    run_id,
                    iteration_id,
                    status.as_str(),
                    decision_reason,
                    completed_at
                ],
            )
            .map_err(loop_error)?;
        require_changed(
            changed,
            "Loop iteration is already complete or ownership is invalid",
        )
    }

    fn save_iteration_fingerprints(
        &self,
        run_id: &str,
        iteration_id: &str,
        diff_fingerprint: &str,
        check_failure_fingerprint: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"UPDATE loop_iterations
                   SET diff_fingerprint = ?3, check_failure_fingerprint = ?4
                   WHERE id = ?2 AND run_id = ?1
                     AND diff_fingerprint IS NULL AND check_failure_fingerprint IS NULL"#,
                params![
                    run_id,
                    iteration_id,
                    diff_fingerprint,
                    check_failure_fingerprint
                ],
            )
            .map_err(loop_error)?;
        require_changed(
            changed,
            "Loop iteration fingerprint ownership is invalid or fingerprints are already saved",
        )
    }

    fn append_evidence(
        &self,
        evidence: &LoopEvidenceView,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.connection()?
            .execute(
                r#"INSERT INTO loop_evidence (
                    id, run_id, iteration_id, kind, status, summary, operation_id, command_id,
                    exit_code, duration_ms, details, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
                params![
                    evidence.id,
                    evidence.run_id,
                    evidence.iteration_id,
                    evidence.kind,
                    evidence.status,
                    evidence.summary,
                    evidence.operation_id,
                    evidence.command_id,
                    evidence.exit_code,
                    evidence.duration_ms.map(to_i64).transpose()?,
                    evidence
                        .details
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()
                        .map_err(loop_error)?,
                    evidence.created_at,
                ],
            )
            .map_err(loop_error)?;
        Ok(())
    }
}

impl SqliteLoopRepository {
    fn connection(&self) -> Result<rusqlite::Connection, AgentRuntimeApplicationError> {
        self.database.connection().map_err(loop_error)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredCommand {
    id: String,
    program: String,
    args: Vec<String>,
    working_directory: Option<String>,
    timeout_seconds: u64,
    required: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredLimits {
    max_iterations: u16,
    step_timeout_seconds: u64,
    total_timeout_seconds: u64,
    max_consecutive_runtime_errors: u16,
    max_consecutive_no_progress: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct StoredDefinition {
    id: String,
    name: String,
    enabled: bool,
    project_path: String,
    base_branch: String,
    goal: String,
    acceptance_criteria: Vec<String>,
    allowed_paths: Vec<String>,
    protected_paths: Vec<String>,
    worker_agent_id: String,
    verifier_agent_id: String,
    verification_commands: Vec<StoredCommand>,
    limits: StoredLimits,
    version: u64,
    created_at: String,
    updated_at: String,
}

impl StoredDefinition {
    fn from_domain(definition: &LoopDefinition) -> Self {
        let value = definition.values();
        Self {
            id: value.id.clone(),
            name: value.name.clone(),
            enabled: value.enabled,
            project_path: value.project_path.clone(),
            base_branch: value.base_branch.clone(),
            goal: value.goal.clone(),
            acceptance_criteria: value.acceptance_criteria.clone(),
            allowed_paths: value.allowed_paths.clone(),
            protected_paths: value.protected_paths.clone(),
            worker_agent_id: value.worker_agent_id.clone(),
            verifier_agent_id: value.verifier_agent_id.clone(),
            verification_commands: value
                .verification_commands
                .iter()
                .map(|command| StoredCommand {
                    id: command.id().to_string(),
                    program: command.program().to_string(),
                    args: command.args().to_vec(),
                    working_directory: command.working_directory().map(str::to_string),
                    timeout_seconds: command.timeout_seconds(),
                    required: command.required(),
                })
                .collect(),
            limits: StoredLimits {
                max_iterations: value.limits.max_iterations(),
                step_timeout_seconds: value.limits.step_timeout_seconds(),
                total_timeout_seconds: value.limits.total_timeout_seconds(),
                max_consecutive_runtime_errors: value.limits.max_consecutive_runtime_errors(),
                max_consecutive_no_progress: value.limits.max_consecutive_no_progress(),
            },
            version: value.version,
            created_at: value.created_at.clone(),
            updated_at: value.updated_at.clone(),
        }
    }

    pub(super) fn into_domain(self) -> Result<LoopDefinition, AgentRuntimeApplicationError> {
        let commands = self
            .verification_commands
            .into_iter()
            .map(|command| {
                LoopVerificationCommand::new(
                    command.id,
                    command.program,
                    command.args,
                    command.working_directory,
                    command.timeout_seconds,
                    command.required,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let limits = LoopLimits::new(
            self.limits.max_iterations,
            self.limits.step_timeout_seconds,
            self.limits.total_timeout_seconds,
            self.limits.max_consecutive_runtime_errors,
            self.limits.max_consecutive_no_progress,
        )?;
        Ok(LoopDefinition::new(LoopDefinitionInput {
            id: self.id,
            name: self.name,
            enabled: self.enabled,
            project_path: self.project_path,
            base_branch: self.base_branch,
            goal: self.goal,
            acceptance_criteria: self.acceptance_criteria,
            allowed_paths: self.allowed_paths,
            protected_paths: self.protected_paths,
            worker_agent_id: self.worker_agent_id,
            verifier_agent_id: self.verifier_agent_id,
            verification_commands: commands,
            limits,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })?)
    }
}

fn definition_select(table: &str) -> String {
    format!("SELECT id, name, enabled, project_path, base_branch, goal, acceptance_criteria, allowed_paths, protected_paths, worker_agent_id, verifier_agent_id, verification_commands, limits, version, created_at, updated_at FROM {table}")
}

fn read_definition(row: &Row<'_>) -> rusqlite::Result<LoopDefinition> {
    let stored = StoredDefinition {
        id: row.get(0)?,
        name: row.get(1)?,
        enabled: row.get(2)?,
        project_path: row.get(3)?,
        base_branch: row.get(4)?,
        goal: row.get(5)?,
        acceptance_criteria: decode_json(row, 6)?,
        allowed_paths: decode_json(row, 7)?,
        protected_paths: decode_json(row, 8)?,
        worker_agent_id: row.get(9)?,
        verifier_agent_id: row.get(10)?,
        verification_commands: decode_json(row, 11)?,
        limits: decode_json(row, 12)?,
        version: row.get::<_, i64>(13)? as u64,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    };
    stored.into_domain().map_err(to_sql_error)
}

fn read_run(row: &Row<'_>) -> rusqlite::Result<LoopRun> {
    LoopRun::rehydrate(
        row.get(0)?,
        row.get(1)?,
        LoopRunStatus::parse(&row.get::<_, String>(2)?).map_err(to_sql_error)?,
        LoopRunPhase::parse(&row.get::<_, String>(3)?).map_err(to_sql_error)?,
        row.get::<_, Option<String>>(4)?
            .map(|value| LoopTerminalReason::parse(&value))
            .transpose()
            .map_err(to_sql_error)?,
        row.get::<_, i64>(5)? as u16,
        row.get::<_, i64>(6)? as u16,
        row.get::<_, i64>(7)? as u16,
        row.get(8)?,
    )
    .map_err(to_sql_error)
}

fn decode_json<T: for<'de> Deserialize<'de>>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    serde_json::from_str(&row.get::<_, String>(index)?).map_err(to_sql_error)
}

fn definition_param_values(
    stored: &StoredDefinition,
) -> Result<Vec<rusqlite::types::Value>, AgentRuntimeApplicationError> {
    use rusqlite::types::Value;
    Ok(vec![
        Value::Text(stored.id.clone()),
        Value::Text(stored.name.clone()),
        Value::Integer(i64::from(stored.enabled)),
        Value::Text(stored.project_path.clone()),
        Value::Text(stored.base_branch.clone()),
        Value::Text(stored.goal.clone()),
        Value::Text(serde_json::to_string(&stored.acceptance_criteria).map_err(loop_error)?),
        Value::Text(serde_json::to_string(&stored.allowed_paths).map_err(loop_error)?),
        Value::Text(serde_json::to_string(&stored.protected_paths).map_err(loop_error)?),
        Value::Text(stored.worker_agent_id.clone()),
        Value::Text(stored.verifier_agent_id.clone()),
        Value::Text(serde_json::to_string(&stored.verification_commands).map_err(loop_error)?),
        Value::Text(serde_json::to_string(&stored.limits).map_err(loop_error)?),
        Value::Integer(to_i64(stored.version)?),
        Value::Text(stored.created_at.clone()),
        Value::Text(stored.updated_at.clone()),
    ])
}

fn require_changed(changed: usize, message: &str) -> Result<(), AgentRuntimeApplicationError> {
    if changed == 1 {
        Ok(())
    } else {
        Err(AgentRuntimeApplicationError::Loop(message.to_string()))
    }
}

fn to_i64(value: u64) -> Result<i64, AgentRuntimeApplicationError> {
    i64::try_from(value).map_err(|_| {
        AgentRuntimeApplicationError::Validation("Loop numeric value is too large.".to_string())
    })
}

fn loop_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(error.to_string())
}

fn to_sql_error(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn definition(version: u64) -> LoopDefinition {
        LoopDefinition::new(LoopDefinitionInput {
            id: "loop-1".to_string(),
            name: format!("Loop {version}"),
            enabled: true,
            project_path: "D:/project".to_string(),
            base_branch: "main".to_string(),
            goal: "Implement the requested change".to_string(),
            acceptance_criteria: vec!["Tests pass".to_string()],
            allowed_paths: vec!["src".to_string()],
            protected_paths: vec![".git".to_string()],
            worker_agent_id: "codex-cli".to_string(),
            verifier_agent_id: "claude-code".to_string(),
            verification_commands: vec![LoopVerificationCommand::new(
                "tests".to_string(),
                "npm".to_string(),
                vec!["test".to_string()],
                None,
                300,
                true,
            )
            .expect("command")],
            limits: LoopLimits::new(3, 300, 1800, 2, 2).expect("limits"),
            version,
            created_at: "2026-07-21T00:00:00Z".to_string(),
            updated_at: format!("2026-07-21T00:00:0{version}Z"),
        })
        .expect("definition")
    }

    fn repository() -> (SqliteLoopRepository, NativeDatabase, TempDirectory) {
        let directory = TempDirectory::new("loop-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteLoopRepository::new(database.clone());
        (repository, database, directory)
    }

    #[test]
    fn definition_updates_use_atomic_versions() {
        let (repository, _, _directory) = repository();
        repository
            .create_definition(&definition(1))
            .expect("create definition");
        assert_eq!(repository.list_definitions().expect("list").len(), 1);
        assert_eq!(
            repository
                .find_definition("loop-1")
                .expect("find")
                .expect("definition")
                .values()
                .version,
            1
        );

        repository
            .update_definition(&definition(2), 1)
            .expect("update definition");
        assert!(repository.update_definition(&definition(2), 1).is_err());
    }

    #[test]
    fn run_transitions_and_evidence_are_atomic() {
        let (repository, database, _directory) = repository();
        let definition = definition(1);
        repository
            .create_definition(&definition)
            .expect("definition");
        let mut run = LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run");
        repository
            .create_run(&run, &definition, "D:/project", "2026-07-21T00:00:00Z")
            .expect("run");
        assert!(repository.has_active_run("loop-1").expect("active run"));
        repository
            .attach_run_operation(
                "run-1",
                "operation-1",
                LoopRunStatus::Queued,
                "2026-07-21T00:00:00Z",
            )
            .expect("attach operation");
        assert!(repository
            .attach_run_operation(
                "run-1",
                "operation-2",
                LoopRunStatus::Queued,
                "2026-07-21T00:00:00Z",
            )
            .is_err());
        repository
            .attach_run_worktree(
                "run-1",
                "D:/project-loop-1",
                "loop-1",
                "vanehub/loop-1",
                LoopRunStatus::Queued,
            )
            .expect("attach worktree");
        assert!(repository
            .attach_run_worktree(
                "run-1",
                "D:/replacement",
                "replacement",
                "vanehub/replacement",
                LoopRunStatus::Queued,
            )
            .is_err());
        let other = LoopRun::new("run-2".to_string(), "loop-1".to_string()).expect("other run");
        assert!(repository
            .create_run(&other, &definition, "D:/project", "2026-07-21T00:00:00Z")
            .is_err());

        run.begin().expect("begin");
        repository
            .save_run_transition(&run, LoopRunStatus::Queued, "2026-07-21T00:00:01Z", None)
            .expect("transition");
        assert!(repository
            .save_run_transition(&run, LoopRunStatus::Queued, "2026-07-21T00:00:01Z", None,)
            .is_err());
        assert_eq!(
            repository
                .find_run("run-1")
                .expect("find run")
                .expect("run")
                .status(),
            LoopRunStatus::Running
        );

        repository
            .insert_iteration(&LoopIterationView {
                id: "iteration-1".to_string(),
                run_id: "run-1".to_string(),
                sequence: 1,
                status: LoopRunStatus::Running,
                worker_session_id: None,
                verifier_session_id: None,
                worker_summary: None,
                verifier_recommendation: None,
                verifier_findings: Vec::new(),
                decision_reason: None,
                diff_fingerprint: None,
                check_failure_fingerprint: None,
                user_feedback: None,
                evidence: Vec::new(),
                started_at: "2026-07-21T00:00:01Z".to_string(),
                completed_at: None,
            })
            .expect("iteration");
        database
            .connection()
            .expect("connection")
            .execute(
                r#"INSERT INTO sessions (
                    id, title, agent_id, interaction_mode, lifecycle_state,
                    pinned, archived, created_at, updated_at
                ) SELECT 'worker-session-1', 'Loop worker', id, 'cli', 'idle', 0, 0,
                    '2026-07-21T00:00:01Z', '2026-07-21T00:00:01Z'
                  FROM agents ORDER BY id LIMIT 1"#,
                [],
            )
            .expect("worker session");
        repository
            .attach_worker_session("iteration-1", "worker-session-1")
            .expect("attach worker session");
        repository
            .attach_worker_session("iteration-1", "worker-session-1")
            .expect("reattach before Worker result");
        repository
            .save_worker_summary("run-1", "iteration-1", "worker-session-1", "Implemented")
            .expect("worker summary");
        assert!(repository
            .attach_worker_session("iteration-1", "worker-session-1")
            .is_err());
        database
            .connection()
            .expect("connection")
            .execute(
                r#"INSERT INTO sessions (
                    id, title, agent_id, interaction_mode, lifecycle_state,
                    pinned, archived, created_at, updated_at
                ) SELECT 'verifier-session-1', 'Loop verifier', id, 'cli', 'idle', 0, 0,
                    '2026-07-21T00:00:01Z', '2026-07-21T00:00:01Z'
                  FROM agents ORDER BY id LIMIT 1"#,
                [],
            )
            .expect("verifier session");
        repository
            .attach_verifier_session("iteration-1", "verifier-session-1")
            .expect("attach verifier session");
        repository
            .attach_verifier_session("iteration-1", "verifier-session-1")
            .expect("reattach before Verifier result");
        repository
            .save_verifier_result(&SaveLoopVerifierResultRequest {
                run_id: "run-1".to_string(),
                iteration_id: "iteration-1".to_string(),
                session_id: "verifier-session-1".to_string(),
                result: LoopVerifierResult {
                    recommendation: LoopVerifierRecommendation::Revise,
                    findings: vec!["Fix the failed check.".to_string()],
                },
            })
            .expect("verifier result");
        assert!(repository
            .attach_verifier_session("iteration-1", "verifier-session-1")
            .is_err());
        assert!(repository
            .save_verifier_result(&SaveLoopVerifierResultRequest {
                run_id: "run-1".to_string(),
                iteration_id: "iteration-1".to_string(),
                session_id: "verifier-session-1".to_string(),
                result: LoopVerifierResult {
                    recommendation: LoopVerifierRecommendation::Pass,
                    findings: Vec::new(),
                },
            })
            .is_err());
        repository
            .save_iteration_fingerprints(
                "run-1",
                "iteration-1",
                "diff-fingerprint",
                "failure-fingerprint",
            )
            .expect("fingerprints");
        repository
            .complete_iteration(
                "run-1",
                "iteration-1",
                LoopRunStatus::Failed,
                "Revision required",
                "2026-07-21T00:00:02Z",
            )
            .expect("iteration decision");
        assert!(repository
            .complete_iteration(
                "run-1",
                "iteration-1",
                LoopRunStatus::Failed,
                "duplicate",
                "2026-07-21T00:00:03Z",
            )
            .is_err());
        assert!(repository
            .save_iteration_fingerprints(
                "run-1",
                "iteration-1",
                "replacement-diff",
                "replacement-failure",
            )
            .is_err());
        repository
            .append_evidence(&LoopEvidenceView {
                id: "evidence-1".to_string(),
                run_id: "run-1".to_string(),
                iteration_id: Some("iteration-1".to_string()),
                kind: "worker".to_string(),
                status: "passed".to_string(),
                summary: "Worker completed".to_string(),
                operation_id: None,
                command_id: None,
                exit_code: None,
                duration_ms: Some(20),
                details: None,
                created_at: "2026-07-21T00:00:02Z".to_string(),
            })
            .expect("evidence");

        run.cancel(LoopTerminalReason::UserStopped)
            .expect("cancel run");
        repository
            .save_run_transition(
                &run,
                LoopRunStatus::Running,
                "2026-07-21T00:00:03Z",
                Some("2026-07-21T00:00:03Z"),
            )
            .expect("terminal transition");

        let connection = database.connection().expect("connection");
        let recommendation: String = connection
            .query_row(
                "SELECT verifier_recommendation FROM loop_iterations WHERE id = 'iteration-1'",
                [],
                |row| row.get(0),
            )
            .expect("recommendation");
        assert_eq!(recommendation, "revise");
        let fingerprints: (String, String) = connection
            .query_row(
                r#"SELECT diff_fingerprint, check_failure_fingerprint
                   FROM loop_iterations WHERE id = 'iteration-1'"#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("fingerprints");
        assert_eq!(
            fingerprints,
            (
                "diff-fingerprint".to_string(),
                "failure-fingerprint".to_string()
            )
        );
        let evidence_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM loop_evidence", [], |row| row.get(0))
            .expect("evidence count");
        assert_eq!(evidence_count, 1);
        let worktree_path: String = connection
            .query_row(
                "SELECT worktree_path FROM loop_runs WHERE id = 'run-1'",
                [],
                |row| row.get(0),
            )
            .expect("worktree path");
        assert_eq!(worktree_path, "D:/project-loop-1");
        let status: String = connection
            .query_row(
                "SELECT status FROM loop_runs WHERE id = 'run-1'",
                [],
                |row| row.get(0),
            )
            .expect("run status");
        assert_eq!(status, "cancelled");
    }
}
