//! Turning a burst of observations into notices somebody can act on.
//!
//! An agent rewriting a package touches forty files in under a second. Forty notices become forty
//! refetches, each one re-reading a directory that the next write is about to invalidate again, and
//! the console spends the burst rendering intermediate states nobody asked to see. So observations
//! are buffered for a short window and published once per distinct thing that changed.
//!
//! The buffer is bounded, and what happens at the bound is the part that matters. Dropping the
//! excess would leave the console showing content it has no way to know is stale — the worst outcome
//! available, because it looks exactly like a workspace where nothing happened. Instead the buffer
//! collapses: past the bound the whole session's pending set becomes one `Workspace` notice that
//! says how many observations it stands in for. Expensive to act on, and honest.
//!
//! This module also holds which directories a session currently has open, because the watcher and
//! the poller both need it and neither should own it. Entries expire rather than being unregistered,
//! so a console that closed, crashed, or was simply hidden stops being watched on its own. A
//! registry that relied on a matching "stop" call would keep watching for any client that never
//! sent one, and the clients most likely to skip it are the ones that died.

use super::invalidation::{
    WorkspaceInvalidationChange, WorkspaceInvalidationNotice, WorkspaceInvalidationPublisher,
    WorkspaceInvalidationScope, WorkspaceInvalidationSource,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// How long observations accumulate before they are published.
///
/// Short enough that a tree refresh still feels like a consequence of the write, long enough that a
/// multi-file edit arrives as one refresh.
pub(crate) const COALESCE_WINDOW_MS: u64 = 250;

/// How many distinct things one session may have pending before the set collapses.
///
/// A ceiling on memory, but mostly a ceiling on the consumer's work: past a few dozen targeted
/// refreshes, refetching everything once is cheaper than refetching each one, and far cheaper than
/// the consumer discovering that for itself.
pub(crate) const MAX_PENDING_SCOPES: usize = 64;

/// How long a directory stays observed without being read again.
///
/// Long enough to survive a slow render or a user reading the screen, short enough that a console
/// that went away stops costing a watch registration within a minute.
pub(crate) const OBSERVATION_TTL_MS: u64 = 60_000;

/// How many directories one session may have observed at once.
///
/// The watcher takes an operating-system handle per entry, and this is the number that decides how
/// many. Bounded here rather than at the watcher so the limit is the same whichever source is
/// reading the registry.
pub(crate) const MAX_OBSERVED_DIRECTORIES: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PendingKey {
    Path(String),
    Directory(String),
    Workspace,
}

#[derive(Debug, Clone)]
struct PendingEntry {
    source: WorkspaceInvalidationSource,
    change: WorkspaceInvalidationChange,
    first_seen_ms: u64,
    /// Observations beyond the first for this same key.
    extra: u32,
}

#[derive(Debug, Default)]
struct SessionState {
    sequence: u64,
    pending: BTreeMap<PendingKey, PendingEntry>,
    /// Set once the pending set collapsed. Everything after it folds into the same notice.
    collapsed: bool,
    window_opened_ms: Option<u64>,
    /// Relative directory path to the last time it was read.
    observed: BTreeMap<String, u64>,
}

/// Buffers observations, publishes notices.
///
/// Synchronous and clock-free by construction: every entry point takes the current time, so the
/// coalescing window and the observation lifetime are both testable without waiting for either.
pub(crate) struct WorkspaceInvalidationDispatcher {
    publisher: Arc<dyn WorkspaceInvalidationPublisher>,
    sessions: Mutex<BTreeMap<String, SessionState>>,
}

impl WorkspaceInvalidationDispatcher {
    pub(crate) fn new(publisher: Arc<dyn WorkspaceInvalidationPublisher>) -> Self {
        Self {
            publisher,
            sessions: Mutex::new(BTreeMap::new()),
        }
    }

    /// Records one observation. Publishes nothing: the window has to close first.
    pub(crate) fn observe(
        &self,
        session_id: &str,
        source: WorkspaceInvalidationSource,
        scope: WorkspaceInvalidationScope,
        now_ms: u64,
    ) {
        let Ok(mut sessions) = self.sessions.lock() else {
            // A poisoned lock means another thread panicked mid-update. Losing this observation is
            // the smaller harm; the alternative is propagating a panic into whatever was writing a
            // file, which is not this notice's business.
            return;
        };
        let state = sessions.entry(session_id.to_string()).or_default();
        state.window_opened_ms.get_or_insert(now_ms);

        if state.collapsed {
            if let Some(entry) = state.pending.get_mut(&PendingKey::Workspace) {
                entry.extra = entry.extra.saturating_add(1);
                entry.source = weaker(entry.source, source);
            }
            return;
        }

        let (key, change) = match scope {
            WorkspaceInvalidationScope::Path {
                relative_path,
                change,
            } => (PendingKey::Path(relative_path), change),
            WorkspaceInvalidationScope::Directory { relative_path } => (
                PendingKey::Directory(relative_path),
                WorkspaceInvalidationChange::Unknown,
            ),
            WorkspaceInvalidationScope::Workspace => {
                (PendingKey::Workspace, WorkspaceInvalidationChange::Unknown)
            }
        };

        match state.pending.get_mut(&key) {
            Some(entry) => {
                entry.extra = entry.extra.saturating_add(1);
                entry.source = weaker(entry.source, source);
                if entry.change != change {
                    // Two different things happened to one path inside one window. Naming either of
                    // them would assert an order nobody observed; `Unknown` is what the consumer
                    // needs anyway, since it refreshes both the entry and its parent.
                    entry.change = WorkspaceInvalidationChange::Unknown;
                }
            }
            None => {
                state.pending.insert(
                    key,
                    PendingEntry {
                        source,
                        change,
                        first_seen_ms: now_ms,
                        extra: 0,
                    },
                );
                if state.pending.len() > MAX_PENDING_SCOPES {
                    collapse(state, now_ms);
                }
            }
        }
    }

    /// Publishes every session whose coalescing window has closed.
    ///
    /// Returns how many notices went out, which is what tells a driver loop whether it still has
    /// work rather than the loop guessing from a timer.
    pub(crate) fn flush_due(&self, now_ms: u64) -> usize {
        let ready = {
            let Ok(mut sessions) = self.sessions.lock() else {
                return 0;
            };
            let due: Vec<String> = sessions
                .iter()
                .filter(|(_, state)| {
                    state
                        .window_opened_ms
                        .is_some_and(|opened| now_ms.saturating_sub(opened) >= COALESCE_WINDOW_MS)
                })
                .map(|(session_id, _)| session_id.clone())
                .collect();
            due.into_iter()
                .filter_map(|session_id| {
                    let state = sessions.get_mut(&session_id)?;
                    let notices = drain(&session_id, state);
                    (!notices.is_empty()).then_some(notices)
                })
                .flatten()
                .collect::<Vec<_>>()
        };
        // Published outside the lock: a publisher reaches the event channel, and holding the
        // dispatcher's lock across that would let a slow consumer block the thread doing the write.
        for notice in &ready {
            self.publisher.publish(notice);
        }
        ready.len()
    }

    /// Records that a session is currently looking at a directory.
    ///
    /// Called from the read itself rather than from a subscribe command, so the registry describes
    /// what a console actually asked for. A separate registration call would be a second thing that
    /// can disagree with the first.
    pub(crate) fn note_directory_read(&self, session_id: &str, relative_path: &str, now_ms: u64) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        let state = sessions.entry(session_id.to_string()).or_default();
        if state.observed.len() >= MAX_OBSERVED_DIRECTORIES
            && !state.observed.contains_key(relative_path)
        {
            // Drop the least recently read rather than refusing the new one: the directory being
            // read now is the one on screen, and refusing it would stop watching exactly where the
            // user is looking.
            if let Some(oldest) = state
                .observed
                .iter()
                .min_by_key(|(_, read_at)| **read_at)
                .map(|(path, _)| path.clone())
            {
                state.observed.remove(&oldest);
            }
        }
        state.observed.insert(relative_path.to_string(), now_ms);
    }

    /// The directories worth watching or polling for a session, freshest read first.
    pub(crate) fn observed_directories(&self, session_id: &str, now_ms: u64) -> Vec<String> {
        let Ok(sessions) = self.sessions.lock() else {
            return Vec::new();
        };
        let Some(state) = sessions.get(session_id) else {
            return Vec::new();
        };
        let mut live: Vec<(&String, &u64)> = state
            .observed
            .iter()
            .filter(|(_, read_at)| now_ms.saturating_sub(**read_at) < OBSERVATION_TTL_MS)
            .collect();
        live.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
        live.into_iter().map(|(path, _)| path.clone()).collect()
    }

    /// Every session with a live observation, so a driver knows who to poll.
    pub(crate) fn observed_sessions(&self, now_ms: u64) -> Vec<String> {
        let Ok(sessions) = self.sessions.lock() else {
            return Vec::new();
        };
        sessions
            .iter()
            .filter(|(_, state)| {
                state
                    .observed
                    .values()
                    .any(|read_at| now_ms.saturating_sub(*read_at) < OBSERVATION_TTL_MS)
            })
            .map(|(session_id, _)| session_id.clone())
            .collect()
    }

    /// Forgets expired observations and sessions with nothing left.
    ///
    /// Returns how many sessions still have something worth doing, which is the driver loop's
    /// condition for staying alive.
    pub(crate) fn expire(&self, now_ms: u64) -> usize {
        let Ok(mut sessions) = self.sessions.lock() else {
            return 0;
        };
        for state in sessions.values_mut() {
            state
                .observed
                .retain(|_, read_at| now_ms.saturating_sub(*read_at) < OBSERVATION_TTL_MS);
        }
        // A session that has published keeps its counter even with nothing else left. Dropping it
        // would restart the sequence at 1 the next time that session changed, and a consumer
        // watching for gaps cannot tell a restarted counter from a replay.
        sessions.retain(|_, state| {
            !state.observed.is_empty() || !state.pending.is_empty() || state.sequence > 0
        });
        sessions
            .values()
            .filter(|state| !state.observed.is_empty() || !state.pending.is_empty())
            .count()
    }
}

/// Which of two sources makes the weaker claim about freshness.
///
/// A notice standing in for several observations can only promise what its least immediate
/// contributor promised. A poll's answer is true as of the poll; folding it together with a
/// watcher's and calling the result "seen as it happened" would overstate the one that matters.
fn weaker(
    left: WorkspaceInvalidationSource,
    right: WorkspaceInvalidationSource,
) -> WorkspaceInvalidationSource {
    if staleness_rank(left) >= staleness_rank(right) {
        left
    } else {
        right
    }
}

fn staleness_rank(source: WorkspaceInvalidationSource) -> u8 {
    match source {
        WorkspaceInvalidationSource::Poll => 2,
        WorkspaceInvalidationSource::Watch => 1,
        WorkspaceInvalidationSource::ExecutionEvidence => 0,
    }
}

/// Replaces a session's pending set with the one notice that covers all of it.
fn collapse(state: &mut SessionState, now_ms: u64) {
    let total: u32 = state
        .pending
        .values()
        .map(|entry| entry.extra.saturating_add(1))
        .fold(0u32, |sum, count| sum.saturating_add(count));
    let source = state
        .pending
        .values()
        .map(|entry| entry.source)
        .reduce(weaker)
        .unwrap_or(WorkspaceInvalidationSource::Watch);
    let first_seen_ms = state
        .pending
        .values()
        .map(|entry| entry.first_seen_ms)
        .min()
        .unwrap_or(now_ms);
    state.pending.clear();
    state.pending.insert(
        PendingKey::Workspace,
        PendingEntry {
            source,
            change: WorkspaceInvalidationChange::Unknown,
            first_seen_ms,
            // One notice stands for all of them, so it accounts for all but itself.
            extra: total.saturating_sub(1),
        },
    );
    state.collapsed = true;
}

/// Empties a session's buffer into notices and closes its window.
fn drain(session_id: &str, state: &mut SessionState) -> Vec<WorkspaceInvalidationNotice> {
    state.window_opened_ms = None;
    state.collapsed = false;
    let pending = std::mem::take(&mut state.pending);
    if pending.is_empty() {
        return Vec::new();
    }

    // A pending workspace-wide notice already covers everything more specific, so the specific ones
    // would only add work a consumer has by then already done.
    let entries: Vec<(PendingKey, PendingEntry)> = match pending.get(&PendingKey::Workspace) {
        Some(entry) => vec![(PendingKey::Workspace, entry.clone())],
        None => pending.into_iter().collect(),
    };

    entries
        .into_iter()
        .map(|(key, entry)| {
            state.sequence = state.sequence.saturating_add(1);
            WorkspaceInvalidationNotice {
                session_id: session_id.to_string(),
                source: entry.source,
                scope: match key {
                    PendingKey::Path(relative_path) => WorkspaceInvalidationScope::Path {
                        relative_path,
                        change: entry.change,
                    },
                    PendingKey::Directory(relative_path) => {
                        WorkspaceInvalidationScope::Directory { relative_path }
                    }
                    PendingKey::Workspace => WorkspaceInvalidationScope::Workspace,
                },
                // The earliest contributing observation, because the question it answers is how
                // long the console has been showing something wrong, not when the buffer drained.
                observed_at: rfc3339_from_ms(entry.first_seen_ms),
                sequence: state.sequence,
                coalesced: (entry.extra > 0).then_some(entry.extra),
            }
        })
        .collect()
}

fn rfc3339_from_ms(milliseconds: u64) -> String {
    chrono::DateTime::from_timestamp_millis(milliseconds as i64)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339()
}
