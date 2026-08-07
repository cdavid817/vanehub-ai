//! Drives a multi-seat session's round: take the completed turn, decide who is next, invoke them,
//! wait, repeat.
//!
//! Runs on its own thread rather than inside the generation sink. The sink holds ports, not the
//! service, and starting a generation from inside a terminal handler would nest one generation's
//! lifecycle inside another's — the same reason the Loop runtime keeps its scheduler separate.
//!
//! Seats respond one at a time. Running a round's seats concurrently would mean a later seat
//! reading a thread that is missing an earlier seat's reply, which is the whole content of the
//! turn it was handed.

use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, AgentRuntimeApplicationService, SeatTurnAssignment, SeatTurnStop,
};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long a single seat's turn may take before the round is abandoned. Generous, because a CLI
/// Agent doing real work routinely runs for minutes; the point is to release the round rather than
/// to bound the work.
const TURN_TIMEOUT: Duration = Duration::from_secs(30 * 60);

const POLL_INTERVAL: Duration = Duration::from_millis(200);

#[derive(Clone)]
pub(crate) struct NativeSeatTurnCoordinator {
    runtime: AgentRuntimeApplicationService,
    running: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

impl NativeSeatTurnCoordinator {
    pub(crate) fn new(runtime: AgentRuntimeApplicationService) -> Self {
        Self {
            runtime,
            running: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        }
    }

    /// Starts driving `session_id`'s round, if one is not already being driven.
    ///
    /// Refusing a second driver is what keeps seats serial: two coordinators on one session would
    /// each take turns off the same queue and invoke seats in parallel.
    pub(crate) fn schedule(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        {
            let mut running = self
                .running
                .lock()
                .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))?;
            if !running.insert(session_id.to_string()) {
                return Ok(());
            }
        }
        let coordinator = self.clone();
        let owned = session_id.to_string();
        let spawned = std::thread::Builder::new()
            .name(format!("seat-turns-{session_id}"))
            .spawn(move || {
                coordinator.run(&owned);
                if let Ok(mut running) = coordinator.running.lock() {
                    running.remove(&owned);
                }
            });
        if spawned.is_err() {
            if let Ok(mut running) = self.running.lock() {
                running.remove(session_id);
            }
            return Err(AgentRuntimeApplicationError::Generation(
                "Could not start the seat turn thread.".to_string(),
            ));
        }
        Ok(())
    }

    fn run(&self, session_id: &str) {
        let mut queue: VecDeque<SeatTurnAssignment> = VecDeque::new();
        loop {
            let Some(terminal) = self.await_terminal(session_id) else {
                return;
            };
            match self.runtime.decide_seat_turn(&terminal) {
                Ok(decision) => {
                    // A stop that transfers the turn abandons the queue: seats still waiting were
                    // routed on the premise that the round would continue.
                    if matches!(
                        decision.stop,
                        Some(SeatTurnStop::AwaitingHuman | SeatTurnStop::RoundComplete)
                    ) {
                        return;
                    }
                    queue.extend(decision.next);
                }
                Err(_) => return,
            }

            let Some(assignment) = queue.pop_front() else {
                return;
            };
            if self
                .runtime
                .start_seat_turn(session_id, &assignment)
                .is_err()
            {
                return;
            }
        }
    }

    fn await_terminal(
        &self,
        session_id: &str,
    ) -> Option<crate::contexts::agent_runtime::application::SeatTurnTerminal> {
        let started = Instant::now();
        loop {
            match self.runtime.take_seat_turn_completion(session_id) {
                Ok(Some(terminal)) => return Some(terminal),
                Ok(None) => {}
                Err(_) => return None,
            }
            if started.elapsed() >= TURN_TIMEOUT {
                return None;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
    }
}
