use crate::contexts::agent_runtime::application::{
    AgentMemory, AgentMemoryPort, AgentRuntimeApplicationError, MemorySource,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Row};
use uuid::Uuid;

/// SQLite-backed `AgentMemoryPort` (`add-agent-cross-session-memory`). Unlike Skill bindings
/// (Phase 4), `agent_runtime` owns this concept outright, so this is a direct implementation, not
/// a cross-context adapter wrapping another context's facade.
#[derive(Clone)]
pub(crate) struct SqliteAgentMemoryRepository {
    database: NativeDatabase,
}

impl SqliteAgentMemoryRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    /// Every memory of every Agent, in one snapshot. Deliberately not on `AgentMemoryPort`: the
    /// only caller is the composition root's retrieval index source, and no use case inside this
    /// context ever wants a cross-Agent view.
    ///
    /// The snapshot has to be global rather than per-Agent because its consumer reconciles it
    /// against the whole index and deletes rows it cannot find here — a partial snapshot would
    /// look like every absent Agent's memories had been deleted.
    pub(crate) fn list_all(&self) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT id, agent_id, folder, content, source, created_at
                FROM agent_memories
                ORDER BY created_at DESC
                "#,
            )
            .map_err(repository_error)?;
        let rows = statement
            .query_map([], MemoryRow::read)
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        rows.into_iter().map(MemoryRow::into_memory).collect()
    }

    /// The memories behind an explicit id list, in no particular order. Same caller as `list_all`
    /// and deliberately also off `AgentMemoryPort` for the same reason: it crosses Agent
    /// boundaries, which no use case inside this context ever wants.
    ///
    /// Exists so the retrieval *search* path does not have to reuse `list_all`: a recall resolves
    /// at most `limit` (<= 20) ids, while `list_all` loads and clones every memory of every Agent
    /// synchronously inside the generation's tool call, and `agent_memories` is INSERT-only with
    /// no cap.
    pub(crate) fn list_by_ids(
        &self,
        ids: &[String],
    ) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let connection = self.database.connection().map_err(app_error)?;
        // Only the *placeholders* are built from the id count; every id itself is bound, so no
        // caller-supplied text ever reaches the SQL text. `ids` comes from the retrieval index and
        // is bounded by the recall limit, so the SQLite variable ceiling is not in reach.
        let placeholders = std::iter::repeat_n("?", ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let mut statement = connection
            .prepare(&format!(
                r#"
                SELECT id, agent_id, folder, content, source, created_at
                FROM agent_memories
                WHERE id IN ({placeholders})
                "#
            ))
            .map_err(repository_error)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(ids.iter()), MemoryRow::read)
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        rows.into_iter().map(MemoryRow::into_memory).collect()
    }
}

impl AgentMemoryPort for SqliteAgentMemoryRepository {
    fn save(
        &self,
        agent_id: &str,
        folder: Option<&str>,
        content: &str,
        source: MemorySource,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let now = SystemClock.rfc3339();
        connection
            .execute(
                r#"
                INSERT INTO agent_memories
                (id, agent_id, folder, content, source, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
                "#,
                params![
                    Uuid::new_v4().to_string(),
                    agent_id,
                    folder.unwrap_or_default(),
                    content,
                    source.as_str(),
                    now,
                ],
            )
            .map_err(repository_error)?;
        Ok(())
    }

    fn list(
        &self,
        agent_id: &str,
        folder: Option<&str>,
    ) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT id, agent_id, folder, content, source, created_at
                FROM agent_memories
                WHERE agent_id = ?1 AND folder = ?2
                ORDER BY created_at DESC
                "#,
            )
            .map_err(repository_error)?;
        let rows = statement
            .query_map(
                params![agent_id, folder.unwrap_or_default()],
                MemoryRow::read,
            )
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        rows.into_iter().map(MemoryRow::into_memory).collect()
    }

    fn list_all_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT id, agent_id, folder, content, source, created_at
                FROM agent_memories
                WHERE agent_id = ?1
                ORDER BY created_at DESC
                "#,
            )
            .map_err(repository_error)?;
        let rows = statement
            .query_map(params![agent_id], MemoryRow::read)
            .map_err(repository_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(repository_error)?;
        rows.into_iter().map(MemoryRow::into_memory).collect()
    }

    fn delete(&self, memory_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        let connection = self.database.connection().map_err(app_error)?;
        connection
            .execute(
                "DELETE FROM agent_memories WHERE id = ?1",
                params![memory_id],
            )
            .map_err(repository_error)?;
        Ok(())
    }
}

struct MemoryRow {
    id: String,
    agent_id: String,
    folder: String,
    content: String,
    source: String,
    created_at: String,
}

impl MemoryRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            agent_id: row.get(1)?,
            folder: row.get(2)?,
            content: row.get(3)?,
            source: row.get(4)?,
            created_at: row.get(5)?,
        })
    }

    fn into_memory(self) -> Result<AgentMemory, AgentRuntimeApplicationError> {
        let source = MemorySource::parse(&self.source).ok_or_else(|| {
            AgentRuntimeApplicationError::Memory(format!(
                "Invalid persisted memory source: {}",
                self.source
            ))
        })?;
        Ok(AgentMemory {
            id: self.id,
            agent_id: self.agent_id,
            folder: (!self.folder.is_empty()).then_some(self.folder),
            content: self.content,
            source,
            created_at: self.created_at,
        })
    }
}

fn app_error(error: crate::platform::database::DatabaseError) -> AgentRuntimeApplicationError {
    match error {
        crate::platform::database::DatabaseError::Database(error) => repository_error(error),
        crate::platform::database::DatabaseError::Storage(message) => {
            AgentRuntimeApplicationError::Memory(message)
        }
    }
}

fn repository_error(error: rusqlite::Error) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Memory(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    struct Fixture {
        _directory: TempDirectory,
        repository: SqliteAgentMemoryRepository,
    }

    impl Fixture {
        /// `agent_memories.agent_id` has a real, enforced `FOREIGN KEY REFERENCES agents(id)`
        /// (confirmed by this test failing with "FOREIGN KEY constraint failed" before this
        /// helper existed) — every agent id a test saves a memory against must first exist here.
        fn new(label: &str, agent_ids: &[&str]) -> Self {
            let directory = TempDirectory::new(label);
            let database =
                NativeDatabase::new(directory.path().to_path_buf()).expect("test database");
            let connection = database.connection().expect("migrated database");
            for agent_id in agent_ids {
                connection
                    .execute(
                        "INSERT INTO agents (id, display_name, provider, launch_kind) VALUES (?1, ?1, 'Test', 'api')",
                        params![agent_id],
                    )
                    .expect("seed agent");
            }
            drop(connection);
            Self {
                repository: SqliteAgentMemoryRepository::new(database),
                _directory: directory,
            }
        }
    }

    #[test]
    fn save_and_list_round_trip_within_a_scope() {
        let fixture = Fixture::new("agent memory save and list", &["my-agent"]);
        fixture
            .repository
            .save(
                "my-agent",
                Some("D:/project"),
                "Uses pnpm.",
                MemorySource::Explicit,
            )
            .expect("save");

        let memories = fixture
            .repository
            .list("my-agent", Some("D:/project"))
            .expect("list");

        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].agent_id, "my-agent");
        assert_eq!(memories[0].folder.as_deref(), Some("D:/project"));
        assert_eq!(memories[0].content, "Uses pnpm.");
        assert_eq!(memories[0].source, MemorySource::Explicit);
    }

    #[test]
    fn list_excludes_other_agents_and_other_folders() {
        let fixture = Fixture::new("agent memory scope isolation", &["my-agent", "other-agent"]);
        fixture
            .repository
            .save(
                "my-agent",
                Some("D:/project-a"),
                "A fact.",
                MemorySource::Explicit,
            )
            .expect("save a");
        fixture
            .repository
            .save(
                "my-agent",
                Some("D:/project-b"),
                "B fact.",
                MemorySource::Explicit,
            )
            .expect("save b");
        fixture
            .repository
            .save(
                "other-agent",
                Some("D:/project-a"),
                "Not mine.",
                MemorySource::Explicit,
            )
            .expect("save other agent");

        let memories = fixture
            .repository
            .list("my-agent", Some("D:/project-a"))
            .expect("list");

        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].content, "A fact.");
    }

    #[test]
    fn folder_less_sessions_use_the_agent_global_bucket() {
        let fixture = Fixture::new("agent memory global bucket", &["my-agent"]);
        fixture
            .repository
            .save("my-agent", None, "Global fact.", MemorySource::Automatic)
            .expect("save");

        let memories = fixture.repository.list("my-agent", None).expect("list");

        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].folder, None);
        assert_eq!(memories[0].source, MemorySource::Automatic);
    }

    #[test]
    fn list_all_for_agent_spans_every_folder() {
        let fixture = Fixture::new("agent memory list all", &["my-agent"]);
        fixture
            .repository
            .save(
                "my-agent",
                Some("D:/project-a"),
                "A fact.",
                MemorySource::Explicit,
            )
            .expect("save a");
        fixture
            .repository
            .save("my-agent", None, "Global fact.", MemorySource::Automatic)
            .expect("save global");

        let memories = fixture
            .repository
            .list_all_for_agent("my-agent")
            .expect("list all");

        assert_eq!(memories.len(), 2);
    }

    #[test]
    fn list_all_spans_every_agent_not_just_one() {
        // The retrieval index reconciler treats anything missing from this snapshot as deleted,
        // so scoping it to a single Agent would silently drop every other Agent's index rows.
        let fixture = Fixture::new("agent memory list all agents", &["agent-a", "agent-b"]);
        fixture
            .repository
            .save(
                "agent-a",
                Some("D:/project"),
                "A fact.",
                MemorySource::Explicit,
            )
            .expect("save a");
        fixture
            .repository
            .save("agent-b", None, "B fact.", MemorySource::Automatic)
            .expect("save b");

        let memories = fixture.repository.list_all().expect("list all");

        let mut contents = memories
            .iter()
            .map(|memory| memory.content.as_str())
            .collect::<Vec<_>>();
        contents.sort_unstable();
        assert_eq!(contents, vec!["A fact.", "B fact."]);
        assert!(memories.iter().any(|memory| memory.agent_id == "agent-b"));
    }

    #[test]
    fn list_by_ids_returns_only_the_requested_memories_and_tolerates_missing_ids() {
        // The retrieval search path resolves at most `limit` (<= 20) ids per recall. Falling back
        // to `list_all` there loads and clones every memory of every Agent synchronously inside
        // the generation's tool call, on an INSERT-only table with no cap.
        let fixture = Fixture::new("agent memory list by ids", &["agent-a", "agent-b"]);
        for (agent, content) in [
            ("agent-a", "A fact."),
            ("agent-a", "Another fact."),
            ("agent-b", "B fact."),
        ] {
            fixture
                .repository
                .save(agent, None, content, MemorySource::Explicit)
                .expect("save");
        }
        let all = fixture.repository.list_all().expect("list all");
        let wanted = all
            .iter()
            .filter(|memory| memory.content != "Another fact.")
            .map(|memory| memory.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(wanted.len(), 2, "the fixture must span both agents");

        let mut ids = wanted.clone();
        // An id whose source row is gone must simply be absent, not an error: the retrieval index
        // can outlive a deleted memory, and skipping it is what keeps deleted memories from ever
        // being surfaced again.
        ids.push("this-memory-was-deleted".to_string());
        let memories = fixture.repository.list_by_ids(&ids).expect("list by ids");

        let mut contents = memories
            .iter()
            .map(|memory| memory.content.as_str())
            .collect::<Vec<_>>();
        contents.sort_unstable();
        assert_eq!(contents, vec!["A fact.", "B fact."]);
    }

    #[test]
    fn list_by_ids_short_circuits_on_an_empty_id_list() {
        // An empty `IN ()` is a SQL syntax error, and a recall that fused nothing is an ordinary
        // "no hits" result rather than a failure — it must not become one here.
        let fixture = Fixture::new("agent memory list by no ids", &["agent-a"]);
        fixture
            .repository
            .save("agent-a", None, "A fact.", MemorySource::Explicit)
            .expect("save");

        assert!(fixture
            .repository
            .list_by_ids(&[])
            .expect("an empty id list is not an error")
            .is_empty());
    }

    #[test]
    fn delete_removes_exactly_the_targeted_memory() {
        let fixture = Fixture::new("agent memory delete", &["my-agent"]);
        fixture
            .repository
            .save("my-agent", None, "Keep me.", MemorySource::Explicit)
            .expect("save keep");
        fixture
            .repository
            .save("my-agent", None, "Delete me.", MemorySource::Explicit)
            .expect("save delete");
        let before = fixture
            .repository
            .list("my-agent", None)
            .expect("list before");
        let target = before
            .iter()
            .find(|memory| memory.content == "Delete me.")
            .expect("target memory");

        fixture.repository.delete(&target.id).expect("delete");

        let after = fixture
            .repository
            .list("my-agent", None)
            .expect("list after");
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].content, "Keep me.");
    }
}
