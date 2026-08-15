use super::*;

fn submitted(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(content, status)| ((*content).to_owned(), (*status).to_owned()))
        .collect()
}

fn valid_pair() -> Vec<(String, String)> {
    submitted(&[
        ("Read the failing test", STATUS_COMPLETED),
        ("Fix the parser", STATUS_IN_PROGRESS),
    ])
}

#[test]
fn a_valid_list_preserves_submitted_order_and_trims_content() {
    let items = validate(&submitted(&[
        ("  Investigate the crash  ", STATUS_PENDING),
        ("Write the fix", STATUS_IN_PROGRESS),
        ("Ship it", STATUS_PENDING),
    ]))
    .expect("valid list");

    assert_eq!(items.len(), 3);
    assert_eq!(items[0].content, "Investigate the crash");
    assert_eq!(items[0].status, TaskStatus::Pending);
    assert_eq!(items[1].status, TaskStatus::InProgress);
    assert_eq!(items[2].content, "Ship it");
}

#[test]
fn more_than_one_in_progress_item_is_rejected() {
    let error = validate(&submitted(&[
        ("First", STATUS_IN_PROGRESS),
        ("Second", STATUS_IN_PROGRESS),
    ]))
    .expect_err("two in-progress items must be rejected");

    assert_eq!(error, TaskListError::MultipleInProgress { count: 2 });
    assert!(error.message().contains("only one task may be in progress"));
}

#[test]
fn a_list_with_no_in_progress_item_is_valid() {
    for status in [STATUS_PENDING, STATUS_COMPLETED] {
        validate(&submitted(&[("Only task", status)]))
            .unwrap_or_else(|_| panic!("a list of all-{status} items is a legitimate state"));
    }
    validate(&submitted(&[
        ("Done", STATUS_COMPLETED),
        ("Later", STATUS_PENDING),
    ]))
    .expect("between steps is a legitimate state");
}

#[test]
fn an_over_long_list_is_rejected_rather_than_truncated() {
    let items: Vec<(String, String)> = (0..=MAX_TASK_ITEMS)
        .map(|index| (format!("Task {index}"), STATUS_PENDING.to_owned()))
        .collect();

    let error = validate(&items).expect_err("over-long list must be rejected");
    assert_eq!(
        error,
        TaskListError::TooManyItems {
            submitted: MAX_TASK_ITEMS + 1
        }
    );
}

#[test]
fn a_list_at_exactly_the_bound_is_accepted() {
    let items: Vec<(String, String)> = (0..MAX_TASK_ITEMS)
        .map(|index| (format!("Task {index}"), STATUS_PENDING.to_owned()))
        .collect();
    assert_eq!(
        validate(&items).expect("at the bound").len(),
        MAX_TASK_ITEMS
    );
}

#[test]
fn empty_or_whitespace_content_is_rejected_with_its_position() {
    for blank in ["", "   ", "\t\n"] {
        let error = validate(&submitted(&[
            ("Fine", STATUS_PENDING),
            (blank, STATUS_PENDING),
        ]))
        .expect_err("blank content must be rejected");
        assert_eq!(error, TaskListError::EmptyContent { index: 1 });
        // Positions are reported 1-based, matching how the model wrote the list.
        assert!(error.message().contains("Task 2"));
    }
}

#[test]
fn over_long_content_is_rejected_and_counts_characters_not_bytes() {
    let content = "中".repeat(MAX_TASK_CONTENT_CHARS + 1);
    let error = validate(&submitted(&[(&content, STATUS_PENDING)]))
        .expect_err("over-long content must be rejected");
    assert_eq!(
        error,
        TaskListError::ContentTooLong {
            index: 0,
            characters: MAX_TASK_CONTENT_CHARS + 1
        }
    );

    let at_bound = "中".repeat(MAX_TASK_CONTENT_CHARS);
    validate(&submitted(&[(&at_bound, STATUS_PENDING)]))
        .expect("a multi-byte item at the character bound fits");
}

#[test]
fn an_unrecognized_status_is_rejected_and_names_the_valid_values() {
    let error =
        validate(&submitted(&[("Task", "blocked")])).expect_err("unknown status must be rejected");
    assert_eq!(
        error,
        TaskListError::UnknownStatus {
            index: 0,
            status: "blocked".to_owned()
        }
    );
    let message = error.message();
    for valid in [STATUS_PENDING, STATUS_IN_PROGRESS, STATUS_COMPLETED] {
        assert!(message.contains(valid), "message omits {valid}: {message}");
    }
}

#[test]
fn replacing_a_list_swaps_it_wholesale_rather_than_merging() {
    let store = TaskListStore::default();
    store.replace("session-a", validate(&valid_pair()).expect("valid"));

    let replacement = validate(&submitted(&[("Something else entirely", STATUS_PENDING)]))
        .expect("valid replacement");
    store.replace("session-a", replacement);

    let stored = store.get("session-a");
    assert_eq!(
        stored.len(),
        1,
        "a replacement must not merge with the old list"
    );
    assert_eq!(stored[0].content, "Something else entirely");
}

#[test]
fn an_empty_submission_clears_the_list() {
    let store = TaskListStore::default();
    store.replace("session-a", validate(&valid_pair()).expect("valid"));

    assert!(store.replace("session-a", Vec::new()).is_empty());
    assert!(store.get("session-a").is_empty());
}

#[test]
fn each_session_sees_only_its_own_list() {
    let store = TaskListStore::default();
    store.replace(
        "session-a",
        validate(&submitted(&[("A's task", STATUS_PENDING)])).expect("valid"),
    );
    store.replace(
        "session-b",
        validate(&submitted(&[("B's task", STATUS_IN_PROGRESS)])).expect("valid"),
    );

    assert_eq!(store.get("session-a")[0].content, "A's task");
    assert_eq!(store.get("session-b")[0].content, "B's task");
    assert!(store.get("session-never-wrote").is_empty());
}

#[test]
fn clearing_one_session_leaves_the_others_intact() {
    let store = TaskListStore::default();
    for session in ["session-a", "session-b"] {
        store.replace(
            session,
            validate(&submitted(&[("Task", STATUS_PENDING)])).expect("valid"),
        );
    }

    store.clear_session("session-a");

    assert!(store.get("session-a").is_empty());
    assert_eq!(store.get("session-b").len(), 1);
}

#[test]
fn rendering_distinguishes_all_three_states() {
    let items = validate(&submitted(&[
        ("Done thing", STATUS_COMPLETED),
        ("Current thing", STATUS_IN_PROGRESS),
        ("Later thing", STATUS_PENDING),
    ]))
    .expect("valid");

    let rendered = render(&items);
    assert_eq!(
        rendered,
        "[x] Done thing\n[~] Current thing\n[ ] Later thing"
    );
}

#[test]
fn statuses_round_trip_through_their_wire_strings() {
    for status in [
        TaskStatus::Pending,
        TaskStatus::InProgress,
        TaskStatus::Completed,
    ] {
        assert_eq!(TaskStatus::parse(status.as_str()), Some(status));
    }
    assert_eq!(TaskStatus::parse("in-progress"), None);
    assert_eq!(TaskStatus::parse(""), None);
}

#[test]
fn the_prompt_section_is_absent_until_a_session_has_tasks() {
    // Uses the process-wide store, so it needs a session id no other test writes.
    let session = "task-list-prompt-section-session";
    assert_eq!(prompt_section(session), None);

    store().replace(
        session,
        validate(&submitted(&[("Only task", STATUS_IN_PROGRESS)])).expect("valid"),
    );
    let section = prompt_section(session).expect("a section once tasks exist");
    assert!(section.starts_with("## Task list"));
    assert!(section.contains("[~] Only task"));

    store().clear_session(session);
    assert_eq!(
        prompt_section(session),
        None,
        "clearing removes the section rather than leaving an empty heading"
    );
}
