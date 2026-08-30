use std::{sync::Arc, time::Duration};

use super::*;

#[tokio::test]
async fn waiter_wakes_on_authoritative_state_change_without_polling() {
    let wakeup = Arc::new(IdleStateWakeupV1::new());
    let observed = wakeup.revision();
    let waiter = {
        let wakeup = wakeup.clone();
        tokio::spawn(async move {
            wakeup
                .wait_for_change(observed, Duration::from_secs(1))
                .await
        })
    };
    tokio::task::yield_now().await;
    assert_eq!(wakeup.notify_state_change(), 1);
    assert_eq!(
        waiter.await.expect("waiter"),
        IdleWakeOutcomeV1::StateChanged { revision: 1 }
    );
}

#[tokio::test]
async fn waiter_observes_prior_change_and_bounded_deadline() {
    let wakeup = IdleStateWakeupV1::new();
    wakeup.notify_state_change();
    assert_eq!(
        wakeup.wait_for_change(0, Duration::ZERO).await,
        IdleWakeOutcomeV1::StateChanged { revision: 1 }
    );
    assert_eq!(
        wakeup.wait_for_change(1, Duration::ZERO).await,
        IdleWakeOutcomeV1::DeadlineElapsed
    );
}
