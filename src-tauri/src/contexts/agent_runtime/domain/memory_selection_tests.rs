use super::memory_freshness::{memory_staleness_caveat, render_memory_age};
use super::memory_selection::{
    parse_memory_selection, MAX_SELECTED_MEMORIES, MEMORY_SELECTION_INSTRUCTION,
};
use std::collections::HashSet;
use std::time::{Duration, SystemTime};

fn available(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[test]
fn selected_names_keep_the_selectors_own_order() {
    let selected = parse_memory_selection(
        r#"["npm-only","user-role"]"#,
        &available(&["user-role", "npm-only", "unused"]),
    );

    assert_eq!(selected, vec!["npm-only", "user-role"]);
}

#[test]
fn an_empty_array_selects_nothing_and_is_not_a_failure() {
    // The expected answer most of the time. A selector that always returns something turns the
    // relevance budget into a random sample, which is worse than injecting nothing.
    assert!(parse_memory_selection("[]", &available(&["a"])).is_empty());
}

#[test]
fn a_hallucinated_name_is_dropped_without_discarding_the_valid_ones_beside_it() {
    let selected = parse_memory_selection(
        r#"["real-one","invented","also-real"]"#,
        &available(&["real-one", "also-real"]),
    );

    assert_eq!(selected, vec!["real-one", "also-real"]);
}

#[test]
fn duplicates_are_collapsed() {
    let selected =
        parse_memory_selection(r#"["same","same","other"]"#, &available(&["same", "other"]));

    assert_eq!(selected, vec!["same", "other"]);
}

#[test]
fn the_selection_is_capped() {
    let names = (0..MAX_SELECTED_MEMORIES + 4)
        .map(|index| format!("memory-{index}"))
        .collect::<Vec<_>>();
    let response = serde_json::to_string(&names).expect("response");
    let pool = names.iter().map(String::as_str).collect::<Vec<_>>();

    let selected = parse_memory_selection(&response, &available(&pool));

    assert_eq!(selected.len(), MAX_SELECTED_MEMORIES);
}

#[test]
fn an_unusable_response_selects_nothing_rather_than_failing() {
    // Selection is an enhancement. Its failure costs relevance, never the generation, so every
    // malformed shape degrades to "no bodies" and the index alone carries the turn.
    for malformed in [
        "",
        "I could not decide.",
        "{\"selected\": []}",
        "[\"unterminated",
        "[1, 2, 3]",
    ] {
        assert!(
            parse_memory_selection(malformed, &available(&["a"])).is_empty(),
            "expected {malformed:?} to select nothing"
        );
    }
}

#[test]
fn prose_and_code_fences_around_the_array_are_tolerated() {
    let selected = parse_memory_selection(
        "Sure:\n```json\n[\"npm-only\"]\n```\n",
        &available(&["npm-only"]),
    );

    assert_eq!(selected, vec!["npm-only"]);
}

#[test]
fn the_instruction_states_the_bound_and_the_permission_to_return_nothing() {
    assert!(MEMORY_SELECTION_INSTRUCTION.contains("at most 5"));
    assert!(MEMORY_SELECTION_INSTRUCTION.contains("empty array"));
    // Judging from descriptions alone is what keeps the call's cost proportional to how many
    // memories exist rather than how large they are.
    assert!(MEMORY_SELECTION_INSTRUCTION.contains("not being shown the memories' contents"));
}

#[test]
fn age_is_rendered_in_words_rather_than_as_a_timestamp() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(400 * 24 * 60 * 60);
    let ago = |days: u64| Some(now - Duration::from_secs(days * 24 * 60 * 60));

    assert_eq!(render_memory_age(ago(0), now).as_deref(), Some("today"));
    assert_eq!(render_memory_age(ago(1), now).as_deref(), Some("yesterday"));
    assert_eq!(
        render_memory_age(ago(5), now).as_deref(),
        Some("5 days ago")
    );
    assert_eq!(
        render_memory_age(ago(30), now).as_deref(),
        Some("4 weeks ago")
    );
    assert_eq!(
        render_memory_age(ago(200), now).as_deref(),
        Some("6 months ago")
    );
    assert_eq!(
        render_memory_age(ago(370), now).as_deref(),
        Some("1 years ago")
    );
}

#[test]
fn an_unknown_or_future_modification_time_has_no_age() {
    // Clock skew and copied files both produce future timestamps. Reporting "unknown" is honest
    // where clamping to "today" would assert a freshness this cannot know.
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    assert_eq!(render_memory_age(None, now), None);
    assert_eq!(
        render_memory_age(Some(now + Duration::from_secs(60)), now),
        None
    );
    assert_eq!(memory_staleness_caveat(None, now), None);
}

#[test]
fn only_a_memory_past_the_threshold_carries_a_caveat() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(10 * 24 * 60 * 60);
    let hours = |count: u64| Some(now - Duration::from_secs(count * 60 * 60));

    // A caveat on something written an hour ago is noise, and noise trains the model to skim past
    // caveats generally -- including the ones that matter.
    assert_eq!(memory_staleness_caveat(hours(1), now), None);
    assert_eq!(memory_staleness_caveat(hours(23), now), None);
    assert!(memory_staleness_caveat(hours(25), now).is_some());
}
