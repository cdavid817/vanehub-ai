use crate::contexts::retrieval::application::{IndexSourcePort, IndexSourceRecord};
use crate::contexts::retrieval::domain::RetrievalError;
use crate::platform::database::{DatabaseError, NativeDatabase};

pub(crate) struct WorkspaceFileIndexSource {
    database: NativeDatabase,
    workspace_id: String,
}

impl WorkspaceFileIndexSource {
    pub(crate) fn new(database: NativeDatabase, workspace_id: String) -> Self {
        Self {
            database,
            workspace_id,
        }
    }
}

impl IndexSourcePort for WorkspaceFileIndexSource {
    fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT source_id, content, created_at
                FROM retrieval_documents
                WHERE source_kind = 'workspace_file' AND scope_folder = ?1
                ORDER BY source_id
                "#,
            )
            .map_err(storage_error)?;
        let records = statement
            .query_map([&self.workspace_id], |row| {
                Ok(IndexSourceRecord {
                    source_id: row.get(0)?,
                    agent_id: String::new(),
                    folder: self.workspace_id.clone(),
                    content: row.get(1)?,
                    created_at: row.get(2)?,
                    // Workspace code is admitted for embedding by the explicit per-workspace
                    // confirmation, which the generation guard checks; no per-record restriction.
                    egress_restricted: false,
                })
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(records)
    }
}

fn database_error(error: DatabaseError) -> RetrievalError {
    match error {
        DatabaseError::Database(error) => storage_error(error),
        DatabaseError::Storage(message) => RetrievalError::Storage(message),
    }
}

fn storage_error(error: rusqlite::Error) -> RetrievalError {
    RetrievalError::Storage(error.to_string())
}
