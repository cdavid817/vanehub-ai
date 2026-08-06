use crate::contexts::permissions::application::{PermissionsApplicationError, PrincipalRepository};
use crate::contexts::permissions::domain::{PolicyTemplateName, Principal};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension, Row};

#[derive(Clone)]
pub(crate) struct SqlitePrincipalRepository {
    database: NativeDatabase,
}

impl SqlitePrincipalRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Principal> {
        let id: String = row.get(0)?;
        let agent_id: String = row.get(1)?;
        let template_name: String = row.get(2)?;
        let parent_principal_id: Option<String> = row.get(3)?;
        let budget_config_raw: Option<String> = row.get(4)?;
        let template =
            PolicyTemplateName::from_str(&template_name).unwrap_or(PolicyTemplateName::Standard);
        let budget_config = budget_config_raw.and_then(|raw| serde_json::from_str(&raw).ok());
        Principal::new(id, agent_id, template, parent_principal_id, budget_config).map_err(
            |error| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    error.to_string(),
                    rusqlite::types::Type::Text,
                )
            },
        )
    }
}

impl PrincipalRepository for SqlitePrincipalRepository {
    fn find_by_agent_id(
        &self,
        agent_id: &str,
    ) -> Result<Option<Principal>, PermissionsApplicationError> {
        self.database
            .connection()
            .map_err(repository_error)?
            .query_row(
                "SELECT id, agent_id, template_name, parent_principal_id, budget_config \
                 FROM agent_principals WHERE agent_id = ?1",
                params![agent_id],
                Self::from_row,
            )
            .optional()
            .map_err(repository_error)
    }

    fn create(&self, principal: &Principal) -> Result<(), PermissionsApplicationError> {
        let now = crate::platform::clock::SystemClock.rfc3339();
        let budget_config = principal
            .budget_config()
            .map(serde_json::to_string)
            .transpose()
            .map_err(repository_error)?;
        self.database
            .connection()
            .map_err(repository_error)?
            .execute(
                "INSERT INTO agent_principals \
                 (id, agent_id, template_name, parent_principal_id, budget_config, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
                params![
                    principal.id(),
                    principal.agent_id(),
                    principal.template().as_str(),
                    principal.parent_principal_id(),
                    budget_config,
                    now,
                ],
            )
            .map(|_| ())
            .map_err(repository_error)
    }

    fn update_template(
        &self,
        principal_id: &str,
        template: PolicyTemplateName,
    ) -> Result<(), PermissionsApplicationError> {
        let now = crate::platform::clock::SystemClock.rfc3339();
        let changed = self
            .database
            .connection()
            .map_err(repository_error)?
            .execute(
                "UPDATE agent_principals SET template_name = ?1, updated_at = ?2 WHERE id = ?3",
                params![template.as_str(), now, principal_id],
            )
            .map_err(repository_error)?;
        if changed == 0 {
            return Err(PermissionsApplicationError::NotFound(format!(
                "no principal with id {principal_id}"
            )));
        }
        Ok(())
    }
}

fn repository_error(error: impl std::fmt::Display) -> PermissionsApplicationError {
    PermissionsApplicationError::infrastructure("sqlite", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    /// `NativeDatabase::new` runs every registered migration (including this context's) during
    /// construction, so no manual schema setup is needed here — matching how sibling contexts'
    /// repository tests already work (e.g. `SqliteCoordinationRepository`'s tests).
    fn repository() -> (SqlitePrincipalRepository, TempDirectory) {
        let directory = TempDirectory::new("permissions-principal-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        (SqlitePrincipalRepository::new(database), directory)
    }

    #[test]
    fn create_then_find_by_agent_id_round_trips() {
        let (repository, _directory) = repository();
        let principal = Principal::new(
            "principal-1".to_string(),
            "agent-1".to_string(),
            PolicyTemplateName::Trusted,
            None,
            None,
        )
        .unwrap();
        repository.create(&principal).unwrap();

        let found = repository
            .find_by_agent_id("agent-1")
            .unwrap()
            .expect("principal should be found");
        assert_eq!(found.id(), "principal-1");
        assert_eq!(found.template(), PolicyTemplateName::Trusted);
    }

    #[test]
    fn find_by_unknown_agent_id_returns_none() {
        let (repository, _directory) = repository();
        assert!(repository
            .find_by_agent_id("does-not-exist")
            .unwrap()
            .is_none());
    }

    #[test]
    fn update_template_changes_the_stored_template() {
        let (repository, _directory) = repository();
        let principal = Principal::new(
            "principal-1".to_string(),
            "agent-1".to_string(),
            PolicyTemplateName::Standard,
            None,
            None,
        )
        .unwrap();
        repository.create(&principal).unwrap();

        repository
            .update_template("principal-1", PolicyTemplateName::Readonly)
            .unwrap();

        let found = repository.find_by_agent_id("agent-1").unwrap().unwrap();
        assert_eq!(found.template(), PolicyTemplateName::Readonly);
    }

    #[test]
    fn update_template_for_an_unknown_id_fails() {
        let (repository, _directory) = repository();
        let result = repository.update_template("does-not-exist", PolicyTemplateName::Standard);
        assert!(result.is_err());
    }
}
