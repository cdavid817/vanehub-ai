use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use thiserror::Error;
use uuid::Uuid;

use super::storage_values;
use crate::contexts::skill_evolution_evidence::domain::{SignalDraft, TaskFingerprintBuilder};
use crate::platform::database::NativeDatabase;

pub(crate) const MAX_SKILL_ASSOCIATIONS_PER_SIGNAL: usize = 32;
pub(crate) const MAX_SOURCE_LINKS_PER_SIGNAL: usize = 16;

#[derive(Debug, Error)]
pub(crate) enum EvidenceRepositoryError {
    #[error("evidence storage operation failed")]
    Storage,
    #[error("evidence receipt references a missing signal")]
    CorruptReceipt,
}

impl From<rusqlite::Error> for EvidenceRepositoryError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Storage
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersistSignalOutcome {
    Inserted { signal_id: String },
    Replayed { signal_id: String },
}

impl PersistSignalOutcome {
    pub(crate) fn signal_id(&self) -> &str {
        match self {
            Self::Inserted { signal_id } | Self::Replayed { signal_id } => signal_id,
        }
    }
}

#[derive(Clone)]
pub(crate) struct SqliteEvolutionEvidenceRepository {
    pub(crate) database: NativeDatabase,
}

impl SqliteEvolutionEvidenceRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn persist_signal(
        &self,
        draft: &SignalDraft,
        fingerprints: &TaskFingerprintBuilder,
    ) -> Result<PersistSignalOutcome, EvidenceRepositoryError> {
        let mut connection = self
            .database
            .connection()
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let outcome = persist_transaction(&transaction, draft, fingerprints)?;
        transaction
            .commit()
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        Ok(outcome)
    }
}

pub(super) fn persist_transaction(
    transaction: &Transaction<'_>,
    draft: &SignalDraft,
    fingerprints: &TaskFingerprintBuilder,
) -> Result<PersistSignalOutcome, EvidenceRepositoryError> {
    let extractor = storage_values::extractor(draft.extractor());
    if let Some((signal_id, signal_exists)) = existing_receipt(transaction, draft, extractor)? {
        if !signal_exists {
            return Err(EvidenceRepositoryError::CorruptReceipt);
        }
        return Ok(PersistSignalOutcome::Replayed { signal_id });
    }

    let signal_id = Uuid::new_v4().to_string();
    insert_signal(transaction, &signal_id, draft, extractor, fingerprints)?;
    insert_associations(transaction, &signal_id, draft)?;
    insert_source_links(transaction, &signal_id, draft)?;
    insert_receipt(transaction, &signal_id, draft, extractor)?;
    transaction.execute(
        "UPDATE evolution_pipeline_state SET state_value = 'dirty', revision = revision + 1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') WHERE state_key = 'signal_set'",
        [],
    )?;
    Ok(PersistSignalOutcome::Inserted { signal_id })
}

fn existing_receipt(
    transaction: &Transaction<'_>,
    draft: &SignalDraft,
    extractor: &str,
) -> Result<Option<(String, bool)>, EvidenceRepositoryError> {
    transaction
        .query_row(
            r#"SELECT receipt.signal_id,
                      EXISTS(SELECT 1 FROM evolution_signals signal WHERE signal.signal_id = receipt.signal_id)
               FROM evolution_signal_receipts receipt
               WHERE source_event_id = ?1 AND extractor_id = ?2
                 AND extractor_version = ?3 AND discriminator = ?4"#,
            params![
                draft.source_event_id(),
                extractor,
                i64::from(draft.extractor_version()),
                draft.discriminator(),
            ],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? != 0)),
        )
        .optional()
        .map_err(Into::into)
}

fn insert_signal(
    transaction: &Transaction<'_>,
    signal_id: &str,
    draft: &SignalDraft,
    extractor: &str,
    fingerprints: &TaskFingerprintBuilder,
) -> Result<(), EvidenceRepositoryError> {
    let attribution = draft.attribution();
    let fingerprint = fingerprints
        .build(draft)
        .map_err(|_| EvidenceRepositoryError::Storage)?;
    transaction.execute(
        r#"INSERT INTO evolution_signals (
            signal_id, source_event_id, source_kind, source_agent_id, extractor_id,
            extractor_version, discriminator, deduplication_key, category, polarity,
            severity, attribution_strength, attribution_rationale, targeting_eligibility,
            source_fidelity, occurred_at, ingested_at, workspace, safe_summary, sanitizer_version,
            task_fingerprint_version, task_fingerprint, association_truncated_count,
            source_link_truncated_count
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
            ?16, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), ?17, ?18, ?19, ?20, ?21, ?22, ?23
        )"#,
        params![
            signal_id,
            draft.source_event_id(),
            storage_values::source_family(draft.source_family()),
            draft.source_agent_id(),
            extractor,
            i64::from(draft.extractor_version()),
            draft.discriminator(),
            deduplication_key(draft, extractor),
            storage_values::category(draft.category()),
            storage_values::polarity(draft.polarity()),
            storage_values::severity(draft.severity()),
            storage_values::attribution(attribution.strength()),
            storage_values::rationale(attribution.rationale()),
            storage_values::targeting(attribution.targeting_eligibility()),
            storage_values::fidelity(draft.source_fidelity()),
            draft.occurred_at(),
            draft.workspace(),
            draft.safe_summary(),
            i64::from(draft.sanitizer_version()),
            i64::from(fingerprint.version()),
            fingerprint.value(),
            attribution
                .associations()
                .len()
                .saturating_sub(MAX_SKILL_ASSOCIATIONS_PER_SIGNAL) as i64,
            draft
                .source_links()
                .len()
                .saturating_sub(MAX_SOURCE_LINKS_PER_SIGNAL) as i64,
        ],
    )?;
    Ok(())
}

fn insert_associations(
    transaction: &Transaction<'_>,
    signal_id: &str,
    draft: &SignalDraft,
) -> Result<(), EvidenceRepositoryError> {
    let eligibility = storage_values::targeting(draft.attribution().targeting_eligibility());
    for association in draft
        .attribution()
        .associations()
        .iter()
        .take(MAX_SKILL_ASSOCIATIONS_PER_SIGNAL)
    {
        transaction.execute(
            r#"INSERT INTO evolution_signal_skill_associations (
                signal_id, skill_id, skill_revision, association_kind, observed_at,
                attribution_strength, attribution_rationale, targeting_eligibility
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
            params![
                signal_id,
                association.skill_id(),
                association.revision(),
                storage_values::association_kind(association.association_kind()),
                association.observed_at(),
                storage_values::attribution(association.strength()),
                storage_values::rationale(association.rationale()),
                eligibility,
            ],
        )?;
    }
    Ok(())
}

fn insert_source_links(
    transaction: &Transaction<'_>,
    signal_id: &str,
    draft: &SignalDraft,
) -> Result<(), EvidenceRepositoryError> {
    for link in draft
        .source_links()
        .iter()
        .take(MAX_SOURCE_LINKS_PER_SIGNAL)
    {
        transaction.execute(
            "INSERT INTO evolution_signal_source_links (signal_id, source_kind, source_id) VALUES (?1, ?2, ?3)",
            params![signal_id, storage_values::source_link(link.kind()), link.source_id()],
        )?;
    }
    Ok(())
}

fn insert_receipt(
    transaction: &Transaction<'_>,
    signal_id: &str,
    draft: &SignalDraft,
    extractor: &str,
) -> Result<(), EvidenceRepositoryError> {
    transaction.execute(
        r#"INSERT INTO evolution_signal_receipts (
            source_event_id, extractor_id, extractor_version, discriminator, signal_id, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))"#,
        params![
            draft.source_event_id(),
            extractor,
            i64::from(draft.extractor_version()),
            draft.discriminator(),
            signal_id,
        ],
    )?;
    Ok(())
}

fn deduplication_key(draft: &SignalDraft, extractor: &str) -> String {
    format!(
        "{}:{}|{}:{}|{}|{}:{}",
        draft.source_event_id().len(),
        draft.source_event_id(),
        extractor.len(),
        extractor,
        draft.extractor_version(),
        draft.discriminator().len(),
        draft.discriminator(),
    )
}
