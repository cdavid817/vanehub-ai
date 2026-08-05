use crate::contexts::permissions::application::{
    GrantQuery, GrantRepository, PermissionsApplicationError,
};
use crate::contexts::permissions::domain::{Action, Effect, Grant, Resource, Scope};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Row};

#[derive(Clone)]
pub(crate) struct SqliteGrantRepository {
    database: NativeDatabase,
}

impl SqliteGrantRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    fn from_row(row: &Row<'_>) -> rusqlite::Result<Grant> {
        let scope_raw: String = row.get(5)?;
        let effect_raw: String = row.get(4)?;
        Ok(Grant {
            id: row.get(0)?,
            principal_id: row.get(1)?,
            action: Action::new(row.get::<_, String>(2)?),
            resource: Resource::new(row.get::<_, String>(3)?),
            effect: effect_from_str(&effect_raw),
            scope: scope_from_str(&scope_raw),
            session_id: row.get(6)?,
            project_key: row.get(7)?,
            created_at: row.get(8)?,
        })
    }
}

impl GrantRepository for SqliteGrantRepository {
    fn find_matching(
        &self,
        query: &GrantQuery<'_>,
    ) -> Result<Option<Grant>, PermissionsApplicationError> {
        // Every grant for this principal/action/resource is fetched and matched in Rust
        // (`Grant::matches`) rather than folded into the SQL predicate — Phase 1's grant volume
        // per principal/action/resource is small (bounded by how many distinct resources one
        // agent has ever been asked about), and this keeps the scope-matching rule defined once,
        // in the domain, instead of duplicated as SQL.
        let candidates: Vec<Grant> = self
            .database
            .connection()
            .map_err(repository_error)?
            .prepare(
                "SELECT id, principal_id, action, resource, effect, scope, session_id, \
                 project_key, created_at FROM permission_grants \
                 WHERE principal_id = ?1 AND action = ?2 AND resource = ?3",
            )
            .map_err(repository_error)?
            .query_map(
                params![query.principal_id, query.action.as_str(), query.resource.as_str()],
                Self::from_row,
            )
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;

        Ok(candidates.into_iter().find(|grant| {
            grant.matches(
                query.principal_id,
                query.action,
                query.resource,
                query.session_id,
                query.project_key,
            )
        }))
    }

    fn create(&self, grant: &Grant) -> Result<(), PermissionsApplicationError> {
        self.database
            .connection()
            .map_err(repository_error)?
            .execute(
                "INSERT INTO permission_grants \
                 (id, principal_id, action, resource, effect, scope, session_id, project_key, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    grant.id,
                    grant.principal_id,
                    grant.action.as_str(),
                    grant.resource.as_str(),
                    effect_to_str(grant.effect),
                    scope_to_str(grant.scope),
                    grant.session_id,
                    grant.project_key,
                    grant.created_at,
                ],
            )
            .map(|_| ())
            .map_err(repository_error)
    }
}

fn effect_to_str(effect: Effect) -> &'static str {
    match effect {
        Effect::Allow => "allow",
        Effect::Deny => "deny",
        Effect::Ask => "ask",
    }
}

fn effect_from_str(value: &str) -> Effect {
    match value {
        "allow" => Effect::Allow,
        "deny" => Effect::Deny,
        _ => Effect::Ask,
    }
}

fn scope_to_str(scope: Scope) -> &'static str {
    match scope {
        Scope::Once => "once",
        Scope::Session => "session",
        Scope::Project => "project",
        Scope::Global => "global",
    }
}

fn scope_from_str(value: &str) -> Scope {
    match value {
        "session" => Scope::Session,
        "project" => Scope::Project,
        "global" => Scope::Global,
        _ => Scope::Once,
    }
}

fn repository_error(error: impl std::fmt::Display) -> PermissionsApplicationError {
    PermissionsApplicationError::infrastructure("sqlite", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn repository() -> (SqliteGrantRepository, TempDirectory) {
        let directory = TempDirectory::new("permissions-grant-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        // A grant's foreign key requires an existing principal row.
        database
            .connection()
            .expect("connection")
            .execute(
                "INSERT INTO agent_principals (id, agent_id, template_name, created_at, updated_at) \
                 VALUES ('principal-1', 'agent-1', 'standard', '0', '0')",
                [],
            )
            .expect("seed principal");
        (SqliteGrantRepository::new(database), directory)
    }

    fn sample_grant(scope: Scope, session_id: Option<&str>, project_key: Option<&str>) -> Grant {
        Grant {
            id: "grant-1".to_string(),
            principal_id: "principal-1".to_string(),
            action: Action::file_write(),
            resource: Resource::file_path("a.txt"),
            effect: Effect::Allow,
            scope,
            session_id: session_id.map(str::to_string),
            project_key: project_key.map(str::to_string),
            created_at: "0".to_string(),
        }
    }

    #[test]
    fn create_then_find_matching_round_trips_a_session_scoped_grant() {
        let (repository, _directory) = repository();
        let grant = sample_grant(Scope::Session, Some("session-1"), None);
        repository.create(&grant).unwrap();

        let query = GrantQuery {
            principal_id: "principal-1",
            action: &Action::file_write(),
            resource: &Resource::file_path("a.txt"),
            session_id: "session-1",
            project_key: "project-1",
        };
        let found = repository
            .find_matching(&query)
            .unwrap()
            .expect("grant should match");
        assert_eq!(found.effect, Effect::Allow);
        assert_eq!(found.scope, Scope::Session);
    }

    #[test]
    fn find_matching_excludes_a_grant_scoped_to_a_different_session() {
        let (repository, _directory) = repository();
        let grant = sample_grant(Scope::Session, Some("session-1"), None);
        repository.create(&grant).unwrap();

        let query = GrantQuery {
            principal_id: "principal-1",
            action: &Action::file_write(),
            resource: &Resource::file_path("a.txt"),
            session_id: "session-2",
            project_key: "project-1",
        };
        assert!(repository.find_matching(&query).unwrap().is_none());
    }

    #[test]
    fn find_matching_with_no_grants_returns_none() {
        let (repository, _directory) = repository();
        let query = GrantQuery {
            principal_id: "principal-1",
            action: &Action::file_write(),
            resource: &Resource::file_path("a.txt"),
            session_id: "session-1",
            project_key: "project-1",
        };
        assert!(repository.find_matching(&query).unwrap().is_none());
    }
}
