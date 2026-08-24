//! The retained Session Shell use cases.
//!
//! Every operation here reads or writes the store and then talks to the runtime, in that order and
//! never with the store's lock still held. A registry that held its map across a PTY open would
//! stall every other Shell in the application behind one slow SSH handshake.

use super::session_shell::ShellOutputSink;
use super::session_shell::{
    AttachSessionShellRequest, CreateSessionShellRequest, ResizeSessionShellRequest,
    SessionShellDescriptor, SessionShellRuntimePort, SessionShellWorkspacePort,
    ShellAttachSnapshot, ShellAttachmentScope, ShellCapacities, ShellClockPort, ShellIdPort,
    ShellRuntimeOpen, WriteSessionShellRequest,
};
use super::session_shell_store::{ShellEntry, ShellStore};
use crate::contexts::workspaces::domain::{
    SessionShellError, SessionShellState, ShellAttachmentId, ShellCapacityScope, ShellId,
    ShellReplayBuffer, ShellTitle, TerminalDimensions,
};
use std::collections::BTreeMap;
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
    capacities: ShellCapacities,
    /// One gate per in-flight create identity.
    ///
    /// Held across the runtime open — which is exactly why it is not the store's lock. Two threads
    /// asking for the same default Shell serialize here, the loser re-checks, and one process is
    /// spawned. Creates for different identities never meet.
    gates: Mutex<BTreeMap<CreateIdentity, Arc<Mutex<()>>>>,
}

impl SessionShellRegistry {
    pub(crate) fn new(
        store: Arc<ShellStore>,
        runtime: Arc<dyn SessionShellRuntimePort>,
        workspaces: Arc<dyn SessionShellWorkspacePort>,
        ids: Arc<dyn ShellIdPort>,
        clock: Arc<dyn ShellClockPort>,
        capacities: ShellCapacities,
    ) -> Self {
        Self {
            store,
            runtime,
            workspaces,
            ids,
            clock,
            capacities,
            gates: Mutex::new(BTreeMap::new()),
        }
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
        let workspace = self.workspaces.resolve(&request.session_id)?;
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

        if self.store.count(None) >= self.capacities.total {
            return Err(SessionShellError::CapacityReached {
                scope: ShellCapacityScope::Application,
            });
        }
        if self.store.count(Some(&request.session_id)) >= self.capacities.per_session {
            return Err(SessionShellError::CapacityReached {
                scope: ShellCapacityScope::Session,
            });
        }

        let shell_id = ShellId::parse(self.ids.next_shell_id())?;
        let title = match &request.title {
            Some(title) => title.clone(),
            None => ShellTitle::parse(format!(
                "Shell {}",
                self.store.count(Some(&request.session_id)) + 1
            ))?,
        };
        let open = ShellRuntimeOpen {
            shell_id: shell_id.clone(),
            session_id: request.session_id.clone(),
            root: workspace.root.clone(),
            dimensions: TerminalDimensions::bounded(request.rows, request.cols),
            remote: workspace.remote.clone(),
        };
        // Opened before the entry exists, so a failed open leaves nothing behind. An entry written
        // first would be a Shell the registry lists, the user can attach to, and nothing is running.
        let opened = self.runtime.open(&open, self.store.clone())?;
        let now = self.clock.now();
        let descriptor = SessionShellDescriptor {
            shell_id,
            session_id: request.session_id.clone(),
            seat_id: request.seat_id.clone(),
            title,
            runtime: opened.runtime,
            state: opened.state,
            created_at: now.clone(),
            last_activity_at: now,
            revision: 1,
            foreground_process:
                crate::contexts::workspaces::domain::ShellForegroundProcessState::Unknown,
        };
        self.store.insert(ShellEntry {
            descriptor: descriptor.clone(),
            replay: ShellReplayBuffer::default(),
            attachment: None,
            request_id: request.request_id.clone(),
            last_activity_millis: self.clock.elapsed_millis(),
        });
        Ok(descriptor)
    }

    fn gate(&self, identity: &CreateIdentity) -> Arc<Mutex<()>> {
        let mut gates = match self.gates.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        // Bounded by the same ceiling as the shells themselves plus whatever is in flight; a gate
        // is one empty mutex, and the map is pruned when it outgrows the Shell capacity.
        if gates.len() > self.capacities.total * 2 {
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

    /// Closing a Shell the registry does not hold is a success.
    ///
    /// A close that failed on an unknown Shell would make cleanup unreliable exactly where it
    /// matters: a caller retrying after a partial failure has no way to tell "already gone" from
    /// "still there and refused".
    pub(crate) fn close(&self, shell_id: &ShellId) -> Result<(), SessionShellError> {
        if !self.store.contains(shell_id) {
            return Ok(());
        }
        // Announced before the entry goes, because a subscriber that learns nothing keeps showing a
        // live view of a process that has ended.
        self.store.on_state(shell_id, SessionShellState::Closed);
        if let Some(mut entry) = self.store.remove(shell_id) {
            entry.replay.release();
        }
        self.runtime.close(shell_id)
    }

    /// Reclaims detached, quiet Shells. Bounded per sweep and never a Shell someone is watching.
    pub(crate) fn sweep_idle(&self) -> Vec<ShellId> {
        let candidates = self
            .store
            .idle_candidates(SHELL_IDLE_MILLIS, SHELL_IDLE_SWEEP_LIMIT);
        for shell_id in &candidates {
            let _ = self.close(shell_id);
        }
        candidates
    }

    /// Closes everything at shutdown, joining each runtime's workers through `close`.
    pub(crate) fn shutdown(&self) {
        for shell_id in self.store.all_shell_ids() {
            let _ = self.close(&shell_id);
        }
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
