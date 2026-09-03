use std::time::Duration;

use tokio::{sync::watch, time::timeout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdleWakeOutcomeV1 {
    StateChanged { revision: u64 },
    DeadlineElapsed,
}

#[derive(Clone)]
pub(crate) struct IdleStateWakeupV1 {
    sender: watch::Sender<u64>,
}

impl Default for IdleStateWakeupV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl IdleStateWakeupV1 {
    pub(crate) fn new() -> Self {
        let (sender, _receiver) = watch::channel(0);
        Self { sender }
    }

    pub(crate) fn revision(&self) -> u64 {
        *self.sender.borrow()
    }

    pub(crate) fn notify_state_change(&self) -> u64 {
        let mut revision = 0;
        self.sender.send_modify(|current| {
            *current = current.saturating_add(1);
            revision = *current;
        });
        revision
    }

    pub(crate) async fn wait_for_change(
        &self,
        observed_revision: u64,
        maximum_wait: Duration,
    ) -> IdleWakeOutcomeV1 {
        let mut receiver = self.sender.subscribe();
        let current = *receiver.borrow_and_update();
        if current != observed_revision {
            return IdleWakeOutcomeV1::StateChanged { revision: current };
        }
        match timeout(maximum_wait, receiver.changed()).await {
            Ok(Ok(())) => IdleWakeOutcomeV1::StateChanged {
                revision: *receiver.borrow_and_update(),
            },
            Ok(Err(_)) | Err(_) => IdleWakeOutcomeV1::DeadlineElapsed,
        }
    }
}
