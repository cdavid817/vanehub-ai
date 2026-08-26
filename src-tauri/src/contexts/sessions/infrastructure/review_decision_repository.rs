use crate::contexts::sessions::application::{ReviewApplicationError, ReviewDecisionRepository};
use crate::contexts::sessions::domain::{ReviewDecision, ReviewHunkDecision};
use crate::platform::database::NativeDatabase;
use rusqlite::params;

/// Hunk decisions, written one row at a time.
///
/// Deliberately not part of `SqliteReviewRepository`, whose `save` deletes and rewrites every file,
/// comment, and finding a review holds. Sharing it would make marking one hunk rewrite the review's
/// own decision — the exact coupling the separate table and the separate port exist to prevent.
#[derive(Clone)]
pub(crate) struct SqliteReviewDecisionRepository {
    database: NativeDatabase,
}

impl SqliteReviewDecisionRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl ReviewDecisionRepository for SqliteReviewDecisionRepository {
    fn upsert_hunk_decision(
        &self,
        review_id: &str,
        decision: &ReviewHunkDecision,
    ) -> Result<(), ReviewApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        connection
            .execute(
                "INSERT INTO review_hunk_decisions \
                 (review_id, relative_path, hunk_fingerprint, snapshot_fingerprint, decision, \
                  decided_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                 ON CONFLICT(review_id, relative_path, hunk_fingerprint) DO UPDATE SET \
                 snapshot_fingerprint = excluded.snapshot_fingerprint, \
                 decision = excluded.decision, \
                 decided_at = excluded.decided_at",
                params![
                    review_id,
                    decision.path,
                    decision.hunk_fingerprint,
                    decision.snapshot_fingerprint,
                    decision_value(decision.decision),
                    decision.decided_at,
                ],
            )
            .map_err(repository_error)?;
        Ok(())
    }

    fn list_hunk_decisions(
        &self,
        review_id: &str,
    ) -> Result<Vec<ReviewHunkDecision>, ReviewApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        let mut statement = connection
            .prepare(
                "SELECT relative_path, hunk_fingerprint, snapshot_fingerprint, decision, decided_at \
                 FROM review_hunk_decisions WHERE review_id = ?1 \
                 ORDER BY relative_path, hunk_fingerprint",
            )
            .map_err(repository_error)?;
        let rows = statement
            .query_map(params![review_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(repository_error)?;

        let mut decisions = Vec::new();
        for row in rows {
            let (path, hunk_fingerprint, snapshot_fingerprint, decision, decided_at) =
                row.map_err(repository_error)?;
            decisions.push(ReviewHunkDecision::try_new(
                path,
                hunk_fingerprint,
                snapshot_fingerprint,
                parse_decision(&decision)?,
                decided_at,
            )?);
        }
        Ok(decisions)
    }
}

fn decision_value(decision: ReviewDecision) -> &'static str {
    match decision {
        ReviewDecision::Pending => "pending",
        ReviewDecision::Accepted => "accepted",
        ReviewDecision::ChangesRequested => "changes_requested",
    }
}

/// A stored value outside the three is corruption, not a fourth state.
///
/// The column has a CHECK that makes it unreachable through this application, so reading one back
/// means the row came from somewhere else. Refusing beats defaulting to `Pending`, which would
/// present tampered data as "nobody has decided".
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the write path is live; reading decisions back arrives with 13.6's \
                 review summary counts"
    )
)]
fn parse_decision(value: &str) -> Result<ReviewDecision, ReviewApplicationError> {
    match value {
        "pending" => Ok(ReviewDecision::Pending),
        "accepted" => Ok(ReviewDecision::Accepted),
        "changes_requested" => Ok(ReviewDecision::ChangesRequested),
        other => Err(ReviewApplicationError::Repository(format!(
            "unknown hunk decision `{other}`"
        ))),
    }
}

fn repository_error(error: impl std::fmt::Display) -> ReviewApplicationError {
    ReviewApplicationError::Repository(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    /// A database holding one review to record decisions against.
    fn reviewed(name: &str) -> (TempDirectory, SqliteReviewDecisionRepository) {
        let directory = TempDirectory::new(name);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let connection = database.connection().expect("connection");
        connection
            .execute_batch(
                "INSERT INTO agents(id,display_name,provider,launch_kind) \
                 VALUES('review-agent','Review Agent','test','api'); \
                 INSERT INTO sessions(id,title,agent_id,interaction_mode,lifecycle_state,\
                 created_at,updated_at) \
                 VALUES('session-1','Review','review-agent','api','idle',\
                 datetime('now'),datetime('now')); \
                 INSERT INTO review_sessions(id,session_id,workspace_id,base_revision,\
                 head_revision,fingerprint,status,decision,created_at,updated_at) \
                 VALUES('review-1','session-1','workspace-1',NULL,NULL,'snapshot-a','active',\
                 'pending',datetime('now'),datetime('now'));",
            )
            .expect("seed review");
        drop(connection);
        (directory, SqliteReviewDecisionRepository::new(database))
    }

    fn decision(path: &str, hunk: &str, value: ReviewDecision) -> ReviewHunkDecision {
        ReviewHunkDecision::try_new(
            path.into(),
            hunk.into(),
            "snapshot-a".into(),
            value,
            "2026-08-27T00:00:00Z".into(),
        )
        .expect("decision")
    }

    #[test]
    fn a_decision_round_trips_through_the_table() {
        let (_directory, repository) = reviewed("review-decisions-round-trip");
        let recorded = decision("src/a.rs", "hunk-1", ReviewDecision::ChangesRequested);

        repository
            .upsert_hunk_decision("review-1", &recorded)
            .expect("upsert");

        assert_eq!(
            repository.list_hunk_decisions("review-1").expect("list"),
            vec![recorded]
        );
    }

    #[test]
    fn recording_the_same_hunk_again_replaces_it_rather_than_adding_a_second() {
        let (_directory, repository) = reviewed("review-decisions-upsert");
        repository
            .upsert_hunk_decision(
                "review-1",
                &decision("src/a.rs", "hunk-1", ReviewDecision::Accepted),
            )
            .expect("first");
        repository
            .upsert_hunk_decision(
                "review-1",
                &decision("src/a.rs", "hunk-1", ReviewDecision::ChangesRequested),
            )
            .expect("second");

        let rows = repository.list_hunk_decisions("review-1").expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].decision, ReviewDecision::ChangesRequested);
    }

    #[test]
    fn a_decision_is_listed_in_an_order_that_does_not_depend_on_when_it_was_made() {
        let (_directory, repository) = reviewed("review-decisions-order");
        for (path, hunk) in [
            ("src/b.rs", "hunk-2"),
            ("src/a.rs", "hunk-9"),
            ("src/a.rs", "hunk-1"),
        ] {
            repository
                .upsert_hunk_decision("review-1", &decision(path, hunk, ReviewDecision::Accepted))
                .expect("upsert");
        }

        // Path then fingerprint. Insertion order would make the same review render its hunks in a
        // different order for every reviewer, which reads as the diff having changed.
        let listed: Vec<(String, String)> = repository
            .list_hunk_decisions("review-1")
            .expect("list")
            .into_iter()
            .map(|row| (row.path, row.hunk_fingerprint))
            .collect();
        assert_eq!(
            listed,
            vec![
                ("src/a.rs".to_string(), "hunk-1".to_string()),
                ("src/a.rs".to_string(), "hunk-9".to_string()),
                ("src/b.rs".to_string(), "hunk-2".to_string()),
            ]
        );
    }

    #[test]
    fn a_decision_does_not_touch_the_review_it_belongs_to() {
        let (directory, repository) = reviewed("review-decisions-independent");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen");
        repository
            .upsert_hunk_decision(
                "review-1",
                &decision("src/a.rs", "hunk-1", ReviewDecision::Accepted),
            )
            .expect("upsert");

        // The review row is what `ReviewRepository::save` rewrites wholesale. If a hunk write ever
        // travelled through it, this is where accepting one hunk would show up as an accepted
        // review.
        let (review_decision, updated_at): (String, String) = database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT decision, updated_at FROM review_sessions WHERE id = 'review-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("review row");
        assert_eq!(review_decision, "pending");
        assert!(!updated_at.is_empty());
    }

    #[test]
    fn a_stored_value_outside_the_three_is_refused_rather_than_read_as_pending() {
        let (directory, repository) = reviewed("review-decisions-corrupt");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen");
        // Written past the CHECK, which is the only way this row can exist: it stands in for a
        // database somebody edited by hand or a future value this binary does not know.
        database
            .connection()
            .expect("connection")
            .execute_batch(
                "PRAGMA ignore_check_constraints = ON; \
                 INSERT INTO review_hunk_decisions \
                 (review_id, relative_path, hunk_fingerprint, snapshot_fingerprint, decision, \
                  decided_at) \
                 VALUES ('review-1', 'src/a.rs', 'hunk-1', 'snapshot-a', 'deferred', \
                 '2026-08-27T00:00:00Z');",
            )
            .expect("tampered row");

        // Defaulting to `Pending` would present a value nobody chose as "nobody has decided".
        assert!(repository.list_hunk_decisions("review-1").is_err());
    }
}
