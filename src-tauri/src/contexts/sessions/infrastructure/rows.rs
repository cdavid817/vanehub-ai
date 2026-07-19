use crate::contexts::sessions::application::{
    CategoryRecord, ChatConfigurationValues, FileReferenceInput, MessageRecord, MessageTokenUsage,
    SessionRecord, SessionRemoteWorkspace, SessionWorkspace, SessionsApplicationError,
};
use crate::contexts::sessions::domain::{
    CategoryId, CategoryName, FileReference, FileReferenceSet, MessageId, MessageRole,
    MessageStatus, SessionAggregate, SessionCategory, SessionId, SessionLifecycle, SessionMessage,
    SessionOwner, SessionTitle,
};
use rusqlite::{Connection, OptionalExtension, Row};
use serde_json::Value;

pub(super) const SESSION_SELECT: &str = "SELECT id, title, agent_id, interaction_mode, lifecycle_state, folder, project_path, worktree_path, worktree_name, worktree_branch, remote_workspace_host, remote_workspace_user, remote_workspace_path, remote_workspace_display_name, remote_workspace_uri, runtime_session_id, category_id, source_kind, source_connector, pinned, archived, created_at, updated_at FROM sessions";
pub(super) const MESSAGE_SELECT: &str = "SELECT id, session_id, role, status, content, thinking_content, tool_use, rich_blocks, token_input, token_output, metadata, file_references, created_at, updated_at FROM messages";
pub(super) const CATEGORY_SELECT: &str =
    "SELECT id, name, sort_order, created_at, updated_at FROM session_categories";

#[derive(Debug)]
pub(super) struct SessionRow {
    id: String,
    title: String,
    agent_id: String,
    interaction_mode: String,
    lifecycle_state: String,
    folder: Option<String>,
    project_path: Option<String>,
    worktree_path: Option<String>,
    worktree_name: Option<String>,
    worktree_branch: Option<String>,
    remote_workspace_host: Option<String>,
    remote_workspace_user: Option<String>,
    remote_workspace_path: Option<String>,
    remote_workspace_display_name: Option<String>,
    remote_workspace_uri: Option<String>,
    runtime_session_id: Option<String>,
    category_id: Option<String>,
    source_kind: String,
    source_connector: Option<String>,
    pinned: bool,
    archived: bool,
    created_at: String,
    updated_at: String,
}

impl SessionRow {
    pub(super) fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            title: row.get(1)?,
            agent_id: row.get(2)?,
            interaction_mode: row.get(3)?,
            lifecycle_state: row.get(4)?,
            folder: row.get(5)?,
            project_path: row.get(6)?,
            worktree_path: row.get(7)?,
            worktree_name: row.get(8)?,
            worktree_branch: row.get(9)?,
            remote_workspace_host: row.get(10)?,
            remote_workspace_user: row.get(11)?,
            remote_workspace_path: row.get(12)?,
            remote_workspace_display_name: row.get(13)?,
            remote_workspace_uri: row.get(14)?,
            runtime_session_id: row.get(15)?,
            category_id: row.get(16)?,
            source_kind: row.get(17)?,
            source_connector: row.get(18)?,
            pinned: row.get::<_, i64>(19)? != 0,
            archived: row.get::<_, i64>(20)? != 0,
            created_at: row.get(21)?,
            updated_at: row.get(22)?,
        })
    }

    pub(super) fn into_record(self) -> Result<SessionRecord, SessionsApplicationError> {
        let remote_workspace = match (
            self.remote_workspace_host,
            self.remote_workspace_path,
            self.remote_workspace_display_name,
            self.remote_workspace_uri,
        ) {
            (Some(host), Some(path), Some(display_name), Some(uri)) => {
                Some(SessionRemoteWorkspace {
                    host,
                    user: self.remote_workspace_user,
                    path,
                    display_name,
                    uri,
                })
            }
            _ => None,
        };
        let aggregate = SessionAggregate::rehydrate(
            SessionId::parse(self.id)?,
            SessionTitle::for_creation(Some(&self.title)),
            SessionLifecycle::from_storage_lossy(&self.lifecycle_state),
            SessionOwner::from_parts(&self.source_kind, self.source_connector.as_deref())?,
            self.category_id.map(CategoryId::parse).transpose()?,
            self.pinned,
            self.archived,
        );
        Ok(SessionRecord {
            aggregate,
            agent_id: self.agent_id,
            interaction_mode: self.interaction_mode,
            workspace: SessionWorkspace {
                folder: self.folder,
                project_path: self.project_path,
                worktree_path: self.worktree_path,
                worktree_name: self.worktree_name,
                worktree_branch: self.worktree_branch,
                remote_workspace,
            },
            runtime_session_id: self.runtime_session_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(Debug)]
pub(super) struct MessageRow {
    id: String,
    session_id: String,
    role: String,
    status: String,
    content: String,
    thinking_content: Option<String>,
    tool_use: Option<String>,
    rich_blocks: Option<String>,
    token_input: i64,
    token_output: i64,
    metadata: Option<String>,
    file_references: Option<String>,
    created_at: String,
    updated_at: String,
}

impl MessageRow {
    pub(super) fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            session_id: row.get(1)?,
            role: row.get(2)?,
            status: row.get(3)?,
            content: row.get(4)?,
            thinking_content: row.get(5)?,
            tool_use: row.get(6)?,
            rich_blocks: row.get(7)?,
            token_input: row.get::<_, Option<i64>>(8)?.unwrap_or(0),
            token_output: row.get::<_, Option<i64>>(9)?.unwrap_or(0),
            metadata: row.get(10)?,
            file_references: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    }

    pub(super) fn into_record(self) -> Result<MessageRecord, SessionsApplicationError> {
        let references = self
            .file_references
            .as_deref()
            .and_then(|value| serde_json::from_str::<Vec<FileReferenceInput>>(value).ok())
            .unwrap_or_default()
            .into_iter()
            .map(|reference| {
                FileReference::new(
                    reference.id,
                    reference.path,
                    reference.name,
                    reference.size_bytes,
                    reference.content_hash,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let message = SessionMessage::rehydrate(
            MessageId::parse(self.id)?,
            SessionId::parse(self.session_id)?,
            MessageRole::parse(&self.role)?,
            MessageStatus::parse(&self.status)?,
            FileReferenceSet::new(references)?,
        );
        let token_usage =
            (self.token_input > 0 || self.token_output > 0).then_some(MessageTokenUsage {
                input: self.token_input,
                output: self.token_output,
            });
        Ok(MessageRecord {
            message,
            content: self.content,
            thinking_content: self.thinking_content,
            tool_use: parse_json_values(self.tool_use.as_deref()),
            rich_blocks: parse_json_values(self.rich_blocks.as_deref()),
            token_usage,
            error: self.metadata,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(Debug)]
pub(super) struct CategoryRow {
    id: String,
    name: String,
    sort_order: i64,
    created_at: String,
    updated_at: String,
}

impl CategoryRow {
    pub(super) fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            sort_order: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    }

    pub(super) fn into_record(self) -> Result<CategoryRecord, SessionsApplicationError> {
        Ok(CategoryRecord {
            category: SessionCategory::new(
                CategoryId::parse(self.id)?,
                CategoryName::parse(self.name)?,
                self.sort_order,
            ),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

pub(super) fn load_session(
    connection: &Connection,
    session_id: &SessionId,
) -> Result<Option<SessionRecord>, SessionsApplicationError> {
    connection
        .query_row(
            &format!("{SESSION_SELECT} WHERE id = ?1"),
            [session_id.as_str()],
            SessionRow::read,
        )
        .optional()
        .map_err(repository_error)?
        .map(SessionRow::into_record)
        .transpose()
}

pub(super) fn load_message(
    connection: &Connection,
    message_id: &MessageId,
) -> Result<Option<MessageRecord>, SessionsApplicationError> {
    connection
        .query_row(
            &format!("{MESSAGE_SELECT} WHERE id = ?1"),
            [message_id.as_str()],
            MessageRow::read,
        )
        .optional()
        .map_err(repository_error)?
        .map(MessageRow::into_record)
        .transpose()
}

pub(super) fn load_category(
    connection: &Connection,
    category_id: &CategoryId,
) -> Result<Option<CategoryRecord>, SessionsApplicationError> {
    connection
        .query_row(
            &format!("{CATEGORY_SELECT} WHERE id = ?1"),
            [category_id.as_str()],
            CategoryRow::read,
        )
        .optional()
        .map_err(repository_error)?
        .map(CategoryRow::into_record)
        .transpose()
}

pub(super) fn file_references_json(
    message: &MessageRecord,
) -> Result<Option<String>, SessionsApplicationError> {
    let references = message.message.file_references();
    if references.as_slice().is_empty() {
        return Ok(None);
    }
    let values = references
        .as_slice()
        .iter()
        .map(|reference| FileReferenceInput {
            id: reference.id().to_string(),
            path: reference.path().to_string(),
            name: reference.name().to_string(),
            size_bytes: reference.size_bytes(),
            content_hash: reference.content_hash().map(str::to_string),
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&values)
        .map(Some)
        .map_err(serialization_error)
}

pub(super) fn json_values(
    values: Option<&Vec<Value>>,
) -> Result<Option<String>, SessionsApplicationError> {
    values
        .map(serde_json::to_string)
        .transpose()
        .map_err(serialization_error)
}

pub(super) fn serialize_configuration(
    values: &ChatConfigurationValues,
) -> Result<String, SessionsApplicationError> {
    serde_json::to_string(values).map_err(serialization_error)
}

pub(super) fn deserialize_configuration(raw: &str) -> Option<ChatConfigurationValues> {
    serde_json::from_str(raw).ok()
}

fn parse_json_values(raw: Option<&str>) -> Option<Vec<Value>> {
    raw.and_then(|value| serde_json::from_str(value).ok())
}

pub(super) fn repository_error(error: rusqlite::Error) -> SessionsApplicationError {
    SessionsApplicationError::Repository(error.to_string())
}

fn serialization_error(error: serde_json::Error) -> SessionsApplicationError {
    SessionsApplicationError::Serialization(error.to_string())
}
