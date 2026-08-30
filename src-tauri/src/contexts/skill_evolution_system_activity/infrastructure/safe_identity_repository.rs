use rusqlite::{params, Transaction};

use super::ActivityProjectionRepositoryError;
use crate::contexts::skill_evolution_system_activity::domain::*;

pub(super) fn persist_safe_identities(
    transaction: &Transaction<'_>,
    envelope: &EvolutionActivityEnvelopeV1,
) -> Result<(), ActivityProjectionRepositoryError> {
    for identity in &envelope.safe_identities {
        transaction.execute(
            "INSERT OR IGNORE INTO evolution_activity_safe_identities
             (event_id,identity_kind,identity_value,normalized_value) VALUES (?1,?2,?3,?4)",
            params![
                envelope.event_id,
                enum_text(identity.kind)?,
                identity.value,
                normalize_safe_identity_token(&identity.value)
                    .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?,
            ],
        )?;
    }
    Ok(())
}

fn enum_text<T: serde::Serialize>(value: T) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}
