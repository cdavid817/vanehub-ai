use crate::contexts::task_orchestration::domain::SubTaskRunStatus;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScheduleNode {
    pub(crate) id: String,
    pub(crate) status: SubTaskRunStatus,
    pub(crate) topological_rank: u16,
    pub(crate) ordinal: u16,
    pub(crate) predecessors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunProjection {
    Continue,
    AwaitingAcceptance,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScheduleDecision {
    pub(crate) next_id: Option<String>,
    pub(crate) blocked_ids: Vec<String>,
    pub(crate) projection: RunProjection,
}

pub(crate) fn decide_serial_schedule(nodes: &[ScheduleNode]) -> ScheduleDecision {
    let mut states = nodes
        .iter()
        .map(|node| (node.id.clone(), node.status))
        .collect::<BTreeMap<_, _>>();
    let mut blocked = BTreeSet::new();
    loop {
        let newly_blocked = nodes
            .iter()
            .filter(|node| {
                matches!(
                    states[&node.id],
                    SubTaskRunStatus::Pending | SubTaskRunStatus::Ready
                ) && node.predecessors.iter().any(|predecessor| {
                    states.get(predecessor).is_some_and(|status| {
                        matches!(
                            status,
                            SubTaskRunStatus::Failed
                                | SubTaskRunStatus::Interrupted
                                | SubTaskRunStatus::Blocked
                                | SubTaskRunStatus::Cancelled
                                | SubTaskRunStatus::Skipped
                        )
                    })
                })
            })
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if newly_blocked.is_empty() {
            break;
        }
        for id in newly_blocked {
            states.insert(id.clone(), SubTaskRunStatus::Blocked);
            blocked.insert(id);
        }
    }

    let active = states.values().any(|status| {
        matches!(
            status,
            SubTaskRunStatus::Dispatching | SubTaskRunStatus::Running | SubTaskRunStatus::Verifying
        )
    });
    let next_id = (!active)
        .then(|| {
            nodes
                .iter()
                .filter(|node| {
                    matches!(
                        states[&node.id],
                        SubTaskRunStatus::Pending | SubTaskRunStatus::Ready
                    ) && node.predecessors.iter().all(|predecessor| {
                        states.get(predecessor) == Some(&SubTaskRunStatus::Succeeded)
                    })
                })
                .min_by_key(|node| (node.topological_rank, node.ordinal, node.id.as_str()))
                .map(|node| node.id.clone())
        })
        .flatten();

    let all_succeeded = states
        .values()
        .all(|status| *status == SubTaskRunStatus::Succeeded);
    let unfinished = states.values().any(|status| {
        matches!(
            status,
            SubTaskRunStatus::Pending
                | SubTaskRunStatus::Ready
                | SubTaskRunStatus::Dispatching
                | SubTaskRunStatus::Running
                | SubTaskRunStatus::Verifying
        )
    });
    let projection = if all_succeeded {
        RunProjection::AwaitingAcceptance
    } else if next_id.is_none() && !unfinished {
        RunProjection::Failed
    } else {
        RunProjection::Continue
    };
    ScheduleDecision {
        next_id,
        blocked_ids: blocked.into_iter().collect(),
        projection,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(
        id: &str,
        status: SubTaskRunStatus,
        rank: u16,
        ordinal: u16,
        predecessors: &[&str],
    ) -> ScheduleNode {
        ScheduleNode {
            id: id.into(),
            status,
            topological_rank: rank,
            ordinal,
            predecessors: predecessors.iter().map(|id| (*id).to_string()).collect(),
        }
    }

    #[test]
    fn deterministically_selects_only_one_eligible_node() {
        let decision = decide_serial_schedule(&[
            node("later", SubTaskRunStatus::Pending, 0, 2, &[]),
            node("first", SubTaskRunStatus::Pending, 0, 1, &[]),
        ]);
        assert_eq!(decision.next_id.as_deref(), Some("first"));
        assert_eq!(decision.projection, RunProjection::Continue);
    }

    #[test]
    fn active_work_prevents_a_second_claim_and_predecessors_wait() {
        let decision = decide_serial_schedule(&[
            node("active", SubTaskRunStatus::Running, 0, 0, &[]),
            node("dependent", SubTaskRunStatus::Pending, 1, 1, &["active"]),
            node("independent", SubTaskRunStatus::Pending, 0, 2, &[]),
        ]);
        assert_eq!(decision.next_id, None);
        assert!(decision.blocked_ids.is_empty());
    }

    #[test]
    fn failure_blocks_only_descendants_and_independent_work_continues() {
        let decision = decide_serial_schedule(&[
            node("failed", SubTaskRunStatus::Failed, 0, 0, &[]),
            node("child", SubTaskRunStatus::Pending, 1, 1, &["failed"]),
            node("grandchild", SubTaskRunStatus::Pending, 2, 2, &["child"]),
            node("independent", SubTaskRunStatus::Pending, 0, 3, &[]),
        ]);
        assert_eq!(decision.next_id.as_deref(), Some("independent"));
        assert_eq!(decision.blocked_ids, vec!["child", "grandchild"]);
    }

    #[test]
    fn terminal_projection_distinguishes_success_from_exhaustion() {
        let success =
            decide_serial_schedule(&[node("done", SubTaskRunStatus::Succeeded, 0, 0, &[])]);
        assert_eq!(success.projection, RunProjection::AwaitingAcceptance);
        let failed = decide_serial_schedule(&[node("failed", SubTaskRunStatus::Failed, 0, 0, &[])]);
        assert_eq!(failed.projection, RunProjection::Failed);
    }
}
