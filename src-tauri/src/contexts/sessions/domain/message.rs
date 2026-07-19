use super::{MessageId, SessionId, SessionsDomainError};
use std::collections::BTreeSet;

pub(crate) const MAX_FILE_REFERENCES: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileReference {
    id: String,
    path: String,
    name: String,
    size_bytes: Option<i64>,
    content_hash: Option<String>,
}

impl FileReference {
    pub(crate) fn new(
        id: impl Into<String>,
        path: impl Into<String>,
        name: impl Into<String>,
        size_bytes: Option<i64>,
        content_hash: Option<String>,
    ) -> Result<Self, SessionsDomainError> {
        let id = required_file_field(id.into(), "id")?;
        let path = required_file_field(path.into(), "path")?;
        let name = required_file_field(name.into(), "name")?;
        if size_bytes.is_some_and(|size| size < 0) {
            return Err(SessionsDomainError::InvalidFileReferenceSize);
        }
        Ok(Self {
            id,
            path,
            name,
            size_bytes,
            content_hash,
        })
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn size_bytes(&self) -> Option<i64> {
        self.size_bytes
    }

    pub(crate) fn content_hash(&self) -> Option<&str> {
        self.content_hash.as_deref()
    }
}

fn required_file_field(value: String, field: &'static str) -> Result<String, SessionsDomainError> {
    if value.trim().is_empty() {
        Err(SessionsDomainError::FileReferenceFieldRequired(field))
    } else {
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FileReferenceSet(Vec<FileReference>);

impl FileReferenceSet {
    pub(crate) fn new(references: Vec<FileReference>) -> Result<Self, SessionsDomainError> {
        if references.len() > MAX_FILE_REFERENCES {
            return Err(SessionsDomainError::TooManyFileReferences);
        }
        let mut paths = BTreeSet::new();
        for reference in &references {
            if !paths.insert(reference.path().to_string()) {
                return Err(SessionsDomainError::DuplicateFileReferencePath(
                    reference.path().to_string(),
                ));
            }
        }
        Ok(Self(references))
    }

    pub(crate) fn as_slice(&self) -> &[FileReference] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageRole {
    User,
    Assistant,
}

impl MessageRole {
    pub(crate) fn parse(value: &str) -> Result<Self, SessionsDomainError> {
        match value {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            value => Err(SessionsDomainError::InvalidMessageRole(value.to_string())),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageStatus {
    Pending,
    Streaming,
    Completed,
    Failed,
    Cancelled,
}

impl MessageStatus {
    pub(crate) fn parse(value: &str) -> Result<Self, SessionsDomainError> {
        match value {
            "pending" => Ok(Self::Pending),
            "streaming" => Ok(Self::Streaming),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            value => Err(SessionsDomainError::InvalidMessageStatus(value.to_string())),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Streaming => "streaming",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    fn can_transition_to(self, next: Self) -> bool {
        self == next
            || matches!(
                (self, next),
                (
                    Self::Pending,
                    Self::Streaming | Self::Completed | Self::Failed | Self::Cancelled
                ) | (
                    Self::Streaming,
                    Self::Completed | Self::Failed | Self::Cancelled
                )
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionMessage {
    id: MessageId,
    session_id: SessionId,
    role: MessageRole,
    status: MessageStatus,
    file_references: FileReferenceSet,
}

impl SessionMessage {
    pub(crate) fn rehydrate(
        id: MessageId,
        session_id: SessionId,
        role: MessageRole,
        status: MessageStatus,
        file_references: FileReferenceSet,
    ) -> Self {
        Self {
            id,
            session_id,
            role,
            status,
            file_references,
        }
    }

    pub(crate) fn ensure_owned_by(
        &self,
        session_id: &SessionId,
    ) -> Result<(), SessionsDomainError> {
        if &self.session_id == session_id {
            Ok(())
        } else {
            Err(SessionsDomainError::MessageOwnershipMismatch {
                message_id: self.id.as_str().to_string(),
                expected_session_id: session_id.as_str().to_string(),
                actual_session_id: self.session_id.as_str().to_string(),
            })
        }
    }

    pub(crate) fn transition_to(&mut self, next: MessageStatus) -> Result<(), SessionsDomainError> {
        if !self.status.can_transition_to(next) {
            return Err(SessionsDomainError::InvalidMessageTransition {
                from: self.status.as_str(),
                to: next.as_str(),
            });
        }
        self.status = next;
        Ok(())
    }

    pub(crate) fn id(&self) -> &MessageId {
        &self.id
    }

    pub(crate) fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub(crate) fn role(&self) -> MessageRole {
        self.role
    }

    pub(crate) fn status(&self) -> MessageStatus {
        self.status
    }

    pub(crate) fn file_references(&self) -> &FileReferenceSet {
        &self.file_references
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(path: &str) -> FileReference {
        FileReference::new(path, path, path, Some(10), Some("hash".to_string()))
            .expect("file reference")
    }

    #[test]
    fn file_reference_sets_validate_fields_size_limit_and_unique_paths() {
        let first = reference("src/main.rs");
        assert_eq!(first.id(), "src/main.rs");
        assert_eq!(first.name(), "src/main.rs");
        assert_eq!(first.size_bytes(), Some(10));
        assert_eq!(first.content_hash(), Some("hash"));
        assert_eq!(
            FileReference::new("id", "path", "name", Some(-1), None),
            Err(SessionsDomainError::InvalidFileReferenceSize)
        );
        assert!(FileReference::new("", "path", "name", None, None).is_err());
        assert_eq!(
            FileReferenceSet::new(vec![first.clone(), first]),
            Err(SessionsDomainError::DuplicateFileReferencePath(
                "src/main.rs".to_string()
            ))
        );
        assert_eq!(
            FileReferenceSet::new(
                (0..=MAX_FILE_REFERENCES)
                    .map(|index| reference(&format!("file-{index}")))
                    .collect()
            ),
            Err(SessionsDomainError::TooManyFileReferences)
        );
    }

    #[test]
    fn message_ownership_and_terminal_transitions_are_explicit() {
        let session_id = SessionId::parse("session-1").expect("session id");
        let mut message = SessionMessage::rehydrate(
            MessageId::parse("message-1").expect("message id"),
            session_id.clone(),
            MessageRole::Assistant,
            MessageStatus::Streaming,
            FileReferenceSet::default(),
        );

        assert_eq!(message.id().as_str(), "message-1");
        assert_eq!(message.session_id(), &session_id);
        assert_eq!(message.role().as_str(), "assistant");
        assert!(message.file_references().as_slice().is_empty());
        assert_eq!(message.ensure_owned_by(&session_id), Ok(()));
        assert!(message
            .ensure_owned_by(&SessionId::parse("session-2").expect("other session"))
            .is_err());
        message
            .transition_to(MessageStatus::Completed)
            .expect("complete");
        assert_eq!(message.status(), MessageStatus::Completed);
        assert!(message.transition_to(MessageStatus::Streaming).is_err());
        assert_eq!(MessageRole::parse("user"), Ok(MessageRole::User));
        assert_eq!(
            MessageStatus::parse("unknown"),
            Err(SessionsDomainError::InvalidMessageStatus(
                "unknown".to_string()
            ))
        );
    }
}
