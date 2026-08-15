use super::{MessageId, SessionId, SessionsDomainError};
use std::collections::BTreeSet;

pub(crate) const MAX_FILE_REFERENCES: usize = 5;

/// An inclusive 1-based line span. Holding both bounds together makes a half-specified
/// range unrepresentable rather than something every consumer has to re-check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FileLineRange {
    start: u32,
    end: u32,
}

impl FileLineRange {
    pub(crate) fn new(start: u32, end: u32) -> Result<Self, SessionsDomainError> {
        if start == 0 || end < start {
            return Err(SessionsDomainError::InvalidFileReferenceRange);
        }
        Ok(Self { start, end })
    }

    /// Boundary conversion: the wire and storage formats carry two independent optional
    /// bounds, and exactly one of them present is the error this catches.
    pub(crate) fn from_optional_bounds(
        start: Option<u32>,
        end: Option<u32>,
    ) -> Result<Option<Self>, SessionsDomainError> {
        match (start, end) {
            (None, None) => Ok(None),
            (Some(start), Some(end)) => Self::new(start, end).map(Some),
            _ => Err(SessionsDomainError::InvalidFileReferenceRange),
        }
    }

    pub(crate) fn start(&self) -> u32 {
        self.start
    }

    pub(crate) fn end(&self) -> u32 {
        self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileReference {
    id: String,
    path: String,
    name: String,
    size_bytes: Option<i64>,
    content_hash: Option<String>,
    line_range: Option<FileLineRange>,
}

impl FileReference {
    pub(crate) fn new(
        id: impl Into<String>,
        path: impl Into<String>,
        name: impl Into<String>,
        size_bytes: Option<i64>,
        content_hash: Option<String>,
        line_range: Option<FileLineRange>,
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
            line_range,
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

    pub(crate) fn line_range(&self) -> Option<FileLineRange> {
        self.line_range
    }

    /// Identity for deduplication and for user-facing collision messages: a path alone no
    /// longer identifies a reference now that two regions of one file can coexist.
    fn identity(&self) -> (String, Option<FileLineRange>) {
        (self.path.clone(), self.line_range)
    }

    fn describe(&self) -> String {
        match self.line_range {
            Some(range) => format!("{} (lines {}-{})", self.path, range.start(), range.end()),
            None => self.path.clone(),
        }
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
        let mut seen = BTreeSet::new();
        for reference in &references {
            if !seen.insert(reference.identity()) {
                return Err(SessionsDomainError::DuplicateFileReferencePath(
                    reference.describe(),
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
    session_sequence: u64,
    execution_run_id: Option<String>,
}

impl SessionMessage {
    pub(crate) fn rehydrate(
        id: MessageId,
        session_id: SessionId,
        role: MessageRole,
        status: MessageStatus,
        file_references: FileReferenceSet,
    ) -> Self {
        Self::rehydrate_with_correlation(id, session_id, role, status, file_references, 0, None)
    }

    pub(crate) fn rehydrate_with_correlation(
        id: MessageId,
        session_id: SessionId,
        role: MessageRole,
        status: MessageStatus,
        file_references: FileReferenceSet,
        session_sequence: u64,
        execution_run_id: Option<String>,
    ) -> Self {
        Self {
            id,
            session_id,
            role,
            status,
            file_references,
            session_sequence,
            execution_run_id,
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

    pub(crate) fn session_sequence(&self) -> u64 {
        self.session_sequence
    }

    pub(crate) fn execution_run_id(&self) -> Option<&str> {
        self.execution_run_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(path: &str) -> FileReference {
        FileReference::new(path, path, path, Some(10), Some("hash".to_string()), None)
            .expect("file reference")
    }

    fn ranged(path: &str, start: u32, end: u32) -> FileReference {
        FileReference::new(
            format!("{path}:{start}-{end}"),
            path,
            path,
            Some(10),
            None,
            Some(FileLineRange::new(start, end).expect("line range")),
        )
        .expect("file reference")
    }

    #[test]
    fn line_ranges_require_both_bounds_a_positive_start_and_a_sane_order() {
        assert_eq!(FileLineRange::new(1, 1).map(|range| range.start()), Ok(1));
        assert_eq!(FileLineRange::new(10, 50).map(|range| range.end()), Ok(50));
        assert_eq!(
            FileLineRange::new(0, 5),
            Err(SessionsDomainError::InvalidFileReferenceRange)
        );
        assert_eq!(
            FileLineRange::new(50, 10),
            Err(SessionsDomainError::InvalidFileReferenceRange)
        );
        assert_eq!(FileLineRange::from_optional_bounds(None, None), Ok(None));
        assert_eq!(
            FileLineRange::from_optional_bounds(Some(10), Some(50)),
            FileLineRange::new(10, 50).map(Some)
        );
        for half in [(Some(10), None), (None, Some(50))] {
            assert_eq!(
                FileLineRange::from_optional_bounds(half.0, half.1),
                Err(SessionsDomainError::InvalidFileReferenceRange)
            );
        }
    }

    #[test]
    fn two_regions_of_one_file_coexist_while_an_exact_duplicate_is_rejected() {
        let first = ranged("src/utils.rs", 10, 20);
        let second = ranged("src/utils.rs", 50, 60);
        let whole = reference("src/utils.rs");
        assert_eq!(first.line_range().map(|range| range.start()), Some(10));
        assert!(FileReferenceSet::new(vec![first.clone(), second]).is_ok());
        assert!(FileReferenceSet::new(vec![first.clone(), whole.clone()]).is_ok());
        assert_eq!(
            FileReferenceSet::new(vec![first.clone(), first]),
            Err(SessionsDomainError::DuplicateFileReferencePath(
                "src/utils.rs (lines 10-20)".to_string()
            ))
        );
        assert_eq!(
            FileReferenceSet::new(vec![whole.clone(), whole]),
            Err(SessionsDomainError::DuplicateFileReferencePath(
                "src/utils.rs".to_string()
            ))
        );
    }

    #[test]
    fn file_reference_sets_validate_fields_size_limit_and_unique_paths() {
        let first = reference("src/main.rs");
        assert_eq!(first.id(), "src/main.rs");
        assert_eq!(first.name(), "src/main.rs");
        assert_eq!(first.size_bytes(), Some(10));
        assert_eq!(first.content_hash(), Some("hash"));
        assert_eq!(
            FileReference::new("id", "path", "name", Some(-1), None, None),
            Err(SessionsDomainError::InvalidFileReferenceSize)
        );
        assert!(FileReference::new("", "path", "name", None, None, None).is_err());
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
