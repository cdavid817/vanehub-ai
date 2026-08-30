use super::repository_support::{actor_name, event_name, from_sql_u64, sql_u64, state_name};
use super::{CandidateTransitionRequest, CuratorRepositoryError};
use crate::contexts::skill_evolution_curation::domain::{
    CuratorActorClass, CuratorCandidateState, CuratorEventKind,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrustedAuditContext {
    actor_class: CuratorActorClass,
    occurred_at_ms: i64,
}

impl TrustedAuditContext {
    pub(crate) fn system(occurred_at_ms: i64) -> Self {
        Self {
            actor_class: CuratorActorClass::System,
            occurred_at_ms,
        }
    }

    pub(crate) fn local_interactive_user(occurred_at_ms: i64) -> Self {
        Self {
            actor_class: CuratorActorClass::LocalInteractiveUser,
            occurred_at_ms,
        }
    }

    pub(crate) fn web_mock_interactive_user(occurred_at_ms: i64) -> Self {
        Self {
            actor_class: CuratorActorClass::WebMockInteractiveUser,
            occurred_at_ms,
        }
    }

    pub(super) fn actor_class(self) -> CuratorActorClass {
        self.actor_class
    }

    pub(super) fn occurred_at_ms(self) -> i64 {
        self.occurred_at_ms
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditHashMaterial<'a> {
    candidate_id: &'a str,
    sequence: u64,
    event_kind: CuratorEventKind,
    actor_class: CuratorActorClass,
    occurred_at_ms: i64,
    prior_state: Option<CuratorCandidateState>,
    next_state: CuratorCandidateState,
    object_revision: u64,
    reason_code: Option<&'a str>,
    prior_hash: Option<&'a str>,
}

pub(super) struct AuditHashInput<'a> {
    pub(super) candidate_id: &'a str,
    pub(super) sequence: u64,
    pub(super) event_kind: CuratorEventKind,
    pub(super) actor_class: CuratorActorClass,
    pub(super) occurred_at_ms: i64,
    pub(super) prior_state: Option<CuratorCandidateState>,
    pub(super) next_state: CuratorCandidateState,
    pub(super) object_revision: u64,
    pub(super) reason_code: Option<&'a str>,
    pub(super) prior_hash: Option<&'a str>,
}

pub(super) fn audit_hash(input: &AuditHashInput<'_>) -> Result<String, ()> {
    let material = AuditHashMaterial {
        candidate_id: input.candidate_id,
        sequence: input.sequence,
        event_kind: input.event_kind,
        actor_class: input.actor_class,
        occurred_at_ms: input.occurred_at_ms,
        prior_state: input.prior_state,
        next_state: input.next_state,
        object_revision: input.object_revision,
        reason_code: input.reason_code,
        prior_hash: input.prior_hash,
    };
    let bytes = serde_json::to_vec(&material).map_err(|_| ())?;
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!("sha256:{hex}"))
}

pub(super) fn append_audit_event(
    transaction: &Transaction<'_>,
    request: &CandidateTransitionRequest<'_>,
    prior_state: CuratorCandidateState,
    next_state: CuratorCandidateState,
    next_revision: u64,
) -> Result<(), CuratorRepositoryError> {
    let prior = transaction
        .query_row(
            "SELECT sequence,event_hash FROM evolution_curator_events
             WHERE candidate_id=?1 ORDER BY sequence DESC LIMIT 1",
            [request.candidate_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|_| CuratorRepositoryError::Storage)?;
    let sequence = prior
        .as_ref()
        .map(|(sequence, _)| from_sql_u64(*sequence))
        .transpose()?
        .map_or(1, |sequence| sequence + 1);
    let prior_hash = prior.as_ref().map(|(_, hash)| hash.as_str());
    let input = AuditHashInput {
        candidate_id: request.candidate_id,
        sequence,
        event_kind: request.event_kind,
        actor_class: request.audit.actor_class(),
        occurred_at_ms: request.audit.occurred_at_ms(),
        prior_state: Some(prior_state),
        next_state,
        object_revision: next_revision,
        reason_code: request.reason_code,
        prior_hash,
    };
    let event_hash = audit_hash(&input).map_err(|_| CuratorRepositoryError::InvalidInput)?;
    transaction
        .execute(
            "INSERT INTO evolution_curator_events
             (candidate_id,sequence,event_kind,actor_class,occurred_at_ms,prior_state,next_state,
              object_revision,reason_code,prior_hash,event_hash)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                request.candidate_id,
                sql_u64(sequence)?,
                event_name(request.event_kind),
                actor_name(request.audit.actor_class()),
                request.audit.occurred_at_ms(),
                state_name(prior_state),
                state_name(next_state),
                sql_u64(next_revision)?,
                request.reason_code,
                prior_hash,
                event_hash
            ],
        )
        .map_err(|_| CuratorRepositoryError::Storage)?;
    super::queue_notification_receipt(
        transaction,
        request.candidate_id,
        next_revision,
        request.event_kind,
    )?;
    Ok(())
}

pub(super) struct SystemAuditEvent<'a> {
    pub(super) candidate_id: &'a str,
    pub(super) event_kind: CuratorEventKind,
    pub(super) occurred_at_ms: i64,
    pub(super) prior_state: Option<CuratorCandidateState>,
    pub(super) next_state: CuratorCandidateState,
    pub(super) object_revision: u64,
    pub(super) reason_code: &'a str,
}

pub(super) fn append_system_event(
    transaction: &Transaction<'_>,
    event: &SystemAuditEvent<'_>,
) -> Result<(), CuratorRepositoryError> {
    let prior = transaction
        .query_row(
            "SELECT sequence,event_hash FROM evolution_curator_events
             WHERE candidate_id=?1 ORDER BY sequence DESC LIMIT 1",
            [event.candidate_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|_| CuratorRepositoryError::Storage)?;
    let sequence = prior
        .as_ref()
        .map(|(value, _)| from_sql_u64(*value))
        .transpose()?
        .map_or(1, |value| value + 1);
    let prior_hash = prior.as_ref().map(|(_, hash)| hash.as_str());
    let input = AuditHashInput {
        candidate_id: event.candidate_id,
        sequence,
        event_kind: event.event_kind,
        actor_class: CuratorActorClass::System,
        occurred_at_ms: event.occurred_at_ms,
        prior_state: event.prior_state,
        next_state: event.next_state,
        object_revision: event.object_revision,
        reason_code: Some(event.reason_code),
        prior_hash,
    };
    let event_hash = audit_hash(&input).map_err(|_| CuratorRepositoryError::InvalidInput)?;
    transaction
        .execute(
            "INSERT INTO evolution_curator_events
             (candidate_id,sequence,event_kind,actor_class,occurred_at_ms,prior_state,next_state,
              object_revision,reason_code,prior_hash,event_hash)
             VALUES (?1,?2,?3,'system',?4,?5,?6,?7,?8,?9,?10)",
            params![
                event.candidate_id,
                sql_u64(sequence)?,
                event_name(event.event_kind),
                event.occurred_at_ms,
                event.prior_state.map(state_name),
                state_name(event.next_state),
                sql_u64(event.object_revision)?,
                event.reason_code,
                prior_hash,
                event_hash
            ],
        )
        .map_err(|_| CuratorRepositoryError::Storage)?;
    super::queue_notification_receipt(
        transaction,
        event.candidate_id,
        event.object_revision,
        event.event_kind,
    )?;
    Ok(())
}

pub(crate) fn verify_audit_chain(
    connection: &Connection,
    candidate_id: &str,
) -> Result<(), AuditChainError> {
    let mut statement = connection
        .prepare(
            "SELECT sequence,event_kind,actor_class,occurred_at_ms,prior_state,next_state,
                    object_revision,reason_code,prior_hash,event_hash
             FROM evolution_curator_events WHERE candidate_id=?1 ORDER BY sequence",
        )
        .map_err(|_| AuditChainError::Storage)?;
    let rows = statement
        .query_map([candidate_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, String>(9)?,
            ))
        })
        .map_err(|_| AuditChainError::Storage)?;
    let mut expected_prior: Option<String> = None;
    for (expected_sequence, row) in (1_u64..).zip(rows) {
        let (
            sequence_sql,
            event,
            actor,
            occurred_at_ms,
            prior_state,
            next_state,
            revision_sql,
            reason,
            prior_hash,
            stored_hash,
        ) = row.map_err(|_| AuditChainError::Storage)?;
        let sequence = u64::try_from(sequence_sql).map_err(|_| AuditChainError::Corrupt)?;
        let revision = u64::try_from(revision_sql).map_err(|_| AuditChainError::Corrupt)?;
        if sequence != expected_sequence || prior_hash != expected_prior {
            return Err(AuditChainError::Corrupt);
        }
        let event_kind = parse_wire::<CuratorEventKind>(&event)?;
        let actor_class = parse_wire::<CuratorActorClass>(&actor)?;
        let prior = prior_state.as_deref().map(parse_wire).transpose()?;
        let next = parse_wire::<CuratorCandidateState>(&next_state)?;
        let input = AuditHashInput {
            candidate_id,
            sequence,
            event_kind,
            actor_class,
            occurred_at_ms,
            prior_state: prior,
            next_state: next,
            object_revision: revision,
            reason_code: reason.as_deref(),
            prior_hash: prior_hash.as_deref(),
        };
        let calculated = audit_hash(&input).map_err(|_| AuditChainError::Storage)?;
        if calculated != stored_hash {
            return Err(AuditChainError::Corrupt);
        }
        expected_prior = Some(stored_hash);
    }
    Ok(())
}

fn parse_wire<T>(value: &str) -> Result<T, AuditChainError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| AuditChainError::Corrupt)
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum AuditChainError {
    #[error("curator audit chain is corrupt")]
    Corrupt,
    #[error("curator audit chain could not be read")]
    Storage,
}
