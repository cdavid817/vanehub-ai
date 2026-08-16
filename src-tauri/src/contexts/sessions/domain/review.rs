use std::collections::HashSet;

pub(crate) const MAX_REVIEW_FILES: usize = 1_000;
pub(crate) const MAX_REVIEW_COMMENTS: usize = 2_000;
pub(crate) const MAX_REVIEW_FINDINGS: usize = 1_000;
pub(crate) const MAX_REVIEW_BODY_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewStatus {
    Active,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewDecision {
    Pending,
    Accepted,
    ChangesRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewCommentStatus {
    Active,
    Resolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewAnchorState {
    Current,
    Relocated,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReviewFindingSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewAnchor {
    pub(crate) file_path: String,
    pub(crate) side: String,
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) hunk_fingerprint: String,
    pub(crate) context_fingerprint: String,
    pub(crate) state: ReviewAnchorState,
}

impl ReviewAnchor {
    pub(crate) fn try_new(
        file_path: String,
        side: String,
        start_line: u32,
        end_line: u32,
        hunk_fingerprint: String,
        context_fingerprint: String,
    ) -> Result<Self, ReviewDomainError> {
        validate_relative_path(&file_path)?;
        if !matches!(side.as_str(), "old" | "new") {
            return Err(ReviewDomainError::InvalidAnchorSide);
        }
        if start_line == 0 || end_line < start_line {
            return Err(ReviewDomainError::InvalidLineRange);
        }
        validate_required(&hunk_fingerprint, "hunk fingerprint")?;
        validate_required(&context_fingerprint, "context fingerprint")?;
        Ok(Self {
            file_path,
            side,
            start_line,
            end_line,
            hunk_fingerprint,
            context_fingerprint,
            state: ReviewAnchorState::Current,
        })
    }

    pub(crate) fn mark_relocated(&mut self, start_line: u32, end_line: u32) {
        self.start_line = start_line;
        self.end_line = end_line;
        self.state = ReviewAnchorState::Relocated;
    }

    pub(crate) fn mark_stale(&mut self) {
        self.state = ReviewAnchorState::Stale;
    }

    pub(crate) fn restore_state(&mut self, state: ReviewAnchorState) {
        self.state = state;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewFile {
    pub(crate) path: String,
    pub(crate) previous_path: Option<String>,
    pub(crate) change_type: String,
    pub(crate) old_hash: Option<String>,
    pub(crate) new_hash: Option<String>,
}

impl ReviewFile {
    pub(crate) fn try_new(
        path: String,
        previous_path: Option<String>,
        change_type: String,
        old_hash: Option<String>,
        new_hash: Option<String>,
    ) -> Result<Self, ReviewDomainError> {
        validate_relative_path(&path)?;
        if let Some(previous) = previous_path.as_deref() {
            validate_relative_path(previous)?;
        }
        validate_required(&change_type, "change type")?;
        Ok(Self {
            path,
            previous_path,
            change_type,
            old_hash,
            new_hash,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewComment {
    pub(crate) id: String,
    pub(crate) anchor: ReviewAnchor,
    pub(crate) body: String,
    pub(crate) status: ReviewCommentStatus,
    pub(crate) selected: bool,
}

impl ReviewComment {
    pub(crate) fn try_new(
        id: String,
        anchor: ReviewAnchor,
        body: String,
    ) -> Result<Self, ReviewDomainError> {
        validate_required(&id, "comment id")?;
        validate_body(&body)?;
        Ok(Self {
            id,
            anchor,
            body,
            status: ReviewCommentStatus::Active,
            selected: true,
        })
    }

    pub(crate) fn resolve(&mut self) {
        self.status = ReviewCommentStatus::Resolved;
    }

    pub(crate) fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }

    pub(crate) fn restore_status(&mut self, status: ReviewCommentStatus, selected: bool) {
        self.status = status;
        self.selected = selected;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewFinding {
    pub(crate) id: String,
    pub(crate) source: String,
    pub(crate) title: String,
    pub(crate) severity: ReviewFindingSeverity,
    pub(crate) anchor: Option<ReviewAnchor>,
    pub(crate) operation_id: String,
    pub(crate) resolved: bool,
}

impl ReviewFinding {
    pub(crate) fn try_new(
        id: String,
        source: String,
        title: String,
        severity: ReviewFindingSeverity,
        anchor: Option<ReviewAnchor>,
        operation_id: String,
    ) -> Result<Self, ReviewDomainError> {
        validate_required(&id, "finding id")?;
        validate_required(&source, "finding source")?;
        validate_body(&title)?;
        validate_required(&operation_id, "operation id")?;
        Ok(Self {
            id,
            source,
            title,
            severity,
            anchor,
            operation_id,
            resolved: false,
        })
    }

    pub(crate) fn restore_resolved(&mut self, resolved: bool) {
        self.resolved = resolved;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewSession {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) workspace_id: String,
    pub(crate) base_revision: Option<String>,
    pub(crate) head_revision: Option<String>,
    pub(crate) fingerprint: String,
    pub(crate) status: ReviewStatus,
    pub(crate) decision: ReviewDecision,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    files: Vec<ReviewFile>,
    comments: Vec<ReviewComment>,
    findings: Vec<ReviewFinding>,
}

impl ReviewSession {
    pub(crate) fn try_new(
        id: String,
        session_id: String,
        workspace_id: String,
        base_revision: Option<String>,
        head_revision: Option<String>,
        fingerprint: String,
        files: Vec<ReviewFile>,
    ) -> Result<Self, ReviewDomainError> {
        for (value, kind) in [
            (&id, "review id"),
            (&session_id, "session id"),
            (&workspace_id, "workspace id"),
            (&fingerprint, "fingerprint"),
        ] {
            validate_required(value, kind)?;
        }
        if files.len() > MAX_REVIEW_FILES {
            return Err(ReviewDomainError::TooManyFiles);
        }
        let mut paths = HashSet::with_capacity(files.len());
        if files.iter().any(|file| !paths.insert(file.path.as_str())) {
            return Err(ReviewDomainError::DuplicateFile);
        }
        Ok(Self {
            id,
            session_id,
            workspace_id,
            base_revision,
            head_revision,
            fingerprint,
            status: ReviewStatus::Active,
            decision: ReviewDecision::Pending,
            created_at: String::new(),
            updated_at: String::new(),
            files,
            comments: Vec::new(),
            findings: Vec::new(),
        })
    }

    pub(crate) fn files(&self) -> &[ReviewFile] {
        &self.files
    }
    pub(crate) fn comments(&self) -> &[ReviewComment] {
        &self.comments
    }
    pub(crate) fn findings(&self) -> &[ReviewFinding] {
        &self.findings
    }

    pub(crate) fn comment_mut(&mut self, comment_id: &str) -> Option<&mut ReviewComment> {
        self.comments
            .iter_mut()
            .find(|comment| comment.id == comment_id)
    }

    pub(crate) fn add_comment(&mut self, comment: ReviewComment) -> Result<(), ReviewDomainError> {
        if self.comments.len() >= MAX_REVIEW_COMMENTS {
            return Err(ReviewDomainError::TooManyComments);
        }
        if !self
            .files
            .iter()
            .any(|file| file.path == comment.anchor.file_path)
        {
            return Err(ReviewDomainError::UnknownFile);
        }
        if self
            .comments
            .iter()
            .any(|existing| existing.id == comment.id)
        {
            return Err(ReviewDomainError::DuplicateComment);
        }
        self.comments.push(comment);
        Ok(())
    }

    pub(crate) fn add_finding(&mut self, finding: ReviewFinding) -> Result<(), ReviewDomainError> {
        if self.findings.len() >= MAX_REVIEW_FINDINGS {
            return Err(ReviewDomainError::TooManyFindings);
        }
        if self
            .findings
            .iter()
            .any(|existing| existing.id == finding.id)
        {
            return Err(ReviewDomainError::DuplicateFinding);
        }
        self.findings.push(finding);
        Ok(())
    }

    pub(crate) fn set_decision(&mut self, decision: ReviewDecision) {
        self.decision = decision;
        if decision == ReviewDecision::Accepted {
            self.status = ReviewStatus::Completed;
        }
    }

    pub(crate) fn restore_lifecycle(&mut self, status: ReviewStatus, decision: ReviewDecision) {
        self.status = status;
        self.decision = decision;
    }

    pub(crate) fn set_timestamps(&mut self, created_at: String, updated_at: String) {
        self.created_at = created_at;
        self.updated_at = updated_at;
    }

    pub(crate) fn reconcile_snapshot(&mut self, fingerprint: String, files: Vec<ReviewFile>) {
        if self.fingerprint == fingerprint {
            return;
        }
        for comment in &mut self.comments {
            if files
                .iter()
                .any(|file| file.path == comment.anchor.file_path)
            {
                comment
                    .anchor
                    .mark_relocated(comment.anchor.start_line, comment.anchor.end_line);
            } else {
                comment.anchor.mark_stale();
            }
        }
        self.fingerprint = fingerprint;
        self.files = files;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReviewDomainError {
    Required(&'static str),
    InvalidPath,
    InvalidAnchorSide,
    InvalidLineRange,
    BodyTooLarge,
    TooManyFiles,
    TooManyComments,
    TooManyFindings,
    DuplicateFile,
    DuplicateComment,
    DuplicateFinding,
    UnknownFile,
}

fn validate_required(value: &str, kind: &'static str) -> Result<(), ReviewDomainError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(ReviewDomainError::Required(kind));
    }
    Ok(())
}

fn validate_body(value: &str) -> Result<(), ReviewDomainError> {
    if value.trim().is_empty() {
        return Err(ReviewDomainError::Required("review body"));
    }
    if value.len() > MAX_REVIEW_BODY_BYTES {
        return Err(ReviewDomainError::BodyTooLarge);
    }
    Ok(())
}

fn validate_relative_path(path: &str) -> Result<(), ReviewDomainError> {
    let candidate = std::path::Path::new(path);
    if path.is_empty()
        || candidate.is_absolute()
        || candidate.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(ReviewDomainError::InvalidPath);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> ReviewFile {
        ReviewFile::try_new(
            "src/a.rs".into(),
            None,
            "modified".into(),
            None,
            Some("new".into()),
        )
        .unwrap()
    }
    fn anchor() -> ReviewAnchor {
        ReviewAnchor::try_new(
            "src/a.rs".into(),
            "new".into(),
            4,
            6,
            "hunk".into(),
            "context".into(),
        )
        .unwrap()
    }

    #[test]
    fn review_enforces_paths_bounds_and_ownership() {
        assert_eq!(
            ReviewFile::try_new("../secret".into(), None, "modified".into(), None, None),
            Err(ReviewDomainError::InvalidPath)
        );
        let mut review = ReviewSession::try_new(
            "review".into(),
            "session".into(),
            "workspace".into(),
            None,
            None,
            "fingerprint".into(),
            vec![file()],
        )
        .unwrap();
        let foreign = ReviewComment::try_new(
            "comment".into(),
            ReviewAnchor::try_new(
                "src/b.rs".into(),
                "new".into(),
                1,
                1,
                "h".into(),
                "c".into(),
            )
            .unwrap(),
            "fix".into(),
        )
        .unwrap();
        assert_eq!(
            review.add_comment(foreign),
            Err(ReviewDomainError::UnknownFile)
        );
    }

    #[test]
    fn anchors_relocate_or_become_stale_and_decisions_complete() {
        let mut anchor = anchor();
        anchor.mark_relocated(10, 12);
        assert_eq!(anchor.state, ReviewAnchorState::Relocated);
        anchor.mark_stale();
        assert_eq!(anchor.state, ReviewAnchorState::Stale);
        let mut review = ReviewSession::try_new(
            "review".into(),
            "session".into(),
            "workspace".into(),
            None,
            None,
            "fingerprint".into(),
            vec![file()],
        )
        .unwrap();
        review
            .add_comment(
                ReviewComment::try_new("comment".into(), anchor, "please fix".into()).unwrap(),
            )
            .unwrap();
        review.set_decision(ReviewDecision::Accepted);
        assert_eq!(review.status, ReviewStatus::Completed);
    }
}
