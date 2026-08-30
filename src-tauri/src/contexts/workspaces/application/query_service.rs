use super::inspection_execution::WorkspaceInspectionExecution;
use super::{
    DirectoryListing, DocumentListing, FileContent, FileSearchListing, GitDiffResult,
    GitDiffSource, GitStatusResult, SessionLogExportResult, SessionLogPage, SessionLogQuery,
    WorkspaceApplicationError, WorkspaceSessionQueryPort,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct WorkspaceQueryApplicationService {
    queries: Arc<dyn WorkspaceSessionQueryPort>,
}

impl WorkspaceQueryApplicationService {
    pub(crate) fn new(queries: Arc<dyn WorkspaceSessionQueryPort>) -> Self {
        Self { queries }
    }

    pub(crate) fn resolve_session_root(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, WorkspaceApplicationError> {
        self.queries.resolve_session_root(session_id)
    }

    pub(crate) fn resolve_session_directory(
        &self,
        session_id: &str,
        relative: &str,
    ) -> Result<Option<String>, WorkspaceApplicationError> {
        self.queries.resolve_session_directory(session_id, relative)
    }

    pub(crate) fn list_directory(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<DirectoryListing, WorkspaceApplicationError> {
        self.queries.list_directory(session_id, path)
    }

    /// One page of a directory, resuming after a cursor.
    ///
    /// The unpaged call above is this with no cursor and the default bound. Kept separate at this
    /// seam rather than folded into one method with two optional arguments, because "give me this
    /// folder" and "give me what comes after this position in this folder" fail differently: only
    /// the second can be refused for a reason that has nothing to do with the folder.
    pub(crate) fn list_directory_page(
        &self,
        session_id: &str,
        path: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<DirectoryListing, WorkspaceApplicationError> {
        self.queries
            .list_directory_page(session_id, path, cursor, limit)
    }

    pub(crate) fn list_documents(
        &self,
        session_id: &str,
        execution: &WorkspaceInspectionExecution,
    ) -> Result<DocumentListing, WorkspaceApplicationError> {
        self.queries.list_documents(session_id, execution)
    }

    pub(crate) fn search_files(
        &self,
        session_id: &str,
        query: &str,
        max_results: usize,
    ) -> Result<FileSearchListing, WorkspaceApplicationError> {
        self.queries.search_files(session_id, query, max_results)
    }

    pub(crate) fn read_file(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<FileContent, WorkspaceApplicationError> {
        self.queries.read_file(session_id, path)
    }

    pub(crate) fn read_text_file(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<FileContent, WorkspaceApplicationError> {
        self.queries.read_text_file(session_id, path)
    }

    pub(crate) fn git_status(
        &self,
        session_id: &str,
    ) -> Result<GitStatusResult, WorkspaceApplicationError> {
        self.queries.git_status(session_id)
    }

    pub(crate) fn git_diff(
        &self,
        session_id: &str,
        path: &str,
        source: GitDiffSource,
    ) -> Result<GitDiffResult, WorkspaceApplicationError> {
        self.queries.git_diff(session_id, path, source)
    }

    pub(crate) fn list_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogPage, WorkspaceApplicationError> {
        self.queries.list_logs(query)
    }

    pub(crate) fn export_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogExportResult, WorkspaceApplicationError> {
        self.queries.export_logs(query)
    }
}
