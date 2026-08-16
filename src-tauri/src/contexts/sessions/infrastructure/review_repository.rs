use crate::contexts::sessions::application::{ReviewApplicationError, ReviewRepository};
use crate::contexts::sessions::domain::{
    ReviewAnchor, ReviewAnchorState, ReviewComment, ReviewCommentStatus, ReviewDecision,
    ReviewFile, ReviewFinding, ReviewFindingSeverity, ReviewSession, ReviewStatus,
};
use crate::platform::database::{DatabaseError, NativeDatabase};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        "CREATE TABLE review_sessions (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            workspace_id TEXT NOT NULL,
            base_revision TEXT,
            head_revision TEXT,
            fingerprint TEXT NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('active','completed')),
            decision TEXT NOT NULL CHECK(decision IN ('pending','accepted','changes_requested')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE UNIQUE INDEX idx_review_sessions_active_session
            ON review_sessions(session_id) WHERE status = 'active';
        CREATE TABLE review_files (
            review_id TEXT NOT NULL REFERENCES review_sessions(id) ON DELETE CASCADE,
            path TEXT NOT NULL,
            previous_path TEXT,
            change_type TEXT NOT NULL,
            old_hash TEXT,
            new_hash TEXT,
            PRIMARY KEY(review_id, path)
        );
        CREATE TABLE review_comments (
            id TEXT PRIMARY KEY,
            review_id TEXT NOT NULL REFERENCES review_sessions(id) ON DELETE CASCADE,
            file_path TEXT NOT NULL,
            side TEXT NOT NULL,
            start_line INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            hunk_fingerprint TEXT NOT NULL,
            context_fingerprint TEXT NOT NULL,
            anchor_state TEXT NOT NULL,
            body TEXT NOT NULL,
            status TEXT NOT NULL,
            selected INTEGER NOT NULL
        );
        CREATE INDEX idx_review_comments_review ON review_comments(review_id, id);
        CREATE TABLE review_findings (
            id TEXT PRIMARY KEY,
            review_id TEXT NOT NULL REFERENCES review_sessions(id) ON DELETE CASCADE,
            source TEXT NOT NULL,
            title TEXT NOT NULL,
            severity TEXT NOT NULL,
            operation_id TEXT NOT NULL,
            resolved INTEGER NOT NULL,
            anchor_file_path TEXT,
            anchor_side TEXT,
            anchor_start_line INTEGER,
            anchor_end_line INTEGER,
            anchor_hunk_fingerprint TEXT,
            anchor_context_fingerprint TEXT,
            anchor_state TEXT
        );
        CREATE INDEX idx_review_findings_review ON review_findings(review_id, id);",
    )?;
    Ok(())
}

#[derive(Clone)]
pub(crate) struct SqliteReviewRepository {
    database: NativeDatabase,
}

pub(crate) struct SystemReviewClock;

impl crate::contexts::sessions::application::ReviewClockPort for SystemReviewClock {
    fn now(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }
}

pub(crate) struct UuidReviewIds;

impl crate::contexts::sessions::application::ReviewIdPort for UuidReviewIds {
    fn next_id(&self, _kind: &'static str) -> String {
        uuid::Uuid::now_v7().to_string()
    }
}

impl SqliteReviewRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn load(
        &self,
        query: &str,
        value: &str,
    ) -> Result<Option<ReviewSession>, ReviewApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        let head = connection
            .query_row(query, [value], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                ))
            })
            .optional()
            .map_err(repository_error)?;
        let Some((
            id,
            session_id,
            workspace_id,
            base,
            head,
            fingerprint,
            status,
            decision,
            created_at,
            updated_at,
        )) = head
        else {
            return Ok(None);
        };
        let files = load_files(&connection, &id)?;
        let mut review = ReviewSession::try_new(
            id.clone(),
            session_id,
            workspace_id,
            base,
            head,
            fingerprint,
            files,
        )
        .map_err(ReviewApplicationError::Domain)?;
        for comment in load_comments(&connection, &id)? {
            review
                .add_comment(comment)
                .map_err(ReviewApplicationError::Domain)?;
        }
        for finding in load_findings(&connection, &id)? {
            review
                .add_finding(finding)
                .map_err(ReviewApplicationError::Domain)?;
        }
        review.restore_lifecycle(parse_status(&status)?, parse_decision(&decision)?);
        review.set_timestamps(created_at, updated_at);
        Ok(Some(review))
    }
}

impl ReviewRepository for SqliteReviewRepository {
    fn find_active_by_session(
        &self,
        session_id: &str,
    ) -> Result<Option<ReviewSession>, ReviewApplicationError> {
        self.load("SELECT id,session_id,workspace_id,base_revision,head_revision,fingerprint,status,decision,created_at,updated_at FROM review_sessions WHERE session_id=?1 AND status='active'", session_id)
    }

    fn find(&self, review_id: &str) -> Result<Option<ReviewSession>, ReviewApplicationError> {
        self.load("SELECT id,session_id,workspace_id,base_revision,head_revision,fingerprint,status,decision,created_at,updated_at FROM review_sessions WHERE id=?1", review_id)
    }

    fn save(&self, review: &ReviewSession) -> Result<(), ReviewApplicationError> {
        let mut connection = self.database.connection().map_err(repository_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        save_head(&transaction, review)?;
        transaction
            .execute("DELETE FROM review_files WHERE review_id=?1", [&review.id])
            .map_err(repository_error)?;
        transaction
            .execute(
                "DELETE FROM review_comments WHERE review_id=?1",
                [&review.id],
            )
            .map_err(repository_error)?;
        transaction
            .execute(
                "DELETE FROM review_findings WHERE review_id=?1",
                [&review.id],
            )
            .map_err(repository_error)?;
        for file in review.files() {
            save_file(&transaction, &review.id, file)?;
        }
        for comment in review.comments() {
            save_comment(&transaction, &review.id, comment)?;
        }
        for finding in review.findings() {
            save_finding(&transaction, &review.id, finding)?;
        }
        transaction.commit().map_err(repository_error)
    }
}

fn save_head(
    transaction: &Transaction<'_>,
    review: &ReviewSession,
) -> Result<(), ReviewApplicationError> {
    transaction.execute(
        "INSERT INTO review_sessions(id,session_id,workspace_id,base_revision,head_revision,fingerprint,status,decision,created_at,updated_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
         ON CONFLICT(id) DO UPDATE SET fingerprint=excluded.fingerprint,status=excluded.status,decision=excluded.decision,updated_at=excluded.updated_at",
        params![review.id, review.session_id, review.workspace_id, review.base_revision, review.head_revision, review.fingerprint, status_value(review.status), decision_value(review.decision),review.created_at,review.updated_at],
    ).map_err(repository_error)?;
    Ok(())
}

fn save_file(
    tx: &Transaction<'_>,
    review_id: &str,
    file: &ReviewFile,
) -> Result<(), ReviewApplicationError> {
    tx.execute("INSERT INTO review_files(review_id,path,previous_path,change_type,old_hash,new_hash) VALUES(?1,?2,?3,?4,?5,?6)", params![review_id,file.path,file.previous_path,file.change_type,file.old_hash,file.new_hash]).map_err(repository_error)?;
    Ok(())
}

fn save_comment(
    tx: &Transaction<'_>,
    review_id: &str,
    value: &ReviewComment,
) -> Result<(), ReviewApplicationError> {
    tx.execute("INSERT INTO review_comments(id,review_id,file_path,side,start_line,end_line,hunk_fingerprint,context_fingerprint,anchor_state,body,status,selected) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)", params![value.id,review_id,value.anchor.file_path,value.anchor.side,value.anchor.start_line,value.anchor.end_line,value.anchor.hunk_fingerprint,value.anchor.context_fingerprint,anchor_state_value(value.anchor.state),value.body,comment_status_value(value.status),value.selected]).map_err(repository_error)?;
    Ok(())
}

fn save_finding(
    tx: &Transaction<'_>,
    review_id: &str,
    value: &ReviewFinding,
) -> Result<(), ReviewApplicationError> {
    let anchor = value.anchor.as_ref();
    tx.execute("INSERT INTO review_findings(id,review_id,source,title,severity,operation_id,resolved,anchor_file_path,anchor_side,anchor_start_line,anchor_end_line,anchor_hunk_fingerprint,anchor_context_fingerprint,anchor_state) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)", params![value.id,review_id,value.source,value.title,severity_value(value.severity),value.operation_id,value.resolved,anchor.map(|v| &v.file_path),anchor.map(|v| &v.side),anchor.map(|v| v.start_line),anchor.map(|v| v.end_line),anchor.map(|v| &v.hunk_fingerprint),anchor.map(|v| &v.context_fingerprint),anchor.map(|v| anchor_state_value(v.state))]).map_err(repository_error)?;
    Ok(())
}

fn load_files(
    connection: &Connection,
    review_id: &str,
) -> Result<Vec<ReviewFile>, ReviewApplicationError> {
    let mut statement = connection.prepare("SELECT path,previous_path,change_type,old_hash,new_hash FROM review_files WHERE review_id=?1 ORDER BY path").map_err(repository_error)?;
    let rows = statement
        .query_map([review_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(repository_error)?;
    rows.map(|row| {
        let (path, previous, kind, old, new) = row.map_err(repository_error)?;
        ReviewFile::try_new(path, previous, kind, old, new).map_err(ReviewApplicationError::Domain)
    })
    .collect()
}

fn load_comments(
    connection: &Connection,
    review_id: &str,
) -> Result<Vec<ReviewComment>, ReviewApplicationError> {
    let mut statement = connection.prepare("SELECT id,file_path,side,start_line,end_line,hunk_fingerprint,context_fingerprint,anchor_state,body,status,selected FROM review_comments WHERE review_id=?1 ORDER BY id").map_err(repository_error)?;
    let rows = statement
        .query_map([review_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, u32>(3)?,
                row.get::<_, u32>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, bool>(10)?,
            ))
        })
        .map_err(repository_error)?;
    rows.map(|row| {
        let (id, path, side, start, end, hunk, context, state, body, status, selected) =
            row.map_err(repository_error)?;
        let mut anchor = ReviewAnchor::try_new(path, side, start, end, hunk, context)
            .map_err(ReviewApplicationError::Domain)?;
        anchor.restore_state(parse_anchor_state(&state)?);
        let mut comment =
            ReviewComment::try_new(id, anchor, body).map_err(ReviewApplicationError::Domain)?;
        comment.restore_status(parse_comment_status(&status)?, selected);
        Ok(comment)
    })
    .collect()
}

fn load_findings(
    connection: &Connection,
    review_id: &str,
) -> Result<Vec<ReviewFinding>, ReviewApplicationError> {
    let mut statement=connection.prepare("SELECT id,source,title,severity,operation_id,resolved,anchor_file_path,anchor_side,anchor_start_line,anchor_end_line,anchor_hunk_fingerprint,anchor_context_fingerprint,anchor_state FROM review_findings WHERE review_id=?1 ORDER BY id").map_err(repository_error)?;
    let rows = statement
        .query_map([review_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, bool>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<u32>>(8)?,
                row.get::<_, Option<u32>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
            ))
        })
        .map_err(repository_error)?;
    rows.map(|row| {
        let (
            id,
            source,
            title,
            severity,
            operation,
            resolved,
            path,
            side,
            start,
            end,
            hunk,
            context,
            state,
        ) = row.map_err(repository_error)?;
        let anchor = match (path, side, start, end, hunk, context, state) {
            (
                Some(path),
                Some(side),
                Some(start),
                Some(end),
                Some(hunk),
                Some(context),
                Some(state),
            ) => {
                let mut anchor = ReviewAnchor::try_new(path, side, start, end, hunk, context)
                    .map_err(ReviewApplicationError::Domain)?;
                anchor.restore_state(parse_anchor_state(&state)?);
                Some(anchor)
            }
            (None, None, None, None, None, None, None) => None,
            _ => return Err(repository_error("incomplete finding anchor")),
        };
        let mut finding = ReviewFinding::try_new(
            id,
            source,
            title,
            parse_severity(&severity)?,
            anchor,
            operation,
        )
        .map_err(ReviewApplicationError::Domain)?;
        finding.restore_resolved(resolved);
        Ok(finding)
    })
    .collect()
}

fn repository_error(error: impl fmt::Display) -> ReviewApplicationError {
    ReviewApplicationError::Repository(error.to_string())
}
use std::fmt;
fn status_value(value: ReviewStatus) -> &'static str {
    match value {
        ReviewStatus::Active => "active",
        ReviewStatus::Completed => "completed",
    }
}
fn decision_value(value: ReviewDecision) -> &'static str {
    match value {
        ReviewDecision::Pending => "pending",
        ReviewDecision::Accepted => "accepted",
        ReviewDecision::ChangesRequested => "changes_requested",
    }
}
fn anchor_state_value(value: ReviewAnchorState) -> &'static str {
    match value {
        ReviewAnchorState::Current => "current",
        ReviewAnchorState::Relocated => "relocated",
        ReviewAnchorState::Stale => "stale",
    }
}
fn comment_status_value(value: ReviewCommentStatus) -> &'static str {
    match value {
        ReviewCommentStatus::Active => "active",
        ReviewCommentStatus::Resolved => "resolved",
    }
}
fn severity_value(value: ReviewFindingSeverity) -> &'static str {
    match value {
        ReviewFindingSeverity::Info => "info",
        ReviewFindingSeverity::Warning => "warning",
        ReviewFindingSeverity::Error => "error",
    }
}
fn parse_status(v: &str) -> Result<ReviewStatus, ReviewApplicationError> {
    match v {
        "active" => Ok(ReviewStatus::Active),
        "completed" => Ok(ReviewStatus::Completed),
        _ => Err(repository_error("invalid review status")),
    }
}
fn parse_decision(v: &str) -> Result<ReviewDecision, ReviewApplicationError> {
    match v {
        "pending" => Ok(ReviewDecision::Pending),
        "accepted" => Ok(ReviewDecision::Accepted),
        "changes_requested" => Ok(ReviewDecision::ChangesRequested),
        _ => Err(repository_error("invalid review decision")),
    }
}
fn parse_anchor_state(v: &str) -> Result<ReviewAnchorState, ReviewApplicationError> {
    match v {
        "current" => Ok(ReviewAnchorState::Current),
        "relocated" => Ok(ReviewAnchorState::Relocated),
        "stale" => Ok(ReviewAnchorState::Stale),
        _ => Err(repository_error("invalid anchor state")),
    }
}
fn parse_comment_status(v: &str) -> Result<ReviewCommentStatus, ReviewApplicationError> {
    match v {
        "active" => Ok(ReviewCommentStatus::Active),
        "resolved" => Ok(ReviewCommentStatus::Resolved),
        _ => Err(repository_error("invalid comment status")),
    }
}
fn parse_severity(v: &str) -> Result<ReviewFindingSeverity, ReviewApplicationError> {
    match v {
        "info" => Ok(ReviewFindingSeverity::Info),
        "warning" => Ok(ReviewFindingSeverity::Warning),
        "error" => Ok(ReviewFindingSeverity::Error),
        _ => Err(repository_error("invalid finding severity")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn schema_and_repository_round_trip_recovered_state() {
        let directory = TempDirectory::new("review-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).unwrap();
        let connection = database.connection().unwrap();
        connection.execute_batch("INSERT INTO agents(id,display_name,provider,launch_kind) VALUES('review-agent','Review Agent','test','api'); INSERT INTO sessions(id,title,agent_id,interaction_mode,lifecycle_state,created_at,updated_at) VALUES('session-1','Review','review-agent','api','idle',datetime('now'),datetime('now'));").unwrap();
        drop(connection);
        let repository = SqliteReviewRepository::new(database.clone());
        let file =
            ReviewFile::try_new("src/a.rs".into(), None, "modified".into(), None, None).unwrap();
        let mut review = ReviewSession::try_new(
            "review-1".into(),
            "session-1".into(),
            "workspace-1".into(),
            None,
            None,
            "fingerprint".into(),
            vec![file],
        )
        .unwrap();
        let anchor = ReviewAnchor::try_new(
            "src/a.rs".into(),
            "new".into(),
            3,
            3,
            "hunk".into(),
            "context".into(),
        )
        .unwrap();
        review
            .add_comment(
                ReviewComment::try_new("comment-1".into(), anchor.clone(), "Fix it".into())
                    .unwrap(),
            )
            .unwrap();
        review
            .add_finding(
                ReviewFinding::try_new(
                    "finding-1".into(),
                    "tests".into(),
                    "Test failed".into(),
                    ReviewFindingSeverity::Warning,
                    Some(anchor),
                    "operation-1".into(),
                )
                .unwrap(),
            )
            .unwrap();
        review.set_decision(ReviewDecision::ChangesRequested);
        repository.save(&review).unwrap();
        let recovered = repository.find("review-1").unwrap().unwrap();
        assert_eq!(recovered.files()[0].path, "src/a.rs");
        assert_eq!(recovered.comments()[0].body, "Fix it");
        assert_eq!(
            recovered.findings()[0].severity,
            ReviewFindingSeverity::Warning
        );
        assert_eq!(
            recovered.findings()[0]
                .anchor
                .as_ref()
                .unwrap()
                .context_fingerprint,
            "context"
        );
        assert_eq!(recovered.decision, ReviewDecision::ChangesRequested);
        assert_eq!(
            repository
                .find_active_by_session("session-1")
                .unwrap()
                .unwrap()
                .id,
            "review-1"
        );
    }

    #[test]
    fn schema_migration_is_transactional_when_following_statement_fails() {
        let connection = Connection::open_in_memory().unwrap();
        let transaction = connection.unchecked_transaction().unwrap();
        apply_schema(&transaction).unwrap();
        assert!(transaction
            .execute("INSERT INTO missing_table VALUES(1)", [])
            .is_err());
        transaction.rollback().unwrap();
        let exists: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='review_sessions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(exists, 0);
    }
}
