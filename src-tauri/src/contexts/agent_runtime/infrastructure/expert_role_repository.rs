use crate::contexts::agent_runtime::application::{AgentRuntimeApplicationError, ExpertRolePort};
use crate::contexts::agent_runtime::domain::{
    ExpertRole, ExpertRoleOrigin, ExpertRoleReviewPolicy,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Row};

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
        let database =
            NativeDatabase::new(directory.path().to_path_buf()).expect("test database");
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
}
