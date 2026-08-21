use crate::contexts::agent_runtime::application::{AgentRuntimeApplicationError, ExpertRolePort};
use crate::contexts::agent_runtime::domain::{
    ExpertRole, ExpertRoleOrigin, ExpertRoleReviewPolicy,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Row};
use std::sync::Arc;

/// SQLite-backed store for reusable expert roles. Roles outlive any single session, so they are
/// persisted rather than derived from a session's seats.
#[derive(Clone)]
pub(crate) struct SqliteExpertRoleRepository {
    database: NativeDatabase,
}

impl SqliteExpertRoleRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl ExpertRolePort for SqliteExpertRoleRepository {
    fn list(&self) -> Result<Vec<ExpertRole>, AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                "SELECT id, display_name, avatar, color, responsibility, instruction, skill_ids, \
                 peer_reviewer, require_different_family, preferred_providers, origin, \
                 created_at, updated_at FROM expert_roles ORDER BY created_at, id",
            )
            .map_err(app_error)?;
        let rows = statement
            .query_map([], |row| Ok(read_role(row)))
            .map_err(app_error)?;
        let mut roles = Vec::new();
        for row in rows {
            roles.push(row.map_err(app_error)?);
        }
        Ok(roles)
    }

    fn upsert(&self, role: &ExpertRole) -> Result<(), AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .execute(
                r#"
                INSERT INTO expert_roles
                (id, display_name, avatar, color, responsibility, instruction, skill_ids,
                 peer_reviewer, require_different_family, preferred_providers, origin,
                 created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                ON CONFLICT(id) DO UPDATE SET
                    display_name = excluded.display_name,
                    avatar = excluded.avatar,
                    color = excluded.color,
                    responsibility = excluded.responsibility,
                    instruction = excluded.instruction,
                    skill_ids = excluded.skill_ids,
                    peer_reviewer = excluded.peer_reviewer,
                    require_different_family = excluded.require_different_family,
                    preferred_providers = excluded.preferred_providers,
                    updated_at = excluded.updated_at
                "#,
                params![
                    role.id,
                    role.display_name,
                    role.avatar,
                    role.color,
                    role.responsibility,
                    role.instruction,
                    encode_list(&role.skill_ids),
                    i64::from(role.review_policy.peer_reviewer),
                    i64::from(role.review_policy.require_different_family),
                    encode_list(&role.preferred_providers),
                    role.origin.as_str(),
                    role.created_at,
                    role.updated_at,
                ],
            )
            .map_err(app_error)?;
        Ok(())
    }

    fn delete(&self, role_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .execute("DELETE FROM expert_roles WHERE id = ?1", params![role_id])
            .map_err(app_error)?;
        Ok(())
    }
}

fn read_role(row: &Row<'_>) -> ExpertRole {
    ExpertRole {
        id: row.get::<_, String>(0).unwrap_or_default(),
        display_name: row.get::<_, String>(1).unwrap_or_default(),
        avatar: row.get::<_, String>(2).unwrap_or_default(),
        color: row.get::<_, String>(3).unwrap_or_default(),
        responsibility: row.get::<_, String>(4).unwrap_or_default(),
        instruction: row.get::<_, String>(5).unwrap_or_default(),
        skill_ids: decode_list(&row.get::<_, String>(6).unwrap_or_default()),
        review_policy: ExpertRoleReviewPolicy {
            peer_reviewer: row.get::<_, i64>(7).unwrap_or_default() != 0,
            require_different_family: row.get::<_, i64>(8).unwrap_or_default() != 0,
        },
        preferred_providers: decode_list(&row.get::<_, String>(9).unwrap_or_default()),
        origin: ExpertRoleOrigin::parse(&row.get::<_, String>(10).unwrap_or_default()),
        created_at: row.get::<_, String>(11).unwrap_or_default(),
        updated_at: row.get::<_, String>(12).unwrap_or_default(),
    }
}

fn encode_list(values: &[String]) -> String {
    serde_json::to_string(values).unwrap_or_else(|_| "[]".to_string())
}

fn decode_list(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn app_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Registry(error.to_string())
}

/// A role list that includes the built-ins, which live in the binary rather than in SQLite.
///
/// `ExpertRoleApplicationService` merges them for everything that goes through it, so
/// `list_expert_roles` reports architect, implementer and reviewer. The seat roster reads the
/// `ExpertRolePort` directly, and the repository alone knows only what the database holds --
/// nothing. A seat assigned a built-in role therefore resolved to no role at all, and the roster
/// fell back to naming the seat after its Agent. That made the handle `OnePiece` instead of
/// `架构师`, so an `@架构师` in a reply addressed nobody, the round ended `NobodyMentioned`, and
/// group chat quietly did not relay for exactly the roles the product ships.
///
/// Composed here rather than merged inside the roster so the port keeps meaning "every role there
/// is", which is what its one caller assumes.
pub(crate) struct BuiltinAwareExpertRoleRepository {
    inner: Arc<dyn ExpertRolePort>,
    builtins: Vec<ExpertRole>,
}

impl BuiltinAwareExpertRoleRepository {
    pub(crate) fn new(inner: Arc<dyn ExpertRolePort>, builtins: Vec<ExpertRole>) -> Self {
        Self { inner, builtins }
    }
}

impl ExpertRolePort for BuiltinAwareExpertRoleRepository {
    /// Built-ins first, then stored roles -- the same order
    /// `ExpertRoleApplicationService::list` uses, so the two views cannot disagree about
    /// precedence.
    fn list(&self) -> Result<Vec<ExpertRole>, AgentRuntimeApplicationError> {
        let mut roles = self.builtins.clone();
        roles.extend(self.inner.list()?);
        Ok(roles)
    }

    /// Writes belong to the database. A built-in is not editable, and the service that owns that
    /// rule rejects the attempt before it reaches any port.
    fn upsert(&self, role: &ExpertRole) -> Result<(), AgentRuntimeApplicationError> {
        self.inner.upsert(role)
    }

    fn delete(&self, role_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.inner.delete(role_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::ExpertRoleInput;
    use crate::platform::database::NativeDatabase;
    use crate::test_support::TempDirectory;

    fn role(id: &str, name: &str) -> ExpertRole {
        ExpertRole::new(
            id.to_string(),
            ExpertRoleInput {
                display_name: name.to_string(),
                avatar: "🏛".to_string(),
                color: "#9B7EBD".to_string(),
                responsibility: "负责系统设计".to_string(),
                instruction: "你是架构师".to_string(),
                skill_ids: vec!["skill-a".to_string()],
                review_policy: ExpertRoleReviewPolicy {
                    peer_reviewer: true,
                    require_different_family: true,
                },
                preferred_providers: vec!["anthropic".to_string()],
            },
            "2026-08-06T00:00:00Z".to_string(),
            "2026-08-06T00:00:00Z".to_string(),
        )
        .expect("valid role")
    }

    #[test]
    fn round_trips_a_role_including_its_lists_and_policy() {
        let directory = TempDirectory::new("expert role round trip");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("test database");
        let repository = SqliteExpertRoleRepository::new(database);

        repository.upsert(&role("r1", "架构师")).expect("insert");
        let stored = repository.list().expect("list");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].display_name, "架构师");
        assert_eq!(stored[0].skill_ids, vec!["skill-a".to_string()]);
        assert!(stored[0].review_policy.require_different_family);

        // Upsert is how an edit lands, so a second write of the same id must replace, not duplicate.
        repository.upsert(&role("r1", "主架构师")).expect("update");
        let updated = repository.list().expect("list again");
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0].display_name, "主架构师");

        repository.delete("r1").expect("delete");
        assert!(repository.list().expect("list after delete").is_empty());
    }

    /// The seat roster resolves a seat's role through this port, so a port that cannot see the
    /// built-ins reports "no role" for the three roles the product ships. The roster then names the
    /// seat after its Agent, and `@架构师` addresses nobody -- which is how multi-Agent handoff
    /// silently stopped relaying.
    #[test]
    fn the_builtin_aware_port_lists_builtins_alongside_stored_roles() {
        let directory = tempfile::tempdir().expect("temp dir");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("test database");
        let stored = Arc::new(SqliteExpertRoleRepository::new(database));
        stored.upsert(&role("r1", "自定义角色")).expect("insert");

        let builtins = crate::contexts::agent_runtime::infrastructure::builtin_expert_roles();
        assert!(
            !builtins.is_empty(),
            "the product ships built-in expert roles"
        );
        let port = BuiltinAwareExpertRoleRepository::new(stored.clone(), builtins.clone());

        let listed = port.list().expect("list");
        for builtin in &builtins {
            assert!(
                listed.iter().any(|role| role.id == builtin.id),
                "built-in {} is not visible to the seat roster",
                builtin.id,
            );
        }
        assert!(
            listed.iter().any(|role| role.id == "r1"),
            "a stored role stopped being listed once built-ins were merged",
        );
        assert_eq!(listed.len(), builtins.len() + 1);

        // Writes still belong to the database, and merging must not have made them disappear.
        port.delete("r1").expect("delete");
        assert!(stored.list().expect("stored").is_empty());
    }
}
