use super::rows::{load_message, load_session, recovery_repository_error, repository_error};
use super::sqlite_repository::{enum_storage_value, insert_message, update_message};
use super::usage_accounting::persist_completed_invocation;
use super::SqliteSessionsRepository;
use crate::contexts::sessions::application::{
    AcknowledgeRecoveryRequest, AcknowledgeRecoveryResult, ClaimRecoveryCandidateRequest,
    GenerationStartRequest, GenerationStartResult, GenerationTerminalRequest,
    GenerationTerminalResult, MessageRecord, MessageUsageRecord, PublishRecoveryRequest,
    RecoveryCandidateClaim, SessionRecord, SessionTransactionPort, SessionsApplicationError,
};
use crate::contexts::sessions::domain::recovery::{
    RecoveryDecision, RecoveryEvidenceReference, RecoveryReasonCode, RecoveryTrigger,
    SessionRecoveryReport,
};
use crate::contexts::sessions::domain::{
    encode_seats, CategoryId, MessageRole, SessionActivation, SessionId,
};
use rusqlite::{params, OptionalExtension, Transaction};

impl SessionTransactionPort for SqliteSessionsRepository {
    fn acknowledge_recovery(
        &self,
        request: &AcknowledgeRecoveryRequest,
    ) -> Result<AcknowledgeRecoveryResult, SessionsApplicationError> {
        let session_id = SessionId::parse(&request.session_id)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        let current = load_session(&transaction, &session_id)?
            .ok_or_else(|| SessionsApplicationError::SessionNotFound(request.session_id.clone()))?;
        let recovery = current.aggregate.recovery();
        let current_status = recovery.status().as_str().to_string();
        if recovery.recovery_revision() != request.expected_recovery_revision {
            return Err(SessionsApplicationError::RecoveryRevisionConflict {
                session_id: request.session_id.clone(),
                expected_revision: request.expected_recovery_revision,
                current_revision: recovery.recovery_revision(),
                current_status,
            });
        }
        if current_status != "action_required" {
            return Err(SessionsApplicationError::RecoveryActionNotAllowed {
                session_id: request.session_id.clone(),
                current_revision: recovery.recovery_revision(),
                current_status,
            });
        }
        let next_recovery_revision = recovery.recovery_revision() + 1;
        let report = SessionRecoveryReport::new(
            format!("recovery:{}:{next_recovery_revision}", request.session_id),
            request.session_id.clone(),
            next_recovery_revision,
            RecoveryTrigger::UserAcknowledgement,
            current.aggregate.lifecycle().as_str().to_string(),
            recovery.active_execution_run_id().map(str::to_string),
            RecoveryDecision::Acknowledged,
            vec![RecoveryReasonCode::AcknowledgedByUser],
            vec![RecoveryEvidenceReference::Session {
                session_id: request.session_id.clone(),
                state_revision: recovery.state_revision(),
                history_revision: recovery.history_revision(),
            }],
            request.acknowledged_at.clone(),
        );
        let changed = transaction
            .execute(
                r#"
                UPDATE sessions
                SET recovery_status = 'clean',
                    recovery_revision = recovery_revision + 1,
                    state_revision = state_revision + 1,
                    active_execution_run_id = NULL,
                    updated_at = ?1
                WHERE id = ?2
                  AND recovery_status = 'action_required'
                  AND recovery_revision = ?3
                  AND state_revision = ?4
                  AND history_revision = ?5
                  AND active_execution_run_id IS ?6
                "#,
                params![
                    request.acknowledged_at,
                    request.session_id,
                    request.expected_recovery_revision as i64,
                    recovery.state_revision() as i64,
                    recovery.history_revision() as i64,
                    recovery.active_execution_run_id(),
                ],
            )
            .map_err(repository_error)?;
        if changed != 1 {
            let latest = load_session(&transaction, &session_id)?.ok_or_else(|| {
                SessionsApplicationError::SessionNotFound(request.session_id.clone())
            })?;
            return Err(SessionsApplicationError::RecoveryRevisionConflict {
                session_id: request.session_id.clone(),
                expected_revision: request.expected_recovery_revision,
                current_revision: latest.aggregate.recovery().recovery_revision(),
                current_status: latest.aggregate.recovery().status().as_str().to_string(),
            });
        }
        insert_recovery_report(&transaction, &report)?;
        transaction.commit().map_err(repository_error)?;
        let session = load_session(&connection, &session_id)?
            .ok_or_else(|| SessionsApplicationError::SessionNotFound(request.session_id.clone()))?;
        Ok(AcknowledgeRecoveryResult { session, report })
    }

    fn start_generation(
        &self,
        request: &GenerationStartRequest,
    ) -> Result<GenerationStartResult, SessionsApplicationError> {
        let session_id = SessionId::parse(&request.session_id)?;
        validate_generation_message(
            &request.assistant_message,
            &session_id,
            &request.execution_run_id,
        )?;
        if let Some(user_message) = &request.user_message {
            validate_generation_message(user_message, &session_id, &request.execution_run_id)?;
        }
        let message_count = 1_i64 + i64::from(request.user_message.is_some());
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        let first_sequence = transaction
            .query_row(
                r#"
                UPDATE sessions
                SET active_execution_run_id = ?2,
                    lifecycle_state = 'starting',
                    next_message_sequence = next_message_sequence + ?3,
                    state_revision = state_revision + 1,
                    history_revision = history_revision + ?3,
                    updated_at = ?4
                WHERE id = ?1
                  AND archived = 0
                  AND recovery_status = 'clean'
                  AND active_execution_run_id IS NULL
                RETURNING next_message_sequence - ?3
                "#,
                params![
                    session_id.as_str(),
                    request.execution_run_id,
                    message_count,
                    request.started_at,
                ],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(repository_error)?;
        let Some(first_sequence) = first_sequence else {
            let exists = transaction
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sessions WHERE id = ?1)",
                    [session_id.as_str()],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(repository_error)?;
            return if exists {
                Err(SessionsApplicationError::Transaction(format!(
                    "session is unavailable for generation: {}",
                    session_id.as_str()
                )))
            } else {
                Err(SessionsApplicationError::SessionNotFound(
                    session_id.as_str().to_string(),
                ))
            };
        };
        let first_sequence = u64::try_from(first_sequence).map_err(|_| {
            SessionsApplicationError::Repository(
                "stored message sequence is outside the supported range".to_string(),
            )
        })?;
        let assistant_sequence = if let Some(user_message) = &request.user_message {
            insert_message(&transaction, user_message, first_sequence)?;
            first_sequence + 1
        } else {
            first_sequence
        };
        insert_message(&transaction, &request.assistant_message, assistant_sequence)?;
        transaction.commit().map_err(repository_error)?;

        let session = load_session(&connection, &session_id)?.ok_or_else(|| {
            SessionsApplicationError::SessionNotFound(session_id.as_str().to_string())
        })?;
        let user_message = request
            .user_message
            .as_ref()
            .map(|message| {
                load_message(&connection, message.message.id())?.ok_or_else(|| {
                    SessionsApplicationError::MessageNotFound(
                        message.message.id().as_str().to_string(),
                    )
                })
            })
            .transpose()?;
        let assistant_message = load_message(&connection, request.assistant_message.message.id())?
            .ok_or_else(|| {
                SessionsApplicationError::MessageNotFound(
                    request.assistant_message.message.id().as_str().to_string(),
                )
            })?;
        Ok(GenerationStartResult {
            session,
            user_message,
            assistant_message,
        })
    }

    fn terminalize_generation(
        &self,
        request: &GenerationTerminalRequest,
    ) -> Result<GenerationTerminalResult, SessionsApplicationError> {
        let session_id = request.message.message.session_id();
        if request.message.message.role() != MessageRole::Assistant
            || request.message.message.execution_run_id() != Some(request.execution_run_id.as_str())
            || request.message.message.status() != request.terminal_status.message_status()
        {
            return Err(SessionsApplicationError::Transaction(
                "generation terminal message does not match its durable claim".to_string(),
            ));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        let changed = transaction
            .execute(
                r#"
                UPDATE messages
                SET status = ?1
                WHERE id = ?2
                  AND session_id = ?3
                  AND execution_run_id = ?4
                  AND role = 'assistant'
                  AND status IN ('pending', 'streaming')
                "#,
                params![
                    request.terminal_status.message_status().as_str(),
                    request.message.message.id().as_str(),
                    session_id.as_str(),
                    request.execution_run_id,
                ],
            )
            .map_err(repository_error)?;
        if changed != 1 {
            return Err(SessionsApplicationError::Transaction(
                "generation terminal was stale or already applied".to_string(),
            ));
        }
        update_message(&transaction, &request.message)?;
        if let Some(usage) = &request.invocation_usage {
            persist_completed_invocation(&transaction, usage)?;
        }
        let changed = transaction
            .execute(
                r#"
                UPDATE sessions
                SET lifecycle_state = ?1,
                    active_execution_run_id = NULL,
                    state_revision = state_revision + 1,
                    history_revision = history_revision + 1,
                    updated_at = ?2
                WHERE id = ?3
                  AND active_execution_run_id = ?4
                "#,
                params![
                    request.terminal_status.lifecycle().as_str(),
                    request.finished_at,
                    session_id.as_str(),
                    request.execution_run_id,
                ],
            )
            .map_err(repository_error)?;
        if changed != 1 {
            return Err(SessionsApplicationError::Transaction(
                "generation terminal does not own the active session claim".to_string(),
            ));
        }
        transaction
            .execute(
                "UPDATE workflow_state SET lifecycle_state = ?1 WHERE active_session_id = ?2",
                params![
                    request.terminal_status.lifecycle().as_str(),
                    session_id.as_str()
                ],
            )
            .map_err(repository_error)?;
        transaction.commit().map_err(repository_error)?;

        let session = load_session(&connection, session_id)?.ok_or_else(|| {
            SessionsApplicationError::SessionNotFound(session_id.as_str().to_string())
        })?;
        let message =
            load_message(&connection, request.message.message.id())?.ok_or_else(|| {
                SessionsApplicationError::MessageNotFound(
                    request.message.message.id().as_str().to_string(),
                )
            })?;
        Ok(GenerationTerminalResult { session, message })
    }

    fn claim_recovery_candidate(
        &self,
        request: &ClaimRecoveryCandidateRequest,
    ) -> Result<Option<RecoveryCandidateClaim>, SessionsApplicationError> {
        let candidate = &request.candidate;
        let connection = self.recovery_connection()?;
        connection
            .query_row(
                r#"
                UPDATE sessions
                SET recovery_status = 'reconciling',
                    recovery_revision = CASE
                        WHEN recovery_revision < 0 THEN 0 ELSE recovery_revision
                    END,
                    state_revision = CASE
                        WHEN state_revision < 0 THEN 1 ELSE state_revision + 1
                    END,
                    history_revision = CASE
                        WHEN history_revision < 0 THEN 0 ELSE history_revision
                    END,
                    updated_at = ?1
                WHERE id = ?2
                  AND recovery_status = ?3
                  AND recovery_revision = ?4
                  AND state_revision = ?5
                  AND history_revision = ?6
                  AND active_execution_run_id IS ?7
                  AND archived = 0
                RETURNING recovery_revision, state_revision, history_revision,
                          active_execution_run_id
                "#,
                params![
                    request.claimed_at,
                    candidate.session_id,
                    candidate.previous_recovery_status,
                    candidate.captured_recovery_revision,
                    candidate.captured_state_revision,
                    candidate.captured_history_revision,
                    candidate.observed_execution_run_id,
                ],
                |row| {
                    Ok(RecoveryCandidateClaim {
                        session_id: candidate.session_id.clone(),
                        observed_lifecycle: candidate.observed_lifecycle.clone(),
                        observed_execution_run_id: row.get(3)?,
                        recovery_revision: row
                            .get::<_, i64>(0)?
                            .try_into()
                            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, i64::MIN))?,
                        state_revision: row
                            .get::<_, i64>(1)?
                            .try_into()
                            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(1, i64::MIN))?,
                        history_revision: row
                            .get::<_, i64>(2)?
                            .try_into()
                            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(2, i64::MIN))?,
                        captured_recovery_revision: row.get(0)?,
                        captured_state_revision: row.get(1)?,
                        captured_history_revision: row.get(2)?,
                        previous_recovery_status: candidate.previous_recovery_status.clone(),
                        structurally_invalid: candidate.structurally_invalid,
                    })
                },
            )
            .optional()
            .map_err(recovery_repository_error)
    }

    fn publish_recovery(
        &self,
        request: &PublishRecoveryRequest,
    ) -> Result<bool, SessionsApplicationError> {
        let projection = recovery_projection(request.report.decision(), &request.report)?;
        if request.report.session_id() != request.claim.session_id
            || request.report.recovery_revision() != request.claim.recovery_revision + 1
            || request.report.observed_execution_run_id()
                != request.claim.observed_execution_run_id.as_deref()
        {
            return Err(SessionsApplicationError::Transaction(
                "recovery report does not match its claimed candidate".to_string(),
            ));
        }
        let mut connection = self.recovery_connection()?;
        let transaction = connection
            .transaction()
            .map_err(recovery_repository_error)?;
        let mut history_increment = 0_i64;
        if let (Some(message_id), Some(message_status)) =
            (&request.assistant_message_id, projection.message_status)
        {
            history_increment = transaction
                .execute(
                    r#"
                    UPDATE messages
                    SET status = ?1,
                        metadata = CASE
                            WHEN ?1 = 'failed' AND metadata IS NULL
                            THEN 'Generation was interrupted before recovery.'
                            ELSE metadata
                        END,
                        updated_at = ?2
                    WHERE id = ?3
                      AND session_id = ?4
                      AND execution_run_id IS ?5
                      AND role = 'assistant'
                      AND status IN ('pending', 'streaming')
                    "#,
                    params![
                        message_status,
                        request.published_at,
                        message_id,
                        request.claim.session_id,
                        request.claim.observed_execution_run_id,
                    ],
                )
                .map_err(recovery_repository_error)? as i64;
        }
        let changed = transaction
            .execute(
                r#"
                UPDATE sessions
                SET lifecycle_state = ?1,
                    recovery_status = ?2,
                    recovery_revision = recovery_revision + 1,
                    state_revision = state_revision + 1,
                    history_revision = history_revision + ?3,
                    active_execution_run_id = CASE WHEN ?4 THEN NULL ELSE active_execution_run_id END,
                    updated_at = ?5
                WHERE id = ?6
                  AND recovery_status = 'reconciling'
                  AND recovery_revision = ?7
                  AND state_revision = ?8
                  AND history_revision = ?9
                  AND active_execution_run_id IS ?10
                "#,
                params![
                    projection.lifecycle,
                    projection.recovery_status,
                    history_increment,
                    projection.clear_active_run,
                    request.published_at,
                    request.claim.session_id,
                    request.claim.recovery_revision as i64,
                    request.claim.state_revision as i64,
                    request.claim.history_revision as i64,
                    request.claim.observed_execution_run_id,
                ],
            )
            .map_err(recovery_repository_error)?;
        if changed != 1 {
            return Ok(false);
        }
        insert_recovery_report(&transaction, &request.report)?;
        transaction.commit().map_err(recovery_repository_error)?;
        Ok(true)
    }

    fn defer_recovery(
        &self,
        claim: &RecoveryCandidateClaim,
        deferred_at: &str,
    ) -> Result<bool, SessionsApplicationError> {
        let connection = self.recovery_connection()?;
        let changed = connection
            .execute(
                r#"
                UPDATE sessions
                SET recovery_status = ?1,
                    state_revision = state_revision + 1,
                    updated_at = ?2
                WHERE id = ?3
                  AND recovery_status = 'reconciling'
                  AND recovery_revision = ?4
                  AND state_revision = ?5
                  AND history_revision = ?6
                  AND active_execution_run_id IS ?7
                "#,
                params![
                    claim.previous_recovery_status,
                    deferred_at,
                    claim.session_id,
                    claim.recovery_revision as i64,
                    claim.state_revision as i64,
                    claim.history_revision as i64,
                    claim.observed_execution_run_id,
                ],
            )
            .map_err(recovery_repository_error)?;
        Ok(changed == 1)
    }

    fn create_session(
        &self,
        session: &SessionRecord,
        activation: SessionActivation,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        insert_session(&transaction, session)?;
        if activation == SessionActivation::Activate {
            update_active_workflow(&transaction, session)?;
        }
        transaction.commit().map_err(repository_error)?;
        Ok(session.clone())
    }

    fn activate_session(
        &self,
        session: &SessionRecord,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        update_active_workflow(&transaction, session)?;
        transaction.commit().map_err(repository_error)?;
        Ok(session.clone())
    }

    fn archive_session(
        &self,
        session: &SessionRecord,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        update_session_state(&transaction, session)?;
        clear_active(&transaction, session.aggregate.id())?;
        transaction.commit().map_err(repository_error)?;
        Ok(session.clone())
    }

    fn clear_active_session_if_matches(
        &self,
        session_id: &SessionId,
    ) -> Result<(), SessionsApplicationError> {
        self.connection()?
            .execute(
                "UPDATE workflow_state SET active_session_id = NULL WHERE id = 1 AND active_session_id = ?1",
                [session_id.as_str()],
            )
            .map_err(repository_error)?;
        Ok(())
    }

    fn delete_session(&self, session_id: &SessionId) -> Result<(), SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        let changed = transaction
            .execute("DELETE FROM sessions WHERE id = ?1", [session_id.as_str()])
            .map_err(repository_error)?;
        if changed == 0 {
            return Err(SessionsApplicationError::SessionNotFound(
                session_id.as_str().to_string(),
            ));
        }
        clear_active(&transaction, session_id)?;
        transaction.commit().map_err(repository_error)
    }

    fn delete_category(
        &self,
        category_id: &CategoryId,
        updated_at: &str,
    ) -> Result<(), SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        transaction
            .execute(
                "UPDATE sessions SET category_id = NULL, updated_at = ?1 WHERE category_id = ?2",
                params![updated_at, category_id.as_str()],
            )
            .map_err(repository_error)?;
        let changed = transaction
            .execute(
                "DELETE FROM session_categories WHERE id = ?1",
                [category_id.as_str()],
            )
            .map_err(repository_error)?;
        if changed == 0 {
            return Err(SessionsApplicationError::CategoryNotFound(
                category_id.as_str().to_string(),
            ));
        }
        transaction.commit().map_err(repository_error)
    }

    fn complete_message(
        &self,
        message: &MessageRecord,
        _usage: Option<&MessageUsageRecord>,
        invocation_usage: Option<
            &crate::contexts::sessions::application::CompletedInvocationAccounting,
        >,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        update_message(&transaction, message)?;
        if let Some(usage) = invocation_usage {
            persist_completed_invocation(&transaction, usage)?;
        }
        transaction.commit().map_err(repository_error)?;
        Ok(message.clone())
    }

    fn save_runtime_session(
        &self,
        session: &SessionRecord,
    ) -> Result<SessionRecord, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        update_session_state(&transaction, session)?;
        transaction
            .execute(
                "UPDATE workflow_state SET lifecycle_state = ?1 WHERE active_session_id = ?2",
                params![session.aggregate.lifecycle().as_str(), session.id()],
            )
            .map_err(repository_error)?;
        transaction.commit().map_err(repository_error)?;
        Ok(session.clone())
    }

    fn cancel_messages(
        &self,
        messages: &[MessageRecord],
    ) -> Result<Vec<String>, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        let mut cancelled = Vec::new();
        for message in messages {
            let changed = transaction
                .execute(
                    "UPDATE messages SET status = 'cancelled', updated_at = ?1 WHERE id = ?2 AND session_id = ?3 AND status = 'streaming'",
                    params![
                        message.updated_at,
                        message.message.id().as_str(),
                        message.message.session_id().as_str(),
                    ],
                )
                .map_err(repository_error)?;
            if changed == 1 {
                cancelled.push(message.message.id().as_str().to_string());
            }
        }
        transaction.commit().map_err(repository_error)?;
        Ok(cancelled)
    }
}

struct RecoveryProjection {
    lifecycle: String,
    recovery_status: &'static str,
    message_status: Option<&'static str>,
    clear_active_run: bool,
}

fn recovery_projection(
    decision: RecoveryDecision,
    report: &SessionRecoveryReport,
) -> Result<RecoveryProjection, SessionsApplicationError> {
    let projection = match decision {
        RecoveryDecision::Completed => RecoveryProjection {
            lifecycle: "idle".to_string(),
            recovery_status: "clean",
            message_status: Some("completed"),
            clear_active_run: true,
        },
        RecoveryDecision::Failed | RecoveryDecision::InterruptedWithoutToolAmbiguity => {
            RecoveryProjection {
                lifecycle: "failed".to_string(),
                recovery_status: "clean",
                message_status: Some("failed"),
                clear_active_run: true,
            }
        }
        RecoveryDecision::Cancelled => RecoveryProjection {
            lifecycle: "stopped".to_string(),
            recovery_status: "clean",
            message_status: Some("cancelled"),
            clear_active_run: true,
        },
        RecoveryDecision::ActionRequired => RecoveryProjection {
            lifecycle: report.observed_lifecycle().to_string(),
            recovery_status: "action_required",
            message_status: None,
            clear_active_run: false,
        },
        RecoveryDecision::Quarantined => RecoveryProjection {
            lifecycle: report.observed_lifecycle().to_string(),
            recovery_status: "quarantined",
            message_status: None,
            clear_active_run: false,
        },
        RecoveryDecision::RetryLater | RecoveryDecision::Acknowledged => {
            return Err(SessionsApplicationError::Transaction(
                "retry and acknowledgement decisions use separate transactions".to_string(),
            ));
        }
    };
    Ok(projection)
}

fn insert_recovery_report(
    transaction: &Transaction<'_>,
    report: &SessionRecoveryReport,
) -> Result<(), SessionsApplicationError> {
    let reason_codes = serde_json::to_string(report.reason_codes()).map_err(|error| {
        SessionsApplicationError::Repository(format!(
            "failed to serialize recovery reasons: {error}"
        ))
    })?;
    let evidence_refs = serde_json::to_string(report.evidence_refs()).map_err(|error| {
        SessionsApplicationError::Repository(format!(
            "failed to serialize recovery evidence references: {error}"
        ))
    })?;
    transaction
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
        .map_err(recovery_repository_error)?;
    Ok(())
}

fn validate_generation_message(
    message: &MessageRecord,
    session_id: &SessionId,
    execution_run_id: &str,
) -> Result<(), SessionsApplicationError> {
    message.message.ensure_owned_by(session_id)?;
    if message.message.execution_run_id() != Some(execution_run_id) {
        return Err(SessionsApplicationError::Transaction(
            "generation message execution correlation does not match the claim".to_string(),
        ));
    }
    Ok(())
}

fn insert_session(
    transaction: &Transaction<'_>,
    session: &SessionRecord,
) -> Result<(), SessionsApplicationError> {
    let remote = session.workspace.remote_workspace.as_ref();
    transaction
        .execute(
            r#"
            INSERT INTO sessions (
                id, title, agent_id, interaction_mode, lifecycle_state, folder,
                project_path, worktree_path, worktree_name, worktree_branch,
                remote_workspace_host, remote_workspace_port, remote_workspace_user,
                remote_workspace_path, remote_workspace_display_name, remote_workspace_uri,
                remote_ssh_connection_id, remote_ssh_connection_revision, runtime_session_id,
                category_id, source_kind, source_connector, pinned, archived,
                created_at, updated_at, loop_run_id, loop_iteration_id, loop_role, seats,
                recovery_status, recovery_revision, state_revision, history_revision,
                active_execution_run_id, next_message_sequence
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23,
                ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33,
                ?34, ?35, ?36
            )
            "#,
            params![
                session.id(),
                session.aggregate.title().as_str(),
                session.agent_id,
                session.interaction_mode,
                session.aggregate.lifecycle().as_str(),
                session.workspace.folder,
                session.workspace.project_path,
                session.workspace.worktree_path,
                session.workspace.worktree_name,
                session.workspace.worktree_branch,
                remote.map(|workspace| workspace.host.as_str()),
                remote.and_then(|workspace| workspace.port.map(i64::from)),
                remote.and_then(|workspace| workspace.user.as_deref()),
                remote.map(|workspace| workspace.path.as_str()),
                remote.map(|workspace| workspace.display_name.as_str()),
                remote.map(|workspace| workspace.uri.as_str()),
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
                session.runtime_session_id,
                session.aggregate.category_id().map(CategoryId::as_str),
                session.aggregate.owner().kind(),
                session.aggregate.owner().connector_id(),
                i64::from(session.aggregate.is_pinned()),
                i64::from(session.aggregate.is_archived()),
                session.created_at,
                session.updated_at,
                session
                    .workspace
                    .loop_ownership
                    .as_ref()
                    .map(|ownership| ownership.run_id.as_str()),
                session
                    .workspace
                    .loop_ownership
                    .as_ref()
                    .map(|ownership| ownership.iteration_id.as_str()),
                session
                    .workspace
                    .loop_ownership
                    .as_ref()
                    .map(|ownership| ownership.role.as_str()),
                encode_seats(&session.seats),
                session.aggregate.recovery().status().as_str(),
                session.aggregate.recovery().recovery_revision() as i64,
                session.aggregate.recovery().state_revision() as i64,
                session.aggregate.recovery().history_revision() as i64,
                session.aggregate.recovery().active_execution_run_id(),
                session.aggregate.recovery().next_message_sequence() as i64,
            ],
        )
        .map_err(repository_error)?;
    Ok(())
}

fn update_active_workflow(
    transaction: &Transaction<'_>,
    session: &SessionRecord,
) -> Result<(), SessionsApplicationError> {
    let changed = transaction
        .execute(
            r#"
            UPDATE workflow_state
            SET active_session_id = ?1, active_agent_id = ?2,
                active_interaction_mode = ?3, lifecycle_state = ?4
            WHERE id = 1
            "#,
            params![
                session.id(),
                session.agent_id,
                session.interaction_mode,
                session.aggregate.lifecycle().as_str(),
            ],
        )
        .map_err(repository_error)?;
    if changed == 1 {
        Ok(())
    } else {
        Err(SessionsApplicationError::Transaction(
            "active workflow row is unavailable".to_string(),
        ))
    }
}

fn update_session_state(
    transaction: &Transaction<'_>,
    session: &SessionRecord,
) -> Result<(), SessionsApplicationError> {
    let changed = transaction
        .execute(
            r#"
            UPDATE sessions
            SET title = ?1, lifecycle_state = ?2, runtime_session_id = ?3,
                category_id = ?4, pinned = ?5, archived = ?6, updated_at = ?7,
                recovery_status = ?8, recovery_revision = ?9, state_revision = ?10,
                history_revision = ?11, active_execution_run_id = ?12,
                next_message_sequence = ?13
            WHERE id = ?14
            "#,
            params![
                session.aggregate.title().as_str(),
                session.aggregate.lifecycle().as_str(),
                session.runtime_session_id,
                session.aggregate.category_id().map(CategoryId::as_str),
                i64::from(session.aggregate.is_pinned()),
                i64::from(session.aggregate.is_archived()),
                session.updated_at,
                session.aggregate.recovery().status().as_str(),
                session.aggregate.recovery().recovery_revision() as i64,
                session.aggregate.recovery().state_revision() as i64,
                session.aggregate.recovery().history_revision() as i64,
                session.aggregate.recovery().active_execution_run_id(),
                session.aggregate.recovery().next_message_sequence() as i64,
                session.id(),
            ],
        )
        .map_err(repository_error)?;
    if changed == 1 {
        Ok(())
    } else {
        Err(SessionsApplicationError::SessionNotFound(
            session.id().to_string(),
        ))
    }
}

fn clear_active(
    transaction: &Transaction<'_>,
    session_id: &SessionId,
) -> Result<(), SessionsApplicationError> {
    transaction
        .execute(
            "UPDATE workflow_state SET active_session_id = NULL WHERE id = 1 AND active_session_id = ?1",
            [session_id.as_str()],
        )
        .map_err(repository_error)?;
    Ok(())
}
