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

use super::super::path_search::{normalize_query, path_match_score};
use super::probe::{capabilities_from, revalidate};
use super::protocol::{
    HelperContentMatches, HelperEntry, HelperFile, HelperFingerprint, HelperGitOutput,
    HelperListing, HelperOperation, HelperRequest, HelperResult, HelperSearch, RemoteHelperError,
};
use super::transport::{exchange, RemoteHelperSession};
use crate::contexts::workspaces::application::{
    bounded_page_size, bounded_search_page, detect_encoding, detect_newline, workspace_identity,
    DirectoryCursor, DirectoryEntry, DirectoryFingerprint, DirectoryFingerprintState,
    DirectoryListing, DirectoryOrder, DirectoryPageScope, DocumentListing, FileContent,
    FileSearchListing, FileSearchMatch, GitDiffRequest, GitDiffResult, GitDiffSource,
    GitStatusResult, ListDirectoryRequest, PathSearchCursor, ReadTextFileRequest,
    RemoteWorkspaceTarget, SearchCancellationCause, SearchCancellationToken,
    SessionWorkspaceContext, WorkspaceContentMatch, WorkspaceContentSearchRequest,
    WorkspaceContentSearchResult, WorkspaceIgnorePolicy, WorkspaceInspectionCapabilities,
    WorkspaceInspectionError, WorkspaceInspectionProvider, WorkspaceInspectionReason,
    WorkspacePathMatch, WorkspacePathSearchRequest, WorkspacePathSearchResult,
    WorkspaceSearchCoverage, WorkspaceSearchRequest, WorkspaceTarget, MAX_CONTENT_MATCHES,
    MAX_FINGERPRINT_PATHS,
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

/// How many candidates one remote walk may return.
///
/// Ranking happens on this side, so this bounds what crosses the wire rather than what a reader
/// sees. It matches the local walk's collection bound, because a workspace should not appear to
/// hold a different number of things depending on which machine it is on.
const MAX_REMOTE_PATH_CANDIDATES: usize = 2_000;

/// Ranks, resumes, and pages a remote walk's candidates.
///
/// A free function so the ordering can be exercised without a connection. Scoring on the remote
/// host would be a second implementation of an ordering that already exists, and two of them
/// disagree first about the ties nobody writes tests for -- which is why the scorer itself is
/// imported from the local walk rather than reimplemented here.
fn rank_path_candidates(
    query: &str,
    cursor: Option<&PathSearchCursor>,
    limit: usize,
    truncated: bool,
    entries: Vec<HelperEntry>,
) -> WorkspacePathSearchResult {
    let mut scored: Vec<(u32, u32, WorkspacePathMatch)> = entries
        .into_iter()
        .filter_map(|entry| {
            let score = path_match_score(query, &entry.name, &entry.path)?;
            let depth = entry.path.matches('/').count() as u32;
            Some((
                score,
                depth,
                WorkspacePathMatch {
                    name: entry.name,
                    path: entry.path,
                    kind: if entry.kind == "directory" {
                        "directory"
                    } else {
                        "file"
                    },
                },
            ))
        })
        .collect();

    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then(left.1.cmp(&right.1))
            .then_with(|| left.2.path.to_lowercase().cmp(&right.2.path.to_lowercase()))
    });

    // Resumed after the sort, because the key the cursor holds is the key this ordering produces.
    let remaining: Vec<(u32, u32, WorkspacePathMatch)> = match cursor {
        Some(cursor) => scored
            .into_iter()
            .filter(|(score, depth, entry)| cursor.precedes(*score, *depth, &entry.path))
            .collect(),
        None => scored,
    };

    let has_more = remaining.len() > limit;
    let page: Vec<(u32, u32, WorkspacePathMatch)> = remaining.into_iter().take(limit).collect();
    let next_cursor = match (has_more, page.last()) {
        (true, Some((score, depth, entry))) => {
            Some(PathSearchCursor::after(query, *score, *depth, &entry.path).encode())
        }
        _ => None,
    };

    WorkspacePathSearchResult {
        // A walk that stopped at its bound left part of the workspace unexamined, which is a
        // different fact from "more matches follow" and one that paging can never fix.
        coverage: if truncated {
            WorkspaceSearchCoverage::stopped(WorkspaceInspectionReason::EntryBudgetExhausted)
        } else {
            WorkspaceSearchCoverage::complete()
        },
        matches: page.into_iter().map(|(_, _, entry)| entry).collect(),
        next_cursor,
    }
}

/// What a stopped search returns.
///
/// Partial with a reason rather than an error: nothing went wrong, the reader simply stopped
/// waiting, and an error would put a failure notice on screen for something they did on purpose.
///
/// The cause is carried through rather than flattened, because a reader who typed another
/// character and a reader who pressed Escape are being told different things.
fn cancelled_result(cause: Option<SearchCancellationCause>) -> WorkspaceContentSearchResult {
    // The same three reasons the local walk reports, from the same mapping. A reader who cancels a
    // search on a remote workspace is being told what happened to their search, not which machine
    // it was running on.
    let reason = WorkspaceInspectionReason::from_cancellation(
        cause.unwrap_or(SearchCancellationCause::Cancelled),
    );
    WorkspaceContentSearchResult {
        coverage: WorkspaceSearchCoverage::stopped(reason),
        matches: Vec::new(),
    }
}

fn content_matches(value: HelperContentMatches) -> WorkspaceContentSearchResult {
    // A host without ripgrep is unavailable rather than empty. An empty result would tell a reader
    // their query matched nothing, which is a claim about their workspace rather than about the
    // host, and the two have completely different remediations.
    if value.unavailable {
        return WorkspaceContentSearchResult {
            coverage: WorkspaceSearchCoverage::unavailable("remote_ripgrep_missing"),
            matches: Vec::new(),
        };
    }
    WorkspaceContentSearchResult {
        coverage: if value.truncated {
            WorkspaceSearchCoverage::stopped(WorkspaceInspectionReason::ResultBudgetExhausted)
        } else {
            WorkspaceSearchCoverage::complete()
        },
        matches: value
            .matches
            .into_iter()
            .map(|entry| WorkspaceContentMatch {
                path: entry.path,
                line: entry.line,
                column: entry.column,
                snippet: entry.snippet,
                snippet_truncated: entry.truncated,
            })
            .collect(),
    }
}

fn fingerprints(values: Vec<HelperFingerprint>) -> Vec<DirectoryFingerprint> {
    values
        .into_iter()
        .map(|value| DirectoryFingerprint {
            relative_path: value.path,
            state: match (value.state.as_str(), value.value) {
                ("known", Some(digest)) => DirectoryFingerprintState::Known(digest),
                ("missing", _) => DirectoryFingerprintState::Missing,
                // `known` with no value, or a state this client does not know. Unreadable rather
                // than missing: an answer that cannot be understood is not evidence that a
                // directory went away, and reporting its removal would announce a change the
                // remote host never made.
                _ => DirectoryFingerprintState::Unreadable,
            },
        })
        .collect()
}

/// What a cursor issued for a remote listing is only valid within.
///
/// The fingerprint is absent, and that is a stated limitation rather than an oversight. The helper
/// protocol does not report one, so this side genuinely cannot tell whether the remote directory
/// changed between two pages. Carrying a placeholder would compare equal on every request and
/// report "unchanged" about something nobody observed; the absence says the detection is not available
/// here, and the cursor rules refuse to treat a page from a provider that *can* detect change as
/// interchangeable with one that cannot.
fn remote_page_scope(remote: &RemoteWorkspaceTarget, path: &str) -> DirectoryPageScope {
    DirectoryPageScope {
        workspace: workspace_identity(&format!("ssh:{}:{}", remote.connection_id, remote.root)),
        path: path.to_string(),
        order: DirectoryOrder::KindThenName,
        policy: WorkspaceIgnorePolicy::direct_navigation().identity(),
        fingerprint: None,
    }
}

fn listing(remote: &RemoteWorkspaceTarget, value: HelperListing) -> DirectoryListing {
    let path = value.path;
    let items: Vec<DirectoryEntry> = value.entries.into_iter().map(entry).collect();
    // The cursor is minted here from the last entry rather than by the helper. One encoding,
    // one directory-binding rule, and a remote host that cannot issue a resume point for a
    // directory it was not asked about.
    let next_cursor = value.truncated.then(|| {
        items.last().map(|entry| {
            DirectoryCursor::after(remote_page_scope(remote, &path), entry.kind, &entry.name)
                .encode()
        })
    });
    DirectoryListing {
        context: context(remote),
        path,
        items,
        truncated: value.truncated,
        next_cursor: next_cursor.flatten(),
        // The helper reports a page, not a scan. It has no budget counters to send back, so there is
        // nothing here that could distinguish "read the whole directory" from "stopped early" —
        // claiming complete is the honest reading of a protocol that has no way to say otherwise,
        // and the truncation flag it does send is carried separately above.
        coverage: WorkspaceSearchCoverage::complete(),
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
        // Classified on this side from the decoded text, by the same detector the local provider
        // uses. Asking the helper would be a second implementation of a rule that has to agree.
        encoding: value
            .content
            .as_deref()
            .map(|text| detect_encoding(text).token()),
        newline: value
            .content
            .as_deref()
            .map(|text| detect_newline(text).token()),
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
        // Decoded here, not on the remote host. The cursor is this side's encoding and its
        // directory binding is this side's rule; sending it whole would put both on a machine
        // that has no reason to know either.
        let cursor = match request.cursor.as_deref() {
            Some(encoded) => {
                let remote = self.remote(target)?;
                let scope = remote_page_scope(remote, &request.path);
                match DirectoryCursor::decode(encoded, &scope) {
                    Ok(cursor) => Some(cursor),
                    // A refusal, not a failure — the same answer the local provider gives, because
                    // a panel written against one adapter has to work against the other. An error
                    // here would leave the caller unable to tell "start again from the top" from
                    // "this host is unreachable", and only one of those is worth retrying.
                    Err(refusal) => {
                        return Ok(DirectoryListing {
                            context: context(remote),
                            path: request.path,
                            items: Vec::new(),
                            truncated: false,
                            next_cursor: None,
                            coverage: WorkspaceSearchCoverage::stopped(refusal.into()),
                        })
                    }
                }
            }
            None => None,
        };
        let (remote, result) = self
            .call(
                target,
                HelperOperation::ListDirectory {
                    path: request.path,
                    after_kind_rank: cursor.as_ref().map(|value| value.kind_rank),
                    after_name_key: cursor.map(|value| value.name_key),
                    limit: bounded_page_size(request.limit),
                },
            )
            .await?;
        result.listing.map(|value| listing(&remote, value)).ok_or(
            WorkspaceInspectionError::RemoteUnavailable("remote_helper_malformed_response"),
        )
    }

    async fn directory_fingerprints(
        &self,
        target: &WorkspaceTarget,
        paths: &[String],
    ) -> Result<Vec<DirectoryFingerprint>, WorkspaceInspectionError> {
        if paths.is_empty() {
            // No round trip for a question with no subject. A poll with nothing open should cost
            // nothing on the remote host, and that is only true if it does not connect.
            return Ok(Vec::new());
        }
        let (_, result) = self
            .call(
                target,
                HelperOperation::DirectoryFingerprints {
                    paths: paths.iter().take(MAX_FINGERPRINT_PATHS).cloned().collect(),
                },
            )
            .await?;
        result
            .fingerprints
            .map(fingerprints)
            .ok_or(WorkspaceInspectionError::RemoteUnavailable(
                "remote_helper_malformed_response",
            ))
    }

    async fn search_paths(
        &self,
        target: &WorkspaceTarget,
        request: WorkspacePathSearchRequest,
    ) -> Result<WorkspacePathSearchResult, WorkspaceInspectionError> {
        let normalized = normalize_query(&request.query);
        let cursor = match request.cursor.as_deref() {
            Some(encoded) => Some(PathSearchCursor::decode(encoded, &normalized)?),
            None => None,
        };
        let (_, result) = self
            .call(
                target,
                HelperOperation::SearchPaths {
                    query: normalized.clone(),
                    // The whole candidate set, not the page. This side ranks, so cutting on the
                    // remote by anything but walk order would drop candidates that would have
                    // ranked well and leave worse ones in.
                    limit: MAX_REMOTE_PATH_CANDIDATES,
                },
            )
            .await?;
        let candidates = result
            .paths
            .ok_or(WorkspaceInspectionError::RemoteUnavailable(
                "remote_helper_malformed_response",
            ))?;
        Ok(rank_path_candidates(
            &normalized,
            cursor.as_ref(),
            bounded_search_page(request.limit),
            candidates.truncated,
            candidates.entries,
        ))
    }

    async fn search_content(
        &self,
        target: &WorkspaceTarget,
        request: WorkspaceContentSearchRequest,
        cancellation: SearchCancellationToken,
    ) -> Result<WorkspaceContentSearchResult, WorkspaceInspectionError> {
        // Checked before connecting rather than only after. A reader who cancels while the request
        // is still being assembled has already stopped waiting, and opening an SSH channel to
        // answer them would spend a remote host's effort on a result nobody will read.
        if cancellation.is_cancelled() {
            return Ok(cancelled_result(cancellation.cause()));
        }
        let (_, result) = self
            .call(
                target,
                HelperOperation::SearchContent {
                    query: request.query.clone(),
                    max_results: request.limit.unwrap_or(MAX_CONTENT_MATCHES),
                },
            )
            .await?;
        // Checked again afterwards. The round trip is where the waiting actually happens, and a
        // result that arrives for an abandoned search must not be handed back as if it were wanted.
        if cancellation.is_cancelled() {
            return Ok(cancelled_result(cancellation.cause()));
        }
        let content = result
            .content
            .ok_or(WorkspaceInspectionError::RemoteUnavailable(
                "remote_helper_malformed_response",
            ))?;
        Ok(content_matches(content))
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
