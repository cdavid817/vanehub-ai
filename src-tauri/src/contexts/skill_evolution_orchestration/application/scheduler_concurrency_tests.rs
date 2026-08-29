use super::*;

#[tokio::test]
async fn workspace_single_flight_rejects_a_parallel_run_and_releases_on_drop() {
    let coordinator = EvolutionConcurrencyCoordinatorV1::new();
    let first = coordinator
        .acquire_workspace("workspace-one")
        .await
        .expect("first");
    assert_eq!(first.workspace_id(), "workspace-one");
    assert!(matches!(
        coordinator.acquire_workspace("workspace-one").await,
        Err(SchedulerConcurrencyError::WorkspaceBusy)
    ));
    drop(first);
    coordinator
        .acquire_workspace("workspace-one")
        .await
        .expect("released workspace");
}

#[tokio::test]
async fn read_concurrency_is_two_and_cancelled_waits_release_workspace_reservations() {
    let coordinator = EvolutionConcurrencyCoordinatorV1::new();
    let first = coordinator
        .acquire_workspace("workspace-one")
        .await
        .expect("first");
    let second = coordinator
        .acquire_workspace("workspace-two")
        .await
        .expect("second");
    let pending = {
        let coordinator = coordinator.clone();
        tokio::spawn(async move { coordinator.acquire_workspace("workspace-three").await })
    };
    tokio::task::yield_now().await;
    assert_eq!(coordinator.active_workspace_count(), Ok(3));
    pending.abort();
    assert!(matches!(pending.await, Err(error) if error.is_cancelled()));
    assert_eq!(coordinator.active_workspace_count(), Ok(2));
    drop(first);
    let third = coordinator
        .acquire_workspace("workspace-three")
        .await
        .expect("third after cancellation");
    assert_eq!(third.workspace_id(), "workspace-three");
    drop(second);
}

#[tokio::test]
async fn automatic_mutation_lane_is_globally_single_flight() {
    let coordinator = EvolutionConcurrencyCoordinatorV1::new();
    let first = coordinator
        .acquire_automatic_mutation()
        .await
        .expect("first mutation");
    let pending = {
        let coordinator = coordinator.clone();
        tokio::spawn(async move { coordinator.acquire_automatic_mutation().await })
    };
    tokio::task::yield_now().await;
    assert!(!pending.is_finished());
    pending.abort();
    assert!(matches!(pending.await, Err(error) if error.is_cancelled()));
    drop(first);
    coordinator
        .acquire_automatic_mutation()
        .await
        .expect("released mutation lane");
}
