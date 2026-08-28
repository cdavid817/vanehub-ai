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

    /// Insert-if-absent then read, both on one checked-out connection.
    ///
    /// `DO NOTHING` rather than a preceding `SELECT`: the read and the write are not atomic, and
    /// `agent_id` is unique, so a losing racer would surface a constraint error that evaluation
    /// then fails closed on — a first-use `Ask` produced by two generations starting together
    /// rather than by policy. The follow-up read runs on the same connection so it observes either
    /// this call's insert or the winner's.
    fn get_or_create(
        &self,
        agent_id: &str,
        id_hint: &str,
        default_template: PolicyTemplateName,
    ) -> Result<Principal, PermissionsApplicationError> {
        let now = crate::platform::clock::SystemClock.rfc3339();
        // Built through the domain constructor rather than written as raw column values, so the
        // row this inserts is one the domain would accept — a first-use principal that skipped
        // `Principal::new` could carry a delegation parent nothing has enabled yet.
        let candidate = Principal::new(
            id_hint.to_string(),
            agent_id.to_string(),
            default_template,
            None,
            None,
        )?;
        let budget_config = candidate
            .budget_config()
            .map(serde_json::to_string)
            .transpose()
            .map_err(repository_error)?;
        let connection = self.database.connection().map_err(repository_error)?;
        connection
            .execute(
                "INSERT INTO agent_principals \
                 (id, agent_id, template_name, parent_principal_id, budget_config, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6) \
                 ON CONFLICT(agent_id) DO NOTHING",
                params![
                    candidate.id(),
                    candidate.agent_id(),
                    candidate.template().as_str(),
                    candidate.parent_principal_id(),
                    budget_config,
                    now,
                ],
            )
            .map_err(repository_error)?;
        connection
            .query_row(
                "SELECT id, agent_id, template_name, parent_principal_id, budget_config \
                 FROM agent_principals WHERE agent_id = ?1",
                params![agent_id],
                Self::from_row,
            )
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
    fn get_or_create_then_find_by_agent_id_round_trips() {
        let (repository, _directory) = repository();
        let created = repository
            .get_or_create("agent-1", "principal-1", PolicyTemplateName::Trusted)
            .unwrap();
        assert_eq!(created.id(), "principal-1");

        let found = repository
            .find_by_agent_id("agent-1")
            .unwrap()
            .expect("principal should be found");
        assert_eq!(found.id(), "principal-1");
        assert_eq!(found.template(), PolicyTemplateName::Trusted);
    }

    #[test]
    fn get_or_create_returns_the_existing_row_and_ignores_the_id_hint() {
        let (repository, _directory) = repository();
        repository
            .get_or_create("agent-1", "principal-1", PolicyTemplateName::Trusted)
            .unwrap();

        // The hint is only used when this call is the one that inserts. Returning a principal
        // under the second hint would hand the caller an id no row has.
        let existing = repository
            .get_or_create("agent-1", "principal-2", PolicyTemplateName::Readonly)
            .unwrap();

        assert_eq!(existing.id(), "principal-1");
        assert_eq!(
            existing.template(),
            PolicyTemplateName::Trusted,
            "get-or-create reassigned an existing principal's template"
        );
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
        repository
            .get_or_create("agent-1", "principal-1", PolicyTemplateName::Standard)
            .unwrap();

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

    /// Below the pool ceiling, so a connection-checkout timeout can never be mistaken for the
    /// write race these tests are about.
    const RACERS: usize = 8;

    fn principal_rows(repository: &SqlitePrincipalRepository) -> i64 {
        repository
            .database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT COUNT(*) FROM agent_principals WHERE agent_id = 'agent-1'",
                [],
                |row| row.get(0),
            )
            .expect("count principals")
    }

    /// Why `get_or_create` exists, stated as a test rather than a comment.
    ///
    /// The interleaving is pinned rather than raced for. Both barriers matter: the first releases
    /// every reader together, and the second holds every writer until all of them have already
    /// answered "absent". A scheduler will not reliably produce that ordering on its own, which is
    /// why the same test without the second barrier passes on most runs while the defect is still
    /// there — it is worth being explicit that this is the trap, because a "flaky" concurrency test
    /// that gets quietly relaxed is how the bug comes back.
    #[test]
    fn a_read_then_write_first_use_loses_the_race_it_is_replacing() {
        let (repository, _directory) = repository();
        let read_gate = std::sync::Arc::new(std::sync::Barrier::new(RACERS));
        let write_gate = std::sync::Arc::new(std::sync::Barrier::new(RACERS));
        let repository = std::sync::Arc::new(repository);

        let outcomes: Vec<bool> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..RACERS)
                .map(|index| {
                    let repository = repository.clone();
                    let read_gate = read_gate.clone();
                    let write_gate = write_gate.clone();
                    scope.spawn(move || {
                        read_gate.wait();
                        let existing = repository
                            .find_by_agent_id("agent-1")
                            .expect("read succeeds");
                        write_gate.wait();
                        if existing.is_some() {
                            return true;
                        }
                        // The pre-change insert, spelled out here rather than kept as a production
                        // method nothing calls any more.
                        repository
                            .database
                            .connection()
                            .expect("connection")
                            .execute(
                                "INSERT INTO agent_principals \
                                 (id, agent_id, template_name, created_at, updated_at) \
                                 VALUES (?1, 'agent-1', 'standard', '0', '0')",
                                params![format!("principal-{index}")],
                            )
                            .is_ok()
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("racer thread"))
                .collect()
        });

        let lost = outcomes.iter().filter(|succeeded| !**succeeded).count();
        assert!(
            lost > 0,
            "the read-then-write race did not reproduce, so this test is no longer evidence"
        );
        // Exactly one row either way — the unique index held. The damage is the error the losers
        // saw, which evaluation turns into a fail-closed Ask that policy never asked for.
        assert_eq!(principal_rows(&repository), 1);
    }

    /// `permissions-core`'s "Concurrent first evaluation of one agent", against the atomic
    /// operation that replaces the race above. Same pinned interleaving, no losers.
    #[test]
    fn concurrent_first_use_of_one_agent_resolves_to_a_single_principal() {
        let (repository, _directory) = repository();
        let read_gate = std::sync::Arc::new(std::sync::Barrier::new(RACERS));
        let write_gate = std::sync::Arc::new(std::sync::Barrier::new(RACERS));
        let repository = std::sync::Arc::new(repository);

        let outcomes: Vec<Result<Principal, PermissionsApplicationError>> =
            std::thread::scope(|scope| {
                let handles: Vec<_> = (0..RACERS)
                    .map(|index| {
                        let repository = repository.clone();
                        let read_gate = read_gate.clone();
                        let write_gate = write_gate.clone();
                        scope.spawn(move || {
                            read_gate.wait();
                            let existing = repository.find_by_agent_id("agent-1")?;
                            write_gate.wait();
                            let _ = existing;
                            repository.get_or_create(
                                "agent-1",
                                &format!("principal-{index}"),
                                PolicyTemplateName::Standard,
                            )
                        })
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|handle| handle.join().expect("racer thread"))
                    .collect()
            });

        let failures: Vec<_> = outcomes.iter().filter(|outcome| outcome.is_err()).collect();
        assert!(
            failures.is_empty(),
            "{} of {RACERS} concurrent first evaluations lost the create race: {:?}",
            failures.len(),
            failures
        );
        assert_eq!(principal_rows(&repository), 1);
        // Every caller has to come back with the same principal, not merely "a" principal: the
        // grants an evaluation reads are keyed by this id.
        let ids: std::collections::BTreeSet<String> = outcomes
            .iter()
            .map(|outcome| outcome.as_ref().expect("checked above").id().to_string())
            .collect();
        assert_eq!(ids.len(), 1, "concurrent first use resolved to {ids:?}");
    }
}
