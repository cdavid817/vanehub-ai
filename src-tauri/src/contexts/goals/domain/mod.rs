mod goal;
#[cfg(test)]
mod goal_tests;
mod link;

pub(crate) use goal::{Goal, GoalInput, GoalStatus};
pub(crate) use link::{GoalLink, GoalLinkTarget};
