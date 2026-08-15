use rusqlite::{params, Row};

use super::super::application::GoalRepository;
use super::super::domain::{Goal, GoalLink, GoalLinkTarget, GoalStatus};
use crate::platform::database::NativeDatabase;

pub(crate) struct SqliteGoalRepository {
    database: NativeDatabase,
}

impl SqliteGoalRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

const GOAL_COLUMNS: &str =
    "id, title, description, acceptance_notes, status, project_path, created_at, updated_at";

impl GoalRepository for SqliteGoalRepository {
    fn insert_goal(&self, goal: &Goal) -> Result<(), String> {
        let connection = self.database.connection().map_err(stringify)?;
        connection
            .execute(
                "INSERT INTO goals (id, title, description, acceptance_notes, status, project_path, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    goal.id,
                    goal.title,
                    goal.description,
                    goal.acceptance_notes,
                    goal.status.as_str(),
                    goal.project_path,
                    goal.created_at,
                    goal.updated_at
                ],
            )
            .map_err(stringify)?;
        Ok(())
    }

    fn update_goal(&self, goal: &Goal) -> Result<(), String> {
        let connection = self.database.connection().map_err(stringify)?;
        connection
            .execute(
                "UPDATE goals
                 SET title = ?2, description = ?3, acceptance_notes = ?4, status = ?5,
                     project_path = ?6, updated_at = ?7
                 WHERE id = ?1",
                params![
                    goal.id,
                    goal.title,
                    goal.description,
                    goal.acceptance_notes,
                    goal.status.as_str(),
                    goal.project_path,
                    goal.updated_at
                ],
            )
            .map_err(stringify)?;
        Ok(())
    }

    fn delete_goal(&self, goal_id: &str) -> Result<(), String> {
        let connection = self.database.connection().map_err(stringify)?;
        // Links are removed explicitly rather than relying on the cascade, so a
        // connection with foreign keys disabled cannot leave orphans behind.
        connection
            .execute(
                "DELETE FROM goal_links WHERE goal_id = ?1",
                params![goal_id],
            )
            .map_err(stringify)?;
        connection
            .execute("DELETE FROM goals WHERE id = ?1", params![goal_id])
            .map_err(stringify)?;
        Ok(())
    }

    fn find_goal(&self, goal_id: &str) -> Result<Option<Goal>, String> {
        let connection = self.database.connection().map_err(stringify)?;
        let mut statement = connection
            .prepare(&format!("SELECT {GOAL_COLUMNS} FROM goals WHERE id = ?1"))
            .map_err(stringify)?;
        let mut rows = statement.query(params![goal_id]).map_err(stringify)?;
        match rows.next().map_err(stringify)? {
            Some(row) => Ok(Some(read_goal(row)?)),
            None => Ok(None),
        }
    }

    fn list_goals(&self) -> Result<Vec<Goal>, String> {
        let connection = self.database.connection().map_err(stringify)?;
        let mut statement = connection
            .prepare(&format!(
                "SELECT {GOAL_COLUMNS} FROM goals ORDER BY created_at DESC, id DESC"
            ))
            .map_err(stringify)?;
        let mut rows = statement.query([]).map_err(stringify)?;
        let mut goals = Vec::new();
        while let Some(row) = rows.next().map_err(stringify)? {
            goals.push(read_goal(row)?);
        }
        Ok(goals)
    }

    fn insert_link(&self, link: &GoalLink) -> Result<(), String> {
        let connection = self.database.connection().map_err(stringify)?;
        connection
            .execute(
                "INSERT INTO goal_links (goal_id, target_kind, target_id, linked_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    link.goal_id,
                    link.target_kind.as_str(),
                    link.target_id,
                    link.linked_at
                ],
            )
            .map_err(stringify)?;
        Ok(())
    }

    fn delete_link(
        &self,
        goal_id: &str,
        target_kind: GoalLinkTarget,
        target_id: &str,
    ) -> Result<(), String> {
        let connection = self.database.connection().map_err(stringify)?;
        connection
            .execute(
                "DELETE FROM goal_links
                 WHERE goal_id = ?1 AND target_kind = ?2 AND target_id = ?3",
                params![goal_id, target_kind.as_str(), target_id],
            )
            .map_err(stringify)?;
        Ok(())
    }

    fn list_links(&self, goal_id: &str) -> Result<Vec<GoalLink>, String> {
        let connection = self.database.connection().map_err(stringify)?;
        let mut statement = connection
            .prepare(
                "SELECT goal_id, target_kind, target_id, linked_at
                 FROM goal_links WHERE goal_id = ?1
                 ORDER BY linked_at ASC, target_id ASC",
            )
            .map_err(stringify)?;
        let mut rows = statement.query(params![goal_id]).map_err(stringify)?;
        let mut links = Vec::new();
        while let Some(row) = rows.next().map_err(stringify)? {
            let kind: String = row.get(1).map_err(stringify)?;
            links.push(GoalLink {
                goal_id: row.get(0).map_err(stringify)?,
                target_kind: GoalLinkTarget::parse(&kind).map_err(|error| error.to_string())?,
                target_id: row.get(2).map_err(stringify)?,
                linked_at: row.get(3).map_err(stringify)?,
            });
        }
        Ok(links)
    }

    fn link_exists(
        &self,
        goal_id: &str,
        target_kind: GoalLinkTarget,
        target_id: &str,
    ) -> Result<bool, String> {
        let connection = self.database.connection().map_err(stringify)?;
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM goal_links
                 WHERE goal_id = ?1 AND target_kind = ?2 AND target_id = ?3",
                params![goal_id, target_kind.as_str(), target_id],
                |row| row.get(0),
            )
            .map_err(stringify)?;
        Ok(count > 0)
    }
}

fn read_goal(row: &Row<'_>) -> Result<Goal, String> {
    let status: String = row.get(4).map_err(stringify)?;
    Ok(Goal {
        id: row.get(0).map_err(stringify)?,
        title: row.get(1).map_err(stringify)?,
        description: row.get(2).map_err(stringify)?,
        acceptance_notes: row.get(3).map_err(stringify)?,
        status: GoalStatus::parse(&status).map_err(|error| error.to_string())?,
        project_path: row.get(5).map_err(stringify)?,
        created_at: row.get(6).map_err(stringify)?,
        updated_at: row.get(7).map_err(stringify)?,
    })
}

fn stringify(value: impl std::fmt::Display) -> String {
    value.to_string()
}
