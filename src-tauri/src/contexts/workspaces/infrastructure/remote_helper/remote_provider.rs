//! The remote half of provider-neutral inspection.
//!
//! Every operation is one helper round trip and one mapping. There is no logic here beyond that,
//! and deliberately so: the confinement is on the remote host because only that machine can tell a
//! symlink from a directory, and the Git parsing is in the local provider's parser because a second
//! implementation would disagree first about exactly the cases nobody writes tests for.
//!
//! Retries are safe because every operation here is a read. Nothing in this file mutates a
//! workspace, sends a Shell command, or writes a file — so re-issuing one after a dropped
//! connection repeats an observation rather than an action.

use super::probe::{capabilities_from, revalidate};
use super::protocol::{
    HelperEntry, HelperFile, HelperGitOutput, HelperListing, HelperOperation, HelperRequest,
    HelperResult, HelperSearch, RemoteHelperError,
};
use super::transport::{exchange, RemoteHelperSession};
use crate::contexts::workspaces::application::{
    DirectoryEntry, DirectoryListing, DocumentListing, FileContent, FileSearchListing,
    FileSearchMatch, GitDiffRequest, GitDiffResult, GitDiffSource, GitStatusResult,
    ListDirectoryRequest, ReadTextFileRequest, RemoteWorkspaceTarget, SessionWorkspaceContext,
    WorkspaceInspectionCapabilities, WorkspaceInspectionError, WorkspaceInspectionProvider,
    WorkspaceSearchRequest, WorkspaceTarget,
};
use async_trait::async_trait;
use base64::Engine;
use std::sync::Arc;

/// What a binding looks like right now.
///
/// A two-value question rather than the connections API, for the same reason `revalidate` takes
/// scalars: the provider needs to know whether the profile moved and whether the host is still
/// trusted, and nothing else about connection management. Taking the whole API would make every
/// test of this provider need a connection pool.
pub(crate) trait RemoteProfileSource: Send + Sync {
    /// The current revision and host trust, or the reason neither could be read.
    fn current(&self, connection_id: &str) -> Result<(i64, bool), RemoteHelperError>;
}

pub(crate) struct RemoteWorkspaceInspectionProvider {
    profiles: Arc<dyn RemoteProfileSource>,
    session: Arc<dyn RemoteHelperSession>,
}

impl RemoteWorkspaceInspectionProvider {
    pub(crate) fn new(
        profiles: Arc<dyn RemoteProfileSource>,
        session: Arc<dyn RemoteHelperSession>,
    ) -> Self {
        Self { profiles, session }
    }

    /// The target, after the binding has been checked against what is registered now.
    ///
    /// Every operation goes through this. A profile can be edited or a host untrusted between two
    /// reads, and the check belongs before the connection rather than after it: reconnecting under
    /// a revision the session was not bound to would answer about a different machine.
    fn remote<'target>(
        &self,
        target: &'target WorkspaceTarget,
    ) -> Result<&'target RemoteWorkspaceTarget, WorkspaceInspectionError> {
        let remote = match target {
            WorkspaceTarget::Remote(remote) => remote,
            // A local target reaching the remote provider is a routing bug, and answering it by
            // connecting somewhere would turn that bug into a wrong answer.
            WorkspaceTarget::Local(_) => {
                return Err(WorkspaceInspectionError::Unsupported(
                    "workspace_provider_remote_only",
                ))
            }
        };
        let (revision, host_trusted) = self
            .profiles
            .current(&remote.connection_id)
            .map_err(inspection_error)?;
        revalidate(remote.connection_revision, revision, host_trusted).map_err(inspection_error)?;
        Ok(remote)
    }

    /// One operation, retried once if the connection dropped.
    ///
    /// Safe to retry because every operation here is a read: re-issuing one repeats an
    /// observation rather than an action. Nothing in this provider mutates a workspace, writes a
    /// file, or sends a Shell command, and a test asserts the absence of every verb that would.
    ///
    /// The binding is revalidated *before each attempt* rather than once at the top. A retry
    /// happens after a failure, and the interesting failure is a connection that dropped because
    /// the profile was edited — reconnecting under the old revision would answer about a machine
    /// the session is no longer bound to.
    async fn call(
        &self,
        target: &WorkspaceTarget,
        operation: HelperOperation,
    ) -> Result<(RemoteWorkspaceTarget, HelperResult), WorkspaceInspectionError> {
        let mut attempt = 0;
        loop {
            attempt += 1;
            let remote = self.remote(target)?.clone();
            let outcome = exchange(
                self.session.as_ref(),
                &remote.connection_id,
                remote.connection_revision,
                &HelperRequest::new(remote.root.clone(), operation.clone()),
            )
            .await;

            match outcome {
                Ok(response) => {
                    let result =
                        response
                            .result
                            .ok_or(WorkspaceInspectionError::RemoteUnavailable(
                                "remote_helper_malformed_response",
                            ))?;
                    return Ok((remote, result));
                }
                Err(error) if attempt < MAX_INSPECTION_ATTEMPTS && is_retryable(&error) => {
                    continue;
                }
                Err(error) => return Err(inspection_error(error)),
            }
        }
    }
}

/// How many times one read is attempted.
///
/// Two, not more. A second attempt covers the ordinary case — a pooled connection that died
/// between requests — and a third would mostly add latency to a host that is genuinely down,
/// while a panel that opens six operations at once turns every extra attempt into six.
const MAX_INSPECTION_ATTEMPTS: usize = 2;

/// Whether a failure is one a second attempt could plausibly get past.
///
/// A dropped connection or channel is: the pool reconnects and the read runs again. A timeout is
/// deliberately *not* — the remote may still be executing the first request, and a retry would
/// leave two helper processes running while the reader waits twice as long for the same answer.
/// Everything else is an answer rather than a failure to reach one, and repeating it would produce
/// the same answer more slowly.
fn is_retryable(error: &RemoteHelperError) -> bool {
    matches!(
        error,
        RemoteHelperError::ConnectionFailed | RemoteHelperError::ChannelFailed
    )
}

/// The helper's vocabulary, in the inspection's terms.
///
/// The two path refusals keep their own meaning rather than collapsing into "remote unavailable":
/// a path that escaped its root is a refusal a reader must not retry, and a missing one is a fact
/// about the workspace.
fn inspection_error(error: RemoteHelperError) -> WorkspaceInspectionError {
    match error {
        RemoteHelperError::Timeout => WorkspaceInspectionError::Timeout,
        RemoteHelperError::Refused(code) if code == "workspace_path_escaped" => {
            WorkspaceInspectionError::PathEscaped
        }
        RemoteHelperError::Refused(code) if code == "workspace_path_not_found" => {
            WorkspaceInspectionError::NotFound
        }
        RemoteHelperError::Refused(code) => match remote_reason(&code) {
            Some(reason) => WorkspaceInspectionError::Unsupported(reason),
            None => WorkspaceInspectionError::RemoteUnavailable("remote_helper_refused"),
        },
        other => WorkspaceInspectionError::RemoteUnavailable(match other.code() {
            "remote_profile_stale" => "remote_profile_stale",
            "remote_host_untrusted" => "remote_host_untrusted",
            "remote_helper_timeout" => "remote_helper_timeout",
            _ => "remote_connection_unavailable",
        }),
    }
}

/// The helper's missing-prerequisite codes, kept as themselves.
///
/// A closed list because these become `unsupported` rather than `unavailable`, and the difference
/// is what a panel shows: one says "install this", the other says "try again".
fn remote_reason(code: &str) -> Option<&'static str> {
    match code {
        "remote_ripgrep_missing" => Some("remote_ripgrep_missing"),
        "remote_git_missing" => Some("remote_git_missing"),
        "remote_helper_unsupported_operation" => Some("remote_helper_unsupported_operation"),
        _ => None,
    }
}

fn context(remote: &RemoteWorkspaceTarget) -> SessionWorkspaceContext {
    // The display name, never the remote root: an absolute path on somebody else's machine is not
    // something this UI should be showing, and it is not what identifies the workspace to a reader.
    SessionWorkspaceContext::available(Some(remote.display_name.clone()))
}

fn entry(value: HelperEntry) -> DirectoryEntry {
    DirectoryEntry {
        name: value.name,
        path: value.path,
        // Mapped to the two the model has. Anything the helper did not classify as a directory is a
        // file as far as a panel is concerned, and the helper already skipped what is neither.
        kind: if value.kind == "directory" {
            "directory"
        } else {
            "file"
        },
        size: value.size,
    }
}

fn listing(remote: &RemoteWorkspaceTarget, value: HelperListing) -> DirectoryListing {
    DirectoryListing {
        context: context(remote),
        path: value.path,
        items: value.entries.into_iter().map(entry).collect(),
        truncated: value.truncated,
        next_cursor: None,
    }
}

fn file(value: HelperFile) -> FileContent {
    FileContent {
        path: value.path,
        name: value.name,
        // The three the model knows. An unrecognised status becomes `binary`, which withholds a
        // preview — the safe direction, because the alternative shows bytes as text.
        status: match value.status.as_str() {
            "text" => "text",
            "oversized" => "oversized",
            _ => "binary",
        },
        size: value.size,
        content: value.content,
    }
}

fn search(remote: &RemoteWorkspaceTarget, value: HelperSearch) -> FileSearchListing {
    FileSearchListing {
        context: context(remote),
        items: value
            .matches
            .into_iter()
            .map(|item| FileSearchMatch {
                name: item.name,
                path: item.path,
            })
            .collect(),
        truncated: value.truncated,
    }
}

/// The bytes git printed, or the reason there are none.
fn git_output(value: &HelperGitOutput) -> Result<Vec<u8>, WorkspaceInspectionError> {
    let Some(encoded) = value.stdout_base64.as_deref() else {
        return Ok(Vec::new());
    };
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| {
            WorkspaceInspectionError::RemoteUnavailable("remote_helper_malformed_response")
        })
}

#[async_trait]
impl WorkspaceInspectionProvider for RemoteWorkspaceInspectionProvider {
    async fn capabilities(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<WorkspaceInspectionCapabilities, WorkspaceInspectionError> {
        let (_, result) = self.call(target, HelperOperation::Probe).await?;
        let probe = result
            .probe
            .ok_or(WorkspaceInspectionError::RemoteUnavailable(
                "remote_helper_malformed_response",
            ))?;
        Ok(capabilities_from(&probe))
    }

    async fn list_directory(
        &self,
        target: &WorkspaceTarget,
        request: ListDirectoryRequest,
    ) -> Result<DirectoryListing, WorkspaceInspectionError> {
        let (remote, result) = self
            .call(
                target,
                HelperOperation::ListDirectory { path: request.path },
            )
            .await?;
        result.listing.map(|value| listing(&remote, value)).ok_or(
            WorkspaceInspectionError::RemoteUnavailable("remote_helper_malformed_response"),
        )
    }

    async fn list_documents(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<DocumentListing, WorkspaceInspectionError> {
        // Not offered yet, and refused rather than answered with an empty list: an empty document
        // list is a claim that the workspace has no documents, which is a different statement from
        // "this build does not collect them remotely".
        self.remote(target)?;
        Err(WorkspaceInspectionError::Unsupported(
            "remote_documents_unavailable",
        ))
    }

    async fn read_text_file(
        &self,
        target: &WorkspaceTarget,
        request: ReadTextFileRequest,
    ) -> Result<FileContent, WorkspaceInspectionError> {
        let (_, result) = self
            .call(target, HelperOperation::ReadTextFile { path: request.path })
            .await?;
        result
            .file
            .map(file)
            .ok_or(WorkspaceInspectionError::RemoteUnavailable(
                "remote_helper_malformed_response",
            ))
    }

    async fn search(
        &self,
        target: &WorkspaceTarget,
        request: WorkspaceSearchRequest,
    ) -> Result<FileSearchListing, WorkspaceInspectionError> {
        let (remote, result) = self
            .call(
                target,
                HelperOperation::Search {
                    query: request.query,
                    max_results: request.max_results,
                },
            )
            .await?;
        result.search.map(|value| search(&remote, value)).ok_or(
            WorkspaceInspectionError::RemoteUnavailable("remote_helper_malformed_response"),
        )
    }

    async fn git_status(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<GitStatusResult, WorkspaceInspectionError> {
        let (remote, result) = self.call(target, HelperOperation::GitStatus).await?;
        let git = result
            .git
            .ok_or(WorkspaceInspectionError::RemoteUnavailable(
                "remote_helper_malformed_response",
            ))?;
        if !git.is_repository {
            // A directory that is not a repository is an answer, not a failure: the panel shows
            // "no version control here" rather than an error a reader would try to fix.
            return Ok(GitStatusResult {
                context: context(&remote),
                is_git: false,
                branch: None,
                items: Vec::new(),
                truncated: false,
                next_cursor: None,
            });
        }
        // The local provider's parser, so the locale-independent classification of a porcelain
        // record has one implementation rather than two that agree until they do not.
        let (branch, items) = super::super::session_queries::parse_git_status(&git_output(&git)?);
        Ok(GitStatusResult {
            context: context(&remote),
            is_git: true,
            branch,
            items,
            truncated: git.truncated,
            next_cursor: None,
        })
    }

    async fn git_diff(
        &self,
        target: &WorkspaceTarget,
        request: GitDiffRequest,
    ) -> Result<GitDiffResult, WorkspaceInspectionError> {
        let path = request.path.clone();
        let (remote, result) = self
            .call(
                target,
                HelperOperation::GitDiff {
                    path: request.path,
                    staged: request.source == GitDiffSource::Staged,
                },
            )
            .await?;
        let git = result
            .git
            .ok_or(WorkspaceInspectionError::RemoteUnavailable(
                "remote_helper_malformed_response",
            ))?;
        let raw = git_output(&git)?;
        Ok(GitDiffResult {
            context: context(&remote),
            source: request.source,
            files: super::super::session_queries::parse_git_diff(
                &String::from_utf8_lossy(&raw),
                &path,
            ),
            truncated: git.truncated,
        })
    }
}
