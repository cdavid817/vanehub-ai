use super::{
    DirectoryListing, DocumentListing, FileContent, GitDiffResult, GitDiffSource, GitStatusResult,
    SessionLogExportResult, SessionLogPage, SessionLogQuery, WorkspaceApplicationError,
    WorkspaceSessionQueryPort,
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

    pub(crate) fn list_directory(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<DirectoryListing, WorkspaceApplicationError> {
        self.queries.list_directory(session_id, path)
    }

    pub(crate) fn list_documents(
        &self,
        session_id: &str,
    ) -> Result<DocumentListing, WorkspaceApplicationError> {
        self.queries.list_documents(session_id)
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
