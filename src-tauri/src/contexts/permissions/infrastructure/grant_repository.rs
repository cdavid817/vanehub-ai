//! Storage for remembered decisions, under canonical identity.
//!
//! Two rules shape every statement here.
//!
//! Precedence is decided by the database, in one query, with an explicit rank and `LIMIT 1`. The
//! previous implementation loaded every candidate for a principal/action/resource and let Rust
//! `.find()` take the first one that matched, which made the effective permission a function of
//! whatever order SQLite returned rows in. There is no ordering guarantee without `ORDER BY`, so
//! that was a security rule delegated to a query plan.
//!
//! A remembered decision is written as the value of a canonical key, never appended. The three
//! partial unique indexes make that physical, and the `ON CONFLICT` targets below have to name
//! each index's own predicate because the owner column differs per scope.

use crate::contexts::permissions::application::{
    GrantQuery, GrantRepository, PendingGrantIntent, PermissionsApplicationError,
};
use crate::contexts::permissions::domain::{
    Action, CanonicalGrantKey, Grant, GrantActivationState, PersistedEffect, RememberedScope,
    Resource, Scope,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection, OptionalExtension, Row};

#[derive(Clone)]
pub(crate) struct SqliteGrantRepository {
    database: NativeDatabase,
}

impl SqliteGrantRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

const GRANT_COLUMNS: &str = "id, principal_id, action, resource, effect, scope, session_id, \
                             project_key, revision, activation_state, resolution_id, created_at, \
                             updated_at";

/// Reads one stored row back into the domain.
///
/// Every enumerated column is parsed rather than defaulted. The previous reader mapped an
/// unrecognised effect to `Ask` and an unrecognised scope to `Once`, which turned a corrupt row
/// into an invisible no-op — the row still existed, still occupied its canonical key, and simply
/// never matched anything. Refusing makes the same corruption visible to the caller.
fn grant_from_row(row: &Row<'_>) -> rusqlite::Result<Grant> {
    let scope_token: String = row.get(5)?;
    let session_id: Option<String> = row.get(6)?;
    let project_key: Option<String> = row.get(7)?;
    let effect_token: String = row.get(4)?;
    let activation_token: String = row.get(9)?;

    let scope = match scope_token.as_str() {
        "session" => Scope::Session,
        "project" => Scope::Project,
        "global" => Scope::Global,
        _ => Scope::Once,
    };
    let binding = RememberedScope::parse(scope, session_id.as_deref(), project_key.as_deref())
        .map_err(|error| invalid_column(5, error))?;
    let effect =
        PersistedEffect::from_token(&effect_token).map_err(|error| invalid_column(4, error))?;
    let activation_state = GrantActivationState::from_token(&activation_token)
        .map_err(|error| invalid_column(9, error))?;
    let key = CanonicalGrantKey::new(
        row.get::<_, String>(1)?,
        Action::new(row.get::<_, String>(2)?),
        Resource::new(row.get::<_, String>(3)?),
        binding,
    )
    .map_err(|error| invalid_column(1, error))?;

    Ok(Grant {
        id: row.get(0)?,
        key,
        effect,
        revision: row.get(8)?,
        activation_state,
        resolution_id: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn invalid_column(
    index: usize,
    error: crate::contexts::permissions::domain::PermissionsDomainError,
) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(index, error.to_string(), rusqlite::types::Type::Text)
}

/// The stored token for a binding, derived from the domain's own scope rather than re-listed here.
/// Two independent spellings of "session" is how a rename becomes a silently unmatched row.
fn scope_token(scope: &RememberedScope) -> &'static str {
    match scope.scope() {
        Scope::Session => "session",
        Scope::Project => "project",
        Scope::Global => "global",
        // Unreachable: `RememberedScope` cannot be built from `Once`. Mapped rather than panicked
        // so a future variant cannot turn a construction rule into a crash on a write path.
        Scope::Once => "once",
    }
}

/// The `ON CONFLICT` target for one binding.
///
/// Each partial unique index needs its own predicate repeated here: SQLite matches an upsert
/// target against the index's columns *and* its `WHERE` clause, so a single target naming only the
/// columns would not resolve to any of the three.
fn conflict_target(scope: &RememberedScope) -> &'static str {
    match scope {
        RememberedScope::Session(_) => {
            "(principal_id, action, resource, session_id) WHERE scope = 'session'"
        }
        RememberedScope::Project(_) => {
            "(principal_id, action, resource, project_key) WHERE scope = 'project'"
        }
        RememberedScope::Global => "(principal_id, action, resource) WHERE scope = 'global'",
    }
}

/// The ranked precedence read, as one statement.
///
/// `revision DESC, id DESC` after the scope rank is unreachable while the unique indexes hold —
/// one key has one row. It is there so the query is *total*: a database that somehow carries two
/// rows for one key still answers deterministically instead of falling back to row order, which is
/// the failure this whole change exists to remove.
const FIND_EFFECTIVE_GRANT: &str =
    "SELECT id, principal_id, action, resource, effect, scope, session_id, project_key, \
            revision, activation_state, resolution_id, created_at, updated_at \
     FROM permission_grants \
     WHERE principal_id = ?1 AND action = ?2 AND resource = ?3 \
       AND activation_state = 'active' \
       AND ( (scope = 'session' AND session_id = ?4) \
          OR (scope = 'project' AND project_key = ?5) \
          OR scope = 'global' ) \
     ORDER BY CASE scope WHEN 'session' THEN 3 WHEN 'project' THEN 2 ELSE 1 END DESC, \
              revision DESC, id DESC \
     LIMIT 1";

/// Writes the current value of one canonical key, on a caller-supplied connection.
///
/// Free function rather than a method so the atomic resolution transaction can perform the same
/// write inside its own transaction without either path reimplementing the upsert. Two copies of
/// this SQL is exactly how the revision rule would drift between the direct and transactional
/// paths.
pub(crate) fn upsert_pending_grant_intent_on(
    connection: &Connection,
    intent: &PendingGrantIntent,
) -> rusqlite::Result<Grant> {
    let pending = GrantActivationState::PendingDelivery.token();
    let statement = format!(
        "INSERT INTO permission_grants \
         (id, principal_id, action, resource, effect, scope, session_id, project_key, \
          revision, activation_state, resolution_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, '{pending}', ?9, ?10, ?10) \
         ON CONFLICT {target} DO UPDATE SET \
           effect = excluded.effect, \
           revision = CASE WHEN permission_grants.resolution_id IS excluded.resolution_id \
                           THEN permission_grants.revision \
                           ELSE permission_grants.revision + 1 END, \
           activation_state = CASE WHEN permission_grants.resolution_id IS excluded.resolution_id \
                                   THEN permission_grants.activation_state \
                                   ELSE '{pending}' END, \
           resolution_id = excluded.resolution_id, \
           updated_at = excluded.updated_at",
        target = conflict_target(&intent.key.scope)
    );
    connection.execute(
        &statement,
        params![
            intent.id,
            intent.key.principal_id,
            intent.key.action.as_str(),
            intent.key.resource.as_str(),
            intent.effect.token(),
            scope_token(&intent.key.scope),
            intent.key.scope.session_id(),
            intent.key.scope.project_key(),
            intent.resolution_id,
            intent.now,
        ],
    )?;
    read_grant_by_key(connection, &intent.key)
}

/// Reads back the row a canonical key currently holds.
///
/// Needed because the upsert may have inserted or updated, and the id of the surviving row is the
/// one that was already there in the update case — returning the intent's own id would hand the
/// caller a row that does not exist.
fn read_grant_by_key(connection: &Connection, key: &CanonicalGrantKey) -> rusqlite::Result<Grant> {
    let statement = format!(
        "SELECT {GRANT_COLUMNS} FROM permission_grants \
         WHERE principal_id = ?1 AND action = ?2 AND resource = ?3 AND scope = ?4 \
           AND COALESCE(session_id, '') = ?5 AND COALESCE(project_key, '') = ?6"
    );
    connection.query_row(
        &statement,
        params![
            key.principal_id,
            key.action.as_str(),
            key.resource.as_str(),
            scope_token(&key.scope),
            key.scope.session_id().unwrap_or_default(),
            key.scope.project_key().unwrap_or_default(),
        ],
        grant_from_row,
    )
}

/// Makes every intent recorded for one resolution visible to evaluation.
///
/// Guarded on the current state rather than written unconditionally so a repeated acknowledgement
/// cannot advance a revision. `updated_at` moves only on the transition, which keeps the row's
/// timestamp meaning "when this became active" rather than "when it was last asked about".
pub(crate) fn activate_grant_for_resolution_on(
    connection: &Connection,
    resolution_id: &str,
    now: &str,
) -> rusqlite::Result<usize> {
    connection.execute(
        &format!(
            "UPDATE permission_grants SET activation_state = '{active}', updated_at = ?2 \
             WHERE resolution_id = ?1 AND activation_state = '{pending}'",
            active = GrantActivationState::Active.token(),
            pending = GrantActivationState::PendingDelivery.token(),
        ),
        params![resolution_id, now],
    )
}

impl SqliteGrantRepository {
    /// Writes the value of one canonical key as `pending_delivery` on this repository's own
    /// connection.
    ///
    /// Inherent rather than part of `GrantRepository`, because the only production writer is the
    /// resolution transaction, which calls [`upsert_pending_grant_intent_on`] with its own
    /// connection. Exposing a standalone write on the port would be a way to create authority
    /// without a decision behind it. This exists so the repository's own tests can exercise the
    /// same SQL the transaction runs.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn upsert_pending_grant_intent(
        &self,
        intent: &PendingGrantIntent,
    ) -> Result<Grant, PermissionsApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        upsert_pending_grant_intent_on(&connection, intent).map_err(repository_error)
    }

    /// Activates an acknowledged resolution's intent, on this repository's own connection. Same
    /// reasoning as above: production activates inside `acknowledge_delivery_and_activate`.
    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn activate_grant_for_resolution(
        &self,
        resolution_id: &str,
        now: &str,
    ) -> Result<(), PermissionsApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        activate_grant_for_resolution_on(&connection, resolution_id, now)
            .map(|_| ())
            .map_err(repository_error)
    }
}

impl GrantRepository for SqliteGrantRepository {
    fn find_effective_grant(
        &self,
        query: &GrantQuery<'_>,
    ) -> Result<Option<Grant>, PermissionsApplicationError> {
        self.database
            .connection()
            .map_err(repository_error)?
            .query_row(
                FIND_EFFECTIVE_GRANT,
                params![
                    query.principal_id,
                    query.action.as_str(),
                    query.resource.as_str(),
                    query.session_id,
                    query.project_key,
                ],
                grant_from_row,
            )
            .optional()
            .map_err(repository_error)
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

    fn key(scope: RememberedScope) -> CanonicalGrantKey {
        CanonicalGrantKey::new(
            "principal-1",
            Action::file_write(),
            Resource::file_path("a.txt"),
            scope,
        )
        .expect("well-formed key")
    }

    fn intent(
        id: &str,
        scope: RememberedScope,
        effect: PersistedEffect,
        resolution_id: &str,
    ) -> PendingGrantIntent {
        PendingGrantIntent {
            id: id.to_string(),
            key: key(scope),
            effect,
            resolution_id: resolution_id.to_string(),
            now: "0".to_string(),
        }
    }

    /// Writes an intent and immediately activates it, which is what a delivered approval does.
    fn remember(
        repository: &SqliteGrantRepository,
        id: &str,
        scope: RememberedScope,
        effect: PersistedEffect,
        resolution_id: &str,
    ) {
        repository
            .upsert_pending_grant_intent(&intent(id, scope, effect, resolution_id))
            .expect("upsert intent");
        repository
            .activate_grant_for_resolution(resolution_id, "0")
            .expect("activate");
    }

    fn query<'a>(session_id: &'a str, project_key: &'a str) -> GrantQuery<'a> {
        GrantQuery {
            principal_id: "principal-1",
            action: &ACTION,
            resource: &RESOURCE,
            session_id,
            project_key,
        }
    }

    static ACTION: std::sync::LazyLock<Action> = std::sync::LazyLock::new(Action::file_write);
    static RESOURCE: std::sync::LazyLock<Resource> =
        std::sync::LazyLock::new(|| Resource::file_path("a.txt"));

    fn count_rows(repository: &SqliteGrantRepository) -> i64 {
        repository
            .database
            .connection()
            .expect("connection")
            .query_row("SELECT COUNT(*) FROM permission_grants", [], |row| {
                row.get(0)
            })
            .expect("count rows")
    }

    #[test]
    fn a_remembered_session_grant_round_trips() {
        let (repository, _directory) = repository();
        remember(
            &repository,
            "grant-1",
            RememberedScope::Session("session-1".into()),
            PersistedEffect::Allow,
            "res-1",
        );

        let found = repository
            .find_effective_grant(&query("session-1", "project-1"))
            .unwrap()
            .expect("grant should match");
        assert_eq!(found.effect, PersistedEffect::Allow);
        assert_eq!(found.scope(), Scope::Session);
        assert_eq!(found.revision, 1);
    }

    #[test]
    fn a_grant_scoped_to_another_session_does_not_apply() {
        let (repository, _directory) = repository();
        remember(
            &repository,
            "grant-1",
            RememberedScope::Session("session-1".into()),
            PersistedEffect::Allow,
            "res-1",
        );

        assert!(repository
            .find_effective_grant(&query("session-2", "project-1"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn no_grants_means_no_effective_grant() {
        let (repository, _directory) = repository();
        assert!(repository
            .find_effective_grant(&query("session-1", "project-1"))
            .unwrap()
            .is_none());
    }

    /// `permissions-core`'s "Session decision overrides broader grants".
    ///
    /// Writing the three rows in every order is what makes this an assertion about the ranking
    /// rule rather than about SQLite. A lookup that returned whichever row the table handed back
    /// first passes for some permutations and fails for others — and the same defect would let a
    /// `VACUUM` or a new query plan silently change a security decision.
    #[test]
    fn the_exact_session_grant_wins_from_every_write_order() {
        let scoped = [
            (
                RememberedScope::Session("session-1".to_string()),
                PersistedEffect::Allow,
            ),
            (
                RememberedScope::Project("project-1".to_string()),
                PersistedEffect::Deny,
            ),
            (RememberedScope::Global, PersistedEffect::Deny),
        ];
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let (repository, _directory) = repository();
            for (index, position) in order.iter().enumerate() {
                let (scope, effect) = &scoped[*position];
                remember(
                    &repository,
                    &format!("grant-{index}"),
                    scope.clone(),
                    *effect,
                    &format!("res-{index}"),
                );
            }

            let found = repository
                .find_effective_grant(&query("session-1", "project-1"))
                .unwrap()
                .expect("one of the three grants applies");
            assert_eq!(
                found.scope(),
                Scope::Session,
                "write order {order:?} selected {:?} instead of the exact session grant",
                found.scope()
            );
            assert_eq!(found.effect, PersistedEffect::Allow);
        }
    }

    #[test]
    fn a_project_grant_outranks_a_global_one_when_no_session_grant_applies() {
        let (repository, _directory) = repository();
        remember(
            &repository,
            "grant-global",
            RememberedScope::Global,
            PersistedEffect::Allow,
            "res-1",
        );
        remember(
            &repository,
            "grant-project",
            RememberedScope::Project("project-1".into()),
            PersistedEffect::Deny,
            "res-2",
        );

        let found = repository
            .find_effective_grant(&query("session-1", "project-1"))
            .unwrap()
            .expect("the project grant applies");
        assert_eq!(found.scope(), Scope::Project);
        assert_eq!(found.effect, PersistedEffect::Deny);
    }

    /// `permissions-core`'s "A repeated remembered decision updates one key".
    #[test]
    fn remembering_the_same_canonical_key_twice_leaves_one_row_at_a_higher_revision() {
        let (repository, _directory) = repository();
        remember(
            &repository,
            "grant-1",
            RememberedScope::Session("session-1".into()),
            PersistedEffect::Allow,
            "res-1",
        );
        remember(
            &repository,
            "grant-2",
            RememberedScope::Session("session-1".into()),
            PersistedEffect::Deny,
            "res-2",
        );

        assert_eq!(count_rows(&repository), 1);
        let found = repository
            .find_effective_grant(&query("session-1", "project-1"))
            .unwrap()
            .expect("the remembered grant applies");
        assert_eq!(found.effect, PersistedEffect::Deny);
        assert_eq!(found.revision, 2);
        // The surviving row keeps the id it was created with; the second write is an update to the
        // key's value, not a new row wearing the second intent's id.
        assert_eq!(found.id, "grant-1");
    }

    #[test]
    fn an_intent_is_invisible_to_evaluation_until_its_delivery_is_acknowledged() {
        let (repository, _directory) = repository();
        repository
            .upsert_pending_grant_intent(&intent(
                "grant-1",
                RememberedScope::Global,
                PersistedEffect::Allow,
                "res-1",
            ))
            .expect("upsert intent");

        assert!(
            repository
                .find_effective_grant(&query("session-1", "project-1"))
                .unwrap()
                .is_none(),
            "an undelivered approval authorized the next evaluation"
        );

        repository
            .activate_grant_for_resolution("res-1", "1")
            .expect("activate");
        assert!(repository
            .find_effective_grant(&query("session-1", "project-1"))
            .unwrap()
            .is_some());
    }

    #[test]
    fn a_repeated_acknowledgement_activates_one_revision_and_no_more() {
        let (repository, _directory) = repository();
        repository
            .upsert_pending_grant_intent(&intent(
                "grant-1",
                RememberedScope::Global,
                PersistedEffect::Allow,
                "res-1",
            ))
            .expect("upsert intent");

        repository
            .activate_grant_for_resolution("res-1", "1")
            .expect("first acknowledgement");
        repository
            .activate_grant_for_resolution("res-1", "2")
            .expect("duplicate acknowledgement");

        let found = repository
            .find_effective_grant(&query("session-1", "project-1"))
            .unwrap()
            .expect("the grant is active");
        assert_eq!(found.revision, 1);
        assert_eq!(count_rows(&repository), 1);
        // `updated_at` records the activation, not the second question about it.
        assert_eq!(found.updated_at, "1");
    }

    #[test]
    fn replaying_one_intent_neither_bumps_the_revision_nor_deactivates_the_grant() {
        let (repository, _directory) = repository();
        let replayed = intent(
            "grant-1",
            RememberedScope::Global,
            PersistedEffect::Allow,
            "res-1",
        );
        repository
            .upsert_pending_grant_intent(&replayed)
            .expect("first write");
        repository
            .activate_grant_for_resolution("res-1", "1")
            .expect("activate");

        repository
            .upsert_pending_grant_intent(&replayed)
            .expect("replayed write");

        let found = repository
            .find_effective_grant(&query("session-1", "project-1"))
            .unwrap()
            .expect("the grant stayed active");
        assert_eq!(found.revision, 1);
        assert_eq!(count_rows(&repository), 1);
    }

    /// `permissions-core`'s "concurrent remember leaves one authoritative revision".
    ///
    /// A hundred writers against one canonical key, released together. The unique index is what
    /// makes the outcome a single row; the revision is what makes "which of these is authoritative"
    /// answerable without consulting a clock.
    #[test]
    fn a_hundred_concurrent_remembers_of_one_key_leave_one_authoritative_row() {
        let (repository, _directory) = repository();
        let remembers = 100;
        // Eight concurrent writers sharing the 100 remembers, not 100 threads.
        //
        // The property is that concurrent upserts on one canonical key converge to one row with a
        // monotonic revision, and eight simultaneous writers on one key is genuine contention for
        // that. A hundred threads additionally requires the connection pool to hand out a hundred
        // simultaneous connections, which it will not: it holds twelve and times a checkout out
        // after five seconds. Under load that made this test fail on pool exhaustion — a fact about
        // r2d2, not about the upsert — while passing every time it was run alone, which is the
        // worst possible shape for a concurrency assertion.
        let writers = 8;
        let gate = std::sync::Arc::new(std::sync::Barrier::new(writers));
        let repository = std::sync::Arc::new(repository);

        std::thread::scope(|scope| {
            for writer in 0..writers {
                let repository = repository.clone();
                let gate = gate.clone();
                scope.spawn(move || {
                    gate.wait();
                    for index in (writer..remembers).step_by(writers) {
                        repository
                            .upsert_pending_grant_intent(&intent(
                                &format!("grant-{index}"),
                                RememberedScope::Global,
                                if index % 2 == 0 {
                                    PersistedEffect::Allow
                                } else {
                                    PersistedEffect::Deny
                                },
                                &format!("res-{index}"),
                            ))
                            .unwrap_or_else(|error| {
                                panic!(
                                    "remember {index} did not commit a value for the key: {error}"
                                )
                            });
                    }
                });
            }
        });

        assert_eq!(count_rows(&repository), 1);
        let stored: (i64, String) = repository
            .database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT revision, activation_state FROM permission_grants",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("the surviving row");
        assert_eq!(
            stored.0, remembers as i64,
            "the revision did not advance once per distinct remember"
        );
        assert_eq!(stored.1, "pending_delivery");
    }

    /// Guards the non-goal explicitly. This change adds deterministic selection, not a wider
    /// matcher — a prefix or path-normalisation rule introduced by accident here would silently
    /// widen every grant a user has ever stored.
    #[test]
    fn selection_stays_exact_on_principal_action_and_resource() {
        let (repository, _directory) = repository();
        remember(
            &repository,
            "grant-1",
            RememberedScope::Global,
            PersistedEffect::Allow,
            "res-1",
        );

        let other_action = Action::shell_exec();
        let other_resource = Resource::file_path("a.txt.bak");
        for candidate in [
            GrantQuery {
                principal_id: "principal-2",
                action: &ACTION,
                resource: &RESOURCE,
                session_id: "session-1",
                project_key: "project-1",
            },
            GrantQuery {
                principal_id: "principal-1",
                action: &other_action,
                resource: &RESOURCE,
                session_id: "session-1",
                project_key: "project-1",
            },
            GrantQuery {
                principal_id: "principal-1",
                action: &ACTION,
                resource: &other_resource,
                session_id: "session-1",
                project_key: "project-1",
            },
        ] {
            assert!(
                repository
                    .find_effective_grant(&candidate)
                    .unwrap()
                    .is_none(),
                "a non-exact query matched a stored grant"
            );
        }
    }

    #[test]
    fn a_row_written_before_the_ledger_still_reads_back_as_an_active_grant() {
        // Migration 95 carries pre-existing grants forward with no resolution id. They have no
        // delivery to wait for, so they are active, and the reader has to accept a null there
        // rather than treat it as a malformed row.
        let (repository, _directory) = repository();
        repository
            .database
            .connection()
            .expect("connection")
            .execute(
                "INSERT INTO permission_grants \
                 (id, principal_id, action, resource, effect, scope, revision, activation_state, \
                  resolution_id, created_at, updated_at) \
                 VALUES ('legacy-1', 'principal-1', 'file.write', 'a.txt', 'allow', 'global', 1, \
                  'active', NULL, '0', '0')",
                [],
            )
            .expect("legacy row");

        let found = repository
            .find_effective_grant(&query("session-1", "project-1"))
            .unwrap()
            .expect("the migrated grant applies");
        assert_eq!(found.resolution_id, None);
        assert_eq!(found.effect, PersistedEffect::Allow);
    }
}
