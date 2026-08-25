//! Choosing which provider answers, and refusing to guess.
//!
//! Selection has one home so there is one place to check the rule that matters: the provider comes
//! from the target, the target comes from the resolver, and the resolver's only input is a session
//! id. A caller cannot pass a root, cannot pass a provider, and cannot ask a local provider about a
//! remote session — not because each call site remembers to, but because no call site is given the
//! opportunity.
//!
//! A missing remote provider is `unsupported`, never a fallback to local. Falling back would show
//! this machine's files under a remote host's name, which is worse than showing nothing: the files
//! would be real, the paths would look plausible, and nothing on screen would say which computer
//! they came from.

use super::inspection::{
    GitDiffRequest, ListDirectoryRequest, ReadTextFileRequest, WorkspaceInspectionCapabilities,
    WorkspaceInspectionError, WorkspaceInspectionProvider, WorkspaceSearchRequest, WorkspaceTarget,
    WorkspaceTargetResolver,
};
use super::models::{
    DirectoryListing, DocumentListing, FileContent, FileSearchListing, GitDiffResult,
    GitStatusResult,
};
use std::sync::Arc;

pub(crate) struct WorkspaceInspectionRouter {
    resolver: Arc<dyn WorkspaceTargetResolver>,
    local: Arc<dyn WorkspaceInspectionProvider>,
    /// Absent until the SSH provider exists. `None` is a state the router reports rather than one it
    /// works around, because the alternative is answering a remote question with a local answer.
    remote: Option<Arc<dyn WorkspaceInspectionProvider>>,
}

impl WorkspaceInspectionRouter {
    pub(crate) fn new(
        resolver: Arc<dyn WorkspaceTargetResolver>,
        local: Arc<dyn WorkspaceInspectionProvider>,
    ) -> Self {
        Self {
            resolver,
            local,
            remote: None,
        }
    }

    pub(crate) fn with_remote(mut self, remote: Arc<dyn WorkspaceInspectionProvider>) -> Self {
        self.remote = Some(remote);
        self
    }

    /// The target for a session, from the registered binding and nothing else.
    pub(crate) fn target(
        &self,
        session_id: &str,
    ) -> Result<WorkspaceTarget, WorkspaceInspectionError> {
        self.resolver.resolve(session_id)
    }

    fn provider(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<&Arc<dyn WorkspaceInspectionProvider>, WorkspaceInspectionError> {
        match target {
            WorkspaceTarget::Local(_) => Ok(&self.local),
            WorkspaceTarget::Remote(_) => {
                self.remote
                    .as_ref()
                    .ok_or(WorkspaceInspectionError::Unsupported(
                        "workspace_remote_inspection_unavailable",
                    ))
            }
        }
    }

    pub(crate) async fn capabilities(
        &self,
        session_id: &str,
    ) -> Result<WorkspaceInspectionCapabilities, WorkspaceInspectionError> {
        let target = self.target(session_id)?;
        self.provider(&target)?.capabilities(&target).await
    }
}

/// The reads the commands switch to in Task Group 12.
///
/// Written now because they are the shape the port already declares, and a router that answered
/// only the capability question would leave the selection rule proved for one call and assumed for
/// six. `expect` rather than `allow` so the attribute fails the moment it stops being true.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "consumed by the inspection commands in 12.x; remove this attribute there"
    )
)]
impl WorkspaceInspectionRouter {
    pub(crate) async fn list_directory(
        &self,
        session_id: &str,
        request: ListDirectoryRequest,
    ) -> Result<DirectoryListing, WorkspaceInspectionError> {
        let target = self.target(session_id)?;
        self.provider(&target)?
            .list_directory(&target, request)
            .await
    }

    pub(crate) async fn list_documents(
        &self,
        session_id: &str,
    ) -> Result<DocumentListing, WorkspaceInspectionError> {
        let target = self.target(session_id)?;
        self.provider(&target)?.list_documents(&target).await
    }

    pub(crate) async fn read_text_file(
        &self,
        session_id: &str,
        request: ReadTextFileRequest,
    ) -> Result<FileContent, WorkspaceInspectionError> {
        let target = self.target(session_id)?;
        self.provider(&target)?
            .read_text_file(&target, request)
            .await
    }

    pub(crate) async fn search(
        &self,
        session_id: &str,
        request: WorkspaceSearchRequest,
    ) -> Result<FileSearchListing, WorkspaceInspectionError> {
        let target = self.target(session_id)?;
        self.provider(&target)?.search(&target, request).await
    }

    pub(crate) async fn git_status(
        &self,
        session_id: &str,
    ) -> Result<GitStatusResult, WorkspaceInspectionError> {
        let target = self.target(session_id)?;
        self.provider(&target)?.git_status(&target).await
    }

    pub(crate) async fn git_diff(
        &self,
        session_id: &str,
        request: GitDiffRequest,
    ) -> Result<GitDiffResult, WorkspaceInspectionError> {
        let target = self.target(session_id)?;
        self.provider(&target)?.git_diff(&target, request).await
    }
}
