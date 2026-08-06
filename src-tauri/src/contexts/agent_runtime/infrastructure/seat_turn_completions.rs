use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, SeatTurnCompletionPort, SeatTurnTerminal,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

/// In-memory hand-off point between the generation sink and the turn coordinator, mirroring
/// `InMemoryLoopRoleGenerationCompletions`.
///
/// Turns are queued per session and taken in order, because seats in a round respond one at a time
/// and a later turn must not overtake an earlier one.
#[derive(Default)]
pub(crate) struct InMemorySeatTurnCompletions {
    state: Mutex<SeatTurnState>,
}

#[derive(Default)]
struct SeatTurnState {
    /// Keyed by (session, message) so a redelivered terminal cannot start the next seat twice.
    delivered: HashSet<(String, String)>,
    pending: HashMap<String, VecDeque<SeatTurnTerminal>>,
}

impl SeatTurnCompletionPort for InMemorySeatTurnCompletions {
    fn deliver(&self, terminal: SeatTurnTerminal) -> Result<bool, AgentRuntimeApplicationError> {
        let mut state = self.lock()?;
        let key = (terminal.session_id.clone(), terminal.message_id.clone());
        if !state.delivered.insert(key) {
            return Ok(false);
        }
        state
            .pending
            .entry(terminal.session_id.clone())
            .or_default()
            .push_back(terminal);
        Ok(true)
    }

    fn take_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<SeatTurnTerminal>, AgentRuntimeApplicationError> {
        let mut state = self.lock()?;
        let Some(queue) = state.pending.get_mut(session_id) else {
            return Ok(None);
        };
        let terminal = queue.pop_front();
        if queue.is_empty() {
            state.pending.remove(session_id);
        }
        Ok(terminal)
    }
}

impl InMemorySeatTurnCompletions {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, SeatTurnState>, AgentRuntimeApplicationError> {
        self.state
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terminal(message_id: &str, seat_index: usize) -> SeatTurnTerminal {
        SeatTurnTerminal {
            session_id: "s1".to_string(),
            message_id: message_id.to_string(),
            seat_index,
            seat_mention: "架构师".to_string(),
            depth: 1,
            reply: Some("@代码审查 看下".to_string()),
        }
    }

    #[test]
    fn delivers_and_takes_a_turn() {
        let completions = InMemorySeatTurnCompletions::default();
        assert!(completions.deliver(terminal("m1", 0)).expect("deliver"));

        let taken = completions.take_for_session("s1").expect("take");
        assert_eq!(taken.map(|value| value.message_id), Some("m1".to_string()));
        assert!(completions.take_for_session("s1").expect("drained").is_none());
    }

    /// A redelivered terminal must not start the next seat a second time.
    #[test]
    fn refuses_to_deliver_the_same_turn_twice() {
        let completions = InMemorySeatTurnCompletions::default();
        assert!(completions.deliver(terminal("m1", 0)).expect("first"));
        assert!(!completions.deliver(terminal("m1", 0)).expect("second"));

        assert!(completions.take_for_session("s1").expect("take").is_some());
        assert!(completions.take_for_session("s1").expect("only once").is_none());
    }

    /// Seats respond one at a time, so a later turn must not overtake an earlier one.
    #[test]
    fn keeps_turns_in_order_within_a_session() {
        let completions = InMemorySeatTurnCompletions::default();
        completions.deliver(terminal("m1", 0)).expect("first");
        completions.deliver(terminal("m2", 1)).expect("second");

        let first = completions.take_for_session("s1").expect("take").expect("present");
        let second = completions.take_for_session("s1").expect("take").expect("present");
        assert_eq!(first.message_id, "m1");
        assert_eq!(second.message_id, "m2");
    }

    #[test]
    fn returns_nothing_for_a_session_with_no_pending_turn() {
        let completions = InMemorySeatTurnCompletions::default();
        assert!(completions.take_for_session("unknown").expect("take").is_none());
    }
}
