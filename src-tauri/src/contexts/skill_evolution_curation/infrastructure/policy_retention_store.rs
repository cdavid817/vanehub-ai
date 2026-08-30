use super::SqliteCuratorRepository;
use super::{policy_retention_purge::*, policy_retention_support::*};
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::{OptionalExtension, TransactionBehavior};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorPolicyUpdateOutcome {
    pub(crate) policy: CuratorPolicyV1,
    pub(crate) policy_hash: String,
    pub(crate) affected_candidates: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CuratorRetentionReport {
    pub(crate) expired_open_candidates: u64,
    pub(crate) purged_terminal_details: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CuratorEvidencePurgeReport {
    pub(crate) redacted_candidates: u64,
    pub(crate) superseded_open_candidates: u64,
    pub(crate) preserved_applied_tombstones: u64,
    pub(crate) skipped_applying_candidates: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorAppliedTombstone {
    pub(crate) candidate_id: String,
    pub(crate) decision_id: String,
    pub(crate) overlay_revision: String,
    pub(crate) overlay_history_id: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum CuratorPolicyRetentionError {
    #[error(transparent)]
    Policy(#[from] CuratorPolicyValidationError),
    #[error("curator workspace policy changed concurrently at revision {current_revision}")]
    Conflict { current_revision: u64 },
    #[error("curator policy or retention input is invalid")]
    InvalidInput,
    #[error("curator policy or retention persistence failed")]
    Storage,
    #[error("curator applied tombstone was not found")]
    NotFound,
}

impl SqliteCuratorRepository<'_> {
    pub(crate) fn load_policy(
        &self,
        workspace_id: &str,
    ) -> Result<CuratorPolicyV1, CuratorPolicyRetentionError> {
        if workspace_id.trim().is_empty() {
            return Err(CuratorPolicyRetentionError::InvalidInput);
        }
        load_policy_from_connection(self.connection, workspace_id)
    }

    pub(crate) fn update_policy(
        &mut self,
        workspace_id: &str,
        expected_revision: u64,
        update: CuratorPolicyUpdateV1,
        occurred_at_ms: i64,
    ) -> Result<CuratorPolicyUpdateOutcome, CuratorPolicyRetentionError> {
        if workspace_id.trim().is_empty() || expected_revision == 0 || occurred_at_ms < 0 {
            return Err(CuratorPolicyRetentionError::InvalidInput);
        }
        update.validate()?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorPolicyRetentionError::Storage)?;
        let current = load_policy_from_transaction(&transaction, workspace_id)?;
        if current.revision != expected_revision {
            return Err(CuratorPolicyRetentionError::Conflict {
                current_revision: current.revision,
            });
        }
        let revision = expected_revision
            .checked_add(1)
            .ok_or(CuratorPolicyRetentionError::InvalidInput)?;
        let policy = update.materialize(workspace_id.to_string(), revision);
        let hash = policy_hash(&policy)?;
        persist_policy(&transaction, &policy, &hash, occurred_at_ms)?;
        let affected = rebind_open_candidates(&transaction, workspace_id, &hash, occurred_at_ms)?;
        transaction
            .commit()
            .map_err(|_| CuratorPolicyRetentionError::Storage)?;
        Ok(CuratorPolicyUpdateOutcome {
            policy,
            policy_hash: hash,
            affected_candidates: affected,
        })
    }

    pub(crate) fn run_retention(
        &mut self,
        workspace_id: &str,
        now_ms: i64,
    ) -> Result<CuratorRetentionReport, CuratorPolicyRetentionError> {
        if workspace_id.trim().is_empty() || now_ms < 0 {
            return Err(CuratorPolicyRetentionError::InvalidInput);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorPolicyRetentionError::Storage)?;
        let policy = load_policy_from_transaction(&transaction, workspace_id)?;
        let open_cutoff = retention_cutoff(now_ms, policy.open_retention_days)?;
        let terminal_cutoff = retention_cutoff(now_ms, policy.terminal_retention_days)?;
        let open = candidate_ids(
            &transaction,
            workspace_id,
            "state IN ('pending','awaiting_draft','ready_for_review','deferred','apply_failed') AND updated_at_ms<=?2",
            open_cutoff,
        )?;
        let terminal = candidate_ids(
            &transaction,
            workspace_id,
            "state IN ('applied','rejected','superseded') AND updated_at_ms<=?2",
            terminal_cutoff,
        )?;
        for candidate_id in &open {
            expire_open_candidate(
                &transaction,
                candidate_id,
                now_ms,
                "open_candidate_retention_expired",
            )?;
        }
        for candidate_id in &terminal {
            purge_candidate_detail(
                &transaction,
                candidate_id,
                now_ms,
                "terminal_detail_retention",
            )?;
        }
        transaction
            .commit()
            .map_err(|_| CuratorPolicyRetentionError::Storage)?;
        Ok(CuratorRetentionReport {
            expired_open_candidates: open.len() as u64,
            purged_terminal_details: terminal.len() as u64,
        })
    }

    pub(crate) fn purge_evidence(
        &mut self,
        evidence_id: &str,
        occurred_at_ms: i64,
    ) -> Result<CuratorEvidencePurgeReport, CuratorPolicyRetentionError> {
        if evidence_id.trim().is_empty() || occurred_at_ms < 0 {
            return Err(CuratorPolicyRetentionError::InvalidInput);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorPolicyRetentionError::Storage)?;
        let candidates = evidence_candidates(&transaction, evidence_id)?;
        let mut report = CuratorEvidencePurgeReport::default();
        for (candidate_id, state) in candidates {
            if state == CuratorCandidateState::Applying {
                report.skipped_applying_candidates += 1;
                continue;
            }
            if !is_terminal(state) {
                expire_open_candidate(
                    &transaction,
                    &candidate_id,
                    occurred_at_ms,
                    "source_evidence_purged",
                )?;
                report.superseded_open_candidates += 1;
            } else {
                purge_candidate_detail(
                    &transaction,
                    &candidate_id,
                    occurred_at_ms,
                    "source_evidence_purged",
                )?;
            }
            report.redacted_candidates += 1;
            if state == CuratorCandidateState::Applied {
                report.preserved_applied_tombstones += 1;
            }
        }
        transaction
            .commit()
            .map_err(|_| CuratorPolicyRetentionError::Storage)?;
        Ok(report)
    }

    pub(crate) fn applied_tombstone(
        &self,
        candidate_id: &str,
    ) -> Result<CuratorAppliedTombstone, CuratorPolicyRetentionError> {
        self.connection
            .query_row(
                "SELECT a.candidate_id,a.decision_id,a.overlay_revision,a.overlay_history_id
                 FROM evolution_curator_applications a JOIN evolution_curator_candidates c
                   ON c.candidate_id=a.candidate_id
                 WHERE a.candidate_id=?1 AND c.state='applied' AND a.overlay_revision IS NOT NULL
                   AND a.overlay_history_id IS NOT NULL ORDER BY a.updated_at_ms DESC LIMIT 1",
                [candidate_id],
                |row| {
                    Ok(CuratorAppliedTombstone {
                        candidate_id: row.get(0)?,
                        decision_id: row.get(1)?,
                        overlay_revision: row.get(2)?,
                        overlay_history_id: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(|_| CuratorPolicyRetentionError::Storage)?
            .ok_or(CuratorPolicyRetentionError::NotFound)
    }
}
