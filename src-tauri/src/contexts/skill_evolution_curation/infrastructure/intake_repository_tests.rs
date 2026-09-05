use super::*;
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::{params, Connection};
use std::{
    sync::{Arc, Barrier},
    time::Duration,
};

fn apply_fixture_schema(connection: &Connection) {
    connection.execute_batch("PRAGMA foreign_keys=ON;
        CREATE TABLE evolution_candidate_seeds (seed_id TEXT PRIMARY KEY,workspace TEXT);
        CREATE TABLE evolution_signals (signal_id TEXT PRIMARY KEY,sanitizer_version INTEGER NOT NULL);
        CREATE TABLE evolution_assessment_attempts (
            attempt_id TEXT PRIMARY KEY,seed_id TEXT NOT NULL,seed_revision TEXT NOT NULL,status TEXT NOT NULL,
            route TEXT,confidence TEXT,risk TEXT,witness_hash TEXT NOT NULL,lineage_hash TEXT NOT NULL,is_current INTEGER NOT NULL);
        CREATE TABLE evolution_assessment_targets (
            attempt_id TEXT NOT NULL,ordinal INTEGER NOT NULL,skill_id TEXT NOT NULL,revision_hash TEXT NOT NULL,scope TEXT NOT NULL);
        CREATE TABLE evolution_assessment_checks (
            attempt_id TEXT NOT NULL,ordinal INTEGER NOT NULL,kind TEXT NOT NULL,result TEXT NOT NULL,reason_code TEXT NOT NULL);
        CREATE TABLE evolution_assessment_evidence_links (attempt_id TEXT NOT NULL,evidence_id TEXT NOT NULL,relation TEXT NOT NULL);")
        .expect("source schemas");
    apply_schema(connection).expect("curator schema");
}

fn insert_assessment(
    connection: &Connection,
    id: &str,
    witness: &str,
    route: &str,
    target: (&str, &str),
    current: bool,
    evidence_present: bool,
) {
    connection
        .execute(
            "INSERT OR IGNORE INTO evolution_candidate_seeds VALUES ('seed-1','workspace:one')",
            [],
        )
        .expect("seed");
    if evidence_present {
        connection
            .execute(
                "INSERT OR IGNORE INTO evolution_signals VALUES ('evidence-1',7)",
                [],
            )
            .expect("evidence");
    }
    connection.execute("INSERT INTO evolution_assessment_attempts VALUES (?1,'seed-1',?2,'completed',?3,'high','low',?2,'lineage-1',?4)",
        params![id, witness, route, i64::from(current)]).expect("assessment");
    connection
        .execute(
            "INSERT INTO evolution_assessment_targets VALUES (?1,0,?2,?3,'project')",
            params![id, target.0, target.1],
        )
        .expect("target");
    connection
        .execute(
            "INSERT INTO evolution_assessment_evidence_links VALUES (?1,'evidence-1','primary')",
            [id],
        )
        .expect("link");
    for ordinal in 0..9 {
        connection
            .execute(
                "INSERT INTO evolution_assessment_checks VALUES (?1,?2,?3,'pass','check_passed')",
                params![id, ordinal, format!("check-{ordinal}")],
            )
            .expect("check");
    }
}

fn envelope(
    id: &str,
    witness: &str,
    route: CuratorAssessmentRoute,
    current: bool,
) -> AssessmentCompletionEnvelopeV1 {
    AssessmentCompletionEnvelopeV1 {
        schema_version: 1,
        assessment_attempt_id: id.into(),
        assessment_revision: witness.into(),
        current,
        route,
        witness_hash: witness.into(),
    }
}

#[test]
fn duplicate_delivery_reuses_candidate_and_preserves_full_snapshot() {
    let mut connection = Connection::open_in_memory().expect("database");
    apply_fixture_schema(&connection);
    insert_assessment(
        &connection,
        "assessment-1",
        "witness-1",
        "advance",
        ("code-review", "target-1"),
        true,
        true,
    );
    let input = envelope(
        "assessment-1",
        "witness-1",
        CuratorAssessmentRoute::Advance,
        true,
    );
    let first = SqliteCuratorIntakeRepository::new(&mut connection)
        .consume(&input, 10)
        .expect("first intake");
    let candidate_id = match first {
        CuratorIntakeOutcome::CandidateCreated { candidate_id } => candidate_id,
        other => panic!("unexpected {other:?}"),
    };
    assert_eq!(
        SqliteCuratorIntakeRepository::new(&mut connection).consume(&input, 11),
        Ok(CuratorIntakeOutcome::ExistingCandidate {
            candidate_id: candidate_id.clone()
        })
    );
    let (candidates, receipts, events): (i64, i64, i64) = (
        connection
            .query_row(
                "SELECT COUNT(*) FROM evolution_curator_candidates",
                [],
                |row| row.get(0),
            )
            .expect("candidates"),
        connection
            .query_row(
                "SELECT COUNT(*) FROM evolution_curator_intake_receipts",
                [],
                |row| row.get(0),
            )
            .expect("receipts"),
        connection
            .query_row("SELECT COUNT(*) FROM evolution_curator_events", [], |row| {
                row.get(0)
            })
            .expect("events"),
    );
    assert_eq!((candidates, receipts, events), (1, 1, 1));
    let snapshot_json: String = connection
        .query_row(
            "SELECT snapshot_json FROM evolution_curator_candidates WHERE candidate_id=?1",
            [&candidate_id],
            |row| row.get(0),
        )
        .expect("snapshot");
    let snapshot: CuratorCandidateSnapshot =
        serde_json::from_str(&snapshot_json).expect("typed snapshot");
    assert_eq!(snapshot.quality_checks.len(), 9);
    assert_eq!(
        snapshot.evidence_sources[0].evidence_revision,
        "sanitizer-v7"
    );
    assert!(!snapshot.policy_witness_hash.is_empty());
}

#[test]
fn non_approvable_non_current_and_purged_inputs_are_receipted_without_candidates() {
    for (name, route, route_wire, current, evidence, expected) in [
        (
            "drop",
            CuratorAssessmentRoute::Drop,
            "drop",
            true,
            true,
            CuratorIntakeOutcome::NonApprovableRecorded,
        ),
        (
            "stale",
            CuratorAssessmentRoute::Advance,
            "advance",
            false,
            true,
            CuratorIntakeOutcome::NonCurrentRejected,
        ),
        (
            "purged",
            CuratorAssessmentRoute::Advance,
            "advance",
            true,
            false,
            CuratorIntakeOutcome::PurgedEvidenceRejected,
        ),
    ] {
        let mut connection = Connection::open_in_memory().expect("database");
        apply_fixture_schema(&connection);
        insert_assessment(
            &connection,
            name,
            &format!("witness-{name}"),
            route_wire,
            ("code-review", "target-1"),
            current,
            evidence,
        );
        let outcome = SqliteCuratorIntakeRepository::new(&mut connection)
            .consume(
                &envelope(name, &format!("witness-{name}"), route, current),
                10,
            )
            .expect("intake");
        assert_eq!(outcome, expected);
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM evolution_curator_candidates",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .expect("count"),
            0
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM evolution_curator_intake_receipts",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .expect("receipt"),
            1
        );
    }
}

#[test]
fn reassessment_supersedes_and_links_the_prior_open_candidate() {
    let mut connection = Connection::open_in_memory().expect("database");
    apply_fixture_schema(&connection);
    insert_assessment(
        &connection,
        "assessment-1",
        "witness-1",
        "advance",
        ("code-review", "target-1"),
        true,
        true,
    );
    let first = SqliteCuratorIntakeRepository::new(&mut connection)
        .consume(
            &envelope(
                "assessment-1",
                "witness-1",
                CuratorAssessmentRoute::Advance,
                true,
            ),
            10,
        )
        .expect("first");
    let first_id = match first {
        CuratorIntakeOutcome::CandidateCreated { candidate_id } => candidate_id,
        _ => panic!("candidate expected"),
    };
    connection
        .execute(
            "UPDATE evolution_assessment_attempts SET is_current=0 WHERE attempt_id='assessment-1'",
            [],
        )
        .expect("supersede assessment");
    insert_assessment(
        &connection,
        "assessment-2",
        "witness-2",
        "needs_human_review",
        ("security-review", "target-2"),
        true,
        true,
    );
    let second = SqliteCuratorIntakeRepository::new(&mut connection)
        .consume(
            &envelope(
                "assessment-2",
                "witness-2",
                CuratorAssessmentRoute::NeedsHumanReview,
                true,
            ),
            20,
        )
        .expect("second");
    let second_id = match second {
        CuratorIntakeOutcome::CandidateCreated { candidate_id } => candidate_id,
        _ => panic!("candidate expected"),
    };
    let (state, successor): (String, Option<String>) = connection.query_row(
        "SELECT state,superseded_by_candidate_id FROM evolution_curator_candidates WHERE candidate_id=?1", [&first_id],
        |row| Ok((row.get(0)?, row.get(1)?))).expect("prior candidate");
    assert_eq!(state, "superseded");
    assert_eq!(successor.as_deref(), Some(second_id.as_str()));
    verify_audit_chain(&connection, &first_id).expect("prior audit chain");
}

#[test]
fn concurrent_intake_creates_exactly_one_candidate() {
    let directory = crate::test_support::TempDirectory::new("curator-intake-concurrent");
    let path = directory.path().join("curator.sqlite");
    let connection = Connection::open(&path).expect("database");
    apply_fixture_schema(&connection);
    insert_assessment(
        &connection,
        "assessment-1",
        "witness-1",
        "advance",
        ("code-review", "target-1"),
        true,
        true,
    );
    drop(connection);
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let path = path.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            let mut connection = Connection::open(path).expect("worker database");
            connection
                .busy_timeout(Duration::from_secs(2))
                .expect("busy timeout");
            barrier.wait();
            SqliteCuratorIntakeRepository::new(&mut connection).consume(
                &envelope(
                    "assessment-1",
                    "witness-1",
                    CuratorAssessmentRoute::Advance,
                    true,
                ),
                10,
            )
        }));
    }
    barrier.wait();
    let outcomes = handles
        .into_iter()
        .map(|handle| handle.join().expect("thread").expect("intake"))
        .collect::<Vec<_>>();
    assert!(outcomes
        .iter()
        .any(|value| matches!(value, CuratorIntakeOutcome::CandidateCreated { .. })));
    assert!(outcomes
        .iter()
        .any(|value| matches!(value, CuratorIntakeOutcome::ExistingCandidate { .. })));
    let connection = Connection::open(path).expect("verification database");
    assert_eq!(
        connection
            .query_row(
                "SELECT COUNT(*) FROM evolution_curator_candidates",
                [],
                |row| row.get::<_, i64>(0)
            )
            .expect("candidate count"),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM evolution_curator_events", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("event count"),
        1
    );
}
