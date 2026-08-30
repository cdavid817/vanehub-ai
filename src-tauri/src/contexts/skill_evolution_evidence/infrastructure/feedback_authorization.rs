use chrono::Utc;
use rusqlite::{params, OptionalExtension, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::FeedbackTransitionError;

pub(crate) const REUSABLE_GUIDANCE_DISCLOSURE_VERSION_V1: &str =
    "reusable-correction-guidance-disclosure-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReusableGuidanceAuthorizationSummary {
    pub(crate) authorization_id: String,
    pub(crate) feedback_revision: u64,
    pub(crate) disclosure_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthorizedCorrectionGuidance {
    pub(crate) authorization_id: String,
    pub(crate) feedback_revision: u64,
    pub(crate) disclosure_version: String,
    pub(crate) sanitized_guidance: String,
    pub(crate) sanitizer_version: u16,
    pub(crate) authorization_witness_hash: String,
}

pub(super) fn authorized_correction_guidance(
    connection: &rusqlite::Connection,
    authorization_id: &str,
) -> Result<Option<AuthorizedCorrectionGuidance>, FeedbackTransitionError> {
    if authorization_id.trim().is_empty() {
        return Err(FeedbackTransitionError::InvalidInput);
    }
    connection
        .query_row(
            "SELECT a.authorization_id,a.feedback_revision,a.disclosure_version,
                    f.sanitized_note,f.sanitizer_version,a.witness_hash
             FROM evolution_correction_authorizations a
             JOIN evolution_feedback_current f ON f.message_id=a.feedback_id
               AND f.feedback_revision=a.feedback_revision
             WHERE a.authorization_id=?1 AND a.authorized=1 AND a.revoked_at_ms IS NULL
               AND f.feedback_state='corrected' AND f.sanitized_note IS NOT NULL",
            [authorization_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|_| FeedbackTransitionError::Storage)?
        .map(
            |(
                authorization_id,
                feedback_revision,
                disclosure_version,
                guidance,
                sanitizer,
                witness,
            )| {
                Ok(AuthorizedCorrectionGuidance {
                    authorization_id,
                    feedback_revision: u64::try_from(feedback_revision)
                        .map_err(|_| FeedbackTransitionError::Storage)?,
                    disclosure_version,
                    sanitized_guidance: guidance,
                    sanitizer_version: u16::try_from(sanitizer)
                        .map_err(|_| FeedbackTransitionError::Storage)?,
                    authorization_witness_hash: witness,
                })
            },
        )
        .transpose()
}

pub(super) fn create_authorization(
    transaction: &Transaction<'_>,
    message_id: &str,
    feedback_revision: u64,
    sanitized_note: &str,
) -> Result<ReusableGuidanceAuthorizationSummary, FeedbackTransitionError> {
    let authorization_id = Uuid::new_v4().to_string();
    let created_at_ms = Utc::now().timestamp_millis();
    let witness_hash = witness_hash(&(
        &authorization_id,
        message_id,
        feedback_revision,
        REUSABLE_GUIDANCE_DISCLOSURE_VERSION_V1,
        sanitized_note,
        created_at_ms,
        "interactive_user",
    ))?;
    transaction
        .execute(
            "INSERT INTO evolution_correction_authorizations
             (authorization_id,feedback_id,feedback_revision,disclosure_version,authorized,
              actor,witness_hash,created_at_ms,revoked_at_ms)
             VALUES (?1,?2,?3,?4,1,'interactive_user',?5,?6,NULL)",
            params![
                authorization_id,
                message_id,
                sql_revision(feedback_revision)?,
                REUSABLE_GUIDANCE_DISCLOSURE_VERSION_V1,
                witness_hash,
                created_at_ms,
            ],
        )
        .map_err(|_| FeedbackTransitionError::Storage)?;
    Ok(ReusableGuidanceAuthorizationSummary {
        authorization_id,
        feedback_revision,
        disclosure_version: REUSABLE_GUIDANCE_DISCLOSURE_VERSION_V1.into(),
    })
}

pub(super) fn current_authorization(
    connection: &rusqlite::Connection,
    message_id: &str,
    feedback_revision: u64,
) -> Result<Option<ReusableGuidanceAuthorizationSummary>, FeedbackTransitionError> {
    let revision = sql_revision(feedback_revision)?;
    connection
        .query_row(
            "SELECT authorization_id,feedback_revision,disclosure_version
             FROM evolution_correction_authorizations
             WHERE feedback_id=?1 AND feedback_revision=?2 AND authorized=1
               AND revoked_at_ms IS NULL",
            params![message_id, revision],
            |row| {
                let stored_revision = row.get::<_, i64>(1)?;
                Ok((
                    row.get::<_, String>(0)?,
                    stored_revision,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|_| FeedbackTransitionError::Storage)?
        .map(|(authorization_id, revision, disclosure_version)| {
            Ok(ReusableGuidanceAuthorizationSummary {
                authorization_id,
                feedback_revision: u64::try_from(revision)
                    .map_err(|_| FeedbackTransitionError::Storage)?,
                disclosure_version,
            })
        })
        .transpose()
}

pub(super) fn revoke_authorizations(
    transaction: &Transaction<'_>,
    message_id: &str,
    feedback_revision: Option<u64>,
) -> Result<Vec<String>, FeedbackTransitionError> {
    let revoked_at_ms = Utc::now().timestamp_millis();
    let authorization_ids = active_authorization_ids(transaction, message_id, feedback_revision)?;
    let changed = match feedback_revision {
        Some(revision) => transaction.execute(
            "UPDATE evolution_correction_authorizations SET authorized=0,revoked_at_ms=?1
             WHERE feedback_id=?2 AND feedback_revision=?3 AND authorized=1
               AND revoked_at_ms IS NULL",
            params![revoked_at_ms, message_id, sql_revision(revision)?],
        ),
        None => transaction.execute(
            "UPDATE evolution_correction_authorizations SET authorized=0,revoked_at_ms=?1
             WHERE feedback_id=?2 AND authorized=1 AND revoked_at_ms IS NULL",
            params![revoked_at_ms, message_id],
        ),
    }
    .map_err(|_| FeedbackTransitionError::Storage)?;
    if changed != authorization_ids.len() {
        return Err(FeedbackTransitionError::Storage);
    }
    if changed > 0 {
        invalidate_derived_eligibility(transaction, message_id, revoked_at_ms)?;
    }
    Ok(authorization_ids)
}

fn active_authorization_ids(
    transaction: &Transaction<'_>,
    message_id: &str,
    feedback_revision: Option<u64>,
) -> Result<Vec<String>, FeedbackTransitionError> {
    let (sql, revision) = match feedback_revision {
        Some(revision) => (
            "SELECT authorization_id FROM evolution_correction_authorizations
             WHERE feedback_id=?1 AND feedback_revision=?2 AND authorized=1
               AND revoked_at_ms IS NULL ORDER BY authorization_id",
            Some(sql_revision(revision)?),
        ),
        None => (
            "SELECT authorization_id FROM evolution_correction_authorizations
             WHERE feedback_id=?1 AND authorized=1 AND revoked_at_ms IS NULL
             ORDER BY authorization_id",
            None,
        ),
    };
    let mut statement = transaction
        .prepare(sql)
        .map_err(|_| FeedbackTransitionError::Storage)?;
    let rows = match revision {
        Some(revision) => statement.query_map(params![message_id, revision], read_authorization_id),
        None => statement.query_map(params![message_id], read_authorization_id),
    }
    .map_err(|_| FeedbackTransitionError::Storage)?;
    rows.collect::<Result<Vec<String>, _>>()
        .map_err(|_| FeedbackTransitionError::Storage)
}

fn read_authorization_id(row: &rusqlite::Row<'_>) -> rusqlite::Result<String> {
    row.get(0)
}

fn invalidate_derived_eligibility(
    transaction: &Transaction<'_>,
    message_id: &str,
    now_ms: i64,
) -> Result<(), FeedbackTransitionError> {
    let proof_hash = witness_hash(&(message_id, now_ms, "authorization-revoked"))?;
    transaction
        .execute(
            "UPDATE evolution_auto_eligibility
             SET result='ineligible',overlay_preview_hash=NULL,evaluated_at_ms=?1,
                 predicates_json='[{\"condition\":\"correction_authorization\",\"passed\":false,\"safeReasonCode\":\"correction-authorization-revoked\",\"witnessHash\":null}]',
                 proof_hash=?2,revision=revision+1
             WHERE result!='ineligible' AND draft_id IN (
               SELECT draft_id FROM evolution_deterministic_drafts WHERE authorization_id IN (
                 SELECT authorization_id FROM evolution_correction_authorizations
                 WHERE feedback_id=?3 AND revoked_at_ms IS NOT NULL
               )
             )",
            params![now_ms, proof_hash, message_id],
        )
        .map_err(|_| FeedbackTransitionError::Storage)?;
    Ok(())
}

fn sql_revision(revision: u64) -> Result<i64, FeedbackTransitionError> {
    i64::try_from(revision).map_err(|_| FeedbackTransitionError::InvalidInput)
}

fn witness_hash<T: Serialize>(value: &T) -> Result<String, FeedbackTransitionError> {
    let bytes = serde_json::to_vec(value).map_err(|_| FeedbackTransitionError::Storage)?;
    let digest = Sha256::digest(bytes);
    Ok(format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}
