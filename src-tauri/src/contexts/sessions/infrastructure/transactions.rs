use super::rows::repository_error;
use super::sqlite_repository::update_message;
use super::usage::upsert_usage;
use super::SqliteSessionsRepository;
use crate::contexts::sessions::application::{
    MessageRecord, MessageUsageRecord, SessionRecord, SessionTransactionPort,
    SessionsApplicationError,
};
use crate::contexts::sessions::domain::{CategoryId, SessionActivation, SessionId};
use rusqlite::{params, Transaction};

impl SessionTransactionPort for SqliteSessionsRepository {
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
        usage: Option<&MessageUsageRecord>,
    ) -> Result<MessageRecord, SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        update_message(&transaction, message)?;
        if let Some(usage) = usage {
            upsert_usage(&transaction, usage)?;
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

    fn recover_orphaned_session(
        &self,
        session: &SessionRecord,
        recovered_at: &str,
    ) -> Result<(), SessionsApplicationError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(repository_error)?;
        transaction
            .execute(
                r#"
                UPDATE messages
                SET status = 'failed',
                    metadata = COALESCE(metadata, 'Recovered after application restart.'),
                    updated_at = ?1
                WHERE session_id = ?2
                  AND role = 'assistant'
                  AND status IN ('pending', 'streaming')
                "#,
                params![recovered_at, session.id()],
            )
            .map_err(repository_error)?;
        update_session_state(&transaction, session)?;
        transaction.commit().map_err(repository_error)
    }
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
                remote_workspace_host, remote_workspace_user, remote_workspace_path,
                remote_workspace_display_name, remote_workspace_uri, runtime_session_id,
                category_id, source_kind, source_connector, pinned, archived,
                created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23
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
                remote.and_then(|workspace| workspace.user.as_deref()),
                remote.map(|workspace| workspace.path.as_str()),
                remote.map(|workspace| workspace.display_name.as_str()),
                remote.map(|workspace| workspace.uri.as_str()),
                session.runtime_session_id,
                session.aggregate.category_id().map(CategoryId::as_str),
                session.aggregate.owner().kind(),
                session.aggregate.owner().connector_id(),
                i64::from(session.aggregate.is_pinned()),
                i64::from(session.aggregate.is_archived()),
                session.created_at,
                session.updated_at,
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
                category_id = ?4, pinned = ?5, archived = ?6, updated_at = ?7
            WHERE id = ?8
            "#,
            params![
                session.aggregate.title().as_str(),
                session.aggregate.lifecycle().as_str(),
                session.runtime_session_id,
                session.aggregate.category_id().map(CategoryId::as_str),
                i64::from(session.aggregate.is_pinned()),
                i64::from(session.aggregate.is_archived()),
                session.updated_at,
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
