use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::contexts::skill_evolution_orchestration::domain::is_safe_identifier;

const GLOBAL_READ_CONCURRENCY_V1: usize = 2;
const GLOBAL_AUTOMATIC_MUTATION_CONCURRENCY_V1: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchedulerConcurrencyError {
    InvalidWorkspace,
    WorkspaceBusy,
    StateUnavailable,
    SchedulerClosed,
}

#[derive(Clone)]
pub(crate) struct EvolutionConcurrencyCoordinatorV1 {
    inner: Arc<ConcurrencyState>,
}

struct ConcurrencyState {
    active_workspaces: Mutex<HashSet<String>>,
    read_lanes: Arc<Semaphore>,
    mutation_lanes: Arc<Semaphore>,
}

pub(crate) struct WorkspaceRunPermitV1 {
    reservation: WorkspaceReservation,
    _read_lane: OwnedSemaphorePermit,
}

pub(crate) struct AutomaticMutationPermitV1 {
    _mutation_lane: OwnedSemaphorePermit,
}

struct WorkspaceReservation {
    workspace_id: String,
    state: Arc<ConcurrencyState>,
}

impl Default for EvolutionConcurrencyCoordinatorV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl EvolutionConcurrencyCoordinatorV1 {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(ConcurrencyState {
                active_workspaces: Mutex::new(HashSet::new()),
                read_lanes: Arc::new(Semaphore::new(GLOBAL_READ_CONCURRENCY_V1)),
                mutation_lanes: Arc::new(Semaphore::new(GLOBAL_AUTOMATIC_MUTATION_CONCURRENCY_V1)),
            }),
        }
    }

    pub(crate) async fn acquire_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<WorkspaceRunPermitV1, SchedulerConcurrencyError> {
        let reservation = self.reserve_workspace(workspace_id)?;
        let read_lane = self
            .inner
            .read_lanes
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| SchedulerConcurrencyError::SchedulerClosed)?;
        Ok(WorkspaceRunPermitV1 {
            reservation,
            _read_lane: read_lane,
        })
    }

    pub(crate) async fn acquire_automatic_mutation(
        &self,
    ) -> Result<AutomaticMutationPermitV1, SchedulerConcurrencyError> {
        let mutation_lane = self
            .inner
            .mutation_lanes
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| SchedulerConcurrencyError::SchedulerClosed)?;
        Ok(AutomaticMutationPermitV1 {
            _mutation_lane: mutation_lane,
        })
    }

    pub(crate) fn active_workspace_count(&self) -> Result<usize, SchedulerConcurrencyError> {
        self.inner
            .active_workspaces
            .lock()
            .map(|workspaces| workspaces.len())
            .map_err(|_| SchedulerConcurrencyError::StateUnavailable)
    }

    fn reserve_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<WorkspaceReservation, SchedulerConcurrencyError> {
        if !is_safe_identifier(workspace_id, 128) {
            return Err(SchedulerConcurrencyError::InvalidWorkspace);
        }
        let mut workspaces = self
            .inner
            .active_workspaces
            .lock()
            .map_err(|_| SchedulerConcurrencyError::StateUnavailable)?;
        if !workspaces.insert(workspace_id.into()) {
            return Err(SchedulerConcurrencyError::WorkspaceBusy);
        }
        Ok(WorkspaceReservation {
            workspace_id: workspace_id.into(),
            state: self.inner.clone(),
        })
    }
}

impl WorkspaceRunPermitV1 {
    pub(crate) fn workspace_id(&self) -> &str {
        &self.reservation.workspace_id
    }
}

impl Drop for WorkspaceReservation {
    fn drop(&mut self) {
        if let Ok(mut workspaces) = self.state.active_workspaces.lock() {
            workspaces.remove(&self.workspace_id);
        }
    }
}
