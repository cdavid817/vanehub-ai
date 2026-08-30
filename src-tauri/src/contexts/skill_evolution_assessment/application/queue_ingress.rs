use super::{
    AssessmentQueueError, AssessmentQueuePersistence, AssessmentQueueRequest, QueueEnqueueOutcome,
};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScheduleAck {
    Accepted,
    Full,
    Closed,
    Disabled,
}

#[derive(Clone)]
pub(crate) struct AssessmentQueueIngress {
    sender: mpsc::Sender<AssessmentQueueRequest>,
}

pub(crate) struct AssessmentQueueDrain {
    receiver: mpsc::Receiver<AssessmentQueueRequest>,
}

pub(crate) fn assessment_queue_channel(
    capacity: usize,
) -> Result<(AssessmentQueueIngress, AssessmentQueueDrain), AssessmentQueueError> {
    if capacity == 0 || capacity > 10_000 {
        return Err(AssessmentQueueError::InvalidInput);
    }
    let (sender, receiver) = mpsc::channel(capacity);
    Ok((
        AssessmentQueueIngress { sender },
        AssessmentQueueDrain { receiver },
    ))
}

impl AssessmentQueueIngress {
    pub(crate) fn try_schedule(&self, request: AssessmentQueueRequest) -> ScheduleAck {
        self.try_schedule_when(true, request)
    }

    pub(crate) fn try_schedule_when(
        &self,
        enabled: bool,
        request: AssessmentQueueRequest,
    ) -> ScheduleAck {
        if !enabled {
            return ScheduleAck::Disabled;
        }
        match self.sender.try_send(request) {
            Ok(()) => ScheduleAck::Accepted,
            Err(mpsc::error::TrySendError::Full(_)) => ScheduleAck::Full,
            Err(mpsc::error::TrySendError::Closed(_)) => ScheduleAck::Closed,
        }
    }
}

impl AssessmentQueueDrain {
    pub(crate) async fn persist_next(
        &mut self,
        repository: Arc<dyn AssessmentQueuePersistence>,
    ) -> Option<Result<QueueEnqueueOutcome, AssessmentQueueError>> {
        let request = self.receiver.recv().await?;
        Some(
            tokio::task::spawn_blocking(move || repository.enqueue(&request))
                .await
                .unwrap_or(Err(AssessmentQueueError::WorkerPanic)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::AssessmentQueueLane;
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturingPersistence(Mutex<Vec<AssessmentQueueRequest>>);

    struct PanicPersistence;

    impl AssessmentQueuePersistence for PanicPersistence {
        fn enqueue(
            &self,
            _request: &AssessmentQueueRequest,
        ) -> Result<QueueEnqueueOutcome, AssessmentQueueError> {
            panic!("injected queue worker panic")
        }
    }

    struct DatabaseLockPersistence;

    impl AssessmentQueuePersistence for DatabaseLockPersistence {
        fn enqueue(
            &self,
            _request: &AssessmentQueueRequest,
        ) -> Result<QueueEnqueueOutcome, AssessmentQueueError> {
            Err(AssessmentQueueError::DatabaseLock)
        }
    }

    impl AssessmentQueuePersistence for CapturingPersistence {
        fn enqueue(
            &self,
            request: &AssessmentQueueRequest,
        ) -> Result<QueueEnqueueOutcome, AssessmentQueueError> {
            self.0.lock().expect("requests").push(request.clone());
            Ok(QueueEnqueueOutcome::Scheduled {
                queue_id: request.queue_id.clone(),
            })
        }
    }

    #[tokio::test]
    async fn scheduling_ack_is_non_blocking_bounded_and_persisted_off_caller_path() {
        let repository = Arc::new(CapturingPersistence::default());
        let (ingress, mut drain) = assessment_queue_channel(1).expect("channel");
        assert_eq!(
            ingress.try_schedule(request("queue-1")),
            ScheduleAck::Accepted
        );
        assert_eq!(ingress.try_schedule(request("queue-2")), ScheduleAck::Full);
        assert!(matches!(
            drain.persist_next(repository.clone()).await,
            Some(Ok(QueueEnqueueOutcome::Scheduled { .. }))
        ));
        assert_eq!(
            ingress.try_schedule(request("queue-2")),
            ScheduleAck::Accepted
        );
        assert_eq!(repository.0.lock().expect("requests").len(), 1);
        drop(drain);
        assert_eq!(
            ingress.try_schedule(request("queue-3")),
            ScheduleAck::Closed
        );
    }

    #[tokio::test]
    async fn disabled_panic_and_database_lock_paths_fail_open() {
        let (ingress, mut drain) = assessment_queue_channel(1).expect("channel");
        assert_eq!(
            ingress.try_schedule_when(false, request("disabled")),
            ScheduleAck::Disabled
        );
        assert_eq!(
            ingress.try_schedule(request("panic")),
            ScheduleAck::Accepted
        );
        assert_eq!(
            drain.persist_next(Arc::new(PanicPersistence)).await,
            Some(Err(AssessmentQueueError::WorkerPanic))
        );
        assert_eq!(
            ingress.try_schedule(request("database-lock")),
            ScheduleAck::Accepted
        );
        assert_eq!(
            drain.persist_next(Arc::new(DatabaseLockPersistence)).await,
            Some(Err(AssessmentQueueError::DatabaseLock))
        );
    }

    fn request(queue_id: &str) -> AssessmentQueueRequest {
        AssessmentQueueRequest {
            queue_id: queue_id.to_string(),
            seed_id: "seed-1".to_string(),
            witness_hash: format!("witness-{queue_id}"),
            lane: AssessmentQueueLane::Deterministic,
            priority: 1,
            available_at_ms: 10,
            created_at_ms: 10,
        }
    }
}
