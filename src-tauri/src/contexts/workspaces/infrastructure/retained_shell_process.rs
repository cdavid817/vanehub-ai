//! The owned halves of a local Shell, and the bounded sequence that ends one.
//!
//! Split out of the runtime adapter because the runtime adapter cannot be tested. Opening a real
//! PTY needs a real operating system, a real shell binary, and a child that cooperates; the cases
//! worth pinning are the ones where it does not — a child that ignores every signal, a reader
//! blocked inside a driver call that will never return, a kill that fails outright. Those are
//! reachable only through a seam, so the seam is here: three narrow traits, one termination
//! algorithm written against them, and an injected clock so a test asserts the *shape* of the
//! sequence rather than waiting out a production deadline.
//!
//! The rule the algorithm exists to keep: nothing on the command path waits without a ceiling.
//! Not `child.wait()`, not `JoinHandle::join()` on a thread that has not already finished. Every
//! stage observes, gives up, and says so.

use crate::contexts::workspaces::application::ShellRuntimeCloseOutcome;
use crate::contexts::workspaces::domain::{
    shell_reason, shell_reason_code, SessionShellError, ShellCloseBudget, TerminalDimensions,
};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Why an owned process could not answer.
///
/// Two variants because they call for different things: a failed termination request may still be
/// followed by the child dying on its own, while a failed observation means nothing further can be
/// learned here at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShellProcessError {
    /// The platform refused the termination request.
    Terminate,
    /// The platform could not report whether the child is still alive.
    Observe,
}

/// A child process this application owns, reduced to the two questions a bounded close asks.
///
/// Deliberately without a `wait()`. A blocking wait is the single call that turns "close this tab"
/// into "the window is gone and the process is still running"; leaving it out of the vocabulary is
/// cheaper than reviewing for its absence.
pub(super) trait ShellProcessHandle: Send + Sync {
    /// Non-blocking. `Ok(Some(code))` once the child has been reaped, `Ok(None)` while it lives.
    fn try_reap(&self) -> Result<Option<i32>, ShellProcessError>;

    /// Asks the child to end. Idempotent: the force stage calls it again, and a platform that has
    /// only one termination primitive is allowed to do the same thing twice.
    fn terminate(&self) -> Result<(), ShellProcessError>;
}

/// The resizable half of a PTY. One method, because that is all a retained Shell asks of it after
/// the process exists.
pub(super) trait ShellPtyHandle: Send + Sync {
    fn resize(&self, dimensions: TerminalDimensions) -> Result<(), ()>;
}

/// Monotonic time, injected so a deadline can be asserted without waiting for one.
pub(super) trait ShellDeadlineClock: Send + Sync {
    fn elapsed(&self) -> Duration;

    /// Waits, at most, the given duration. A virtual implementation advances instead.
    fn park(&self, duration: Duration);
}

/// The production clock. `Instant` rather than wall time: a host adjusting its clock mid-close must
/// not make a bounded close unbounded, or make it give up immediately.
pub(super) struct MonotonicDeadlineClock {
    started: Instant,
}

impl Default for MonotonicDeadlineClock {
    fn default() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl ShellDeadlineClock for MonotonicDeadlineClock {
    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    fn park(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// Sets its flag however the thread body leaves — returning, or unwinding.
///
/// A flag set on the last line of the body would stay false for a worker that panicked, and a
/// close would then wait out its whole worker window for a thread that finished before it started.
struct CompletionGuard(Arc<AtomicBool>);

impl Drop for CompletionGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// One worker thread, plus the flag that says whether joining it can block.
///
/// The flag is the whole point. `JoinHandle::join()` is unbounded, and the threads here are readers
/// blocked inside a driver call that may never return; joining one unconditionally on the command
/// path is how closing a tab hangs the application. So the handle is joined only once the worker
/// has reported itself finished, and otherwise stays owned for the Reaper to try again.
pub(super) struct ShellWorker {
    completed: Arc<AtomicBool>,
    join: Mutex<Option<JoinHandle<()>>>,
}

impl ShellWorker {
    pub(super) fn spawn<F>(name: String, body: F) -> Result<Self, SessionShellError>
    where
        F: FnOnce() + Send + 'static,
    {
        let completed = Arc::new(AtomicBool::new(false));
        let flag = completed.clone();
        let join = std::thread::Builder::new()
            .name(name)
            .spawn(move || {
                let _guard = CompletionGuard(flag);
                body();
            })
            .map_err(|_| SessionShellError::RuntimeUnavailable {
                reason: shell_reason("shell_worker_thread_unavailable"),
            })?;
        Ok(Self {
            completed,
            join: Mutex::new(Some(join)),
        })
    }

    /// A worker with nothing to run, for the paths that own a completion flag without a thread.
    #[cfg(test)]
    pub(super) fn detached(completed: Arc<AtomicBool>) -> Self {
        Self {
            completed,
            join: Mutex::new(None),
        }
    }

    pub(super) fn is_complete(&self) -> bool {
        self.completed.load(Ordering::SeqCst)
    }

    /// Joins, but only a worker that has already reported itself finished.
    ///
    /// Answers whether the handle is now given up. `false` means the thread is still running and
    /// this call did not wait for it.
    pub(super) fn try_join(&self) -> bool {
        if !self.is_complete() {
            return false;
        }
        let handle = match self.join.lock() {
            Ok(mut guard) => guard.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        };
        if let Some(handle) = handle {
            // Already past its last statement, so this returns rather than waits. A panicking
            // worker is joined the same way: the completion guard fires while unwinding.
            let _joined = handle.join();
        }
        true
    }
}

/// How the bounded close is progressing, for tests and metrics.
///
/// Counts rather than durations. A test that asserted "closed within 1.8 seconds" would be
/// asserting the CI runner's mood; a test asserting "made four observations and never joined" is
/// asserting the algorithm.
#[derive(Debug, Default)]
pub(super) struct CloseObservations {
    reap_checks: AtomicU32,
    terminate_requests: AtomicU32,
}

impl CloseObservations {
    pub(super) fn reap_checks(&self) -> u32 {
        self.reap_checks.load(Ordering::SeqCst)
    }

    pub(super) fn terminate_requests(&self) -> u32 {
        self.terminate_requests.load(Ordering::SeqCst)
    }
}

/// One bounded attempt at ending an owned child and giving up its workers.
///
/// Staged rather than "kill and wait": a shell whose input has just been closed is usually already
/// on its way out, and signalling it would replace an orderly exit with a killed one for no gain.
/// Each stage has its own ceiling, and the sum of them is bounded again by the total, so a caller
/// can state what a close costs without reading this function.
pub(super) fn close_process_bounded(
    process: &dyn ShellProcessHandle,
    workers: &[Arc<ShellWorker>],
    budget: ShellCloseBudget,
    clock: &dyn ShellDeadlineClock,
    observations: &CloseObservations,
    on_reaped: &dyn Fn(),
) -> ShellRuntimeCloseOutcome {
    if !budget.is_finite_and_positive() {
        return ShellRuntimeCloseOutcome::Retained {
            reason: shell_reason(shell_reason_code::CLOSE_DEADLINE_REACHED),
            retryable: true,
        };
    }
    let deadline = clock.elapsed().saturating_add(budget.total);
    let mut terminate_failed = false;
    let mut observe_failed = false;

    // Input was closed by the caller before this ran, so the first stage is the shell noticing
    // rather than anything being signalled at it.
    let mut reaped = observe_reaped(
        process,
        budget.graceful,
        budget.poll,
        deadline,
        clock,
        observations,
        &mut observe_failed,
    );
    for stage in [budget.terminate, budget.force] {
        if reaped {
            break;
        }
        observations
            .terminate_requests
            .fetch_add(1, Ordering::SeqCst);
        if process.terminate().is_err() {
            terminate_failed = true;
        }
        reaped = observe_reaped(
            process,
            stage,
            budget.poll,
            deadline,
            clock,
            observations,
            &mut observe_failed,
        );
    }

    if !reaped {
        // The handles stay here. A `Retained` outcome is the adapter saying "this is still mine",
        // which is the only honest answer while a process it owns may still be running.
        return ShellRuntimeCloseOutcome::Retained {
            reason: shell_reason(if terminate_failed {
                shell_reason_code::TERMINATE_FAILED
            } else if observe_failed {
                shell_reason_code::REAP_DEADLINE_REACHED
            } else {
                shell_reason_code::CLOSE_DEADLINE_REACHED
            }),
            retryable: true,
        };
    }

    // The child is gone, so whatever the caller was holding on its behalf can go too. This runs
    // before the workers are waited on and not after: a reader parked on a terminal that stays open
    // because this code is holding it open is a wait that can only ever time out.
    // The child is gone, so whatever the caller was holding on its behalf can go too. This runs
    // before the workers are waited on and not after: a reader parked on a terminal that stays open
    // because this code is holding it open is a wait that can only ever time out.
    on_reaped();

    if !complete_workers(workers, budget.worker, budget.poll, deadline, clock) {
        return ShellRuntimeCloseOutcome::Retained {
            reason: shell_reason(shell_reason_code::WORKER_COMPLETION_PENDING),
            retryable: true,
        };
    }
    ShellRuntimeCloseOutcome::Confirmed
}

/// Polls until the child is reaped, the stage window closes, or the total budget runs out.
fn observe_reaped(
    process: &dyn ShellProcessHandle,
    stage: Duration,
    poll: Duration,
    deadline: Duration,
    clock: &dyn ShellDeadlineClock,
    observations: &CloseObservations,
    observe_failed: &mut bool,
) -> bool {
    let stage_deadline = clock.elapsed().saturating_add(stage).min(deadline);
    loop {
        observations.reap_checks.fetch_add(1, Ordering::SeqCst);
        match process.try_reap() {
            Ok(Some(_)) => return true,
            Ok(None) => {}
            // Nothing further can be learned here, but the stage still ends rather than spinning:
            // a platform that cannot answer is not a platform that will answer if asked faster.
            Err(_) => {
                *observe_failed = true;
                return false;
            }
        }
        if clock.elapsed() >= stage_deadline {
            return false;
        }
        clock.park(poll);
    }
}

/// Waits for workers to report themselves finished, then gives up their handles.
///
/// Never joins a worker that has not reported. A reader blocked in a driver read is exactly the
/// thread this would hang on, and it is also exactly the thread most likely to be blocked.
fn complete_workers(
    workers: &[Arc<ShellWorker>],
    stage: Duration,
    poll: Duration,
    deadline: Duration,
    clock: &dyn ShellDeadlineClock,
) -> bool {
    let stage_deadline = clock.elapsed().saturating_add(stage).min(deadline);
    loop {
        if workers.iter().all(|worker| worker.is_complete()) {
            return workers.iter().all(|worker| worker.try_join());
        }
        if clock.elapsed() >= stage_deadline {
            return false;
        }
        clock.park(poll);
    }
}
