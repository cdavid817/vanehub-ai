use rusqlite::Transaction;

use super::EvidenceRepositoryError;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct DeletedEvidence {
    pub(super) signals: usize,
    pub(super) seeds: usize,
    pub(super) feedback: usize,
}

pub(super) fn delete_signal_set(
    transaction: &Transaction<'_>,
    signal_ids: &[String],
) -> Result<DeletedEvidence, EvidenceRepositoryError> {
    let mut deleted = DeletedEvidence::default();
    for signal_id in signal_ids {
        deleted.feedback += transaction.execute(
            "DELETE FROM evolution_feedback_current WHERE active_signal_id = ?1",
            [signal_id],
        )?;
        deleted.feedback += transaction.execute(
            "DELETE FROM evolution_feedback_events WHERE signal_id = ?1",
            [signal_id],
        )?;
        deleted.seeds += transaction.execute(
            "DELETE FROM evolution_candidate_seeds WHERE seed_id IN (SELECT seed_id FROM evolution_candidate_seed_signals WHERE signal_id = ?1)",
            [signal_id],
        )?;
        deleted.signals += transaction.execute(
            "DELETE FROM evolution_signals WHERE signal_id = ?1",
            [signal_id],
        )?;
    }
    if deleted.signals > 0 {
        transaction.execute(
            "UPDATE evolution_pipeline_state SET state_value = 'dirty', revision = revision + 1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE state_key = 'signal_set'",
            [],
        )?;
    }
    Ok(deleted)
}
