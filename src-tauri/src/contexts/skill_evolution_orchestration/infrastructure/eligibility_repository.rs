use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::{
    contexts::skill_evolution_orchestration::domain::{
        is_safe_identifier, AutoApplyEligibilityV1, AutoEligibilityResult,
    },
    platform::database::NativeDatabase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PersistEligibilityOutcome {
    Inserted,
    Updated,
    Duplicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EligibilityRepositoryError {
    InvalidInput,
    Conflict,
    Storage,
}

#[derive(Clone)]
pub(crate) struct SqliteEligibilityRepository {
    database: NativeDatabase,
}

impl SqliteEligibilityRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn persist(
        &self,
        proof: &AutoApplyEligibilityV1,
        expected_revision: Option<u64>,
    ) -> Result<PersistEligibilityOutcome, EligibilityRepositoryError> {
        validate(proof)?;
        let predicates = serde_json::to_string(&proof.predicates)
            .map_err(|_| EligibilityRepositoryError::InvalidInput)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| EligibilityRepositoryError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| EligibilityRepositoryError::Storage)?;
        let current = transaction
            .query_row(
                "SELECT revision,proof_hash FROM evolution_auto_eligibility
                 WHERE eligibility_id=?1",
                [&proof.eligibility_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| EligibilityRepositoryError::Storage)?;
        let outcome = match current {
            None => {
                if expected_revision.is_some() || proof.revision != 0 {
                    return Err(EligibilityRepositoryError::Conflict);
                }
                insert(&transaction, proof, &predicates)?;
                PersistEligibilityOutcome::Inserted
            }
            Some((revision, proof_hash)) => {
                let revision =
                    u64::try_from(revision).map_err(|_| EligibilityRepositoryError::Storage)?;
                if proof_hash == proof.proof_hash {
                    PersistEligibilityOutcome::Duplicate
                } else {
                    if expected_revision != Some(revision)
                        || proof.revision != revision.saturating_add(1)
                    {
                        return Err(EligibilityRepositoryError::Conflict);
                    }
                    update(&transaction, proof, &predicates, revision)?;
                    PersistEligibilityOutcome::Updated
                }
            }
        };
        transaction
            .commit()
            .map_err(|_| EligibilityRepositoryError::Storage)?;
        Ok(outcome)
    }
}

fn validate(proof: &AutoApplyEligibilityV1) -> Result<(), EligibilityRepositoryError> {
    if !is_safe_identifier(&proof.eligibility_id, 256)
        || !is_safe_identifier(&proof.run_id, 256)
        || !is_safe_identifier(&proof.draft_id, 256)
        || !is_safe_identifier(&proof.target_skill_id, 256)
        || proof.predicates.len() != 26
        || proof.predicates.iter().any(|predicate| {
            predicate.condition.is_empty()
                || predicate.witness_hash.as_deref().is_none_or(str::is_empty)
        })
    {
        return Err(EligibilityRepositoryError::InvalidInput);
    }
    Ok(())
}

fn insert(
    transaction: &rusqlite::Transaction<'_>,
    proof: &AutoApplyEligibilityV1,
    predicates: &str,
) -> Result<(), EligibilityRepositoryError> {
    let revision = sql_revision(proof.revision)?;
    transaction
        .execute(
            "INSERT INTO evolution_auto_eligibility VALUES
             (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                proof.eligibility_id,
                proof.run_id,
                proof.draft_id,
                proof.target_skill_id,
                result_name(proof.result),
                predicates,
                proof.proof_hash,
                proof.overlay_preview_hash,
                proof.evaluated_at_ms,
                revision,
            ],
        )
        .map_err(|_| EligibilityRepositoryError::Storage)?;
    Ok(())
}

fn update(
    transaction: &rusqlite::Transaction<'_>,
    proof: &AutoApplyEligibilityV1,
    predicates: &str,
    expected_revision: u64,
) -> Result<(), EligibilityRepositoryError> {
    let changed = transaction
        .execute(
            "UPDATE evolution_auto_eligibility SET run_id=?2,draft_id=?3,target_skill_id=?4,
             result=?5,predicates_json=?6,proof_hash=?7,overlay_preview_hash=?8,
             evaluated_at_ms=?9,revision=?10 WHERE eligibility_id=?1 AND revision=?11",
            params![
                proof.eligibility_id,
                proof.run_id,
                proof.draft_id,
                proof.target_skill_id,
                result_name(proof.result),
                predicates,
                proof.proof_hash,
                proof.overlay_preview_hash,
                proof.evaluated_at_ms,
                sql_revision(proof.revision)?,
                sql_revision(expected_revision)?,
            ],
        )
        .map_err(|_| EligibilityRepositoryError::Storage)?;
    if changed != 1 {
        return Err(EligibilityRepositoryError::Conflict);
    }
    Ok(())
}

fn sql_revision(value: u64) -> Result<i64, EligibilityRepositoryError> {
    i64::try_from(value).map_err(|_| EligibilityRepositoryError::InvalidInput)
}

fn result_name(result: AutoEligibilityResult) -> &'static str {
    match result {
        AutoEligibilityResult::Ineligible => "ineligible",
        AutoEligibilityResult::Waiting => "waiting",
        AutoEligibilityResult::RoutedToCurator => "routed_to_curator",
        AutoEligibilityResult::WouldApply => "would_apply",
        AutoEligibilityResult::Eligible => "eligible",
    }
}
