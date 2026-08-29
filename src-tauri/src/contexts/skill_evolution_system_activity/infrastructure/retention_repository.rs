use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    ActivityProjectionRepositoryError, SqliteActivityProjectionRepository, LOCAL_ACTIVITY_USER_ID,
};
use crate::contexts::skill_evolution_system_activity::domain::*;

const DAY_MS: i64 = 86_400_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityRetentionReport {
    pub(crate) removed_items: u64,
    pub(crate) redacted_payloads: u64,
    pub(crate) preserved_mandatory_items: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivitySourcePurgeReport {
    pub(crate) removed_detail_items: u64,
    pub(crate) preserved_tombstones: u64,
}

impl SqliteActivityProjectionRepository<'_> {
    pub(crate) fn apply_detail_retention(
        &self,
        session_id: &str,
        now_ms: i64,
    ) -> Result<ActivityRetentionReport, ActivityProjectionRepositoryError> {
        validate_input(session_id, "retention.session_id", now_ms)?;
        let transaction = self.connection.unchecked_transaction()?;
        let retention_days = transaction
            .query_row(
                "SELECT p.detail_retention_days FROM evolution_system_activity_sessions s
                 JOIN evolution_activity_preferences p
                   ON p.scope_kind=s.scope_kind AND p.canonical_scope_id=s.canonical_scope_id
                 WHERE s.session_id=?1",
                [session_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .ok_or(ActivityProjectionRepositoryError::InvalidInput)?;
        let cutoff = now_ms
            .checked_sub(retention_days.saturating_mul(DAY_MS))
            .ok_or(ActivityProjectionRepositoryError::InvalidInput)?;
        let removable = removable_event_ids(&transaction, session_id, cutoff)?;
        let preserved_mandatory_items = mandatory_item_count(&transaction, session_id, cutoff)?;
        let mut removed_items = 0_u64;
        let mut redacted_payloads = 0_u64;
        for event_id in &removable {
            removed_items += transaction.execute(
                "DELETE FROM evolution_activity_items WHERE session_id=?1 AND event_id=?2",
                params![session_id, event_id],
            )? as u64;
            redacted_payloads += transaction.execute(
                "UPDATE evolution_activity_envelopes SET payload_json=NULL
                 WHERE event_id=?1 AND payload_json IS NOT NULL",
                [event_id],
            )? as u64;
            transaction.execute(
                "DELETE FROM evolution_activity_safe_identities WHERE event_id=?1",
                [event_id],
            )?;
        }
        refresh_session_summary(&transaction, session_id)?;
        transaction.commit()?;
        Ok(ActivityRetentionReport {
            removed_items,
            redacted_payloads,
            preserved_mandatory_items,
        })
    }

    pub(crate) fn apply_source_purge(
        &self,
        source_domain: EvolutionSourceDomain,
        source_id: &str,
        purged_at_ms: i64,
    ) -> Result<ActivitySourcePurgeReport, ActivityProjectionRepositoryError> {
        validate_input(source_id, "purge.source_id", purged_at_ms)?;
        let transaction = self.connection.unchecked_transaction()?;
        let events = affected_source_events(&transaction, source_domain, source_id)?;
        let mut removed_detail_items = 0_u64;
        let mut preserved_tombstones = 0_u64;
        for (event_id, event_code) in events {
            transaction.execute(
                "INSERT OR IGNORE INTO evolution_activity_purge_tombstones
                 (event_id,purged_source_domain,purged_source_id,detail_unavailable_reason,purged_at_ms)
                 VALUES (?1,?2,?3,'source_purged',?4)",
                params![event_id, source_domain.as_str(), source_id, purged_at_ms],
            )?;
            if preserves_committed_outcome(&event_code) {
                preserved_tombstones += 1;
            } else {
                removed_detail_items += transaction.execute(
                    "DELETE FROM evolution_activity_items WHERE event_id=?1",
                    [&event_id],
                )? as u64;
            }
            transaction.execute(
                "UPDATE evolution_activity_envelopes SET payload_json=NULL
                 WHERE event_id=?1",
                [&event_id],
            )?;
            transaction.execute(
                "DELETE FROM evolution_activity_safe_identities
                 WHERE event_id=?1 AND identity_kind NOT IN ('skill','application')",
                [&event_id],
            )?;
        }
        refresh_all_session_summaries(&transaction)?;
        transaction.commit()?;
        Ok(ActivitySourcePurgeReport {
            removed_detail_items,
            preserved_tombstones,
        })
    }
}

fn removable_event_ids(
    transaction: &Transaction<'_>,
    session_id: &str,
    cutoff: i64,
) -> Result<Vec<String>, ActivityProjectionRepositoryError> {
    let mut statement = transaction.prepare(
        "SELECT i.event_id FROM evolution_activity_items i
         JOIN evolution_activity_envelopes e ON e.event_id=i.event_id
         WHERE i.session_id=?1 AND e.committed_at_ms<?2
           AND NOT (e.severity IN ('warning','error','critical') AND e.attention_kind IN
             ('security','integrity','regression','application_failure','breaker'))",
    )?;
    let rows = statement.query_map(params![session_id, cutoff], |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn mandatory_item_count(
    transaction: &Transaction<'_>,
    session_id: &str,
    cutoff: i64,
) -> Result<u64, ActivityProjectionRepositoryError> {
    let count = transaction.query_row(
        "SELECT COUNT(*) FROM evolution_activity_items i
         JOIN evolution_activity_envelopes e ON e.event_id=i.event_id
         WHERE i.session_id=?1 AND e.committed_at_ms<?2
           AND e.severity IN ('warning','error','critical') AND e.attention_kind IN
             ('security','integrity','regression','application_failure','breaker')",
        params![session_id, cutoff],
        |row| row.get::<_, i64>(0),
    )?;
    u64::try_from(count).map_err(|_| ActivityProjectionRepositoryError::Storage)
}

fn affected_source_events(
    transaction: &Transaction<'_>,
    source_domain: EvolutionSourceDomain,
    source_id: &str,
) -> Result<Vec<(String, String)>, ActivityProjectionRepositoryError> {
    let identity_kind = source_identity_kind(source_domain);
    let normalized = normalize_safe_identity_token(source_id)
        .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
    let mut statement = transaction.prepare(
        "SELECT DISTINCT e.event_id,e.event_code FROM evolution_activity_envelopes e
         LEFT JOIN evolution_activity_safe_identities si ON si.event_id=e.event_id
         WHERE (e.source_domain=?1 AND e.source_id=?2)
            OR (?3 IS NOT NULL AND si.identity_kind=?3 AND si.normalized_value=?4)",
    )?;
    let rows = statement.query_map(
        params![source_domain.as_str(), source_id, identity_kind, normalized],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn source_identity_kind(domain: EvolutionSourceDomain) -> Option<&'static str> {
    match domain {
        EvolutionSourceDomain::Orchestration => Some("run"),
        EvolutionSourceDomain::Evidence => Some("evidence"),
        EvolutionSourceDomain::Assessment => Some("assessment"),
        EvolutionSourceDomain::Generation => Some("generation_job"),
        EvolutionSourceDomain::Curator => Some("curator_candidate"),
        EvolutionSourceDomain::Probation => Some("probation"),
        EvolutionSourceDomain::Breaker => Some("breaker"),
        EvolutionSourceDomain::SkillCreation => Some("skill"),
        EvolutionSourceDomain::Overlay
        | EvolutionSourceDomain::AutomaticApplication
        | EvolutionSourceDomain::Recovery
        | EvolutionSourceDomain::Retention => None,
    }
}

fn preserves_committed_outcome(event_code: &str) -> bool {
    matches!(
        event_code,
        "overlay_applied" | "automatic_applied" | "skill_created" | "source_purged"
    )
}

fn refresh_all_session_summaries(
    transaction: &Transaction<'_>,
) -> Result<(), ActivityProjectionRepositoryError> {
    let mut statement =
        transaction.prepare("SELECT session_id FROM evolution_system_activity_sessions")?;
    let sessions = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    for session_id in sessions {
        refresh_session_summary(transaction, &session_id)?;
    }
    Ok(())
}

pub(super) fn refresh_session_summary(
    transaction: &Transaction<'_>,
    session_id: &str,
) -> Result<(), ActivityProjectionRepositoryError> {
    let effective_read = transaction
        .query_row(
            "SELECT MIN(highest_read_sequence,
                    COALESCE(mark_unread_sequence-1,highest_read_sequence))
             FROM evolution_activity_read_state WHERE session_id=?1 AND user_id=?2",
            params![session_id, LOCAL_ACTIVITY_USER_ID],
            |row| row.get::<_, Option<i64>>(0),
        )?
        .unwrap_or(0);
    transaction.execute(
        "UPDATE evolution_system_activity_sessions SET
         unread_count=(SELECT COUNT(*) FROM evolution_activity_items i
           WHERE i.session_id=?1 AND i.generation_id=active_generation_id AND i.sequence>?2),
         attention_kind=COALESCE((SELECT e.attention_kind FROM evolution_activity_items i
           JOIN evolution_activity_envelopes e ON e.event_id=i.event_id
           WHERE i.session_id=?1 AND i.generation_id=active_generation_id AND i.sequence>?2
           ORDER BY CASE e.attention_kind WHEN 'security' THEN 7 WHEN 'integrity' THEN 6
             WHEN 'breaker' THEN 5 WHEN 'application_failure' THEN 4 WHEN 'regression' THEN 3
             WHEN 'review' THEN 2 ELSE 0 END DESC LIMIT 1),'none') WHERE session_id=?1",
        params![session_id, effective_read],
    )?;
    Ok(())
}

fn validate_input(
    value: &str,
    field: &'static str,
    timestamp_ms: i64,
) -> Result<(), ActivityProjectionRepositoryError> {
    if timestamp_ms < 0 || sanitize_text(value, field, 160).is_err() {
        Err(ActivityProjectionRepositoryError::InvalidInput)
    } else {
        Ok(())
    }
}
