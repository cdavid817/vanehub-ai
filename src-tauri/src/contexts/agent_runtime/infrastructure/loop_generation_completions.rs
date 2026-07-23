use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LoopRoleGenerationCompletionPort, LoopRoleGenerationTerminal,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;

#[derive(Default)]
pub(crate) struct InMemoryLoopRoleGenerationCompletions {
    state: Mutex<CompletionState>,
}

#[derive(Default)]
struct CompletionState {
    delivered: HashSet<(String, String)>,
    pending: HashMap<String, VecDeque<LoopRoleGenerationTerminal>>,
}

impl LoopRoleGenerationCompletionPort for InMemoryLoopRoleGenerationCompletions {
    fn deliver(
        &self,
        terminal: LoopRoleGenerationTerminal,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Loop(error.to_string()))?;
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
    ) -> Result<Option<LoopRoleGenerationTerminal>, AgentRuntimeApplicationError> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Loop(error.to_string()))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::LoopRoleGenerationOutcome;

    fn terminal(outcome: LoopRoleGenerationOutcome) -> LoopRoleGenerationTerminal {
        LoopRoleGenerationTerminal {
            run_id: "run-1".to_string(),
            iteration_id: "iteration-1".to_string(),
            role: "worker".to_string(),
            session_id: "session-1".to_string(),
            message_id: "message-1".to_string(),
            outcome,
            content: None,
            error: None,
        }
    }

    #[test]
    fn first_terminal_outcome_wins_and_can_be_taken_only_once() {
        let completions = InMemoryLoopRoleGenerationCompletions::default();

        assert!(completions
            .deliver(terminal(LoopRoleGenerationOutcome::Cancelled))
            .expect("cancel delivery"));
        assert!(!completions
            .deliver(terminal(LoopRoleGenerationOutcome::Failed))
            .expect("late failure ignored"));

        let delivered = completions
            .take_for_session("session-1")
            .expect("take")
            .expect("terminal");
        assert_eq!(delivered.outcome, LoopRoleGenerationOutcome::Cancelled);
        assert_eq!(
            completions
                .take_for_session("session-1")
                .expect("second take"),
            None
        );
        assert!(!completions
            .deliver(terminal(LoopRoleGenerationOutcome::Completed))
            .expect("late completion ignored"));
    }
}
