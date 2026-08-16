use std::sync::Arc;

use super::application::{GoalApplicationService, LinkProgressProbe};
use super::infrastructure::{
    LoopProgressProbe, PlanProgressProbe, RunProgressProbe, SessionProgressProbe,
    SqliteGoalRepository, WorkItemProgressProbe,
};
use crate::platform::database::NativeDatabase;

pub(crate) use super::application::GoalDetail;
pub(crate) use super::domain::{GoalInput, GoalLinkTarget};

/// Composes the goal service at the application edge. The probes are the only
/// place this context touches another one, and they are injected here so the
/// domain and application layers stay free of executor imports.
pub(crate) fn build_service(database: NativeDatabase) -> GoalApplicationService {
    let probes: Vec<Arc<dyn LinkProgressProbe>> = vec![
        Arc::new(PlanProgressProbe::new(database.clone())),
        Arc::new(LoopProgressProbe::new(database.clone())),
        Arc::new(WorkItemProgressProbe::new(database.clone())),
        Arc::new(SessionProgressProbe::new(database.clone())),
        Arc::new(RunProgressProbe::new(database.clone())),
    ];

    GoalApplicationService::new(Arc::new(SqliteGoalRepository::new(database)), probes)
}
