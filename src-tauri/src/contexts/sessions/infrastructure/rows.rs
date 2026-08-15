use crate::contexts::sessions::application::{
    CategoryRecord, ChatConfigurationValues, FileReferenceInput, LoopSessionOwnership,
    MessageRecord, MessageTokenUsage, SessionRecord, SessionRemoteWorkspace, SessionWorkspace,
    SessionsApplicationError,
};
use crate::contexts::sessions::domain::recovery::{SessionRecoveryMetadata, SessionRecoveryStatus};
use crate::contexts::sessions::domain::{
    decode_seats, CategoryId, CategoryName, FileLineRange, FileReference, FileReferenceSet,
    LoopSessionRole, MessageId, MessageRole, MessageStatus, SessionAggregate, SessionCategory,
    SessionId, SessionLifecycle, SessionMessage, SessionOwner, SessionTitle,
};
use rusqlite::{Connection, OptionalExtension, Row};
use serde_json::Value;

pub(super) const SESSION_SELECT: &str = "SELECT id, title, agent_id, interaction_mode, lifecycle_state, folder, project_path, worktree_path, worktree_name, worktree_branch, remote_workspace_host, remote_workspace_port, remote_workspace_user, remote_workspace_path, remote_workspace_display_name, remote_workspace_uri, remote_ssh_connection_id, remote_ssh_connection_revision, runtime_session_id, category_id, source_kind, source_connector, pinned, archived, created_at, updated_at, loop_run_id, loop_iteration_id, loop_role, seats, recovery_status, recovery_revision, state_revision, history_revision, active_execution_run_id, next_message_sequence, COALESCE(origin_kind, 'user'), origin_id FROM sessions";
pub(super) const SESSION_SEARCH_SELECT: &str = "SELECT sessions.id, sessions.title, sessions.agent_id, sessions.interaction_mode, sessions.lifecycle_state, sessions.folder, sessions.project_path, sessions.worktree_path, sessions.worktree_name, sessions.worktree_branch, sessions.remote_workspace_host, sessions.remote_workspace_port, sessions.remote_workspace_user, sessions.remote_workspace_path, sessions.remote_workspace_display_name, sessions.remote_workspace_uri, sessions.remote_ssh_connection_id, sessions.remote_ssh_connection_revision, sessions.runtime_session_id, sessions.category_id, sessions.source_kind, sessions.source_connector, sessions.pinned, sessions.archived, sessions.created_at, sessions.updated_at, sessions.loop_run_id, sessions.loop_iteration_id, sessions.loop_role, sessions.seats, sessions.recovery_status, sessions.recovery_revision, sessions.state_revision, sessions.history_revision, sessions.active_execution_run_id, sessions.next_message_sequence, COALESCE(sessions.origin_kind, 'user'), sessions.origin_id, message_matches.id, message_matches.content FROM sessions";
pub(super) const MESSAGE_SELECT: &str = "SELECT id, session_id, role, status, content, thinking_content, tool_use, rich_blocks, token_input, token_output, metadata, file_references, created_at, updated_at, seat_index, speaker_seat_id, session_sequence, execution_run_id, seat_round_id, parent_execution_run_id FROM messages";
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
    remote_workspace_port: Option<i64>,
    remote_workspace_user: Option<String>,
    remote_workspace_path: Option<String>,
    remote_workspace_display_name: Option<String>,
    remote_workspace_uri: Option<String>,
    remote_ssh_connection_id: Option<String>,
    remote_ssh_connection_revision: Option<i64>,
    runtime_session_id: Option<String>,
    category_id: Option<String>,
    source_kind: String,
    source_connector: Option<String>,
    pinned: bool,
    archived: bool,
    created_at: String,
    updated_at: String,
    loop_run_id: Option<String>,
    loop_iteration_id: Option<String>,
    loop_role: Option<String>,
    seats: String,
    recovery_status: String,
    recovery_revision: i64,
    state_revision: i64,
    history_revision: i64,
    active_execution_run_id: Option<String>,
    next_message_sequence: i64,
    execution_origin_kind: String,
    execution_origin_id: Option<String>,
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
            remote_workspace_port: row.get(11)?,
            remote_workspace_user: row.get(12)?,
            remote_workspace_path: row.get(13)?,
            remote_workspace_display_name: row.get(14)?,
            remote_workspace_uri: row.get(15)?,
            remote_ssh_connection_id: row.get(16)?,
            remote_ssh_connection_revision: row.get(17)?,
            runtime_session_id: row.get(18)?,
            category_id: row.get(19)?,
            source_kind: row.get(20)?,
            source_connector: row.get(21)?,
            pinned: row.get::<_, i64>(22)? != 0,
            archived: row.get::<_, i64>(23)? != 0,
            created_at: row.get(24)?,
            updated_at: row.get(25)?,
            loop_run_id: row.get(26)?,
            loop_iteration_id: row.get(27)?,
            loop_role: row.get(28)?,
            seats: row.get(29)?,
            recovery_status: row.get(30)?,
            recovery_revision: row.get(31)?,
            state_revision: row.get(32)?,
            history_revision: row.get(33)?,
            active_execution_run_id: row.get(34)?,
            next_message_sequence: row.get(35)?,
            execution_origin_kind: row.get(36)?,
            execution_origin_id: row.get(37)?,
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
                    port: self
                        .remote_workspace_port
                        .and_then(|port| u16::try_from(port).ok()),
                    user: self.remote_workspace_user,
                    path,
                    display_name,
                    uri,
                })
            }
            _ => None,
        };
        let session_id = self.id.clone();
        let recovery_status = SessionRecoveryStatus::from_storage(&self.recovery_status)
            .ok_or_else(|| {
                SessionsApplicationError::Repository(format!(
                    "invalid session recovery status: {}",
                    self.recovery_status
                ))
            })?;
        let aggregate = SessionAggregate::rehydrate_with_recovery(
            SessionId::parse(self.id)?,
            SessionTitle::for_creation(Some(&self.title)),
            SessionLifecycle::from_storage_lossy(&self.lifecycle_state),
            SessionOwner::from_parts(&self.source_kind, self.source_connector.as_deref())?,
            self.category_id.map(CategoryId::parse).transpose()?,
            self.pinned,
            self.archived,
            SessionRecoveryMetadata::rehydrate(
                recovery_status,
                non_negative_revision(self.recovery_revision, "recovery_revision")?,
                non_negative_revision(self.state_revision, "state_revision")?,
                non_negative_revision(self.history_revision, "history_revision")?,
                self.active_execution_run_id,
                positive_sequence(self.next_message_sequence, "next_message_sequence")?,
            ),
        );
        let loop_ownership = match (self.loop_run_id, self.loop_iteration_id, self.loop_role) {
            (Some(run_id), Some(iteration_id), Some(role)) => Some(LoopSessionOwnership {
                run_id,
                iteration_id,
                role: LoopSessionRole::parse(&role)?,
            }),
            (None, None, None) => None,
            _ => {
                return Err(SessionsApplicationError::Repository(
                    "incomplete Loop session ownership metadata".to_string(),
                ));
            }
        };
        let remote_ssh_binding = match (
            self.remote_ssh_connection_id,
            self.remote_ssh_connection_revision,
        ) {
            (Some(connection_id), Some(revision)) => {
                Some(crate::contexts::sessions::application::SessionSshBinding {
                    connection_id,
                    revision,
                })
            }
            (None, None) => None,
            _ => {
                return Err(SessionsApplicationError::Repository(
                    "incomplete remote SSH binding metadata".to_string(),
                ));
            }
        };
        let seats = decode_seats(&self.seats, &session_id, &self.agent_id, &self.created_at);
        Ok(SessionRecord {
            aggregate,
            agent_id: self.agent_id,
            seats,
            interaction_mode: self.interaction_mode,
            workspace: SessionWorkspace {
                folder: self.folder,
                project_path: self.project_path,
                worktree_path: self.worktree_path,
                worktree_name: self.worktree_name,
                worktree_branch: self.worktree_branch,
                remote_workspace,
                remote_ssh_binding,
                loop_ownership,
            },
            runtime_session_id: self.runtime_session_id,
            execution_origin_kind: self.execution_origin_kind,
            execution_origin_id: self.execution_origin_id,
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
    seat_index: Option<i64>,
    speaker_seat_id: Option<String>,
    session_sequence: i64,
    execution_run_id: Option<String>,
    seat_round_id: Option<String>,
    parent_execution_run_id: Option<String>,
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
            seat_index: row.get(14)?,
            speaker_seat_id: row.get(15)?,
            session_sequence: row.get(16)?,
            execution_run_id: row.get(17)?,
            seat_round_id: row.get(18)?,
            parent_execution_run_id: row.get(19)?,
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
                    FileLineRange::from_optional_bounds(reference.start_line, reference.end_line)?,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let message = SessionMessage::rehydrate_with_correlation(
            MessageId::parse(self.id)?,
            SessionId::parse(self.session_id)?,
            MessageRole::parse(&self.role)?,
            MessageStatus::parse(&self.status)?,
            FileReferenceSet::new(references)?,
            non_negative_revision(self.session_sequence, "session_sequence")?,
            self.execution_run_id,
        );
        let token_usage =
            (self.token_input > 0 || self.token_output > 0).then_some(MessageTokenUsage {
                input: self.token_input,
                output: self.token_output,
            });
        Ok(MessageRecord {
            message,
            speaker_seat_id: self.speaker_seat_id,
            seat_index: self
                .seat_index
                .and_then(|index| usize::try_from(index).ok()),
            seat_round_id: self.seat_round_id,
            parent_execution_run_id: self.parent_execution_run_id,
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
            start_line: reference.line_range().map(|range| range.start()),
            end_line: reference.line_range().map(|range| range.end()),
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

fn non_negative_revision(value: i64, field: &str) -> Result<u64, SessionsApplicationError> {
    u64::try_from(value).map_err(|_| {
        SessionsApplicationError::Repository(format!("invalid negative {field}: {value}"))
    })
}

fn positive_sequence(value: i64, field: &str) -> Result<u64, SessionsApplicationError> {
    let value = non_negative_revision(value, field)?;
    if value == 0 {
        Err(SessionsApplicationError::Repository(format!(
            "invalid zero {field}"
        )))
    } else {
        Ok(value)
    }
}

pub(super) fn repository_error(error: rusqlite::Error) -> SessionsApplicationError {
    SessionsApplicationError::Repository(error.to_string())
}

pub(super) fn recovery_repository_error(error: rusqlite::Error) -> SessionsApplicationError {
    if matches!(
        error,
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked,
                ..
            },
            _
        )
    ) {
        SessionsApplicationError::RetryableStorage(error.to_string())
    } else {
        SessionsApplicationError::Repository(error.to_string())
    }
}

fn serialization_error(error: serde_json::Error) -> SessionsApplicationError {
    SessionsApplicationError::Serialization(error.to_string())
}
