use crate::contexts::agent_runtime::application::{
    AgentMessageTerminal, AgentMessageTerminalCompletionPort, AgentMessageTerminalReceiver,
    AgentRuntimeApplicationError,
};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex, MutexGuard};

#[derive(Default)]
pub(crate) struct InMemoryAgentMessageTerminalCompletions {
    pending: Arc<Mutex<HashMap<String, mpsc::SyncSender<AgentMessageTerminal>>>>,
}

impl InMemoryAgentMessageTerminalCompletions {
    fn pending(
        &self,
    ) -> Result<
        MutexGuard<'_, HashMap<String, mpsc::SyncSender<AgentMessageTerminal>>>,
        AgentRuntimeApplicationError,
    > {
        self.pending
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))
    }

    #[cfg(test)]
    pub(crate) fn pending_count(&self) -> usize {
        self.pending()
            .map(|pending| pending.len())
            .unwrap_or_default()
    }
}

impl AgentMessageTerminalCompletionPort for InMemoryAgentMessageTerminalCompletions {
    fn register(
        &self,
        session_id: &str,
    ) -> Result<AgentMessageTerminalReceiver, AgentRuntimeApplicationError> {
        let (sender, receiver) = mpsc::sync_channel(1);
        if self
            .pending()?
            .insert(session_id.to_string(), sender)
            .is_some()
        {
            return Err(AgentRuntimeApplicationError::GenerationConflict(
                session_id.to_string(),
            ));
        }
        let pending = Arc::downgrade(&self.pending);
        let cleanup_session_id = session_id.to_string();
        Ok(AgentMessageTerminalReceiver::new(
            receiver,
            Box::new(move || {
                if let Some(pending) = pending.upgrade() {
                    if let Ok(mut pending) = pending.lock() {
                        pending.remove(&cleanup_session_id);
                    }
                }
            }),
        ))
    }

    fn deliver(
        &self,
        terminal: AgentMessageTerminal,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let sender = self.pending()?.remove(&terminal.session_id);
        let Some(sender) = sender else {
            return Ok(false);
        };
        let _ = sender.try_send(terminal);
        Ok(true)
    }

    fn remove(&self, session_id: &str) -> Result<bool, AgentRuntimeApplicationError> {
        Ok(self.pending()?.remove(session_id).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::AgentMessageTerminalOutcome;
    use std::time::Duration;

    fn terminal(outcome: AgentMessageTerminalOutcome) -> AgentMessageTerminal {
        AgentMessageTerminal {
            session_id: "session-1".to_string(),
            message_id: "message-1".to_string(),
            outcome,
            content: (outcome == AgentMessageTerminalOutcome::Completed)
                .then(|| "done".to_string()),
        }
    }

    #[test]
    fn completed_failed_and_cancelled_terminals_are_delivered_exactly_once() {
        for outcome in [
            AgentMessageTerminalOutcome::Completed,
            AgentMessageTerminalOutcome::Failed,
            AgentMessageTerminalOutcome::Cancelled,
        ] {
            let completions = InMemoryAgentMessageTerminalCompletions::default();
            let receiver = completions.register("session-1").expect("register");
            assert!(completions.deliver(terminal(outcome)).expect("deliver"));
            assert!(!completions.deliver(terminal(outcome)).expect("duplicate"));
            assert_eq!(
                receiver
                    .recv_timeout(Duration::ZERO)
                    .expect("terminal")
                    .outcome,
                outcome
            );
            assert_eq!(completions.pending_count(), 0);
        }
    }

    #[test]
    fn terminal_before_receive_and_dropped_receivers_cleanup_registration() {
        let completions = InMemoryAgentMessageTerminalCompletions::default();
        let receiver = completions.register("session-1").expect("register");
        assert!(completions
            .deliver(terminal(AgentMessageTerminalOutcome::Completed))
            .expect("deliver before receive"));
        assert_eq!(completions.pending_count(), 0);
        assert_eq!(
            receiver
                .recv_timeout(Duration::ZERO)
                .expect("buffered")
                .content,
            Some("done".to_string())
        );

        let dropped = completions.register("session-1").expect("register dropped");
        drop(dropped);
        assert_eq!(completions.pending_count(), 0);
        assert!(!completions
            .deliver(terminal(AgentMessageTerminalOutcome::Failed))
            .expect("dropped already cleaned"));
    }

    #[test]
    fn explicit_removal_cleans_up_failed_launch_registration() {
        let completions = InMemoryAgentMessageTerminalCompletions::default();
        let _receiver = completions.register("session-1").expect("register");
        assert!(completions.remove("session-1").expect("remove"));
        assert!(!completions.remove("session-1").expect("remove again"));
        assert_eq!(completions.pending_count(), 0);
    }
}
