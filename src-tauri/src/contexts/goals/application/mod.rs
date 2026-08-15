mod goal_service;
mod ports;
mod progress;
#[cfg(test)]
mod progress_tests;

pub(crate) use goal_service::{GoalApplicationService, GoalDetail};
pub(crate) use ports::{GoalRepository, LinkProgress, LinkProgressProbe};
