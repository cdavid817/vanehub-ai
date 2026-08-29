use crate::contexts::skill_evolution_assessment::domain::{
    ModelEvaluationConsent, DISCLOSURE_VERSION_V1, EVALUATOR_POLICY_V1,
};
use crate::platform::database::NativeDatabase;
use rusqlite::params;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssessmentPolicyError {
    InvalidInput,
    Storage,
}

#[derive(Clone)]
pub(crate) struct SqliteAssessmentPolicyRepository {
    database: NativeDatabase,
}

impl SqliteAssessmentPolicyRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn load(&self) -> Result<ModelEvaluationConsent, AssessmentPolicyError> {
        let connection = self
            .database
            .connection()
            .map_err(|_| AssessmentPolicyError::Storage)?;
        connection
            .query_row(
                "SELECT evaluator_policy_version, disclosure_version, \
                        model_evaluation_enabled, changed_at_ms, local_actor \
                 FROM evolution_assessment_policy WHERE policy_id = 1",
                [],
                |row| {
                    Ok(ModelEvaluationConsent {
                        policy_version: row.get(0)?,
                        disclosure_version: row.get(1)?,
                        enabled: row.get::<_, i64>(2)? != 0,
                        changed_at_ms: row.get(3)?,
                        local_actor: row.get(4)?,
                    })
                },
            )
            .map_err(|_| AssessmentPolicyError::Storage)
    }

    pub(crate) fn update(
        &self,
        consent: &ModelEvaluationConsent,
    ) -> Result<ModelEvaluationConsent, AssessmentPolicyError> {
        let actor = consent.local_actor.trim();
        if consent.policy_version != EVALUATOR_POLICY_V1
            || consent.disclosure_version != DISCLOSURE_VERSION_V1
            || consent.changed_at_ms < 0
            || actor.is_empty()
            || actor.len() > 128
        {
            return Err(AssessmentPolicyError::InvalidInput);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| AssessmentPolicyError::Storage)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| AssessmentPolicyError::Storage)?;
        transaction
            .execute(
                "UPDATE evolution_assessment_policy SET \
                    evaluator_policy_version = ?1, disclosure_version = ?2, \
                    model_evaluation_enabled = ?3, changed_at_ms = ?4, local_actor = ?5 \
                 WHERE policy_id = 1",
                params![
                    consent.policy_version,
                    consent.disclosure_version,
                    i64::from(consent.enabled),
                    consent.changed_at_ms,
                    actor,
                ],
            )
            .map_err(|_| AssessmentPolicyError::Storage)?;
        transaction
            .commit()
            .map_err(|_| AssessmentPolicyError::Storage)?;
        self.load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    #[test]
    fn consent_is_default_disabled_versioned_and_persists_revocation() {
        let directory = TempDirectory::new("assessment-policy");
        let database = NativeDatabase::new(directory.path().to_path_buf())
            .unwrap_or_else(|error| panic!("database: {error}"));
        let repository = SqliteAssessmentPolicyRepository::new(database.clone());

        assert_eq!(repository.load(), Ok(ModelEvaluationConsent::default()));
        let enabled = ModelEvaluationConsent {
            enabled: true,
            changed_at_ms: 10,
            local_actor: "local-user".to_string(),
            ..ModelEvaluationConsent::default()
        };
        assert_eq!(repository.update(&enabled), Ok(enabled));

        let reopened = SqliteAssessmentPolicyRepository::new(database);
        let revoked = ModelEvaluationConsent {
            enabled: false,
            changed_at_ms: 20,
            local_actor: "local-user".to_string(),
            ..ModelEvaluationConsent::default()
        };
        assert_eq!(reopened.update(&revoked), Ok(revoked.clone()));
        assert_eq!(reopened.load(), Ok(revoked));
    }

    #[test]
    fn stale_disclosure_cannot_enable_external_evaluation() {
        let directory = TempDirectory::new("assessment-policy-version");
        let database = NativeDatabase::new(directory.path().to_path_buf())
            .unwrap_or_else(|error| panic!("database: {error}"));
        let repository = SqliteAssessmentPolicyRepository::new(database);
        let stale = ModelEvaluationConsent {
            disclosure_version: "stale".to_string(),
            enabled: true,
            changed_at_ms: 10,
            local_actor: "local-user".to_string(),
            ..ModelEvaluationConsent::default()
        };

        assert_eq!(
            repository.update(&stale),
            Err(AssessmentPolicyError::InvalidInput)
        );
        assert_eq!(repository.load(), Ok(ModelEvaluationConsent::default()));
    }
}
