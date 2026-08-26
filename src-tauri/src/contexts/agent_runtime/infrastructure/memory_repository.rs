use super::memory_naming::{derive_description, derive_name};
#[cfg(test)]
use crate::contexts::agent_runtime::application::SaveMemoryInput;
use crate::contexts::agent_runtime::application::{
    AgentMemory, AgentMemoryPort, AgentRuntimeApplicationError, MemorySource,
};
#[cfg(test)]
use crate::platform::clock::SystemClock;
use crate::platform::database::NativeDatabase;
#[cfg(test)]
use rusqlite::params;
use rusqlite::Row;
use std::collections::HashSet;
#[cfg(test)]
#[cfg(test)]
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
}

/// The legacy row write, kept for this repository's own tests.
///
/// `name`, `description` and `type` have no columns here and are deliberately dropped. Nothing in
/// production writes rows any more — this repository exists as the migration source.
#[cfg(test)]
impl SqliteAgentMemoryRepository {
    pub(crate) fn save(
        &self,
        input: SaveMemoryInput<'_>,
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
                    input.agent_id,
                    input.folder.unwrap_or_default(),
                    input.content,
                    input.source.as_str(),
                    now,
                ],
            )
            .map_err(repository_error)?;
        Ok(())
    }
}

impl AgentMemoryPort for SqliteAgentMemoryRepository {
    fn list_all(&self) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError> {
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
        // A row carries no name, description, or type. They are derived here so the shared
        // `AgentMemory` shape stays uniform across both stores; the values a migrated row ends up
        // with come from `migrate_memory_rows`, which derives them the same way.
        let description = derive_description(&self.content).unwrap_or_else(|| self.id.clone());
        let name = derive_name(&self.content, &self.id, &HashSet::new());
        Ok(AgentMemory {
            id: self.id,
            agent_id: self.agent_id,
            folder: (!self.folder.is_empty()).then_some(self.folder),
            name,
            description,
            memory_type: None,
            content: self.content,
            source,
            created_at: self.created_at,
            modified_at: None,
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
    fn save_records_provenance_and_list_all_returns_it() {
        let fixture = Fixture::new("agent memory save and list", &["my-agent"]);
        fixture
            .repository
            .save(SaveMemoryInput::derived(
                "my-agent",
                Some("D:/project"),
                "Uses pnpm.",
                MemorySource::Explicit,
            ))
            .expect("save");

        let memories = fixture.repository.list_all().expect("list");

        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].agent_id, "my-agent");
        assert_eq!(memories[0].folder.as_deref(), Some("D:/project"));
        assert_eq!(memories[0].content, "Uses pnpm.");
        assert_eq!(memories[0].source, MemorySource::Explicit);
    }

    /// `add-cli-memory-support`: memories are a shared host-level pool — unlike the previous
    /// per-agent-scoped behavior, `list_all` SHALL return memories from every agent and every
    /// folder together, not just the one that produced them.
    #[test]
    fn list_all_spans_every_agent_and_every_folder() {
        let fixture = Fixture::new("agent memory shared pool", &["my-agent", "other-agent"]);
        fixture
            .repository
            .save(SaveMemoryInput::derived(
                "my-agent",
                Some("D:/project-a"),
                "A fact.",
                MemorySource::Explicit,
            ))
            .expect("save a");
        fixture
            .repository
            .save(SaveMemoryInput::derived(
                "my-agent",
                None,
                "Global fact.",
                MemorySource::Automatic,
            ))
            .expect("save global");
        fixture
            .repository
            .save(SaveMemoryInput::derived(
                "other-agent",
                Some("D:/project-b"),
                "Other agent's fact.",
                MemorySource::Automatic,
            ))
            .expect("save other agent");

        let memories = fixture.repository.list_all().expect("list all");

        assert_eq!(memories.len(), 3);
        assert!(memories
            .iter()
            .any(|memory| memory.agent_id == "other-agent"
                && memory.content == "Other agent's fact."));
    }

    #[test]
    fn folder_is_recorded_as_provenance_without_restricting_list_all() {
        let fixture = Fixture::new("agent memory global bucket", &["my-agent"]);
        fixture
            .repository
            .save(SaveMemoryInput::derived(
                "my-agent",
                None,
                "Global fact.",
                MemorySource::Automatic,
            ))
            .expect("save");

        let memories = fixture.repository.list_all().expect("list");

        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].folder, None);
        assert_eq!(memories[0].source, MemorySource::Automatic);
    }

    #[test]
    fn list_all_spans_every_agent_not_just_one() {
        // The retrieval index reconciler treats anything missing from this snapshot as deleted,
        // so scoping it to a single Agent would silently drop every other Agent's index rows.
        let fixture = Fixture::new("agent memory list all agents", &["agent-a", "agent-b"]);
        fixture
            .repository
            .save(SaveMemoryInput::derived(
                "agent-a",
                Some("D:/project"),
                "A fact.",
                MemorySource::Explicit,
            ))
            .expect("save a");
        fixture
            .repository
            .save(SaveMemoryInput::derived(
                "agent-b",
                None,
                "B fact.",
                MemorySource::Automatic,
            ))
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
}
