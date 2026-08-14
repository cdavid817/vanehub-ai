use rusqlite::{params, Transaction, TransactionBehavior};
use serde::Serialize;

use super::deletion::delete_signal_set;
use super::{EvidenceRepositoryError, SqliteEvolutionEvidenceRepository};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EvidencePurgeScope {
    All,
    Workspace(String),
    Skill(String),
    WorkspaceSkill { workspace: String, skill_id: String },
    Conversation(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvidencePurgeRequest {
    pub(crate) operation_id: String,
    pub(crate) scope: EvidencePurgeScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvidencePurgeOutcome {
    pub(crate) operation_id: String,
    pub(crate) deleted_signals: usize,
    pub(crate) deleted_seeds: usize,
    pub(crate) deleted_feedback: usize,
}

impl SqliteEvolutionEvidenceRepository {
    pub(crate) fn purge(
        &self,
        request: &EvidencePurgeRequest,
    ) -> Result<EvidencePurgeOutcome, EvidenceRepositoryError> {
        let mut connection = self
            .database
            .connection()
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let signal_ids = scoped_signal_ids(&transaction, &request.scope)?;
        let deleted = delete_signal_set(&transaction, &signal_ids)?;
        let mut deleted_seeds = deleted.seeds;
        let mut deleted_feedback = deleted.feedback;
        if request.scope == EvidencePurgeScope::All {
            deleted_seeds += transaction.execute("DELETE FROM evolution_candidate_seeds", [])?;
            deleted_feedback +=
                transaction.execute("DELETE FROM evolution_feedback_current", [])?;
            deleted_feedback += transaction.execute("DELETE FROM evolution_feedback_events", [])?;
        }
        transaction.commit()?;
        Ok(EvidencePurgeOutcome {
            operation_id: request.operation_id.clone(),
            deleted_signals: deleted.signals,
            deleted_seeds,
            deleted_feedback,
        })
    }
}

fn scoped_signal_ids(
    transaction: &Transaction<'_>,
    scope: &EvidencePurgeScope,
) -> Result<Vec<String>, EvidenceRepositoryError> {
    let (sql, value) = match scope {
        EvidencePurgeScope::All => (
            "SELECT signal_id FROM evolution_signals ORDER BY signal_id",
            None,
        ),
        EvidencePurgeScope::Workspace(workspace) => (
            "SELECT signal_id FROM evolution_signals WHERE workspace = ?1 ORDER BY signal_id",
            Some(workspace.as_str()),
        ),
        EvidencePurgeScope::Skill(skill_id) => (
            "SELECT DISTINCT signal_id FROM evolution_signal_skill_associations WHERE skill_id = ?1 ORDER BY signal_id",
            Some(skill_id.as_str()),
        ),
        EvidencePurgeScope::WorkspaceSkill {
            workspace,
            skill_id,
        } => {
            let mut statement = transaction.prepare(
                "SELECT DISTINCT signal.signal_id FROM evolution_signals signal JOIN evolution_signal_skill_associations association ON association.signal_id = signal.signal_id WHERE signal.workspace = ?1 AND association.skill_id = ?2 ORDER BY signal.signal_id",
            )?;
            return Ok(statement
                .query_map(params![workspace, skill_id], map_string)?
                .collect::<Result<Vec<_>, _>>()?);
        }
        EvidencePurgeScope::Conversation(conversation_id) => (
            "SELECT DISTINCT signal_id FROM evolution_signal_source_links WHERE source_kind IN ('session', 'message') AND source_id = ?1 ORDER BY signal_id",
            Some(conversation_id.as_str()),
        ),
    };
    let mut statement = transaction.prepare(sql)?;
    let values = if let Some(value) = value {
        statement
            .query_map(params![value], map_string)?
            .collect::<Result<Vec<_>, _>>()?
    } else {
        statement
            .query_map([], map_string)?
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(values)
}

fn map_string(row: &rusqlite::Row<'_>) -> rusqlite::Result<String> {
    row.get(0)
}
