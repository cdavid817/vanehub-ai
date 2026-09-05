/// Content search, the registry that lets one be stopped, and the ceiling on how many inspections
/// run at once.
pub(crate) use super::application::bounded_page_size;
use super::application::WorktreeCleanupService;
pub(crate) use super::application::{
    deliver_content_search, MonotonicClockPort, SystemMonotonicClock,
    WorkspaceContentSearchDelivery, WorkspaceContentSearchRequest, WorkspaceContentSearchResult,
    WorkspaceInspectionAdmission, WorkspaceInspectionExecution, WorkspaceInspectionReason,
    WorkspaceSearchCancellation, WorkspaceSearchCoverage,
};
/// Provider-neutral workspace inspection.
///
/// Published so bootstrap can assemble the router and a command can ask it questions. The
/// `WorkspaceTarget` itself is published too, because a caller has to be able to tell a reader
/// which machine an answer came from — but nothing outside this context can construct one.
pub(crate) use super::application::{
    deliver_path_search, CapabilityState, WorkspaceInspectionCapabilities,
    WorkspaceInspectionError, WorkspaceInspectionRouter, WorkspacePathSearchDelivery,
    WorkspacePathSearchRequest, WorkspacePathSearchResult, WorkspaceTarget,
};
/// Ordinary-session worktree cleanup: the records, the read-only inspection vocabulary, the
/// policy that decides what may be offered, and the gate other contexts consult before they
/// admit new work into a directory.
pub(crate) use super::application::{
    evaluate_cleanup, reason as cleanup_reason, CheckCompleteness, GateClaim, GateHolder,
    GateOwner, GateRejection, Presence, ProbeBudget, ReferenceSummary, WorktreeCleanupPolicy,
    WorktreeInspection, WorktreeObservation, WorktreeRemovalOutcome, WorktreeRemovalReport,
    WorktreeResolution, WorktreeSessionView,
};
pub(crate) use super::application::{
    AttachSessionShellRequest, CreateSessionShellRequest, ResizeSessionShellRequest,
    SessionShellCleanupReport, SessionShellCloseResult, SessionShellDescriptor,
    SessionShellRegistry, ShellAttachSnapshot, ShellAttachmentScope, WriteSessionShellRequest,
};
pub(crate) use super::application::{
    CreatedWorktree, DirectoryListing, DocumentListing, FileContent, FileSearchListing,
    GitBranchReference, GitDiffFile, GitDiffHunk, GitDiffLine, GitDiffResult, GitDiffSource,
    GitStatusResult, KnownProject, KnownRemoteWorkspace, ReviewDiffFile, ReviewPatch,
    ReviewPatchRequest, ReviewRevertReceipt, ReviewRevertRequest, ReviewSnapshot,
    SessionLogExportResult, SessionLogQuery, SessionWorkspaceContext,
    WorkspaceApplicationError as WorkspaceError, WorkspaceLogLevel, WorkspaceReviewPort,
};
use super::application::{WorkspaceApplicationService, WorkspaceQueryApplicationService};
/// Normalized workspace change notices.
///
/// The scope and source vocabularies are published because producers outside this context observe
/// changes — the runtime knows it wrote a file long before any watcher could see it. The dispatcher
/// is published so bootstrap can assemble it once and hand the same one to every producer.
pub(crate) use super::application::{
    WorkspaceChangeObserverPort, WorkspaceInvalidationChange, WorkspaceInvalidationDispatcher,
    WorkspaceInvalidationScope, WorkspaceInvalidationSource,
};
pub(crate) use super::application::{
    WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceFileChangeKind,
    WorkspaceShellCloseReason, WorkspaceShellRuntimeKind,
};
pub(crate) use super::domain::{
    ensure_git_worktree_available, ensure_worktree_compatible, ProjectInspection, RemoteWorkspace,
    ShellRuntimeDescriptor,
};
pub(crate) use super::domain::{ManagedWorktree, WorktreeIdentity, WorktreeOrigin};
pub(crate) use super::domain::{SessionShellError, ShellId};
pub(crate) use super::infrastructure::PreparedEvaluationFixture;
use super::infrastructure::SystemWorkspaceChangeObserver;
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// What another context answers when asked whether a session may start new work.
///
/// Implemented by whoever owns session deletion claims and bound at bootstrap: a Shell opened
/// for a session that is mid-deletion would be an orphan with a live process behind it.
pub(crate) trait WorkspaceExecutionAdmissionPort: Send + Sync {
    /// `Err` carries the stable reason code that refuses admission.
    fn ensure_session_admits_execution(&self, session_id: &str) -> Result<(), &'static str>;
}
use std::time::{SystemTime, UNIX_EPOCH};

/// Wall-clock milliseconds, for the coalescing window and the observation lifetime.
///
/// A clock before the epoch is treated as zero rather than refused: the consequence is one poll
/// cycle behaving oddly on a machine whose clock is badly wrong, which is not worth failing a file
/// listing over.
fn unix_milliseconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone)]
pub(crate) struct WorkspaceApi {
    service: WorkspaceApplicationService,
    queries: WorkspaceQueryApplicationService,
    review: Arc<dyn WorkspaceReviewPort>,
    shells: Arc<SessionShellRegistry>,
    /// Provider-neutral inspection. Shared rather than owned per call because selection is a
    /// property of the session, not of the caller, and two routers could disagree about it.
    inspection: Arc<WorkspaceInspectionRouter>,
    /// Where change notices are buffered, and what remembers which directories are open.
    ///
    /// Held here because the reads that populate it come through this API. A separate "subscribe to
    /// this directory" call would be a second statement of what a console is looking at, and the
    /// two would disagree the first time one of them was forgotten.
    invalidation: Arc<WorkspaceInvalidationDispatcher>,
    /// Which content searches are in flight.
    ///
    /// Owned here rather than by the router, because cancelling is a different command from
    /// searching and both need to reach the same registry.
    searches: Arc<WorkspaceSearchCancellation>,
    /// How many inspections may run at once, globally and per workspace.
    ///
    /// Held here rather than inside a provider, because the point of a ceiling is to refuse
    /// *before* a blocking task or an SSH channel exists. A provider that admitted itself would be
    /// deciding after the cost had already been paid.
    admission: Arc<WorkspaceInspectionAdmission>,
    /// The clock every bounded inspection measures its deadline against.
    ///
    /// Assembled here rather than constructed inside each traversal. A walk that reaches for
    /// `Instant::now` itself cannot be given a different clock, so a deadline is only provable by
    /// waiting one out — and a suite that waits out a twenty-second deadline is a suite nobody runs.
    clock: Arc<dyn MonotonicClockPort>,
    /// Absent only in assemblies that never delete a worktree (tests of unrelated surfaces).
    cleanup: Option<WorktreeCleanupService>,
    /// Bound after the sessions context exists; until then admission is unconditional, which is
    /// the pre-existing behaviour and never a silent refusal.
    execution_admission: Arc<OnceLock<Arc<dyn WorkspaceExecutionAdmissionPort>>>,
}

impl WorkspaceApi {
    pub(crate) fn prepare_evaluation_fixture(
        &self,
        source: &Path,
        root: &Path,
        attempt_id: &str,
    ) -> Result<PreparedEvaluationFixture, String> {
        super::infrastructure::prepare_evaluation_fixture(source, root, attempt_id)
    }

    pub(crate) fn cleanup_evaluation_fixture(
        &self,
        root: &Path,
        attempt_id: &str,
    ) -> Result<(), String> {
        super::infrastructure::cleanup_evaluation_fixture(root, attempt_id)
    }
    pub(crate) fn changed_evaluation_paths(
        &self,
        source: &Path,
        workspace: &Path,
    ) -> Result<Vec<String>, String> {
        super::infrastructure::changed_evaluation_paths(source, workspace)
    }
    pub(crate) fn new(
        service: WorkspaceApplicationService,
        queries: WorkspaceQueryApplicationService,
        review: Arc<dyn WorkspaceReviewPort>,
        shells: Arc<SessionShellRegistry>,
        inspection: Arc<WorkspaceInspectionRouter>,
        invalidation: Arc<WorkspaceInvalidationDispatcher>,
    ) -> Self {
        Self {
            service,
            queries,
            review,
            shells,
            inspection,
            invalidation,
            searches: Arc::new(WorkspaceSearchCancellation::default()),
            admission: Arc::new(WorkspaceInspectionAdmission::default()),
            clock: Arc::new(SystemMonotonicClock::default()),
            cleanup: None,
            execution_admission: Arc::new(OnceLock::new()),
        }
    }

    pub(crate) fn with_worktree_cleanup(mut self, cleanup: WorktreeCleanupService) -> Self {
        self.cleanup = Some(cleanup);
        self
    }

    /// Called once from bootstrap after the sessions context is assembled. A second call is an
    /// ordering bug and is ignored rather than surfaced, matching the other deferred bindings.
    pub(crate) fn bind_execution_admission(
        &self,
        admission: Arc<dyn WorkspaceExecutionAdmissionPort>,
    ) {
        let _ = self.execution_admission.set(admission);
    }

    fn cleanup(&self) -> Result<&WorktreeCleanupService, WorkspaceError> {
        self.cleanup.as_ref().ok_or_else(|| {
            WorkspaceError::Storage("worktree cleanup service is unavailable".to_string())
        })
    }

    /// Refuses new execution in a session that is being deleted or whose directory is gated.
    fn ensure_execution_admitted(&self, session_id: &str) -> Result<(), SessionShellError> {
        if let Some(admission) = self.execution_admission.get() {
            if let Err(code) = admission.ensure_session_admits_execution(session_id) {
                return Err(SessionShellError::Runtime {
                    reason: crate::contexts::workspaces::domain::shell_reason(code),
                });
            }
        }
        if let (Some(cleanup), Ok(Some(root))) = (
            self.cleanup.as_ref(),
            self.queries.resolve_session_root(session_id),
        ) {
            if cleanup.is_path_gated(&root).unwrap_or(true) {
                return Err(SessionShellError::Runtime {
                    reason: crate::contexts::workspaces::domain::shell_reason(
                        cleanup_reason::GATE_HELD,
                    ),
                });
            }
        }
        Ok(())
    }

    // --- Ordinary-session worktree cleanup -------------------------------------------------

    pub(crate) fn confirm_worktree_created(
        &self,
        worktree_id: &str,
        session_id: &str,
    ) -> Result<ManagedWorktree, WorkspaceError> {
        self.cleanup()?.confirm_created(worktree_id, session_id)
    }

    pub(crate) fn mark_worktree_needs_attention(
        &self,
        worktree_id: &str,
        reason: &str,
    ) -> Result<(), WorkspaceError> {
        self.cleanup()?.mark_needs_attention(worktree_id, reason)
    }

    pub(crate) fn resolve_session_worktree(
        &self,
        view: &WorktreeSessionView,
    ) -> Result<Option<WorktreeResolution>, WorkspaceError> {
        self.cleanup()?.resolve_for_session(view)
    }

    pub(crate) fn session_has_managed_worktree(
        &self,
        session_id: &str,
    ) -> Result<bool, WorkspaceError> {
        self.cleanup()?.has_record_for_session(session_id)
    }

    pub(crate) fn inspect_managed_worktree(
        &self,
        worktree_id: &str,
        budget: &ProbeBudget,
    ) -> Result<WorktreeInspection, WorkspaceError> {
        self.cleanup()?.inspect(worktree_id, budget)
    }

    pub(crate) fn managed_worktree_sessions(
        &self,
        worktree_id: &str,
    ) -> Result<Vec<String>, WorkspaceError> {
        self.cleanup()?.bound_sessions(worktree_id)
    }

    pub(crate) fn claim_worktree_gate(
        &self,
        worktree_id: &str,
        owner: &GateOwner,
    ) -> Result<GateClaim, GateRejection> {
        match self.cleanup() {
            Ok(cleanup) => cleanup.claim_gate(worktree_id, owner),
            Err(error) => Err(GateRejection::Storage(error.to_string())),
        }
    }

    pub(crate) fn release_worktree_gate(&self, claim: &GateClaim) -> Result<(), WorkspaceError> {
        self.cleanup()?.release_gate(claim)
    }

    pub(crate) fn foreign_worktree_gate_holder(
        &self,
        canonical_root: &str,
        owner: Option<&GateOwner>,
    ) -> Result<Option<GateHolder>, WorkspaceError> {
        self.cleanup()?.foreign_gate_holder(canonical_root, owner)
    }

    /// Whether a live cleanup holds `path` or a parent of it. Consulted before a directory is
    /// bound to anything new.
    pub(crate) fn is_path_gated(&self, path: &str) -> Result<bool, WorkspaceError> {
        match self.cleanup.as_ref() {
            Some(cleanup) => cleanup.is_path_gated(path),
            None => Ok(false),
        }
    }

    pub(crate) fn begin_worktree_removal(
        &self,
        worktree_id: &str,
        expected_revision: u64,
    ) -> Result<ManagedWorktree, WorkspaceError> {
        self.cleanup()?
            .begin_removal(worktree_id, expected_revision)
    }

    pub(crate) fn remove_worktree_safely(
        &self,
        worktree_id: &str,
        expected: &WorktreeIdentity,
        claim: &GateClaim,
    ) -> Result<WorktreeRemovalReport, WorkspaceError> {
        self.cleanup()?.remove_safely(worktree_id, expected, claim)
    }

    pub(crate) fn observe_managed_worktree(
        &self,
        worktree_id: &str,
    ) -> Result<Option<WorktreeObservation>, WorkspaceError> {
        self.cleanup()?.observe(worktree_id)
    }

    pub(crate) fn finalize_worktree_removed(
        &self,
        worktree_id: &str,
        session_ids: &[String],
    ) -> Result<(), WorkspaceError> {
        self.cleanup()?.finalize_removed(worktree_id, session_ids)
    }

    pub(crate) fn finalize_worktree_retained(
        &self,
        worktree_id: &str,
        session_ids: &[String],
    ) -> Result<(), WorkspaceError> {
        self.cleanup()?.finalize_retained(worktree_id, session_ids)
    }

    pub(crate) fn worktree_removal_refused(&self, worktree_id: &str) -> Result<(), WorkspaceError> {
        self.cleanup()?.removal_refused(worktree_id)
    }

    /// How a producer elsewhere in the process reports a change it saw.
    ///
    /// Handed out as the narrow port rather than as this API, so the runtime's mutation fanout takes
    /// a dependency on "somewhere to report a change" instead of on workspaces as a whole.
    pub(crate) fn change_observer(&self) -> Arc<dyn WorkspaceChangeObserverPort> {
        Arc::new(SystemWorkspaceChangeObserver::new(
            self.invalidation.clone(),
        ))
    }

    pub(crate) fn create_review_snapshot(
        &self,
        session_id: &str,
    ) -> Result<ReviewSnapshot, WorkspaceError> {
        self.review.create_review_snapshot(session_id)
    }

    pub(crate) fn load_review_file(
        &self,
        session_id: &str,
        path: &str,
        expected_snapshot: &str,
    ) -> Result<ReviewDiffFile, WorkspaceError> {
        self.review
            .load_review_file(session_id, path, expected_snapshot)
    }

    pub(crate) fn render_review_patch(
        &self,
        request: &ReviewPatchRequest,
    ) -> Result<ReviewPatch, WorkspaceError> {
        self.review.render_review_patch(request)
    }

    pub(crate) fn revert_review_change(
        &self,
        request: &ReviewRevertRequest,
    ) -> Result<ReviewRevertReceipt, WorkspaceError> {
        self.review.revert_review_change(request)
    }

    pub(crate) fn list_known_projects(&self) -> Result<Vec<KnownProject>, WorkspaceError> {
        self.service.list_known_projects()
    }

    pub(crate) fn resolve_session_root(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, WorkspaceError> {
        self.queries.resolve_session_root(session_id)
    }

    /// A directory inside a session's workspace, as an absolute path.
    ///
    /// The one place that turns a workspace-relative directory into something an external tool can
    /// be handed. `None` means the session has no local workspace at all, which is a different
    /// answer from a path that is not inside one — the second is a refusal.
    pub(crate) fn resolve_session_directory(
        &self,
        session_id: &str,
        relative: &str,
    ) -> Result<Option<String>, WorkspaceError> {
        self.queries.resolve_session_directory(session_id, relative)
    }

    pub(crate) fn list_known_remote_workspaces(
        &self,
    ) -> Result<Vec<KnownRemoteWorkspace>, WorkspaceError> {
        self.service.list_known_remote_workspaces()
    }

    pub(crate) fn inspect_project(&self, path: &str) -> Result<ProjectInspection, WorkspaceError> {
        self.service.inspect_project(path)
    }

    pub(crate) fn remember_project(
        &self,
        inspection: &ProjectInspection,
    ) -> Result<(), WorkspaceError> {
        self.service.remember_project(inspection)
    }

    pub(crate) fn remember_remote_workspace(
        &self,
        workspace: &RemoteWorkspace,
    ) -> Result<(), WorkspaceError> {
        self.service.remember_remote_workspace(workspace)
    }

    pub(crate) fn select_project_directory(&self) -> Result<Option<String>, WorkspaceError> {
        self.service.select_project_directory()
    }

    /// Creates an ordinary-session worktree with its provenance recorded first.
    ///
    /// Intent is persisted against the settled target before `git worktree add` runs; a record
    /// that cannot be written means Git does not run. A Git failure leaves the record marked for
    /// attention rather than deleting anything, and a success returns the record id so the
    /// session that owns the directory can be bound to it once the session exists.
    pub(crate) fn create_worktree(
        &self,
        project_path: &str,
        name: &str,
    ) -> Result<CreatedWorktree, WorkspaceError> {
        let plan = self.service.plan_worktree(project_path, name)?;
        let Some(cleanup) = self.cleanup.as_ref() else {
            return self.service.create_planned_worktree(&plan);
        };
        let intent = cleanup.register_intent(
            WorktreeOrigin::OrdinarySession,
            &plan.project,
            &plan.target,
            None,
        )?;
        match self.service.create_planned_worktree(&plan) {
            Ok(mut created) => {
                created.worktree_id = Some(intent.id);
                Ok(created)
            }
            Err(error) => {
                let _ = cleanup.mark_needs_attention(&intent.id, "creation_failed");
                Err(error)
            }
        }
    }

    pub(crate) fn list_git_branches(
        &self,
        project_path: &str,
    ) -> Result<Vec<GitBranchReference>, WorkspaceError> {
        self.service.list_git_branches(project_path)
    }

    pub(crate) async fn list_git_branches_blocking(
        &self,
        project_path: String,
    ) -> Result<Vec<GitBranchReference>, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || api.list_git_branches(&project_path))
            .await
            .map_err(|_| WorkspaceError::Storage("Git branch discovery task failed".to_string()))?
    }

    pub(crate) fn create_guarded_loop_worktree(
        &self,
        project_path: &str,
        name: &str,
        base_branch: &str,
    ) -> Result<CreatedWorktree, WorkspaceError> {
        self.service
            .create_guarded_loop_worktree(project_path, name, base_branch)
    }

    /// One page of a directory, and the note that somebody is looking at it.
    ///
    /// The note is recorded on the way out and only when the read worked, for every page rather
    /// than only the first. A directory that could not be listed is not one a console is showing,
    /// and polling it would spend a stat every tick to rediscover that.
    pub(crate) fn list_session_directory_page(
        &self,
        session_id: &str,
        path: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<DirectoryListing, WorkspaceError> {
        let listing = self
            .queries
            .list_directory_page(session_id, path, cursor, limit)?;
        self.invalidation
            .note_directory_read(session_id, path, unix_milliseconds());
        Ok(listing)
    }

    /// Every Markdown and text file in the project, as a recursive walk.
    ///
    /// Admission is acquired before the walk and held until it exits, and the walk polls the
    /// registration's token. It is a recursive traversal of an entire project on a blocking thread,
    /// which is the shape of work the ceiling exists for — and until now it was the one such walk
    /// that could start any number of times with nothing to stop it.
    pub(crate) async fn list_session_documents(
        &self,
        session_id: String,
        search_id: String,
    ) -> Result<DocumentListing, WorkspaceError> {
        let registration = self.searches.begin(&search_id);
        let Ok(_permit) = self.admission.acquire(&session_id).await else {
            registration.complete();
            return Ok(DocumentListing {
                context: SessionWorkspaceContext::available(None),
                items: Vec::new(),
                truncated: false,
                next_cursor: None,
                coverage: WorkspaceSearchCoverage::stopped(
                    WorkspaceInspectionReason::InspectionBusy,
                ),
            });
        };
        let api = self.clone();
        let execution = WorkspaceInspectionExecution::document_discovery(
            registration.generation(),
            registration.token(),
            Arc::clone(&self.clock),
        );
        let outcome = tauri::async_runtime::spawn_blocking(move || {
            api.queries.list_documents(&session_id, &execution)
        })
        .await
        .map_err(|_| WorkspaceError::Storage("session documents task failed".to_string()))?;
        registration.complete();
        outcome
    }

    pub(crate) fn search_session_files(
        &self,
        session_id: &str,
        query: &str,
        max_results: usize,
    ) -> Result<FileSearchListing, WorkspaceError> {
        self.queries.search_files(session_id, query, max_results)
    }

    pub(crate) fn read_session_file(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<FileContent, WorkspaceError> {
        self.queries.read_file(session_id, path)
    }

    pub(crate) fn read_session_text_file(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<FileContent, WorkspaceError> {
        self.queries.read_text_file(session_id, path)
    }

    /// Which machine this session's workspace is on, and what can be read there.
    ///
    /// Resolved from the registered binding on every call rather than cached: a session can be
    /// rebound between two reads, and a cached target would keep answering about the host it was
    /// bound to when the panel opened.
    pub(crate) fn inspection_target(
        &self,
        session_id: &str,
    ) -> Result<WorkspaceTarget, WorkspaceInspectionError> {
        self.inspection.target(session_id)
    }

    pub(crate) async fn inspection_capabilities(
        &self,
        session_id: &str,
    ) -> Result<WorkspaceInspectionCapabilities, WorkspaceInspectionError> {
        self.inspection.capabilities(session_id).await
    }

    /// Content search, admitted and registered so it can be stopped.
    ///
    /// Three things happen here and nowhere else. Admission is acquired before any blocking or
    /// remote work exists, so a burst is refused rather than queued. The registration is taken
    /// before the search starts, so a cancel arriving in the window between the request leaving
    /// the frontend and the first directory being read still lands. And the guard that owns the
    /// registration stays in *this* future - so an abort releases it and signals the worker, and a
    /// walk on the blocking pool never touches the registry at all.
    ///
    /// The permit is held for as long as this future is, and the provider's future is awaited
    /// inside it, so the ceiling counts work that is actually running rather than callers that are
    /// still interested in the answer.
    pub(crate) async fn search_workspace_content(
        &self,
        session_id: &str,
        request: WorkspaceContentSearchRequest,
    ) -> Result<WorkspaceContentSearchDelivery, WorkspaceInspectionError> {
        // Registered before admission is awaited, not after. Admission is a semaphore and the wait
        // is unbounded in principle, so a cancel arriving during it would otherwise find no slot and
        // report that nothing was running — for a request the reader can see is in progress.
        let registration = self.searches.begin(&request.search_id);
        let generation = registration.generation().value();

        let Ok(_permit) = self.admission.acquire(session_id).await else {
            // Busy is an answer, not an error: no blocking task was queued and no remote process
            // was launched, and telling a reader the system is busy is more honest than starting
            // hidden work whose result they will never see.
            registration.complete();
            return Ok(WorkspaceContentSearchDelivery {
                generation,
                result: WorkspaceContentSearchResult {
                    coverage: WorkspaceSearchCoverage::stopped(
                        WorkspaceInspectionReason::InspectionBusy,
                    ),
                    matches: Vec::new(),
                },
            });
        };
        let execution = WorkspaceInspectionExecution::content_search(
            registration.generation(),
            registration.token(),
            Arc::clone(&self.clock),
        );
        let outcome = self
            .inspection
            .search_content(session_id, request, execution)
            .await;

        // Decided before the guard is released, because releasing it removes the slot the decision
        // compares against.
        let delivery = outcome.map(|result| deliver_content_search(&registration, result));
        registration.complete();
        delivery
    }

    /// Asks a running search to stop, and says whether one was there to ask.
    pub(crate) fn cancel_workspace_search(&self, search_id: &str) -> bool {
        self.searches.cancel(search_id)
    }

    /// Quick Open. Routed through the same seam as every other read, so a remote workspace answers
    /// for the same reason a local one does.
    pub(crate) async fn search_workspace_paths(
        &self,
        session_id: &str,
        request: WorkspacePathSearchRequest,
    ) -> Result<WorkspacePathSearchDelivery, WorkspaceInspectionError> {
        // The same order content search uses, for the same two reasons. Registered before admission
        // is awaited, so a cancel arriving during the wait has a slot to land on; admission acquired
        // before the walk, so a refusal costs nothing rather than being decided after a blocking
        // thread is already committed. Quick Open is cheaper than a content search but it is still a
        // filesystem walk, and a reader holding a key down starts one per repeat.
        let registration = self.searches.begin(&request.search_id);
        let generation = registration.generation().value();

        let Ok(_permit) = self.admission.acquire(session_id).await else {
            registration.complete();
            return Ok(WorkspacePathSearchDelivery {
                generation,
                result: WorkspacePathSearchResult {
                    coverage: WorkspaceSearchCoverage::stopped(
                        WorkspaceInspectionReason::InspectionBusy,
                    ),
                    matches: Vec::new(),
                    next_cursor: None,
                },
            });
        };
        // Built here, where the registration is, because the generation is half of what a stale
        // arrival is judged against and the walk has no other way to learn it.
        let execution = WorkspaceInspectionExecution::path_search(
            registration.generation(),
            registration.token(),
            Arc::clone(&self.clock),
        );
        let outcome = self
            .inspection
            .search_paths(session_id, request, execution)
            .await;

        // Decided before the guard is released, because releasing it removes the slot the decision
        // compares against.
        let delivery = outcome.map(|result| deliver_path_search(&registration, result));
        registration.complete();
        delivery
    }

    pub(crate) fn get_session_git_status(
        &self,
        session_id: &str,
    ) -> Result<GitStatusResult, WorkspaceError> {
        self.queries.git_status(session_id)
    }

    /// Async wrapper that runs `git status` on the blocking pool, since it can hit the
    /// process timeout on slow repositories and must not freeze the async executor.
    pub(crate) async fn get_session_git_status_blocking(
        &self,
        session_id: String,
    ) -> Result<GitStatusResult, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || api.get_session_git_status(&session_id))
            .await
            .map_err(|_| WorkspaceError::Storage("git status task failed".to_string()))?
    }

    pub(crate) fn get_session_git_diff(
        &self,
        session_id: &str,
        path: &str,
        source: GitDiffSource,
    ) -> Result<GitDiffResult, WorkspaceError> {
        self.queries.git_diff(session_id, path, source)
    }

    /// Async wrapper for `git diff`, which can spawn git twice on slow repositories.
    pub(crate) async fn get_session_git_diff_blocking(
        &self,
        session_id: String,
        path: String,
        source: GitDiffSource,
    ) -> Result<GitDiffResult, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            api.get_session_git_diff(&session_id, &path, source)
        })
        .await
        .map_err(|_| WorkspaceError::Storage("git diff task failed".to_string()))?
    }

    // The interactive log query moved to the operations-owned index. Nothing here scans log
    // files for a query any more: a fallback would be a second implementation with different
    // bounds and different coverage semantics, reached exactly when a reader is least able to
    // tell which one answered. Export still reads the redacted files, which is what an export is.
    pub(crate) fn export_session_logs(
        &self,
        query: &SessionLogQuery,
    ) -> Result<SessionLogExportResult, WorkspaceError> {
        self.queries.export_logs(query)
    }

    /// Async wrapper for log export, which writes a file and may surface a save dialog.
    pub(crate) async fn export_session_logs_blocking(
        &self,
        query: SessionLogQuery,
    ) -> Result<SessionLogExportResult, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || api.export_session_logs(&query))
            .await
            .map_err(|_| WorkspaceError::Storage("session log export task failed".to_string()))?
    }

    /// Async wrapper for directory listing, which walks the filesystem synchronously.
    ///
    /// Admission is acquired before the blocking task is spawned and held until it returns. A
    /// listing is a filesystem walk on a pool with a fixed number of threads, and a console with
    /// twenty folders open would otherwise queue twenty of them behind whatever else is reading the
    /// same disk — with nothing anywhere to say the system had run out of capacity.
    pub(crate) async fn list_session_directory_blocking(
        &self,
        session_id: String,
        path: String,
        cursor: Option<String>,
        limit: Option<usize>,
    ) -> Result<DirectoryListing, WorkspaceError> {
        let Ok(_permit) = self.admission.acquire(&session_id).await else {
            return Ok(DirectoryListing {
                context: SessionWorkspaceContext::available(None),
                path,
                items: Vec::new(),
                truncated: false,
                next_cursor: None,
                coverage: WorkspaceSearchCoverage::stopped(
                    WorkspaceInspectionReason::InspectionBusy,
                ),
            });
        };
        let api = self.clone();
        let limit = bounded_page_size(limit);
        tauri::async_runtime::spawn_blocking(move || {
            api.list_session_directory_page(&session_id, &path, cursor.as_deref(), limit)
        })
        .await
        .map_err(|_| WorkspaceError::Storage("session directory task failed".to_string()))?
    }

    /// Async wrapper for mention candidate search, which walks the filesystem synchronously.
    pub(crate) async fn search_session_files_blocking(
        &self,
        session_id: String,
        query: String,
        max_results: usize,
    ) -> Result<FileSearchListing, WorkspaceError> {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            api.search_session_files(&session_id, &query, max_results)
        })
        .await
        .map_err(|_| WorkspaceError::Storage("session file search task failed".to_string()))?
    }

    /// Ends every Shell a session owns.
    ///
    /// Called on the "this session is done" edge — archive and delete — and on no other. A retained
    /// Shell outlives its view by design, so nothing else would ever close it: the session it
    /// belonged to would be gone from the list while its process kept running with no way left to
    /// reach it.
    /// Strict by default: a session whose Shells are not all confirmed gone is not a session that
    /// finished archiving. Reporting success here would delete the last thing that could reach the
    /// process still running behind it.
    ///
    /// A `Conflict` rather than a storage failure, because the code is what a caller matches on and
    /// the retry is a real one: the Shells and the session are both still addressable, and the same
    /// call made again finishes the job once cleanup confirms.
    pub(crate) fn kill_shells_for_session(&self, session_id: &str) -> Result<(), WorkspaceError> {
        if self.shells.close_for_session(session_id).is_complete() {
            return Ok(());
        }
        Err(WorkspaceError::Conflict(
            crate::contexts::workspaces::domain::shell_reason_code::SESSION_CLEANUP_INCOMPLETE,
        ))
    }

    pub(crate) fn list_session_shells(&self, session_id: &str) -> Vec<SessionShellDescriptor> {
        self.shells.list(Some(session_id))
    }

    pub(crate) fn create_session_shell(
        &self,
        request: &CreateSessionShellRequest,
    ) -> Result<SessionShellDescriptor, SessionShellError> {
        self.ensure_execution_admitted(&request.session_id)?;
        self.shells.create(request)
    }

    /// Opens a Shell off the main thread.
    ///
    /// A PTY spawn is quick and an SSH handshake is not, and a synchronous Tauri command runs where
    /// the webview runs: the window would stop repainting until the far end answered. The four
    /// Shell operations that reach the runtime all go through the blocking pool for that reason —
    /// closing is the worst of them, because it kills a process, waits for it, and joins its reader.
    pub(crate) async fn create_session_shell_blocking(
        &self,
        request: CreateSessionShellRequest,
    ) -> Result<SessionShellDescriptor, SessionShellError> {
        self.on_blocking_pool(move |api| api.create_session_shell(&request))
            .await
    }

    pub(crate) async fn write_session_shell_blocking(
        &self,
        request: WriteSessionShellRequest,
    ) -> Result<(), SessionShellError> {
        self.on_blocking_pool(move |api| api.write_session_shell(&request))
            .await
    }

    pub(crate) async fn resize_session_shell_blocking(
        &self,
        request: ResizeSessionShellRequest,
    ) -> Result<(), SessionShellError> {
        self.on_blocking_pool(move |api| api.resize_session_shell(&request))
            .await
    }

    pub(crate) async fn close_session_shell_blocking(
        &self,
        shell_id: ShellId,
    ) -> Result<SessionShellCloseResult, SessionShellError> {
        self.on_blocking_pool(move |api| Ok(api.close_session_shell(&shell_id)))
            .await
    }

    /// A task that could not be scheduled is reported as a runtime failure rather than swallowed:
    /// the caller has to know its Shell operation did not happen.
    async fn on_blocking_pool<T, F>(&self, work: F) -> Result<T, SessionShellError>
    where
        T: Send + 'static,
        F: FnOnce(WorkspaceApi) -> Result<T, SessionShellError> + Send + 'static,
    {
        let api = self.clone();
        tauri::async_runtime::spawn_blocking(move || work(api))
            .await
            .map_err(|_| SessionShellError::Runtime {
                reason: crate::contexts::workspaces::domain::shell_reason("shell_task_failed"),
            })?
    }

    pub(crate) fn attach_session_shell(
        &self,
        request: &AttachSessionShellRequest,
    ) -> Result<ShellAttachSnapshot, SessionShellError> {
        self.shells.attach(request)
    }

    pub(crate) fn detach_session_shell(
        &self,
        scope: &ShellAttachmentScope,
    ) -> Result<(), SessionShellError> {
        self.shells.detach(scope)
    }

    pub(crate) fn write_session_shell(
        &self,
        request: &WriteSessionShellRequest,
    ) -> Result<(), SessionShellError> {
        self.shells.write(request)
    }

    pub(crate) fn resize_session_shell(
        &self,
        request: &ResizeSessionShellRequest,
    ) -> Result<(), SessionShellError> {
        self.shells.resize(request)
    }

    pub(crate) fn rename_session_shell(
        &self,
        shell_id: &ShellId,
        title: &str,
    ) -> Result<SessionShellDescriptor, SessionShellError> {
        self.shells.rename(shell_id, title)
    }

    /// Ends a Shell and reports what that achieved.
    ///
    /// Returns a value rather than `Result<(), _>` because three of the four outcomes are not
    /// errors and only two of them mean the process is gone.
    pub(crate) fn close_session_shell(&self, shell_id: &ShellId) -> SessionShellCloseResult {
        self.shells.close(shell_id)
    }

    /// How many Shells a session is holding, for the workspace summary.
    ///
    /// Owned by the registry rather than counted by the panel: a badge produced by mounting a list
    /// is a badge that opens what it is describing.
    pub(crate) fn live_session_shell_count(&self, session_id: &str) -> usize {
        self.shells.live_count(session_id)
    }

    /// Reclaims detached, quiet Shells and advances outstanding cleanup.
    ///
    /// Bounded per sweep and never a Shell someone is watching. The report distinguishes a Shell
    /// that was confirmed gone from one that is still being reaped; counting the second as
    /// reclaimed would make the sweep's own figures the first place this application lies about a
    /// process it did not end.
    pub(crate) fn sweep_idle_session_shells(&self) -> SessionShellCleanupReport {
        self.shells.sweep_idle()
    }

    /// Closes every Shell within one global finite budget and reports what is left.
    ///
    /// Called on the way out rather than left to the process teardown. It never waits without a
    /// ceiling: an exit path that blocks until every child dies is an application that cannot be
    /// closed, and that is a worse failure than a residual process reported honestly.
    pub(crate) fn shutdown_session_shells(&self) -> SessionShellCleanupReport {
        self.shells.shutdown()
    }
}
