use super::super::domain::{GoalLinkTarget, GoalStatus};
use super::ports::LinkProgress;
use super::progress::{derive, DerivedGoalStatus, LinkProgressView};

fn link(target_kind: GoalLinkTarget, progress: LinkProgress) -> LinkProgressView {
    LinkProgressView {
        target_kind,
        target_id: format!("{}-1", target_kind.as_str()),
        progress,
    }
}

#[test]
fn an_active_goal_with_no_children_is_not_awaiting_acceptance() {
    let progress = derive(GoalStatus::Active, &[]);

    assert_eq!(progress.derived_status, DerivedGoalStatus::Active);
    assert_eq!(progress.counted, 0);
}

#[test]
fn a_partially_finished_goal_is_not_awaiting_acceptance() {
    let progress = derive(
        GoalStatus::Active,
        &[
            link(GoalLinkTarget::WorkItem, LinkProgress::Terminal),
            link(GoalLinkTarget::Loop, LinkProgress::Active),
        ],
    );

    assert_eq!(progress.derived_status, DerivedGoalStatus::Active);
    assert_eq!(progress.counted, 2);
    assert_eq!(progress.terminal, 1);
}

#[test]
fn a_goal_whose_children_all_finished_awaits_acceptance() {
    let progress = derive(
        GoalStatus::Active,
        &[
            link(GoalLinkTarget::Loop, LinkProgress::Terminal),
            link(GoalLinkTarget::WorkItem, LinkProgress::Terminal),
        ],
    );

    assert!(progress.awaiting_acceptance());
    assert_eq!(progress.counted, 2);
    assert_eq!(progress.terminal, 2);
}

#[test]
fn a_deleted_child_leaves_the_denominator_instead_of_blocking_acceptance() {
    let progress = derive(
        GoalStatus::Active,
        &[
            link(GoalLinkTarget::Plan, LinkProgress::Unresolvable),
            link(GoalLinkTarget::Loop, LinkProgress::Terminal),
        ],
    );

    assert!(progress.awaiting_acceptance());
    assert_eq!(progress.counted, 1);
    assert_eq!(progress.unresolvable, 1);
}

#[test]
fn a_goal_whose_only_children_are_unresolvable_stays_active() {
    let progress = derive(
        GoalStatus::Active,
        &[link(GoalLinkTarget::Plan, LinkProgress::Unresolvable)],
    );

    assert_eq!(progress.derived_status, DerivedGoalStatus::Active);
    assert_eq!(progress.counted, 0);
    assert_eq!(progress.unresolvable, 1);
}

#[test]
fn sessions_never_push_a_goal_toward_acceptance() {
    let progress = derive(
        GoalStatus::Active,
        &[
            link(GoalLinkTarget::Session, LinkProgress::Terminal),
            link(GoalLinkTarget::Session, LinkProgress::Active),
        ],
    );

    assert_eq!(progress.derived_status, DerivedGoalStatus::Active);
    assert_eq!(progress.counted, 0);
    assert_eq!(progress.terminal, 0);
}

#[test]
fn a_session_alongside_finished_work_does_not_hold_the_goal_back() {
    let progress = derive(
        GoalStatus::Active,
        &[
            link(GoalLinkTarget::WorkItem, LinkProgress::Terminal),
            link(GoalLinkTarget::Session, LinkProgress::Active),
        ],
    );

    assert!(progress.awaiting_acceptance());
    assert_eq!(progress.counted, 1);
}

#[test]
fn a_child_parked_at_its_own_acceptance_keeps_the_goal_active() {
    // The child still needs a human, so the goal must not claim it is ready
    // for acceptance as well.
    let progress = derive(
        GoalStatus::Active,
        &[link(GoalLinkTarget::Loop, LinkProgress::Active)],
    );

    assert_eq!(progress.derived_status, DerivedGoalStatus::Active);
}

#[test]
fn non_active_goals_pass_their_stored_status_straight_through() {
    let finished = [link(GoalLinkTarget::WorkItem, LinkProgress::Terminal)];

    for (stored, expected) in [
        (GoalStatus::Draft, DerivedGoalStatus::Draft),
        (GoalStatus::Achieved, DerivedGoalStatus::Achieved),
        (GoalStatus::Abandoned, DerivedGoalStatus::Abandoned),
    ] {
        assert_eq!(derive(stored, &finished).derived_status, expected);
    }
}

#[test]
fn reopening_a_child_pulls_a_goal_back_out_of_acceptance() {
    let finished = [link(GoalLinkTarget::Loop, LinkProgress::Terminal)];
    assert!(derive(GoalStatus::Active, &finished).awaiting_acceptance());

    let reopened = [link(GoalLinkTarget::Loop, LinkProgress::Active)];
    assert!(!derive(GoalStatus::Active, &reopened).awaiting_acceptance());
}
