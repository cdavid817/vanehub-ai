//! The retained Session Shell use cases.
//!
//! Every operation here reads or writes the store and then talks to the runtime, in that order and
//! never with the store's lock still held. A registry that held its map across a PTY open would
//! stall every other Shell in the application behind one slow SSH handshake.
//!
//! Two orderings in this file are load-bearing and neither is obvious.
//!
//! **Starting up.** Capacity is reserved, then the Shell is registered as `Opening`, and only then
//! does the runtime get invoked. Registering last — which reads as the safer order, since a failed
//! open then leaves nothing behind — is what loses the first line of output of an `echo && exit`:
//! the runtime's reader publishes into a store that has never heard of the Shell. Registering first
//! and rolling back on failure keeps both properties.
//!
//! **Shutting down.** The Shell keeps its entry, its replay, its route, and its capacity until the
//! runtime *confirms* the process is gone. Marking it closed and removing the entry first is what
//! produces a Shell the UI reports as terminated, the registry cannot find, and the operating
//! system is still running.

use super::evidence::{
    WorkspaceEvidencePort, WorkspaceEvidenceSignal, WorkspaceShellCloseReason,
    WorkspaceShellRuntimeKind,
};
use super::ports::ShellLifecycleDiagnosticsPort;
use super::session_shell::{
    AttachSessionShellRequest, CreateSessionShellRequest, ResizeSessionShellRequest,
    SessionShellDescriptor, SessionShellRuntimePort, SessionShellWorkspacePort,
    ShellAttachSnapshot, ShellAttachmentScope, ShellCapacities, ShellClockPort, ShellIdPort,
    ShellRuntimeOpen, WriteSessionShellRequest,
};
use super::session_shell_capacity::ShellCapacityController;
use super::session_shell_close::{
    SessionShellCleanupReport, SessionShellCloseResult, ShellRuntimeCloseOutcome,
};
use super::session_shell_reaper::{ShellReaperLimits, ShellReaperQueue, ShellReaperRejection};
use super::session_shell_store::{ShellEntry, ShellStore};
use crate::contexts::workspaces::domain::{
    shell_reason, shell_reason_code, SessionShellError, SessionShellState, ShellAttachmentId,
    ShellCloseBudget, ShellGeneration, ShellId, ShellReasonCode, ShellReplayBuffer,
    ShellRuntimeDescriptor, ShellTitle, TerminalDimensions,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// How long a detached, quiet Shell is kept before it is reclaimed.
pub(crate) const SHELL_IDLE_MILLIS: u64 = 30 * 60 * 1000;

/// How many Shells one idle sweep may close, so a sweep cannot become a long stall.
pub(crate) const SHELL_IDLE_SWEEP_LIMIT: usize = 4;

/// What two concurrent creates have to agree on to be the same request.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CreateIdentity {
    /// The user pressed Add. Its key is the client's, so a retried press is the same request.
    Requested(String),
    /// A tab opening its Shell for the first time. Two of these racing are one request.
    Default { session_id: String, seat_id: String },
}

pub(crate) struct SessionShellRegistry {
    store: Arc<ShellStore>,
    runtime: Arc<dyn SessionShellRuntimePort>,
    workspaces: Arc<dyn SessionShellWorkspacePort>,
    ids: Arc<dyn ShellIdPort>,
    clock: Arc<dyn ShellClockPort>,
    capacity: Arc<ShellCapacityController>,
    reaper: Arc<ShellReaperQueue>,
    budget: ShellCloseBudget,
    /// Monotonic across the process. Opaque rather than per-identity, because Shell ids are never
    /// reused and what needs telling apart is two *lives*, not two names.
    generations: AtomicU64,
    /// Where an opened or closed Shell is reported.
    ///
    /// A retained Shell is work a session did, and the console counts it. Without this the Shell
    /// figures would go quiet the moment the tab stopped using the older one-view service — which
    /// would read as "this session opened no shells" rather than as a missing wire.
    evidence: Arc<dyn WorkspaceEvidencePort>,
    /// Where a lifecycle no-op is recorded.
    ///
    /// Required rather than optional. The two events it carries are invisible by construction, and
    /// an optional port is one production can be assembled without — which would restore exactly the
    /// silence this exists to remove, with nothing to notice it.
    diagnostics: Arc<dyn ShellLifecycleDiagnosticsPort>,
    /// One gate per in-flight create identity.
    ///
    /// Held across the runtime open — which is exactly why it is not the store's lock. Two threads
    /// asking for the same default Shell serialize here, the loser re-checks, and one process is
    /// spawned. Creates for different identities never meet.
    gates: Mutex<BTreeMap<CreateIdentity, Arc<Mutex<()>>>>,
}

/// Everything the registry talks to that is not itself.
///
/// Grouped rather than passed one by one. Seven collaborators is where the argument-count rule bites,
/// and the alternative to grouping is dropping one — which in this set means dropping the one whose
/// absence is silent. Every field here is a seam an assembly has to fill deliberately.
pub(crate) struct SessionShellPorts {
    pub(crate) runtime: Arc<dyn SessionShellRuntimePort>,
    pub(crate) workspaces: Arc<dyn SessionShellWorkspacePort>,
    pub(crate) ids: Arc<dyn ShellIdPort>,
    pub(crate) clock: Arc<dyn ShellClockPort>,
    pub(crate) evidence: Arc<dyn WorkspaceEvidencePort>,
    pub(crate) diagnostics: Arc<dyn ShellLifecycleDiagnosticsPort>,
}

impl SessionShellRegistry {
    pub(crate) fn new(
        store: Arc<ShellStore>,
        ports: SessionShellPorts,
        capacities: ShellCapacities,
    ) -> Self {
        Self {
            store,
            runtime: ports.runtime,
            workspaces: ports.workspaces,
            ids: ports.ids,
            clock: ports.clock,
            capacity: Arc::new(ShellCapacityController::new(capacities)),
            reaper: Arc::new(ShellReaperQueue::new(ShellReaperLimits::default())),
            budget: ShellCloseBudget::default(),
            generations: AtomicU64::new(0),
            evidence: ports.evidence,
            diagnostics: ports.diagnostics,
            gates: Mutex::new(BTreeMap::new()),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_close_budget(mut self, budget: ShellCloseBudget) -> Self {
        self.budget = budget;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_reaper_limits(mut self, limits: ShellReaperLimits) -> Self {
        self.reaper = Arc::new(ShellReaperQueue::new(limits));
        self
    }

    #[cfg(test)]
    pub(crate) fn capacity(&self) -> Arc<ShellCapacityController> {
        self.capacity.clone()
    }

    #[cfg(test)]
    pub(crate) fn reaper_depth(&self) -> usize {
        self.reaper.depth()
    }

    pub(crate) fn store(&self) -> Arc<ShellStore> {
        self.store.clone()
    }

    pub(crate) fn list(&self, session_id: Option<&str>) -> Vec<SessionShellDescriptor> {
        self.store.descriptors(session_id)
    }

    pub(crate) fn create(
        &self,
        request: &CreateSessionShellRequest,
    ) -> Result<SessionShellDescriptor, SessionShellError> {
        let workspace = match request.working_directory.as_deref() {
            Some(relative) if !relative.is_empty() => {
                self.workspaces.resolve_at(&request.session_id, relative)?
            }
            _ => self.workspaces.resolve(&request.session_id)?,
        };
        if workspace.read_only {
            return Err(SessionShellError::PolicyDenied);
        }
        // A Shell is one interactive channel with one runtime owner. Picking a seat for a caller
        // that did not name one would attribute a session's work to a participant at random.
        if workspace.seat_count > 1 && request.seat_id.is_none() {
            return Err(SessionShellError::SeatRequired);
        }

        let identity = match &request.request_id {
            Some(request_id) => CreateIdentity::Requested(request_id.as_str().to_string()),
            None => CreateIdentity::Default {
                session_id: request.session_id.clone(),
                seat_id: request.seat_id.clone().unwrap_or_default(),
            },
        };
        let gate = self.gate(&identity);
        let _guard = match gate.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        // Re-checked inside the gate, which is what makes the second caller a reader rather than a
        // second spawner.
        if let Some(existing) = self.existing_for(&identity, request) {
            if let Some(descriptor) = self.store.descriptor(&existing) {
                return Ok(descriptor);
            }
        }

        let shell_id = ShellId::parse(self.ids.next_shell_id())?;
        let generation = ShellGeneration::new(self.generations.fetch_add(1, Ordering::SeqCst) + 1);
        // Reserved before anything external exists. The count-then-open version of this let every
        // concurrent request see the same free slot and start a process against it.
        let lease = self
            .capacity
            .reserve(&request.session_id, &shell_id, generation)?;

        let title = match &request.title {
            Some(title) => title.clone(),
            None => ShellTitle::parse(format!(
                "Shell {}",
                self.capacity.active_for_session(&request.session_id)
            ))?,
        };
        let now = self.clock.now();
        let descriptor = SessionShellDescriptor {
            shell_id: shell_id.clone(),
            generation,
            session_id: request.session_id.clone(),
            seat_id: request.seat_id.clone(),
            title,
            runtime: ShellRuntimeDescriptor::Native,
            state: SessionShellState::Opening,
            created_at: now.clone(),
            last_activity_at: now,
            revision: 1,
            foreground_process:
                crate::contexts::workspaces::domain::ShellForegroundProcessState::Unknown,
        };
        // Registered before the runtime is invoked, so the reader thread's first byte lands in a
        // Shell that already exists. `Opening` is not writable, so the window this opens is a Shell
        // a view can see and cannot type into — which is the honest description of it.
        self.store.insert(ShellEntry {
            descriptor: descriptor.clone(),
            replay: ShellReplayBuffer::default(),
            attachment: None,
            request_id: request.request_id.clone(),
            last_activity_millis: self.clock.elapsed_millis(),
            capacity: Some(lease),
            close_attempts: 0,
        });

        let open = ShellRuntimeOpen {
            shell_id: shell_id.clone(),
            generation,
            session_id: request.session_id.clone(),
            root: workspace.root.clone(),
            dimensions: TerminalDimensions::bounded(request.rows, request.cols),
            remote: workspace.remote.clone(),
        };
        let opened = match self.runtime.open(&open, self.store.clone()) {
            Ok(opened) => opened,
            Err(error) => {
                self.roll_back_startup(&shell_id, generation);
                return Err(error);
            }
        };
        // Conditional, and that is the fix. A Shell that echoed and exited before this line has
        // already reached a terminal state, and writing `Running` over it would leave a dead
        // process reported as live, with nothing left to end it.
        self.store
            .transition(&shell_id, generation, opened.state.clone());
        self.store
            .set_runtime(&shell_id, generation, opened.runtime.clone());
        let descriptor = self.store.descriptor(&shell_id).unwrap_or(descriptor);
        // Reported after the entry exists, so nothing is announced that a reader could then fail to
        // find. `local` and `remote` are the whole vocabulary: a hostname would make this a
        // location record.
        self.evidence
            .try_publish(WorkspaceEvidenceSignal::ShellOpened {
                session_id: descriptor.session_id.clone(),
                shell_id: descriptor.shell_id.as_str().to_string(),
                seat_id: descriptor.seat_id.clone(),
                runtime: match descriptor.runtime {
                    ShellRuntimeDescriptor::Remote { .. } => WorkspaceShellRuntimeKind::Remote,
                    _ => WorkspaceShellRuntimeKind::Local,
                },
                occurred_at: descriptor.created_at.clone(),
            });
        Ok(descriptor)
    }

    /// Undoes a startup that acquired the registration but not the runtime.
    ///
    /// The runtime's own guard has already ended whatever it managed to acquire; what is left here
    /// is the registration and the slot. Nothing is published: no `ShellOpened` was reported for
    /// this Shell, so a `ShellClosed` would be an ending for something that never began.
    fn roll_back_startup(&self, shell_id: &ShellId, generation: ShellGeneration) {
        let rolled_back = self.store.finalize(
            shell_id,
            generation,
            SessionShellState::Failed {
                reason: shell_reason(shell_reason_code::OPEN_SETUP_FAILED),
            },
        );
        // Dropping the entry drops the lease, which is what returns the slot.
        drop(rolled_back);
    }

    fn gate(&self, identity: &CreateIdentity) -> Arc<Mutex<()>> {
        let mut gates = match self.gates.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Bounded by the same ceiling as the shells themselves plus whatever is in flight; a gate
        // is one empty mutex, and the map is pruned when it outgrows the Shell capacity.
        if gates.len() > self.capacity.capacities().total * 2 {
            gates.retain(|_, gate| Arc::strong_count(gate) > 1);
        }
        gates.entry(identity.clone()).or_default().clone()
    }

    fn existing_for(
        &self,
        identity: &CreateIdentity,
        request: &CreateSessionShellRequest,
    ) -> Option<ShellId> {
        match identity {
            CreateIdentity::Requested(_) => request
                .request_id
                .as_ref()
                .and_then(|id| self.store.by_request(id)),
            CreateIdentity::Default { .. } => self
                .store
                .default_shell(&request.session_id, request.seat_id.as_deref()),
        }
    }

    /// Claims a Shell for one view. Never creates: a Shell that appeared because a view mounted
    /// would be a process nobody asked for.
    pub(crate) fn attach(
        &self,
        request: &AttachSessionShellRequest,
    ) -> Result<ShellAttachSnapshot, SessionShellError> {
        let attachment_id = ShellAttachmentId::parse(self.ids.next_attachment_id())?;
        let (descriptor, snapshot) = self.store.attach(
            &request.shell_id,
            attachment_id.clone(),
            request.after_sequence,
        )?;
        self.store.set_foreground(
            &request.shell_id,
            self.runtime.foreground_process(&request.shell_id),
        );
        Ok(ShellAttachSnapshot {
            attachment_id,
            descriptor: self
                .store
                .descriptor(&request.shell_id)
                .unwrap_or(descriptor),
            replay: snapshot.frames,
            next_sequence: snapshot.next_sequence,
            gap: snapshot.gap,
        })
    }

    /// Idempotent by design, including for an attachment that is no longer current.
    pub(crate) fn detach(&self, scope: &ShellAttachmentScope) -> Result<(), SessionShellError> {
        self.store.detach(&scope.shell_id, &scope.attachment_id);
        Ok(())
    }

    pub(crate) fn write(
        &self,
        request: &WriteSessionShellRequest,
    ) -> Result<(), SessionShellError> {
        self.store
            .authorize(&request.scope.shell_id, &request.scope.attachment_id)?;
        self.runtime
            .write(&request.scope.shell_id, &request.content)?;
        self.store.touch(&request.scope.shell_id);
        Ok(())
    }

    pub(crate) fn resize(
        &self,
        request: &ResizeSessionShellRequest,
    ) -> Result<(), SessionShellError> {
        self.store
            .authorize(&request.scope.shell_id, &request.scope.attachment_id)?;
        self.runtime.resize(
            &request.scope.shell_id,
            TerminalDimensions::bounded(request.rows, request.cols),
        )
    }

    pub(crate) fn rename(
        &self,
        shell_id: &ShellId,
        title: &str,
    ) -> Result<SessionShellDescriptor, SessionShellError> {
        self.store.rename(shell_id, ShellTitle::parse(title)?)
    }

    /// Ends a Shell, and says what that achieved.
    ///
    /// Closing a Shell the registry does not hold is `AlreadyTerminal` rather than an error: a
    /// caller retrying after a partial failure has no way to tell "already gone" from "still there
    /// and refused", and making the first case an error makes cleanup unreliable exactly where it
    /// matters.
    pub(crate) fn close(&self, shell_id: &ShellId) -> SessionShellCloseResult {
        self.close_with(shell_id, WorkspaceShellCloseReason::ExplicitClose)
    }

    /// Why a Shell ended is carried rather than inferred.
    ///
    /// A reader groups by the reason, and a reclaimed Shell, a shut-down one, and one the user
    /// closed are three different facts about a session. Nothing downstream could recover the
    /// difference from the fact that a Shell is gone.
    fn close_with(
        &self,
        shell_id: &ShellId,
        origin: WorkspaceShellCloseReason,
    ) -> SessionShellCloseResult {
        let Some(descriptor) = self.store.descriptor(shell_id) else {
            // Nothing to end and no generation to name. Reported as settled, because a caller
            // retrying a cleanup has to be able to stop.
            return SessionShellCloseResult::already_terminal(
                shell_id.clone(),
                ShellGeneration::new(0),
                SessionShellState::Closed,
            );
        };
        let generation = descriptor.generation;
        let Some(attempt) = self.store.begin_close_attempt(shell_id, generation) else {
            return SessionShellCloseResult::already_terminal(
                shell_id.clone(),
                generation,
                descriptor.state,
            );
        };
        // An ended Shell keeps its entry, its replay and — until the runtime says otherwise — its
        // workers, so closing it is a real operation. Its state stays `Exited`: the process ended
        // by itself, and overwriting that with `Closing` would lose how it ended.
        if !descriptor.state.has_ended() {
            self.store
                .transition(shell_id, generation, SessionShellState::Closing);
        }
        let outcome = self.runtime.close(shell_id, generation, self.budget);
        self.settle(&descriptor, origin, attempt, outcome)
    }

    /// Turns one runtime outcome into the Shell's next state, and finalizes when it can.
    fn settle(
        &self,
        descriptor: &SessionShellDescriptor,
        origin: WorkspaceShellCloseReason,
        attempt: u32,
        outcome: ShellRuntimeCloseOutcome,
    ) -> SessionShellCloseResult {
        let shell_id = &descriptor.shell_id;
        let generation = descriptor.generation;
        match outcome {
            ShellRuntimeCloseOutcome::Confirmed | ShellRuntimeCloseOutcome::NotHeld => {
                self.finalize(shell_id, generation, origin);
                SessionShellCloseResult::confirmed(
                    shell_id.clone(),
                    generation,
                    SessionShellState::Closed,
                    attempt,
                )
            }
            ShellRuntimeCloseOutcome::Retained { reason, retryable } => {
                self.hand_to_reaper(descriptor, origin, attempt, reason, retryable)
            }
        }
    }

    /// Moves an unconfirmed close onto the Reaper, or records why it could not be moved.
    ///
    /// A full queue is not a reason to drop anything. Nothing was ever taken out of an owner to
    /// offer it, so refusing the handoff leaves the runtime holding exactly what it held before,
    /// and the Shell stays `CloseFailed` — addressable, capacity still charged, retryable by hand.
    fn hand_to_reaper(
        &self,
        descriptor: &SessionShellDescriptor,
        origin: WorkspaceShellCloseReason,
        attempt: u32,
        reason: ShellReasonCode,
        retryable: bool,
    ) -> SessionShellCloseResult {
        let shell_id = &descriptor.shell_id;
        let generation = descriptor.generation;
        let queued = retryable
            && matches!(
                self.reaper.offer(
                    shell_id,
                    generation,
                    &descriptor.session_id,
                    origin,
                    attempt,
                    self.clock.elapsed_millis(),
                ),
                Ok(()) | Err(ShellReaperRejection::AlreadyQueued)
            );
        if queued {
            self.store
                .transition(shell_id, generation, SessionShellState::Reaping);
            return SessionShellCloseResult::reaping(shell_id.clone(), generation, reason, attempt);
        }
        let recorded = if retryable {
            shell_reason(shell_reason_code::REAPER_CAPACITY_EXHAUSTED)
        } else {
            reason.clone()
        };
        self.store.transition(
            shell_id,
            generation,
            SessionShellState::CloseFailed {
                reason: recorded.clone(),
                retryable,
            },
        );
        SessionShellCloseResult::failed(
            shell_id.clone(),
            generation,
            recorded,
            retryable,
            attempt,
            true,
        )
    }

    /// The one place a Shell becomes terminal.
    ///
    /// Ordering matters and is the whole method: the terminal state is written and published while
    /// the entry still exists, the entry is then given up, dropping it releases the capacity lease,
    /// and only then is the ending reported to the evidence journal. Publishing before the compare
    /// would announce the end of a Shell a newer generation had already replaced.
    fn finalize(
        &self,
        shell_id: &ShellId,
        generation: ShellGeneration,
        origin: WorkspaceShellCloseReason,
    ) {
        self.reaper.forget(shell_id);
        let Some(entry) = self
            .store
            .finalize(shell_id, generation, SessionShellState::Closed)
        else {
            // A newer generation, or an already-finalized one. Publishing anything here would be
            // an ending attributed to whatever now answers to this id.
            return;
        };
        let descriptor = entry.descriptor.clone();
        drop(entry);
        self.evidence
            .try_publish(WorkspaceEvidenceSignal::ShellClosed {
                session_id: descriptor.session_id,
                shell_id: shell_id.as_str().to_string(),
                seat_id: descriptor.seat_id,
                reason: origin,
                occurred_at: self.clock.now(),
            });
    }

    /// Makes one more bounded attempt at each Shell whose cleanup is due, and returns what happened.
    ///
    /// Driven by whoever already runs a periodic sweep rather than by threads of its own: the
    /// number of attempts in flight is then the drain limit, which is a number somebody chose,
    /// rather than the number of stuck shells, which is not.
    pub(crate) fn advance_reaper(&self) -> SessionShellCleanupReport {
        let mut report = SessionShellCleanupReport::default();
        for item in self.reaper.drain_due(self.clock.elapsed_millis()) {
            let Some(descriptor) = self.store.descriptor(&item.shell_id) else {
                // The entry is gone, so there is nothing to finalize and no slot to return. Recorded
                // because a queue item outliving its Shell is not a case anybody designed for.
                self.diagnostics
                    .orphaned_reaper_completion(item.shell_id.as_str(), item.generation.value());
                continue;
            };
            if descriptor.generation != item.generation {
                // A completion for a superseded attempt. Dropped with no effect: releasing here
                // would return a slot the current generation is using. Recorded, because a correct
                // no-op leaves nothing behind — and if this ever fires for a reason nobody
                // predicted, this line is the only thing that will say so.
                self.diagnostics.stale_reaper_completion(
                    item.shell_id.as_str(),
                    item.generation.value(),
                    descriptor.generation.value(),
                );
                continue;
            }
            let outcome = self
                .runtime
                .close(&item.shell_id, item.generation, self.budget);
            if outcome.is_released() {
                self.finalize(&item.shell_id, item.generation, item.origin);
                report.push(SessionShellCloseResult::confirmed(
                    item.shell_id.clone(),
                    item.generation,
                    SessionShellState::Closed,
                    item.attempts,
                ));
                continue;
            }
            let ShellRuntimeCloseOutcome::Retained { reason, retryable } = outcome else {
                continue;
            };
            let attempts = item.attempts;
            if self
                .reaper
                .requeue(item.clone(), self.clock.elapsed_millis())
            {
                report.push(SessionShellCloseResult::reaping(
                    item.shell_id.clone(),
                    item.generation,
                    reason,
                    attempts,
                ));
                continue;
            }
            // Out of automatic attempts. Left `CloseFailed` with its ownership intact rather than
            // forgotten, because the alternative is a live process nobody is accountable for.
            self.store.transition(
                &item.shell_id,
                item.generation,
                SessionShellState::CloseFailed {
                    reason: reason.clone(),
                    retryable,
                },
            );
            report.push(SessionShellCloseResult::failed(
                item.shell_id.clone(),
                item.generation,
                reason,
                retryable,
                attempts,
                true,
            ));
        }
        report
    }

    /// Reclaims detached, quiet Shells. Bounded per sweep and never a Shell someone is watching.
    ///
    /// A Shell that could not be confirmed closed is reported as reaping or failed rather than
    /// counted as reclaimed: counting it would make the sweep's own figures the first place the
    /// application lies about a process it did not end.
    pub(crate) fn sweep_idle(&self) -> SessionShellCleanupReport {
        let mut report = self.advance_reaper();
        for shell_id in self
            .store
            .idle_candidates(SHELL_IDLE_MILLIS, SHELL_IDLE_SWEEP_LIMIT)
        {
            report.push(self.close_with(&shell_id, WorkspaceShellCloseReason::IdleCleanup));
        }
        report
    }

    /// Closes everything at shutdown, inside one global finite budget.
    ///
    /// Every Shell gets its bounded attempt, then the Reaper is advanced once for whatever is left.
    /// What remains after that is reported rather than waited on: an exit path that blocks until
    /// every child dies is an application that cannot be closed.
    pub(crate) fn shutdown(&self) -> SessionShellCleanupReport {
        let mut report = SessionShellCleanupReport::default();
        for shell_id in self.store.all_shell_ids() {
            report.push(self.close_with(&shell_id, WorkspaceShellCloseReason::Shutdown));
        }
        for result in self.advance_reaper().entries() {
            report.push(result.clone());
        }
        report
    }

    /// Ends every Shell a session owns, and reports each one.
    pub(crate) fn close_for_session(&self, session_id: &str) -> SessionShellCleanupReport {
        let mut report = SessionShellCleanupReport::default();
        for descriptor in self.store.descriptors(Some(session_id)) {
            report.push(self.close_with(
                &descriptor.shell_id,
                WorkspaceShellCloseReason::ExplicitClose,
            ));
        }
        report
    }

    /// The Shells a session is showing, for the workspace summary. Owned here because the count is
    /// a property of the registry, and asking a panel to mount a list to produce a badge is how a
    /// badge becomes a request.
    pub(crate) fn live_count(&self, session_id: &str) -> usize {
        self.store
            .descriptors(Some(session_id))
            .into_iter()
            .filter(|descriptor| !descriptor.state.is_terminal())
            .count()
    }
}
