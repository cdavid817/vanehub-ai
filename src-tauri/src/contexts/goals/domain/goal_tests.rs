use super::goal::{Goal, GoalDomainError, GoalInput, GoalStatus};

const ALL_STATUSES: [GoalStatus; 4] = [
    GoalStatus::Draft,
    GoalStatus::Active,
    GoalStatus::Achieved,
    GoalStatus::Abandoned,
];

const LEGAL_TRANSITIONS: [(GoalStatus, GoalStatus); 7] = [
    (GoalStatus::Draft, GoalStatus::Active),
    (GoalStatus::Draft, GoalStatus::Abandoned),
    (GoalStatus::Active, GoalStatus::Achieved),
    (GoalStatus::Active, GoalStatus::Abandoned),
    (GoalStatus::Achieved, GoalStatus::Active),
    (GoalStatus::Achieved, GoalStatus::Abandoned),
    (GoalStatus::Abandoned, GoalStatus::Active),
];

fn input(title: &str) -> GoalInput {
    GoalInput {
        id: "goal-1".to_string(),
        title: title.to_string(),
        description: String::new(),
        acceptance_notes: String::new(),
        project_path: None,
    }
}

#[test]
fn every_status_pair_matches_the_declared_transition_table() {
    for from in ALL_STATUSES {
        for to in ALL_STATUSES {
            let expected = LEGAL_TRANSITIONS.contains(&(from, to));
            assert_eq!(
                from.can_transition_to(to),
                expected,
                "transition {} -> {} should be {}",
                from.as_str(),
                to.as_str(),
                if expected { "legal" } else { "rejected" }
            );
        }
    }
}

#[test]
fn illegal_transitions_report_both_ends() {
    let error = GoalStatus::Draft
        .transition(GoalStatus::Achieved)
        .expect_err("draft cannot jump straight to achieved");
    assert_eq!(
        error,
        GoalDomainError::InvalidTransition {
            from: "draft",
            to: "achieved",
        }
    );
}

#[test]
fn every_status_survives_a_string_round_trip() {
    for status in ALL_STATUSES {
        assert_eq!(GoalStatus::parse(status.as_str()), Ok(status));
    }
}

#[test]
fn an_unknown_status_string_is_rejected() {
    assert_eq!(
        GoalStatus::parse("awaiting_acceptance"),
        Err(GoalDomainError::InvalidStatus(
            "awaiting_acceptance".to_string()
        ))
    );
}

#[test]
fn acceptance_requires_derived_readiness() {
    assert_eq!(
        GoalStatus::Active.accept(false),
        Err(GoalDomainError::AcceptanceNotReady)
    );
    assert_eq!(GoalStatus::Active.accept(true), Ok(GoalStatus::Achieved));
}

#[test]
fn readiness_alone_does_not_bypass_the_transition_table() {
    assert_eq!(
        GoalStatus::Draft.accept(true),
        Err(GoalDomainError::InvalidTransition {
            from: "draft",
            to: "achieved",
        })
    );
}

#[test]
fn a_new_goal_starts_as_a_draft() {
    let goal = Goal::new(input("Ship the goal system"), "2026-08-15T00:00:00Z")
        .expect("a titled goal is valid");
    assert_eq!(goal.status, GoalStatus::Draft);
    assert_eq!(goal.created_at, goal.updated_at);
}

#[test]
fn a_blank_title_is_rejected() {
    assert_eq!(
        Goal::new(input("   "), "2026-08-15T00:00:00Z"),
        Err(GoalDomainError::MissingTitle)
    );
}

#[test]
fn normalization_trims_text_and_drops_an_empty_project_path() {
    let goal = Goal::new(
        GoalInput {
            id: " goal-1 ".to_string(),
            title: "  Ship it  ".to_string(),
            description: "  why  ".to_string(),
            acceptance_notes: "  how we judge  ".to_string(),
            project_path: Some("   ".to_string()),
        },
        "2026-08-15T00:00:00Z",
    )
    .expect("whitespace around a real title is fine");

    assert_eq!(goal.id, "goal-1");
    assert_eq!(goal.title, "Ship it");
    assert_eq!(goal.description, "why");
    assert_eq!(goal.acceptance_notes, "how we judge");
    assert_eq!(goal.project_path, None);
}

#[test]
fn moving_a_goal_stamps_the_update_time() {
    let mut goal =
        Goal::new(input("Ship it"), "2026-08-15T00:00:00Z").expect("a titled goal is valid");
    goal.move_to(GoalStatus::Active, "2026-08-15T01:00:00Z")
        .expect("draft can be activated");

    assert_eq!(goal.status, GoalStatus::Active);
    assert_eq!(goal.updated_at, "2026-08-15T01:00:00Z");
    assert_eq!(goal.created_at, "2026-08-15T00:00:00Z");
}

#[test]
fn a_goal_cannot_be_accepted_while_a_child_is_still_running() {
    let mut goal =
        Goal::new(input("Ship it"), "2026-08-15T00:00:00Z").expect("a titled goal is valid");
    goal.move_to(GoalStatus::Active, "2026-08-15T01:00:00Z")
        .expect("draft can be activated");

    assert_eq!(
        goal.accept(false, "2026-08-15T02:00:00Z"),
        Err(GoalDomainError::AcceptanceNotReady)
    );
    assert_eq!(goal.status, GoalStatus::Active);
    assert_eq!(goal.updated_at, "2026-08-15T01:00:00Z");
}
