use super::rows::{
    deserialize_configuration, file_references_json, json_values, load_category, load_message,
    load_session, recovery_repository_error, repository_error, serialize_configuration,
    CategoryRow, MessageRow, SessionRow, CATEGORY_SELECT, MESSAGE_SELECT, SESSION_SEARCH_SELECT,
    SESSION_SELECT,
};
use crate::contexts::sessions::application::{
    CategoryRecord, ChatConfigurationValues, MessagePageQuery, MessageRecord,
    RecoveryCandidateClaim, SessionCategoryRepository, SessionConfigurationRepository,
    SessionListScope, SessionMessageRepository, SessionRecord, SessionRecoveryReportRepository,
    SessionRepository, SessionSearchMatch, SessionSearchMatchKind, SessionSearchQuery,
    SessionSearchResult, SessionTerminalEvidencePort, SessionsApplicationError,
};
use crate::contexts::sessions::domain::evidence::{
    ExecutionEvidenceFidelity, LiveHandleEvidence, MessageTerminalEvidence, ProviderResumeEvidence,
    SessionEvidenceState, SessionTerminalEvidence, ToolActivityEvidence, ToolEvidenceFidelity,
    MAX_RECOVERY_EVIDENCE_MESSAGES,
};
use crate::contexts::sessions::domain::recovery::{
    RecoveryDecision, RecoveryEvidenceReference, RecoveryReasonCode, RecoveryTrigger,
    SessionRecoveryReport,
};
use crate::contexts::sessions::domain::{
    encode_seats, CategoryId, ChatPreferences, MessageId, SessionId,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

#[derive(Clone)]
pub(crate) struct SqliteSessionsRepository {
    pub(super) database: NativeDatabase,
}

impl SqliteSessionsRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(super) fn connection(&self) -> Result<PooledSqlite, SessionsApplicationError> {
        self.database
            .connection()
            .map_err(|error| SessionsApplicationError::Repository(error.to_string()))
    }

    pub(super) fn recovery_connection(&self) -> Result<PooledSqlite, SessionsApplicationError> {
        self.database
            .connection()
            .map_err(|error| SessionsApplicationError::RetryableStorage(error.to_string()))
    }
}

impl SessionTerminalEvidencePort for SqliteSessionsRepository {
    fn read_terminal_evidence(
        &self,
        session_id: &SessionId,
        execution_run_id: Option<&str>,
    ) -> Result<SessionTerminalEvidence, SessionsApplicationError> {
        let mut connection = self.recovery_connection()?;
        let transaction = connection
            .transaction()
            .map_err(recovery_repository_error)?;
        let session = transaction
            .query_row(
                &format!("{SESSION_SELECT} WHERE id = ?1"),
                [session_id.as_str()],
                SessionRow::read,
            )
            .optional()
            .map_err(recovery_evidence_row_error)?
            .map(SessionRow::into_record)
            .transpose()
            .map_err(structural_recovery_evidence_error)?
            .ok_or_else(|| {
                SessionsApplicationError::SessionNotFound(session_id.as_str().to_string())
            })?;
        let run_filter = if execution_run_id.is_some() {
            "execution_run_id = ?2"
        } else {
            "execution_run_id IS NULL"
        };
        let limit_parameter = if execution_run_id.is_some() {
            "?3"
        } else {
            "?2"
        };
        let mut statement = transaction
            .prepare(&format!(
                "{MESSAGE_SELECT} INDEXED BY idx_messages_session_run_sequence \
                 WHERE session_id = ?1 AND {run_filter} \
                 ORDER BY session_sequence, id LIMIT {limit_parameter}"
            ))
            .map_err(recovery_repository_error)?;
        let limit = (MAX_RECOVERY_EVIDENCE_MESSAGES + 1) as i64;
        let records = if let Some(run_id) = execution_run_id {
            statement
                .query_map(
                    params![session_id.as_str(), run_id, limit],
                    MessageRow::read,
                )
                .map_err(recovery_repository_error)?
                .map(|row| {
                    row.map_err(recovery_evidence_row_error).and_then(|row| {
                        row.into_record()
                            .map_err(structural_recovery_evidence_error)
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            statement
                .query_map(params![session_id.as_str(), limit], MessageRow::read)
                .map_err(recovery_repository_error)?
                .map(|row| {
                    row.map_err(recovery_evidence_row_error).and_then(|row| {
                        row.into_record()
                            .map_err(structural_recovery_evidence_error)
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        drop(statement);
        let conflicting_record = execution_run_id
            .map(|run_id| {
                transaction
                    .query_row(
                        &format!(
                            "{MESSAGE_SELECT} INDEXED BY idx_messages_unfinished_session_sequence \
                             WHERE session_id = ?1 \
                             AND execution_run_id IS NOT NULL AND execution_run_id <> ?2 \
                             AND status IN ('pending', 'streaming') \
                             ORDER BY session_sequence, id LIMIT 1"
                        ),
                        params![session_id.as_str(), run_id],
                        MessageRow::read,
                    )
                    .optional()
                    .map_err(recovery_evidence_row_error)?
                    .map(|row| {
                        row.into_record()
                            .map_err(structural_recovery_evidence_error)
                    })
                    .transpose()
            })
            .transpose()?
            .flatten();
        transaction.commit().map_err(recovery_repository_error)?;
        let execution_fidelity = execution_fidelity(&session.interaction_mode);
        let tool_fidelity = match execution_fidelity {
            ExecutionEvidenceFidelity::ManagedApi => ToolEvidenceFidelity::Managed,
            ExecutionEvidenceFidelity::ManagedCliOpaque
            | ExecutionEvidenceFidelity::InteractiveOpaque => ToolEvidenceFidelity::ProviderOpaque,
        };
        let messages = records
            .into_iter()
            .map(|record| message_terminal_evidence(record, tool_fidelity))
            .collect();
        let evidence_state = SessionEvidenceState {
            session_id: session.id().to_string(),
            lifecycle: session.aggregate.lifecycle(),
            recovery_status: session.aggregate.recovery().status(),
            active_execution_run_id: session
                .aggregate
                .recovery()
                .active_execution_run_id()
                .map(str::to_string),
            recovery_revision: session.aggregate.recovery().recovery_revision(),
            state_revision: session.aggregate.recovery().state_revision(),
            history_revision: session.aggregate.recovery().history_revision(),
            execution_fidelity,
        };
        let mut evidence = SessionTerminalEvidence::try_new(
            evidence_state,
            execution_run_id.map(str::to_string),
            messages,
            Vec::new(),
            ProviderResumeEvidence {
                metadata_present: session.runtime_session_id.is_some(),
            },
            LiveHandleEvidence::Unavailable,
        )
        .map_err(|error| {
            SessionsApplicationError::StructuralRecoveryEvidence(format!(
                "terminal evidence exceeded its bounded read: {error:?}"
            ))
        })?;
        evidence.set_conflicting_message(
            conflicting_record.map(|record| message_terminal_evidence(record, tool_fidelity)),
        );
        Ok(evidence)
    }
}

fn recovery_evidence_row_error(error: rusqlite::Error) -> SessionsApplicationError {
    match error {
        sqlite @ rusqlite::Error::SqliteFailure(_, _) => recovery_repository_error(sqlite),
        _ => SessionsApplicationError::StructuralRecoveryEvidence(
            "persisted recovery evidence row could not be decoded".to_string(),
        ),
    }
}

fn structural_recovery_evidence_error(
    _error: SessionsApplicationError,
) -> SessionsApplicationError {
    SessionsApplicationError::StructuralRecoveryEvidence(
        "persisted recovery evidence violates the session schema".to_string(),
    )
}

fn message_terminal_evidence(
    record: MessageRecord,
    tool_fidelity: ToolEvidenceFidelity,
) -> MessageTerminalEvidence {
    MessageTerminalEvidence {
        message_id: record.message.id().as_str().to_string(),
        session_sequence: record.message.session_sequence(),
        execution_run_id: record.message.execution_run_id().map(str::to_string),
        role: record.message.role(),
        status: record.message.status(),
        has_content: !record.content.is_empty(),
        tool_activity: classify_tool_activity(record.tool_use.as_deref(), tool_fidelity),
    }
}

fn execution_fidelity(interaction_mode: &str) -> ExecutionEvidenceFidelity {
    match interaction_mode {
        "api" => ExecutionEvidenceFidelity::ManagedApi,
        "cli" => ExecutionEvidenceFidelity::ManagedCliOpaque,
        _ => ExecutionEvidenceFidelity::InteractiveOpaque,
    }
}

fn classify_tool_activity(
    values: Option<&[Value]>,
    fidelity: ToolEvidenceFidelity,
) -> ToolActivityEvidence {
    let Some(values) = values.filter(|values| !values.is_empty()) else {
        return ToolActivityEvidence::None;
    };
    let count = u32::try_from(values.len()).unwrap_or(u32::MAX);
    let complete = values.iter().all(|value| {
        value
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| {
                matches!(
                    status,
                    "completed" | "succeeded" | "success" | "failed" | "error" | "cancelled"
                )
            })
    });
    if complete {
        ToolActivityEvidence::Complete { count, fidelity }
    } else {
        ToolActivityEvidence::Incomplete { count, fidelity }
    }
}

impl SessionRepository for SqliteSessionsRepository {
    fn find(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        load_session(&*self.connection()?, session_id)
    }

    fn list(
        &self,
        scope: SessionListScope,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        self.list_with_loop_visibility(scope, false)
    }

    fn recovery_candidates(
        &self,
        limit: usize,
    ) -> Result<Vec<RecoveryCandidateClaim>, SessionsApplicationError> {
        self.recovery_candidates_after(None, limit)
    }

    fn recovery_candidates_after(
        &self,
        after_session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecoveryCandidateClaim>, SessionsApplicationError> {
        let bounded_limit = limit.clamp(1, 100);
        let connection = self.recovery_connection()?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT id, active_execution_run_id, recovery_revision,
                       state_revision, history_revision, recovery_status, lifecycle_state
                FROM sessions
                WHERE archived = 0
                  AND recovery_status NOT IN ('action_required', 'quarantined')
                  AND (
                    active_execution_run_id IS NOT NULL
                    OR lifecycle_state IN ('starting', 'running')
                    OR recovery_status = 'reconciling'
                  )
                  AND (?1 IS NULL OR id > ?1)
                ORDER BY id
                LIMIT ?2
                "#,
            )
            .map_err(recovery_repository_error)?;
        let candidates = statement
            .query_map(params![after_session_id, bounded_limit as i64], |row| {
                let observed_lifecycle = row.get::<_, String>(6)?;
                let previous_recovery_status = row.get::<_, String>(5)?;
                let observed_execution_run_id = row.get::<_, Option<String>>(1)?;
                let recovery_revision = row.get::<_, i64>(2)?;
                let state_revision = row.get::<_, i64>(3)?;
                let history_revision = row.get::<_, i64>(4)?;
                let structurally_invalid = recovery_revision < 0
                    || state_revision < 0
                    || history_revision < 0
                    || !matches!(
                        observed_lifecycle.as_str(),
                        "idle" | "starting" | "running" | "failed" | "stopped"
                    )
                    || !matches!(previous_recovery_status.as_str(), "clean" | "reconciling")
                    || observed_execution_run_id.as_deref() == Some("");
                Ok(RecoveryCandidateClaim {
                    session_id: row.get(0)?,
                    observed_lifecycle: if matches!(
                        observed_lifecycle.as_str(),
                        "idle" | "starting" | "running" | "failed" | "stopped"
                    ) {
                        observed_lifecycle
                    } else {
                        "failed".to_string()
                    },
                    observed_execution_run_id,
                    recovery_revision: u64::try_from(recovery_revision).unwrap_or(0),
                    state_revision: u64::try_from(state_revision).unwrap_or(0),
                    history_revision: u64::try_from(history_revision).unwrap_or(0),
                    captured_recovery_revision: recovery_revision,
                    captured_state_revision: state_revision,
                    captured_history_revision: history_revision,
                    previous_recovery_status,
                    structurally_invalid,
                })
            })
            .map_err(recovery_repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(recovery_repository_error)?;
        Ok(candidates)
    }

    #[cfg(test)]
    fn recovery_candidate_count(&self) -> Result<usize, SessionsApplicationError> {
        self.recovery_connection()?
            .query_row(
                r#"
                SELECT COUNT(*)
                FROM sessions
                WHERE archived = 0
                  AND recovery_status NOT IN ('action_required', 'quarantined')
                  AND (
                    active_execution_run_id IS NOT NULL
                    OR lifecycle_state IN ('starting', 'running')
                    OR recovery_status = 'reconciling'
                  )
                "#,
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(recovery_repository_error)
            .and_then(|count| {
                usize::try_from(count).map_err(|_| {
                    SessionsApplicationError::Repository(
                        "recovery candidate count exceeded the platform limit".to_string(),
                    )
                })
            })
    }

    #[cfg(test)]
    fn list_including_loop_owned(
        &self,
        scope: SessionListScope,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        self.list_with_loop_visibility(scope, true)
    }

    fn search(
        &self,
        query: &SessionSearchQuery,
    ) -> Result<Vec<SessionSearchResult>, SessionsApplicationError> {
        let connection = self.connection()?;
        let pattern = like_pattern(&query.text);
        let (sql, message_query) = if query.text.chars().count() >= MIN_INDEXED_QUERY_CHARS {
            (indexed_search_statement(), fts_literal(&query.text))
        } else {
            (compatibility_search_statement(), pattern.clone())
        };
        let mut statement = connection.prepare(&sql).map_err(repository_error)?;
        let rows = statement
            .query_map(params![message_query, pattern, query.limit as i64], |row| {
                Ok((
                    SessionRow::read(row)?,
                    row.get::<_, Option<String>>(36)?,
                    row.get::<_, Option<String>>(37)?,
                ))
            })
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        rows.into_iter()
            .map(|(row, message_id, message_content)| {
                let session = row.into_record()?;
                let matches =
                    search_matches(&session, &query.text, message_id.zip(message_content));
                Ok(SessionSearchResult { session, matches })
            })
            .collect()
    }

    fn active_session(&self) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        let connection = self.connection()?;
        let active_session_id = connection
            .query_row(
                "SELECT active_session_id FROM workflow_state WHERE id = 1",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(repository_error)?
            .flatten();
        let Some(session_id) = active_session_id else {
            return Ok(None);
        };
        let session_id = SessionId::parse(session_id)?;
        let session = load_session(&connection, &session_id)?;
        if session.is_none() {
            connection
                .execute(
                    "UPDATE workflow_state SET active_session_id = NULL WHERE id = 1 AND active_session_id = ?1",
                    [session_id.as_str()],
                )
                .map_err(repository_error)?;
        }
        Ok(session)
    }

    fn save(&self, session: &SessionRecord) -> Result<SessionRecord, SessionsApplicationError> {
        self.save_with_revision(session, None)?
            .ok_or_else(|| SessionsApplicationError::SessionNotFound(session.id().to_string()))
    }

    fn save_if_revision(
        &self,
        session: &SessionRecord,
        expected_updated_at: &str,
    ) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        self.save_with_revision(session, Some(expected_updated_at))
    }

    fn inactive_sessions(
        &self,
        cutoff: &str,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        self.query_sessions(
            "WHERE archived = 0 AND pinned = 0 AND loop_run_id IS NULL AND recovery_status = 'clean' AND active_execution_run_id IS NULL AND lifecycle_state NOT IN ('starting', 'running') AND updated_at < ?1 ORDER BY updated_at ASC",
            Some(cutoff),
        )
    }
}

impl SqliteSessionsRepository {
    fn save_with_revision(
        &self,
        session: &SessionRecord,
        expected_updated_at: Option<&str>,
    ) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        let connection = self.connection()?;
        let changed = connection
            .execute(
                r#"
                UPDATE sessions
                SET title = ?1, lifecycle_state = ?2, runtime_session_id = ?3,
                    category_id = ?4, pinned = ?5, archived = ?6, updated_at = ?7,
                    remote_ssh_connection_id = ?8, remote_ssh_connection_revision = ?9,
                    seats = ?10, agent_id = ?11, recovery_status = ?12,
                    recovery_revision = ?13, state_revision = ?14, history_revision = ?15,
                    active_execution_run_id = ?16, next_message_sequence = ?17
                WHERE id = ?18 AND (?19 IS NULL OR updated_at = ?19)
                "#,
                params![
                    session.aggregate.title().as_str(),
                    session.aggregate.lifecycle().as_str(),
                    session.runtime_session_id,
                    session.aggregate.category_id().map(CategoryId::as_str),
                    i64::from(session.aggregate.is_pinned()),
                    i64::from(session.aggregate.is_archived()),
                    session.updated_at,
                    session
                        .workspace
                        .remote_ssh_binding
                        .as_ref()
                        .map(|binding| binding.connection_id.as_str()),
                    session
                        .workspace
                        .remote_ssh_binding
                        .as_ref()
                        .map(|binding| binding.revision),
                    encode_seats(&session.seats),
                    session.agent_id,
                    session.aggregate.recovery().status().as_str(),
                    session.aggregate.recovery().recovery_revision() as i64,
                    session.aggregate.recovery().state_revision() as i64,
                    session.aggregate.recovery().history_revision() as i64,
                    session.aggregate.recovery().active_execution_run_id(),
                    session.aggregate.recovery().next_message_sequence() as i64,
                    session.id(),
                    expected_updated_at,
                ],
            )
            .map_err(repository_error)?;
        if changed == 0 {
            return Ok(None);
        }
        load_session(&connection, session.aggregate.id())
    }
}

impl SqliteSessionsRepository {
    fn list_with_loop_visibility(
        &self,
        scope: SessionListScope,
        include_loop_owned: bool,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        let connection = self.connection()?;
        let archived = i64::from(scope == SessionListScope::Archived);
        let loop_filter = if include_loop_owned {
            ""
        } else {
            " AND loop_run_id IS NULL"
        };
        let mut statement = connection
            .prepare(&format!(
                "{SESSION_SELECT} WHERE archived = ?1{loop_filter} ORDER BY pinned DESC, updated_at DESC"
            ))
            .map_err(repository_error)?;
        let records = statement
            .query_map([archived], SessionRow::read)
            .map_err(repository_error)?
            .map(|row| {
                row.map_err(repository_error)
                    .and_then(SessionRow::into_record)
            })
            .collect();
        records
    }

    fn query_sessions(
        &self,
        condition: &str,
        parameter: Option<&str>,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!("{SESSION_SELECT} {condition}"))
            .map_err(repository_error)?;
        let rows = match parameter {
            Some(value) => statement
                .query_map([value], SessionRow::read)
                .map_err(repository_error)?
                .collect::<Result<Vec<_>, _>>(),
            None => statement
                .query_map([], SessionRow::read)
                .map_err(repository_error)?
                .collect::<Result<Vec<_>, _>>(),
        }
        .map_err(repository_error)?;
        rows.into_iter().map(SessionRow::into_record).collect()
    }
}

impl SessionMessageRepository for SqliteSessionsRepository {
    fn find(
        &self,
        message_id: &MessageId,
    ) -> Result<Option<MessageRecord>, SessionsApplicationError> {
        load_message(&*self.connection()?, message_id)
    }

    fn insert(&self, message: &MessageRecord) -> Result<MessageRecord, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        let session_sequence =
            allocate_message_sequences(&transaction, message.message.session_id(), 1)?;
        insert_message(&transaction, message, session_sequence)?;
        transaction.commit().map_err(repository_error)?;
        load_message(&connection, message.message.id())?.ok_or_else(|| {
            SessionsApplicationError::MessageNotFound(message.message.id().as_str().to_string())
        })
    }

    fn save(&self, message: &MessageRecord) -> Result<MessageRecord, SessionsApplicationError> {
        let connection = self.connection()?;
        update_message(&connection, message)?;
        load_message(&connection, message.message.id())?.ok_or_else(|| {
            SessionsApplicationError::MessageNotFound(message.message.id().as_str().to_string())
        })
    }

    fn save_stream_fields(&self, message: &MessageRecord) -> Result<(), SessionsApplicationError> {
        let changed = self
            .connection()?
            .execute(
                r#"
                UPDATE messages
                SET content = ?1, thinking_content = ?2, tool_use = ?3,
                    rich_blocks = ?4, updated_at = ?5
                WHERE id = ?6 AND session_id = ?7
                "#,
                params![
                    message.content,
                    message.thinking_content,
                    json_values(message.tool_use.as_ref())?,
                    json_values(message.rich_blocks.as_ref())?,
                    message.updated_at,
                    message.message.id().as_str(),
                    message.message.session_id().as_str(),
                ],
            )
            .map_err(repository_error)?;
        if changed == 0 {
            Err(SessionsApplicationError::MessageNotFound(
                message.message.id().as_str().to_string(),
            ))
        } else {
            Ok(())
        }
    }

    fn list(
        &self,
        query: &MessagePageQuery,
    ) -> Result<Vec<MessageRecord>, SessionsApplicationError> {
        let connection = self.connection()?;
        let rows = if let Some(before_id) = &query.before_id {
            let mut statement = connection
                .prepare(&format!(
                    "{MESSAGE_SELECT} WHERE session_id = ?1
                     AND session_sequence < (
                         SELECT session_sequence FROM messages WHERE id = ?2 AND session_id = ?1
                     )
                     ORDER BY session_sequence DESC LIMIT ?3"
                ))
                .map_err(repository_error)?;
            let rows = statement
                .query_map(
                    params![query.session_id, before_id, query.limit as i64],
                    MessageRow::read,
                )
                .map_err(repository_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(repository_error)?;
            rows
        } else {
            let mut statement = connection
                .prepare(&format!(
                    "{MESSAGE_SELECT} WHERE session_id = ?1 ORDER BY session_sequence DESC LIMIT ?2"
                ))
                .map_err(repository_error)?;
            let rows = statement
                .query_map(
                    params![query.session_id, query.limit as i64],
                    MessageRow::read,
                )
                .map_err(repository_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(repository_error)?;
            rows
        };
        let mut records = rows
            .into_iter()
            .map(MessageRow::into_record)
            .collect::<Result<Vec<_>, _>>()?;
        records.reverse();
        Ok(records)
    }

    fn list_all(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<MessageRecord>, SessionsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!(
                "{MESSAGE_SELECT} WHERE session_id = ?1 ORDER BY session_sequence ASC"
            ))
            .map_err(repository_error)?;
        let records = statement
            .query_map([session_id.as_str()], MessageRow::read)
            .map_err(repository_error)?
            .map(|row| {
                row.map_err(repository_error)
                    .and_then(MessageRow::into_record)
            })
            .collect();
        records
    }
}

impl SessionRecoveryReportRepository for SqliteSessionsRepository {
    #[cfg(test)]
    fn insert_report(
        &self,
        report: &SessionRecoveryReport,
    ) -> Result<(), SessionsApplicationError> {
        let reason_codes = serde_json::to_string(report.reason_codes())
            .map_err(|error| SessionsApplicationError::Serialization(error.to_string()))?;
        let evidence_refs = serde_json::to_string(report.evidence_refs())
            .map_err(|error| SessionsApplicationError::Serialization(error.to_string()))?;
        self.connection()?
            .execute(
                r#"
                INSERT INTO session_recovery_reports (
                    report_id, session_id, recovery_revision, trigger,
                    observed_lifecycle, observed_execution_run_id, decision,
                    reason_codes_json, evidence_refs_json, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
                params![
                    report.report_id(),
                    report.session_id(),
                    report.recovery_revision() as i64,
                    enum_storage_value(report.trigger())?,
                    report.observed_lifecycle(),
                    report.observed_execution_run_id(),
                    enum_storage_value(report.decision())?,
                    reason_codes,
                    evidence_refs,
                    report.created_at(),
                ],
            )
            .map_err(repository_error)?;
        Ok(())
    }

    fn list_reports(
        &self,
        session_id: &SessionId,
        limit: usize,
    ) -> Result<Vec<SessionRecoveryReport>, SessionsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT report_id, session_id, recovery_revision, trigger,
                       observed_lifecycle, observed_execution_run_id, decision,
                       reason_codes_json, evidence_refs_json, created_at
                FROM session_recovery_reports
                WHERE session_id = ?1
                ORDER BY recovery_revision DESC
                LIMIT ?2
                "#,
            )
            .map_err(repository_error)?;
        let rows = statement
            .query_map(params![session_id.as_str(), limit.min(100) as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;

        rows.into_iter()
            .map(
                |(
                    report_id,
                    session_id,
                    recovery_revision,
                    trigger,
                    observed_lifecycle,
                    observed_execution_run_id,
                    decision,
                    reason_codes,
                    evidence_refs,
                    created_at,
                )| {
                    Ok(SessionRecoveryReport::new(
                        report_id,
                        session_id,
                        u64::try_from(recovery_revision).map_err(|_| {
                            SessionsApplicationError::Repository(
                                "invalid negative recovery report revision".to_string(),
                            )
                        })?,
                        enum_from_storage::<RecoveryTrigger>(&trigger)?,
                        observed_lifecycle,
                        observed_execution_run_id,
                        enum_from_storage::<RecoveryDecision>(&decision)?,
                        serde_json::from_str::<Vec<RecoveryReasonCode>>(&reason_codes).map_err(
                            |error| SessionsApplicationError::Serialization(error.to_string()),
                        )?,
                        serde_json::from_str::<Vec<RecoveryEvidenceReference>>(&evidence_refs)
                            .map_err(|error| {
                                SessionsApplicationError::Serialization(error.to_string())
                            })?,
                        created_at,
                    ))
                },
            )
            .collect()
    }
}

pub(super) fn enum_storage_value<T: serde::Serialize>(
    value: T,
) -> Result<String, SessionsApplicationError> {
    serde_json::to_value(value)
        .map_err(|error| SessionsApplicationError::Serialization(error.to_string()))?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| {
            SessionsApplicationError::Serialization(
                "recovery enum did not serialize as a string".to_string(),
            )
        })
}

fn enum_from_storage<T: serde::de::DeserializeOwned>(
    value: &str,
) -> Result<T, SessionsApplicationError> {
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|error| SessionsApplicationError::Serialization(error.to_string()))
}

impl SessionCategoryRepository for SqliteSessionsRepository {
    fn list(&self) -> Result<Vec<CategoryRecord>, SessionsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!(
                "{CATEGORY_SELECT} ORDER BY sort_order ASC, name ASC"
            ))
            .map_err(repository_error)?;
        let records = statement
            .query_map([], CategoryRow::read)
            .map_err(repository_error)?
            .map(|row| {
                row.map_err(repository_error)
                    .and_then(CategoryRow::into_record)
            })
            .collect();
        records
    }

    fn find(
        &self,
        category_id: &CategoryId,
    ) -> Result<Option<CategoryRecord>, SessionsApplicationError> {
        load_category(&*self.connection()?, category_id)
    }

    fn name_exists(
        &self,
        name: &str,
        excluding: Option<&CategoryId>,
    ) -> Result<bool, SessionsApplicationError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1 FROM session_categories
                    WHERE LOWER(name) = LOWER(?1) AND (?2 IS NULL OR id != ?2)
                 )",
                params![name, excluding.map(CategoryId::as_str)],
                |row| row.get::<_, i64>(0),
            )
            .map(|exists| exists != 0)
            .map_err(repository_error)
    }

    fn next_sort_order(&self) -> Result<i64, SessionsApplicationError> {
        self.connection()?
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM session_categories",
                [],
                |row| row.get(0),
            )
            .map_err(repository_error)
    }

    fn insert(
        &self,
        category: &CategoryRecord,
    ) -> Result<CategoryRecord, SessionsApplicationError> {
        let connection = self.connection()?;
        connection
            .execute(
                "INSERT INTO session_categories (id, name, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    category.category.id().as_str(),
                    category.category.name().as_str(),
                    category.category.sort_order(),
                    category.created_at,
                    category.updated_at,
                ],
            )
            .map_err(repository_error)?;
        load_category(&connection, category.category.id())?.ok_or_else(|| {
            SessionsApplicationError::CategoryNotFound(category.category.id().as_str().to_string())
        })
    }

    fn save(&self, category: &CategoryRecord) -> Result<CategoryRecord, SessionsApplicationError> {
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "UPDATE session_categories SET name = ?1, sort_order = ?2, updated_at = ?3 WHERE id = ?4",
                params![
                    category.category.name().as_str(),
                    category.category.sort_order(),
                    category.updated_at,
                    category.category.id().as_str(),
                ],
            )
            .map_err(repository_error)?;
        if changed == 0 {
            return Err(SessionsApplicationError::CategoryNotFound(
                category.category.id().as_str().to_string(),
            ));
        }
        load_category(&connection, category.category.id())?.ok_or_else(|| {
            SessionsApplicationError::CategoryNotFound(category.category.id().as_str().to_string())
        })
    }
}

impl SessionConfigurationRepository for SqliteSessionsRepository {
    fn load(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<ChatConfigurationValues>, SessionsApplicationError> {
        let raw = self
            .connection()?
            .query_row(
                "SELECT chat_preferences FROM sessions WHERE id = ?1",
                [session_id.as_str()],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(repository_error)?
            .flatten();
        Ok(raw.as_deref().and_then(deserialize_configuration))
    }

    fn save(
        &self,
        session_id: &SessionId,
        preferences: &ChatPreferences,
        updated_at: &str,
    ) -> Result<(), SessionsApplicationError> {
        let values = ChatConfigurationValues::from_preferences(preferences);
        let changed = self
            .connection()?
            .execute(
                "UPDATE sessions SET chat_preferences = ?1, updated_at = ?2 WHERE id = ?3",
                params![
                    serialize_configuration(&values)?,
                    updated_at,
                    session_id.as_str()
                ],
            )
            .map_err(repository_error)?;
        if changed == 0 {
            Err(SessionsApplicationError::SessionNotFound(
                session_id.as_str().to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

pub(super) fn insert_message(
    connection: &Connection,
    message: &MessageRecord,
    session_sequence: u64,
) -> Result<(), SessionsApplicationError> {
    let token_input = message.token_usage.as_ref().map(|usage| usage.input);
    let token_output = message.token_usage.as_ref().map(|usage| usage.output);
    connection
        .execute(
            r#"
            INSERT INTO messages (
                id, session_id, role, status, content, thinking_content, tool_use,
                rich_blocks, token_input, token_output, metadata, file_references,
                created_at, updated_at, seat_index, speaker_seat_id, session_sequence,
                execution_run_id, seat_round_id, parent_execution_run_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
            "#,
            params![
                message.message.id().as_str(),
                message.message.session_id().as_str(),
                message.message.role().as_str(),
                message.message.status().as_str(),
                message.content,
                message.thinking_content,
                json_values(message.tool_use.as_ref())?,
                json_values(message.rich_blocks.as_ref())?,
                token_input,
                token_output,
                message.error,
                file_references_json(message)?,
                message.created_at,
                message.updated_at,
                message.seat_index.map(|index| index as i64),
                message.speaker_seat_id,
                session_sequence as i64,
                message.message.execution_run_id(),
                message.seat_round_id,
                message.parent_execution_run_id,
            ],
        )
        .map_err(repository_error)?;
    Ok(())
}

pub(super) fn allocate_message_sequences(
    connection: &Connection,
    session_id: &SessionId,
    count: u64,
) -> Result<u64, SessionsApplicationError> {
    if count == 0 {
        return Err(SessionsApplicationError::Transaction(
            "message sequence allocation count must be positive".to_string(),
        ));
    }
    let count = i64::try_from(count).map_err(|_| {
        SessionsApplicationError::Transaction(
            "message sequence allocation count exceeds SQLite range".to_string(),
        )
    })?;
    connection
        .query_row(
            r#"
            UPDATE sessions
            SET next_message_sequence = next_message_sequence + ?2,
                history_revision = history_revision + ?2
            WHERE id = ?1
            RETURNING next_message_sequence - ?2
            "#,
            params![session_id.as_str(), count],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(repository_error)?
        .ok_or_else(|| SessionsApplicationError::SessionNotFound(session_id.as_str().to_string()))
        .and_then(|value| {
            u64::try_from(value).map_err(|_| {
                SessionsApplicationError::Repository(
                    "stored message sequence is outside the supported range".to_string(),
                )
            })
        })
}

pub(super) fn update_message(
    connection: &Connection,
    message: &MessageRecord,
) -> Result<(), SessionsApplicationError> {
    let token_input = message.token_usage.as_ref().map(|usage| usage.input);
    let token_output = message.token_usage.as_ref().map(|usage| usage.output);
    let changed = connection
        .execute(
            r#"
            UPDATE messages
            SET role = ?1, status = ?2, content = ?3, thinking_content = ?4,
                tool_use = ?5, rich_blocks = ?6, token_input = ?7, token_output = ?8,
                metadata = ?9, file_references = ?10, updated_at = ?11,
                speaker_seat_id = ?12
            WHERE id = ?13 AND session_id = ?14
            "#,
            params![
                message.message.role().as_str(),
                message.message.status().as_str(),
                message.content,
                message.thinking_content,
                json_values(message.tool_use.as_ref())?,
                json_values(message.rich_blocks.as_ref())?,
                token_input,
                token_output,
                message.error,
                file_references_json(message)?,
                message.updated_at,
                message.speaker_seat_id,
                message.message.id().as_str(),
                message.message.session_id().as_str(),
            ],
        )
        .map_err(repository_error)?;
    if changed == 0 {
        Err(SessionsApplicationError::MessageNotFound(
            message.message.id().as_str().to_string(),
        ))
    } else {
        Ok(())
    }
}

fn search_matches(
    session: &SessionRecord,
    query: &str,
    message_match: Option<(String, String)>,
) -> Vec<SessionSearchMatch> {
    let mut matches = Vec::new();
    if contains_case_insensitive(Some(session.aggregate.title().as_str()), query) {
        matches.push(SessionSearchMatch {
            kind: SessionSearchMatchKind::Title,
            excerpt: session.aggregate.title().as_str().to_string(),
            message_id: None,
        });
    }
    let workspace = &session.workspace;
    let remote = workspace.remote_workspace.as_ref();
    let project_values = [
        workspace.folder.as_deref(),
        workspace.project_path.as_deref(),
        workspace.worktree_path.as_deref(),
        workspace.worktree_name.as_deref(),
        workspace.worktree_branch.as_deref(),
        remote.map(|workspace| workspace.host.as_str()),
        remote.and_then(|workspace| workspace.user.as_deref()),
        remote.map(|workspace| workspace.path.as_str()),
        remote.map(|workspace| workspace.display_name.as_str()),
        remote.map(|workspace| workspace.uri.as_str()),
    ];
    if let Some(value) = project_values
        .into_iter()
        .flatten()
        .find(|value| contains_case_insensitive(Some(value), query))
    {
        matches.push(SessionSearchMatch {
            kind: SessionSearchMatchKind::Project,
            excerpt: value.to_string(),
            message_id: None,
        });
    }
    if let Some((message_id, content)) = message_match {
        matches.push(SessionSearchMatch {
            kind: SessionSearchMatchKind::Message,
            excerpt: bounded_excerpt(&content, query),
            message_id: Some(message_id),
        });
    }
    matches
}

/// Below this length the trigram tokenizer emits no tokens at all, so an FTS `MATCH`
/// cannot match anything and the compatibility statement is required for correctness.
const MIN_INDEXED_QUERY_CHARS: usize = 3;

/// Session-metadata predicates, kept in one place so the two search statements cannot
/// drift apart on which fields a query is matched against.
const SESSION_SEARCH_METADATA_PREDICATES: &str = r"sessions.title LIKE ?2 ESCAPE '\'
                   OR COALESCE(sessions.project_path, '') LIKE ?2 ESCAPE '\'
                   OR COALESCE(sessions.folder, '') LIKE ?2 ESCAPE '\'
                   OR COALESCE(sessions.worktree_path, '') LIKE ?2 ESCAPE '\'
                   OR COALESCE(sessions.worktree_name, '') LIKE ?2 ESCAPE '\'
                   OR COALESCE(sessions.worktree_branch, '') LIKE ?2 ESCAPE '\'
                   OR COALESCE(sessions.remote_workspace_host, '') LIKE ?2 ESCAPE '\'
                   OR COALESCE(sessions.remote_workspace_user, '') LIKE ?2 ESCAPE '\'
                   OR COALESCE(sessions.remote_workspace_path, '') LIKE ?2 ESCAPE '\'
                   OR COALESCE(sessions.remote_workspace_display_name, '') LIKE ?2 ESCAPE '\'
                   OR COALESCE(sessions.remote_workspace_uri, '') LIKE ?2 ESCAPE '\'";

/// Ranks the FTS match set and keeps the newest match per session.
///
/// Materializing the match set is affordable here because `MATCH` prunes to matching rows
/// through the index first. Correlating it per session instead would re-run the full-text
/// query once for every candidate.
pub(super) fn indexed_search_statement() -> String {
    format!(
        r#"
                WITH ranked_message_matches AS (
                    SELECT messages.session_id, messages.id, messages.content,
                           ROW_NUMBER() OVER (
                               PARTITION BY messages.session_id
                               ORDER BY messages.session_sequence DESC
                           ) AS match_rank
                    FROM session_message_fts
                    JOIN messages ON messages.rowid = session_message_fts.rowid
                    WHERE session_message_fts MATCH ?1
                ),
                message_matches AS (
                    SELECT session_id, id, content
                    FROM ranked_message_matches
                    WHERE match_rank = 1
                )
                {SESSION_SEARCH_SELECT}
                LEFT JOIN message_matches ON message_matches.session_id = sessions.id
                WHERE sessions.loop_run_id IS NULL
                  AND ({SESSION_SEARCH_METADATA_PREDICATES}
                   OR message_matches.id IS NOT NULL)
                ORDER BY sessions.updated_at DESC
                LIMIT ?3
                "#
    )
}

/// Resolves each candidate session's newest matching message with a correlated lookup.
///
/// Without an index to prune with, ranking every match would mean sorting the entire scan
/// before the outer `LIMIT` could apply. The correlated form drives
/// `idx_messages_session_created` and stops at the first match per session instead, and it
/// selects the same message the ranking form would: the ordering key, `rowid` tiebreak
/// included, is identical.
pub(super) fn compatibility_search_statement() -> String {
    format!(
        r#"
                {SESSION_SEARCH_SELECT}
                LEFT JOIN messages AS message_matches
                       ON message_matches.id = (
                              SELECT candidate.id
                              FROM messages AS candidate
                              WHERE candidate.session_id = sessions.id
                                AND candidate.content LIKE ?1 ESCAPE '\'
                              ORDER BY candidate.session_sequence DESC
                              LIMIT 1
                          )
                WHERE sessions.loop_run_id IS NULL
                  AND ({SESSION_SEARCH_METADATA_PREDICATES}
                   OR message_matches.id IS NOT NULL)
                ORDER BY sessions.updated_at DESC
                LIMIT ?3
                "#
    )
}

fn fts_literal(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}

fn like_pattern(query: &str) -> String {
    let escaped = query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

fn contains_case_insensitive(value: Option<&str>, query: &str) -> bool {
    value
        .map(|value| value.to_lowercase().contains(&query.to_lowercase()))
        .unwrap_or(false)
}

fn bounded_excerpt(content: &str, query: &str) -> String {
    const MAX_EXCERPT_CHARS: usize = 160;
    let lower_content = content.to_lowercase();
    let lower_query = query.to_lowercase();
    let start = lower_content
        .find(&lower_query)
        .map(|index| index.saturating_sub(40))
        .unwrap_or(0);
    content
        .chars()
        .skip(start)
        .take(MAX_EXCERPT_CHARS)
        .collect()
}
