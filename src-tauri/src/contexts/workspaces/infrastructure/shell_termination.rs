//! Termination lifecycle for managed PTY children.
//!
//! Split from `portable_pty` because these are two jobs. That module routes io and owns the
//! shell registry; this one answers a narrower question — what happened when we tried to end a
//! child, and who owns that child afterwards if we could not.
//!
//! The distinction this file exists to keep is between **what the attempt did** and **where the
//! child stands now**. A reap that timed out is a finished attempt with an unfinished child, and
//! collapsing those two into one word is how the previous implementation could report a cleanup
//! that had not cleaned anything up. `TerminationOutcome` is fixed once the owner settles it;
//! `CleanupState` keeps moving afterwards.

use super::super::application::{ShellLog, WorkspaceLogLevel, WorkspaceShellLogPort};
use portable_pty::Child;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long one kill is given to become an observed exit before the child is handed to the
/// pending registry.
pub(super) const REAP_DEADLINE: Duration = Duration::from_secs(5);

/// A single budget shared by *every* shell during shutdown, rather than one per shell. Ten
/// wedged shells must not multiply into ten deadlines.
pub(super) const SHUTDOWN_REAP_DEADLINE: Duration = Duration::from_secs(5);

/// Poll backoff floor and ceilings. Not a fixed sleep: each iteration reads the child's real
/// state through the non-blocking `try_wait`, and the loop ends on a real answer or a real
/// deadline. The interval only decides how often the truth is sampled.
pub(super) const POLL_INTERVAL_FLOOR: Duration = Duration::from_millis(1);
pub(super) const POLL_INTERVAL_CEILING: Duration = Duration::from_millis(25);
/// The exit monitor waits for a *natural* exit, which is legitimately open-ended. It settles at
/// a slower cadence because a quarter second of latency on a "disconnected" event is
/// imperceptible, and it never holds the child lock while waiting.
pub(super) const MONITOR_INTERVAL_CEILING: Duration = Duration::from_millis(250);

/// What one termination attempt did. Fixed once the owner settles it — a later successful reap
/// updates [`CleanupState`] and never rewrites this, because "we timed out and recovered" and
/// "we succeeded" are different histories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TerminationOutcome {
    /// The exit was observed within the deadline.
    Reaped,
    /// The deadline passed with the child still alive.
    ReapTimedOut,
    /// The signal itself was refused and the child had not exited.
    KillFailed,
    /// Probing the child returned an error rather than a status.
    ReapFailed,
}

impl TerminationOutcome {
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::Reaped => "reaped",
            Self::ReapTimedOut => "reap_timed_out",
            Self::KillFailed => "kill_failed",
            Self::ReapFailed => "reap_failed",
        }
    }

    /// Whether this outcome leaves a process that something still has to own.
    fn leaves_a_live_child(self) -> bool {
        matches!(self, Self::ReapTimedOut | Self::KillFailed)
    }

    fn as_state(self) -> u8 {
        SETTLED_BASE
            + match self {
                Self::Reaped => 0,
                Self::ReapTimedOut => 1,
                Self::KillFailed => 2,
                Self::ReapFailed => 3,
            }
    }

    fn from_state(state: u8) -> Option<Self> {
        match state.checked_sub(SETTLED_BASE)? {
            0 => Some(Self::Reaped),
            1 => Some(Self::ReapTimedOut),
            2 => Some(Self::KillFailed),
            3 => Some(Self::ReapFailed),
            _ => None,
        }
    }
}

/// Where the child stands. Unlike the outcome, this keeps moving: a child adopted as `Pending`
/// becomes `ReapedLater` when a sweep finally observes its exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CleanupState {
    /// Nothing is owed — the child was reaped by the attempt itself.
    NotRequired,
    /// An attempt is in flight; the owner has not settled yet.
    Reaping,
    /// The child outlived its attempt and the pending registry now owns it.
    Pending,
    /// A later sweep observed the exit of a child that had been `Pending`.
    ReapedLater,
    /// Shutdown ended with the child still unaccounted for.
    UnresolvedAtShutdown,
}

impl CleanupState {
    pub(super) fn code(self) -> &'static str {
        match self {
            Self::NotRequired => "not_required",
            Self::Reaping => "reaping",
            Self::Pending => "pending",
            Self::ReapedLater => "reaped_later",
            Self::UnresolvedAtShutdown => "unresolved_at_shutdown",
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            Self::NotRequired => 0,
            Self::Reaping => 1,
            Self::Pending => 2,
            Self::ReapedLater => 3,
            Self::UnresolvedAtShutdown => 4,
        }
    }

    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Reaping,
            2 => Self::Pending,
            3 => Self::ReapedLater,
            4 => Self::UnresolvedAtShutdown,
            _ => Self::NotRequired,
        }
    }
}

/// What a caller learns from asking a shell to terminate.
///
/// `outcome` is `None` only for a follower: a reap is in flight and this caller is not the one
/// running it, so there is no outcome yet to report. Reporting someone else's future result as
/// though it were this call's would be the same lie as reporting a timeout as success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TerminationReport {
    pub(super) outcome: Option<TerminationOutcome>,
    pub(super) cleanup: CleanupState,
}

const STATE_IDLE: u8 = 0;
const STATE_IN_FLIGHT: u8 = 1;
/// Settled states carry the outcome itself, offset past the two live states so the ranges can
/// never collide.
const SETTLED_BASE: u8 = 2;

/// Single-flight termination state for one shell.
#[derive(Debug, Default)]
pub(super) struct ShellTermination {
    state: AtomicU8,
    cleanup: AtomicU8,
}

impl ShellTermination {
    /// Claims ownership of the reap. `Ok(())` means this caller is the owner and must run
    /// exactly one kill and one reap loop. `Err(report)` means someone else owns it, or it has
    /// already settled, and the caller returns that report without touching the child.
    pub(super) fn claim(&self) -> Result<(), TerminationReport> {
        match self.state.compare_exchange(
            STATE_IDLE,
            STATE_IN_FLIGHT,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                self.set_cleanup(CleanupState::Reaping);
                Ok(())
            }
            Err(STATE_IN_FLIGHT) => Err(TerminationReport {
                outcome: None,
                cleanup: CleanupState::Reaping,
            }),
            Err(settled) => Err(TerminationReport {
                outcome: TerminationOutcome::from_state(settled),
                cleanup: self.cleanup(),
            }),
        }
    }

    /// Records the owner's result. Called once, by the owner only.
    pub(super) fn settle(&self, outcome: TerminationOutcome, cleanup: CleanupState) {
        self.set_cleanup(cleanup);
        self.state.store(outcome.as_state(), Ordering::Release);
    }

    pub(super) fn set_cleanup(&self, cleanup: CleanupState) {
        self.cleanup.store(cleanup.as_u8(), Ordering::Release);
    }

    pub(super) fn cleanup(&self) -> CleanupState {
        CleanupState::from_u8(self.cleanup.load(Ordering::Acquire))
    }

    pub(super) fn outcome(&self) -> Option<TerminationOutcome> {
        TerminationOutcome::from_state(self.state.load(Ordering::Acquire))
    }

    pub(super) fn is_settled(&self) -> bool {
        self.state.load(Ordering::Acquire) >= SETTLED_BASE
    }
}

/// Set once when the manager begins shutting down. Monitors read it so an open-ended wait for a
/// natural exit still has a way to end.
#[derive(Debug, Default)]
pub(super) struct ShutdownToken {
    signalled: AtomicBool,
}

impl ShutdownToken {
    pub(super) fn signal(&self) {
        self.signalled.store(true, Ordering::Release);
    }

    pub(super) fn is_signalled(&self) -> bool {
        self.signalled.load(Ordering::Acquire)
    }
}

/// One non-blocking look at the child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChildProbe {
    Exited,
    Running,
    Failed,
}

pub(super) fn probe_child(child: &mut dyn Child) -> ChildProbe {
    match child.try_wait() {
        Ok(Some(_)) => ChildProbe::Exited,
        Ok(None) => ChildProbe::Running,
        Err(_) => ChildProbe::Failed,
    }
}

/// Probes a shared child without ever holding its lock across a wait. One `try_wait` per call,
/// so the lock is held for microseconds and a terminate can always acquire it.
pub(super) fn probe_shared_child(child: &Mutex<Box<dyn Child + Send + Sync>>) -> ChildProbe {
    match child.lock() {
        Ok(mut child) => probe_child(&mut **child),
        Err(_) => ChildProbe::Failed,
    }
}

/// Why a poll loop stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PollEnd {
    Exited,
    ProbeFailed,
    DeadlineReached,
    Abandoned,
}

/// Polls until the child exits, the probe fails, the deadline passes, or `keep_waiting` says to
/// stop.
///
/// A `deadline` of `None` is an open-ended wait for a *natural* exit, used only by the exit
/// monitor — which is why `keep_waiting` exists: shutdown and terminate both need a way to end
/// that wait. `park_timeout` rather than `sleep`, so an unpark cuts it short; a spurious wake
/// just re-probes.
pub(super) fn poll_until_exit(
    mut probe: impl FnMut() -> ChildProbe,
    mut keep_waiting: impl FnMut() -> bool,
    deadline: Option<Instant>,
    ceiling: Duration,
) -> PollEnd {
    let mut interval = POLL_INTERVAL_FLOOR;
    loop {
        match probe() {
            ChildProbe::Exited => return PollEnd::Exited,
            ChildProbe::Failed => return PollEnd::ProbeFailed,
            ChildProbe::Running => {}
        }
        if !keep_waiting() {
            return PollEnd::Abandoned;
        }
        let mut wait_for = interval;
        if let Some(deadline) = deadline {
            let now = Instant::now();
            if now >= deadline {
                return PollEnd::DeadlineReached;
            }
            wait_for = interval.min(deadline - now);
        }
        thread::park_timeout(wait_for);
        interval = interval.saturating_mul(2).min(ceiling);
    }
}

/// A child that outlived the attempt to end it.
struct PendingReap {
    session_id: String,
    shell_id: String,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    termination: Arc<ShellTermination>,
}

/// Owns every child whose termination attempt finished without the child doing so.
///
/// This is what closes the lifecycle. Before it existed, a `reap_timed_out` was reported
/// honestly and then the sole handle to the process was dropped at the end of the scope, so the
/// system knew it had failed and had permanently lost the ability to do anything about it. An
/// honest report of a permanent leak is still a permanent leak.
#[derive(Default)]
pub(super) struct PendingReapRegistry {
    entries: Mutex<Vec<PendingReap>>,
}

impl PendingReapRegistry {
    /// Takes ownership of a child before the caller releases its `ManagedShell`.
    pub(super) fn adopt(
        &self,
        session_id: &str,
        shell_id: &str,
        child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
        termination: Arc<ShellTermination>,
    ) {
        termination.set_cleanup(CleanupState::Pending);
        if let Ok(mut entries) = self.entries.lock() {
            entries.push(PendingReap {
                session_id: session_id.to_string(),
                shell_id: shell_id.to_string(),
                child,
                termination,
            });
        }
    }

    /// One non-blocking pass over everything still owed. `try_wait` only — a blocking wait here
    /// would recreate, in the cleaner, the exact hang the cleaner exists to recover from.
    ///
    /// A child observed to have exited moves to `ReapedLater`. The original outcome is left
    /// alone: a timeout that recovered is not the same history as a clean kill.
    pub(super) fn sweep(&self, logging: &dyn WorkspaceShellLogPort) -> usize {
        let Ok(mut entries) = self.entries.lock() else {
            return 0;
        };
        let mut resolved = 0;
        entries.retain(|entry| match probe_shared_child(&entry.child) {
            ChildProbe::Running => true,
            probe => {
                entry.termination.set_cleanup(CleanupState::ReapedLater);
                resolved += 1;
                write_shell_log(
                    logging,
                    WorkspaceLogLevel::Info,
                    &entry.session_id,
                    &entry.shell_id,
                    &format!(
                        "Shell process pending since {} was reclaimed (cleanup: {}{}).",
                        entry
                            .termination
                            .outcome()
                            .map(TerminationOutcome::code)
                            .unwrap_or("unknown"),
                        CleanupState::ReapedLater.code(),
                        if probe == ChildProbe::Failed {
                            ", probe error treated as gone"
                        } else {
                            ""
                        }
                    ),
                );
                false
            }
        });
        resolved
    }

    /// Marks everything still owed as unresolved, at the end of shutdown.
    pub(super) fn mark_unresolved(&self, logging: &dyn WorkspaceShellLogPort) -> usize {
        let Ok(mut entries) = self.entries.lock() else {
            return 0;
        };
        let unresolved = entries.len();
        for entry in entries.drain(..) {
            entry
                .termination
                .set_cleanup(CleanupState::UnresolvedAtShutdown);
            write_shell_log(
                logging,
                WorkspaceLogLevel::Warn,
                &entry.session_id,
                &entry.shell_id,
                &format!(
                    "Shell process was still unreaped when the runtime shut down (cleanup: {}).",
                    CleanupState::UnresolvedAtShutdown.code()
                ),
            );
        }
        unresolved
    }

    pub(super) fn len(&self) -> usize {
        self.entries
            .lock()
            .map(|entries| entries.len())
            .unwrap_or(0)
    }
}

pub(super) fn write_shell_log(
    logging: &dyn WorkspaceShellLogPort,
    level: WorkspaceLogLevel,
    session_id: &str,
    shell_id: &str,
    message: &str,
) {
    logging.write(ShellLog {
        level,
        session_id: session_id.to_string(),
        shell_id: shell_id.to_string(),
        message: message.to_string(),
    });
}

/// Writes a termination result, and only when it is worth writing. A clean reap is not a
/// diagnostic; anything that leaves a live child is. The codes are the message — no raw command
/// or PTY output goes near it.
pub(super) fn log_termination(
    logging: &dyn WorkspaceShellLogPort,
    session_id: &str,
    shell_id: &str,
    report: TerminationReport,
    shutdown: bool,
) {
    let Some(outcome) = report.outcome else {
        return;
    };
    if outcome == TerminationOutcome::Reaped {
        return;
    }
    let phase = if shutdown { " during shutdown" } else { "" };
    let message = if outcome.leaves_a_live_child() {
        format!(
            "Shell process was not reaped within the termination deadline{phase} \
             (outcome: {}, cleanup: {}). The child may still be running and its handle is \
             retained for a later sweep.",
            outcome.code(),
            report.cleanup.code()
        )
    } else {
        format!(
            "Shell process termination did not complete{phase} (outcome: {}, cleanup: {}).",
            outcome.code(),
            report.cleanup.code()
        )
    };
    write_shell_log(
        logging,
        WorkspaceLogLevel::Warn,
        session_id,
        shell_id,
        &message,
    );
}

/// Runs one kill and one bounded reap against a shared child.
///
/// Returns the outcome only; deciding what to do with a child that survived — adoption into the
/// pending registry — belongs to the caller, which is the only party that can hand over
/// ownership before releasing the shell.
pub(super) fn reap_shared_child(
    child: &Mutex<Box<dyn Child + Send + Sync>>,
    killer: &Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>,
    deadline: Instant,
) -> TerminationOutcome {
    match probe_shared_child(child) {
        // A child already gone is reaped, not a special case. The distinction the caller needs
        // is whether anything is still owed, and here nothing is.
        ChildProbe::Exited => return TerminationOutcome::Reaped,
        ChildProbe::Failed => return TerminationOutcome::ReapFailed,
        ChildProbe::Running => {}
    }
    // portable-pty 0.9 inverts the Windows TerminateProcess result, so the signal's own return
    // value is not authoritative alone. A refused kill is only reported as such once a fresh
    // probe confirms the child is still there.
    let kill_refused = match killer.lock() {
        Ok(mut killer) => killer.kill().is_err(),
        Err(_) => true,
    };
    if kill_refused {
        return match probe_shared_child(child) {
            ChildProbe::Exited => TerminationOutcome::Reaped,
            ChildProbe::Failed => TerminationOutcome::ReapFailed,
            ChildProbe::Running => TerminationOutcome::KillFailed,
        };
    }
    match poll_until_exit(
        || probe_shared_child(child),
        || true,
        Some(deadline),
        POLL_INTERVAL_CEILING,
    ) {
        PollEnd::Exited => TerminationOutcome::Reaped,
        PollEnd::ProbeFailed => TerminationOutcome::ReapFailed,
        PollEnd::DeadlineReached | PollEnd::Abandoned => TerminationOutcome::ReapTimedOut,
    }
}

/// Whether an outcome means the caller must hand the child to the pending registry rather than
/// drop it.
pub(super) fn requires_adoption(outcome: TerminationOutcome) -> bool {
    outcome.leaves_a_live_child()
}

/// The cleanup state an owner settles on, given what its attempt did.
pub(super) fn settled_cleanup(outcome: TerminationOutcome) -> CleanupState {
    if outcome.leaves_a_live_child() {
        CleanupState::Pending
    } else {
        CleanupState::NotRequired
    }
}
