use rusqlite::OptionalExtension;

use crate::contexts::agent_runtime::application::{
    LoopVerificationEvidenceFact, LoopVerificationEvidencePort,
};
use crate::contexts::skill_evolution_evidence::application::{
    PlanVerificationFact, RuntimeEvidenceProjector,
};
use crate::contexts::skill_evolution_evidence::domain::{
    EnvelopeCommon, SourceFidelity, VerificationClass, VerificationOutcome,
};
use crate::platform::database::NativeDatabase;

#[derive(Clone)]
pub(crate) struct RuntimeLoopVerificationEvidenceAdapter {
    evidence: RuntimeEvidenceProjector,
    database: NativeDatabase,
}

impl RuntimeLoopVerificationEvidenceAdapter {
    pub(crate) fn new(evidence: RuntimeEvidenceProjector, database: NativeDatabase) -> Self {
        Self { evidence, database }
    }

    fn predecessor(&self, iteration_id: &str) -> Option<String> {
        self.database
            .connection()
            .ok()?
            .query_row(
                r#"SELECT previous.id
                   FROM loop_iterations current
                   JOIN loop_iterations previous
                     ON previous.run_id = current.run_id
                    AND previous.sequence = current.sequence - 1
                   WHERE current.id = ?1"#,
                [iteration_id],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()
    }
}

impl LoopVerificationEvidencePort for RuntimeLoopVerificationEvidenceAdapter {
    fn project(&self, fact: LoopVerificationEvidenceFact) {
        let outcome = if fact.cancelled {
            VerificationOutcome::Skipped
        } else if fact.failed_count == 0 {
            VerificationOutcome::Passed
        } else {
            VerificationOutcome::Failed
        };
        let predecessor_attempt_id = self.predecessor(&fact.iteration_id);
        let _ = self.evidence.verification(PlanVerificationFact {
            common: EnvelopeCommon {
                source_event_id: format!("verification:{}:{}", fact.run_id, fact.iteration_id),
                occurred_at: fact.occurred_at,
                stable_agent_id: None,
                session_id: None,
                message_id: None,
                run_id: Some(fact.run_id.clone()),
                attempt_id: Some(fact.iteration_id.clone()),
                workspace: self.evidence.workspace_scope(Some(&fact.workspace)),
                fidelity: SourceFidelity::Native,
                observed_skill_revisions: Vec::new(),
            },
            plan_run_id: fact.run_id,
            verifier: VerificationClass::Plan,
            outcome,
            passed_count: fact.passed_count,
            failed_count: fact.failed_count,
            predecessor_attempt_id,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn predecessor_comes_from_the_authoritative_iteration_sequence() {
        let directory = TempDirectory::new("loop-evidence-predecessor");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let connection = database.connection().expect("connection");
        connection
            .execute_batch(
                r#"PRAGMA foreign_keys = OFF;
                   INSERT INTO loop_iterations
                     (id, run_id, sequence, status, started_at)
                   VALUES
                     ('iteration-1', 'run-1', 1, 'completed', '2026-08-13T10:00:00Z'),
                     ('iteration-2', 'run-1', 2, 'running', '2026-08-13T10:01:00Z');"#,
            )
            .expect("iteration fixtures");
        drop(connection);
        let adapter = RuntimeLoopVerificationEvidenceAdapter::new(
            RuntimeEvidenceProjector::disabled(),
            database,
        );

        assert_eq!(
            adapter.predecessor("iteration-2").as_deref(),
            Some("iteration-1")
        );
        assert_eq!(adapter.predecessor("iteration-1"), None);
    }
}
