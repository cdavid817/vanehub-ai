use super::rows::{
    deserialize_configuration, file_references_json, json_values, load_category, load_message,
    load_session, repository_error, serialize_configuration, CategoryRow, MessageRow, SessionRow,
    CATEGORY_SELECT, MESSAGE_SELECT, SESSION_SELECT,
};
use crate::contexts::sessions::application::{
    CategoryRecord, ChatConfigurationValues, MessagePageQuery, MessageRecord,
    SessionCategoryRepository, SessionConfigurationRepository, SessionListScope,
    SessionMessageRepository, SessionRecord, SessionRepository, SessionSearchMatch,
    SessionSearchMatchKind, SessionSearchQuery, SessionSearchResult, SessionsApplicationError,
};
use crate::contexts::sessions::domain::{CategoryId, ChatPreferences, MessageId, SessionId};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, Connection, OptionalExtension};

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
        let connection = self.connection()?;
        let archived = i64::from(scope == SessionListScope::Archived);
        let mut statement = connection
            .prepare(&format!(
                "{SESSION_SELECT} WHERE archived = ?1 ORDER BY pinned DESC, updated_at DESC"
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

    fn search(
        &self,
        query: &SessionSearchQuery,
    ) -> Result<Vec<SessionSearchResult>, SessionsApplicationError> {
        let connection = self.connection()?;
        let pattern = like_pattern(&query.text);
        let mut statement = connection
            .prepare(
                r#"
                SELECT DISTINCT sessions.id
                FROM sessions
                WHERE sessions.title LIKE ?1 ESCAPE '\'
                   OR COALESCE(sessions.project_path, '') LIKE ?1 ESCAPE '\'
                   OR COALESCE(sessions.folder, '') LIKE ?1 ESCAPE '\'
                   OR COALESCE(sessions.worktree_path, '') LIKE ?1 ESCAPE '\'
                   OR COALESCE(sessions.worktree_name, '') LIKE ?1 ESCAPE '\'
                   OR COALESCE(sessions.worktree_branch, '') LIKE ?1 ESCAPE '\'
                   OR COALESCE(sessions.remote_workspace_host, '') LIKE ?1 ESCAPE '\'
                   OR COALESCE(sessions.remote_workspace_user, '') LIKE ?1 ESCAPE '\'
                   OR COALESCE(sessions.remote_workspace_path, '') LIKE ?1 ESCAPE '\'
                   OR COALESCE(sessions.remote_workspace_display_name, '') LIKE ?1 ESCAPE '\'
                   OR COALESCE(sessions.remote_workspace_uri, '') LIKE ?1 ESCAPE '\'
                   OR EXISTS (
                        SELECT 1 FROM messages
                        WHERE messages.session_id = sessions.id
                          AND messages.content LIKE ?1 ESCAPE '\'
                   )
                ORDER BY sessions.updated_at DESC
                LIMIT ?2
                "#,
            )
            .map_err(repository_error)?;
        let session_ids = statement
            .query_map(params![pattern, query.limit as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        session_ids
            .into_iter()
            .map(|session_id| {
                let session_id = SessionId::parse(session_id)?;
                let session = load_session(&connection, &session_id)?.ok_or_else(|| {
                    SessionsApplicationError::SessionNotFound(session_id.as_str().to_string())
                })?;
                let matches = search_matches(&connection, &session, &query.text, &pattern)?;
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
        let connection = self.connection()?;
        let changed = connection
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
        if changed == 0 {
            return Err(SessionsApplicationError::SessionNotFound(
                session.id().to_string(),
            ));
        }
        load_session(&connection, session.aggregate.id())?
            .ok_or_else(|| SessionsApplicationError::SessionNotFound(session.id().to_string()))
    }

    fn recoverable_sessions(&self) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        self.query_sessions(
            "WHERE lifecycle_state IN ('starting', 'running') ORDER BY updated_at ASC",
            None,
        )
    }

    fn inactive_sessions(
        &self,
        cutoff: &str,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        self.query_sessions(
            "WHERE archived = 0 AND pinned = 0 AND lifecycle_state NOT IN ('starting', 'running') AND updated_at < ?1 ORDER BY updated_at ASC",
            Some(cutoff),
        )
    }
}

impl SqliteSessionsRepository {
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
        let connection = self.connection()?;
        insert_message(&connection, message)?;
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
                     AND created_at < (
                         SELECT created_at FROM messages WHERE id = ?2 AND session_id = ?1
                     )
                     ORDER BY created_at DESC LIMIT ?3"
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
                    "{MESSAGE_SELECT} WHERE session_id = ?1 ORDER BY created_at DESC LIMIT ?2"
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
                "{MESSAGE_SELECT} WHERE session_id = ?1 ORDER BY created_at ASC"
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
) -> Result<(), SessionsApplicationError> {
    let token_input = message.token_usage.as_ref().map(|usage| usage.input);
    let token_output = message.token_usage.as_ref().map(|usage| usage.output);
    connection
        .execute(
            r#"
            INSERT INTO messages (
                id, session_id, role, status, content, thinking_content, tool_use,
                rich_blocks, token_input, token_output, metadata, file_references,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
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
            ],
        )
        .map_err(repository_error)?;
    Ok(())
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
                metadata = ?9, file_references = ?10, updated_at = ?11
            WHERE id = ?12 AND session_id = ?13
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
    connection: &Connection,
    session: &SessionRecord,
    query: &str,
    pattern: &str,
) -> Result<Vec<SessionSearchMatch>, SessionsApplicationError> {
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
    let message_match = connection
        .query_row(
            "SELECT id, content FROM messages WHERE session_id = ?1 AND content LIKE ?2 ESCAPE '\\' ORDER BY created_at DESC LIMIT 1",
            params![session.id(), pattern],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(repository_error)?;
    if let Some((message_id, content)) = message_match {
        matches.push(SessionSearchMatch {
            kind: SessionSearchMatchKind::Message,
            excerpt: bounded_excerpt(&content, query),
            message_id: Some(message_id),
        });
    }
    Ok(matches)
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
