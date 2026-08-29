//! The retained Shell state, and the sink the runtime writes into.
//!
//! Split from the registry because these two have different callers. The registry is driven by user
//! intent on a command thread; the store is driven by PTY reads on a worker thread, and it is the
//! only thing both of them touch. Keeping it separate is what makes "never hold the map lock across
//! runtime IO" checkable by reading one file.

use super::session_shell::{
    SessionShellDescriptor, SessionShellNotice, SessionShellNoticePort, ShellClockPort,
    ShellOutputSink,
};
use crate::contexts::workspaces::domain::{
    SessionShellError, SessionShellState, ShellAttachmentId, ShellCreateRequestId,
    ShellForegroundProcessState, ShellId, ShellReplayBuffer, ShellReplaySnapshot, ShellStream,
    ShellTitle,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

pub(crate) struct ShellEntry {
    pub(crate) descriptor: SessionShellDescriptor,
    pub(crate) replay: ShellReplayBuffer,
    /// At most one. A second view attaching replaces the first, and the first's later detach is a
    /// no-op rather than a teardown of its successor.
    pub(crate) attachment: Option<ShellAttachmentId>,
    /// The create request that produced it, so a retried Add returns this Shell instead of a second.
    pub(crate) request_id: Option<ShellCreateRequestId>,
    /// Monotonic milliseconds at the last activity, for idle arithmetic a clock change cannot move.
    pub(crate) last_activity_millis: u64,
}

/// Everything the registry and the runtime worker share.
pub(crate) struct ShellStore {
    shells: Mutex<BTreeMap<ShellId, ShellEntry>>,
    notices: Arc<dyn SessionShellNoticePort>,
    clock: Arc<dyn ShellClockPort>,
}

impl ShellStore {
    pub(crate) fn new(
        notices: Arc<dyn SessionShellNoticePort>,
        clock: Arc<dyn ShellClockPort>,
    ) -> Self {
        Self {
            shells: Mutex::new(BTreeMap::new()),
            notices,
            clock,
        }
    }

    /// A poisoned lock must not take the registry down with it: a Shell that cannot be listed is
    /// still a Shell that has to be closable at shutdown.
    fn lock(&self) -> std::sync::MutexGuard<'_, BTreeMap<ShellId, ShellEntry>> {
        match self.shells.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub(crate) fn insert(&self, entry: ShellEntry) {
        self.lock().insert(entry.descriptor.shell_id.clone(), entry);
    }

    pub(crate) fn remove(&self, shell_id: &ShellId) -> Option<ShellEntry> {
        self.lock().remove(shell_id)
    }

    pub(crate) fn contains(&self, shell_id: &ShellId) -> bool {
        self.lock().contains_key(shell_id)
    }

    pub(crate) fn descriptor(&self, shell_id: &ShellId) -> Option<SessionShellDescriptor> {
        self.lock()
            .get(shell_id)
            .map(|entry| entry.descriptor.clone())
    }

    pub(crate) fn descriptors(&self, session_id: Option<&str>) -> Vec<SessionShellDescriptor> {
        self.lock()
            .values()
            .filter(|entry| session_id.is_none_or(|id| entry.descriptor.session_id == id))
            .map(|entry| entry.descriptor.clone())
            .collect()
    }

    pub(crate) fn count(&self, session_id: Option<&str>) -> usize {
        self.lock()
            .values()
            .filter(|entry| session_id.is_none_or(|id| entry.descriptor.session_id == id))
            .count()
    }

    /// The Shell a create request already produced, if it is still held.
    pub(crate) fn by_request(&self, request_id: &ShellCreateRequestId) -> Option<ShellId> {
        self.lock()
            .values()
            .find(|entry| entry.request_id.as_ref() == Some(request_id))
            .map(|entry| entry.descriptor.shell_id.clone())
    }

    /// The default Shell for a session and seat, if one is already open.
    pub(crate) fn default_shell(&self, session_id: &str, seat_id: Option<&str>) -> Option<ShellId> {
        self.lock()
            .values()
            .find(|entry| {
                entry.request_id.is_none()
                    && entry.descriptor.session_id == session_id
                    && entry.descriptor.seat_id.as_deref() == seat_id
                    && !entry.descriptor.state.is_terminal()
            })
            .map(|entry| entry.descriptor.shell_id.clone())
    }

    /// Claims the Shell for one view and returns its replay.
    ///
    /// The previous attachment is displaced rather than refused: a view that never got to run its
    /// cleanup — a crashed render, a killed tab — must not lock the Shell out of reach forever.
    pub(crate) fn attach(
        &self,
        shell_id: &ShellId,
        attachment_id: ShellAttachmentId,
        after_sequence: u64,
    ) -> Result<(SessionShellDescriptor, ShellReplaySnapshot), SessionShellError> {
        let mut shells = self.lock();
        let entry = shells
            .get_mut(shell_id)
            .ok_or(SessionShellError::NotFound)?;
        entry.attachment = Some(attachment_id);
        Ok((
            entry.descriptor.clone(),
            entry.replay.snapshot(after_sequence),
        ))
    }

    /// Releases the claim, but only if it is still the current one.
    ///
    /// Returns whether anything changed. A stale detach answers `false` and leaves the newer
    /// attachment alone, which is what makes a late React cleanup harmless.
    pub(crate) fn detach(&self, shell_id: &ShellId, attachment_id: &ShellAttachmentId) -> bool {
        let mut shells = self.lock();
        let Some(entry) = shells.get_mut(shell_id) else {
            return false;
        };
        if entry.attachment.as_ref() != Some(attachment_id) {
            return false;
        }
        entry.attachment = None;
        true
    }

    /// Checks a write or resize before it reaches the runtime.
    ///
    /// Stale here is refused rather than ignored: input is not idempotent, and delivering a
    /// keystroke from a view the user has left would run it in the session they are looking at now.
    pub(crate) fn authorize(
        &self,
        shell_id: &ShellId,
        attachment_id: &ShellAttachmentId,
    ) -> Result<(), SessionShellError> {
        let shells = self.lock();
        let entry = shells.get(shell_id).ok_or(SessionShellError::NotFound)?;
        if entry.attachment.as_ref() != Some(attachment_id) {
            return Err(SessionShellError::AttachmentStale);
        }
        if !entry.descriptor.state.accepts_input() {
            return Err(SessionShellError::NotAcceptingInput {
                state: entry.descriptor.state.token(),
            });
        }
        Ok(())
    }

    pub(crate) fn rename(
        &self,
        shell_id: &ShellId,
        title: ShellTitle,
    ) -> Result<SessionShellDescriptor, SessionShellError> {
        let mut shells = self.lock();
        let entry = shells
            .get_mut(shell_id)
            .ok_or(SessionShellError::NotFound)?;
        entry.descriptor.title = title;
        entry.descriptor.revision = entry.descriptor.revision.saturating_add(1);
        Ok(entry.descriptor.clone())
    }

    pub(crate) fn set_foreground(&self, shell_id: &ShellId, state: ShellForegroundProcessState) {
        if let Some(entry) = self.lock().get_mut(shell_id) {
            entry.descriptor.foreground_process = state;
        }
    }

    pub(crate) fn touch(&self, shell_id: &ShellId) {
        let now = self.clock.now();
        let millis = self.clock.elapsed_millis();
        if let Some(entry) = self.lock().get_mut(shell_id) {
            entry.descriptor.last_activity_at = now;
            entry.last_activity_millis = millis;
        }
    }

    /// Shells detached and quiet for longer than the window, oldest first and bounded per sweep.
    ///
    /// An attached Shell is never idle by definition — someone is looking at it — and a Shell with
    /// known foreground work is not idle either, because reclaiming it would kill a running job to
    /// save memory.
    pub(crate) fn idle_candidates(&self, idle_millis: u64, limit: usize) -> Vec<ShellId> {
        let now = self.clock.elapsed_millis();
        let mut candidates = self
            .lock()
            .values()
            .filter(|entry| {
                entry.attachment.is_none()
                    && entry.descriptor.foreground_process != ShellForegroundProcessState::Present
                    && now.saturating_sub(entry.last_activity_millis) >= idle_millis
            })
            .map(|entry| {
                (
                    entry.last_activity_millis,
                    entry.descriptor.shell_id.clone(),
                )
            })
            .collect::<Vec<_>>();
        candidates.sort();
        candidates
            .into_iter()
            .take(limit)
            .map(|(_, shell_id)| shell_id)
            .collect()
    }

    pub(crate) fn all_shell_ids(&self) -> Vec<ShellId> {
        self.lock().keys().cloned().collect()
    }
}

impl ShellOutputSink for ShellStore {
    /// Records first, notifies second.
    ///
    /// The retained buffer is the durable half and the notice is the hint. A notice published
    /// before the frame was stored could reach a subscriber that then attaches and does not find
    /// it — and the subscriber would read that as a gap, which is a lie about lost output.
    fn on_output(&self, shell_id: &ShellId, stream: ShellStream, bytes: &[u8]) {
        let occurred_at = self.clock.now();
        let millis = self.clock.elapsed_millis();
        let published = {
            let mut shells = self.lock();
            let Some(entry) = shells.get_mut(shell_id) else {
                return;
            };
            entry.descriptor.last_activity_at = occurred_at.clone();
            entry.last_activity_millis = millis;
            entry
                .replay
                .push_bytes(stream, &occurred_at, bytes)
                .map(|frame| (entry.descriptor.session_id.clone(), frame))
        };
        if let Some((session_id, frame)) = published {
            self.notices.publish(SessionShellNotice::Output {
                shell_id: shell_id.clone(),
                session_id,
                frame,
            });
        }
    }

    fn on_state(&self, shell_id: &ShellId, state: SessionShellState) {
        let occurred_at = self.clock.now();
        let published = {
            let mut shells = self.lock();
            let Some(entry) = shells.get_mut(shell_id) else {
                return;
            };
            // A Shell that already ended does not end again. Two terminal states would publish two
            // closings for one Shell, and a reader counting them would see work that never happened.
            if entry.descriptor.state == state || entry.descriptor.state.has_ended() {
                return;
            }
            entry.descriptor.state = state.clone();
            entry.descriptor.revision = entry.descriptor.revision.saturating_add(1);
            entry.descriptor.last_activity_at = occurred_at.clone();
            Some((
                entry.descriptor.session_id.clone(),
                entry.descriptor.revision,
            ))
        };
        if let Some((session_id, revision)) = published {
            self.notices.publish(SessionShellNotice::State {
                shell_id: shell_id.clone(),
                session_id,
                state,
                revision,
                occurred_at,
            });
        }
    }
}
